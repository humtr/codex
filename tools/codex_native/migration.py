"""Legacy store migration for Codex native wrapper metadata."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from . import atomic, schemas
from .errors import IntegrityError, SchemaError
from .hashing import sha256_file
from .store import copy_immutable_tree


def migrate_legacy_store_cache(
    *,
    legacy_store: Path,
    runtime_store: Path,
    raw_store: Path,
    registry_file: Path,
    runtime_builder: Path,
    manager_dir: Path,
    patch_policy: str,
    report_file: Path,
    completed_at: str,
) -> schemas.MigrationReportV1:
    if legacy_store.resolve() == runtime_store.parent.resolve():
        return _no_report(legacy_store, runtime_store.parent, completed_at)
    if not (legacy_store / "runtime").is_dir() or report_file.exists():
        return _read_report_or_noop(report_file, legacy_store, runtime_store, completed_at)
    data = _load_legacy_registry(registry_file)
    source_data = json.loads(json.dumps(data))
    report = _report(legacy_store, runtime_store.parent, completed_at)
    runtime_store.mkdir(parents=True, exist_ok=True)
    raw_store.mkdir(parents=True, exist_ok=True)
    _migrate_entries(
        data=data,
        source_data=source_data,
        report=report,
        legacy_store=legacy_store,
        runtime_store=runtime_store,
        raw_store=raw_store,
        runtime_builder=runtime_builder,
        manager_dir=manager_dir,
        patch_policy=patch_policy,
    )
    if report["imported"]:
        atomic.write_json(registry_file, data)
    atomic.write_json(report_file, schemas.validate_migration_report_v1(report))
    return schemas.validate_migration_report_v1(report)


def migration_status(report_file: Path, legacy_store: Path, store_root: Path) -> dict[str, Any]:
    status: dict[str, Any] = {
        "status": "not-needed",
        "report": str(report_file),
        "legacyStore": str(legacy_store),
        "imported": [],
        "skipped": [],
    }
    if report_file.exists():
        data = schemas.validate_migration_report_v1(schemas.load_json_object(report_file))
        status.update(
            {
                "status": "issues" if data.get("error") or data["skipped"] else "completed",
                "imported": data["imported"],
                "skipped": data["skipped"],
            }
        )
        if data.get("error"):
            status["error"] = data["error"]
    elif (legacy_store / "runtime").is_dir() and legacy_store.resolve() != store_root.resolve():
        status["status"] = "pending"
    return status


def _load_legacy_registry(path: Path) -> dict[str, Any]:
    data = schemas.load_json_object(path)
    if data.get("schema") != schemas.SCHEMA_VERSION:
        raise SchemaError("legacy registry schema must be 3")
    installs = data.get("installs")
    runtime = data.get("runtime")
    raw = data.get("raw")
    wrapper = data.get("wrapper")
    if not isinstance(installs, list):
        raise SchemaError("legacy registry installs must be a list")
    if not isinstance(runtime, dict):
        raise SchemaError("legacy registry runtime must be an object")
    if not isinstance(raw, dict):
        raise SchemaError("legacy registry raw must be an object")
    if not isinstance(wrapper, dict):
        raise SchemaError("legacy registry wrapper must be an object")
    for field in ("active_tuple_id", "verified_tuple_id"):
        if field in data and not isinstance(data[field], str):
            raise SchemaError(f"legacy registry {field} must be a string")
    for item in installs:
        if not isinstance(item, dict):
            raise SchemaError("legacy install entry must be an object")
        for field in ("tuple_id", "raw_id", "runtime_path", "raw_path", "runtime_sha256", "raw_sha256"):
            if not isinstance(item.get(field), str) or not item[field]:
                raise SchemaError(f"legacy install entry {field} must be a non-empty string")
    for group_name, group in (("runtime", runtime), ("raw", raw), ("wrapper", wrapper)):
        for key, value in group.items():
            if not isinstance(key, str):
                raise SchemaError(f"legacy registry {group_name} keys must be strings")
            if not isinstance(value, dict):
                raise SchemaError(f"legacy registry {group_name} entry must be an object")
    return data


def _migrate_entries(
    *,
    data: dict[str, Any],
    source_data: dict[str, Any],
    report: schemas.MigrationReportV1,
    legacy_store: Path,
    runtime_store: Path,
    raw_store: Path,
    runtime_builder: Path,
    manager_dir: Path,
    patch_policy: str,
) -> None:
    processed: set[str] = set()
    for install in source_data.get("installs", []):
        tuple_id = str(install.get("tuple_id", ""))
        if tuple_id in processed:
            continue
        processed.add(tuple_id)
        try:
            _migrate_one(
                data,
                source_data,
                install,
                legacy_store,
                runtime_store,
                raw_store,
                runtime_builder,
                manager_dir,
                patch_policy,
            )
            report["imported"].append(tuple_id)
        except Exception as exc:
            report["skipped"].append({"tuple_id": tuple_id or "unknown", "reason": str(exc)})


def _migrate_one(
    data: dict[str, Any],
    source_data: dict[str, Any],
    install: dict[str, Any],
    legacy_store: Path,
    runtime_store: Path,
    raw_store: Path,
    runtime_builder: Path,
    manager_dir: Path,
    patch_policy: str,
) -> None:
    tuple_id = str(install.get("tuple_id", ""))
    raw_id = str(install.get("raw_id", ""))
    runtime_entry = data.get("runtime", {}).get(tuple_id)
    raw_entry = data.get("raw", {}).get(raw_id)
    source_runtime_entry = source_data.get("runtime", {}).get(tuple_id)
    source_raw_entry = source_data.get("raw", {}).get(raw_id)
    if not tuple_id or not raw_id:
        raise IntegrityError("missing tuple or raw id")
    if runtime_entry is None:
        raise IntegrityError("runtime registry entry is missing")
    if raw_entry is None:
        raise IntegrityError("raw registry entry is missing")
    runtime_source = Path(install.get("runtime_path") or (source_runtime_entry or {}).get("path", ""))
    raw_source = Path(install.get("raw_path") or (source_raw_entry or {}).get("path", ""))
    _validate_legacy_tuple(
        install,
        runtime_source,
        raw_source,
        legacy_store,
        runtime_builder,
        manager_dir,
        patch_policy,
    )
    runtime_target = runtime_store / runtime_source.name
    raw_target = raw_store / raw_source.name
    copy_immutable_tree(runtime_source, runtime_target)
    copy_immutable_tree(raw_source, raw_target)
    runtime_entry["path"] = str(runtime_target)
    raw_entry["path"] = str(raw_target)
    for item in data.get("installs", []):
        if item.get("tuple_id") == tuple_id:
            item["runtime_path"] = str(runtime_target)
        if item.get("raw_id") == raw_id:
            item["raw_path"] = str(raw_target)


def _validate_legacy_tuple(
    install: dict[str, Any],
    runtime_source: Path,
    raw_source: Path,
    legacy_store: Path,
    runtime_builder: Path,
    manager_dir: Path,
    patch_policy: str,
) -> None:
    legacy_runtime = legacy_store / "runtime"
    legacy_raw = legacy_store / "raw"
    if runtime_source.resolve().parent != legacy_runtime.resolve():
        raise IntegrityError("runtime source is outside legacy runtime store")
    if raw_source.resolve().parent != legacy_raw.resolve():
        raise IntegrityError("raw source is outside legacy raw store")
    manifest = json.loads((runtime_source / "runtime-build.json").read_text())
    runtime_sha = sha256_file(runtime_source / "codex")
    raw_binary = raw_source / "vendor/aarch64-unknown-linux-musl/bin/codex"
    raw_sha = sha256_file(raw_binary)
    if manifest.get("patch_policy") != patch_policy:
        raise IntegrityError("patch policy mismatch")
    if manifest.get("builder_sha256") != sha256_file(runtime_builder):
        raise IntegrityError("builder mismatch")
    if manifest.get("runtime_sha256") != runtime_sha or install.get("runtime_sha256") != runtime_sha:
        raise IntegrityError("runtime hash mismatch")
    if manifest.get("raw_sha256") != raw_sha or install.get("raw_sha256") != raw_sha:
        raise IntegrityError("raw hash mismatch")
    _validate_support(runtime_source, manager_dir)
    raw_bytes = raw_binary.read_bytes()
    runtime_bytes = (runtime_source / "codex").read_bytes()
    if raw_bytes.replace(b"/etc/resolv.conf", b"/proc/self/fd/33") != runtime_bytes:
        raise IntegrityError("runtime is not a DNS-only raw patch")


def _validate_support(runtime_source: Path, manager_dir: Path) -> None:
    required = (
        runtime_source / "codex-resources",
        runtime_source / "codex-path/rg.real",
        runtime_source / "codex-package.json",
        manager_dir / "bwrap-termux-compat.py",
        manager_dir / "rg-termux-shim.sh",
    )
    if not all(path.exists() for path in required):
        raise IntegrityError("required runtime or manager support is missing")
    if sha256_file(runtime_source / "codex-path/bwrap") != sha256_file(manager_dir / "bwrap-termux-compat.py"):
        raise IntegrityError("bwrap support mismatch")
    if sha256_file(runtime_source / "codex-path/rg") != sha256_file(manager_dir / "rg-termux-shim.sh"):
        raise IntegrityError("rg support mismatch")


def _report(
    legacy_store: Path,
    target_store: Path,
    completed_at: str,
) -> schemas.MigrationReportV1:
    return schemas.validate_migration_report_v1(
        {
            "schema": 1,
            "completed_at": completed_at,
            "legacy_store": str(legacy_store),
            "target_store": str(target_store),
            "imported": [],
            "skipped": [],
        }
    )


def _no_report(
    legacy_store: Path,
    target_store: Path,
    completed_at: str,
) -> schemas.MigrationReportV1:
    return _report(legacy_store, target_store, completed_at)


def _read_report_or_noop(
    report_file: Path,
    legacy_store: Path,
    runtime_store: Path,
    completed_at: str,
) -> schemas.MigrationReportV1:
    if report_file.exists():
        return schemas.validate_migration_report_v1(schemas.load_json_object(report_file))
    return _report(legacy_store, runtime_store.parent, completed_at)
