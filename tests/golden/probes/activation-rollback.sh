#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
export PYTHONPATH="$ROOT_DIR/tools"

python3 -B - <<'PYTHON'
from __future__ import annotations

import json
import os
from pathlib import Path

from codex_termux import activation
from codex_termux.errors import IntegrityError
from codex_termux.schemas import ActivationPlan

sandbox = Path(os.environ["CODEX_TERMUX_STATE_DIR"]).parent
root = sandbox / "activation-rollback"
state_file = root / "state/state.json"
registry_file = root / "state/registry.json"
runtime_store = root / "store/runtime"
raw_store = root / "store/raw"
old_runtime = runtime_store / "old-runtime"
new_runtime = runtime_store / "new-runtime"
old_raw = raw_store / "old-raw"
new_raw = raw_store / "new-raw"
current_link = root / "current"
verified_link = root / "verified"
raw_link = root / "raw"

for path in (old_runtime, new_runtime, old_raw, new_raw, state_file.parent):
    path.mkdir(parents=True, exist_ok=True)
state_file.write_text('{"marker":"old-state"}\n', encoding="utf-8")
registry_file.write_text('{"marker":"old-registry"}\n', encoding="utf-8")
current_link.symlink_to(old_runtime, target_is_directory=True)
verified_link.symlink_to(old_runtime, target_is_directory=True)
raw_link.symlink_to(old_raw, target_is_directory=True)

plan = ActivationPlan(
    candidate_runtime=new_runtime,
    candidate_raw=new_raw,
    runtime_target=new_runtime,
    raw_target=new_raw,
    current_link=current_link,
    verified_link=verified_link,
    raw_link=raw_link,
    state_file=state_file,
    registry_file=registry_file,
    version="0.142.0-linux-arm64",
    raw_sha256="a" * 64,
    runtime_sha256="b" * 64,
    package_spec="@openai/codex@0.142.0-linux-arm64",
    wrapper_version="260715-1",
    wrapper_commit="abcdef123456",
    updated_at="2026-07-15T00:00:00+00:00",
    shell_bin=Path("/bin/sh"),
    shell_lib=Path("/unused"),
    probe_env={},
    cleanup_runtime_source=False,
    cleanup_raw_source=False,
)


def write_registry() -> str:
    registry_file.write_text('{"marker":"new-registry"}\n', encoding="utf-8")
    return "candidate-tuple"


def write_state(_: str) -> None:
    state_file.write_text('{"marker":"new-state"}\n', encoding="utf-8")


def fail_probe(*_: object, **__: object) -> None:
    raise IntegrityError("forced post-switch probe failure")

original_probe = activation._run_probe
caught: Exception | None = None
activation._run_probe = fail_probe
try:
    activation._activate(plan, new_runtime, new_raw, write_registry, write_state)
except Exception as exc:  # the golden output records the exact public error type and message
    caught = exc
finally:
    activation._run_probe = original_probe

if caught is None:
    raise AssertionError("activation unexpectedly succeeded")

result = {
    "error": {
        "type": type(caught).__name__,
        "message": str(caught),
    },
    "metadata": {
        "state": json.loads(state_file.read_text(encoding="utf-8")),
        "registry": json.loads(registry_file.read_text(encoding="utf-8")),
    },
    "pointers": {
        "current": os.readlink(current_link),
        "verified": os.readlink(verified_link),
        "raw": os.readlink(raw_link),
    },
    "transaction_entries": sorted(path.name for path in runtime_store.glob(".activation.*")),
}
(root / "result.json").write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
PYTHON
