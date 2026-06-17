"""Hashing primitives for immutable Codex Termux artifacts."""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path
from typing import Protocol

from .errors import IntegrityError


CHUNK_SIZE = 1024 * 1024


class HashWriter(Protocol):
    def update(self, data: bytes) -> None: ...


def sha256_file(path: Path) -> str:
    """Return the SHA-256 digest of a regular file."""
    try:
        if not path.is_file():
            raise IntegrityError(f"hash source is not a regular file: {path}")
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(CHUNK_SIZE), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except OSError as exc:
        raise IntegrityError(f"failed to hash {path}: {exc}") from exc


def tree_digest(root: Path) -> str:
    """Hash a directory tree including relative paths, modes, and symlinks."""
    try:
        root_mode = root.lstat().st_mode
        if not stat.S_ISDIR(root_mode) or stat.S_ISLNK(root_mode):
            raise IntegrityError(f"tree root must be a directory: {root}")
        digest = hashlib.sha256()
        digest.update(b".\0")
        digest.update(_mode_bytes(root_mode) + b"\0D\0")
        for path in sorted(
            root.rglob("*"),
            key=lambda item: item.relative_to(root).as_posix(),
        ):
            _update_tree_entry(digest, root, path)
        return digest.hexdigest()
    except OSError as exc:
        raise IntegrityError(f"failed to hash tree {root}: {exc}") from exc


def _mode_bytes(mode: int) -> bytes:
    return f"{stat.S_IMODE(mode):04o}".encode("ascii")


def _update_tree_entry(
    digest: HashWriter,
    root: Path,
    path: Path,
) -> None:
    mode = path.lstat().st_mode
    digest.update(path.relative_to(root).as_posix().encode("utf-8") + b"\0")
    digest.update(_mode_bytes(mode) + b"\0")
    if stat.S_ISLNK(mode):
        digest.update(b"L\0" + os.readlink(path).encode("utf-8") + b"\0")
        return
    if stat.S_ISDIR(mode):
        digest.update(b"D\0")
        return
    if stat.S_ISREG(mode):
        digest.update(b"F\0")
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(CHUNK_SIZE), b""):
                digest.update(chunk)
        return
    raise IntegrityError(f"unsupported artifact entry: {path}")
