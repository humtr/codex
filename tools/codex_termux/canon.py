"""Canonicalization audit helpers for the Codex Termux wrapper.

This module intentionally reports structure drift without changing runtime
behavior.  The release validator can opt into strict enforcement only after the
corresponding canon phases have landed.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class Finding:
    code: str
    severity: str
    path: str
    detail: str

    def as_dict(self) -> dict[str, str]:
        return {
            "code": self.code,
            "severity": self.severity,
            "path": self.path,
            "detail": self.detail,
        }


WRAPPER_TOP_LEVEL_COMMANDS = (
    "install",
    "update",
    "repair",
    "notify",
    "use",
    "session",
    "profile",
    "doctor",
    "version",
    "remove",
    "setup",
)

REMOVED_CONTRACT_TERMS = (
    "".join(("codex", "_native")),
    "".join(("CODEX", "_NATIVE")),
    "".join(("codex/", "native")),
    "".join(("codex", " ", "native")),
    "".join(("native", ".lock")),
    "".join(("CODEX_TERMUX", "_RESOLVER_FD")),
    "".join(("CODEX_TERMUX", "_SHARED_PLUGINS_DIR")),
    "".join(("codex_profile", "_share_plugins")),
)

SOURCE_CONFIG_OWNER_MARKERS = (
    "codex_load_wrapper_source_config()",
    "codex_normalize_wrapper_source_config()",
    "codex_wrapper_auth_token()",
    "CODEX_TERMUX_WRAPPER_GIT_REPO",
    "CODEX_TERMUX_WRAPPER_RELEASE_TOKEN",
)

NOTIFY_DOMAIN_MARKERS = (
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Stop",
)

PROFILE_SHELL_MODEL_MARKERS = (
    "codex_profile_validate_name()",
    "codex_profile_dir()",
    "codex_profile_choice_to_name()",
    "codex_profile_write_recent()",
    "codex_profile_read_recent()",
    "codex_profile_menu_ids()",
)

PHASE_SEVERITIES = {
    "c001": "phase-c001",
    "c002": "phase-c002",
    "c003": "phase-c003",
    "c004": "phase-c004",
}

_TOP_LEVEL_RE = re.compile(
    r"\bcodex\s+(" + "|".join(re.escape(item) for item in WRAPPER_TOP_LEVEL_COMMANDS) + r")(?:\s|$)"
)
_SHELL_FUNCTION_RE = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_]*\(\) \{", re.M)


def audit(root: Path) -> dict[str, object]:
    root = root.resolve()
    findings: list[Finding] = []
    files = list(_source_files(root))

    findings.extend(_audit_removed_contracts(root, files))
    findings.extend(_audit_top_level_command_refs(root, files))
    findings.extend(_audit_source_config_owners(root, files))
    findings.extend(_audit_notify_owners(root, files))
    findings.extend(_audit_profile_shell_model(root))

    metrics = _metrics(root)
    return {
        "status": "ok" if not _blocking_findings(findings) else "needs-canon",
        "phases": _phase_status(findings),
        "metrics": metrics,
        "findings": [finding.as_dict() for finding in findings],
    }


def _source_files(root: Path) -> Iterable[Path]:
    ignored_dirs = {".git", "__pycache__", ".pytest_cache", ".mypy_cache"}
    ignored_suffixes = {".pyc", ".zip", ".tgz", ".tar", ".gz"}
    for path in root.rglob("*"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        if not path.is_file():
            continue
        if path.suffix in ignored_suffixes:
            continue
        yield path


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return ""


def _relative(root: Path, path: Path) -> str:
    return str(path.relative_to(root))


def _is_test_or_dev_only(relative: str) -> bool:
    return relative.startswith(("tests/", ".github/", "docs/", ".agents/"))


def _is_bootstrap_source_config_compat(relative: str) -> bool:
    return relative == "install.sh"


def _audit_removed_contracts(root: Path, files: Iterable[Path]) -> list[Finding]:
    findings: list[Finding] = []
    for path in files:
        text = _read_text(path)
        for term in REMOVED_CONTRACT_TERMS:
            if term in text:
                findings.append(
                    Finding(
                        code="removed-contract",
                        severity="blocker",
                        path=_relative(root, path),
                        detail=f"removed contract term remains: {term}",
                    )
                )
    return findings


def _audit_top_level_command_refs(root: Path, files: Iterable[Path]) -> list[Finding]:
    findings: list[Finding] = []
    for path in files:
        relative = _relative(root, path)
        text = _read_text(path)
        for lineno, line in enumerate(text.splitlines(), start=1):
            if "codex termux" in line:
                continue
            match = _TOP_LEVEL_RE.search(line)
            if match:
                findings.append(
                    Finding(
                        code="top-level-wrapper-command",
                        severity="phase-c001",
                        path=f"{relative}:{lineno}",
                        detail=f"top-level wrapper command reference: {match.group(0).strip()}",
                    )
                )
    return findings


def _audit_source_config_owners(root: Path, files: Iterable[Path]) -> list[Finding]:
    owners: list[str] = []
    for path in files:
        relative = _relative(root, path)
        if (
            relative == "tools/codex_termux/canon.py"
            or _is_test_or_dev_only(relative)
            or _is_bootstrap_source_config_compat(relative)
        ):
            continue
        text = _read_text(path)
        if any(marker in text for marker in SOURCE_CONFIG_OWNER_MARKERS):
            owners.append(relative)
    if len(owners) <= 1:
        return []
    return [
        Finding(
            code="source-config-multiple-owners",
            severity="phase-c002",
            path=", ".join(sorted(owners)),
            detail="wrapper source config normalization has multiple textual owners",
        )
    ]


def _audit_notify_owners(root: Path, files: Iterable[Path]) -> list[Finding]:
    owners: list[str] = []
    for path in files:
        relative = _relative(root, path)
        if relative == "tools/codex_termux/canon.py" or _is_test_or_dev_only(relative):
            continue
        text = _read_text(path)
        if sum(1 for marker in NOTIFY_DOMAIN_MARKERS if marker in text) >= 3:
            owners.append(relative)
    if len(owners) <= 1:
        return []
    return [
        Finding(
            code="notify-domain-multiple-owners",
            severity="phase-c003",
            path=", ".join(sorted(owners)),
            detail="notify hook/event domain appears in multiple files",
        )
    ]


def _audit_profile_shell_model(root: Path) -> list[Finding]:
    shell = root / "lib/codex-termux.sh"
    if not shell.is_file():
        return []
    text = _read_text(shell)
    markers = [marker for marker in PROFILE_SHELL_MODEL_MARKERS if marker in text]
    if not markers:
        return []
    return [
        Finding(
            code="profile-shell-model",
            severity="phase-c004",
            path="lib/codex-termux.sh",
            detail="profile model still lives in shell: " + ", ".join(markers),
        )
    ]


def _metrics(root: Path) -> dict[str, object]:
    lib = root / "lib/codex-termux.sh"
    install_runtime = root / "bin/install-runtime.sh"
    session_py = root / "tools/codex_termux/session.py"
    metrics: dict[str, object] = {}
    for key, path in (
        ("lib_lines", lib),
        ("install_runtime_lines", install_runtime),
        ("session_py_lines", session_py),
    ):
        metrics[key] = len(_read_text(path).splitlines()) if path.is_file() else 0
    shell = _read_text(lib) if lib.is_file() else ""
    metrics["lib_shell_functions"] = len(_SHELL_FUNCTION_RE.findall(shell))
    metrics["notify_shell_functions"] = len(re.findall(r"^codex_notify[A-Za-z0-9_]*\(\) \{", shell, re.M))
    metrics["profile_shell_functions"] = len(re.findall(r"^codex_profile[A-Za-z0-9_]*\(\) \{", shell, re.M))
    return metrics


def _blocking_findings(findings: Iterable[Finding]) -> list[Finding]:
    return [finding for finding in findings if finding.severity == "blocker"]


def _phase_status(findings: Iterable[Finding]) -> dict[str, str]:
    active = {finding.severity for finding in findings}
    return {
        phase: "needs-canon" if severity in active else "ok"
        for phase, severity in PHASE_SEVERITIES.items()
    }
