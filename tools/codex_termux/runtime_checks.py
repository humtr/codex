"""Runtime and metadata checks moved out of shell."""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

from . import registry, schemas
from .errors import IntegrityError, SchemaError
from .hashing import sha256_file


def extract_pack_field(json_file: Path, field: str) -> str:
    data = json.loads(json_file.read_text(encoding="utf-8"))
    item = data[0] if isinstance(data, list) else data
    if not isinstance(item, dict):
        return ""
    value = item.get(field, "")
    return value if isinstance(value, str) else ""


def package_spec(requested: str, default_spec: str) -> str:
    if not requested or requested in {"latest", "stable"}:
        return default_spec
    if requested.startswith("@openai/codex@"):
        return requested
    if requested.endswith("linux-arm64"):
        return f"@openai/codex@{requested}"
    return f"@openai/codex@{requested}-linux-arm64"


def runtime_retention_ok(value: str) -> bool:
    try:
        return int(value) > 0
    except ValueError:
        return False


def support_source_dir(*, manager_dir: Path, runtime_dir: Path) -> Path:
    if _support_tools_available(manager_dir):
        return manager_dir
    if _support_tools_available(runtime_dir):
        return runtime_dir
    return manager_dir


def wrapper_metadata_field(*, manager_dir: Path, runtime_dir: Path, field: str) -> str:
    data: dict[str, str] = {}
    manager_file = manager_dir / "wrapper-version.env"
    runtime_file = runtime_dir / "wrapper-version.env"
    if manager_file.is_file():
        data = _read_env_metadata(manager_file)
    elif runtime_file.is_file():
        data = _read_env_metadata(runtime_file)
    if field == "version":
        return data.get("CODEX_TERMUX_WRAPPER_VERSION", "unknown") or "unknown"
    if field == "commit":
        return data.get("CODEX_TERMUX_WRAPPER_COMMIT", "unknown") or "unknown"
    raise IntegrityError(f"unknown wrapper metadata field: {field}")


def _support_tools_available(path: Path) -> bool:
    return (path / "bwrap-termux-compat.py").is_file() and (path / "rg-termux-shim.sh").is_file()


def _read_env_metadata(path: Path) -> dict[str, str]:
    data: dict[str, str] = {}
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return data
    for raw_line in lines:
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        data[key] = value
    return data


def runtime_integrity_ok(
    *,
    runtime: Path,
    manifest_path: Path,
    builder: Path,
    state_path: Path,
    patch_policy: str,
) -> bool:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        state_data = schemas.validate_state_v3(schemas.load_json_object(state_path))
        runtime_sha = sha256_file(runtime)
        return bool(
            manifest.get("patch_policy") == patch_policy
            and manifest.get("builder_sha256") == sha256_file(builder)
            and manifest.get("runtime_sha256") == runtime_sha
            and state_data.get("runtime_sha256") == runtime_sha
        )
    except (IntegrityError, OSError, SchemaError, json.JSONDecodeError):
        return False


def raw_integrity_ok(*, raw_binary: Path, state_path: Path) -> bool:
    try:
        expected = schemas.validate_state_v3(
            schemas.load_json_object(state_path)
        ).get("raw_sha256", "")
        return bool(expected and sha256_file(raw_binary) == expected)
    except (IntegrityError, OSError, SchemaError):
        return False


def support_tools_match(*, support_dir: Path, runtime_dir: Path) -> bool:
    try:
        return (
            (support_dir / "bwrap-termux-compat.py").read_bytes()
            == (runtime_dir / "codex-path/bwrap").read_bytes()
            and (support_dir / "rg-termux-shim.sh").read_bytes()
            == (runtime_dir / "codex-path/rg").read_bytes()
        )
    except OSError:
        return False


def runtime_layout_ok(*, runtime_dir: Path, runtime: Path, support_dir: Path) -> bool:
    return bool(
        _executable(runtime)
        and _executable(runtime_dir / "codex-resources/bwrap")
        and _executable(runtime_dir / "codex-path/bwrap")
        and _executable(runtime_dir / "codex-path/rg")
        and _executable(runtime_dir / "codex-path/rg.real")
        and support_tools_match(support_dir=support_dir, runtime_dir=runtime_dir)
    )


def support_layer_ok(*, managed_shell: Path, manager_dir: Path, public_codex: Path, marker: str) -> bool:
    return bool(
        _executable(managed_shell)
        and (manager_dir / "lib.sh").is_file()
        and _executable(manager_dir / "build-runtime.py")
        and _executable(manager_dir / "bwrap-termux-compat.py")
        and _executable(manager_dir / "rg-termux-shim.sh")
        and _executable(manager_dir / "codex-turn-notify.sh")
        and _path_contains(public_codex, marker.encode("utf-8"))
    )


def runtime_metadata_current(
    *,
    state_path: Path,
    registry_path: Path,
    current: Path,
    verified: Path,
    raw: Path,
    wrapper_version: str,
    wrapper_commit: str,
) -> bool:
    try:
        state_data = schemas.validate_state_v3(schemas.load_json_object(state_path))
        registry_data = registry.load(registry_path)
        active_id = state_data.get("active_tuple_id", "")
        verified_id = state_data.get("verified_tuple_id", "")
        install, active_entry, raw_entry = registry.tuple_activation_entries(
            registry_data, active_id
        )
        verified_entry = registry_data.get("runtime", {}).get(verified_id, {})
        return bool(
            active_id
            and verified_id
            and state_data.get("verified_at")
            and state_data.get("wrapper_version") == wrapper_version
            and state_data.get("wrapper_commit") == wrapper_commit
            and verified_id == active_id
            and registry_data.get("active_tuple_id") == active_id
            and registry_data.get("verified_tuple_id") == verified_id
            and Path(active_entry.get("path", "")).resolve() == current.resolve()
            and Path(verified_entry.get("path", "")).resolve() == verified.resolve()
            and Path(raw_entry.get("path", "")).resolve() == raw.resolve()
            and install.get("raw_id") == active_entry.get("raw_id")
        )
    except (IntegrityError, OSError, RuntimeError, SchemaError):
        return False


def _executable(path: Path) -> bool:
    return path.is_file() and os.access(path, os.X_OK)


def _path_contains(path: Path, needle: bytes) -> bool:
    try:
        return needle in path.read_bytes()
    except OSError:
        return False


def state_field(state_path: Path, field: str) -> str:
    """Return a string field from state.json, or empty string if unavailable."""
    try:
        data = schemas.validate_state_v3(schemas.load_json_object(state_path))
        value = data.get(field, "")
        return value if isinstance(value, str) else ""
    except Exception:
        return ""


def normalize_auto_update_mode(value: str) -> str:
    mode = value or "prompt"
    if mode in {"0", "off", "false", "no", "none"}:
        return "off"
    if mode in {"force", "auto", "always"}:
        return "force"
    return "prompt"


def auto_update_due(
    *,
    enabled: str,
    mode: str,
    now: int,
    last: str,
    interval: int,
) -> bool:
    if enabled == "0" or normalize_auto_update_mode(mode) == "off":
        return False
    try:
        last_checked = int(last or "0")
    except ValueError:
        last_checked = 0
    return now - last_checked >= interval


def failed_auto_update_due(
    *,
    record: str,
    version: str,
    now: int,
    interval: int,
) -> bool:
    if not record:
        return True
    failed_version, sep, failed_at = record.partition("\t")
    if not sep or failed_version != version:
        return True
    try:
        failed_time = int(failed_at)
    except ValueError:
        return True
    return now - failed_time >= interval


def update_prompt_decision(choice: str) -> str:
    if choice in {"y", "Y"}:
        return "apply"
    if choice in {"n", "N"}:
        return "keep"
    return "cancel"


def upstream_release_date(payload: str, version: str) -> str:
    try:
        data = json.loads(payload)
    except json.JSONDecodeError:
        return ""
    if not isinstance(data, dict):
        return ""
    value = data.get(version, "")
    if not value:
        return ""
    text = str(value).split("T", 1)[0]
    match = re.match(r"(\d{4})[-.](\d{2})[-.](\d{2})", text)
    if match:
        return "-".join(match.groups())
    digits = "".join(ch for ch in text if ch.isdigit())
    if len(digits) >= 8:
        return f"{digits[:4]}-{digits[4:6]}-{digits[6:8]}"
    return ""
