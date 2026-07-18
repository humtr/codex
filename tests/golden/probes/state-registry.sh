#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
export PYTHONPATH="$ROOT_DIR/tools"

python3 -B - <<'PYTHON'
from __future__ import annotations

import os
from pathlib import Path

from codex_termux import registry, state

state_dir = Path(os.environ["CODEX_TERMUX_STATE_DIR"])
state_file = state_dir / "state.json"
registry_file = state_dir / "registry.json"
artifact_root = state_dir.parent / "artifacts"
runtime_store = artifact_root / "runtime-store"
runtime_path = runtime_store / "runtime-a"
raw_path = artifact_root / "raw-store" / "raw-a"

runtime_path.mkdir(parents=True, exist_ok=True)
(runtime_path / "codex").write_text("runtime\n", encoding="utf-8")
raw_exe = raw_path / "vendor/aarch64-unknown-linux-musl/bin/codex"
raw_exe.parent.mkdir(parents=True, exist_ok=True)
raw_exe.write_text("raw\n", encoding="utf-8")

version = "0.142.0-linux-arm64"
raw_sha256 = "a" * 64
runtime_sha256 = "b" * 64
package_spec = "@openai/codex@0.142.0-linux-arm64"
wrapper_version = "260715-1"
wrapper_commit = "abcdef1234567890"
updated_at = "2026-07-15T00:00:00+00:00"

tuple_id = registry.record(
    registry_file=registry_file,
    version=version,
    raw_sha256=raw_sha256,
    runtime_sha256=runtime_sha256,
    package_spec=package_spec,
    runtime_path=str(runtime_path),
    wrapper_version=wrapper_version,
    wrapper_commit=wrapper_commit,
    runtime_store_dir=runtime_store,
    updated_at=updated_at,
    smoke_tested_at=updated_at,
    raw_path=str(raw_path),
)
state.write(
    state_file=state_file,
    version=version,
    raw_sha256=raw_sha256,
    runtime_sha256=runtime_sha256,
    package_spec=package_spec,
    active_tuple_id=tuple_id,
    wrapper_version=wrapper_version,
    wrapper_commit=wrapper_commit,
    updated_at=updated_at,
    verified_tuple_id=tuple_id,
    verified_at=updated_at,
)
print(tuple_id)
PYTHON
