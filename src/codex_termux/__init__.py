"""Compatibility namespace for the legacy ``codex_termux`` package name."""

from __future__ import annotations

from pathlib import Path


_SRC_DIR = Path(__file__).resolve().parent.parent
_ROOT_OR_MANAGER = _SRC_DIR.parent
for _candidate in (
    _SRC_DIR / "wrapper",
    _ROOT_OR_MANAGER / "tools/codex_termux",
    _ROOT_OR_MANAGER / "codex_termux",
):
    if _candidate.is_dir():
        __path__.append(str(_candidate))

# Modules loaded through this namespace are distinct Python module objects from
# ``wrapper.*``. Install the same canon policy on the compatibility instance so
# legacy CLI execution cannot bypass role ownership checks.
from . import canon as _canon  # noqa: E402
from . import canon_policy as _canon_policy  # noqa: E402

_canon_policy.install(_canon)

__all__: tuple[str, ...] = ()
