#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-support-doctor.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - "$TMP_DIR" <<'PYTHON'
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from wrapper import support_diagnostics

base = Path(sys.argv[1])
root = base / "termux"
state = base / "state"
os.environ["CODEX_TERMUX_STATE_DIR"] = str(state)
state.mkdir(parents=True)
support = root / "support-store/support-a"
source = root / "source-store/source-a"
support.mkdir(parents=True)
source.mkdir(parents=True)
(support / "shell").mkdir()
(support / "src/wrapper").mkdir(parents=True)
(support / "libexec").mkdir()
(support / "shell/loader.sh").write_text("# loader\n", encoding="utf-8")
(support / "src/wrapper/cli.py").write_text("# cli\n", encoding="utf-8")
(support / "libexec/build-runtime.py").write_text("#!/usr/bin/env python3\n", encoding="utf-8")
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
assert report["checks"]["support_recovery_clean"] is True
assert report["checks"]["support_recovery_backups_clean"] is True

recovery = state / "support-recovery.json"
recovery.write_text(
    json.dumps({"schema": 2, "status": "switched"}) + "\n",
    encoding="utf-8",
)
stale = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(stale, root / "manager")
assert stale["overallStatus"] == "fail", stale
assert stale["checks"]["support_recovery_clean"] is False
assert stale["checks"]["support_recovery_readable"] is True
assert stale["supportLayout"]["recoveryStatus"] == "switched"

recovery.write_text("{broken\n", encoding="utf-8")
corrupt = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(corrupt, root / "manager")
assert corrupt["overallStatus"] == "fail", corrupt
assert corrupt["checks"]["support_recovery_readable"] is False
recovery.unlink()

backup = state / ".launcher-test.backup"
backup.write_text("backup\n", encoding="utf-8")
leftover = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(leftover, root / "manager")
assert leftover["overallStatus"] == "fail", leftover
assert leftover["checks"]["support_recovery_backups_clean"] is False
assert str(backup) in leftover["paths"]["support_recovery_backups"]
backup.unlink()

bad_source = root / "source-store/source-b"
bad_source.mkdir()
(root / "source-snapshot").unlink()
(root / "source-snapshot").symlink_to(bad_source)
bad = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(bad, root / "manager")
assert bad["overallStatus"] == "fail", bad
assert bad["checks"]["source_id_match"] is False

legacy = base / "legacy/manager"
legacy.mkdir(parents=True)
legacy_report = {"overallStatus": "ok", "checks": {}, "paths": {}}
support_diagnostics.augment_report(legacy_report, legacy)
assert legacy_report["overallStatus"] == "ok"
assert legacy_report["supportLayout"]["mode"] == "legacy"
PYTHON

printf 'support-doctor: ok\n'
