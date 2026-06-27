"""Transactional activation of Codex Termux runtime tuples."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import uuid
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Callable

from . import registry, state, store
from .errors import CodexTermuxError, IntegrityError, TransactionError
from .schemas import ActivationPlan, ActivationResult


@dataclass(frozen=True)
class MetadataSnapshot:
    path: Path
    backup: Path
    existed: bool


@dataclass(frozen=True)
class PointerSnapshot:
    path: Path
    target: str | None
    backup: Path | None = None


@dataclass(frozen=True)
class ActivationSnapshot:
    transaction_dir: Path
    state: MetadataSnapshot
    registry: MetadataSnapshot
    current: PointerSnapshot
    verified: PointerSnapshot
    raw: PointerSnapshot


RegistryWriter = Callable[[], str]
StateWriter = Callable[[str], None]
RollbackAction = tuple[str, Callable[[], None]]


def commit(plan: ActivationPlan) -> ActivationResult:
    _run_probe(plan, "codex_smoke_test_runtime \"$2\"", plan.candidate_runtime / "codex")
    runtime_path = store.publish_runtime_artifact(
        plan.candidate_runtime,
        plan.runtime_target,
        plan.runtime_sha256,
    )
    raw_path = store.publish_raw_artifact(
        plan.candidate_raw,
        plan.raw_target,
        plan.raw_sha256,
    )

    def write_registry() -> str:
        return registry.record(
            registry_file=plan.registry_file,
            version=plan.version,
            raw_sha256=plan.raw_sha256,
            runtime_sha256=plan.runtime_sha256,
            package_spec=plan.package_spec,
            runtime_path=str(runtime_path),
            wrapper_version=plan.wrapper_version,
            wrapper_commit=plan.wrapper_commit,
            runtime_store_dir=runtime_path.parent,
            updated_at=plan.updated_at,
            smoke_tested_at=plan.updated_at,
            raw_path=str(raw_path),
        )

    return _activate(plan, runtime_path, raw_path, write_registry, lambda tuple_id: _write_state(plan, tuple_id))


def restore_verified(plan: ActivationPlan) -> ActivationResult:
    data = registry.load(plan.registry_file)
    tuple_id = data.get("verified_tuple_id", "")
    if not tuple_id:
        raise IntegrityError("registry has no verified tuple")
    install, runtime_entry, raw_entry = registry.tuple_activation_entries(data, tuple_id)
    runtime_path = Path(runtime_entry["path"])
    raw_path = Path(raw_entry["path"])
    restore_plan = replace(
        plan,
        candidate_runtime=runtime_path,
        candidate_raw=raw_path,
        runtime_target=runtime_path,
        raw_target=raw_path,
        version=install["version"],
        raw_sha256=install["raw_sha256"],
        runtime_sha256=install["runtime_sha256"],
        package_spec=install["package_spec"],
        cleanup_runtime_source=False,
        cleanup_raw_source=False,
    )
    _run_probe(restore_plan, "codex_smoke_test_runtime \"$2\"", runtime_path / "codex")

    def write_registry() -> str:
        registry.activate_existing_tuple(restore_plan.registry_file, tuple_id)
        return tuple_id

    return _activate(
        restore_plan,
        runtime_path,
        raw_path,
        write_registry,
        lambda active_id: _write_state(restore_plan, active_id),
    )


def _write_state(plan: ActivationPlan, tuple_id: str) -> None:
    state.write(
        state_file=plan.state_file,
        version=plan.version,
        raw_sha256=plan.raw_sha256,
        runtime_sha256=plan.runtime_sha256,
        package_spec=plan.package_spec,
        active_tuple_id=tuple_id,
        wrapper_version=plan.wrapper_version,
        wrapper_commit=plan.wrapper_commit,
        updated_at=plan.updated_at,
        verified_tuple_id=tuple_id,
        verified_at=plan.updated_at,
    )


def _activate(
    plan: ActivationPlan,
    runtime_path: Path,
    raw_path: Path,
    write_registry: RegistryWriter,
    write_state: StateWriter,
) -> ActivationResult:
    snapshot = _take_snapshot(plan)
    rollback_actions: list[RollbackAction] = []
    try:
        tuple_id = write_registry()
        rollback_actions.append(("registry", lambda: _restore_metadata(snapshot.registry)))
        write_state(tuple_id)
        rollback_actions.append(("state", lambda: _restore_metadata(snapshot.state)))
        _replace_pointer(snapshot.current, runtime_path)
        rollback_actions.append(("current pointer", lambda: _restore_pointer(snapshot.current)))
        _replace_pointer(snapshot.verified, runtime_path)
        rollback_actions.append(("verified pointer", lambda: _restore_pointer(snapshot.verified)))
        _replace_pointer(snapshot.raw, raw_path)
        rollback_actions.append(("raw pointer", lambda: _restore_pointer(snapshot.raw)))
        _run_probe(
            plan,
            'codex_smoke_test_runtime "$CODEX_TERMUX_RUNTIME" && codex_runtime_ok',
            None,
        )
        _cleanup_source(plan.candidate_runtime, runtime_path, plan.cleanup_runtime_source)
        _cleanup_source(plan.candidate_raw, raw_path, plan.cleanup_raw_source)
    except Exception as exc:
        _rollback_or_raise(snapshot, rollback_actions, exc)
    _cleanup_transaction(snapshot.transaction_dir)
    return ActivationResult(tuple_id=tuple_id, runtime_path=runtime_path, raw_path=raw_path)


def _take_snapshot(plan: ActivationPlan) -> ActivationSnapshot:
    plan.state_file.parent.mkdir(parents=True, exist_ok=True)
    transaction_dir = Path(
        tempfile.mkdtemp(prefix=".activation.", dir=plan.runtime_target.parent)
    )
    try:
        return ActivationSnapshot(
            transaction_dir=transaction_dir,
            state=_snapshot_metadata(plan.state_file, transaction_dir / "state"),
            registry=_snapshot_metadata(plan.registry_file, transaction_dir / "registry"),
            current=_snapshot_runtime_pointer(
                plan.current_link,
                plan.runtime_sha256,
                transaction_dir / "current-path",
            ),
            verified=_snapshot_runtime_pointer(
                plan.verified_link,
                plan.runtime_sha256,
                transaction_dir / "verified-path",
            ),
            raw=_snapshot_raw_pointer(
                plan.raw_link,
                plan.raw_sha256,
                transaction_dir / "raw-path",
            ),
        )
    except Exception:
        _cleanup_transaction(transaction_dir)
        raise


def _snapshot_metadata(path: Path, backup: Path) -> MetadataSnapshot:
    if not _lexists(path):
        return MetadataSnapshot(path=path, backup=backup, existed=False)
    if path.is_symlink() or not path.is_file():
        raise TransactionError(f"metadata path must be a regular file: {path}")
    shutil.copy2(path, backup)
    return MetadataSnapshot(path=path, backup=backup, existed=True)


def _snapshot_runtime_pointer(
    path: Path,
    expected_sha256: str,
    backup: Path,
) -> PointerSnapshot:
    return _snapshot_pointer(path, backup, _validate_existing_runtime_pointer)

def _snapshot_raw_pointer(
    path: Path,
    expected_sha256: str,
    backup: Path,
) -> PointerSnapshot:
    return _snapshot_pointer(path, backup, _validate_existing_raw_pointer)


def _validate_existing_runtime_pointer(path: Path) -> None:
    if path.is_symlink() or not path.is_dir():
        raise TransactionError(f"runtime pointer must be a directory: {path}")
    runtime = path / "codex"
    if runtime.is_symlink() or not runtime.is_file() or not os.access(runtime, os.X_OK):
        raise TransactionError(f"runtime pointer has no executable codex: {path}")


def _validate_existing_raw_pointer(path: Path) -> None:
    if path.is_symlink() or not path.is_dir():
        raise TransactionError(f"raw pointer must be a directory: {path}")
    raw = path / "vendor/aarch64-unknown-linux-musl/bin/codex"
    if raw.is_symlink() or not raw.is_file() or not os.access(raw, os.X_OK):
        raise TransactionError(f"raw pointer has no executable raw codex: {path}")

def _snapshot_pointer(
    path: Path,
    backup: Path,
    validate_existing: Callable[[Path], Path],
) -> PointerSnapshot:
    if not _lexists(path):
        return PointerSnapshot(path=path, target=None)
    if not path.is_symlink():
        validate_existing(path)
        return PointerSnapshot(path=path, target=None, backup=backup)
    return PointerSnapshot(path=path, target=os.readlink(path))


def _replace_pointer(snapshot: PointerSnapshot, target: Path) -> None:
    snapshot.path.parent.mkdir(parents=True, exist_ok=True)
    temp = snapshot.path.with_name(f".{snapshot.path.name}.activation.{uuid.uuid4().hex}")
    moved_existing = False
    try:
        if snapshot.backup is not None and _lexists(snapshot.path):
            os.replace(snapshot.path, snapshot.backup)
            moved_existing = True
        temp.symlink_to(target)
        os.replace(temp, snapshot.path)
    except OSError as exc:
        temp.unlink(missing_ok=True)
        if moved_existing:
            _restore_backup_path(snapshot)
        raise TransactionError(f"failed to replace pointer {snapshot.path}: {exc}") from exc


def _run_probe(plan: ActivationPlan, command: str, executable: Path | None) -> None:
    env = dict(os.environ)
    env.update(plan.probe_env)
    args = [str(plan.shell_bin), "-c", f'. "$1"; {command}', "activation-probe", str(plan.shell_lib)]
    if executable is not None:
        args.append(str(executable))
    result = subprocess.run(args, env=env, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        details = _tail_probe_output(result.stdout, result.stderr)
        message = f"activation probe failed with exit {result.returncode}"
        if details:
            message = f"{message}: {details}"
        raise IntegrityError(message)


def _tail_probe_output(stdout: str, stderr: str, limit: int = 20) -> str:
    lines = []
    for label, text in (("stdout", stdout), ("stderr", stderr)):
        if not text:
            continue
        tail = text.splitlines()[-limit:]
        if tail:
            lines.append(f"{label}: " + " | ".join(tail))
    return "; ".join(lines)


def _rollback_or_raise(
    snapshot: ActivationSnapshot,
    rollback_actions: list[RollbackAction],
    cause: Exception,
) -> None:
    errors = _rollback(snapshot, rollback_actions)
    if errors:
        joined = "; ".join(errors)
        raise TransactionError(f"{cause}; rollback failed: {joined}") from cause
    if isinstance(cause, CodexTermuxError):
        raise cause
    raise TransactionError(str(cause)) from cause


def _rollback(
    snapshot: ActivationSnapshot,
    rollback_actions: list[RollbackAction],
) -> list[str]:
    errors: list[str] = []
    for label, action in reversed(rollback_actions):
        try:
            action()
        except Exception as exc:
            errors.append(f"{label}: {exc}")
    try:
        _cleanup_transaction(snapshot.transaction_dir)
    except Exception as exc:
        errors.append(f"transaction cleanup: {exc}")
    return errors


def _restore_pointer(snapshot: PointerSnapshot) -> None:
    if snapshot.backup is not None:
        _remove_path(snapshot.path)
        _restore_backup_path(snapshot)
        return
    if snapshot.target is None:
        _remove_path(snapshot.path)
        return
    _replace_pointer(snapshot, Path(snapshot.target))


def _restore_backup_path(snapshot: PointerSnapshot) -> None:
    if snapshot.backup is None:
        return
    try:
        os.replace(snapshot.backup, snapshot.path)
    except OSError as exc:
        raise TransactionError(f"failed to restore path {snapshot.path}: {exc}") from exc


def _restore_metadata(snapshot: MetadataSnapshot) -> None:
    if not snapshot.existed:
        _remove_path(snapshot.path)
        return
    temp = snapshot.path.with_name(f".{snapshot.path.name}.restore.{uuid.uuid4().hex}")
    try:
        shutil.copy2(snapshot.backup, temp)
        os.replace(temp, snapshot.path)
    except OSError as exc:
        temp.unlink(missing_ok=True)
        raise TransactionError(f"failed to restore metadata {snapshot.path}: {exc}") from exc


def _cleanup_source(source: Path, target: Path, enabled: bool) -> None:
    if not enabled or not _lexists(source) or source.resolve() == target.resolve():
        return
    _remove_path(source)


def _cleanup_transaction(path: Path) -> None:
    try:
        shutil.rmtree(path)
    except OSError as exc:
        raise TransactionError(f"failed to clean activation transaction {path}: {exc}") from exc


def _remove_path(path: Path) -> None:
    try:
        if path.is_symlink() or path.is_file():
            path.unlink()
        elif path.is_dir():
            shutil.rmtree(path)
    except OSError as exc:
        raise TransactionError(f"failed to remove {path}: {exc}") from exc


def _lexists(path: Path) -> bool:
    return path.exists() or path.is_symlink()
