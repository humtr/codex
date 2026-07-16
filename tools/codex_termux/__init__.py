"""Compatibility namespace for the legacy ``codex_termux`` package name."""

from __future__ import annotations

from pathlib import Path
import sys

_ROOT = Path(__file__).resolve().parents[2]
_SRC = _ROOT / "src"
if str(_SRC) not in sys.path:
    sys.path.insert(0, str(_SRC))
_WRAPPER = _SRC / "wrapper"
if _WRAPPER.is_dir():
    __path__.append(str(_WRAPPER))

__all__: tuple[str, ...] = ()
