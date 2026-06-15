#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHONPATH="$ROOT_DIR/tools"
export PYTHONPATH

FIXTURE_ROOT="${TMPDIR:-/tmp}/codex-state-registry.$$"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT
mkdir -p "$FIXTURE_ROOT"

cli() {
    python3 -m codex_native.cli "$@"
}

fail() {
    printf 'state-registry: %s\n' "$*" >&2
    exit 1
}

STATE_FILE="$FIXTURE_ROOT/state/state.json"
REGISTRY_FILE="$FIXTURE_ROOT/state/registry.json"
RUNTIME_STORE="$FIXTURE_ROOT/store/runtime"
RUNTIME_PATH="$RUNTIME_STORE/runtime-one"
RAW_PATH="$FIXTURE_ROOT/store/raw/raw-one/vendor/aarch64-unknown-linux-musl/bin/codex"
mkdir -p "$RUNTIME_PATH" "$(dirname "$RAW_PATH")"
printf '#!/bin/sh\n' >"$RUNTIME_PATH/codex"
printf '#!/bin/sh\n' >"$RAW_PATH"

VERSION="1.2.3"
RAW_SHA="aaaaaaaaaaaa0000000000000000000000000000000000000000000000000000"
RUNTIME_SHA="cccccccccccc0000000000000000000000000000000000000000000000000000"
PACKAGE_SPEC="@openai/codex@1.2.3"
WRAPPER_VERSION="0.1.0"
WRAPPER_COMMIT="bbbbbbbbbbbb0000000000000000000000000000"
UPDATED_AT="2026-06-15T00:00:00Z"
SMOKE_TESTED_AT="2026-06-15T00:01:00Z"
TUPLE_ID="raw-1.2.3-aaaaaaaaaaaa__wrapper-0.1.0-bbbbbbbbbbbb"

cli state-write \
    --state-file "$STATE_FILE" \
    --version "$VERSION" \
    --raw-sha256 "$RAW_SHA" \
    --runtime-sha256 "$RUNTIME_SHA" \
    --package-spec "$PACKAGE_SPEC" \
    --active-tuple-id "$TUPLE_ID" \
    --wrapper-version "$WRAPPER_VERSION" \
    --wrapper-commit "$WRAPPER_COMMIT" \
    --updated-at "$UPDATED_AT" \
    --verified-tuple-id "" \
    --verified-at ""

[ "$(cli state-read-field --state-file "$STATE_FILE" --field version)" = "$VERSION" ] ||
    fail "state read returned wrong version"
[ "$(cli state-read-field --state-file "$STATE_FILE" --field absent)" = "" ] ||
    fail "absent state field did not print empty output"
python3 -c 'import json, stat, sys; from pathlib import Path; path = Path(sys.argv[1]); data = path.read_text(); assert data == json.dumps(json.loads(data), ensure_ascii=True, sort_keys=True) + "\n"; assert stat.S_IMODE(path.stat().st_mode) == 0o600' "$STATE_FILE" ||
    fail "state file was not sorted ASCII JSON with mode 0600"

[ "$(cli state-read-field --state-file "$FIXTURE_ROOT/missing-state.json" --field version)" = "" ] ||
    fail "missing state read did not print empty output"

printf '{bad json\n' >"$FIXTURE_ROOT/malformed-state.json"
if cli state-read-field --state-file "$FIXTURE_ROOT/malformed-state.json" --field version >/dev/null 2>&1; then
    fail "malformed state read succeeded"
fi

printf '{"schema":2,"version":"bad","active_tuple_id":"tuple"}\n' >"$FIXTURE_ROOT/wrong-schema-state.json"
if cli state-read-field --state-file "$FIXTURE_ROOT/wrong-schema-state.json" --field version >/dev/null 2>&1; then
    fail "wrong-schema state read succeeded"
fi

printf '{"schema":3,"active_tuple_id":"tuple"}\n' >"$FIXTURE_ROOT/missing-required-state.json"
if cli state-read-field --state-file "$FIXTURE_ROOT/missing-required-state.json" --field version >/dev/null 2>&1; then
    fail "state read with missing required field succeeded"
fi

python3 -c 'import json, sys; from pathlib import Path; p = Path(sys.argv[1]); data = json.loads(p.read_text()); del data["verified_tuple_id"]; del data["verified_at"]; p.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n")' "$STATE_FILE"
[ "$(cli state-read-field --state-file "$STATE_FILE" --field version)" = "$VERSION" ] ||
    fail "state read failed when optional verified fields were missing"

if cli state-write \
    --state-file "$FIXTURE_ROOT/bad-state.json" \
    --raw-sha256 "$RAW_SHA" \
    --runtime-sha256 "$RUNTIME_SHA" \
    --package-spec "$PACKAGE_SPEC" \
    --active-tuple-id "" \
    --wrapper-version "$WRAPPER_VERSION" \
    --wrapper-commit "$WRAPPER_COMMIT" \
    --updated-at "$UPDATED_AT" \
    --verified-tuple-id "" \
    --verified-at "" >/dev/null 2>&1; then
    fail "state write without required version argument succeeded"
fi

tuple="$(
    cli registry-record \
        --registry-file "$REGISTRY_FILE" \
        --version "$VERSION" \
        --raw-sha256 "$RAW_SHA" \
        --runtime-sha256 "$RUNTIME_SHA" \
        --package-spec "$PACKAGE_SPEC" \
        --runtime-path "$RUNTIME_PATH" \
        --wrapper-version "$WRAPPER_VERSION" \
        --wrapper-commit "$WRAPPER_COMMIT" \
        --runtime-store-dir "$RUNTIME_STORE" \
        --updated-at "$UPDATED_AT" \
        --smoke-tested-at "$SMOKE_TESTED_AT" \
        --raw-path "$RAW_PATH"
)"
[ "$tuple" = "$TUPLE_ID" ] || fail "registry record printed wrong tuple id"
python3 -c 'import json, stat, sys; from pathlib import Path; path = Path(sys.argv[1]); data = json.loads(path.read_text()); assert data["schema"] == 3; assert data["active_tuple_id"] == sys.argv[2]; assert data["verified_tuple_id"] == sys.argv[2]; assert len(data["installs"]) == 1; assert data["runtime"][sys.argv[2]]["smoke_tested_at"] == sys.argv[3]; assert stat.S_IMODE(path.stat().st_mode) == 0o600' "$REGISTRY_FILE" "$TUPLE_ID" "$SMOKE_TESTED_AT" ||
    fail "registry record did not create expected schema 3 file"

[ "$(cli registry-tuple-for-runtime-path --registry-file "$REGISTRY_FILE" --runtime-path "$RUNTIME_PATH")" = "$TUPLE_ID" ] ||
    fail "registry tuple lookup failed"
expected_fields="${VERSION}"$'\037'"${RAW_SHA}"$'\037'"${RUNTIME_SHA}"$'\037'"${PACKAGE_SPEC}"
[ "$(cli registry-tuple-state-fields --registry-file "$REGISTRY_FILE" --tuple-id "$TUPLE_ID")" = "$expected_fields" ] ||
    fail "registry tuple state fields were wrong"

BAD_REGISTRY="$FIXTURE_ROOT/bad-registry.json"
BAD_COPY="$FIXTURE_ROOT/bad-registry.copy"
printf '{bad registry\n' >"$BAD_REGISTRY"
cp "$BAD_REGISTRY" "$BAD_COPY"
if cli registry-record \
    --registry-file "$BAD_REGISTRY" \
    --version "$VERSION" \
    --raw-sha256 "$RAW_SHA" \
    --runtime-sha256 "$RUNTIME_SHA" \
    --package-spec "$PACKAGE_SPEC" \
    --runtime-path "$RUNTIME_PATH" \
    --wrapper-version "$WRAPPER_VERSION" \
    --wrapper-commit "$WRAPPER_COMMIT" \
    --runtime-store-dir "$RUNTIME_STORE" \
    --updated-at "$UPDATED_AT" \
    --smoke-tested-at "" \
    --raw-path "$RAW_PATH" >/dev/null 2>&1; then
    fail "malformed registry record succeeded"
fi
cmp -s "$BAD_REGISTRY" "$BAD_COPY" ||
    fail "malformed registry record did not preserve exact file content"

SCHEMA_BAD_REGISTRY="$FIXTURE_ROOT/schema-bad-registry.json"
SCHEMA_BAD_COPY="$FIXTURE_ROOT/schema-bad-registry.copy"
python3 -c 'import json, sys; from pathlib import Path; Path(sys.argv[1]).write_text(json.dumps({"schema": 3, "installs": [{}], "raw": {}, "wrapper": {}, "runtime": {}}, ensure_ascii=True, sort_keys=True) + "\n")' "$SCHEMA_BAD_REGISTRY"
cp "$SCHEMA_BAD_REGISTRY" "$SCHEMA_BAD_COPY"
if cli registry-record \
    --registry-file "$SCHEMA_BAD_REGISTRY" \
    --version "$VERSION" \
    --raw-sha256 "$RAW_SHA" \
    --runtime-sha256 "$RUNTIME_SHA" \
    --package-spec "$PACKAGE_SPEC" \
    --runtime-path "$RUNTIME_PATH" \
    --wrapper-version "$WRAPPER_VERSION" \
    --wrapper-commit "$WRAPPER_COMMIT" \
    --runtime-store-dir "$RUNTIME_STORE" \
    --updated-at "$UPDATED_AT" \
    --smoke-tested-at "" \
    --raw-path "$RAW_PATH" >/dev/null 2>&1; then
    fail "schema-invalid registry record succeeded"
fi
cmp -s "$SCHEMA_BAD_REGISTRY" "$SCHEMA_BAD_COPY" ||
    fail "schema-invalid registry record did not preserve exact file content"

(
    export CODEX_NATIVE_HOME="$FIXTURE_ROOT/shell-home"
    export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/shell-native"
    export CODEX_NATIVE_MANAGER_DIR="$FIXTURE_ROOT/shell-manager"
    export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/shell-state"
    export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
    export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
    export CODEX_NATIVE_RUNTIME_STORE_DIR="$FIXTURE_ROOT/shell-store/runtime"
    mkdir -p "$CODEX_NATIVE_MANAGER_DIR" "$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two" "$(dirname "$RAW_PATH")"
    {
        printf 'CODEX_NATIVE_WRAPPER_VERSION=%s\n' "$WRAPPER_VERSION"
        printf 'CODEX_NATIVE_WRAPPER_COMMIT=%s\n' "$WRAPPER_COMMIT"
    } >"$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    printf '#!/bin/sh\n' >"$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two/codex"

    # shellcheck disable=SC1091
    . "$ROOT_DIR/lib/codex-termux-lib.sh"

    shell_tuple="$(
        codex_record_registry \
            "$VERSION" \
            "$RAW_SHA" \
            "$RUNTIME_SHA" \
            "$PACKAGE_SPEC" \
            "$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two" \
            "$SMOKE_TESTED_AT" \
            "$RAW_PATH"
    )"
    [ "$shell_tuple" = "$TUPLE_ID" ] || fail "shell registry facade printed wrong tuple"
    codex_write_json_state \
        "$VERSION" \
        "$RAW_SHA" \
        "$RUNTIME_SHA" \
        "$PACKAGE_SPEC" \
        "$shell_tuple" \
        "$shell_tuple" \
        "$SMOKE_TESTED_AT"
    [ "$(codex_read_state_field version)" = "$VERSION" ] ||
        fail "shell state facade read wrong version"
    [ "$(codex_registry_tuple_for_runtime_path "$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two")" = "$TUPLE_ID" ] ||
        fail "shell registry tuple lookup failed"

    printf '{"schema":2,"version":"bad","active_tuple_id":"tuple"}\n' >"$CODEX_NATIVE_STATE_FILE"
    if codex_read_state_field version >/dev/null 2>&1; then
        fail "shell wrong-schema state read succeeded"
    fi

    python3 -c 'import json, sys; from pathlib import Path; Path(sys.argv[1]).write_text(json.dumps({"schema": 3, "installs": [{}], "raw": {}, "wrapper": {}, "runtime": {}}, ensure_ascii=True, sort_keys=True) + "\n")' "$CODEX_NATIVE_REGISTRY_FILE"
    cp "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_REGISTRY_FILE.schema-bad-copy"
    if codex_record_registry \
        "$VERSION" \
        "$RAW_SHA" \
        "$RUNTIME_SHA" \
        "$PACKAGE_SPEC" \
        "$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two" \
        "" \
        "$RAW_PATH" >/dev/null 2>&1; then
        fail "shell schema-invalid registry record succeeded"
    fi
    cmp -s "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_REGISTRY_FILE.schema-bad-copy" ||
        fail "shell schema-invalid registry record did not preserve exact file content"

    printf '{bad shell registry\n' >"$CODEX_NATIVE_REGISTRY_FILE"
    cp "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_REGISTRY_FILE.copy"
    if codex_record_registry \
        "$VERSION" \
        "$RAW_SHA" \
        "$RUNTIME_SHA" \
        "$PACKAGE_SPEC" \
        "$CODEX_NATIVE_RUNTIME_STORE_DIR/runtime-two" \
        "" \
        "$RAW_PATH" >/dev/null 2>&1; then
        fail "shell malformed registry record succeeded"
    fi
    cmp -s "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_REGISTRY_FILE.copy" ||
        fail "shell malformed registry record did not preserve exact file content"
)

printf 'state-registry: ok\n'
