"""State file operations for Codex native metadata."""

from __future__ import annotations

from pathlib import Path

from . import atomic, schemas
from .errors import SchemaError


def read_field(state_file: Path, field: str) -> str:
    if not state_file.exists():
        return ""
    data = schemas.validate_state_v3(schemas.load_json_object(state_file))
    value = data.get(field, "")
    if field not in data:
        return ""
    if value is None:
        return ""
    if not isinstance(value, str):
        raise SchemaError(f"state field {field} must be a string")
    return value


def write(
    *,
    state_file: Path,
    version: str,
    raw_sha256: str,
    runtime_sha256: str,
    package_spec: str,
    active_tuple_id: str,
    wrapper_version: str,
    wrapper_commit: str,
    updated_at: str,
    verified_tuple_id: str,
    verified_at: str,
) -> None:
    data = schemas.build_state_v3(
        version=version,
        raw_sha256=raw_sha256,
        runtime_sha256=runtime_sha256,
        package_spec=package_spec,
        active_tuple_id=active_tuple_id,
        wrapper_version=wrapper_version,
        wrapper_commit=wrapper_commit,
        updated_at=updated_at,
        verified_tuple_id=verified_tuple_id,
        verified_at=verified_at,
    )
    atomic.write_json(state_file, data)
