"""Atomic file helpers for Codex Termux metadata."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from .errors import TransactionError


def write_json(path: Path, data: dict[str, Any]) -> None:
    """Write JSON atomically with wrapper metadata permissions."""
    tmp = path.with_name("." + path.name + ".tmp")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp.write_text(
            json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        os.chmod(tmp, 0o600)
        os.replace(tmp, path)
    except OSError as exc:
        try:
            tmp.unlink(missing_ok=True)
        except OSError:
            pass
        raise TransactionError(f"failed to write {path}: {exc}") from exc
