#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'pointer-activation: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/pointer-activation-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

manager_dir="$FIXTURE_ROOT/native/manager"
candidate_runtime="$FIXTURE_ROOT/candidate.runtime"
raw_root="$FIXTURE_ROOT/raw"
raw_vendor="$raw_root/vendor/aarch64-unknown-linux-musl"
mkdir -p "$manager_dir" "$candidate_runtime/codex-resources/zsh/bin" "$candidate_runtime/codex-path" \
    "$raw_vendor/bin"

cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
printf '#!/bin/sh\nexit 0\n' >"$manager_dir/managed.sh"
printf 'support-lib\n' >"$manager_dir/lib.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$manager_dir/wrapper-version.env"
chmod 755 "$manager_dir/managed.sh" "$manager_dir/build-runtime.py" \
    "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

cat >"$candidate_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex pointer\n'
exit 0
EOF
printf '#!/bin/sh\nexit 0\n' >"$candidate_runtime/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$candidate_runtime/codex-resources/zsh/bin/zsh"
cp "$manager_dir/bwrap-termux-compat.py" "$candidate_runtime/codex-path/bwrap"
cp "$manager_dir/rg-termux-shim.sh" "$candidate_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$candidate_runtime/codex-path/rg.real"
printf '{}\n' >"$candidate_runtime/codex-package.json"
printf 'raw pointer\n' >"$raw_vendor/bin/codex"
chmod 755 "$candidate_runtime/codex" "$candidate_runtime/codex-resources/bwrap" \
    "$candidate_runtime/codex-resources/zsh/bin/zsh" "$candidate_runtime/codex-path/bwrap" \
    "$candidate_runtime/codex-path/rg" "$candidate_runtime/codex-path/rg.real" "$raw_vendor/bin/codex"

runtime_sha="$(sha256sum "$candidate_runtime/codex" | awk '{print $1}')"
raw_sha="$(sha256sum "$raw_vendor/bin/codex" | awk '{print $1}')"
builder_sha="$(sha256sum "$manager_dir/build-runtime.py" | awk '{print $1}')"
python3 - "$candidate_runtime/runtime-build.json" "$runtime_sha" "$raw_sha" "$builder_sha" <<'PY'
import json, sys
from pathlib import Path
manifest, runtime_sha, raw_sha, builder_sha = sys.argv[1:5]
Path(manifest).write_text(json.dumps({
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": runtime_sha,
    "raw_sha256": raw_sha,
}) + "\n")
PY

printf 'nameserver 127.0.0.1\n' >"$FIXTURE_ROOT/resolv.conf"
printf 'cert\n' >"$FIXTURE_ROOT/cert.pem"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/native/raw"
export CODEX_NATIVE_RAW_VENDOR="$CODEX_NATIVE_RAW_DIR/vendor/aarch64-unknown-linux-musl"
export CODEX_NATIVE_RUNTIME_DIR="$FIXTURE_ROOT/native/current"
export CODEX_NATIVE_RUNTIME="$CODEX_NATIVE_RUNTIME_DIR/codex"
export CODEX_NATIVE_CURRENT_LINK="$CODEX_NATIVE_RUNTIME_DIR"
export CODEX_NATIVE_VERIFIED_LINK="$FIXTURE_ROOT/native/verified"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$FIXTURE_ROOT/native/store"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
export CODEX_NATIVE_RESOLV_CONF="$FIXTURE_ROOT/resolv.conf"
export CODEX_NATIVE_CERT_FILE="$FIXTURE_ROOT/cert.pem"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

codex_activate_tuple_unlocked "$candidate_runtime" "pointer" "$raw_sha" "$runtime_sha" "@openai/codex@pointer" "$raw_root" \
    || fail "tuple activation failed"

[ -L "$CODEX_NATIVE_RUNTIME_DIR" ] || fail "current runtime is not a pointer"
[ -L "$CODEX_NATIVE_VERIFIED_LINK" ] || fail "verified runtime is not a pointer"
[ -L "$CODEX_NATIVE_RAW_DIR" ] || fail "raw cache is not a pointer"
case "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" in
    "$CODEX_NATIVE_RUNTIME_STORE_DIR"/*) ;;
    *) fail "current runtime pointer targets the wrong root" ;;
esac
[ "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" = "$(readlink "$CODEX_NATIVE_VERIFIED_LINK")" ] \
    || fail "current and verified pointers differ after verified activation"
codex_runtime_ok || fail "activated runtime is not ready"

python3 - "$CODEX_NATIVE_REGISTRY_FILE" <<'PY'
import json, sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text())
active = data["active_tuple_id"]
data["active_tuple_id"] = "broken"
data["verified_tuple_id"] = "broken"
data["runtime"][active]["path"] = "/broken/runtime"
data["raw"][data["runtime"][active]["raw_id"]]["path"] = "/broken/raw"
for entry in data["installs"]:
    if entry.get("tuple_id") == active:
        entry["runtime_path"] = "/broken/runtime"
        entry["raw_path"] = "/broken/raw"
path.write_text(json.dumps(data) + "\n")
PY
if codex_runtime_metadata_current; then
    fail "registry drift passed metadata readiness"
fi
codex_refresh_runtime_metadata || fail "registry drift metadata refresh failed"
codex_runtime_metadata_current || fail "metadata refresh did not restore pointer/registry agreement"

doctor_json="$(codex_native_cmd doctor-report \
    --runtime "$CODEX_NATIVE_RUNTIME" \
    --current-link "$CODEX_NATIVE_RUNTIME_DIR" \
    --verified-link "$CODEX_NATIVE_VERIFIED_LINK" \
    --raw-link "$CODEX_NATIVE_RAW_DIR" \
    --manager-dir "$CODEX_NATIVE_MANAGER_DIR" \
    --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
    --raw-store-dir "$CODEX_NATIVE_RAW_STORE_DIR" \
    --raw-vendor "$CODEX_NATIVE_RAW_VENDOR" \
    --resolv-conf "$CODEX_NATIVE_RESOLV_CONF" \
    --cert-file "$CODEX_NATIVE_CERT_FILE" \
    --state-file "$CODEX_NATIVE_STATE_FILE" \
    --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
    --migration-report-file "$CODEX_NATIVE_STORE_MIGRATION_REPORT" \
    --legacy-store-dir "$CODEX_NATIVE_LEGACY_STORE_DIR" \
    --version "$(codex_read_state_field version)" \
    --raw-sha256 "$(codex_read_state_field raw_sha256)" \
    --runtime-sha256 "$(codex_read_state_field runtime_sha256)" \
    --prefix "$CODEX_NATIVE_PREFIX" \
    --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
    --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
    --network-json '{"overallStatus":"ok","checks":{"baseline_socket":true,"network_off":true,"network_on":true,"network_reset":true}}')"
python3 - "$doctor_json" "$CODEX_NATIVE_RUNTIME_DIR" "$CODEX_NATIVE_VERIFIED_LINK" \
    "$CODEX_NATIVE_RAW_DIR" "$CODEX_NATIVE_RUNTIME_STORE_DIR" "$CODEX_NATIVE_RAW_STORE_DIR" <<'PY'
import json, sys
from pathlib import Path

data = json.loads(sys.argv[1])
current, verified, raw, runtime_store, raw_store = map(Path, sys.argv[2:7])
assert data["schema"] == 4
for name in (
    "manager",
    "runtime_store",
    "raw_store",
    "current_pointer",
    "verified_pointer",
    "raw_pointer",
    "current_in_store",
    "verified_in_store",
    "raw_in_store",
    "current_verified_match",
    "registry_current_match",
    "registry_verified_match",
):
    assert data["checks"][name] is True, name
assert data["paths"]["current"] == str(current)
assert data["paths"]["verified"] == str(verified)
assert data["paths"]["raw"] == str(raw)
assert data["paths"]["runtime_store"] == str(runtime_store)
assert data["paths"]["raw_store"] == str(raw_store)
assert data["migration"]["status"] == "not-needed"
PY

printf 'pointer-activation: ok\n'
