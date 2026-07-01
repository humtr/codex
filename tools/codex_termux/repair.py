"""Repair diagnosis for the Termux wrapper runtime."""

from __future__ import annotations

import os
from dataclasses import asdict, dataclass
from pathlib import Path

from . import registry, runtime_checks, schemas
from .errors import IntegrityError, SchemaError


ACTION_NONE = "none"
ACTION_REFRESH_SUPPORT = "refresh_support"
ACTION_REFRESH_METADATA = "refresh_metadata"
ACTION_RESTORE_VERIFIED = "restore_verified"
ACTION_REBUILD_CACHED = "rebuild_cached"
ACTION_UNRECOVERABLE = "unrecoverable"


@dataclass(frozen=True)
class RepairInputs:
    managed_shell: Path
    manager_dir: Path
    public_codex: Path
    marker: str
    runtime_dir: Path
    runtime: Path
    support_dir: Path
    manifest_path: Path
    builder: Path
    state_path: Path
    registry_path: Path
    current: Path
    verified: Path
    raw: Path
    raw_binary: Path
    patch_policy: str
    wrapper_version: str
    wrapper_commit: str


@dataclass(frozen=True)
class RepairDiagnosis:
    support_ok: bool
    runtime_layout_ok: bool
    runtime_integrity_ok: bool
    runtime_ok: bool
    raw_ok: bool
    metadata_current: bool
    verified_rollback_available: bool
    action: str

    def to_dict(self) -> dict[str, object]:
        return asdict(self)


def diagnose(inputs: RepairInputs) -> RepairDiagnosis:
    support_ok = runtime_checks.support_layer_ok(
        managed_shell=inputs.managed_shell,
        manager_dir=inputs.manager_dir,
        public_codex=inputs.public_codex,
        marker=inputs.marker,
    )
    runtime_layout_ok = runtime_checks.runtime_layout_ok(
        runtime_dir=inputs.runtime_dir,
        runtime=inputs.runtime,
        support_dir=inputs.support_dir,
    )
    runtime_integrity_ok = inputs.state_path.is_file() and runtime_checks.runtime_integrity_ok(
        runtime=inputs.runtime,
        manifest_path=inputs.manifest_path,
        builder=inputs.builder,
        state_path=inputs.state_path,
        patch_policy=inputs.patch_policy,
    )
    runtime_ok = runtime_layout_ok and runtime_integrity_ok
    raw_ok = _executable(inputs.raw_binary) and runtime_checks.raw_integrity_ok(
        raw_binary=inputs.raw_binary,
        state_path=inputs.state_path,
    )
    metadata_current = runtime_ok and runtime_checks.runtime_metadata_current(
        state_path=inputs.state_path,
        registry_path=inputs.registry_path,
        current=inputs.current,
        verified=inputs.verified,
        raw=inputs.raw,
        wrapper_version=inputs.wrapper_version,
        wrapper_commit=inputs.wrapper_commit,
    )
    verified_rollback_available = (not runtime_ok) and _verified_rollback_available(
        current=inputs.current,
        verified=inputs.verified,
        state_path=inputs.state_path,
        registry_path=inputs.registry_path,
    )
    action = action_from_checks(
        support_ok=support_ok,
        runtime_ok=runtime_ok,
        metadata_current=metadata_current,
        verified_rollback_available=verified_rollback_available,
        raw_ok=raw_ok,
    )
    return RepairDiagnosis(
        support_ok=support_ok,
        runtime_layout_ok=runtime_layout_ok,
        runtime_integrity_ok=runtime_integrity_ok,
        runtime_ok=runtime_ok,
        raw_ok=raw_ok,
        metadata_current=metadata_current,
        verified_rollback_available=verified_rollback_available,
        action=action,
    )


def action_from_checks(
    *,
    support_ok: bool,
    runtime_ok: bool,
    metadata_current: bool,
    verified_rollback_available: bool,
    raw_ok: bool,
) -> str:
    if not support_ok:
        return ACTION_REFRESH_SUPPORT
    if runtime_ok:
        return ACTION_NONE if metadata_current else ACTION_REFRESH_METADATA
    if verified_rollback_available:
        return ACTION_RESTORE_VERIFIED
    if raw_ok:
        return ACTION_REBUILD_CACHED
    return ACTION_UNRECOVERABLE


def _verified_rollback_available(
    *,
    current: Path,
    verified: Path,
    state_path: Path,
    registry_path: Path,
) -> bool:
    try:
        if not (verified.exists() or verified.is_symlink()):
            return False
        state_data = schemas.validate_state_v3(schemas.load_json_object(state_path))
        verified_id = state_data.get("verified_tuple_id", "")
        if not verified_id:
            return False
        registry_data = registry.load(registry_path)
        _install, runtime_entry, raw_entry = registry.tuple_activation_entries(
            registry_data, verified_id
        )
        verified_runtime = verified.resolve()
        runtime_path = Path(runtime_entry.get("path", "")).resolve()
        raw_path = Path(raw_entry.get("path", "")).resolve()
        return bool(
            runtime_path == verified_runtime
            and _executable(verified_runtime / "codex")
            and raw_path.exists()
            and current.resolve() != verified_runtime
        )
    except (IntegrityError, OSError, RuntimeError, SchemaError):
        return False


def _executable(path: Path) -> bool:
    return path.is_file() and os.access(path, os.X_OK)
