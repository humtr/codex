#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
SANDBOX_DIR="$(dirname "$CODEX_TERMUX_STATE_DIR")"
CONTRACT_DIR="$SANDBOX_DIR/runtime-contract"
CHILD_PROBE="$CONTRACT_DIR/inspect-runtime.py"
PYTHON_BIN="$(command -v python3)"

mkdir -p "$CONTRACT_DIR/system-config" "$CONTRACT_DIR/certs"
printf 'nameserver 127.0.0.1\n' >"$CONTRACT_DIR/resolv.conf"
printf 'cert\n' >"$CONTRACT_DIR/cert.pem"

printf '#!%s\n' "$PYTHON_BIN" >"$CHILD_PROBE"
cat >>"$CHILD_PROBE" <<'PYTHON'
from __future__ import annotations

import json
import os
import stat
import sys
from pathlib import Path


def fd_kind(fd: int) -> str:
    mode = os.fstat(fd).st_mode
    if stat.S_ISDIR(mode):
        return "directory"
    if stat.S_ISREG(mode):
        return "file"
    return "other"


result = {
    "argv": sys.argv[1:],
    "environment": {
        "CODEX_MANAGED_BY_NPM": os.environ.get("CODEX_MANAGED_BY_NPM"),
        "CODEX_MANAGED_BY_BUN": os.environ.get("CODEX_MANAGED_BY_BUN"),
        "CODEX_MANAGED_PACKAGE_ROOT": os.environ.get("CODEX_MANAGED_PACKAGE_ROOT"),
        "LD_PRELOAD": os.environ.get("LD_PRELOAD"),
        "LD_LIBRARY_PATH": os.environ.get("LD_LIBRARY_PATH"),
        "CODEX_RUNTIME_CONTRACT": os.environ.get("CODEX_RUNTIME_CONTRACT"),
    },
    "fd_33": {
        "kind": fd_kind(33),
        "target": os.readlink("/proc/self/fd/33"),
        "text": os.read(33, 4096).decode("utf-8"),
    },
    "fd_34": {
        "kind": fd_kind(34),
        "target": os.readlink("/proc/self/fd/34"),
    },
}
Path(os.environ["CODEX_RUNTIME_CONTRACT_OUTPUT"]).write_text(
    json.dumps(result, sort_keys=True) + "\n",
    encoding="utf-8",
)
raise SystemExit(23)
PYTHON
chmod 755 "$CHILD_PROBE"

export CODEX_TERMUX_RESOLV_CONF="$CONTRACT_DIR/resolv.conf"
export CODEX_TERMUX_SYSTEM_CONFIG_DIR="$CONTRACT_DIR/system-config"
export CODEX_TERMUX_CERT_FILE="$CONTRACT_DIR/cert.pem"
export CODEX_TERMUX_CERT_DIR="$CONTRACT_DIR/certs"
export CODEX_TERMUX_PREFIX="$PREFIX"
export CODEX_RUNTIME_CONTRACT_OUTPUT="$CONTRACT_DIR/result.json"
export CODEX_MANAGED_BY_NPM=legacy-npm
export CODEX_MANAGED_BY_BUN=legacy-bun
export CODEX_MANAGED_PACKAGE_ROOT=legacy-root
export LD_PRELOAD=legacy-preload
export LD_LIBRARY_PATH=legacy-library

# shellcheck source=/dev/null
source "$ROOT_DIR/lib/codex-termux/exec.sh"

codex_runtime_apply_env_plan() {
    printf 'CODEX_SELF_EXE=%q\n' "$CHILD_PROBE"
    printf 'export CODEX_RUNTIME_CONTRACT=ready\n'
}

codex_prepare_system_config() {
    mkdir -p "$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_fail() {
    printf 'Error: %s\n' "$*" >&2
    return 1
}

set +e
codex_runtime_exec "$CONTRACT_DIR/runtime/codex" alpha "two words" --flag=value
status=$?
set -e
exit "$status"
