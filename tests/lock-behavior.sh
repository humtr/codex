#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'lock-behavior: FAIL: %s\n' "$*" >&2
    exit 1
}

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

path_bin="$fixture_root/path-bin"
mkdir -p "$path_bin"
mkdir_bin="$(command -v mkdir)"
sleep_bin="$(command -v sleep)"
rmdir_bin="$(command -v rmdir)"
rm_bin="$(command -v rm)"
ln -s "$mkdir_bin" "$path_bin/mkdir"
ln -s "$sleep_bin" "$path_bin/sleep"
ln -s "$rmdir_bin" "$path_bin/rmdir"
ln -s "$rm_bin" "$path_bin/rm"

export PATH="$path_bin"
export CODEX_NATIVE_LOCK_FILE="$fixture_root/native.lock"
export CODEX_NATIVE_STATE_DIR="$fixture_root/state"
export CODEX_NATIVE_LOCK_WAIT_SECONDS=1
mkdir -p "$CODEX_NATIVE_STATE_DIR"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

locked_path="$fixture_root/locked.txt"
write_locked() {
    printf 'ok\n' >"$1"
}

codex_with_lock write_locked "$locked_path"
[ -f "$locked_path" ] || fail "fallback lock did not run command"
[ ! -e "${CODEX_NATIVE_LOCK_FILE}.d" ] || fail "fallback lock directory was not cleaned up"

mkdir -p "${CODEX_NATIVE_LOCK_FILE}.d"
if codex_with_lock write_locked "$fixture_root/should-not-exist.txt" >/dev/null 2>&1; then
    fail "lock contention did not time out"
fi
rm -rf "${CODEX_NATIVE_LOCK_FILE}.d"

printf 'lock-behavior: ok\n'
