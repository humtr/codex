#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_ROOT"' EXIT

fail() {
    printf 'tmp-paths: FAIL: %s\n' "$*" >&2
    exit 1
}

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_TMPDIR="$TMP_ROOT/prefix/tmp" \
TMPDIR=/tmp \
bash -lc '. "$1"; tmp="$(codex_mktemp_dir codex-test)" || exit 1; case "$tmp" in "$CODEX_TERMUX_TMPDIR"/*) ;; *) printf "%s\n" "$tmp"; exit 2 ;; esac' _ "$ROOT_DIR/lib/codex-termux.sh" \
    || fail 'codex_mktemp_dir did not use CODEX_TERMUX_TMPDIR'

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_TMPDIR="$TMP_ROOT/prefix/tmp" \
TMPDIR=/tmp \
bash -lc '. "$1"; tmp="$(codex_mktemp_file codex-session)" || exit 1; case "$tmp" in "$CODEX_TERMUX_TMPDIR"/*) ;; *) printf "%s\n" "$tmp"; exit 2 ;; esac' _ "$ROOT_DIR/lib/codex-termux.sh" \
    || fail 'codex_mktemp_file did not use CODEX_TERMUX_TMPDIR'

PREFIX="$TMP_ROOT/prefix" TMPDIR=/tmp bash -lc '. "$1"; dir="$(install_tmp_dir)"; [ "$dir" = "$PREFIX/tmp" ]' _ "$ROOT_DIR/install.sh" \
    || fail 'install_tmp_dir did not avoid /tmp'

printf 'tmp-paths: ok\n'
