#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${BASH_SOURCE[0]%/*}"
[ "$ROOT_DIR" = "${BASH_SOURCE[0]}" ] && ROOT_DIR="."
ROOT_DIR="$(cd "$ROOT_DIR/.." && pwd)"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux.sh"
CODEX_TERMUX_WRAPPER_SOURCE_DIR="${CODEX_TERMUX_WRAPPER_SOURCE_DIR:-$ROOT_DIR}"
CODEX_TERMUX_WRAPPER_SOURCE_TMP=""

usage() {
    cat <<'USAGE'
Usage: bash bin/install-runtime.sh [install|update|repair|remove|doctor] [ARGS]

install [VERSION]           Install support files, launcher, and a fresh patched runtime.
install support             Refresh support files and the launcher only.
install upstream [VERSION]  Install a fresh patched runtime from upstream raw.
install rebuild             Refresh support files and rebuild patched runtime from cached raw.
update [VERSION]            Same as install [VERSION]: refresh support and patched runtime.
repair                      Diagnose and repair the managed installation.
remove                      Remove the managed launcher/runtime and restore a launcher backup.
doctor                      Run wrapper diagnostics. Use: doctor --json for machine output.
USAGE
}

codex_source_commit() {
    local source_dir="${1:-$CODEX_TERMUX_WRAPPER_SOURCE_DIR}"
    if command -v git >/dev/null 2>&1 && git -C "$source_dir" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git -C "$source_dir" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown\n'
    else
        printf 'unknown\n'
    fi
}

codex_validate_wrapper_source() {
    local source_dir="$1"
    [ -f "$source_dir/install.sh" ] || return 1
    [ -f "$source_dir/bin/install-runtime.sh" ] || return 1
    [ -f "$source_dir/lib/codex-termux.sh" ] || return 1
    [ -f "$source_dir/tools/build-runtime.py" ] || return 1
    [ -d "$source_dir/tools/codex_termux" ] || return 1
    [ -f "$source_dir/config/wrapper-version.env" ] || return 1
}

codex_find_extracted_wrapper_source() {
    local extract_dir="$1" candidate
    if codex_validate_wrapper_source "$extract_dir"; then
        printf '%s\n' "$extract_dir"
        return 0
    fi
    while IFS= read -r candidate; do
        candidate="${candidate%/bin/install-runtime.sh}"
        if codex_validate_wrapper_source "$candidate"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done <<EOF
$(find "$extract_dir" -maxdepth 3 -type f -path '*/bin/install-runtime.sh' 2>/dev/null)
EOF
    return 1
}

codex_download_wrapper_archive() {
    local source="$1" target="$2"
    if [ -r "$source" ]; then
        cp "$source" "$target"
        return $?
    fi
    curl -fsSL "$source" -o "$target"
}

codex_fetch_release_wrapper_source() {
    local url="${CODEX_TERMUX_WRAPPER_RELEASE_URL:-}" repo="${CODEX_TERMUX_WRAPPER_RELEASE_REPO:-}" tag="${CODEX_TERMUX_WRAPPER_RELEASE_TAG:-}"
    local tmp archive extract source_dir actual_sha expected_sha
    if [ -z "$url" ] && [ -n "$repo" ] && [ -n "$tag" ]; then
        url="https://github.com/$repo/archive/refs/tags/$tag.tar.gz"
    fi
    [ -n "$url" ] || return 1
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/codex-wrapper-release.XXXXXX")"
    archive="$tmp/wrapper.tar.gz"
    extract="$tmp/extract"
    mkdir -p "$extract"
    codex_download_wrapper_archive "$url" "$archive" || {
        rm -rf "$tmp"
        return 1
    }
    expected_sha="${CODEX_TERMUX_WRAPPER_RELEASE_SHA256:-}"
    if [ -n "$expected_sha" ]; then
        actual_sha="$(codex_sha256 "$archive")"
        [ "$actual_sha" = "$expected_sha" ] || {
            rm -rf "$tmp"
            codex_fail "Wrapper release checksum mismatch"
            return 1
        }
    fi
    tar -xf "$archive" -C "$extract" || {
        rm -rf "$tmp"
        return 1
    }
    source_dir="$(codex_find_extracted_wrapper_source "$extract")" || {
        rm -rf "$tmp"
        codex_fail "Wrapper release archive does not contain a valid wrapper source"
        return 1
    }
    CODEX_TERMUX_WRAPPER_SOURCE_TMP="$tmp"
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$source_dir"
}

codex_release_wrapper_source_configured() {
    [ -n "${CODEX_TERMUX_WRAPPER_RELEASE_URL:-}" ] ||
        { [ -n "${CODEX_TERMUX_WRAPPER_RELEASE_REPO:-}" ] && [ -n "${CODEX_TERMUX_WRAPPER_RELEASE_TAG:-}" ]; }
}

codex_prepare_fresh_wrapper_source() {
    if codex_release_wrapper_source_configured; then
        codex_fetch_release_wrapper_source
        return $?
    fi
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$ROOT_DIR"
    codex_validate_wrapper_source "$CODEX_TERMUX_WRAPPER_SOURCE_DIR"
}

codex_cleanup_fresh_wrapper_source() {
    [ -z "$CODEX_TERMUX_WRAPPER_SOURCE_TMP" ] || rm -rf "$CODEX_TERMUX_WRAPPER_SOURCE_TMP"
    CODEX_TERMUX_WRAPPER_SOURCE_TMP=""
}

codex_copy_wrapper_source_snapshot() {
    local source_dir="$1" target_dir="$2"
    if [ "$(cd "$source_dir" && pwd)" = "$(mkdir -p "$target_dir" && cd "$target_dir" && pwd)" ]; then
        return 0
    fi
    codex_rm_rf_managed "$target_dir" || return $?
    mkdir -p "$target_dir" || return $?
    cp "$source_dir/install.sh" "$target_dir/install.sh"
    cp -R "$source_dir/bin" "$target_dir/bin"
    cp -R "$source_dir/lib" "$target_dir/lib"
    cp -R "$source_dir/tools" "$target_dir/tools"
    cp -R "$source_dir/config" "$target_dir/config"
    [ ! -f "$source_dir/README.md" ] || cp "$source_dir/README.md" "$target_dir/README.md"
}

codex_write_managed_shell() {
    mkdir -p "$CODEX_TERMUX_MANAGER_DIR"
    cat >"$CODEX_TERMUX_MANAGED_SHELL.$$" <<MANAGED
#!$CODEX_TERMUX_PREFIX/bin/bash
# codex termux managed shell
set -euo pipefail
export CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="$CODEX_TERMUX_SOURCE_DIR/bin/install-runtime.sh"
# shellcheck disable=SC1091
. "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
codex_main "\$@"
MANAGED
    chmod 755 "$CODEX_TERMUX_MANAGED_SHELL.$$"
    mv "$CODEX_TERMUX_MANAGED_SHELL.$$" "$CODEX_TERMUX_MANAGED_SHELL"
}

codex_remove_python_bytecode() {
    local python_root="$1"
    [ -d "$python_root" ] || return 0
    find "$python_root" \( -type d -name __pycache__ -o -type f -name '*.pyc' \) -exec rm -rf {} +
}

codex_check_manager_python() {
    local manager_package="$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    codex_remove_python_bytecode "$manager_package"
    python3 - "$manager_package" <<'PYTHON'
import pathlib
import sys

package = pathlib.Path(sys.argv[1])
for path in sorted(package.glob("*.py")):
    source = path.read_text(encoding="utf-8")
    compile(source, str(path), "exec")
PYTHON
    codex_remove_python_bytecode "$manager_package"
}

codex_install_support_files() {
    local wrapper_commit source_dir="$CODEX_TERMUX_WRAPPER_SOURCE_DIR"
    codex_validate_wrapper_source "$source_dir" || {
        codex_fail "Invalid wrapper source: $source_dir"
        return 1
    }
    mkdir -p "$CODEX_TERMUX_MANAGER_DIR" "$CODEX_TERMUX_STATE_DIR"
    codex_prepare_system_config
    codex_copy_wrapper_source_snapshot "$source_dir" "$CODEX_TERMUX_SOURCE_DIR" || return $?
    cp "$source_dir/lib/codex-termux.sh" "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
    chmod 755 "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
    codex_rm_rf_managed "$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    cp -R "$source_dir/tools/codex_termux" "$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    codex_check_manager_python
    cp "$source_dir/tools/build-runtime.py" "$CODEX_TERMUX_MANAGER_DIR/build-runtime.py"
    cp "$source_dir/tools/bwrap-termux-compat.py" "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py"
    cp "$source_dir/tools/rg-termux-shim.sh" "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh"
    cp "$source_dir/tools/codex-turn-notify.sh" "$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh"
    chmod 755 "$CODEX_TERMUX_MANAGER_DIR/build-runtime.py" \
        "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py" \
        "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh" \
        "$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh"
    if [ -f "$source_dir/config/wrapper-version.env" ]; then
        cp "$source_dir/config/wrapper-version.env" "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    else
        printf 'CODEX_TERMUX_WRAPPER_VERSION=unknown\nCODEX_TERMUX_WRAPPER_CHANNEL=local\nCODEX_TERMUX_WRAPPER_REPO=local/codex-termux\n' >"$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    fi
    wrapper_commit="$(codex_source_commit "$source_dir")"
    {
        printf 'CODEX_TERMUX_WRAPPER_COMMIT=%s\n' "$wrapper_commit"
        printf 'CODEX_TERMUX_WRAPPER_INSTALLED_AT=%s\n' "$(date -Is)"
    } >>"$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    chmod 644 "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    codex_write_managed_shell
}

codex_launcher_available() { command -v clang >/dev/null 2>&1; }

codex_build_launcher() {
    clang -O2 -Wall -Wextra -o "$1" "$CODEX_TERMUX_WRAPPER_SOURCE_DIR/tools/codex-launcher.c"
}

codex_prepare_launcher_slot() {
    local public="$1" backup base
    mkdir -p "${public%/*}" "$CODEX_TERMUX_BACKUP_DIR"
    if [ -d "$public" ] && [ ! -L "$public" ]; then
        codex_fail "refusing to replace launcher directory $public"
        return 1
    fi
    if [ -e "$public" ] || [ -L "$public" ]; then
        if codex_file_has_marker "$public"; then
            return 0
        fi
        base="$(basename "$public")"
        backup="$CODEX_TERMUX_BACKUP_DIR/$base.$(date +%Y%m%d-%H%M%S).bak"
        cp -Pp "$public" "$backup"
    fi
}

codex_write_shell_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    cat >"$tmp" <<LAUNCHER
#!/bin/sh
# $CODEX_TERMUX_MANAGED_LAUNCHER_MARKER
exec "$CODEX_TERMUX_MANAGED_SHELL" "\$@"
LAUNCHER
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public" || ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_write_compiled_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    codex_build_launcher "$tmp"
    if ! grep -a -q "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" "$tmp"; then
        rm -f "$tmp"
        codex_fail "compiled launcher missing managed marker"
        return 1
    fi
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public" || ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_install_launchers() {
    if codex_launcher_available; then
        codex_write_compiled_launcher "$CODEX_TERMUX_PUBLIC_CODEX"
    else
        codex_write_shell_launcher "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
}

codex_install_full() {
    local status=0
    local print_version="${CODEX_TERMUX_INSTALL_PRINT_VERSION:-1}"
    codex_prepare_fresh_wrapper_source || return $?
    {
        codex_validate_runtime_retention &&
        codex_install_support_files &&
        codex_install_launchers &&
        codex_update "${1:-}" &&
        codex_refresh_runtime_metadata &&
        { [ "$print_version" = "0" ] && codex_status_clear || codex_version; }
    } || status=$?
    codex_cleanup_fresh_wrapper_source
    [ "$status" -eq 0 ] && [ "$print_version" = "0" ] && codex_status_clear
    return "$status"
}

codex_install_support() {
    local status=0
    codex_prepare_fresh_wrapper_source || return $?
    {
        codex_install_support_files &&
        codex_install_launchers
    } || status=$?
    codex_cleanup_fresh_wrapper_source
    return "$status"
}

codex_install_upstream() {
    codex_update "${1:-}"
}

codex_install_rebuild() {
    local status=0
    codex_prepare_fresh_wrapper_source || return $?
    {
        codex_install_support_files &&
        codex_install_launchers &&
        codex_repair_public
    } || status=$?
    codex_cleanup_fresh_wrapper_source
    return "$status"
}

codex_install_dispatch() {
    case "${1:-}" in
        ""|-h|--help|help)
            if [ "${1:-}" = "" ]; then
                codex_install_full
            else
                usage
            fi
            ;;
        support)
            shift
            [ $# -eq 0 ] || {
                codex_fail "install support does not take arguments"
                return 2
            }
            codex_install_support
            ;;
        upstream)
            shift
            codex_install_upstream "${1:-}"
            ;;
        rebuild)
            shift
            [ $# -eq 0 ] || {
                codex_fail "install rebuild does not take arguments"
                return 2
            }
            codex_install_rebuild
            ;;
        *)
            codex_install_full "$1"
            ;;
    esac
}

main() {
    case "${1:-install}" in
        install)
            shift || true
            codex_install_dispatch "$@"
            ;;
        repair)
            codex_repair_public
            ;;
        update)
            shift || true
            case "${1:-}" in
                -h|--help|help)
                    usage
                    ;;
                *)
                    codex_install_full "${1:-}"
                    ;;
            esac
            ;;
        setup)
            printf 'codex setup is reserved for configuration. Use install, update, or repair.\n' >&2
            exit 2
            ;;
        remove)
            codex_remove
            ;;
        doctor)
            shift || true
            codex_wrapper_doctor "$@"
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    main "$@"
fi
