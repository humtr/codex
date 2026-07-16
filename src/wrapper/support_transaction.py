"""Complete support activation across manager, source, hooks, and launcher.

The lower-level :mod:`wrapper.support_layout` module owns immutable support and
source artifacts plus their pointers.  This module extends that transaction to
cover the generated system configuration and the public launcher.  Callers keep
using the historical prepare/commit/rollback API through ``wrapper.source``.
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


EXTRA_SCHEMA = 1
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
    transaction_file = state_dir.resolve() / layout.TRANSACTION_NAME
    if transaction_file.exists():
        rollback_support_install(transaction_file)

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
    try:
        metadata = _read_transaction(transaction_file)
        nonce = uuid.uuid4().hex[:12]
        launcher = prefix.resolve() / "bin/codex"
        system_config = state_dir.resolve() / "system-config"
        launcher_backup = state_dir.resolve() / f".launcher-{nonce}.backup"
        config_backup = state_dir.resolve() / f".system-config-{nonce}.backup"
        launcher_existed = _backup_path(launcher, launcher_backup)
        config_existed = _backup_path(system_config, config_backup)
        metadata["extended_transaction"] = {
            "schema": EXTRA_SCHEMA,
            "launcher": str(launcher),
            "launcher_backup": str(launcher_backup),
            "launcher_existed": launcher_existed,
            "system_config": str(system_config),
            "system_config_backup": str(config_backup),
            "system_config_existed": config_existed,
            "prefix": str(prefix.resolve()),
        }
        _write_transaction(transaction_file, metadata)
        return activation
    except Exception:
        try:
            layout.rollback_support_install(transaction_file)
        finally:
            _cleanup_backup_locals(locals())
        raise


def commit_support_install(transaction_file: Path) -> layout.SupportActivation:
    transaction_file = transaction_file.resolve()
    metadata = _read_transaction(transaction_file)
    extra = _extra(metadata)
    try:
        if os.environ.get("CODEX_TERMUX_INSTALL_FAIL_LAUNCHER", "0") == "1":
            raise IntegrityError("forced launcher installation failure")
        _install_launcher(metadata, extra)
        activation = layout.commit_support_install(transaction_file)
    except Exception:
        rollback_support_install(transaction_file)
        raise
    _discard_backup(extra, "launcher_backup")
    _discard_backup(extra, "system_config_backup")
    return activation


def rollback_support_install(transaction_file: Path) -> layout.SupportActivation:
    transaction_file = transaction_file.resolve()
    metadata = _read_transaction(transaction_file)
    extra = _extra(metadata, required=False)
    activation = layout.rollback_support_install(transaction_file)
    if extra:
        _restore_path(
            Path(extra["launcher"]),
            Path(extra["launcher_backup"]),
            bool(extra["launcher_existed"]),
        )
        _restore_path(
            Path(extra["system_config"]),
            Path(extra["system_config_backup"]),
            bool(extra["system_config_existed"]),
        )
    return activation


def _install_launcher(metadata: dict[str, Any], extra: dict[str, Any]) -> None:
    launcher = Path(extra["launcher"])
    source_target = Path(str(metadata.get("source_target", "")))
    native_source = source_target / "native/codex-launcher.c"
    manager_link = Path(str(metadata.get("manager_link", "")))
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
            temporary.write_text(
                "#!/bin/sh\n"
                f"# {MANAGED_LAUNCHER_MARKER.decode()}\n"
                f'exec "{manager_link}/managed.sh" "$@"\n',
                encoding="utf-8",
            )
        temporary.chmod(0o755)
        if MANAGED_LAUNCHER_MARKER not in temporary.read_bytes():
            raise IntegrityError("candidate launcher is missing the managed marker")
        os.replace(temporary, launcher)
    finally:
        temporary.unlink(missing_ok=True)


def _backup_path(path: Path, backup: Path) -> bool:
    _remove_path(backup)
    if not (path.exists() or path.is_symlink()):
        return False
    if path.is_symlink():
        backup.symlink_to(os.readlink(path))
    elif path.is_dir():
        shutil.copytree(path, backup, symlinks=True)
    elif path.is_file():
        shutil.copy2(path, backup)
    else:
        raise IntegrityError(f"unsupported managed path type: {path}")
    return True


def _restore_path(path: Path, backup: Path, existed: bool) -> None:
    _remove_path(path)
    if not existed:
        _remove_path(backup)
        return
    if not (backup.exists() or backup.is_symlink()):
        raise IntegrityError(f"support rollback backup is missing: {backup}")
    path.parent.mkdir(parents=True, exist_ok=True)
    os.replace(backup, path)


def _discard_backup(extra: dict[str, Any], key: str) -> None:
    value = extra.get(key)
    if isinstance(value, str) and value:
        _remove_path(Path(value))


def _remove_path(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path)


def _read_transaction(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IntegrityError(f"support transaction is unreadable: {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise IntegrityError(f"support transaction is not an object: {path}")
    return data


def _write_transaction(path: Path, data: dict[str, Any]) -> None:
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n", encoding="utf-8")
    temporary.chmod(0o600)
    os.replace(temporary, path)


def _extra(data: dict[str, Any], *, required: bool = True) -> dict[str, Any]:
    extra = data.get("extended_transaction")
    if not isinstance(extra, dict):
        if required:
            raise IntegrityError("support transaction lacks launcher/config rollback metadata")
        return {}
    if extra.get("schema") != EXTRA_SCHEMA:
        raise IntegrityError("support extended transaction schema is invalid")
    for key in ("launcher", "launcher_backup", "system_config", "system_config_backup", "prefix"):
        if not isinstance(extra.get(key), str):
            raise IntegrityError(f"support extended transaction field is invalid: {key}")
    for key in ("launcher_existed", "system_config_existed"):
        if not isinstance(extra.get(key), bool):
            raise IntegrityError(f"support extended transaction field is invalid: {key}")
    return extra


def _cleanup_backup_locals(values: dict[str, object]) -> None:
    for name in ("launcher_backup", "config_backup"):
        value = values.get(name)
        if isinstance(value, Path):
            _remove_path(value)
