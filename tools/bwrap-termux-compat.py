#!/usr/bin/env python3
"""Compatibility entrypoint for libexec/bwrap-termux-compat.py."""

from pathlib import Path
import runpy

_HERE = Path(__file__).resolve().parent
_CANDIDATES = (
    _HERE.parent / "libexec" / "bwrap-termux-compat.py",
    _HERE / "source" / "libexec" / "bwrap-termux-compat.py",
)
for _target in _CANDIDATES:
    if _target.is_file():
        runpy.run_path(str(_target), run_name="__main__")
        break
else:
    raise SystemExit("runtime artifact implementation is unavailable")
