#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${BASH_SOURCE[0]%/*}"
[ "$ROOT_DIR" = "${BASH_SOURCE[0]}" ] && ROOT_DIR="."
ROOT_DIR="$(cd "$ROOT_DIR/.." && pwd)"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

usage() {
    cat <<'EOF'
Usage: bash bin/install-runtime.sh [setup|support|update|remove|doctor]

setup        Install support files, the public Codex launcher, and upstream runtime.
support      Refresh support files and the public Codex launcher only.
update       Fetch, patch, and promote the official linux-arm64 Codex runtime.
remove       Remove the managed Codex launcher/runtime and restore a launcher backup.
doctor       Run wrapper diagnostics.
EOF
}

codex_source_commit() {
    if command -v git >/dev/null 2>&1 && git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || printf 'unknown\n'
    else
        printf 'unknown\n'
    fi
}

codex_write_managed_shell() {
    mkdir -p "$CODEX_NATIVE_MANAGER_DIR"
    cat >"$CODEX_NATIVE_MANAGED_SHELL.$$" <<EOF
#!$CODEX_NATIVE_PREFIX/bin/bash
# codex native managed shell
set -euo pipefail
export CODEX_NATIVE_INSTALL_RUNTIME_SOURCE="$ROOT_DIR/bin/install-runtime.sh"
# shellcheck disable=SC1091
. "$CODEX_NATIVE_MANAGER_DIR/lib.sh"
codex_main "\$@"
EOF
    chmod 755 "$CODEX_NATIVE_MANAGED_SHELL.$$"
    mv "$CODEX_NATIVE_MANAGED_SHELL.$$" "$CODEX_NATIVE_MANAGED_SHELL"
}

codex_install_support_files() {
    local wrapper_commit
    mkdir -p "$CODEX_NATIVE_MANAGER_DIR" "$CODEX_NATIVE_STATE_DIR"
    cp "$ROOT_DIR/lib/codex-termux-lib.sh" "$CODEX_NATIVE_MANAGER_DIR/lib.sh"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/lib.sh"
    cp "$ROOT_DIR/lib/codex-termux-interactive.sh" "$CODEX_NATIVE_MANAGER_DIR/codex-termux-interactive.sh"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/codex-termux-interactive.sh"
    cp "$ROOT_DIR/lib/codex-termux-runtime.sh" "$CODEX_NATIVE_MANAGER_DIR/codex-termux-runtime.sh"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/codex-termux-runtime.sh"
    if [ -d "$ROOT_DIR/tools/codex_native" ]; then
        rm -rf "$CODEX_NATIVE_MANAGER_DIR/codex_native"
        cp -R "$ROOT_DIR/tools/codex_native" "$CODEX_NATIVE_MANAGER_DIR/codex_native"
        python3 -m py_compile "$CODEX_NATIVE_MANAGER_DIR"/codex_native/*.py
    fi
    cp "$ROOT_DIR/tools/build-runtime.py" "$CODEX_NATIVE_MANAGER_DIR/build-runtime.py"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/build-runtime.py"
    cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$CODEX_NATIVE_MANAGER_DIR/bwrap-termux-compat.py"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/bwrap-termux-compat.py"
    cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$CODEX_NATIVE_MANAGER_DIR/rg-termux-shim.sh"
    chmod 755 "$CODEX_NATIVE_MANAGER_DIR/rg-termux-shim.sh"
    if [ -f "$ROOT_DIR/config/wrapper-version.env" ]; then
        cp "$ROOT_DIR/config/wrapper-version.env" "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    else
        printf 'CODEX_NATIVE_WRAPPER_VERSION=unknown\nCODEX_NATIVE_WRAPPER_CHANNEL=unknown\nCODEX_NATIVE_WRAPPER_REPO=local/codex\n' >"$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    fi
    wrapper_commit="$(codex_source_commit)"
    {
        printf 'CODEX_NATIVE_WRAPPER_COMMIT=%s\n' "$wrapper_commit"
        printf 'CODEX_NATIVE_WRAPPER_INSTALLED_AT=%s\n' "$(date -Is)"
    } >>"$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    chmod 644 "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    codex_write_managed_shell
}

codex_launcher_available() {
    command -v clang >/dev/null 2>&1
}

codex_build_launcher() {
    clang -O2 -Wall -Wextra -o "$1" "$ROOT_DIR/tools/codex-launcher.c"
}

codex_prepare_launcher_slot() {
    local public="$1" backup base
    mkdir -p "${public%/*}" "$CODEX_NATIVE_BACKUP_DIR"
    if [ -d "$public" ] && [ ! -L "$public" ]; then
        codex_fail "refusing to replace launcher directory $public"
        return 1
    fi
    if [ -e "$public" ] || [ -L "$public" ]; then
        if codex_file_has_marker "$public"; then
            return 0
        fi
        base="$(basename "$public")"
        backup="$CODEX_NATIVE_BACKUP_DIR/$base.$(date +%Y%m%d-%H%M%S).bak"
        cp -Pp "$public" "$backup"
    fi
}

codex_write_shell_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    cat >"$tmp" <<EOF
#!/bin/sh
# $CODEX_NATIVE_MANAGED_LAUNCHER_MARKER
exec "$CODEX_NATIVE_MANAGED_SHELL" "\$@"
EOF
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public"; then
        rm -f "$tmp"
        return 1
    fi
    if ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_write_compiled_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    codex_build_launcher "$tmp"
    if ! grep -a -q "$CODEX_NATIVE_MANAGED_LAUNCHER_MARKER" "$tmp"; then
        rm -f "$tmp"
        codex_fail "compiled launcher missing managed marker"
        return 1
    fi
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public"; then
        rm -f "$tmp"
        return 1
    fi
    if ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_install_launchers() {
    if codex_launcher_available; then
        codex_write_compiled_launcher "$CODEX_NATIVE_PUBLIC_CODEX"
    else
        codex_write_shell_launcher "$CODEX_NATIVE_PUBLIC_CODEX"
    fi
}

codex_setup() {
    codex_validate_runtime_retention || return $?
    codex_install_support_files
    codex_install_launchers
    codex_migrate_legacy_runtime_layout
    if ! codex_runtime_ok; then
        if [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ]; then
            codex_repair_runtime_from_raw
        else
            codex_update "${1:-}"
        fi
    fi
    codex_bootstrap_store
    codex_refresh_runtime_metadata
    codex_version
}

main() {
    case "${1:-setup}" in
        setup)
            shift || true
            codex_setup "${1:-}"
            ;;
        support)
            codex_install_support_files
            codex_install_launchers
            ;;
        update)
            shift || true
            codex_update "${1:-}"
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
