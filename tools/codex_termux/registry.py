"""Registry file operations for Codex Termux metadata."""

from __future__ import annotations

import re
from pathlib import Path

from . import atomic, paths, schemas
from .errors import SchemaError




def _component(value: str, fallback: str = "unknown") -> str:
    clean = re.sub(r"[^A-Za-z0-9._+-]+", "_", value or fallback)
    return clean or fallback


def _first_value(*values: str) -> str:
    for value in values:
        if isinstance(value, str) and value:
            return value
    return ""


def display_runtime_date(value: str) -> str:
    if not value:
        return ""
    match = re.match(r"(\d{4})-(\d{2})-(\d{2})", value)
    if match:
        return "-".join(match.groups())
    digits = "".join(ch for ch in value if ch.isdigit())
    if len(digits) >= 8:
        return f"{digits[:4]}-{digits[4:6]}-{digits[6:8]}"
    return ""


def display_wrapper_version(value: str) -> str:
    if not value:
        return ""
    match = re.fullmatch(r"(\d{2})(\d{2})(\d{2})-(\d+)", value)
    if match:
        year, month, day, rev = match.groups()
        return f"20{year}-{month}-{day} (r{rev})"
    match = re.fullmatch(r"(\d{4})(\d{2})(\d{2})-(\d+)", value)
    if match:
        year, month, day, rev = match.groups()
        return f"{year}-{month}-{day} (r{rev})"
    return value


def _managed_runtime_path(value: str, runtime_store_dir: Path) -> bool:
    if not value:
        return False
    try:
        path_value = Path(value).resolve()
    except (OSError, RuntimeError):
        return False
    return (
        path_value.exists()
        and (path_value / "codex").exists()
        and path_value.parent == runtime_store_dir.resolve()
    )


def _load_or_create(registry_file: Path) -> schemas.RegistryV3:
    if not registry_file.exists():
        return schemas.empty_registry_v3()
    data = schemas.load_json_object(registry_file)
    return schemas.validate_registry_v3(data)


def load(registry_file: Path) -> schemas.RegistryV3:
    return schemas.validate_registry_v3(schemas.load_json_object(registry_file))


def write(registry_file: Path, data: schemas.RegistryV3) -> None:
    schemas.validate_registry_v3(data)
    atomic.write_json(registry_file, data)


def _filter_managed(
    data: schemas.RegistryV3, runtime_store_dir: Path
) -> schemas.RegistryV3:
    data["installs"] = [
        item
        for item in data.get("installs", [])
        if _managed_runtime_path(item.get("runtime_path", ""), runtime_store_dir)
    ]
    data["runtime"] = {
        key: value
        for key, value in data.get("runtime", {}).items()
        if _managed_runtime_path(value.get("path", ""), runtime_store_dir)
    }
    return data


def record(
    *,
    registry_file: Path,
    version: str,
    raw_sha256: str,
    runtime_sha256: str,
    package_spec: str,
    runtime_path: str,
    wrapper_version: str,
    wrapper_commit: str,
    runtime_store_dir: Path,
    updated_at: str,
    smoke_tested_at: str,
    raw_path: str,
) -> str:
    raw_id = f"raw-{_component(version)}-{_component(raw_sha256[:12])}"
    wrapper_id = (
        f"wrapper-{_component(wrapper_version)}-{_component(wrapper_commit[:12])}"
    )
    tuple_id = f"{raw_id}__{wrapper_id}"
    data = _filter_managed(_load_or_create(registry_file), runtime_store_dir)
    created_at = _existing_created_at(
        data,
        runtime_path=runtime_path,
        raw_path=raw_path,
        runtime_sha256=runtime_sha256,
        raw_sha256=raw_sha256,
    ) or updated_at
    entry = schemas.validate_install_entry(
        {
            "version": version,
            "raw_sha256": raw_sha256,
            "runtime_sha256": runtime_sha256,
            "package_spec": package_spec,
            "runtime_path": runtime_path,
            "raw_path": raw_path,
            "updated_at": updated_at,
            "created_at": created_at,
            "raw_id": raw_id,
            "wrapper_id": wrapper_id,
            "tuple_id": tuple_id,
        }
    )
    data["raw"][raw_id] = schemas.validate_raw_entry(
        {
            "version": version,
            "sha256": raw_sha256,
            "package_spec": package_spec,
            "path": raw_path,
            "updated_at": updated_at,
        }
    )
    data["wrapper"][wrapper_id] = schemas.validate_wrapper_entry(
        {
            "version": wrapper_version,
            "commit": wrapper_commit,
            "repo": "local/codex-termux",
            "updated_at": updated_at,
        }
    )
    previous = data["runtime"].get(tuple_id, {})
    runtime_entry: dict[str, str] = {
        "raw_id": raw_id,
        "wrapper_id": wrapper_id,
        "runtime_sha256": runtime_sha256,
        "path": runtime_path,
        "updated_at": updated_at,
        "created_at": created_at,
    }
    if smoke_tested_at:
        runtime_entry["smoke_tested_at"] = smoke_tested_at
    elif previous.get("smoke_tested_at"):
        runtime_entry["smoke_tested_at"] = previous["smoke_tested_at"]
    data["runtime"][tuple_id] = schemas.validate_runtime_entry(runtime_entry)
    data["active_tuple_id"] = tuple_id
    if smoke_tested_at:
        data["verified_tuple_id"] = tuple_id
    data["installs"].insert(0, entry)
    data["installs"] = data["installs"][:20]
    schemas.validate_registry_v3(data)
    atomic.write_json(registry_file, data)
    return tuple_id


def tuple_activation_entries(
    data: schemas.RegistryV3,
    tuple_id: str,
) -> tuple[schemas.InstallEntry, schemas.RuntimeEntry, schemas.RawEntry]:
    runtime = data["runtime"].get(tuple_id)
    install = _find_install(data["installs"], tuple_id)
    if runtime is None or install is None:
        raise SchemaError("tuple not found in registry")
    raw = data["raw"].get(install["raw_id"])
    if raw is None:
        raise SchemaError("tuple raw entry not found in registry")
    return install, runtime, raw


def activate_existing_tuple(
    registry_file: Path,
    tuple_id: str,
) -> tuple[schemas.InstallEntry, schemas.RuntimeEntry, schemas.RawEntry]:
    data = load(registry_file)
    entries = tuple_activation_entries(data, tuple_id)
    data["active_tuple_id"] = tuple_id
    data["verified_tuple_id"] = tuple_id
    write(registry_file, data)
    return entries


def _find_install(
    installs: list[schemas.InstallEntry], tuple_id: str
) -> schemas.InstallEntry | None:
    for item in installs:
        if item.get("tuple_id") == tuple_id:
            return item
    return None


def _existing_created_at(
    data: schemas.RegistryV3,
    *,
    runtime_path: str,
    raw_path: str,
    runtime_sha256: str,
    raw_sha256: str,
) -> str:
    for item in data.get("installs", []):
        if (
            item.get("runtime_path") == runtime_path
            and item.get("raw_path") == raw_path
            and item.get("runtime_sha256") == runtime_sha256
            and item.get("raw_sha256") == raw_sha256
        ):
            return _first_value(item.get("created_at", ""), item.get("updated_at", ""))
    return ""


def _resolved_text(path: Path) -> str:
    try:
        return str(path.resolve())
    except (OSError, RuntimeError):
        return str(path)


def _dedupe_row_key(row: dict[str, str]) -> tuple[str, str, str, str]:
    return (
        row.get("runtime_path_resolved", row.get("runtime_path", "")),
        row.get("raw_path_resolved", row.get("raw_path", "")),
        row.get("runtime_sha256", ""),
        row.get("raw_sha256", ""),
    )


def _should_replace_row(current: dict[str, str], candidate: dict[str, str]) -> bool:
    if candidate.get("active") == "1" and current.get("active") != "1":
        return True
    if candidate.get("verified") == "1" and current.get("verified") != "1":
        return True
    return False


def list_usable_runtimes(
    *,
    registry_file: Path,
    latest: str,
    runtime_store_dir: Path,
    runtime_builder: Path,
    patch_policy: str,
) -> list[dict[str, str]]:
    data = load(registry_file) if registry_file.exists() else schemas.empty_registry_v3()
    raw_store = runtime_store_dir.parent / "raw"
    verified_tuple_id = data.get("verified_tuple_id", "")
    rows: list[dict[str, str]] = []
    seen_by_artifact: dict[tuple[str, str, str, str], int] = {}
    for entry in data["installs"]:
        runtime = paths.managed_runtime_path(
            entry["runtime_path"],
            runtime_store_dir,
            runtime_builder,
            patch_policy,
        )
        raw = paths.managed_raw_path(entry["raw_path"], raw_store, entry["raw_sha256"])
        if runtime is None or raw is None:
            continue
        runtime_entry = data.get("runtime", {}).get(entry["tuple_id"], {})
        wrapper_entry = data.get("wrapper", {}).get(entry["wrapper_id"], {})
        row = dict(entry)
        row["kind"] = "cached"
        row["active"] = "1" if entry["tuple_id"] == data.get("active_tuple_id", "") else "0"
        row["verified"] = "1" if entry["tuple_id"] == verified_tuple_id else "0"
        row["runtime_path_resolved"] = _resolved_text(runtime)
        row["raw_path_resolved"] = _resolved_text(raw)
        row["wrapper_version"] = wrapper_entry.get("version", "")
        row["wrapper_commit"] = wrapper_entry.get("commit", "")
        row["wrapper_updated_at"] = wrapper_entry.get("updated_at", "")
        row["created_at"] = _first_value(
            row.get("created_at", ""),
            runtime_entry.get("created_at", ""),
            runtime_entry.get("updated_at", ""),
            entry.get("updated_at", ""),
        )
        dedupe_key = _dedupe_row_key(row)
        index = seen_by_artifact.get(dedupe_key)
        if index is None:
            seen_by_artifact[dedupe_key] = len(rows)
            rows.append(row)
            continue
        if _should_replace_row(rows[index], row):
            rows[index] = row
    if latest and latest not in {row["version"] for row in rows if row.get("kind") == "cached"}:
        rows.append(
            {
                "kind": "remote",
                "version": latest,
                "runtime_sha256": "",
                "raw_sha256": "",
                "package_spec": f"@openai/codex@{latest}",
                "runtime_path": "npm:linux-arm64",
                "raw_path": "",
                "tuple_id": "",
                "raw_id": "",
                "wrapper_id": "latest wrapper",
                "active": "0",
                "verified": "0",
                "updated_at": "",
                "created_at": "",
                "wrapper_version": "",
                "wrapper_commit": "",
                "wrapper_updated_at": "",
            }
        )
    return rows


def menu_rows(
    *,
    registry_file: Path,
    latest: str,
    runtime_store_dir: Path,
    runtime_builder: Path,
    patch_policy: str,
) -> tuple[dict[str, str] | None, list[dict[str, str]]]:
    rows = list_usable_runtimes(
        registry_file=registry_file,
        latest=latest,
        runtime_store_dir=runtime_store_dir,
        runtime_builder=runtime_builder,
        patch_policy=patch_policy,
    )
    latest_row = next((row for row in rows if row.get("kind") == "remote"), None)
    remaining = [row for row in rows if row is not latest_row]
    return latest_row, remaining


def resolve_runtime_selection(
    *,
    registry_file: Path,
    choice: str,
    latest: str,
    runtime_store_dir: Path,
    runtime_builder: Path,
    patch_policy: str,
) -> dict[str, str]:
    latest_row, remaining = menu_rows(
        registry_file=registry_file,
        latest=latest,
        runtime_store_dir=runtime_store_dir,
        runtime_builder=runtime_builder,
        patch_policy=patch_policy,
    )
    if choice == "0" and latest_row is not None:
        return latest_row
    if choice.isdigit() and 1 <= int(choice) <= len(remaining):
        return remaining[int(choice) - 1]
    for row in ([latest_row] if latest_row is not None else []) + remaining:
        short_version = display_version(row.get("version", "unknown"))
        if choice in (row["version"], short_version, row["runtime_sha256"][:12]):
            return row
    raise SchemaError("runtime selection not found")


def active_runtime_created_at(registry_file: Path) -> str:
    if not registry_file.exists():
        return ""
    data = load(registry_file)
    tuple_id = data.get("active_tuple_id", "")
    if not tuple_id:
        return ""
    install = _find_install(data.get("installs", []), tuple_id)
    runtime = data.get("runtime", {}).get(tuple_id, {})
    return display_runtime_date(
        _first_value(
            runtime.get("created_at", ""),
            runtime.get("updated_at", ""),
            install.get("created_at", "") if install else "",
            install.get("updated_at", "") if install else "",
        )
    )


def display_version(version: str) -> str:
    suffix = "-linux-arm64"
    if version.endswith(suffix):
        return version[: -len(suffix)]
    return version
