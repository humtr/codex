#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-support-doctor.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$TMP_DIR" <<'PYTHON'
from __future__ import annotations

import json
import sys
from pathlib import Path

from wrapper import support_diagnostics

root = Path(sys.argv[1]) / "termux"
support = root / "support-store/support-a"
source = root / "source-store/source-a"
support.mkdir(parents=True)
source.mkdir(parents=True)
(support / "shell").mkdir()
(support / "src/wrapper").mkdir(parents=True)
(support / "libexec").mkdir()
(support / "shell/loader.sh").write_text("# loader\n", encoding="utf-8")
(support / "src/wrapper/cli.py").write_text("# cli\n", encoding="utf-8")
(support / "libexec/notify").write_text("#!/bin/sh\n", encoding="utf-8")
manifest = {
    "schema": 2,
    "layout": "role-oriented-v1",
    "support_id": "support-a",
    "source_id": "source-a",
    "entrypoints": {"source_snapshot": str(root / "source-snapshot")},
}
(support / "support-manifest.json").write_text(json.dumps(manifest) + "\n", encoding="utf-8")
(root / "manager").symlink_to(support)
(root / "verified-manager").symlink_to(support)
(root / "source-snapshot").symlink_to(source)
(root / "verified-source-snapshot").symlink_to(source)

report = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(report, root / "manager")
assert report["overallStatus"] == "ok", report
assert report["supportLayout"]["mode"] == "role-oriented"
assert all(report["supportLayout"]["checks"].values()), report
assert report["paths"]["manager_target"].endswith("support-a")
assert report["paths"]["source_snapshot_target"].endswith("source-a")

bad_source = root / "source-store/source-b"
bad_source.mkdir()
(root / "source-snapshot").unlink()
(root / "source-snapshot").symlink_to(bad_source)
bad = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(bad, root / "manager")
assert bad["overallStatus"] == "fail", bad
assert bad["checks"]["source_id_match"] is False

legacy = Path(sys.argv[1]) / "legacy/manager"
legacy.mkdir(parents=True)
legacy_report = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(legacy_report, legacy)
assert legacy_report["overallStatus"] == "ok"
assert legacy_report["supportLayout"]["mode"] == "legacy"
PYTHON

printf 'support-doctor: ok\n'
