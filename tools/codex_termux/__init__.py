"""Compatibility namespace for the legacy ``codex_termux`` package name."""

from __future__ import annotations

from pathlib import Path
import sys

_PACKAGE_DIR = Path(__file__).resolve().parent
_ROOT = _PACKAGE_DIR.parents[1] if _PACKAGE_DIR.parent.name == "tools" else _PACKAGE_DIR.parent
_SRC = _ROOT / "src"
if str(_SRC) not in sys.path:
    sys.path.insert(0, str(_SRC))
_WRAPPER = _SRC / "wrapper"
if _WRAPPER.is_dir():
    __path__.append(str(_WRAPPER))

from . import canon as _canon  # noqa: E402
from . import canon_policy as _canon_policy  # noqa: E402

_canon_policy.install(_canon)

__all__: tuple[str, ...] = ()
