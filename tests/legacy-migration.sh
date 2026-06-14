#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'legacy-migration: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/legacy-migration-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

native_root="$FIXTURE_ROOT/native"
manager_dir="$native_root/manager"
legacy_runtime="$native_root/runtime"
legacy_raw="$native_root/raw/vendor/aarch64-unknown-linux-musl"
mkdir -p "$manager_dir" "$legacy_runtime/codex-resources/zsh/bin" "$legacy_runtime/codex-path" \
    "$legacy_raw/bin" "$FIXTURE_ROOT/state"

cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$manager_dir/wrapper-version.env"
chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

cat >"$legacy_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex legacy\n'
exit 0
EOF
printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-resources/zsh/bin/zsh"
cp "$manager_dir/bwrap-termux-compat.py" "$legacy_runtime/codex-path/bwrap"
cp "$manager_dir/rg-termux-shim.sh" "$legacy_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-path/rg.real"
printf '{}\n' >"$legacy_runtime/codex-package.json"
printf 'raw legacy\n' >"$legacy_raw/bin/codex"
chmod 755 "$legacy_runtime/codex" "$legacy_runtime/codex-resources/bwrap" \
    "$legacy_runtime/codex-resources/zsh/bin/zsh" "$legacy_runtime/codex-path/bwrap" \
    "$legacy_runtime/codex-path/rg" "$legacy_runtime/codex-path/rg.real" "$legacy_raw/bin/codex"

runtime_sha="$(sha256sum "$legacy_runtime/codex" | awk '{print $1}')"
raw_sha="$(sha256sum "$legacy_raw/bin/codex" | awk '{print $1}')"
builder_sha="$(sha256sum "$manager_dir/build-runtime.py" | awk '{print $1}')"
python3 - "$legacy_runtime/runtime-build.json" "$runtime_sha" "$raw_sha" "$builder_sha" "$FIXTURE_ROOT/state/state.json" <<'PY'
import json, sys
from pathlib import Path
manifest, runtime_sha, raw_sha, builder_sha, state = sys.argv[1:6]
Path(manifest).write_text(json.dumps({
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": runtime_sha,
    "raw_sha256": raw_sha,
}) + "\n")
Path(state).write_text(json.dumps({
    "version": "legacy",
    "raw_sha256": raw_sha,
    "runtime_sha256": runtime_sha,
    "package_spec": "@openai/codex@legacy",
}) + "\n")
PY

printf 'nameserver 127.0.0.1\n' >"$FIXTURE_ROOT/resolv.conf"
printf 'cert\n' >"$FIXTURE_ROOT/cert.pem"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$native_root"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RUNTIME_DIR="$native_root/current"
export CODEX_NATIVE_RUNTIME="$CODEX_NATIVE_RUNTIME_DIR/codex"
export CODEX_NATIVE_CURRENT_LINK="$CODEX_NATIVE_RUNTIME_DIR"
export CODEX_NATIVE_VERIFIED_LINK="$native_root/verified"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$native_root/store"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
export CODEX_NATIVE_RESOLV_CONF="$FIXTURE_ROOT/resolv.conf"
export CODEX_NATIVE_CERT_FILE="$FIXTURE_ROOT/cert.pem"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

codex_migrate_legacy_runtime_layout || fail "legacy migration failed"
[ -L "$CODEX_NATIVE_RUNTIME_DIR" ] || fail "migration did not create current pointer"
[ -L "$CODEX_NATIVE_VERIFIED_LINK" ] || fail "migration did not create verified pointer"
[ -d "$legacy_runtime" ] || fail "migration removed legacy runtime directory"
codex_runtime_ok || fail "migrated runtime is not ready"
first_target="$(readlink "$CODEX_NATIVE_RUNTIME_DIR")"
codex_migrate_legacy_runtime_layout || fail "second migration pass failed"
[ "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" = "$first_target" ] || fail "migration was not idempotent"

printf 'legacy-migration: ok\n'
