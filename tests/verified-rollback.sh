#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'verified-rollback: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/verified-rollback-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

manager_dir="$FIXTURE_ROOT/native/manager"
runtime_store="$FIXTURE_ROOT/native/store/runtime"
good_runtime="$runtime_store/good"
bad_runtime="$runtime_store/bad"
mkdir -p "$manager_dir" "$good_runtime/codex-resources/zsh/bin" "$good_runtime/codex-path" \
    "$bad_runtime/codex-resources" "$bad_runtime/codex-path" "$FIXTURE_ROOT/state"

cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$manager_dir/wrapper-version.env"
chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

cat >"$good_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex good\n'
exit 0
EOF
cat >"$bad_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex bad\n'
exit 0
EOF
printf '#!/bin/sh\nexit 0\n' >"$good_runtime/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$good_runtime/codex-resources/zsh/bin/zsh"
cp "$manager_dir/bwrap-termux-compat.py" "$good_runtime/codex-path/bwrap"
cp "$manager_dir/rg-termux-shim.sh" "$good_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$good_runtime/codex-path/rg.real"
printf '{}\n' >"$good_runtime/codex-package.json"
printf '#!/bin/sh\nexit 0\n' >"$bad_runtime/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$bad_runtime/codex-path/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$bad_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$bad_runtime/codex-path/rg.real"
printf '{}\n' >"$bad_runtime/codex-package.json"
chmod 755 "$good_runtime/codex" "$good_runtime/codex-resources/bwrap" \
    "$good_runtime/codex-resources/zsh/bin/zsh" "$good_runtime/codex-path/bwrap" \
    "$good_runtime/codex-path/rg" "$good_runtime/codex-path/rg.real" \
    "$bad_runtime/codex" "$bad_runtime/codex-resources/bwrap" \
    "$bad_runtime/codex-path/bwrap" "$bad_runtime/codex-path/rg" "$bad_runtime/codex-path/rg.real"

good_sha="$(sha256sum "$good_runtime/codex" | awk '{print $1}')"
builder_sha="$(sha256sum "$manager_dir/build-runtime.py" | awk '{print $1}')"
python3 - "$good_runtime/runtime-build.json" "$bad_runtime/runtime-build.json" "$good_sha" "$builder_sha" "$FIXTURE_ROOT/state/registry.json" "$good_runtime" <<'PY'
import json, sys
from pathlib import Path

good_manifest, bad_manifest, good_sha, builder_sha, registry, good_runtime = sys.argv[1:7]
manifest = {
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": good_sha,
}
Path(good_manifest).write_text(json.dumps(manifest) + "\n")
Path(bad_manifest).write_text(json.dumps(manifest) + "\n")
tuple_id = "raw-good__wrapper-test"
Path(registry).write_text(json.dumps({
    "schema": 3,
    "active_tuple_id": "bad",
    "verified_tuple_id": tuple_id,
    "installs": [{
        "tuple_id": tuple_id,
        "raw_id": "raw-good",
        "wrapper_id": "wrapper-test",
        "version": "good",
        "raw_sha256": "raw-good",
        "runtime_sha256": good_sha,
        "package_spec": "@openai/codex@good",
        "runtime_path": str(Path(good_runtime)),
        "raw_path": "/raw",
    }],
    "runtime": {
        tuple_id: {
            "path": str(Path(good_runtime)),
            "raw_id": "raw-good",
            "wrapper_id": "wrapper-test",
            "runtime_sha256": good_sha,
        },
    },
    "raw": {"raw-good": {"path": "/raw", "sha256": "raw-good"}},
    "wrapper": {"wrapper-test": {}},
}) + "\n")
PY

printf 'nameserver 127.0.0.1\n' >"$FIXTURE_ROOT/resolv.conf"
printf 'cert\n' >"$FIXTURE_ROOT/cert.pem"
ln -s "$bad_runtime" "$FIXTURE_ROOT/native/current"
ln -s "$good_runtime" "$FIXTURE_ROOT/native/verified"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RUNTIME_DIR="$FIXTURE_ROOT/native/current"
export CODEX_NATIVE_RUNTIME="$CODEX_NATIVE_RUNTIME_DIR/codex"
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

if codex_runtime_ok; then
    fail "bad current runtime unexpectedly passed readiness"
fi
codex_try_verified_rollback || fail "verified rollback failed"
[ "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" = "$good_runtime" ] || fail "current pointer did not move to verified runtime"
codex_runtime_ok || fail "verified runtime is not ready after rollback"

printf 'verified-rollback: ok\n'
