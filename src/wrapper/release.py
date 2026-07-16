"""Release package validation and archive writing."""

from __future__ import annotations

import os
import zipfile
from pathlib import Path

from .errors import IntegrityError
from .source import REQUIRED_WRAPPER_SOURCE_PATHS


FORBIDDEN_RELEASE_ROOTS = (
    "tests",
    ".github",
    "docs",
    ".agents",
    ".git",
    "dist",
)
FORBIDDEN_RELEASE_EXACT = (
    ".gitignore",
    "GOAL.md",
    "tools/install-git-hooks.sh",
    "tools/update-wrapper-version.sh",
    "tools/golden_capture.py",
)
REMOVED_CONTRACT_TERMS = (
    "".join(("codex", "_native")),
    "".join(("CODEX", "_NATIVE")),
    "".join(("codex/", "native")),
    "".join(("codex", " native")),
    "".join(("native", ".lock")),
    "".join(("CODEX_TERMUX", "_RESOLVER_FD")),
    "".join(("CODEX_TERMUX", "_SHARED_PLUGINS_DIR")),
    "".join(("codex_profile", "_share_plugins")),
)

ROLE_PACKAGE_MODULES = (
    "__init__.py",
    "_legacy_canon.py",
    "_legacy_source.py",
    "activation.py",
    "atomic.py",
    "canon.py",
    "canon_policy.py",
    "cli.py",
    "cli_activation.py",
    "cli_artifacts.py",
    "cli_doctor.py",
    "cli_notify.py",
    "cli_product.py",
    "cli_profile.py",
    "cli_repair.py",
    "cli_runtime.py",
    "cli_session.py",
    "cli_store.py",
    "cli_ui.py",
    "cli_use.py",
    "doctor.py",
    "errors.py",
    "hashing.py",
    "install_plan.py",
    "notify.py",
    "paths.py",
    "prune.py",
    "registry.py",
    "release.py",
    "repair.py",
    "runtime_checks.py",
    "runtime_env.py",
    "schemas.py",
    "session.py",
    "source.py",
    "state.py",
    "store.py",
    "support_diagnostics.py",
    "support_layout.py",
    "support_transaction.py",
    "ui.py",
    "use.py",
)


def required_release_entries() -> tuple[str, ...]:
    source_entries = tuple(
        entry for entry in REQUIRED_WRAPPER_SOURCE_PATHS if entry != "tools/codex_termux"
    )
    compatibility = (
        "tools/codex_termux/__init__.py",
        "tools/codex_termux/cli.py",
        "tools/codex_termux/notify.py",
    )
    modules = tuple(f"src/wrapper/{name}" for name in ROLE_PACKAGE_MODULES)
    notification = (
        "src/wrapper/notification/__init__.py",
        "src/wrapper/notification/config.py",
        "src/wrapper/notification/hooks.py",
        "src/wrapper/notification/model.py",
        "src/wrapper/notification/provider.py",
        "src/wrapper/notification/service.py",
        "src/codex_termux/__init__.py",
    )
    return tuple(
        dict.fromkeys(("README.md", *source_entries, *compatibility, *modules, *notification))
    )


def validate_package_root(package_root: Path) -> None:
    for relative in required_release_entries():
        if not (package_root / relative).exists():
            raise IntegrityError(f"required release entry is missing: {relative}")
    for root_name in FORBIDDEN_RELEASE_ROOTS:
        if (package_root / root_name).exists():
            raise IntegrityError(f"forbidden release entry: {root_name}")
    for relative in FORBIDDEN_RELEASE_EXACT:
        if (package_root / relative).exists():
            raise IntegrityError(f"forbidden release entry: {relative}")
    for path in package_root.rglob("*"):
        if "__pycache__" in path.parts or path.suffix == ".pyc":
            raise IntegrityError(
                f"Python bytecode artifact in release package: {path.relative_to(package_root)}"
            )
        if path.is_file():
            _validate_removed_terms(package_root, path)


def _validate_removed_terms(package_root: Path, path: Path) -> None:
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return
    for term in REMOVED_CONTRACT_TERMS:
        if term in text:
            relative = path.relative_to(package_root)
            raise IntegrityError(
                f"removed legacy contract remains in release package: {term} in {relative}"
            )


def write_zip(package_root: Path, out: Path) -> None:
    package_root = package_root.resolve()
    out = out.resolve()
    validate_package_root(package_root)
    if out.exists():
        out.unlink()
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(package_root.rglob("*")):
            if path.is_dir():
                continue
            relative = path.relative_to(package_root.parent)
            info = zipfile.ZipInfo(str(relative).replace(os.sep, "/"))
            info.external_attr = (0o755 if os.access(path, os.X_OK) else 0o644) << 16
            with path.open("rb") as handle:
                archive.writestr(info, handle.read())
