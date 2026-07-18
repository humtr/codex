"""Compatibility namespace for the role-oriented wrapper package."""
from __future__ import annotations
from pathlib import Path
_here = Path(__file__).resolve().parent
_candidate = _here.parent / "wrapper"
if not (_candidate / "__init__.py").is_file():
    raise ImportError("wrapper implementation package is unavailable")
__path__.append(str(_candidate))
del Path, _here, _candidate
