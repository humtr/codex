"""Plan/apply pruning for managed immutable stores."""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any

from . import atomic, paths, schemas
from .errors import IntegrityError, TransactionError


def build_prune_plan(
    *,
    runtime_store: Path,
    raw_store: Path,
    registry_file: Path,
    state_file: Path,
    builder: Path,
    policy: str,
    retention: int,
    current_link: Path,
    verified_link: Path,
    raw_link: Path,
) -> schemas.PrunePlan:
    if retention < 1:
        raise IntegrityError("retention must be greater than zero")
    registry_data = _load_registry(registry_file)
    state_data = _load_state(state_file)
    _validate_registry_paths(registry_data, runtime_store, raw_store)
    protected_runtime = _protected_paths(
        (current_link, verified_link),
        runtime_store,
    )
    protected_raw = _protected_paths((raw_link,), raw_store)
    compatible = _compatible_runtimes(runtime_store, builder, policy)
    runtime_children = _store_children(runtime_store)
    keep_runtime = _keep_runtime_paths(
        registry_data,
        state_data,
        compatible,
        protected_runtime,
        retention,
    )
    delete_runtime = sorted(runtime_children - keep_runtime)
    kept_registry = _rewrite_registry(registry_data, keep_runtime)
    keep_raw = _keep_raw_paths(kept_registry, protected_raw, raw_store)
    delete_raw = sorted(_store_children(raw_store) - keep_raw)
    return schemas.validate_prune_plan(
        {
            "schema": 1,
            "delete_runtime_paths": delete_runtime,
            "delete_raw_paths": delete_raw,
            "keep_runtime_paths": sorted(keep_runtime),
            "keep_raw_paths": sorted(keep_raw),
            "registry_rewrite": kept_registry,
        }
    )


def apply_prune_plan(
    *,
    plan: schemas.PrunePlan,
    runtime_store: Path,
    raw_store: Path,
    registry_file: Path,
) -> schemas.PruneResult:
    plan = schemas.validate_prune_plan(dict(plan))
    deleted_runtime = _delete_paths(plan["delete_runtime_paths"], runtime_store)
    deleted_raw = _delete_paths(plan["delete_raw_paths"], raw_store)
    if registry_file.exists():
        atomic.write_json(registry_file, plan["registry_rewrite"])
    return {
        "status": "ok",
        "deleted_runtime_paths": deleted_runtime,
        "deleted_raw_paths": deleted_raw,
    }


def build_and_apply_prune(**kwargs: Any) -> schemas.PruneResult:
    plan = build_prune_plan(**kwargs)
    return apply_prune_plan(
        plan=plan,
        runtime_store=kwargs["runtime_store"],
        raw_store=kwargs["raw_store"],
        registry_file=kwargs["registry_file"],
    )


def _load_registry(path: Path) -> schemas.RegistryV3:
    if not path.exists():
        return schemas.empty_registry_v3()
    return schemas.validate_registry_v3(schemas.load_json_object(path))


def _load_state(path: Path) -> schemas.StateV3 | None:
    if not path.exists():
        return None
    return schemas.validate_state_v3(schemas.load_json_object(path))


def _validate_registry_paths(
    data: schemas.RegistryV3,
    runtime_store: Path,
    raw_store: Path,
) -> None:
    for entry in data["runtime"].values():
        paths.require_direct_child(Path(entry["path"]), runtime_store, "runtime path")
    for entry in data["raw"].values():
        paths.require_direct_child(Path(entry["path"]), raw_store, "raw path")
    for entry in data["installs"]:
        paths.require_direct_child(Path(entry["runtime_path"]), runtime_store, "install runtime path")
        paths.require_direct_child(Path(entry["raw_path"]), raw_store, "install raw path")


def _protected_paths(items: tuple[Path, ...], root: Path) -> set[str]:
    protected: set[str] = set()
    for item in items:
        child = paths.direct_child(item, root)
        if child is not None:
            protected.add(str(child))
    return protected


def _store_children(root: Path) -> set[str]:
    if not root.exists():
        return set()
    return {
        str(item.resolve())
        for item in root.iterdir()
        if item.is_dir() and not item.is_symlink()
    }


def _compatible_runtimes(root: Path, builder: Path, policy: str) -> list[Path]:
    if not root.exists():
        return []
    found = []
    for item in root.iterdir():
        runtime = paths.managed_runtime_path(str(item), root, builder, policy)
        if runtime is not None:
            found.append(runtime)
    found.sort(key=lambda item: item.stat().st_mtime, reverse=True)
    return found


def _registry_runtime_path(data: schemas.RegistryV3, tuple_id: str) -> str:
    if not tuple_id:
        return ""
    return paths.resolve_text(Path(data["runtime"].get(tuple_id, {}).get("path", "")))


def _keep_runtime_paths(
    registry_data: schemas.RegistryV3,
    state_data: schemas.StateV3 | None,
    compatible: list[Path],
    protected: set[str],
    retention: int,
) -> set[str]:
    keep = set(protected)
    compatible_text = {str(item.resolve()): item for item in compatible}
    active = registry_data.get("active_tuple_id", "")
    verified = registry_data.get("verified_tuple_id", "")
    if state_data is not None:
        verified = state_data.get("verified_tuple_id", verified)
    for tuple_id in (active, verified):
        resolved = _registry_runtime_path(registry_data, tuple_id)
        if resolved in compatible_text:
            keep.add(resolved)
    for item in compatible:
        if len(keep) >= retention:
            break
        keep.add(str(item.resolve()))
    return keep


def _rewrite_registry(
    data: schemas.RegistryV3,
    keep_runtime: set[str],
) -> schemas.RegistryV3:
    runtime = {
        key: value
        for key, value in data["runtime"].items()
        if paths.resolve_text(Path(value["path"])) in keep_runtime
    }
    installs = []
    seen = set()
    for entry in data["installs"]:
        if paths.resolve_text(Path(entry["runtime_path"])) not in keep_runtime:
            continue
        key = entry["tuple_id"]
        if key not in seen:
            installs.append(entry)
            seen.add(key)
    raw_ids = {entry["raw_id"] for entry in installs}
    wrapper_ids = {entry["wrapper_id"] for entry in installs}
    rewritten: dict[str, Any] = {
        "schema": schemas.SCHEMA_VERSION,
        "installs": installs,
        "raw": {key: value for key, value in data["raw"].items() if key in raw_ids},
        "wrapper": {
            key: value for key, value in data["wrapper"].items() if key in wrapper_ids
        },
        "runtime": runtime,
        "active_tuple_id": data.get("active_tuple_id", "") if data.get("active_tuple_id", "") in runtime else "",
        "verified_tuple_id": data.get("verified_tuple_id", "") if data.get("verified_tuple_id", "") in runtime else "",
    }
    return schemas.validate_registry_v3(rewritten)


def _keep_raw_paths(
    data: schemas.RegistryV3,
    protected_raw: set[str],
    raw_store: Path,
) -> set[str]:
    keep = set(protected_raw)
    for value in data["raw"].values():
        child = paths.direct_child(Path(value["path"]), raw_store)
        if child is not None:
            keep.add(str(child))
    return keep


def _delete_paths(items: list[str], root: Path) -> list[str]:
    deleted: list[str] = []
    for item in items:
        target = paths.require_direct_child(Path(item), root, "prune target")
        if not target.exists():
            continue
        try:
            shutil.rmtree(target)
        except OSError as exc:
            raise TransactionError(f"failed to prune {target}: {exc}") from exc
        deleted.append(str(target))
    return deleted
