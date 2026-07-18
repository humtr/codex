"""Compatibility exports for the legacy notification configuration module."""

from __future__ import annotations

from pathlib import Path
import sys

_PACKAGE_DIR = Path(__file__).resolve().parent
_ROOT = _PACKAGE_DIR.parents[1] if _PACKAGE_DIR.parent.name == "tools" else _PACKAGE_DIR.parent
_SRC = _ROOT / "src"
if str(_SRC) not in sys.path:
    sys.path.insert(0, str(_SRC))

from wrapper.notify import *  # noqa: F401,F403,E402
