"""Path and identity helpers for managed Codex native stores."""

from __future__ import annotations

import json
import re
from pathlib import Path

from .errors import IntegrityError
from .hashing import sha256_file


RAW_BINARY = Path("vendor/aarch64-unknown-linux-musl/bin/codex")


def component(value: str, fallback: str = "unknown") -> str:
    clean = re.sub(r"[^A-Za-z0-9._+-]+", "_", value or fallback)
    return clean or fallback


def store_id(
    version: str,
    sha256: str,
    builder_sha256: str,
    bwrap_sha256: str,
    rg_sha256: str,
    tree_sha256: str = "",
) -> str:
    tree_suffix = f"+{tree_sha256[:12]}" if tree_sha256 else ""
    return (
        f"{component(version)}+{(sha256 or 'unknown')[:12]}+"
        f"{(builder_sha256 or 'unknown')[:12]}+"
        f"{(bwrap_sha256 or 'unknown')[:8]}+{(rg_sha256 or 'unknown')[:8]}"
        f"{tree_suffix}"
    )


def resolve_text(path: Path) -> str:
    try:
        return str(path.resolve())
    except (OSError, RuntimeError):
        return ""


def direct_child(path: Path, root: Path) -> Path | None:
    try:
        resolved = path.resolve()
        root_resolved = root.resolve()
    except (OSError, RuntimeError):
        return None
    if resolved.is_dir() and resolved.parent == root_resolved:
        return resolved
    return None


def require_direct_child(path: Path, root: Path, label: str) -> Path:
    child = direct_child(path, root)
    if child is None:
        raise IntegrityError(f"{label} is outside managed store: {path}")
    return child


def managed_runtime_path(
    value: str,
    runtime_store: Path,
    builder: Path,
    policy: str,
) -> Path | None:
    if not value:
        return None
    path = direct_child(Path(value), runtime_store)
    if path is None or not (path / "codex").exists():
        return None
    try:
        manifest = json.loads((path / "runtime-build.json").read_text())
        digest = sha256_file(path / "codex")
        builder_sha = sha256_file(builder)
    except (OSError, json.JSONDecodeError, IntegrityError):
        return None
    if (
        manifest.get("patch_policy") == policy
        and manifest.get("builder_sha256") == builder_sha
        and manifest.get("runtime_sha256") == digest
    ):
        return path
    return None


def managed_raw_path(value: str, raw_store: Path, expected_sha256: str) -> Path | None:
    if not value:
        return None
    path = direct_child(Path(value), raw_store)
    if path is None:
        return None
    try:
        if sha256_file(path / RAW_BINARY) == expected_sha256:
            return path
    except IntegrityError:
        return None
    return None
