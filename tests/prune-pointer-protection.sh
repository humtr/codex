#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'prune-pointer-protection: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/prune-pointer-protection-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

manager_dir="$FIXTURE_ROOT/native/manager"
runtime_store="$FIXTURE_ROOT/native/store/runtime"
raw_store="$FIXTURE_ROOT/native/store/raw"
mkdir -p "$manager_dir" "$runtime_store/current-bad" "$runtime_store/verified-bad" \
    "$runtime_store/discard-bad" "$raw_store/raw-protected" "$raw_store/raw-discard" "$FIXTURE_ROOT/state"
cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
chmod 755 "$manager_dir/build-runtime.py"
for path in "$runtime_store/current-bad" "$runtime_store/verified-bad" "$runtime_store/discard-bad"; do
    printf 'bad\n' >"$path/codex"
    printf '{}\n' >"$path/runtime-build.json"
done
ln -s "$runtime_store/current-bad" "$FIXTURE_ROOT/native/current"
ln -s "$runtime_store/verified-bad" "$FIXTURE_ROOT/native/verified"
ln -s "$raw_store/raw-protected" "$FIXTURE_ROOT/native/raw"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RUNTIME_DIR="$FIXTURE_ROOT/native/current"
export CODEX_NATIVE_VERIFIED_LINK="$FIXTURE_ROOT/native/verified"
export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/native/raw"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$FIXTURE_ROOT/native/store"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
export CODEX_NATIVE_RUNTIME_RETENTION=1

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_STATE_FILE" \
    "$runtime_store/current-bad" "$runtime_store/verified-bad" "$runtime_store/discard-bad" \
    "$raw_store/raw-protected" "$raw_store/raw-discard" <<'PY'
import json, sys
from pathlib import Path

registry_path, state_path, current_runtime, verified_runtime, discard_runtime, raw_protected, raw_discard = map(Path, sys.argv[1:8])
registry_path.write_text(json.dumps({
    "schema": 3,
    "active_tuple_id": "tuple-current",
    "verified_tuple_id": "tuple-verified",
    "installs": [
        {
            "tuple_id": "tuple-current",
            "raw_id": "raw-protected-id",
            "wrapper_id": "wrapper-test",
            "version": "current",
            "raw_sha256": "raw-protected",
            "runtime_sha256": "runtime-current",
            "package_spec": "@openai/codex@current",
            "runtime_path": str(current_runtime),
            "raw_path": str(raw_protected),
            "updated_at": "2026-06-15T00:00:00Z",
        },
        {
            "tuple_id": "tuple-verified",
            "raw_id": "raw-protected-id",
            "wrapper_id": "wrapper-test",
            "version": "verified",
            "raw_sha256": "raw-protected",
            "runtime_sha256": "runtime-verified",
            "package_spec": "@openai/codex@verified",
            "runtime_path": str(verified_runtime),
            "raw_path": str(raw_protected),
            "updated_at": "2026-06-15T00:00:00Z",
        },
        {
            "tuple_id": "tuple-discard",
            "raw_id": "raw-discard-id",
            "wrapper_id": "wrapper-test",
            "version": "discard",
            "raw_sha256": "raw-discard",
            "runtime_sha256": "runtime-discard",
            "package_spec": "@openai/codex@discard",
            "runtime_path": str(discard_runtime),
            "raw_path": str(raw_discard),
            "updated_at": "2026-06-15T00:00:00Z",
        },
    ],
    "raw": {
        "raw-protected-id": {
            "version": "current",
            "sha256": "raw-protected",
            "package_spec": "@openai/codex@current",
            "path": str(raw_protected),
            "updated_at": "2026-06-15T00:00:00Z",
        },
        "raw-discard-id": {
            "version": "discard",
            "sha256": "raw-discard",
            "package_spec": "@openai/codex@discard",
            "path": str(raw_discard),
            "updated_at": "2026-06-15T00:00:00Z",
        },
    },
    "wrapper": {
        "wrapper-test": {
            "version": "test",
            "commit": "test",
            "repo": "local/codex",
            "updated_at": "2026-06-15T00:00:00Z",
        },
    },
    "runtime": {
        "tuple-current": {
            "raw_id": "raw-protected-id",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": "runtime-current",
            "path": str(current_runtime),
            "updated_at": "2026-06-15T00:00:00Z",
        },
        "tuple-verified": {
            "raw_id": "raw-protected-id",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": "runtime-verified",
            "path": str(verified_runtime),
            "updated_at": "2026-06-15T00:00:00Z",
        },
        "tuple-discard": {
            "raw_id": "raw-discard-id",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": "runtime-discard",
            "path": str(discard_runtime),
            "updated_at": "2026-06-15T00:00:00Z",
        },
    },
}) + "\n")
state_path.write_text(json.dumps({
    "schema": 3,
    "version": "current",
    "raw_sha256": "raw-protected",
    "runtime_sha256": "runtime-current",
    "package_spec": "@openai/codex@current",
    "active_tuple_id": "tuple-current",
    "wrapper_version": "test",
    "wrapper_commit": "test",
    "updated_at": "2026-06-15T00:00:00Z",
    "verified_tuple_id": "tuple-verified",
    "verified_at": "2026-06-15T00:00:00Z",
}) + "\n")
PY

codex_prune_runtime_store || fail "runtime store prune failed"
[ -d "$runtime_store/current-bad" ] || fail "current pointer target was pruned"
[ -d "$runtime_store/verified-bad" ] || fail "verified pointer target was pruned"
[ ! -e "$runtime_store/discard-bad" ] || fail "unprotected incompatible runtime was retained"
[ -d "$raw_store/raw-protected" ] || fail "raw pointer target was pruned"
[ ! -e "$raw_store/raw-discard" ] || fail "unprotected raw artifact was retained"

mkdir -p "$runtime_store/discard-bad" "$raw_store/raw-discard"
printf '{}\n' >"$CODEX_NATIVE_REGISTRY_FILE"
if codex_prune_runtime_store; then
    fail "malformed registry prune unexpectedly succeeded"
fi
[ -d "$runtime_store/discard-bad" ] || fail "malformed registry prune deleted runtime"
[ -d "$raw_store/raw-discard" ] || fail "malformed registry prune deleted raw"

python3 - "$CODEX_NATIVE_REGISTRY_FILE" <<'PY'
import json, sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({
    "schema": 3,
    "active_tuple_id": "tuple-current",
    "verified_tuple_id": "tuple-verified",
    "installs": [],
    "raw": {},
    "wrapper": {},
    "runtime": {},
}) + "\n")
PY
printf '{}\n' >"$CODEX_NATIVE_STATE_FILE"
if codex_prune_runtime_store; then
    fail "malformed state prune unexpectedly succeeded"
fi
[ -d "$runtime_store/discard-bad" ] || fail "malformed state prune deleted runtime"
[ -d "$raw_store/raw-discard" ] || fail "malformed state prune deleted raw"

printf 'prune-pointer-protection: ok\n'
