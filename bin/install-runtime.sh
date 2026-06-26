#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${BASH_SOURCE[0]%/*}"
[ "$ROOT_DIR" = "${BASH_SOURCE[0]}" ] && ROOT_DIR="."
ROOT_DIR="$(cd "$ROOT_DIR/.." && pwd)"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux.sh"

usage() {
    cat <<'USAGE'
Usage: bash bin/install-runtime.sh [install|support|rebuild|repair|update|remove|doctor]

install      Install support files, launcher, and a fresh upstream Codex runtime.
support      Refresh support files and the launcher only.
rebuild      Refresh support files and rebuild the runtime from cached raw.
repair       Rebuild the runtime from the cached raw package without network access.
update       Fetch, patch, smoke-test, and promote the linux-arm64 Codex runtime.
remove       Remove the managed launcher/runtime and restore a launcher backup.
doctor       Run wrapper diagnostics. Use: doctor --json for machine output.
USAGE
}

codex_source_commit() {
    if command -v git >/dev/null 2>&1 && git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown\n'
    else
        printf 'unknown\n'
    fi
}

codex_write_managed_shell() {
    mkdir -p "$CODEX_TERMUX_MANAGER_DIR"
    cat >"$CODEX_TERMUX_MANAGED_SHELL.$$" <<MANAGED
#!$CODEX_TERMUX_PREFIX/bin/bash
# codex termux managed shell
set -euo pipefail
export CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="$ROOT_DIR/bin/install-runtime.sh"
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
    local wrapper_commit
    mkdir -p "$CODEX_TERMUX_MANAGER_DIR" "$CODEX_TERMUX_STATE_DIR"
    codex_prepare_system_config
    cp "$ROOT_DIR/lib/codex-termux.sh" "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
    chmod 755 "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
    codex_rm_rf_managed "$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    cp -R "$ROOT_DIR/tools/codex_termux" "$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    codex_check_manager_python
    cp "$ROOT_DIR/tools/build-runtime.py" "$CODEX_TERMUX_MANAGER_DIR/build-runtime.py"
    cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py"
    cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh"
    cp "$ROOT_DIR/tools/codex-turn-notify.sh" "$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh"
    chmod 755 "$CODEX_TERMUX_MANAGER_DIR/build-runtime.py" \
        "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py" \
        "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh" \
        "$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh"
    if [ -f "$ROOT_DIR/config/wrapper-version.env" ]; then
        cp "$ROOT_DIR/config/wrapper-version.env" "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    else
        printf 'CODEX_TERMUX_WRAPPER_VERSION=unknown\nCODEX_TERMUX_WRAPPER_CHANNEL=local\nCODEX_TERMUX_WRAPPER_REPO=local/codex-termux\n' >"$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    fi
    wrapper_commit="$(codex_source_commit)"
    {
        printf 'CODEX_TERMUX_WRAPPER_COMMIT=%s\n' "$wrapper_commit"
        printf 'CODEX_TERMUX_WRAPPER_INSTALLED_AT=%s\n' "$(date -Is)"
    } >>"$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    chmod 644 "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    codex_write_managed_shell
}

codex_launcher_available() { command -v clang >/dev/null 2>&1; }

codex_build_launcher() {
    clang -O2 -Wall -Wextra -o "$1" "$ROOT_DIR/tools/codex-launcher.c"
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

codex_install() {
    codex_validate_runtime_retention || return $?
    codex_install_support_files
    codex_install_launchers
    codex_update "${1:-}" || return $?
    codex_refresh_runtime_metadata
    [ "${CODEX_TERMUX_INSTALL_PRINT_VERSION:-1}" = "0" ] || codex_version
}

codex_rebuild() {
    codex_install_support_files
    codex_install_launchers
    codex_repair_public
}

main() {
    case "${1:-install}" in
        install)
            shift || true
            codex_install "${1:-}"
            ;;
        support)
            codex_install_support_files
            codex_install_launchers
            ;;
        rebuild)
            codex_rebuild
            ;;
        repair)
            codex_repair_public
            ;;
        update)
            shift || true
            codex_update "${1:-}"
            ;;
        setup)
            printf 'codex setup is reserved for configuration. Use install, update, rebuild, or repair.\n' >&2
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
