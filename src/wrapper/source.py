"""Wrapper source layout validation."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import shutil
import shlex
import subprocess
from pathlib import Path

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


def prepare_support_install(**kwargs):
    from .support_transaction import prepare_support_install as implementation
    return implementation(**kwargs)

def commit_support_install(transaction_file: Path):
    from .support_transaction import commit_support_install as implementation
    return implementation(transaction_file)

def rollback_support_install(transaction_file: Path):
    from .support_transaction import rollback_support_install as implementation
    return implementation(transaction_file)
