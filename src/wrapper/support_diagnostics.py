"""Diagnostics for immutable role-oriented manager and source artifacts."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, TextIO


ROLE_LAYOUT = "role-oriented-v1"
RECOVERY_SCHEMA = 2
RECOVERY_NAME = "support-recovery.json"
TRANSACTION_NAME = "support-activation.json"


def augment_report(report: dict[str, Any], manager_dir: Path) -> dict[str, Any]:
    manager_dir = manager_dir.absolute()
    wrapper_root = manager_dir.parent
    manifest_path = manager_dir / "support-manifest.json"
    role_mode = manifest_path.is_file() or _managed_target(manager_dir, wrapper_root / "support-store")
    support: dict[str, Any] = {
        "mode": "role-oriented" if role_mode else "legacy",
        "manifest": str(manifest_path),
    }
    if not role_mode:
        report["supportLayout"] = support
        return report

    manifest = _read_json(manifest_path)
    support_store = wrapper_root / "support-store"
    source_store = wrapper_root / "source-store"
    verified_manager = wrapper_root / "verified-manager"
    source_snapshot = wrapper_root / "source-snapshot"
    verified_source = wrapper_root / "verified-source-snapshot"
    manager_target = _resolved(manager_dir)
    source_target = _resolved(source_snapshot)
    support_id = str(manifest.get("support_id", ""))
    source_id = str(manifest.get("source_id", ""))
    state_dir = _state_dir(report, wrapper_root)
    recovery_file = state_dir / RECOVERY_NAME
    transaction_file = state_dir / TRANSACTION_NAME
    recovery = _read_json(recovery_file) if recovery_file.exists() else {}
    recovery_backups = sorted(
        str(path)
        for pattern in (".launcher-*.backup", ".system-config-*.backup")
        for path in state_dir.glob(pattern)
    )

    checks = {
        "support_manifest": bool(
            manifest.get("schema") == 2
            and manifest.get("layout") == ROLE_LAYOUT
            and support_id
            and source_id
        ),
        "manager_pointer": manager_dir.is_symlink(),
        "manager_in_support_store": _managed_target(manager_dir, support_store),
        "verified_manager_pointer": verified_manager.is_symlink(),
        "verified_manager_in_support_store": _managed_target(verified_manager, support_store),
        "source_snapshot_pointer": source_snapshot.is_symlink(),
        "source_snapshot_in_source_store": _managed_target(source_snapshot, source_store),
        "verified_source_pointer": verified_source.is_symlink(),
        "verified_source_in_source_store": _managed_target(verified_source, source_store),
        "support_id_match": bool(manager_target and Path(manager_target).name == support_id),
        "source_id_match": bool(source_target and Path(source_target).name == source_id),
        "role_shell": (manager_dir / "shell/loader.sh").is_file(),
        "role_python": (manager_dir / "src/wrapper/cli.py").is_file(),
        "role_libexec": (manager_dir / "libexec/build-runtime.py").is_file(),
        "support_source_alignment": bool(
            manifest.get("entrypoints", {}).get("source_snapshot") == str(source_snapshot)
        ),
        "support_recovery_clean": not recovery_file.exists(),
        "support_transaction_clean": not transaction_file.exists(),
        "support_recovery_readable": bool(
            not recovery_file.exists()
            or (
                recovery.get("schema") == RECOVERY_SCHEMA
                and recovery.get("status")
                in {
                    "prepared",
                    "switched",
                    "launcher-installed",
                    "committing",
                    "committed",
                }
            )
        ),
        "support_recovery_backups_clean": not recovery_backups,
    }
    report_checks = report.setdefault("checks", {})
    report_checks.update(checks)
    paths = report.setdefault("paths", {})
    paths.update(
        {
            "support_store": str(support_store),
            "source_store": str(source_store),
            "manager_target": manager_target,
            "verified_manager": str(verified_manager),
            "verified_manager_target": _resolved(verified_manager),
            "source_snapshot": str(source_snapshot),
            "source_snapshot_target": source_target,
            "verified_source_snapshot": str(verified_source),
            "verified_source_snapshot_target": _resolved(verified_source),
            "support_manifest": str(manifest_path),
            "support_recovery_journal": str(recovery_file),
            "support_activation_journal": str(transaction_file),
            "support_recovery_backups": recovery_backups,
        }
    )
    support.update(
        {
            "supportId": support_id,
            "sourceId": source_id,
            "layout": manifest.get("layout", ""),
            "recoveryStatus": recovery.get("status", "") if recovery else "",
            "checks": checks,
        }
    )
    report["supportLayout"] = support
    if not all(checks.values()):
        report["overallStatus"] = "fail"
    return report


def render_human(report: dict[str, Any], output: TextIO) -> None:
    support = report.get("supportLayout", {})
    if not isinstance(support, dict) or support.get("mode") != "role-oriented":
        return
    checks = support.get("checks", {})
    if not isinstance(checks, dict):
        return
    print("\nSupport layout", file=output)
    rows = (
        ("manager", "manager_pointer", "manager_in_support_store"),
        ("verified manager", "verified_manager_pointer", "verified_manager_in_support_store"),
        ("source snapshot", "source_snapshot_pointer", "source_snapshot_in_source_store"),
        ("verified source", "verified_source_pointer", "verified_source_in_source_store"),
        ("manifest", "support_manifest", "support_id_match", "source_id_match"),
        ("role files", "role_shell", "role_python", "role_libexec"),
        (
            "recovery journal",
            "support_recovery_clean",
            "support_transaction_clean",
            "support_recovery_readable",
            "support_recovery_backups_clean",
        ),
    )
    for row in rows:
        label, *keys = row
        ok = all(bool(checks.get(key)) for key in keys)
        print(f"  {'ok' if ok else 'FAIL':<4} {label}", file=output)


def _state_dir(report: dict[str, Any], wrapper_root: Path) -> Path:
    configured = os.environ.get("CODEX_TERMUX_STATE_DIR", "")
    if configured:
        return Path(configured).expanduser().absolute()
    paths = report.get("paths", {})
    if isinstance(paths, dict):
        for key in ("state_dir", "state", "state_file"):
            value = paths.get(key)
            if isinstance(value, str) and value:
                candidate = Path(value).expanduser().absolute()
                return candidate if candidate.suffix == "" else candidate.parent
    return wrapper_root.parent.parent.parent / "share/codex/termux"


def _read_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def _resolved(path: Path) -> str:
    try:
        return str(path.resolve(strict=True))
    except (OSError, RuntimeError):
        return ""


def _managed_target(link: Path, store: Path) -> bool:
    target = _resolved(link)
    if not target:
        return False
    try:
        Path(target).relative_to(store.resolve(strict=True))
        return True
    except (OSError, RuntimeError, ValueError):
        return False
