"""Role-oriented ownership checks layered over the mature canon audit."""

from __future__ import annotations

from pathlib import Path
from typing import Any


_REPLACED_FINDINGS = {
    "source-config-multiple-owners",
    "notify-domain-multiple-owners",
}
_SOURCE_FACADES = (
    "lib/codex-termux/state.sh",
    "tools/codex_termux/__init__.py",
    "tools/codex_termux/cli.py",
)
_NOTIFY_FACADES = (
    "lib/codex-termux/notify.sh",
    "tools/codex-turn-notify.sh",
    "tools/termux-notify.sh",
    "tools/codex_termux/notify.py",
    "src/wrapper/notify.py",
)
_AUDIT_IMPLEMENTATION = {
    "src/wrapper/_legacy_canon.py",
    "src/wrapper/canon.py",
    "src/wrapper/canon_policy.py",
}
_SOURCE_POLICY_MARKERS = (
    "CODEX_TERMUX_WRAPPER_GIT_REPO",
    "CODEX_TERMUX_WRAPPER_RELEASE_TOKEN",
    "def source_env_exports",
    "def wrapper_source_plan",
)
_NOTIFY_POLICY_MARKERS = (
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "SubagentStop",
    "toast_duration",
    "ClickAction",
)


def install(canon_module: Any) -> None:
    if getattr(canon_module, "_ROLE_POLICY_INSTALLED", False):
        return
    original = canon_module.audit

    def audit(root: Path) -> dict[str, Any]:
        report = original(root)
        return enforce(report, root)

    canon_module.audit = audit
    canon_module._ROLE_POLICY_INSTALLED = True


def enforce(report: dict[str, Any], root: Path) -> dict[str, Any]:
    root = root.resolve()
    findings = [
        item
        for item in report.get("findings", [])
        if isinstance(item, dict) and item.get("code") not in _REPLACED_FINDINGS
    ]
    findings.extend(_facade_findings(root, _SOURCE_FACADES, _SOURCE_POLICY_MARKERS, "source"))
    findings.extend(_facade_findings(root, _NOTIFY_FACADES, _NOTIFY_POLICY_MARKERS, "notify"))
    findings.extend(_policy_location_findings(root))
    report["findings"] = findings
    if any(item.get("severity") == "blocker" for item in findings):
        report["status"] = "needs-canon"
    elif report.get("status") == "needs-canon":
        report["status"] = "ok"
    return report


def _facade_findings(
    root: Path,
    paths: tuple[str, ...],
    markers: tuple[str, ...],
    domain: str,
) -> list[dict[str, str]]:
    findings: list[dict[str, str]] = []
    for relative in paths:
        path = root / relative
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        present = [marker for marker in markers if marker in text]
        if present:
            findings.append(
                {
                    "code": f"{domain}-policy-in-compat-facade",
                    "severity": "blocker",
                    "path": relative,
                    "detail": "compatibility facade contains policy markers: " + ", ".join(present),
                }
            )
    return findings


def _policy_location_findings(root: Path) -> list[dict[str, str]]:
    findings: list[dict[str, str]] = []
    ignored = {".git", "tests", "docs", ".github", ".agents", "__pycache__"}
    for path in root.rglob("*"):
        if not path.is_file() or any(part in ignored for part in path.parts):
            continue
        relative = str(path.relative_to(root))
        if (
            relative in _SOURCE_FACADES
            or relative in _NOTIFY_FACADES
            or relative in _AUDIT_IMPLEMENTATION
        ):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        source_score = sum(marker in text for marker in _SOURCE_POLICY_MARKERS)
        notify_score = sum(marker in text for marker in _NOTIFY_POLICY_MARKERS)
        if source_score >= 2 and not relative.startswith(("src/wrapper/", "shell/state.sh", "install.sh")):
            findings.append(
                {
                    "code": "source-policy-outside-role-owner",
                    "severity": "blocker",
                    "path": relative,
                    "detail": "wrapper source policy must be owned by src/wrapper with shell/state.sh as adapter",
                }
            )
        if notify_score >= 3 and not relative.startswith("src/wrapper/notification/"):
            findings.append(
                {
                    "code": "notify-policy-outside-notification-package",
                    "severity": "blocker",
                    "path": relative,
                    "detail": "notification event and rendering policy must live under src/wrapper/notification",
                }
            )
    return findings
