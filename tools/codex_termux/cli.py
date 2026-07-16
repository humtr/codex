"""Compatibility entrypoint for ``python -m codex_termux.cli``."""

from __future__ import annotations

from pathlib import Path
import sys

_ROOT = Path(__file__).resolve().parents[2]
_SRC = _ROOT / "src"
if str(_SRC) not in sys.path:
    sys.path.insert(0, str(_SRC))

from wrapper.cli import main  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(main())
