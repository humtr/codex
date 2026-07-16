"""Compatibility namespace for the legacy ``codex_termux`` package name."""

from __future__ import annotations

from pathlib import Path


_SRC_DIR = Path(__file__).resolve().parent.parent
_ROOT_OR_MANAGER = _SRC_DIR.parent
for _candidate in (
    _ROOT_OR_MANAGER / "tools/codex_termux",
    _ROOT_OR_MANAGER / "codex_termux",
):
    if _candidate.is_dir():
        __path__.append(str(_candidate))

__all__: tuple[str, ...] = ()
