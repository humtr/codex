#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-support-transaction.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - "$ROOT_DIR" "$TMP_DIR" <<'PYTHON'
from __future__ import annotations

import json
import sys
from pathlib import Path

from codex_termux import source

source_root = Path(sys.argv[1])
temp = Path(sys.argv[2])
wrapper_root = temp / "home/.local/lib/codex/termux"
manager = wrapper_root / "manager"
verified = wrapper_root / "verified-manager"
state_dir = temp / "home/.local/share/codex/termux"
prefix = temp / "prefix"
(prefix / "bin").mkdir(parents=True)

first = source.prepare_support_install(
    source_root=source_root,
    wrapper_root=wrapper_root,
    manager_link=manager,
    verified_manager_link=verified,
    state_dir=state_dir,
    prefix=prefix,
    installed_at="2026-07-17T00:00:00+09:00",
    wrapper_commit="firstcommit",
)
first_target = Path(first.target)
assert manager.is_symlink()
assert manager.resolve() == first_target.resolve()
assert verified.is_symlink()
assert verified.resolve() == first_target.resolve()
assert (manager / "managed.sh").is_file()
assert (manager / "codex_termux/cli.py").is_file()
assert (manager / "source/bin/install-runtime.sh").is_file()
manifest = json.loads((manager / "support-manifest.json").read_text(encoding="utf-8"))
assert manifest["support_id"] == first.support_id
assert manifest["layout"] == "legacy-compatible-v1"
source.commit_support_install(Path(first.transaction_file))
assert not Path(first.transaction_file).exists()

second = source.prepare_support_install(
    source_root=source_root,
    wrapper_root=wrapper_root,
    manager_link=manager,
    verified_manager_link=verified,
    state_dir=state_dir,
    prefix=prefix,
    installed_at="2026-07-17T00:01:00+09:00",
    wrapper_commit="secondcommit",
)
second_target = Path(second.target)
assert manager.resolve() == second_target.resolve()
assert verified.resolve() == first_target.resolve()
assert Path(second.previous).resolve() == first_target.resolve()
assert Path(second.transaction_file).is_file()

source.rollback_support_install(Path(second.transaction_file))
assert manager.resolve() == first_target.resolve()
assert verified.resolve() == first_target.resolve()
assert not second_target.exists()
assert not Path(second.transaction_file).exists()

legacy_root = temp / "legacy-root"
legacy_manager = legacy_root / "manager"
legacy_manager.mkdir(parents=True)
(legacy_manager / "marker").write_text("legacy\n", encoding="utf-8")
legacy = source.prepare_support_install(
    source_root=source_root,
    wrapper_root=legacy_root,
    manager_link=legacy_manager,
    verified_manager_link=legacy_root / "verified-manager",
    state_dir=temp / "legacy-state",
    prefix=prefix,
    installed_at="2026-07-17T00:02:00+09:00",
    wrapper_commit="migrationcommit",
)
legacy_previous = Path(legacy.previous)
assert legacy_previous.is_dir()
assert (legacy_previous / "marker").read_text(encoding="utf-8") == "legacy\n"
assert legacy_manager.is_symlink()
source.rollback_support_install(Path(legacy.transaction_file))
assert legacy_manager.is_symlink()
assert legacy_manager.resolve() == legacy_previous.resolve()
assert (legacy_manager / "marker").is_file()
PYTHON

printf 'support-transaction: ok\n'
