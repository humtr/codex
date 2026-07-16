#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-install-dispatch-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'install-dispatch: FAIL: %s\n' "$*" >&2
    exit 1
}

# shellcheck disable=SC1090
. "$ROOT_DIR/bin/install-runtime.sh"

USAGE_CALLED=0
FAILED_MESSAGE=""
UPSTREAM_ARG="__unset__"
SUPPORT_COUNT=0
LAUNCHER_COUNT=0
CACHED_COUNT=0
REPAIR_COUNT=0
VERSION_COUNT=0
STATUS_LOG=""
SAY_LOG=""

usage() { USAGE_CALLED=1; }
codex_fail() { FAILED_MESSAGE="$*"; return 1; }
codex_with_lock() { local cmd="$1"; shift; "$cmd" "$@"; }
codex_prepare_fresh_wrapper_source() { return 0; }
codex_cleanup_fresh_wrapper_source() { return 0; }
codex_validate_runtime_retention() { return 0; }
codex_install_support_files() { SUPPORT_COUNT=$((SUPPORT_COUNT + 1)); }
codex_install_launchers() { LAUNCHER_COUNT=$((LAUNCHER_COUNT + 1)); }
codex_runtime_install_upstream() { UPSTREAM_ARG="${1:-}"; }
codex_runtime_install_cached() { CACHED_COUNT=$((CACHED_COUNT + 1)); }
codex_refresh_runtime_metadata() { return 0; }
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); }
codex_repair_core_unlocked() { REPAIR_COUNT=$((REPAIR_COUNT + 1)); }
codex_status() { STATUS_LOG="${STATUS_LOG}${STATUS_LOG:+|}$*"; }
codex_status_clear() { STATUS_LOG="${STATUS_LOG}${STATUS_LOG:+|}<clear>"; }
codex_say() { SAY_LOG="${SAY_LOG}${SAY_LOG:+|}$*"; }

codex_install_run_plan install upstream --help
[ "$USAGE_CALLED" -eq 1 ] || fail 'install upstream --help did not show usage'
[ "$UPSTREAM_ARG" = "__unset__" ] || fail 'install upstream --help reached upstream install'

if codex_install_run_plan install upstream --bad-option; then
    fail 'install upstream accepted an option-looking version'
fi
case "$FAILED_MESSAGE" in
    *"must not start with '-'"*) ;;
    *) fail "unexpected upstream option error: $FAILED_MESSAGE" ;;
esac

if codex_install_run_plan install upstream 0.1.0 extra; then
    fail 'install upstream accepted extra arguments'
fi
case "$FAILED_MESSAGE" in
    *"at most one version"*) ;;
    *) fail "unexpected upstream arity error: $FAILED_MESSAGE" ;;
esac

SUPPORT_COUNT=0; LAUNCHER_COUNT=0; CACHED_COUNT=0; REPAIR_COUNT=0; VERSION_COUNT=0
STATUS_LOG=""; SAY_LOG=""
codex_install_run_plan install rebuild
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'install rebuild did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'install rebuild did not refresh launcher'
[ "$CACHED_COUNT" -eq 1 ] || fail 'install rebuild did not rebuild cached runtime'
[ "$REPAIR_COUNT" -eq 0 ] || fail 'install rebuild called repair'
[ "$VERSION_COUNT" -eq 1 ] || fail 'install rebuild did not render version from surface'
case "$STATUS_LOG" in
    "Rebuilding runtime from cached raw package"*) ;;
    *) fail "install rebuild did not use rebuild surface: $STATUS_LOG" ;;
esac

SUPPORT_COUNT=0; LAUNCHER_COUNT=0; VERSION_COUNT=0; STATUS_LOG=""; SAY_LOG=""
codex_install_run_plan install support
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'install support did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'install support did not refresh launcher'
[ "$VERSION_COUNT" -eq 0 ] || fail 'install support should not render version'
case "$STATUS_LOG" in
    "Installing wrapper support and launcher"*) ;;
    *) fail "install support did not use support surface: $STATUS_LOG" ;;
esac
[ "$SAY_LOG" = "Support files and launcher are ready" ] \
    || fail "install support did not render support completion: $SAY_LOG"

REPAIR_COUNT=0; VERSION_COUNT=0; STATUS_LOG=""; SAY_LOG=""
main repair
[ "$REPAIR_COUNT" -eq 1 ] || fail 'repair did not call repair core'
[ "$VERSION_COUNT" -eq 1 ] || fail 'repair did not render version from surface'
case "$STATUS_LOG" in
    "Repairing managed installation"*) ;;
    *) fail "repair did not use repair surface: $STATUS_LOG" ;;
esac

DOCTOR_ARGS=""
codex_termux_doctor() { DOCTOR_ARGS="$*"; }
main doctor --json
[ "$DOCTOR_ARGS" = "--json" ] || fail "doctor dispatch mismatch: $DOCTOR_ARGS"

VERSION_COUNT=0; STATUS_LOG=""; SAY_LOG=""
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); return 7; }
if codex_install_surface_run install "" codex_validate_runtime_retention; then
    fail 'surface finish failure did not propagate'
fi
codex_version() { VERSION_COUNT=$((VERSION_COUNT + 1)); }

SUPPORT_COUNT=0; LAUNCHER_COUNT=0; VERSION_COUNT=0; STATUS_LOG=""; SAY_LOG=""
CODEX_TERMUX_INSTALL_SURFACE=0 codex_install_run_plan install support
[ "$SUPPORT_COUNT" -eq 1 ] || fail 'quiet support did not refresh support'
[ "$LAUNCHER_COUNT" -eq 1 ] || fail 'quiet support did not refresh launcher'
[ -z "$STATUS_LOG" ] || fail "quiet support emitted status: $STATUS_LOG"
[ -z "$SAY_LOG" ] || fail "quiet support emitted completion: $SAY_LOG"
unset CODEX_TERMUX_INSTALL_SURFACE

curl() {
    local out="" arg
    printf '%s\n' "$*" >"$TMP_DIR/curl-args"
    while [ "$#" -gt 0 ]; do
        arg="$1"; shift
        if [ "$arg" = "-o" ]; then out="${1:-}"; shift || true; fi
    done
    [ -n "$out" ] || fail 'mock curl did not receive output path'
    printf 'archive\n' >"$out"
}
CODEX_TERMUX_WRAPPER_TOKEN="test-token" \
    codex_download_wrapper_archive \
        "https://api.github.com/repos/example/private/releases/assets/123" \
        "$TMP_DIR/release.tgz"
grep -F 'Authorization: Bearer test-token' "$TMP_DIR/curl-args" >/dev/null \
    || fail 'release token was not passed to curl'
grep -F 'Accept: application/octet-stream' "$TMP_DIR/curl-args" >/dev/null \
    || fail 'release asset accept header missing'

mkdir -p "$TMP_DIR/gh-bin"
cat >"$TMP_DIR/gh-bin/gh" <<'SCRIPT'
#!/bin/sh
[ "$1" = auth ] && [ "$2" = token ] || exit 3
printf '%s\n' gh_token_test
SCRIPT
chmod 755 "$TMP_DIR/gh-bin/gh"
PATH="$TMP_DIR/gh-bin:$PATH" \
CODEX_TERMUX_WRAPPER_TOKEN= \
CODEX_TERMUX_WRAPPER_GIT_TOKEN= \
CODEX_TERMUX_WRAPPER_RELEASE_TOKEN= \
GITHUB_TOKEN= \
    bash -c '. "$1"; [ "$(codex_wrapper_auth_token)" = gh_token_test ]' \
        _ "$ROOT_DIR/bin/install-runtime.sh" \
    || fail 'gh auth token fallback failed'

git() {
    local target="${@: -1}" arg
    printf '%s\n' "$*" >"$TMP_DIR/git-args"
    printf '%s\n' "${GIT_TERMINAL_PROMPT:-}" >"$TMP_DIR/git-terminal-prompt"
    printf '%s\n' "${GIT_ASKPASS:-}" >"$TMP_DIR/git-askpass-path"
    printf '%s\n' "${CODEX_TERMUX_WRAPPER_GIT_TOKEN_VALUE:-}" >"$TMP_DIR/git-token-value"
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$target" <<'PYTHON'
import sys
from pathlib import Path
from wrapper import source
root = Path(sys.argv[1])
for relative in source.REQUIRED_WRAPPER_SOURCE_PATHS:
    path = root / relative
    if relative == "tools/codex_termux":
        path.mkdir(parents=True, exist_ok=True)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fixture\n", encoding="utf-8")
PYTHON
    for arg in "$@"; do
        [ "$arg" != test-token ] || fail 'git token leaked into command arguments'
    done
}

CODEX_TERMUX_WRAPPER_SOURCE_TMP=""
CODEX_TERMUX_WRAPPER_SOURCE_DIR=""
CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/prefix/tmp" \
CODEX_TERMUX_WRAPPER_REPO="example/private" \
CODEX_TERMUX_WRAPPER_REF="main" \
CODEX_TERMUX_WRAPPER_TOKEN="test-token" \
    codex_git_clone_wrapper_source
wrapper_cmd validate-wrapper-source --root "$CODEX_TERMUX_WRAPPER_SOURCE_DIR" >/dev/null \
    || fail 'mock git checkout was not accepted as wrapper source'
grep -F 'https://github.com/example/private.git' "$TMP_DIR/git-args" >/dev/null \
    || fail 'repo shorthand did not expand'
grep -F -- '--branch main' "$TMP_DIR/git-args" >/dev/null \
    || fail 'git ref was not passed as clone branch'
grep -Fx 0 "$TMP_DIR/git-terminal-prompt" >/dev/null \
    || fail 'git prompts were not disabled'
[ -x "$(cat "$TMP_DIR/git-askpass-path")" ] || fail 'git askpass helper missing'
grep -Fx test-token "$TMP_DIR/git-token-value" >/dev/null \
    || fail 'git token was not passed through environment'
if grep -F test-token "$(cat "$TMP_DIR/git-askpass-path")" >/dev/null; then
    fail 'git token was written into askpass helper'
fi

mkdir -p "$TMP_DIR/incomplete/bin"
printf 'install\n' >"$TMP_DIR/incomplete/install.sh"
printf 'install runtime\n' >"$TMP_DIR/incomplete/bin/install-runtime.sh"
if wrapper_cmd validate-wrapper-source --root "$TMP_DIR/incomplete" >/dev/null; then
    fail 'incomplete wrapper source passed validation'
fi
FAILED_MESSAGE=""
if codex_require_wrapper_source "$TMP_DIR/incomplete" 'Wrapper git repository'; then
    fail 'incomplete wrapper source was accepted'
fi
case "$FAILED_MESSAGE" in
    *missing:*shell/loader.sh*src/wrapper/cli.py*libexec/notify*) ;;
    *) fail "missing source error was not role-oriented: $FAILED_MESSAGE" ;;
esac

printf 'install-dispatch: ok\n'
