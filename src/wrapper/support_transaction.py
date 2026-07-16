"""Crash-recoverable support activation across manager, source, hooks, and launcher.

The lower-level :mod:`wrapper.support_layout` module builds and validates immutable
support/source artifacts.  This module owns a durable recovery journal that is
written before any active path is changed.  The journal remains until manager,
source, generated system configuration, and the public launcher are committed or
fully restored.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Any

from .errors import IntegrityError
from . import support_layout as layout


RECOVERY_SCHEMA = 2
RECOVERY_NAME = "support-recovery.json"
MANAGED_LAUNCHER_MARKER = b"codex termux managed launcher"


def prepare_support_install(
    *,
    source_root: Path,
    wrapper_root: Path,
    manager_link: Path,
    verified_manager_link: Path,
    state_dir: Path,
    prefix: Path,
    installed_at: str,
    wrapper_commit: str = "",
) -> layout.SupportActivation:
    source_root = source_root.resolve()
    wrapper_root = wrapper_root.resolve()
    state_dir = state_dir.resolve()
    prefix = prefix.resolve()
    state_dir.mkdir(parents=True, exist_ok=True)
    (wrapper_root / "support-store").mkdir(parents=True, exist_ok=True)
    (wrapper_root / "source-store").mkdir(parents=True, exist_ok=True)

    transaction_file = state_dir / layout.TRANSACTION_NAME
    recovery_file = state_dir / RECOVERY_NAME
    if recovery_file.exists():
        _recover(recovery_file)
    if transaction_file.exists():
        _recover_legacy_transaction(transaction_file)

    nonce = uuid.uuid4().hex[:12]
    source_link = wrapper_root / "source-snapshot"
    verified_source_link = wrapper_root / "verified-source-snapshot"
    launcher = prefix / "bin/codex"
    system_config = state_dir / "system-config"
    launcher_backup = state_dir / f".launcher-{nonce}.backup"
    config_backup = state_dir / f".system-config-{nonce}.backup"

    launcher_existed = _backup_path(launcher, launcher_backup)
    config_existed = _backup_path(system_config, config_backup)
    manager_snapshot = _active_snapshot(
        manager_link,
        wrapper_root / "support-store" / f"legacy-manager-{nonce}",
    )
    source_snapshot = _active_snapshot(
        source_link,
        wrapper_root / "source-store" / f"legacy-source-{nonce}",
    )
    recovery: dict[str, Any] = {
        "schema": RECOVERY_SCHEMA,
        "status": "prepared",
        "installed_at": installed_at,
        "transaction_file": str(transaction_file),
        "wrapper_root": str(wrapper_root),
        "support_store": str(wrapper_root / "support-store"),
        "source_store": str(wrapper_root / "source-store"),
        "preexisting_support_entries": _entry_names(wrapper_root / "support-store"),
        "preexisting_source_entries": _entry_names(wrapper_root / "source-store"),
        "manager_link": str(manager_link),
        "verified_manager_link": str(verified_manager_link),
        "source_link": str(source_link),
        "verified_source_link": str(verified_source_link),
        "manager_before": manager_snapshot,
        "verified_manager_before": _pointer_snapshot(verified_manager_link),
        "source_before": source_snapshot,
        "verified_source_before": _pointer_snapshot(verified_source_link),
        "launcher": str(launcher),
        "launcher_backup": str(launcher_backup),
        "launcher_existed": launcher_existed,
        "system_config": str(system_config),
        "system_config_backup": str(config_backup),
        "system_config_existed": config_existed,
        "candidate_support": "",
        "candidate_source": "",
    }
    _write_json_durable(recovery_file, recovery, mode=0o600)

    try:
        _pre_adopt(manager_link, manager_snapshot)
        _pre_adopt(source_link, source_snapshot)
        activation = layout.prepare_support_install(
            source_root=source_root,
            wrapper_root=wrapper_root,
            manager_link=manager_link,
            verified_manager_link=verified_manager_link,
            state_dir=state_dir,
            prefix=prefix,
            installed_at=installed_at,
            wrapper_commit=wrapper_commit,
        )
        recovery.update(
            {
                "status": "switched",
                "candidate_support": activation.target,
                "candidate_source": activation.source_target,
                "activation": activation.to_dict(),
            }
        )
        _write_json_durable(recovery_file, recovery, mode=0o600)
        _sync_paths(
            transaction_file,
            manager_link.parent,
            source_link.parent,
            Path(activation.target).parent,
            Path(activation.source_target).parent,
        )
        return activation
    except Exception:
        try:
            _recover(recovery_file)
        except Exception as recovery_error:
            raise IntegrityError(
                f"support preparation failed and recovery is incomplete: {recovery_error}"
            ) from recovery_error
        raise


def commit_support_install(transaction_file: Path) -> layout.SupportActivation:
    transaction_file = transaction_file.resolve()
    recovery_file = transaction_file.with_name(RECOVERY_NAME)
    recovery = _read_recovery(recovery_file)
    activation = _activation(recovery, transaction_file)
    if recovery["status"] == "committed":
        _finalize_committed(recovery_file, recovery)
        return activation

    try:
        if os.environ.get("CODEX_TERMUX_INSTALL_FAIL_LAUNCHER", "0") == "1":
            raise IntegrityError("forced launcher installation failure")
        _install_launcher(recovery)
        recovery["status"] = "launcher-installed"
        _write_json_durable(recovery_file, recovery, mode=0o600)
        if os.environ.get("CODEX_TERMUX_INSTALL_CRASH_POINT", "") == "after-launcher":
            raise IntegrityError("forced interruption after launcher installation")

        recovery["status"] = "committing"
        _write_json_durable(recovery_file, recovery, mode=0o600)
        committed = layout.commit_support_install(transaction_file)
        recovery["status"] = "committed"
        recovery["activation"] = committed.to_dict()
        _write_json_durable(recovery_file, recovery, mode=0o600)
        _finalize_committed(recovery_file, recovery)
        return committed
    except Exception:
        _recover(recovery_file)
        raise


def rollback_support_install(transaction_file: Path) -> layout.SupportActivation:
    transaction_file = transaction_file.resolve()
    recovery_file = transaction_file.with_name(RECOVERY_NAME)
    if recovery_file.exists():
        return _recover(recovery_file)
    activation = layout.rollback_support_install(transaction_file)
    _fsync_dir(transaction_file.parent)
    return activation


def _recover(recovery_file: Path) -> layout.SupportActivation:
    recovery_file = recovery_file.resolve()
    recovery = _read_recovery(recovery_file)
    transaction_file = Path(recovery["transaction_file"])
    activation = _activation(recovery, transaction_file)
    if recovery["status"] == "committed":
        _finalize_committed(recovery_file, recovery)
        return activation

    # Restore independently copied paths first.  Backups are copied, not moved,
    # so an interruption during rollback can safely retry from the same journal.
    _restore_backup(
        Path(recovery["launcher"]),
        Path(recovery["launcher_backup"]),
        bool(recovery["launcher_existed"]),
    )
    _restore_backup(
        Path(recovery["system_config"]),
        Path(recovery["system_config_backup"]),
        bool(recovery["system_config_existed"]),
    )
    if os.environ.get("CODEX_TERMUX_INSTALL_CRASH_POINT", "") == "rollback-after-files":
        raise IntegrityError("forced interruption during support rollback")

    _restore_active(Path(recovery["manager_link"]), recovery["manager_before"])
    _restore_active(Path(recovery["source_link"]), recovery["source_before"])
    _restore_pointer(
        Path(recovery["verified_manager_link"]),
        recovery["verified_manager_before"],
    )
    _restore_pointer(
        Path(recovery["verified_source_link"]),
        recovery["verified_source_before"],
    )
    if os.environ.get("CODEX_TERMUX_INSTALL_CRASH_POINT", "") == "rollback-after-pointers":
        raise IntegrityError("forced interruption after support pointer rollback")

    _cleanup_candidates(recovery)
    transaction_file.unlink(missing_ok=True)
    _fsync_dir(transaction_file.parent)
    _discard_recovery_files(recovery)
    recovery_file.unlink(missing_ok=True)
    _fsync_dir(recovery_file.parent)
    return activation


def _recover_legacy_transaction(transaction_file: Path) -> None:
    """Recover journals produced before the durable outer journal existed."""
    try:
        layout.rollback_support_install(transaction_file)
    finally:
        _fsync_dir(transaction_file.parent)


def _activation(recovery: dict[str, Any], transaction_file: Path) -> layout.SupportActivation:
    raw = recovery.get("activation")
    if isinstance(raw, dict):
        return _activation_from_mapping(raw, transaction_file)
    if transaction_file.is_file():
        try:
            lower = json.loads(transaction_file.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            lower = None
        if isinstance(lower, dict):
            recovery["candidate_support"] = str(lower.get("target", ""))
            recovery["candidate_source"] = str(lower.get("source_target", ""))
            return _activation_from_mapping(lower, transaction_file)
    return layout.SupportActivation(
        support_id=Path(str(recovery.get("candidate_support", ""))).name,
        target=str(recovery.get("candidate_support", "")),
        source_id=Path(str(recovery.get("candidate_source", ""))).name,
        source_target=str(recovery.get("candidate_source", "")),
        previous=_snapshot_target(recovery["manager_before"]),
        previous_source=_snapshot_target(recovery["source_before"]),
        manager_link=str(recovery["manager_link"]),
        verified_manager_link=str(recovery["verified_manager_link"]),
        source_link=str(recovery["source_link"]),
        verified_source_link=str(recovery["verified_source_link"]),
        transaction_file=str(transaction_file),
    )


def _activation_from_mapping(
    raw: dict[str, Any], transaction_file: Path
) -> layout.SupportActivation:
    fields = (
        "support_id",
        "target",
        "source_id",
        "source_target",
        "previous",
        "previous_source",
        "manager_link",
        "verified_manager_link",
        "source_link",
        "verified_source_link",
    )
    for field in fields:
        if not isinstance(raw.get(field), str):
            raise IntegrityError(f"support activation field is invalid: {field}")
    return layout.SupportActivation(
        support_id=raw["support_id"],
        target=raw["target"],
        source_id=raw["source_id"],
        source_target=raw["source_target"],
        previous=raw["previous"],
        previous_source=raw["previous_source"],
        manager_link=raw["manager_link"],
        verified_manager_link=raw["verified_manager_link"],
        source_link=raw["source_link"],
        verified_source_link=raw["verified_source_link"],
        transaction_file=str(transaction_file),
    )


def _install_launcher(recovery: dict[str, Any]) -> None:
    launcher = Path(recovery["launcher"])
    source_target = Path(recovery["candidate_source"])
    native_source = source_target / "native/codex-launcher.c"
    manager_link = Path(recovery["manager_link"])
    if not native_source.is_file():
        raise IntegrityError(f"native launcher source is missing: {native_source}")
    launcher.parent.mkdir(parents=True, exist_ok=True)
    temporary = launcher.with_name(f".{launcher.name}.{uuid.uuid4().hex}.new")
    try:
        clang = shutil.which("clang")
        if clang:
            result = subprocess.run(
                [clang, "-O2", "-Wall", "-Wextra", "-o", str(temporary), str(native_source)],
                check=False,
                capture_output=True,
                text=True,
                timeout=60,
            )
            if result.returncode != 0:
                detail = (result.stderr or result.stdout).strip()
                raise IntegrityError(f"launcher compilation failed: {detail}")
        else:
            _write_text_durable(
                temporary,
                "#!/bin/sh\n"
                f"# {MANAGED_LAUNCHER_MARKER.decode()}\n"
                f'exec "{manager_link}/managed.sh" "$@"\n',
                mode=0o755,
            )
        temporary.chmod(0o755)
        _fsync_file(temporary)
        if MANAGED_LAUNCHER_MARKER not in temporary.read_bytes():
            raise IntegrityError("candidate launcher is missing the managed marker")
        os.replace(temporary, launcher)
        _fsync_dir(launcher.parent)
    finally:
        temporary.unlink(missing_ok=True)


def _active_snapshot(path: Path, adopted_path: Path) -> dict[str, str]:
    if path.is_symlink():
        return {"kind": "symlink", "target": os.readlink(path)}
    if not path.exists():
        return {"kind": "absent"}
    if path.is_dir():
        return {"kind": "directory", "adopted": str(adopted_path)}
    raise IntegrityError(f"managed active path has unsupported type: {path}")


def _pointer_snapshot(path: Path) -> dict[str, str]:
    if path.is_symlink():
        return {"kind": "symlink", "target": os.readlink(path)}
    if not path.exists():
        return {"kind": "absent"}
    raise IntegrityError(f"managed pointer is not a symlink: {path}")


def _pre_adopt(path: Path, snapshot: dict[str, str]) -> None:
    if snapshot["kind"] != "directory":
        return
    adopted = Path(snapshot["adopted"])
    if path.is_dir() and not path.is_symlink():
        adopted.parent.mkdir(parents=True, exist_ok=True)
        os.replace(path, adopted)
        _fsync_dir(adopted.parent)
        _fsync_dir(path.parent)
    if adopted.is_dir():
        _replace_symlink_durable(path, adopted)


def _restore_active(path: Path, snapshot: dict[str, Any]) -> None:
    kind = snapshot.get("kind")
    if kind == "directory":
        adopted = Path(str(snapshot.get("adopted", "")))
        if path.is_dir() and not path.is_symlink() and not adopted.exists():
            return
        _remove_path(path)
        if not adopted.is_dir():
            raise IntegrityError(f"adopted legacy directory is missing: {adopted}")
        os.replace(adopted, path)
        _fsync_dir(path.parent)
        _fsync_dir(adopted.parent)
        return
    _restore_pointer(path, snapshot)


def _restore_pointer(path: Path, snapshot: dict[str, Any]) -> None:
    kind = snapshot.get("kind")
    if kind == "absent":
        _remove_path(path)
        _fsync_dir(path.parent)
        return
    if kind != "symlink" or not isinstance(snapshot.get("target"), str):
        raise IntegrityError(f"invalid managed pointer snapshot for {path}")
    target = str(snapshot["target"])
    if path.is_symlink() and os.readlink(path) == target:
        return
    _replace_symlink_durable(path, target)


def _snapshot_target(snapshot: dict[str, Any]) -> str:
    if snapshot.get("kind") == "directory":
        return str(snapshot.get("adopted", ""))
    if snapshot.get("kind") == "symlink":
        return str(snapshot.get("target", ""))
    return ""


def _backup_path(path: Path, backup: Path) -> bool:
    _remove_path(backup)
    if not (path.exists() or path.is_symlink()):
        return False
    backup.parent.mkdir(parents=True, exist_ok=True)
    if path.is_symlink():
        backup.symlink_to(os.readlink(path))
        _fsync_dir(backup.parent)
    elif path.is_dir():
        shutil.copytree(path, backup, symlinks=True)
        _fsync_tree(backup)
    elif path.is_file():
        shutil.copy2(path, backup)
        _fsync_file(backup)
        _fsync_dir(backup.parent)
    else:
        raise IntegrityError(f"unsupported managed path type: {path}")
    return True


def _restore_backup(path: Path, backup: Path, existed: bool) -> None:
    if not existed:
        _remove_path(path)
        _fsync_dir(path.parent)
        return
    if not (backup.exists() or backup.is_symlink()):
        raise IntegrityError(f"support rollback backup is missing: {backup}")
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.restore")
    _remove_path(temporary)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        if backup.is_symlink():
            temporary.symlink_to(os.readlink(backup))
        elif backup.is_dir():
            shutil.copytree(backup, temporary, symlinks=True)
            _fsync_tree(temporary)
        elif backup.is_file():
            shutil.copy2(backup, temporary)
            _fsync_file(temporary)
        else:
            raise IntegrityError(f"unsupported rollback backup type: {backup}")
        _remove_path(path)
        os.replace(temporary, path)
        _fsync_dir(path.parent)
    finally:
        _remove_path(temporary)


def _cleanup_candidates(recovery: dict[str, Any]) -> None:
    for key in ("candidate_support", "candidate_source"):
        value = recovery.get(key)
        if isinstance(value, str) and value:
            _remove_path(Path(value))
    _cleanup_new_entries(
        Path(recovery["support_store"]),
        set(recovery["preexisting_support_entries"]),
        ("support-", ".support-"),
    )
    _cleanup_new_entries(
        Path(recovery["source_store"]),
        set(recovery["preexisting_source_entries"]),
        ("source-", ".source-"),
    )


def _cleanup_new_entries(store: Path, previous: set[str], prefixes: tuple[str, ...]) -> None:
    if not store.is_dir():
        return
    for child in store.iterdir():
        if child.name in previous:
            continue
        if child.name.startswith(prefixes):
            _remove_path(child)
    _fsync_dir(store)


def _finalize_committed(recovery_file: Path, recovery: dict[str, Any]) -> None:
    transaction_file = Path(recovery["transaction_file"])
    transaction_file.unlink(missing_ok=True)
    _discard_recovery_files(recovery)
    recovery_file.unlink(missing_ok=True)
    _fsync_dir(recovery_file.parent)


def _discard_recovery_files(recovery: dict[str, Any]) -> None:
    for key in ("launcher_backup", "system_config_backup"):
        value = recovery.get(key)
        if isinstance(value, str) and value:
            _remove_path(Path(value))
    _fsync_dir(Path(recovery["transaction_file"]).parent)


def _entry_names(path: Path) -> list[str]:
    if not path.is_dir():
        return []
    return sorted(child.name for child in path.iterdir())


def _read_recovery(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IntegrityError(f"support recovery journal is unreadable: {path}: {exc}") from exc
    if not isinstance(data, dict) or data.get("schema") != RECOVERY_SCHEMA:
        raise IntegrityError(f"support recovery journal schema is invalid: {path}")
    required_strings = (
        "status",
        "transaction_file",
        "wrapper_root",
        "support_store",
        "source_store",
        "manager_link",
        "verified_manager_link",
        "source_link",
        "verified_source_link",
        "launcher",
        "launcher_backup",
        "system_config",
        "system_config_backup",
        "candidate_support",
        "candidate_source",
    )
    for key in required_strings:
        if not isinstance(data.get(key), str):
            raise IntegrityError(f"support recovery field is invalid: {key}")
    for key in ("launcher_existed", "system_config_existed"):
        if not isinstance(data.get(key), bool):
            raise IntegrityError(f"support recovery field is invalid: {key}")
    for key in (
        "manager_before",
        "verified_manager_before",
        "source_before",
        "verified_source_before",
    ):
        if not isinstance(data.get(key), dict):
            raise IntegrityError(f"support recovery snapshot is invalid: {key}")
    for key in ("preexisting_support_entries", "preexisting_source_entries"):
        value = data.get(key)
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            raise IntegrityError(f"support recovery entry list is invalid: {key}")
    if data["status"] not in {
        "prepared",
        "switched",
        "launcher-installed",
        "committing",
        "committed",
    }:
        raise IntegrityError(f"support recovery status is invalid: {data['status']}")
    return data


def _write_json_durable(path: Path, data: dict[str, Any], *, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    payload = json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n"
    try:
        with temporary.open("w", encoding="utf-8") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(mode)
        os.replace(temporary, path)
        _fsync_dir(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def _write_text_durable(path: Path, value: str, *, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        handle.write(value)
        handle.flush()
        os.fsync(handle.fileno())
    path.chmod(mode)
    _fsync_dir(path.parent)


def _replace_symlink_durable(link: Path, target: Path | str) -> None:
    link.parent.mkdir(parents=True, exist_ok=True)
    temporary = link.with_name(f".{link.name}.{uuid.uuid4().hex}.new")
    temporary.symlink_to(target)
    try:
        os.replace(temporary, link)
        _fsync_dir(link.parent)
    finally:
        temporary.unlink(missing_ok=True)


def _sync_paths(*paths: Path) -> None:
    for path in paths:
        if path.is_file():
            _fsync_file(path)
        elif path.is_dir():
            _fsync_dir(path)
        else:
            _fsync_dir(path.parent)


def _fsync_tree(root: Path) -> None:
    for path in sorted(root.rglob("*")):
        if path.is_file() and not path.is_symlink():
            _fsync_file(path)
    for path in sorted((item for item in root.rglob("*") if item.is_dir()), reverse=True):
        _fsync_dir(path)
    _fsync_dir(root)
    _fsync_dir(root.parent)


def _fsync_file(path: Path) -> None:
    try:
        with path.open("rb") as handle:
            os.fsync(handle.fileno())
    except OSError as exc:
        raise IntegrityError(f"failed to sync file {path}: {exc}") from exc


def _fsync_dir(path: Path) -> None:
    try:
        flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        descriptor = os.open(path, flags)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    except OSError as exc:
        raise IntegrityError(f"failed to sync directory {path}: {exc}") from exc


def _remove_path(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path)
