#!/usr/bin/env python3
"""Compatibility entrypoint for libexec/bwrap-termux-compat.py."""

from pathlib import Path
import runpy

_ROOT = Path(__file__).resolve().parent.parent
runpy.run_path(str(_ROOT / "libexec/bwrap-termux-compat.py"), run_name="__main__")
