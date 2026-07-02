#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="$ROOT_DIR/.tmp"
mkdir -p "$TMP_PARENT"
TMP_DIR="$(mktemp -d "$TMP_PARENT/codex-termux-smoke-contracts.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'termux-smoke-contracts: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$TMP_DIR/bin"
{
    printf '#!%s\n' "$(command -v bash)"
    cat <<'SCRIPT'
set -euo pipefail

case "$*" in
    "termux version")
        printf 'codex-cli 0.142.4 (test)\n'
        printf 'runtime   2026-07-02\n'
        printf 'wrapper   %s (%s)\n' "$CODEX_TERMUX_EXPECT_WRAPPER_VERSION" "$CODEX_TERMUX_EXPECT_WRAPPER_COMMIT"
        ;;
    "termux doctor --json")
        python3 - "$CODEX_TERMUX_EXPECT_WRAPPER_VERSION" "$CODEX_TERMUX_EXPECT_WRAPPER_COMMIT" <<'PYTHON'
import json
import sys

version, commit = sys.argv[1:3]
tuple_id = f"raw-test__wrapper-{version}-{commit}"
print(json.dumps({
    "overallStatus": "ok",
    "wrapper": {
        "version": version,
        "commit": commit,
    },
    "activeTupleId": tuple_id,
    "verifiedTupleId": tuple_id,
    "checks": {
        "raw_hash": True,
        "runtime_hash": True,
        "registry_active_tuple": True,
    },
}))
PYTHON
        ;;
    "--version")
        printf 'codex-cli 0.142.4\n'
        ;;
    "termux install rebuild")
        {
            printf 'CODEX_TERMUX_AUTO_UPDATE=%s\n' "${CODEX_TERMUX_AUTO_UPDATE-}"
            printf 'CODEX_TERMUX_INSTALL_RUNTIME_SOURCE=%s\n' "${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE-}"
            printf 'CODEX_TERMUX_WRAPPER_SOURCE_CONFIG=%s\n' "${CODEX_TERMUX_WRAPPER_SOURCE_CONFIG-}"
            printf 'CODEX_TERMUX_WRAPPER_SOURCE_DIR=%s\n' "${CODEX_TERMUX_WRAPPER_SOURCE_DIR-}"
        } >"$CODEX_TERMUX_SMOKE_ENV_CAPTURE"
        printf 'rebuild ok\n'
        ;;
    *)
        printf 'unexpected codex call: %s\n' "$*" >&2
        exit 97
        ;;
esac
SCRIPT
} >"$TMP_DIR/bin/codex"
chmod +x "$TMP_DIR/bin/codex"

set +e
PREFIX=/data/data/com.termux/files/usr \
PATH="$TMP_DIR/bin:$PATH" \
CODEX_TERMUX_EXPECT_WRAPPER_VERSION=260702-test \
CODEX_TERMUX_EXPECT_WRAPPER_COMMIT=abcdef123456 \
CODEX_TERMUX_AUTO_UPDATE=1 \
bash "$ROOT_DIR/tests/run-termux.sh" >"$TMP_DIR/autoupdate.out" 2>"$TMP_DIR/autoupdate.err"
auto_update_status=$?
set -e
[ "$auto_update_status" -ne 0 ] || fail 'run-termux accepted CODEX_TERMUX_AUTO_UPDATE=1'
grep -F 'requires CODEX_TERMUX_AUTO_UPDATE=0' "$TMP_DIR/autoupdate.err" >/dev/null \
    || fail 'run-termux did not explain network-disabled auto-update requirement'

PREFIX=/data/data/com.termux/files/usr \
PATH="$TMP_DIR/bin:$PATH" \
CODEX_TERMUX_EXPECT_WRAPPER_VERSION=260702-test \
CODEX_TERMUX_EXPECT_WRAPPER_COMMIT=abcdef123456 \
CODEX_TERMUX_AUTO_UPDATE=0 \
CODEX_TERMUX_RUN_REBUILD_SMOKE=1 \
CODEX_TERMUX_SMOKE_ENV_CAPTURE="$TMP_DIR/rebuild.env" \
    bash "$ROOT_DIR/tests/run-termux.sh" >"$TMP_DIR/rebuild.out" 2>"$TMP_DIR/rebuild.err" || {
        cat "$TMP_DIR/rebuild.out" >&2 || true
        cat "$TMP_DIR/rebuild.err" >&2 || true
        fail 'run-termux rebuild smoke contract failed'
    }

grep -Fx "CODEX_TERMUX_AUTO_UPDATE=0" "$TMP_DIR/rebuild.env" >/dev/null \
    || fail 'rebuild smoke did not keep auto-update disabled'
grep -Fx "CODEX_TERMUX_INSTALL_RUNTIME_SOURCE=$ROOT_DIR/bin/install-runtime.sh" "$TMP_DIR/rebuild.env" >/dev/null \
    || fail 'rebuild smoke did not use local install-runtime source'
grep -Fx "CODEX_TERMUX_WRAPPER_SOURCE_DIR=$ROOT_DIR" "$TMP_DIR/rebuild.env" >/dev/null \
    || fail 'rebuild smoke did not bind wrapper source to checkout root'
grep -E '^CODEX_TERMUX_WRAPPER_SOURCE_CONFIG=.+/wrapper-source\.env$' "$TMP_DIR/rebuild.env" >/dev/null \
    || fail 'rebuild smoke did not isolate wrapper source config'

printf 'termux-smoke-contracts: ok\n'
