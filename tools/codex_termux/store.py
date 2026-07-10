"""Immutable artifact store operations for Codex Termux runtimes."""

from __future__ import annotations

import errno
import fcntl
import json
import os
import shutil
import tempfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

from .errors import CollisionError, IntegrityError, TransactionError
from .hashing import sha256_file, tree_digest


RAW_BINARY = Path("vendor/aarch64-unknown-linux-musl/bin/codex")


def validate_runtime_artifact(source: Path, expected_sha256: str) -> Path:
    root = _resolve_artifact_root(source)
    _validate_executable(root / "codex", "runtime executable")
    _validate_executable(root / "codex-code-mode-host", "code-mode host executable")
    _validate_directory(root / "codex-resources", "runtime resources")
    _validate_directory(root / "codex-path", "runtime path tools")
    _validate_regular_file(root / "codex-package.json", "runtime package metadata")
    _validate_regular_file(root / "runtime-build.json", "runtime build manifest")
    _validate_upstream_tree(root)
    _validate_expected_hash(root / "codex", expected_sha256)
    tree_digest(root)
    return root


def _validate_upstream_tree(root: Path) -> None:
    try:
        manifest = json.loads((root / "runtime-build.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IntegrityError(f"invalid runtime build manifest: {root / 'runtime-build.json'}") from exc
    expected = manifest.get("upstream_tree_sha256", "")
    if expected:
        upstream = root / "upstream"
        _validate_directory(upstream, "preserved upstream tree")
        actual = tree_digest(upstream)
        if actual != expected:
            raise IntegrityError(
                f"preserved upstream tree hash mismatch: expected {expected}, got {actual}"
            )


def validate_raw_artifact(source: Path, expected_sha256: str) -> Path:
    root = _resolve_artifact_root(source)
    _validate_directory(root / "vendor", "raw vendor tree")
    _validate_executable(root / RAW_BINARY, "raw executable")
    _validate_expected_hash(root / RAW_BINARY, expected_sha256)
    tree_digest(root)
    return root


def publish_immutable_tree(source: Path, target: Path) -> Path:
    """Consume source and publish it at target without replacing target."""
    source_digest = tree_digest(source)
    if source.resolve() == target.resolve():
        raise TransactionError(f"source and target are identical: {source}")
    target.parent.mkdir(parents=True, exist_ok=True)
    with _publish_lock(target.parent):
        if _path_exists(target):
            _reuse_or_raise(target, source_digest)
            _remove_tree(source)
            return target
        try:
            os.rename(source, target)
        except OSError as exc:
            if exc.errno != errno.EXDEV:
                if _path_exists(target):
                    _reuse_or_raise(target, source_digest)
                    _remove_tree(source)
                    return target
                raise TransactionError(f"failed to publish {target}: {exc}") from exc
            _publish_cross_device(source, target, source_digest)
    return target


def copy_immutable_tree(source: Path, target: Path) -> Path:
    """Copy source into immutable storage while preserving source."""
    tree_digest(source)
    staging = _new_staging_path(target)
    try:
        shutil.copytree(source, staging, symlinks=True)
        return publish_immutable_tree(staging, target)
    except OSError as exc:
        _cleanup_staging(staging)
        raise TransactionError(f"failed to copy artifact into {target}: {exc}") from exc
    except (CollisionError, IntegrityError, TransactionError):
        _cleanup_staging(staging)
        raise


def publish_runtime_artifact(
    source: Path,
    target: Path,
    expected_sha256: str,
) -> Path:
    source = validate_runtime_artifact(source, expected_sha256)
    staging = _new_staging_path(target)
    try:
        shutil.copytree(source, staging, symlinks=True)
        validate_runtime_artifact(staging, expected_sha256)
        return publish_immutable_tree(staging, target)
    except OSError as exc:
        _cleanup_staging(staging)
        raise TransactionError(f"failed to stage runtime artifact: {exc}") from exc
    except (CollisionError, IntegrityError, TransactionError):
        _cleanup_staging(staging)
        raise


def publish_raw_artifact(
    source: Path,
    target: Path,
    expected_sha256: str,
) -> Path:
    source = validate_raw_artifact(source, expected_sha256)
    staging = _new_staging_path(target)
    try:
        staging.mkdir()
        _copy_entry(source / "vendor", staging / "vendor")
        validate_raw_artifact(staging, expected_sha256)
        return publish_immutable_tree(staging, target)
    except OSError as exc:
        _cleanup_staging(staging)
        raise TransactionError(f"failed to stage raw artifact: {exc}") from exc
    except (CollisionError, IntegrityError, TransactionError):
        _cleanup_staging(staging)
        raise


def build_prune_plan(**kwargs: object) -> object:
    from . import prune

    return prune.build_prune_plan(**kwargs)


def apply_prune_plan(**kwargs: object) -> object:
    from . import prune

    return prune.apply_prune_plan(**kwargs)


def _resolve_artifact_root(path: Path) -> Path:
    try:
        root = path.resolve(strict=True)
    except OSError as exc:
        raise IntegrityError(f"failed to resolve artifact root {path}: {exc}") from exc
    if root.is_symlink() or not root.is_dir():
        raise IntegrityError(f"artifact root must resolve to a directory: {path}")
    return root


def _validate_directory(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_dir():
        raise IntegrityError(f"{label} must be a directory: {path}")


def _validate_regular_file(path: Path, label: str) -> None:
    if path.is_symlink() or not path.is_file():
        raise IntegrityError(f"{label} must be a regular file: {path}")


def _validate_executable(path: Path, label: str) -> None:
    _validate_regular_file(path, label)
    if not os.access(path, os.X_OK):
        raise IntegrityError(f"{label} is not executable: {path}")


def _validate_expected_hash(path: Path, expected_sha256: str) -> None:
    if not expected_sha256:
        raise IntegrityError(f"expected SHA-256 is empty for {path}")
    actual = sha256_file(path)
    if actual != expected_sha256:
        raise IntegrityError(
            f"SHA-256 mismatch for {path}: expected {expected_sha256}, got {actual}"
        )


def _copy_entry(source: Path, target: Path) -> None:
    if source.is_symlink():
        target.symlink_to(os.readlink(source))
    elif source.is_dir():
        shutil.copytree(source, target, symlinks=True)
    elif source.is_file():
        shutil.copy2(source, target, follow_symlinks=False)
    else:
        raise IntegrityError(f"unsupported artifact entry: {source}")


def _new_staging_path(target: Path) -> Path:
    target.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=f".{target.name}.stage.", dir=target.parent)
    )
    staging.rmdir()
    return staging


@contextmanager
def _publish_lock(parent: Path) -> Iterator[None]:
    lock_path = parent / ".codex-termux-publish.lock"
    try:
        with lock_path.open("a+b") as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            yield
    except OSError as exc:
        raise TransactionError(f"failed to lock immutable store {parent}: {exc}") from exc


def _path_exists(path: Path) -> bool:
    return path.exists() or path.is_symlink()


def _reuse_or_raise(target: Path, source_digest: str) -> None:
    if target.is_symlink() or not target.is_dir():
        raise CollisionError(f"immutable store collision at {target}")
    if tree_digest(target) != source_digest:
        raise CollisionError(f"immutable store collision at {target}")


def _publish_cross_device(source: Path, target: Path, source_digest: str) -> None:
    staging = _new_staging_path(target)
    try:
        shutil.copytree(source, staging, symlinks=True)
        if tree_digest(staging) != source_digest:
            raise IntegrityError(f"cross-device copy changed artifact: {source}")
        os.rename(staging, target)
        _remove_tree(source)
    except OSError as exc:
        _cleanup_staging(staging)
        raise TransactionError(f"failed to publish {target}: {exc}") from exc


def _remove_tree(path: Path) -> None:
    try:
        shutil.rmtree(path)
    except OSError as exc:
        raise TransactionError(f"failed to remove artifact tree {path}: {exc}") from exc


def _cleanup_staging(path: Path) -> None:
    if not _path_exists(path):
        return
    try:
        shutil.rmtree(path)
    except OSError as exc:
        raise TransactionError(f"failed to clean staging tree {path}: {exc}") from exc
