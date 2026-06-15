"""Runtime and metadata checks moved out of shell."""

from __future__ import annotations

import json
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


def parse_upstream_commands(help_text: str) -> list[str]:
    commands: set[str] = set()
    in_commands = False
    for line in help_text.splitlines():
        if not in_commands:
            if re.match(r"^\s*Commands:\s*$", line):
                in_commands = True
            continue
        if re.match(r"^\s*(Arguments|Options):\s*$", line):
            break
        match = re.match(r"^\s{2,}([a-z0-9][a-z0-9-]*)\s{2,}", line, re.I)
        if not match:
            continue
        commands.add(match.group(1))
        aliases = re.search(r"\[aliases?: ([^\]]+)\]", line)
        if aliases:
            commands.update(
                alias.strip() for alias in aliases.group(1).split(",") if alias.strip()
            )
    return sorted(commands)
