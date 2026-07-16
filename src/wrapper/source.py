"""Wrapper source validation and role-oriented support installation."""

from __future__ import annotations

from pathlib import Path

from . import _legacy_source as _legacy_source_module
from ._legacy_source import *  # noqa: F401,F403
from .errors import IntegrityError
from .support_layout import SupportActivation
from .support_transaction import (
    commit_support_install,
    prepare_support_install,
    rollback_support_install,
)


SHELL_DOMAINS = (
    "loader", "dispatch", "state", "fs", "ui", "prompt", "exec", "store",
    "build", "runtime", "repair", "version", "profile", "use", "remove",
    "session", "notify", "doctor",
)
WRAPPER_MODULES = (
    "__init__", "_legacy_canon", "_legacy_source", "activation", "atomic",
    "canon", "canon_policy", "cli", "cli_activation", "cli_artifacts",
    "cli_doctor", "cli_notify", "cli_product", "cli_profile", "cli_repair",
    "cli_runtime", "cli_session", "cli_store", "cli_ui", "cli_use", "doctor",
    "errors", "hashing", "install_plan", "notify", "paths", "prune",
    "registry", "release", "repair", "runtime_checks", "runtime_env", "schemas",
    "session", "source", "state", "store", "support_diagnostics",
    "support_layout", "support_transaction", "ui", "use",
)
NOTIFICATION_MODULES = (
    "__init__", "config", "hooks", "model", "provider", "service",
)
ROLE_WRAPPER_SOURCE_PATHS = tuple(
    [f"shell/{name}.sh" for name in SHELL_DOMAINS]
    + [f"src/wrapper/{name}.py" for name in WRAPPER_MODULES]
    + [f"src/wrapper/notification/{name}.py" for name in NOTIFICATION_MODULES]
    + [
        "src/codex_termux/__init__.py",
        "libexec/notify",
        "libexec/build-runtime.py",
        "libexec/bwrap-termux-compat.py",
        "libexec/bwrap-compat.py",
        "libexec/rg-termux-shim.sh",
        "libexec/rg-shim.sh",
        "native/codex-launcher.c",
        "config/layout-contracts.json",
        "config/schema-compatibility.json",
        "codex-wrapper.manifest.json",
    ]
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
