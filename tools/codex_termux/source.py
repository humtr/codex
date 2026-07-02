"""Wrapper source layout validation."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from pathlib import Path

from .errors import IntegrityError


REQUIRED_WRAPPER_SOURCE_PATHS = (
    "install.sh",
    "bin/install-local.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "lib/codex-termux/dispatch.sh",
    "lib/codex-termux/state.sh",
    "lib/codex-termux/profile.sh",
    "lib/codex-termux/session.sh",
    "lib/codex-termux/runtime.sh",
    "lib/codex-termux/notify.sh",
    "lib/codex-termux/doctor.sh",
    "codex-wrapper.manifest.json",
    "tools/build-runtime.py",
    "tools/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh",
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
