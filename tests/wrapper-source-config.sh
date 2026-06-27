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
    bash "$ROOT_DIR/install.sh" source --save-only example/private dev >/dev/null \
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
    bash "$ROOT_DIR/install.sh" source --save-only example/private "" >/dev/null \
    || fail 'install.sh source with empty ref failed'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "main" ]
' _ "$CONFIG_DEFAULT_REF" || fail 'empty source ref did not default to main'

bash -lc '
    . "$1"
    CONFIGURE_ARGS=""
    INSTALL_CALLED=0
    configure_wrapper_source() { CONFIGURE_ARGS="$*"; }
    install_managed_runtime() { INSTALL_CALLED=1; [ "$#" -eq 0 ]; }
    main source example/private dev
    [ "$CONFIGURE_ARGS" = "example/private dev" ] &&
    [ "$INSTALL_CALLED" -eq 1 ]
' _ "$ROOT_DIR/install.sh" || fail 'source command did not continue into install by default'

bash -lc '
    . "$1"
    INSTALL_CALLED=0
    configure_wrapper_source() { return 0; }
    install_managed_runtime() { INSTALL_CALLED=1; }
    main source --save-only example/private dev
    [ "$INSTALL_CALLED" -eq 0 ]
' _ "$ROOT_DIR/install.sh" || fail 'source --save-only continued into install'

printf 'wrapper-source-config: ok\n'
