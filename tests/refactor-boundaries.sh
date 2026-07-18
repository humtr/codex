#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$ROOT_DIR" <<'PYTHON'
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
contract_path = root / "config/refactor-boundaries.json"
contract = json.loads(contract_path.read_text(encoding="utf-8"))

assert contract["schema"] == 1
assert contract["name"] == "codex-termux-refactor-boundaries"
assert re.fullmatch(r"[0-9a-f]{40}", contract["baseline_commit"])
assert contract["baseline_commit"] == "803fef15fe17960bdf8e9f7d5d199b813d08224b"

archive = contract["archive"]
assert archive["branch"] == "archive/pr-6-role-oriented-layout-20260717"
assert archive["head_commit"] == "46c16ae58ba33b44f437adac670089f4da13104c"

rules = contract["rules"]
assert rules["single_slice_per_pull_request"] is True
assert rules["product_changes_require_focused_evidence"] is True
assert rules["main_must_remain_runnable"] is True
assert rules["force_push_shared_branches"] is False
assert rules["scope_command"] == "bash tests/refactor-scope.sh <slice> <base> <head>"

required_public_contracts = {
    "public codex argument passthrough",
    "codex termux command compatibility",
    "CODEX_TERMUX environment compatibility",
    "state and registry schema 3",
    "FD 33 resolver inheritance",
    "FD 34 system config inheritance",
    "current, verified, and raw runtime semantics",
    "managed path deletion guards",
    "release filename compatibility",
}
assert required_public_contracts <= set(contract["public_contracts"])

order = contract["dependency_order"]
slices = contract["slices"]
assert order
assert len(order) == len(set(order))
assert set(order) == set(slices)
assert order[0] == "boundary-contracts"

position = {name: index for index, name in enumerate(order)}
for name in order:
    item = slices[name]
    assert isinstance(item["purpose"], str) and item["purpose"].strip()

    dependencies = item["depends_on"]
    assert len(dependencies) == len(set(dependencies))
    for dependency in dependencies:
        assert dependency in slices, (name, dependency)
        assert position[dependency] < position[name], (name, dependency)

    allowed_paths = item["allowed_paths"]
    assert allowed_paths
    assert len(allowed_paths) == len(set(allowed_paths))
    for pattern in allowed_paths:
        assert pattern and not pattern.startswith("/"), (name, pattern)
        assert ".." not in Path(pattern).parts, (name, pattern)
        assert pattern not in {"*", "**"}, (name, pattern)

    groups = item["required_change_groups"]
    assert groups
    for group_name, patterns in groups.items():
        assert group_name and patterns
        assert len(patterns) == len(set(patterns))
        for pattern in patterns:
            assert pattern in allowed_paths, (name, group_name, pattern)

    checks = item["required_checks"]
    assert checks
    scope_prefix = f"bash tests/refactor-scope.sh {name} "
    assert any(check.startswith(scope_prefix) for check in checks), name

boundary_allowed = set(slices["boundary-contracts"]["allowed_paths"])
assert boundary_allowed == {
    "config/refactor-boundaries.json",
    "tests/refactor-boundaries.sh",
    "tests/refactor-scope.sh",
    "tests/run-portable.sh",
}

for required_path in (
    "AGENTS.md",
    "install.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "tools/codex_termux/cli.py",
    "tests/run-portable.sh",
    "tests/refactor-scope.sh",
):
    assert (root / required_path).is_file(), required_path
PYTHON

printf 'refactor-boundaries: ok\n'
