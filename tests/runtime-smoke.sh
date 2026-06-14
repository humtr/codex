#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'runtime-smoke: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/runtime-smoke-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

raw_vendor="$FIXTURE_ROOT/raw/vendor/aarch64-unknown-linux-musl"
runtime_dir="$FIXTURE_ROOT/runtime"
mkdir -p "$raw_vendor/bin" "$raw_vendor/codex-resources/zsh/bin" "$raw_vendor/codex-path"
cat >"$raw_vendor/bin/codex" <<'EOF'
#!/bin/sh
for arg in "$@"; do
    case "$arg" in
        --version)
            printf 'codex 0.1.0\n'
            exit 0
            ;;
    esac
done
printf 'prefix /etc/resolv.conf middle /etc/resolv.conf suffix\n' >&2
exit 0
EOF
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/zsh/bin/zsh"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-path/rg"
printf '{}\n' >"$raw_vendor/codex-package.json"
chmod 755 "$raw_vendor/bin/codex" "$raw_vendor/codex-resources/bwrap" \
    "$raw_vendor/codex-resources/zsh/bin/zsh" "$raw_vendor/codex-path/rg"

mkdir -p "$runtime_dir"
printf '#!/bin/sh\nexit 0\n' >"$runtime_dir/managed.sh"
printf 'support-lib\n' >"$runtime_dir/lib.sh"
cp "$ROOT_DIR/tools/build-runtime.py" "$runtime_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$runtime_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$runtime_dir/rg-termux-shim.sh"
printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$runtime_dir/wrapper-version.env"
chmod 755 "$runtime_dir/managed.sh" "$runtime_dir/build-runtime.py" \
    "$runtime_dir/bwrap-termux-compat.py" "$runtime_dir/rg-termux-shim.sh"

printf 'nameserver 127.0.0.1\n' >"$FIXTURE_ROOT/resolv.conf"
printf 'cert\n' >"$FIXTURE_ROOT/cert.pem"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_RAW_DIR="$FIXTURE_ROOT/raw"
export CODEX_NATIVE_RAW_VENDOR="$raw_vendor"
export CODEX_NATIVE_RUNTIME_DIR="$runtime_dir"
export CODEX_NATIVE_RUNTIME="$runtime_dir/codex"
export CODEX_NATIVE_RUNTIME_BUILDER="$runtime_dir/build-runtime.py"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$CODEX_NATIVE_STATE_DIR/store"
export CODEX_NATIVE_DOCTOR_DIR="$CODEX_NATIVE_STATE_DIR/doctor"
export CODEX_NATIVE_RESOLV_CONF="$FIXTURE_ROOT/resolv.conf"
export CODEX_NATIVE_CERT_FILE="$FIXTURE_ROOT/cert.pem"
export CODEX_NATIVE_RUNTIME_RETENTION=3
mkdir -p "$CODEX_NATIVE_STATE_DIR"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

codex_rebuild_runtime_unlocked "0.1.0-linux-arm64" "@openai/codex@0.1.0-linux-arm64" \
    || fail "runtime rebuild failed"

python3 - "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_REGISTRY_FILE" <<'PY'
import json, sys
from pathlib import Path

state_path, registry_path = map(Path, sys.argv[1:3])
state = json.loads(state_path.read_text())
registry = json.loads(registry_path.read_text())
active = state["active_tuple_id"]
assert state["verified_tuple_id"] == active
assert state["verified_at"]
assert active in registry["runtime"]
assert registry["runtime"][active]["smoke_tested_at"]
PY

codex_version >/dev/null || fail "version failed after smoke-tested rebuild"
[ "$(cat "$runtime_dir/lib.sh")" = "support-lib" ] || fail "runtime rebuild discarded wrapper support"
[ -x "$runtime_dir/managed.sh" ] || fail "runtime rebuild discarded managed launcher target"
[ -x "$runtime_dir/build-runtime.py" ] || fail "runtime rebuild discarded runtime builder"

printf 'runtime-smoke: ok\n'
