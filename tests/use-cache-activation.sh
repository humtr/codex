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

active_runtime="$FIXTURE_ROOT/native/runtime"
active_raw="$FIXTURE_ROOT/native/raw/vendor/aarch64-unknown-linux-musl"
store_runtime="$FIXTURE_ROOT/state/store/runtime/cached"
store_raw="$FIXTURE_ROOT/state/store/raw/cached"
mkdir -p "$active_runtime/codex-resources/zsh/bin" "$active_runtime/codex-path" "$active_raw/bin" \
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

printf '#!/bin/sh\nexit 0\n' >"$active_runtime/managed.sh"
printf 'support-lib\n' >"$active_runtime/lib.sh"
cp "$ROOT_DIR/tools/build-runtime.py" "$active_runtime/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$active_runtime/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$active_runtime/rg-termux-shim.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$active_runtime/wrapper-version.env"
chmod 755 "$active_runtime/managed.sh" "$active_runtime/build-runtime.py" \
    "$active_runtime/bwrap-termux-compat.py" "$active_runtime/rg-termux-shim.sh"

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
export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/native/raw"
export CODEX_NATIVE_RAW_VENDOR="$active_raw"
export CODEX_NATIVE_RUNTIME_DIR="$active_runtime"
export CODEX_NATIVE_RUNTIME="$active_runtime/codex"
export CODEX_NATIVE_RUNTIME_BUILDER="$active_runtime/build-runtime.py"
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
    "version": "old",
    "raw_sha256": "old",
    "runtime_sha256": "old",
    "package_spec": "old",
    "active_tuple_id": "old",
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
    }],
    "raw": {
        "raw-cached": {
            "sha256": raw_sha,
            "path": str(raw_path),
        },
    },
    "wrapper": {
        "wrapper-test": {},
    },
    "runtime": {
        tuple_id: {
            "path": str(runtime_path),
            "raw_id": "raw-cached",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": runtime_sha,
        },
    },
}) + "\n")
PY

codex_use_select 1 >/dev/null || fail "cached runtime activation failed"

[ "$(cat "$CODEX_NATIVE_RAW_VENDOR/bin/codex")" = "cached raw" ] || fail "cached activation did not promote matching raw package"
codex_version | grep -q 'codex cached' || fail "cached activation did not promote cached runtime"
cmp -s "$CODEX_NATIVE_RUNTIME_DIR/bwrap-termux-compat.py" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" \
    || fail "cached activation did not refresh runtime-private bwrap"
cmp -s "$CODEX_NATIVE_RUNTIME_DIR/rg-termux-shim.sh" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg" \
    || fail "cached activation did not refresh runtime-private rg"

python3 - "$CODEX_NATIVE_STATE_FILE" "$raw_sha" "$runtime_sha" <<'PY'
import json, sys
from pathlib import Path
state = json.loads(Path(sys.argv[1]).read_text())
assert state["raw_sha256"] == sys.argv[2]
assert state["runtime_sha256"] == sys.argv[3]
assert state["verified_tuple_id"] == state["active_tuple_id"]
PY

printf 'use-cache-activation: ok\n'
