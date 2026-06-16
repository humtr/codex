"""Schema definitions and validators for Codex native metadata."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, NotRequired, TypedDict

from .errors import SchemaError


SCHEMA_VERSION = 3


class StateV3(TypedDict):
    schema: int
    version: str
    raw_sha256: str
    runtime_sha256: str
    package_spec: str
    active_tuple_id: str
    wrapper_version: str
    wrapper_commit: str
    updated_at: str
    verified_tuple_id: NotRequired[str]
    verified_at: NotRequired[str]

class RawEntry(TypedDict):
    version: str
    sha256: str
    package_spec: str
    path: str
    updated_at: str

class WrapperEntry(TypedDict):
    version: str
    commit: str
    repo: str
    updated_at: str

class RuntimeEntry(TypedDict):
    raw_id: str
    wrapper_id: str
    runtime_sha256: str
    path: str
    updated_at: str
    smoke_tested_at: NotRequired[str]

class InstallEntry(TypedDict):
    version: str
    raw_sha256: str
    runtime_sha256: str
    package_spec: str
    runtime_path: str
    raw_path: str
    updated_at: str
    raw_id: str
    wrapper_id: str
    tuple_id: str

class RegistryV3(TypedDict):
    schema: int
    installs: list[InstallEntry]
    raw: dict[str, RawEntry]
    wrapper: dict[str, WrapperEntry]
    runtime: dict[str, RuntimeEntry]
    active_tuple_id: NotRequired[str]
    verified_tuple_id: NotRequired[str]

class BuildManifestV2(TypedDict, total=False):
    schema: int
    version: str
    raw_sha256: str
    runtime_sha256: str
    builder_sha256: str
    patch_policy: str

class DoctorReport(TypedDict, total=False):
    schema: int
    overallStatus: str
    version: str
    checks: dict[str, bool]
    paths: dict[str, str]

@dataclass(frozen=True)
class ActivationPlan:
    candidate_runtime: Path
    candidate_raw: Path
    runtime_target: Path
    raw_target: Path
    current_link: Path
    verified_link: Path
    raw_link: Path
    state_file: Path
    registry_file: Path
    version: str
    raw_sha256: str
    runtime_sha256: str
    package_spec: str
    wrapper_version: str
    wrapper_commit: str
    updated_at: str
    shell_bin: Path
    shell_lib: Path
    probe_env: Mapping[str, str]
    cleanup_runtime_source: bool = True
    cleanup_raw_source: bool = False

@dataclass(frozen=True)
class ActivationResult:
    tuple_id: str
    runtime_path: Path
    raw_path: Path

class PrunePlan(TypedDict, total=False):
    schema: int
    delete_runtime_paths: list[str]
    delete_raw_paths: list[str]
    keep_runtime_paths: list[str]
    keep_raw_paths: list[str]
    registry_rewrite: RegistryV3

class PruneResult(TypedDict, total=False):
    status: str
    deleted_runtime_paths: list[str]
    deleted_raw_paths: list[str]
    error: str

def load_json_object(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SchemaError(f"{path} is malformed JSON: {exc}") from exc
    except OSError as exc:
        raise SchemaError(f"failed to read {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise SchemaError(f"{path} must contain a JSON object")
    return data

def require_string(data: dict[str, Any], field: str) -> str:
    value = data.get(field)
    if not isinstance(value, str) or value == "":
        raise SchemaError(f"required field {field} must be a non-empty string")
    return value

def require_present_string(data: dict[str, Any], field: str) -> str:
    if field not in data:
        raise SchemaError(f"required field {field} must be present")
    value = data[field]
    if not isinstance(value, str):
        raise SchemaError(f"required field {field} must be a string")
    return value

def optional_string(data: dict[str, Any], field: str) -> str:
    value = data.get(field, "")
    if not isinstance(value, str):
        raise SchemaError(f"optional field {field} must be a string")
    return value

def validate_state_v3(data: dict[str, Any]) -> StateV3:
    if data.get("schema") != SCHEMA_VERSION:
        raise SchemaError("state schema must be 3")
    required_non_empty = (
        "version",
        "raw_sha256",
        "runtime_sha256",
        "package_spec",
        "wrapper_version",
        "wrapper_commit",
        "updated_at",
    )
    for field in required_non_empty:
        require_string(data, field)
    require_present_string(data, "active_tuple_id")
    optional_string(data, "verified_tuple_id")
    optional_string(data, "verified_at")
    return data  # type: ignore[return-value]


def build_state_v3(
    *,
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
) -> StateV3:
    data: dict[str, Any] = {
        "schema": SCHEMA_VERSION,
        "version": version,
        "raw_sha256": raw_sha256,
        "runtime_sha256": runtime_sha256,
        "package_spec": package_spec,
        "active_tuple_id": active_tuple_id,
        "wrapper_version": wrapper_version,
        "wrapper_commit": wrapper_commit,
        "updated_at": updated_at,
        "verified_tuple_id": verified_tuple_id,
        "verified_at": verified_at,
    }
    return validate_state_v3(data)


def validate_registry_v3(data: dict[str, Any]) -> RegistryV3:
    if data.get("schema") != SCHEMA_VERSION:
        raise SchemaError("registry schema must be 3")
    installs = data.get("installs")
    raw = data.get("raw")
    wrapper = data.get("wrapper")
    runtime = data.get("runtime")
    if not isinstance(installs, list):
        raise SchemaError("registry installs must be a list")
    if not isinstance(raw, dict):
        raise SchemaError("registry raw must be an object")
    if not isinstance(wrapper, dict):
        raise SchemaError("registry wrapper must be an object")
    if not isinstance(runtime, dict):
        raise SchemaError("registry runtime must be an object")
    optional_string(data, "active_tuple_id")
    optional_string(data, "verified_tuple_id")
    for item in installs:
        validate_install_entry(item)
    for key, value in raw.items():
        if not isinstance(key, str):
            raise SchemaError("registry raw keys must be strings")
        validate_raw_entry(value)
    for key, value in wrapper.items():
        if not isinstance(key, str):
            raise SchemaError("registry wrapper keys must be strings")
        validate_wrapper_entry(value)
    for key, value in runtime.items():
        if not isinstance(key, str):
            raise SchemaError("registry runtime keys must be strings")
        validate_runtime_entry(value)
    return data  # type: ignore[return-value]

def empty_registry_v3() -> RegistryV3:
    return validate_registry_v3(
        {"schema": SCHEMA_VERSION, "installs": [], "raw": {}, "wrapper": {}, "runtime": {}}
    )

def validate_raw_entry(value: Any) -> RawEntry:
    if not isinstance(value, dict):
        raise SchemaError("raw entry must be an object")
    for field in ("version", "sha256", "package_spec", "path", "updated_at"):
        require_present_string(value, field)
    return value  # type: ignore[return-value]

def validate_wrapper_entry(value: Any) -> WrapperEntry:
    if not isinstance(value, dict):
        raise SchemaError("wrapper entry must be an object")
    for field in ("version", "commit", "repo", "updated_at"):
        require_present_string(value, field)
    return value  # type: ignore[return-value]

def validate_runtime_entry(value: Any) -> RuntimeEntry:
    if not isinstance(value, dict):
        raise SchemaError("runtime entry must be an object")
    for field in ("raw_id", "wrapper_id", "runtime_sha256", "path", "updated_at"):
        require_present_string(value, field)
    optional_string(value, "smoke_tested_at")
    return value  # type: ignore[return-value]

def validate_install_entry(value: Any) -> InstallEntry:
    if not isinstance(value, dict):
        raise SchemaError("install entry must be an object")
    required = (
        "version",
        "raw_sha256",
        "runtime_sha256",
        "package_spec",
        "runtime_path",
        "raw_path",
        "updated_at",
        "raw_id",
        "wrapper_id",
        "tuple_id",
    )
    for field in required:
        require_present_string(value, field)
    return value  # type: ignore[return-value]


def validate_prune_plan(value: dict[str, Any]) -> PrunePlan:
    if value.get("schema") != 1:
        raise SchemaError("prune plan schema must be 1")
    for field in (
        "delete_runtime_paths",
        "delete_raw_paths",
        "keep_runtime_paths",
        "keep_raw_paths",
    ):
        items = value.get(field)
        if not isinstance(items, list) or not all(isinstance(item, str) for item in items):
            raise SchemaError(f"prune plan {field} must be a list of strings")
    registry_rewrite = value.get("registry_rewrite")
    if not isinstance(registry_rewrite, dict):
        raise SchemaError("prune plan registry_rewrite must be an object")
    validate_registry_v3(registry_rewrite)
    return value  # type: ignore[return-value]
