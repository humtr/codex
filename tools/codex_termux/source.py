"""Wrapper source layout validation."""

from __future__ import annotations

from pathlib import Path

from .errors import IntegrityError


REQUIRED_WRAPPER_SOURCE_PATHS = (
    "install.sh",
    "bin/install-local.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "lib/codex-termux/dispatch.sh",
    "lib/codex-termux/state.sh",
    "lib/codex-termux/profile.sh",
    "lib/codex-termux/session.sh",
    "lib/codex-termux/runtime.sh",
    "lib/codex-termux/notify.sh",
    "lib/codex-termux/doctor.sh",
    "codex-wrapper.manifest.json",
    "tools/build-runtime.py",
    "tools/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh",
    "tools/codex-turn-notify.sh",
    "tools/codex-launcher.c",
    "tools/codex_termux",
    "config/wrapper-version.env",
)


def missing_wrapper_source_paths(root: Path) -> list[str]:
    return [
        relative
        for relative in REQUIRED_WRAPPER_SOURCE_PATHS
        if not (root / relative).exists()
    ]


def is_wrapper_source(root: Path) -> bool:
    return not missing_wrapper_source_paths(root)


def require_wrapper_source(root: Path, label: str) -> None:
    missing = missing_wrapper_source_paths(root)
    if missing:
        missing_text = " ".join(missing)
        raise IntegrityError(f"{label} does not contain a valid wrapper source (missing: {missing_text})")
