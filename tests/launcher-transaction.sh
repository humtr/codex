#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'launcher-transaction: FAIL: %s\n' "$*" >&2
    exit 1
}

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

export CODEX_NATIVE_STATE_DIR="$fixture_root/state"
export CODEX_NATIVE_BACKUP_DIR="$fixture_root/backups"
export CODEX_NATIVE_PREFIX="$fixture_root/prefix"
export CODEX_NATIVE_PUBLIC_CODEX="$CODEX_NATIVE_PREFIX/bin/codex"
export CODEX_NATIVE_MANAGED_SHELL="$fixture_root/managed.sh"
mkdir -p "$CODEX_NATIVE_STATE_DIR" "$CODEX_NATIVE_BACKUP_DIR" "$CODEX_NATIVE_PREFIX/bin"
printf '#!/bin/sh\necho original\n' >"$CODEX_NATIVE_PUBLIC_CODEX"
chmod 755 "$CODEX_NATIVE_PUBLIC_CODEX"
printf '#!/bin/sh\nexit 0\n' >"$CODEX_NATIVE_MANAGED_SHELL"
chmod 755 "$CODEX_NATIVE_MANAGED_SHELL"

# shellcheck disable=SC1091
. "$ROOT_DIR/bin/install-runtime.sh"

codex_build_launcher() { return 1; }
if codex_write_compiled_launcher "$CODEX_NATIVE_PUBLIC_CODEX" >/dev/null 2>&1; then
    fail "launcher build failure was not propagated"
fi
[ "$(cat "$CODEX_NATIVE_PUBLIC_CODEX")" = '#!/bin/sh
echo original' ] || fail "public launcher changed after build failure"

codex_build_launcher() {
    printf '#!/bin/sh\nexit 0\n' >"$1"
}
if codex_write_compiled_launcher "$CODEX_NATIVE_PUBLIC_CODEX" >/dev/null 2>&1; then
    fail "markerless launcher was accepted"
fi
[ "$(cat "$CODEX_NATIVE_PUBLIC_CODEX")" = '#!/bin/sh
echo original' ] || fail "public launcher changed after marker validation failure"

mv() { return 1; }
if codex_write_shell_launcher "$CODEX_NATIVE_PUBLIC_CODEX" >/dev/null 2>&1; then
    fail "launcher final rename failure was not propagated"
fi
unset -f mv
[ "$(cat "$CODEX_NATIVE_PUBLIC_CODEX")" = '#!/bin/sh
echo original' ] || fail "public launcher changed after final rename failure"

printf 'launcher-transaction: ok\n'
