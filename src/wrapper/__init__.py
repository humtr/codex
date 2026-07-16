"""Role-oriented Python package for the Codex Termux wrapper.

During the compatibility window, modules not yet moved into ``src/wrapper`` are
loaded from the legacy package directory. New implementation must be added under
this package; the legacy path is only a temporary module search fallback.
"""

from __future__ import annotations

from pathlib import Path


_PACKAGE_DIR = Path(__file__).resolve().parent
_MANAGER_OR_ROOT = _PACKAGE_DIR.parents[1]
_LEGACY_CANDIDATES = (
    _MANAGER_OR_ROOT / "tools/codex_termux",
    _MANAGER_OR_ROOT / "codex_termux",
)

for _candidate in _LEGACY_CANDIDATES:
    if _candidate.is_dir():
        __path__.append(str(_candidate))

# Canon is patched once at package import so every command path, including the
# compatibility namespace, applies the same role-oriented ownership rules.
from . import canon as _canon  # noqa: E402
from . import canon_policy as _canon_policy  # noqa: E402

_canon_policy.install(_canon)

__all__: tuple[str, ...] = ()
