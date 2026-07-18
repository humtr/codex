"""Compatibility namespace for the role-oriented wrapper package."""
from __future__ import annotations
from pathlib import Path
_here = Path(__file__).resolve().parent
_candidates = (
    _here.parent.parent / "src" / "wrapper",
    _here.parent / "source" / "src" / "wrapper",
    _here.parent.parent / "source" / "src" / "wrapper",
)
for _candidate in _candidates:
    if (_candidate / "__init__.py").is_file():
        __path__.append(str(_candidate))
        break
else:
    raise ImportError("wrapper implementation package is unavailable")
del Path, _here, _candidates, _candidate
