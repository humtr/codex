#!/usr/bin/env python3
"""Compatibility name for the installed bwrap launcher."""

from pathlib import Path
import runpy

runpy.run_path(str(Path(__file__).with_name("bwrap-termux-compat.py")), run_name="__main__")
