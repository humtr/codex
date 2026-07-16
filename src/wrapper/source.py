"""Wrapper source layout validation and support installation transactions."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import json
import os
import re
import shutil
import shlex
import subprocess
import uuid
from pathlib import Path
from typing import Any

from .errors import IntegrityError


REQUIRED_WRAPPER_SOURCE_PATHS = (
    "install.sh",
    "bin/install-local.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "lib/codex-termux/prompt.sh",
    "lib/codex-termux/exec.sh",
    "lib/codex-termux/store.sh",
    "lib/codex-termux/build.sh",
    "lib/codex-termux/ui.sh",
    "lib/codex-termux/fs.sh",
    "lib/codex-termux/repair.sh",
    "lib/codex-termux/version.sh",
    "lib/codex-termux/dispatch.sh",
    "lib/codex-termux/state.sh",
    "lib/codex-termux/profile.sh",
    "lib/codex-termux/use.sh",
    "lib/codex-termux/remove.sh",
    "lib/codex-termux/session.sh",
    "lib/codex-termux/runtime.sh",
    "lib/codex-termux/notify.sh",
    "lib/codex-termux/doctor.sh",
    "codex-wrapper.manifest.json",
    "tools/build-runtime.py",
    "tools/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh",
    "tools/termux-notify.sh",
    "tools/codex-turn-notify.sh",
    "tools/codex-launcher.c",
    "tools/codex_termux",
    "config/wrapper-version.env",
)

SUPPORT_TRANSACTION_SCHEMA = 1
SUPPORT_TRANSACTION_NAME = "support-activation.json"


def missing_wrapper_source_paths(root: Path) -> list[str]:
    return [
        relative
        for relative in REQUIRED_WRAPPER_SOURCE_PATHS
        if not (root / relative).exists()
    ]


def is_wrapper_source(root: Path) -> bool:
    return not missing_wrapper_source_paths(root)


def require_wrapper_source(root: Path, label: str) -> None:
    missing = missing_wrapper_source_paths(root)
    if missing:
        missing_text = " ".join(missing)
        raise IntegrityError(f"{label} does not contain a valid wrapper source (missing: {missing_text})")


def source_commit(root: Path) -> str:
    if shutil.which("git") is None:
        return "unknown"
    try:
        inside = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--is-inside-work-tree"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
        if inside.returncode != 0:
            return "unknown"
        result = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--short=12", "HEAD"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unknown"
    if result.returncode != 0:
        return "unknown"
    return result.stdout.splitlines()[0] if result.stdout.splitlines() else "unknown"


def find_extracted_wrapper_source(extract_root: Path) -> Path:
    """Return the wrapper source root inside an extracted archive tree."""
    root = extract_root.resolve()
    if is_wrapper_source(root):
        return root
    for candidate in _archive_source_candidates(root):
        if is_wrapper_source(candidate):
            return candidate
    raise IntegrityError("wrapper source root not found in extracted archive")


def _archive_source_candidates(root: Path) -> list[Path]:
    candidates: list[Path] = []
    for marker in root.glob("*/bin/install-runtime.sh"):
        candidates.append(marker.parent.parent)
    marker = root / "bin/install-runtime.sh"
    if marker.is_file():
        candidates.insert(0, root)
    return candidates


def normalized_source_env(values: dict[str, str]) -> dict[str, str]:
    repo = values.get("CODEX_TERMUX_WRAPPER_REPO") or values.get(_legacy_key("GIT_REPO"), "")
    ref = values.get("CODEX_TERMUX_WRAPPER_REF") or values.get(_legacy_key("GIT_REF"), "")
    token = (
        values.get("CODEX_TERMUX_WRAPPER_TOKEN")
        or values.get(_legacy_key("GIT_TOKEN"), "")
        or values.get(_legacy_key("RELEASE_TOKEN"), "")
    )
    result: dict[str, str] = {}
    if repo:
        result["CODEX_TERMUX_WRAPPER_REPO"] = repo
    if ref:
        result["CODEX_TERMUX_WRAPPER_REF"] = ref
    if token:
        result["CODEX_TERMUX_WRAPPER_TOKEN"] = token
    return result


def source_env_exports(values: dict[str, str]) -> str:
    lines = []
    for key, value in normalized_source_env(values).items():
        lines.append(f"export {key}={shlex.quote(value)}")
    return "\n".join(lines)


def auth_token(values: dict[str, str], *, allow_gh: bool = False) -> str:
    normalized = normalized_source_env(values)
    token = normalized.get(env_key("TOKEN")) or values.get("GITHUB_TOKEN", "")
    if token:
        return token
    if not allow_gh:
        return ""
    return _gh_auth_token()


def _legacy_key(suffix: str) -> str:
    return "CODEX_TERMUX_WRAPPER_" + suffix


def env_key(suffix: str) -> str:
    return _legacy_key(suffix)


def _gh_auth_token() -> str:
    if shutil.which("gh") is None:
        return ""
    try:
        result = subprocess.run(
            ["gh", "auth", "token"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return ""
    if result.returncode != 0:
        return ""
    return result.stdout.splitlines()[0] if result.stdout.splitlines() else ""


@dataclass(frozen=True)
class WrapperSourcePlan:
    kind: str
    git_url: str = ""
    release_url: str = ""
    label: str = ""
    local_root: str = ""

    def to_dict(self) -> dict[str, str]:
        return asdict(self)


def wrapper_source_plan(
    *,
    repo: str = "",
    ref: str = "",
    release_url: str = "",
    release_repo: str = "",
    release_tag: str = "",
    local_root: str = "",
) -> WrapperSourcePlan:
    if repo:
        git_url = _git_url(repo)
        label = _git_label(repo, ref)
        return WrapperSourcePlan(kind="git", git_url=git_url, label=label)
    resolved_release_url = _release_url(release_url, release_repo, release_tag)
    if resolved_release_url:
        return WrapperSourcePlan(
            kind="release",
            release_url=resolved_release_url,
            label="release archive",
        )
    return WrapperSourcePlan(kind="local", local_root=local_root, label=f"local {local_root}")


def wrapper_source_plan_exports(plan: WrapperSourcePlan) -> str:
    data = {
        "CODEX_WRAPPER_SOURCE_KIND": plan.kind,
        "CODEX_WRAPPER_SOURCE_GIT_URL": plan.git_url,
        "CODEX_WRAPPER_SOURCE_RELEASE_URL": plan.release_url,
        "CODEX_WRAPPER_SOURCE_LABEL": plan.label,
        "CODEX_WRAPPER_SOURCE_LOCAL_ROOT": plan.local_root,
    }
    return "\n".join(f"{key}={shlex.quote(value)}" for key, value in data.items())


def _git_url(repo: str) -> str:
    if repo.startswith(("https://", "http://", "git@", "ssh://")):
        return repo
    return f"https://github.com/{repo}.git"


def _git_label(repo: str, ref: str) -> str:
    suffix = f"@{ref}" if ref else ""
    if repo.startswith(("https://", "http://", "git@", "ssh://")):
        return f"{repo}{suffix}"
    return f"github.com/{repo}{suffix}"


def _release_url(release_url: str, release_repo: str, release_tag: str) -> str:
    if release_url:
        return release_url
    if release_repo and release_tag:
        return f"https://github.com/{release_repo}/archive/refs/tags/{release_tag}.tar.gz"
    return ""


@dataclass(frozen=True)
class SupportActivation:
    support_id: str
    target: str
    previous: str
    manager_link: str
    verified_manager_link: str
    transaction_file: str

    def to_dict(self) -> dict[str, str | int]:
        return {
            "schema": SUPPORT_TRANSACTION_SCHEMA,
            "support_id": self.support_id,
            "target": self.target,
            "previous": self.previous,
            "manager_link": self.manager_link,
            "verified_manager_link": self.verified_manager_link,
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
    """Publish and activate a complete support artifact.

    A transaction file remains until commit_support_install() is called. Callers
    must roll back when post-switch hook or launcher validation fails.
    """
    source_root = source_root.resolve()
    wrapper_root = wrapper_root.resolve()
    state_dir = state_dir.resolve()
    require_wrapper_source(source_root, "Wrapper source")
    state_dir.mkdir(parents=True, exist_ok=True)
    support_store = wrapper_root / "support-store"
    support_store.mkdir(parents=True, exist_ok=True)
    transaction_file = state_dir / SUPPORT_TRANSACTION_NAME
    if transaction_file.exists():
        rollback_support_install(transaction_file)

    metadata = _read_env_file(source_root / "config/wrapper-version.env")
    version = metadata.get("CODEX_TERMUX_WRAPPER_VERSION", "unknown")
    commit = wrapper_commit or source_commit(source_root)
    support_id = "support-{}-{}-{}".format(
        _component(version),
        _component(commit[:12]),
        uuid.uuid4().hex[:12],
    )
    staging = support_store / f".{support_id}.staging"
    target = support_store / support_id
    if staging.exists() or target.exists():
        raise IntegrityError(f"support artifact already exists: {support_id}")

    previous = ""
    manager_was_legacy = False
    try:
        staging.mkdir(mode=0o700)
        _populate_support_artifact(
            source_root=source_root,
            target=staging,
            manager_link=manager_link,
            prefix=prefix,
            support_id=support_id,
            wrapper_commit=commit,
            installed_at=installed_at,
        )
        _validate_support_artifact(staging)
        os.replace(staging, target)

        previous_path, manager_was_legacy = _current_manager_target(
            manager_link=manager_link,
            support_store=support_store,
        )
        previous = str(previous_path) if previous_path else ""
        activation = SupportActivation(
            support_id=support_id,
            target=str(target),
            previous=previous,
            manager_link=str(manager_link),
            verified_manager_link=str(verified_manager_link),
            transaction_file=str(transaction_file),
        )
        _write_json_atomic(
            transaction_file,
            {
                **activation.to_dict(),
                "status": "prepared",
                "manager_was_legacy": manager_was_legacy,
                "installed_at": installed_at,
            },
        )
        if previous_path is not None:
            _replace_symlink(verified_manager_link, previous_path)
        elif not verified_manager_link.exists() and not verified_manager_link.is_symlink():
            _replace_symlink(verified_manager_link, target)
        _replace_symlink(manager_link, target)
        _write_json_atomic(
            transaction_file,
            {
                **activation.to_dict(),
                "status": "switched",
                "manager_was_legacy": manager_was_legacy,
                "installed_at": installed_at,
            },
        )
        return activation
    except Exception:
        if transaction_file.exists():
            try:
                rollback_support_install(transaction_file)
            except Exception:
                pass
        elif manager_was_legacy and previous:
            try:
                _replace_symlink(manager_link, Path(previous))
            except Exception:
                pass
        if staging.exists():
            shutil.rmtree(staging, ignore_errors=True)
        if target.exists() and not _same_target(manager_link, target):
            shutil.rmtree(target, ignore_errors=True)
        raise


def commit_support_install(transaction_file: Path) -> SupportActivation:
    data = _load_support_transaction(transaction_file)
    activation = _activation_from_data(data, transaction_file)
    target = Path(activation.target)
    manager_link = Path(activation.manager_link)
    if not _same_target(manager_link, target):
        raise IntegrityError("active manager does not match prepared support artifact")
    verified = Path(activation.verified_manager_link)
    if not verified.exists() and not verified.is_symlink():
        _replace_symlink(verified, target)
    transaction_file.unlink(missing_ok=True)
    return activation


def rollback_support_install(transaction_file: Path) -> SupportActivation:
    data = _load_support_transaction(transaction_file)
    activation = _activation_from_data(data, transaction_file)
    manager_link = Path(activation.manager_link)
    verified_link = Path(activation.verified_manager_link)
    target = Path(activation.target)
    previous = Path(activation.previous) if activation.previous else None

    if previous is not None and previous.exists():
        _replace_symlink(manager_link, previous)
        _replace_symlink(verified_link, previous)
    else:
        _remove_path(manager_link)
        if _same_target(verified_link, target):
            _remove_path(verified_link)
    transaction_file.unlink(missing_ok=True)
    if target.exists() and not _same_target(manager_link, target) and not _same_target(verified_link, target):
        shutil.rmtree(target)
    return activation


def _populate_support_artifact(
    *,
    source_root: Path,
    target: Path,
    manager_link: Path,
    prefix: Path,
    support_id: str,
    wrapper_commit: str,
    installed_at: str,
) -> None:
    source_snapshot = target / "source"
    _copy_source_snapshot(source_root, source_snapshot)
    shutil.copy2(source_root / "lib/codex-termux.sh", target / "lib.sh")
    shutil.copytree(source_root / "lib/codex-termux", target / "codex-termux", symlinks=True)
    shutil.copytree(source_root / "tools/codex_termux", target / "codex_termux", symlinks=True)
    for source_name, target_name in (
        ("build-runtime.py", "build-runtime.py"),
        ("bwrap-termux-compat.py", "bwrap-termux-compat.py"),
        ("rg-termux-shim.sh", "rg-termux-shim.sh"),
        ("termux-notify.sh", "termux-notify.sh"),
        ("codex-turn-notify.sh", "codex-turn-notify.sh"),
    ):
        shutil.copy2(source_root / "tools" / source_name, target / target_name)

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
                f'export CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="${{CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-{manager_link}/source/bin/install-runtime.sh}}"',
                "# shellcheck disable=SC1091",
                f'. "{manager_link}/lib.sh"',
                'codex_main "$@"',
                "",
            )
        ),
        encoding="utf-8",
    )
    managed.chmod(0o755)
    for executable in (
        "lib.sh",
        "build-runtime.py",
        "bwrap-termux-compat.py",
        "rg-termux-shim.sh",
        "termux-notify.sh",
        "codex-turn-notify.sh",
    ):
        (target / executable).chmod(0o755)

    _remove_python_bytecode(target)
    manifest = {
        "schema": 1,
        "support_id": support_id,
        "layout": "legacy-compatible-v1",
        "wrapper_commit": wrapper_commit,
        "installed_at": installed_at,
        "entrypoints": {
            "managed_shell": "managed.sh",
            "shell_loader": "lib.sh",
            "python_package": "codex_termux",
            "source_snapshot": "source",
        },
    }
    _write_json_atomic(target / "support-manifest.json", manifest)


def _copy_source_snapshot(source_root: Path, target: Path) -> None:
    target.mkdir(parents=True)
    for name in ("bin", "lib", "tools", "config"):
        shutil.copytree(source_root / name, target / name, symlinks=True)
    for name in ("install.sh", "codex-wrapper.manifest.json"):
        shutil.copy2(source_root / name, target / name)
    for optional in ("README.md",):
        path = source_root / optional
        if path.is_file():
            shutil.copy2(path, target / optional)


def _validate_support_artifact(target: Path) -> None:
    required = (
        "managed.sh",
        "lib.sh",
        "codex-termux/dispatch.sh",
        "codex_termux/cli.py",
        "build-runtime.py",
        "bwrap-termux-compat.py",
        "rg-termux-shim.sh",
        "termux-notify.sh",
        "codex-turn-notify.sh",
        "wrapper-version.env",
        "source/bin/install-runtime.sh",
        "support-manifest.json",
    )
    missing = [relative for relative in required if not (target / relative).exists()]
    if missing:
        raise IntegrityError("support artifact is incomplete: " + " ".join(missing))

    for path in sorted((target / "codex_termux").glob("*.py")):
        source_text = path.read_text(encoding="utf-8")
        compile(source_text, str(path), "exec")
    bash = shutil.which("bash")
    if bash is None:
        raise IntegrityError("bash is unavailable for support validation")
    shell_paths = [target / "managed.sh", target / "lib.sh"]
    shell_paths.extend(sorted((target / "codex-termux").glob("*.sh")))
    shell_paths.extend(
        (
            target / "rg-termux-shim.sh",
            target / "termux-notify.sh",
            target / "codex-turn-notify.sh",
        )
    )
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
            raise IntegrityError(f"support shell validation failed for {path.name}: {detail}")


def _current_manager_target(*, manager_link: Path, support_store: Path) -> tuple[Path | None, bool]:
    if manager_link.is_symlink():
        return _resolved_link(manager_link), False
    if not manager_link.exists():
        return None, False
    if not manager_link.is_dir():
        raise IntegrityError(f"manager path is not a directory or symlink: {manager_link}")
    legacy_target = support_store / f"legacy-{uuid.uuid4().hex[:12]}"
    os.replace(manager_link, legacy_target)
    return legacy_target, True


def _replace_symlink(link: Path, target: Path) -> None:
    link.parent.mkdir(parents=True, exist_ok=True)
    temporary = link.with_name(f".{link.name}.{uuid.uuid4().hex}.new")
    temporary.symlink_to(target)
    try:
        os.replace(temporary, link)
    except OSError as exc:
        temporary.unlink(missing_ok=True)
        raise IntegrityError(f"failed to replace support pointer {link}: {exc}") from exc


def _resolved_link(path: Path) -> Path:
    try:
        return path.resolve(strict=True)
    except (OSError, RuntimeError) as exc:
        raise IntegrityError(f"support pointer is invalid: {path}: {exc}") from exc


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


def _remove_python_bytecode(root: Path) -> None:
    for path in sorted(root.rglob("*"), reverse=True):
        if path.is_file() and path.suffix == ".pyc":
            path.unlink()
        elif path.is_dir() and path.name == "__pycache__":
            shutil.rmtree(path)


def _read_env_file(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        result[key] = value
    return result


def _component(value: str) -> str:
    result = re.sub(r"[^A-Za-z0-9._+-]+", "_", value or "unknown")
    return result or "unknown"


def _write_json_atomic(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    temporary.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n", encoding="utf-8")
    temporary.chmod(0o600)
    os.replace(temporary, path)


def _load_support_transaction(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise IntegrityError(f"support transaction is unreadable: {path}: {exc}") from exc
    if not isinstance(data, dict) or data.get("schema") != SUPPORT_TRANSACTION_SCHEMA:
        raise IntegrityError(f"support transaction schema is invalid: {path}")
    for field in ("support_id", "target", "previous", "manager_link", "verified_manager_link"):
        if not isinstance(data.get(field), str):
            raise IntegrityError(f"support transaction field is invalid: {field}")
    return data


def _activation_from_data(data: dict[str, Any], path: Path) -> SupportActivation:
    return SupportActivation(
        support_id=data["support_id"],
        target=data["target"],
        previous=data["previous"],
        manager_link=data["manager_link"],
        verified_manager_link=data["verified_manager_link"],
        transaction_file=str(path),
    )
