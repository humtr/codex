"""Transactional role-oriented manager and source-snapshot installation."""

from __future__ import annotations

from dataclasses import dataclass
import json
import os
import re
import shutil
import subprocess
import uuid
from pathlib import Path
from typing import Any

from .errors import IntegrityError


SCHEMA = 2
TRANSACTION_NAME = "support-activation.json"


@dataclass(frozen=True)
class SupportActivation:
    support_id: str
    target: str
    source_id: str
    source_target: str
    previous: str
    previous_source: str
    manager_link: str
    verified_manager_link: str
    source_link: str
    verified_source_link: str
    transaction_file: str

    def to_dict(self) -> dict[str, str | int]:
        return {
            "schema": SCHEMA,
            "support_id": self.support_id,
            "target": self.target,
            "source_id": self.source_id,
            "source_target": self.source_target,
            "previous": self.previous,
            "previous_source": self.previous_source,
            "manager_link": self.manager_link,
            "verified_manager_link": self.verified_manager_link,
            "source_link": self.source_link,
            "verified_source_link": self.verified_source_link,
            "transaction_file": self.transaction_file,
        }


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
) -> SupportActivation:
    source_root = source_root.resolve()
    wrapper_root = wrapper_root.resolve()
    state_dir = state_dir.resolve()
    from . import source as source_module

    source_module.require_wrapper_source(source_root, "Wrapper source")
    _require_role_layout(source_root)

    support_store = wrapper_root / "support-store"
    source_store = wrapper_root / "source-store"
    source_link = wrapper_root / "source-snapshot"
    verified_source_link = wrapper_root / "verified-source-snapshot"
    transaction_file = state_dir / TRANSACTION_NAME
    support_store.mkdir(parents=True, exist_ok=True)
    source_store.mkdir(parents=True, exist_ok=True)
    state_dir.mkdir(parents=True, exist_ok=True)
    if transaction_file.exists():
        rollback_support_install(transaction_file)

    metadata = _read_env(source_root / "config/wrapper-version.env")
    version = metadata.get("CODEX_TERMUX_WRAPPER_VERSION", "unknown")
    commit = wrapper_commit or source_module.source_commit(source_root)
    nonce = uuid.uuid4().hex[:12]
    support_id = f"support-{_component(version)}-{_component(commit[:12])}-{nonce}"
    source_id = f"source-{_component(version)}-{_component(commit[:12])}-{nonce}"
    support_staging = support_store / f".{support_id}.staging"
    source_staging = source_store / f".{source_id}.staging"
    support_target = support_store / support_id
    source_target = source_store / source_id
    for path in (support_staging, source_staging, support_target, source_target):
        if path.exists() or path.is_symlink():
            raise IntegrityError(f"support transaction target already exists: {path}")

    previous_manager: Path | None = None
    previous_source: Path | None = None
    try:
        _copy_source_snapshot(source_root, source_staging)
        _validate_source_snapshot(source_staging)
        os.replace(source_staging, source_target)

        _populate_support(
            source_root=source_root,
            target=support_staging,
            wrapper_root=wrapper_root,
            manager_link=manager_link,
            source_link=source_link,
            prefix=prefix,
            support_id=support_id,
            source_id=source_id,
            wrapper_commit=commit,
            installed_at=installed_at,
        )
        _validate_support(support_staging)
        os.replace(support_staging, support_target)

        previous_manager = _adopt_current_path(manager_link, support_store, "legacy-manager")
        previous_source = _adopt_current_path(source_link, source_store, "legacy-source")
        activation = SupportActivation(
            support_id=support_id,
            target=str(support_target),
            source_id=source_id,
            source_target=str(source_target),
            previous=str(previous_manager) if previous_manager else "",
            previous_source=str(previous_source) if previous_source else "",
            manager_link=str(manager_link),
            verified_manager_link=str(verified_manager_link),
            source_link=str(source_link),
            verified_source_link=str(verified_source_link),
            transaction_file=str(transaction_file),
        )
        _write_transaction(transaction_file, activation, "prepared", installed_at)
        if previous_manager is not None:
            _replace_symlink(verified_manager_link, previous_manager)
        elif not _lexists(verified_manager_link):
            _replace_symlink(verified_manager_link, support_target)
        if previous_source is not None:
            _replace_symlink(verified_source_link, previous_source)
        elif not _lexists(verified_source_link):
            _replace_symlink(verified_source_link, source_target)
        _replace_symlink(source_link, source_target)
        _replace_symlink(manager_link, support_target)
        _write_transaction(transaction_file, activation, "switched", installed_at)
        return activation
    except Exception:
        if transaction_file.exists():
            try:
                rollback_support_install(transaction_file)
            except Exception:
                pass
        if support_staging.exists():
            shutil.rmtree(support_staging, ignore_errors=True)
        if source_staging.exists():
            shutil.rmtree(source_staging, ignore_errors=True)
        if support_target.exists() and not _same_target(manager_link, support_target):
            shutil.rmtree(support_target, ignore_errors=True)
        if source_target.exists() and not _same_target(source_link, source_target):
            shutil.rmtree(source_target, ignore_errors=True)
        raise


def commit_support_install(transaction_file: Path) -> SupportActivation:
    activation = _load_activation(transaction_file)
    if not _same_target(Path(activation.manager_link), Path(activation.target)):
        raise IntegrityError("active manager does not match prepared support artifact")
    if not _same_target(Path(activation.source_link), Path(activation.source_target)):
        raise IntegrityError("active source snapshot does not match prepared source artifact")
    transaction_file.unlink(missing_ok=True)
    return activation


def rollback_support_install(transaction_file: Path) -> SupportActivation:
    activation = _load_activation(transaction_file)
    manager_link = Path(activation.manager_link)
    verified_manager = Path(activation.verified_manager_link)
    source_link = Path(activation.source_link)
    verified_source = Path(activation.verified_source_link)
    target = Path(activation.target)
    source_target = Path(activation.source_target)
    previous = Path(activation.previous) if activation.previous else None
    previous_source = Path(activation.previous_source) if activation.previous_source else None

    if previous is not None and previous.exists():
        _replace_symlink(manager_link, previous)
        _replace_symlink(verified_manager, previous)
    else:
        _remove_path(manager_link)
        if _same_target(verified_manager, target):
            _remove_path(verified_manager)
    if previous_source is not None and previous_source.exists():
        _replace_symlink(source_link, previous_source)
        _replace_symlink(verified_source, previous_source)
    else:
        _remove_path(source_link)
        if _same_target(verified_source, source_target):
            _remove_path(verified_source)

    transaction_file.unlink(missing_ok=True)
    if target.exists() and not _same_target(manager_link, target) and not _same_target(verified_manager, target):
        shutil.rmtree(target)
    if source_target.exists() and not _same_target(source_link, source_target) and not _same_target(verified_source, source_target):
        shutil.rmtree(source_target)
    return activation


def _require_role_layout(root: Path) -> None:
    required = (
        "shell/loader.sh",
        "shell/dispatch.sh",
        "src/wrapper/cli.py",
        "src/wrapper/source.py",
        "libexec/build-runtime.py",
        "libexec/bwrap-termux-compat.py",
        "libexec/rg-termux-shim.sh",
        "native/codex-launcher.c",
    )
    missing = [relative for relative in required if not (root / relative).exists()]
    if missing:
        raise IntegrityError("role-oriented wrapper source is incomplete: " + " ".join(missing))


def _copy_source_snapshot(source_root: Path, target: Path) -> None:
    target.mkdir(parents=True, mode=0o700)
    for name in ("bin", "lib", "shell", "src", "libexec", "native", "tools", "config"):
        shutil.copytree(source_root / name, target / name, symlinks=True)
    for name in ("install.sh", "codex-wrapper.manifest.json"):
        shutil.copy2(source_root / name, target / name)
    if (source_root / "README.md").is_file():
        shutil.copy2(source_root / "README.md", target / "README.md")
    _remove_bytecode(target)


def _populate_support(
    *,
    source_root: Path,
    target: Path,
    wrapper_root: Path,
    manager_link: Path,
    source_link: Path,
    prefix: Path,
    support_id: str,
    source_id: str,
    wrapper_commit: str,
    installed_at: str,
) -> None:
    target.mkdir(parents=True, mode=0o700)
    shutil.copytree(source_root / "shell", target / "shell", symlinks=True)
    shutil.copytree(source_root / "src", target / "src", symlinks=True)
    shutil.copytree(source_root / "libexec", target / "libexec", symlinks=True)
    shutil.copytree(source_root / "lib/codex-termux", target / "codex-termux", symlinks=True)
    shutil.copytree(source_root / "tools/codex_termux", target / "codex_termux", symlinks=True)
    shutil.copy2(source_root / "lib/codex-termux.sh", target / "lib.sh")

    (target / "source").symlink_to(source_link)
    for compatibility, destination in (
        ("build-runtime.py", "libexec/build-runtime.py"),
        ("bwrap-termux-compat.py", "libexec/bwrap-termux-compat.py"),
        ("rg-termux-shim.sh", "libexec/rg-termux-shim.sh"),
    ):
        (target / compatibility).symlink_to(destination)
    version_file = target / "wrapper-version.env"
    shutil.copy2(source_root / "config/wrapper-version.env", version_file)
    with version_file.open("a", encoding="utf-8") as handle:
        handle.write(f"CODEX_TERMUX_WRAPPER_COMMIT={wrapper_commit}\n")
        handle.write(f"CODEX_TERMUX_WRAPPER_INSTALLED_AT={installed_at}\n")
    version_file.chmod(0o644)

    managed = target / "managed.sh"
    managed.write_text(
        "\n".join(
            (
                f"#!{prefix}/bin/bash",
                "# codex termux managed shell",
                "set -euo pipefail",
                f'export CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="${{CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-{source_link}/bin/install-runtime.sh}}"',
                f'export CODEX_TERMUX_SOURCE_DIR="${{CODEX_TERMUX_SOURCE_DIR:-{source_link}}}"',
                "# shellcheck disable=SC1091",
                f'. "{manager_link}/lib.sh"',
                'codex_main "$@"',
                "",
            )
        ),
        encoding="utf-8",
    )
    managed.chmod(0o755)
    for path in (
        target / "lib.sh",
        target / "managed.sh",
        target / "libexec/build-runtime.py",
        target / "libexec/bwrap-termux-compat.py",
        target / "libexec/rg-termux-shim.sh",
    ):
        path.chmod(0o755)
    _remove_bytecode(target)
    manifest = {
        "schema": 2,
        "support_id": support_id,
        "source_id": source_id,
        "layout": "role-oriented-v1",
        "wrapper_commit": wrapper_commit,
        "installed_at": installed_at,
        "entrypoints": {
            "managed_shell": "managed.sh",
            "shell_loader": "shell/loader.sh",
            "python_package": "src/wrapper",
            "runtime_builder": "libexec/build-runtime.py",
            "source_snapshot": str(source_link),
        },
    }
    _write_json(target / "support-manifest.json", manifest, mode=0o644)


def _validate_source_snapshot(target: Path) -> None:
    _require_role_layout(target)
    _compile_python(target / "src/wrapper")


def _validate_support(target: Path) -> None:
    required = (
        "managed.sh",
        "lib.sh",
        "shell/loader.sh",
        "shell/dispatch.sh",
        "src/wrapper/cli.py",
        "src/codex_termux/__init__.py",
        "libexec/build-runtime.py",
        "libexec/bwrap-termux-compat.py",
        "libexec/rg-termux-shim.sh",
        "codex-termux/dispatch.sh",
        "source",
        "support-manifest.json",
    )
    missing = [relative for relative in required if not _lexists(target / relative)]
    if missing:
        raise IntegrityError("support artifact is incomplete: " + " ".join(missing))
    _compile_python(target / "src/wrapper")
    bash = shutil.which("bash")
    if bash is None:
        raise IntegrityError("bash is unavailable for support validation")
    shell_paths = [target / "managed.sh", target / "lib.sh"]
    shell_paths.extend(sorted((target / "shell").glob("*.sh")))
    shell_paths.extend(sorted((target / "codex-termux").glob("*.sh")))
    for path in shell_paths:
        result = subprocess.run(
            [bash, "-n", str(path)],
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode != 0:
            detail = (result.stderr or result.stdout).strip()
            raise IntegrityError(f"support shell validation failed for {path}: {detail}")

def _compile_python(root: Path) -> None:
    for path in sorted(root.rglob("*.py")):
        compile(path.read_text(encoding="utf-8"), str(path), "exec")
    _remove_bytecode(root)


def _adopt_current_path(path: Path, store: Path, prefix: str) -> Path | None:
    if path.is_symlink():
        try:
            return path.resolve(strict=True)
        except (OSError, RuntimeError) as exc:
            raise IntegrityError(f"managed pointer is invalid: {path}: {exc}") from exc
    if not path.exists():
        return None
    if not path.is_dir():
        raise IntegrityError(f"managed path is not a directory: {path}")
    adopted = store / f"{prefix}-{uuid.uuid4().hex[:12]}"
    os.replace(path, adopted)
    return adopted


def _write_transaction(path: Path, activation: SupportActivation, status: str, installed_at: str) -> None:
    _write_json(path, {**activation.to_dict(), "status": status, "installed_at": installed_at}, mode=0o600)


def _load_activation(path: Path) -> SupportActivation:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IntegrityError(f"support transaction is unreadable: {path}: {exc}") from exc
    if not isinstance(data, dict) or data.get("schema") != SCHEMA:
        raise IntegrityError(f"support transaction schema is invalid: {path}")
    fields = (
        "support_id", "target", "source_id", "source_target", "previous",
        "previous_source", "manager_link", "verified_manager_link", "source_link",
        "verified_source_link",
    )
    for field in fields:
        if not isinstance(data.get(field), str):
            raise IntegrityError(f"support transaction field is invalid: {field}")
    return SupportActivation(
        support_id=data["support_id"],
        target=data["target"],
        source_id=data["source_id"],
        source_target=data["source_target"],
        previous=data["previous"],
        previous_source=data["previous_source"],
        manager_link=data["manager_link"],
        verified_manager_link=data["verified_manager_link"],
        source_link=data["source_link"],
        verified_source_link=data["verified_source_link"],
        transaction_file=str(path),
    )


def _replace_symlink(link: Path, target: Path) -> None:
    link.parent.mkdir(parents=True, exist_ok=True)
    temporary = link.with_name(f".{link.name}.{uuid.uuid4().hex}.new")
    temporary.symlink_to(target)
    try:
        os.replace(temporary, link)
    except OSError as exc:
        temporary.unlink(missing_ok=True)
        raise IntegrityError(f"failed to replace managed pointer {link}: {exc}") from exc


def _same_target(link: Path, target: Path) -> bool:
    if not link.is_symlink():
        return False
    try:
        return link.resolve(strict=True) == target.resolve(strict=True)
    except (OSError, RuntimeError):
        return False


def _remove_path(path: Path) -> None:
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path)


def _remove_bytecode(root: Path) -> None:
    for path in sorted(root.rglob("*"), reverse=True):
        if path.is_file() and path.suffix == ".pyc":
            path.unlink()
        elif path.is_dir() and path.name == "__pycache__":
            shutil.rmtree(path)


def _read_env(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        result[key] = value
    return result


def _component(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._+-]+", "_", value or "unknown") or "unknown"


def _write_json(path: Path, data: dict[str, Any], *, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n", encoding="utf-8")
    temporary.chmod(mode)
    os.replace(temporary, path)


def _lexists(path: Path) -> bool:
    return path.exists() or path.is_symlink()
