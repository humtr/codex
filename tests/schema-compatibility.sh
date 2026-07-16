#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-schema-compat.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$TMP_DIR" <<'PYTHON'
from __future__ import annotations

import json
import sys
from pathlib import Path

from wrapper import registry, state
from wrapper.errors import SchemaError

root = Path(sys.argv[1])
contract = json.loads((Path(__file__).resolve().parents[2] / "config/schema-compatibility.json").read_text(encoding="utf-8")) if "__file__" in globals() else None
state_file = root / "state.json"
registry_file = root / "registry.json"

managed_state = {
    "schema": 3,
    "version": "0.142.0-linux-arm64",
    "raw_sha256": "a" * 64,
    "runtime_sha256": "b" * 64,
    "package_spec": "@openai/codex@0.142.0-linux-arm64",
    "active_tuple_id": "old-tuple",
    "wrapper_version": "260715-1",
    "wrapper_commit": "oldcommit",
    "updated_at": "2026-07-15T00:00:00+00:00",
    "verified_tuple_id": "old-tuple",
    "verified_at": "2026-07-15T00:00:00+00:00",
    "extension": {"owner": "future-writer", "flags": [1, 2, 3]},
}
state_file.write_text(json.dumps(managed_state, sort_keys=True) + "\n", encoding="utf-8")
state.write(
    state_file=state_file,
    version="0.143.0-linux-arm64",
    raw_sha256="c" * 64,
    runtime_sha256="d" * 64,
    package_spec="@openai/codex@0.143.0-linux-arm64",
    active_tuple_id="new-tuple",
    wrapper_version="260717-1",
    wrapper_commit="newcommit",
    updated_at="2026-07-17T00:00:00+00:00",
    verified_tuple_id="verified-tuple",
    verified_at="2026-07-17T00:00:00+00:00",
)
updated_state = json.loads(state_file.read_text(encoding="utf-8"))
assert updated_state["extension"] == managed_state["extension"]
assert updated_state["active_tuple_id"] == "new-tuple"

for schema in (2, 4):
    rejected = dict(managed_state)
    rejected["schema"] = schema
    payload = (json.dumps(rejected, separators=(",", ":"), sort_keys=False) + "\n").encode()
    state_file.write_bytes(payload)
    try:
        state.write(
            state_file=state_file,
            version="must-not-write",
            raw_sha256="e" * 64,
            runtime_sha256="f" * 64,
            package_spec="must-not-write",
            active_tuple_id="must-not-write",
            wrapper_version="must-not-write",
            wrapper_commit="must-not-write",
            updated_at="must-not-write",
            verified_tuple_id="must-not-write",
            verified_at="must-not-write",
        )
    except SchemaError:
        pass
    else:
        raise AssertionError(f"state schema {schema} was accepted")
    assert state_file.read_bytes() == payload

runtime_store = root / "store/runtime"
runtime_path = runtime_store / "runtime-a"
raw_path = root / "store/raw/raw-a"
runtime_path.mkdir(parents=True)
(runtime_path / "codex").write_text("runtime\n", encoding="utf-8")
raw_executable = raw_path / "vendor/aarch64-unknown-linux-musl/bin/codex"
raw_executable.parent.mkdir(parents=True)
raw_executable.write_text("raw\n", encoding="utf-8")
record_args = {
    "registry_file": registry_file,
    "version": "0.143.0-linux-arm64",
    "raw_sha256": "1" * 64,
    "runtime_sha256": "2" * 64,
    "package_spec": "@openai/codex@0.143.0-linux-arm64",
    "runtime_path": str(runtime_path),
    "wrapper_version": "260717-1",
    "wrapper_commit": "abcdef1234567890",
    "runtime_store_dir": runtime_store,
    "updated_at": "2026-07-17T00:00:00+00:00",
    "smoke_tested_at": "2026-07-17T00:00:00+00:00",
    "raw_path": str(raw_path),
}
tuple_id = registry.record(**record_args)
seed = json.loads(registry_file.read_text(encoding="utf-8"))
raw_id = seed["installs"][0]["raw_id"]
wrapper_id = seed["installs"][0]["wrapper_id"]
seed["extension"] = {"format": "future-registry", "revision": 9}
seed["installs"][0]["extension_install"] = {"keep": True}
seed["raw"][raw_id]["extension_raw"] = ["keep"]
seed["wrapper"][wrapper_id]["extension_wrapper"] = "keep"
seed["runtime"][tuple_id]["extension_runtime"] = {"keep": "yes"}
registry_file.write_text(json.dumps(seed, sort_keys=True) + "\n", encoding="utf-8")
record_args["updated_at"] = "2026-07-17T01:00:00+00:00"
record_args["smoke_tested_at"] = "2026-07-17T01:00:00+00:00"
assert registry.record(**record_args) == tuple_id
updated_registry = json.loads(registry_file.read_text(encoding="utf-8"))
assert updated_registry["extension"] == seed["extension"]
assert len([row for row in updated_registry["installs"] if row["tuple_id"] == tuple_id]) == 1
assert updated_registry["installs"][0]["extension_install"] == {"keep": True}
assert updated_registry["raw"][raw_id]["extension_raw"] == ["keep"]
assert updated_registry["wrapper"][wrapper_id]["extension_wrapper"] == "keep"
assert updated_registry["runtime"][tuple_id]["extension_runtime"] == {"keep": "yes"}

for schema in (2, 4):
    rejected = dict(updated_registry)
    rejected["schema"] = schema
    payload = (json.dumps(rejected, separators=(",", ":"), sort_keys=False) + "\n").encode()
    registry_file.write_bytes(payload)
    try:
        registry.record(**record_args)
    except SchemaError:
        pass
    else:
        raise AssertionError(f"registry schema {schema} was accepted")
    assert registry_file.read_bytes() == payload
PYTHON

printf 'schema-compatibility: ok\n'
