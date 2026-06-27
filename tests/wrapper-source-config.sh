#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${PREFIX:-/data/data/com.termux/files/usr}/tmp}"
TMP_DIR="$TMP_PARENT/codex-wrapper-source-config-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'wrapper-source-config: FAIL: %s\n' "$*" >&2
    exit 1
}

file_mode() {
    stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

CONFIG="$TMP_DIR/wrapper-source.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG" \
    CODEX_TERMUX_WRAPPER_GIT_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source example/private dev >/dev/null \
    || fail 'install.sh source failed'

[ -f "$CONFIG" ] || fail 'source config was not created'
[ "$(file_mode "$CONFIG")" = "600" ] || fail "source config mode is not 600: $(file_mode "$CONFIG")"

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "github_pat_test" ]
' _ "$CONFIG" || fail 'source config did not restore saved values'

CODEX_TERMUX_WRAPPER_GIT_REPO="override/repo" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "override/repo" ]
' _ "$CONFIG" || fail 'explicit environment did not override saved repo'

CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "github_pat_test" ]
' _ "$ROOT_DIR/bin/install-runtime.sh" || fail 'install-runtime did not auto-load source config'

CONFIG_DEFAULT_REF="$TMP_DIR/wrapper-source-default-ref.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_DEFAULT_REF" \
    CODEX_TERMUX_WRAPPER_GIT_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source example/private "" >/dev/null \
    || fail 'install.sh source with empty ref failed'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "main" ]
' _ "$CONFIG_DEFAULT_REF" || fail 'empty source ref did not default to main'

printf 'wrapper-source-config: ok\n'
