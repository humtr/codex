"""Wrapper source validation and role-oriented support installation."""

from __future__ import annotations

from pathlib import Path

from . import _legacy_source as _legacy_source_module
from ._legacy_source import *  # noqa: F401,F403
from .errors import IntegrityError
from .support_layout import (
    SupportActivation,
    commit_support_install,
    prepare_support_install,
    rollback_support_install,
)


ROLE_WRAPPER_SOURCE_PATHS = (
    "shell/loader.sh",
    "shell/state.sh",
    "shell/exec.sh",
    "shell/dispatch.sh",
    "src/wrapper/__init__.py",
    "src/wrapper/cli.py",
    "src/wrapper/source.py",
    "src/wrapper/prune.py",
    "src/wrapper/notification/model.py",
    "src/wrapper/notification/service.py",
    "libexec/notify",
    "libexec/build-runtime.py",
    "libexec/bwrap-termux-compat.py",
    "libexec/rg-termux-shim.sh",
    "native/codex-launcher.c",
    "config/layout-contracts.json",
)
REQUIRED_WRAPPER_SOURCE_PATHS = tuple(
    dict.fromkeys((*_legacy_source_module.REQUIRED_WRAPPER_SOURCE_PATHS, *ROLE_WRAPPER_SOURCE_PATHS))
)


def missing_wrapper_source_paths(root: Path) -> list[str]:
    return [relative for relative in REQUIRED_WRAPPER_SOURCE_PATHS if not (root / relative).exists()]


def is_wrapper_source(root: Path) -> bool:
    return not missing_wrapper_source_paths(root)


def require_wrapper_source(root: Path, label: str) -> None:
    missing = missing_wrapper_source_paths(root)
    if missing:
        raise IntegrityError(
            f"{label} does not contain a valid wrapper source (missing: {' '.join(missing)})"
        )


def find_extracted_wrapper_source(extract_root: Path) -> Path:
    root = extract_root.resolve()
    if is_wrapper_source(root):
        return root
    for marker in root.glob("*/bin/install-runtime.sh"):
        candidate = marker.parent.parent
        if is_wrapper_source(candidate):
            return candidate
    raise IntegrityError("wrapper source root not found in extracted archive")
