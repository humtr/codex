#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'runtime-integrity: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/runtime-integrity-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

raw_vendor="$FIXTURE_ROOT/raw/vendor/aarch64-unknown-linux-musl"
runtime_dir="$FIXTURE_ROOT/runtime"
mkdir -p "$raw_vendor/bin" "$raw_vendor/codex-resources/zsh/bin" "$raw_vendor/codex-path"
cat >"$raw_vendor/bin/codex" <<'EOF'
#!/bin/sh
# /etc/resolv.conf /etc/resolv.conf
[ "${1:-}" != "--version" ] || printf 'codex 0.1.0\n'
exit 0
EOF
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/zsh/bin/zsh"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-path/rg"
printf '{}\n' >"$raw_vendor/codex-package.json"
chmod 755 "$raw_vendor/bin/codex" "$raw_vendor/codex-resources/bwrap" \
    "$raw_vendor/codex-resources/zsh/bin/zsh" "$raw_vendor/codex-path/rg"

python "$ROOT_DIR/tools/build-runtime.py" "$raw_vendor" --runtime-dir "$runtime_dir" >/dev/null
python - "$raw_vendor/bin/codex" "$runtime_dir/codex" "$runtime_dir/runtime-build.json" <<'PY'
import hashlib, json, sys
from pathlib import Path

raw, runtime, manifest_path = map(Path, sys.argv[1:4])
raw_bytes = raw.read_bytes()
runtime_bytes = runtime.read_bytes()
assert raw_bytes.replace(b"/etc/resolv.conf", b"/proc/self/fd/33") == runtime_bytes
manifest = json.loads(manifest_path.read_text())
assert manifest["patch_policy"] == "dns-fd33-only-v1"
assert manifest["raw_sha256"] == hashlib.sha256(raw_bytes).hexdigest()
assert manifest["runtime_sha256"] == hashlib.sha256(runtime_bytes).hexdigest()
assert manifest["resolver_source_count"] == 2
assert manifest["changed_byte_count"] > 0
PY

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/raw"
export CODEX_NATIVE_RAW_VENDOR="$raw_vendor"
export CODEX_NATIVE_RUNTIME_DIR="$runtime_dir"
export CODEX_NATIVE_RUNTIME="$runtime_dir/codex"
export CODEX_NATIVE_RUNTIME_BUILDER="$ROOT_DIR/tools/build-runtime.py"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$CODEX_NATIVE_STATE_DIR/store"
export CODEX_NATIVE_RUNTIME_RETENTION=3
export CODEX_NATIVE_RESOLV_CONF="$FIXTURE_ROOT/resolv.conf"
export CODEX_NATIVE_CERT_FILE="$FIXTURE_ROOT/cert.pem"
mkdir -p "$CODEX_NATIVE_STATE_DIR"
printf 'nameserver 127.0.0.1\n' >"$CODEX_NATIVE_RESOLV_CONF"
printf 'cert\n' >"$CODEX_NATIVE_CERT_FILE"
python - "$CODEX_NATIVE_STATE_FILE" "$raw_vendor/bin/codex" "$runtime_dir/codex" <<'PY'
import hashlib, json, sys
from pathlib import Path

state, raw, runtime = map(Path, sys.argv[1:4])
state.write_text(json.dumps({
    "raw_sha256": hashlib.sha256(raw.read_bytes()).hexdigest(),
    "runtime_sha256": hashlib.sha256(runtime.read_bytes()).hexdigest(),
}) + "\n")
PY

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"
codex_runtime_integrity_ok || fail "valid runtime failed integrity check"
printf 'drift\n' >>"$runtime_dir/codex"
if codex_runtime_integrity_ok; then
    fail "runtime drift passed integrity check"
fi
python "$ROOT_DIR/tools/build-runtime.py" "$raw_vendor" --runtime-dir "$runtime_dir" >/dev/null

raw_sha="$(codex_sha256 "$raw_vendor/bin/codex")"
runtime_sha="$(codex_sha256 "$runtime_dir/codex")"
runtime_path="$(codex_store_runtime_payload "0.1.0-linux-arm64" "$runtime_sha")"
codex_write_json_state "0.1.0-linux-arm64" "$raw_sha" "$runtime_sha" "@openai/codex@0.1.0-linux-arm64" "old-tuple"
python - "$CODEX_NATIVE_STATE_FILE" <<'PY'
import json, sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text())
data["wrapper_version"] = "old"
data["wrapper_commit"] = "old"
path.write_text(json.dumps(data) + "\n")
PY
codex_refresh_runtime_metadata
python - "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_REGISTRY_FILE" "$runtime_path" "$(codex_current_wrapper_commit)" <<'PY'
import json, sys
from pathlib import Path

state_path, registry_path, runtime_path = map(Path, sys.argv[1:4])
wrapper_commit = sys.argv[4]
state = json.loads(state_path.read_text())
registry = json.loads(registry_path.read_text())
active = state["active_tuple_id"]
assert state["wrapper_commit"] == wrapper_commit
assert state["verified_tuple_id"] == active
assert state["verified_at"]
assert wrapper_commit[:12] in active
assert active in registry["runtime"]
assert Path(registry["runtime"][active]["path"]).resolve() == runtime_path.resolve()
PY

python - "$CODEX_NATIVE_STORE_DIR/runtime" "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_RUNTIME_BUILDER" <<'PY'
import hashlib, json, os, sys
from pathlib import Path

store, registry, builder = map(Path, sys.argv[1:4])
store.mkdir(parents=True, exist_ok=True)
builder_sha = hashlib.sha256(builder.read_bytes()).hexdigest()
runtime_entries = {}
installs = []
for index in range(5):
    path = store / f"runtime-{index}"
    path.mkdir()
    codex = path / "codex"
    codex.write_text(f"runtime-{index}\n")
    digest = hashlib.sha256(codex.read_bytes()).hexdigest()
    manifest = {
        "patch_policy": "dns-fd33-only-v1" if index < 4 else "incompatible",
        "builder_sha256": builder_sha,
        "runtime_sha256": digest,
    }
    (path / "runtime-build.json").write_text(json.dumps(manifest) + "\n")
    os.utime(path, (index + 1, index + 1))
    key = f"tuple-{index}"
    runtime_entries[key] = {"path": str(path), "raw_id": "raw", "wrapper_id": "wrapper"}
    installs.append({
        "tuple_id": key,
        "runtime_path": str(path),
        "raw_id": "raw",
        "wrapper_id": "wrapper",
    })
registry.write_text(json.dumps({
    "schema": 2,
    "active_tuple_id": "tuple-0",
    "runtime": runtime_entries,
    "installs": installs,
    "raw": {"raw": {}},
    "wrapper": {"wrapper": {}},
}) + "\n")
PY
codex_prune_runtime_store
[ -d "$CODEX_NATIVE_STORE_DIR/runtime/runtime-0" ] || fail "active runtime was pruned"
[ ! -d "$CODEX_NATIVE_STORE_DIR/runtime/runtime-4" ] || fail "incompatible runtime was kept"
[ "$(find "$CODEX_NATIVE_STORE_DIR/runtime" -mindepth 1 -maxdepth 1 -type d | wc -l)" -eq 3 ] \
    || fail "runtime retention did not keep exactly three entries"

printf 'runtime-integrity: ok\n'
