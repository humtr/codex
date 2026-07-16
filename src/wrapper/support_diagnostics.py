"""Diagnostics for immutable role-oriented manager and source artifacts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, TextIO


ROLE_LAYOUT = "role-oriented-v1"


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
        "role_libexec": (manager_dir / "libexec/notify").is_file(),
        "support_source_alignment": bool(
            manifest.get("entrypoints", {}).get("source_snapshot") == str(source_snapshot)
        ),
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
        }
    )
    support.update(
        {
            "supportId": support_id,
            "sourceId": source_id,
            "layout": manifest.get("layout", ""),
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
    )
    for row in rows:
        label, *keys = row
        ok = all(bool(checks.get(key)) for key in keys)
        print(f"  {'ok' if ok else 'FAIL':<4} {label}", file=output)


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
