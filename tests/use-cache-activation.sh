#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'use-cache-activation: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/use-cache-activation-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

active_runtime="$FIXTURE_ROOT/state/store/runtime/active"
active_raw_root="$FIXTURE_ROOT/state/store/raw/active"
active_raw="$active_raw_root/vendor/aarch64-unknown-linux-musl"
current_link="$FIXTURE_ROOT/native/current"
raw_link="$FIXTURE_ROOT/native/raw"
manager_dir="$FIXTURE_ROOT/native/manager"
store_runtime="$FIXTURE_ROOT/state/store/runtime/cached"
store_raw="$FIXTURE_ROOT/state/store/raw/cached"
mkdir -p "$active_runtime/codex-resources/zsh/bin" "$active_runtime/codex-path" "$active_raw/bin" \
    "$manager_dir" \
    "$store_runtime/codex-resources/zsh/bin" "$store_runtime/codex-path" \
    "$store_raw/vendor/aarch64-unknown-linux-musl/bin" "$FIXTURE_ROOT/state"

cat >"$active_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex old\n'
exit 0
EOF
cat >"$store_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex cached\n'
exit 0
EOF
printf 'old raw\n' >"$active_raw/bin/codex"
printf 'cached raw\n' >"$store_raw/vendor/aarch64-unknown-linux-musl/bin/codex"
chmod 755 "$active_runtime/codex" "$store_runtime/codex" \
    "$active_raw/bin/codex" "$store_raw/vendor/aarch64-unknown-linux-musl/bin/codex"
ln -s "$active_runtime" "$current_link"
ln -s "$active_runtime" "$FIXTURE_ROOT/native/verified"
ln -s "$active_raw_root" "$raw_link"

printf '#!/bin/sh\nexit 0\n' >"$manager_dir/managed.sh"
printf 'support-lib\n' >"$manager_dir/lib.sh"
cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$manager_dir/wrapper-version.env"
chmod 755 "$manager_dir/managed.sh" "$manager_dir/build-runtime.py" \
    "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

printf '#!/bin/sh\nexit 0\n' >"$store_runtime/codex-path/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$store_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$store_runtime/codex-path/rg.real"
printf '#!/bin/sh\nexit 0\n' >"$store_runtime/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$store_runtime/codex-resources/zsh/bin/zsh"
printf '{}\n' >"$store_runtime/codex-package.json"
chmod 755 "$store_runtime/codex-path/bwrap" "$store_runtime/codex-path/rg" \
    "$store_runtime/codex-path/rg.real" "$store_runtime/codex-resources/bwrap" \
    "$store_runtime/codex-resources/zsh/bin/zsh"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_RAW_DIR="$raw_link"
export CODEX_NATIVE_RAW_VENDOR="$raw_link/vendor/aarch64-unknown-linux-musl"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RUNTIME_DIR="$current_link"
export CODEX_NATIVE_RUNTIME="$current_link/codex"
export CODEX_NATIVE_VERIFIED_LINK="$FIXTURE_ROOT/native/verified"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$CODEX_NATIVE_STATE_DIR/store"
export CODEX_NATIVE_RESOLV_CONF="$FIXTURE_ROOT/resolv.conf"
export CODEX_NATIVE_CERT_FILE="$FIXTURE_ROOT/cert.pem"
export CODEX_NATIVE_RUNTIME_RETENTION=3
export CODEX_USE_LAST_LATEST="cached"
printf 'nameserver 127.0.0.1\n' >"$CODEX_NATIVE_RESOLV_CONF"
printf 'cert\n' >"$CODEX_NATIVE_CERT_FILE"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

runtime_sha="$(codex_sha256 "$store_runtime/codex")"
raw_sha="$(codex_sha256 "$store_raw/vendor/aarch64-unknown-linux-musl/bin/codex")"
builder_sha="$(codex_sha256 "$CODEX_NATIVE_RUNTIME_BUILDER")"
python3 - "$store_runtime/runtime-build.json" "$runtime_sha" "$builder_sha" <<'PY'
import json, sys
from pathlib import Path
manifest, runtime_sha, builder_sha = sys.argv[1:4]
Path(manifest).write_text(json.dumps({
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": runtime_sha,
}) + "\n")
PY
python3 - "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_REGISTRY_FILE" "$store_runtime" "$store_raw" "$raw_sha" "$runtime_sha" <<'PY'
import json, sys
from pathlib import Path
state_path, registry_path, runtime_path, raw_path = map(Path, sys.argv[1:5])
raw_sha, runtime_sha = sys.argv[5:7]
tuple_id = "raw-cached__wrapper-test"
state_path.write_text(json.dumps({
    "schema": 3,
    "version": "old",
    "raw_sha256": "old",
    "runtime_sha256": "old",
    "package_spec": "old",
    "active_tuple_id": "old",
    "wrapper_version": "test",
    "wrapper_commit": "test",
    "updated_at": "2026-06-15T00:00:00Z",
}) + "\n")
registry_path.write_text(json.dumps({
    "schema": 3,
    "active_tuple_id": "old",
    "installs": [{
        "tuple_id": tuple_id,
        "raw_id": "raw-cached",
        "wrapper_id": "wrapper-test",
        "version": "cached",
        "raw_sha256": raw_sha,
        "runtime_sha256": runtime_sha,
        "package_spec": "@openai/codex@cached",
        "runtime_path": str(runtime_path),
        "raw_path": str(raw_path),
        "updated_at": "2026-06-15T00:00:00Z",
    }],
    "raw": {
        "raw-cached": {
            "version": "cached",
            "sha256": raw_sha,
            "package_spec": "@openai/codex@cached",
            "path": str(raw_path),
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
        tuple_id: {
            "path": str(runtime_path),
            "raw_id": "raw-cached",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": runtime_sha,
            "updated_at": "2026-06-15T00:00:00Z",
        },
    },
}) + "\n")
PY

codex_use_select 0 >/dev/null || fail "latest cached runtime activation failed"

[ "$(cat "$CODEX_NATIVE_RAW_VENDOR/bin/codex")" = "cached raw" ] || fail "cached activation did not promote matching raw package"
[ -L "$CODEX_NATIVE_RAW_DIR" ] || fail "cached activation did not create raw pointer"
[ -L "$CODEX_NATIVE_RUNTIME_DIR" ] || fail "cached activation did not create runtime pointer"
codex_version | grep -q 'codex cached' || fail "cached activation did not promote cached runtime"
cmp -s "$CODEX_NATIVE_MANAGER_DIR/bwrap-termux-compat.py" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" \
    || fail "cached activation did not refresh runtime-private bwrap"
cmp -s "$CODEX_NATIVE_MANAGER_DIR/rg-termux-shim.sh" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg" \
    || fail "cached activation did not refresh runtime-private rg"

python3 - "$CODEX_NATIVE_STATE_FILE" "$raw_sha" "$runtime_sha" <<'PY'
import json, sys
from pathlib import Path
state = json.loads(Path(sys.argv[1]).read_text())
assert state["raw_sha256"] == sys.argv[2]
assert state["runtime_sha256"] == sys.argv[3]
assert state["verified_tuple_id"] == state["active_tuple_id"]
PY

codex_use_select "cached" >/dev/null || fail "selection by version failed"
codex_use_select "${runtime_sha:0:12}" >/dev/null || fail "selection by hash prefix failed"
if codex_use_select "missing" >/dev/null 2>&1; then
    fail "unknown selection unexpectedly succeeded"
fi

python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$FIXTURE_ROOT/state/store/runtime/invalid" "$FIXTURE_ROOT/state/store/raw/invalid" <<'PY'
import json, sys
from pathlib import Path

registry_path, invalid_runtime, invalid_raw = map(Path, sys.argv[1:4])
invalid_runtime.mkdir(parents=True, exist_ok=True)
invalid_raw.mkdir(parents=True, exist_ok=True)
data = json.loads(registry_path.read_text())
data["installs"].append({
    "tuple_id": "tuple-invalid-runtime",
    "raw_id": "raw-invalid",
    "wrapper_id": "wrapper-test",
    "version": "invalid-runtime",
    "raw_sha256": "raw-invalid",
    "runtime_sha256": "runtime-invalid",
    "package_spec": "@openai/codex@invalid-runtime",
    "runtime_path": str(invalid_runtime),
    "raw_path": str(invalid_raw),
    "updated_at": "2026-06-15T00:00:00Z",
})
data["installs"].append({
    "tuple_id": "tuple-invalid-raw",
    "raw_id": "raw-invalid-two",
    "wrapper_id": "wrapper-test",
    "version": "invalid-raw",
    "raw_sha256": "raw-invalid-two",
    "runtime_sha256": data["installs"][0]["runtime_sha256"],
    "package_spec": "@openai/codex@invalid-raw",
    "runtime_path": data["installs"][0]["runtime_path"],
    "raw_path": str(invalid_raw),
    "updated_at": "2026-06-15T00:00:00Z",
})
registry_path.write_text(json.dumps(data) + "\n")
PY
list_output="$(codex_use_render "cached" 10 list)"
[[ "$list_output" == *$'cached\t'* ]] || fail "cached runtime missing from list output"
[[ "$list_output" != *"invalid-runtime"* ]] || fail "invalid runtime path was not excluded"
[[ "$list_output" != *"invalid-raw"* ]] || fail "invalid raw path was not excluded"

menu_count="$(codex_use_render "latest-new" 10 menu 2>"$FIXTURE_ROOT/use-menu.txt")"
[ "$menu_count" = "2" ] || fail "menu count should reflect cached choices only"
menu_output="$(cat "$FIXTURE_ROOT/use-menu.txt")"
[[ "$menu_output" == *"0. latest-new"* ]] && [[ "$menu_output" == *"⬇ update"* ]] \
    || fail "remote update row should be listed first as 0"
[[ "$menu_output" == *"1. cached"* ]] && [[ "$menu_output" == *"🟢 active"* ]] \
    || fail "active cached row should be listed as 1"

remote_selected="$(codex_native_cmd use-select \
    --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
    --choice 0 \
    --latest latest-new \
    --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
    --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
    --patch-policy "$CODEX_NATIVE_PATCH_POLICY")" || fail "remote selection by 0 failed"
IFS=$'\037' read -r remote_kind _ _ remote_version _ _ _ <<EOF
$remote_selected
EOF
[ "$remote_kind" = "remote" ] || fail "selection 0 did not resolve to remote install"
[ "$remote_version" = "latest-new" ] || fail "selection 0 resolved the wrong remote version"

cached_selected="$(codex_native_cmd use-select \
    --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
    --choice 1 \
    --latest latest-new \
    --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
    --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
    --patch-policy "$CODEX_NATIVE_PATCH_POLICY")" || fail "cached selection by 1 failed"
IFS=$'\037' read -r cached_kind _ _ cached_version _ _ _ <<EOF
$cached_selected
EOF
[ "$cached_kind" = "cached" ] || fail "selection 1 did not resolve to cached runtime"
[ "$cached_version" = "cached" ] || fail "selection 1 resolved the wrong cached version"

printf 'use-cache-activation: ok\n'
