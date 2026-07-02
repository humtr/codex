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
bash -lc '. "$1"; [ "$(codex_termux_cmd parent-dir --path /a/b/c)" = "/a/b" ]; [ "$(codex_termux_cmd parent-dir --path file)" = "file" ]; [ "$(codex_termux_cmd strip-trailing-slashes --path /a/b///)" = "/a/b" ]' _ "$ROOT_DIR/lib/codex-termux.sh" \
    || fail 'Python path normalization helpers changed behavior'

helper_root="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli helper-package-root \
            --source-root "$ROOT_DIR" \
            --root-dir "" \
            --manager-dir "$TMP_ROOT/manager"
)"
[ "$helper_root" = "$ROOT_DIR/tools" ] || fail "helper package root mismatch: $helper_root"

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_TMPDIR="$TMP_ROOT/prefix/tmp" \
CODEX_TERMUX_WRAPPER_ROOT="$ROOT_DIR" \
TMPDIR=/tmp \
bash -lc '. "$1"; [ "$(codex_termux_package_root)" = "$2/tools" ]' _ "$ROOT_DIR/lib/codex-termux.sh" "$ROOT_DIR" \
    || fail 'codex_termux_package_root did not delegate helper root selection'

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

CODEX_TERMUX_HOME="$TMP_ROOT/home" \
CODEX_TERMUX_PREFIX="$TMP_ROOT/prefix" \
CODEX_TERMUX_TMPDIR="$TMP_ROOT/prefix/tmp" \
TMPDIR=/tmp \
bash -lc '. "$1"; codex_assert_managed_tree_target "$CODEX_TERMUX_ROOT/store" managed; codex_assert_managed_tree_target "$CODEX_TERMUX_STATE_DIR/registry" state; ! codex_assert_managed_tree_target "$CODEX_TERMUX_HOME" home >/dev/null 2>&1; ! codex_assert_managed_tree_target "$CODEX_TERMUX_TMPDIR" tmp >/dev/null 2>&1; ! codex_assert_managed_tree_target "$CODEX_TERMUX_HOME/outside" outside >/dev/null 2>&1' _ "$ROOT_DIR/lib/codex-termux.sh" \
    || fail 'managed path guard did not enforce Python policy'

PREFIX="$TMP_ROOT/prefix" TMPDIR=/tmp bash -lc '. "$1"; dir="$(install_tmp_dir)"; [ "$dir" = "$PREFIX/tmp" ]' _ "$ROOT_DIR/install.sh" \
    || fail 'install_tmp_dir did not avoid /tmp'

printf 'tmp-paths: ok\n'
