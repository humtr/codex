#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"
SH_BIN="$(command -v sh)"
PWD_BIN="$(command -v pwd)"

fail() {
    printf 'runtime-bwrap-path: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/runtime-bwrap-path-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export PREFIX="$FIXTURE_ROOT/prefix"
mkdir -p "$PREFIX/bin"
printf '#!/bin/sh\nprintf external-bwrap\\n\n' >"$PREFIX/bin/bwrap"
chmod 755 "$PREFIX/bin/bwrap"
external_bwrap_before="$(sha256sum "$PREFIX/bin/bwrap")"

bash "$ROOT_DIR/bin/install-runtime.sh" support

[ -x "$PREFIX/bin/codex" ] || fail "public codex launcher was not installed"
[ "$(sha256sum "$PREFIX/bin/bwrap")" = "$external_bwrap_before" ] \
    || fail "support changed the public bwrap"

# shellcheck disable=SC1091
. "$CODEX_NATIVE_HOME/.local/lib/codex/native/runtime/lib.sh"
runtime_dir="$CODEX_NATIVE_RUNTIME_DIR"
mkdir -p "$runtime_dir/codex-path" "$runtime_dir/codex-resources"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$runtime_dir/codex-path/bwrap"
chmod 755 "$runtime_dir/codex-path/bwrap"

codex_prepare_runtime_env
[ "$(command -v bwrap)" = "$runtime_dir/codex-path/bwrap" ] \
    || fail "runtime-private bwrap is not first on PATH"

compat=(python "$ROOT_DIR/tools/bwrap-termux-compat.py")
[ "$("${compat[@]}" --version)" = "bubblewrap termux compat for Codex" ] \
    || fail "compat version probe failed"
[ "$("${compat[@]}" --setenv BWRAP_TEST yes -- "$SH_BIN" -c 'printf %s "$BWRAP_TEST"')" = "yes" ] \
    || fail "compat --setenv was not applied"
[ "$(BWRAP_TEST=present "${compat[@]}" --unsetenv BWRAP_TEST -- "$SH_BIN" -c 'printf %s "${BWRAP_TEST-unset}"')" = "unset" ] \
    || fail "compat --unsetenv was not applied"
[ "$("${compat[@]}" --clearenv --setenv BWRAP_TEST yes -- "$SH_BIN" -c 'printf %s "$BWRAP_TEST"')" = "yes" ] \
    || fail "compat --clearenv was not applied"
[ "$("${compat[@]}" --argv0 bwrap-test-argv0 -- "$SH_BIN" -c 'printf %s "$0"')" = "bwrap-test-argv0" ] \
    || fail "compat --argv0 was not applied"
mkdir -p "$FIXTURE_ROOT/cwd"
[ "$("${compat[@]}" --chdir "$FIXTURE_ROOT/cwd" -- "$PWD_BIN")" = "$FIXTURE_ROOT/cwd" ] \
    || fail "compat --chdir was not applied"
printf '%s\0' --setenv BWRAP_ARGS yes -- "$SH_BIN" -c 'printf %s "$BWRAP_ARGS"' \
    >"$FIXTURE_ROOT/args"
exec 9<"$FIXTURE_ROOT/args"
[ "$("${compat[@]}" --args 9)" = "yes" ] || fail "compat --args was not applied"
exec 9<&-
if "${compat[@]}" /bin/true >/dev/null 2>&1; then
    fail "compat accepted argv without -- separator"
fi

codex_remove >/dev/null
[ ! -e "$PREFIX/bin/codex" ] || fail "remove kept the managed public Codex launcher"
[ "$(sha256sum "$PREFIX/bin/bwrap")" = "$external_bwrap_before" ] \
    || fail "remove changed the public bwrap"

printf 'runtime-bwrap-path: ok\n'
