"""Canonicalization audit helpers for the Codex Termux wrapper.

This module intentionally reports structure drift without changing runtime
behavior.  The release validator can opt into strict enforcement only after the
corresponding canon phases have landed.
"""

from __future__ import annotations

import json
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

MANIFEST_PATH = "codex-wrapper.manifest.json"
DOMAIN_DIR = "lib/codex-termux"
_FUNCTION_CALL_RE = re.compile(r"\b(codex_[A-Za-z0-9_]+)\b")


def _load_manifest(root: Path, findings: list[Finding]) -> dict[str, object]:
    path = root / MANIFEST_PATH
    if not path.is_file():
        findings.append(Finding("manifest-missing", "blocker", MANIFEST_PATH, "wrapper contract manifest is missing"))
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        findings.append(Finding("manifest-invalid-json", "blocker", MANIFEST_PATH, str(exc)))
        return {}
    return data if isinstance(data, dict) else {}


def _shell_contract_files(root: Path) -> list[Path]:
    files = [root / "lib/codex-termux.sh"]
    domain_dir = root / DOMAIN_DIR
    if domain_dir.is_dir():
        files.extend(sorted(domain_dir.glob("*.sh")))
    return [path for path in files if path.is_file()]


def _shell_contract_text(root: Path) -> str:
    return "\n".join(_read_text(path) for path in _shell_contract_files(root))


def audit(root: Path) -> dict[str, object]:
    root = root.resolve()
    findings: list[Finding] = []
    files = list(_source_files(root))

    findings.extend(_audit_removed_contracts(root, files))
    findings.extend(_audit_top_level_command_refs(root, files))
    findings.extend(_audit_source_config_owners(root, files))
    findings.extend(_audit_notify_owners(root, files))
    manifest = _load_manifest(root, findings)
    findings.extend(_audit_manifest_consistency(root, manifest))
    findings.extend(_audit_domain_ownership(root, manifest))
    findings.extend(_audit_protected_path_contracts(root, manifest))
    findings.extend(_audit_public_entrypoints(root, manifest))
    metrics = _metrics(root)
    findings.extend(_audit_profile_shell_model(root))
    findings.extend(_audit_shell_budgets(manifest, metrics))
    findings.extend(_audit_forbidden_shell_patterns(root, manifest))
    return {
        "status": "ok" if not _blocking_findings(findings) else "needs-canon",
        "phases": _phase_status(findings),
        "metrics": metrics,
        "findings": [finding.as_dict() for finding in findings],
    }


def _source_files(root: Path) -> Iterable[Path]:
    ignored_dirs = {".git", "__pycache__", ".pytest_cache", ".mypy_cache", "out"}
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
    allowed_split = {
        "lib/codex-termux/notify.sh",
        "tools/codex-turn-notify.sh",
        "tools/codex_termux/notify.py",
    }
    for path in files:
        relative = _relative(root, path)
        if relative == "tools/codex_termux/canon.py" or _is_test_or_dev_only(relative):
            continue
        text = _read_text(path)
        if sum(1 for marker in NOTIFY_DOMAIN_MARKERS if marker in text) >= 3:
            owners.append(relative)
    if owners and set(owners).issubset(allowed_split):
        return []
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
    if not (root / "lib/codex-termux.sh").is_file():
        return []
    text = _shell_contract_text(root)
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


def _audit_shell_budgets(manifest: dict[str, object], metrics: dict[str, object]) -> list[Finding]:
    budgets = manifest.get("shell_budgets", {})
    if not isinstance(budgets, dict):
        return []
    findings: list[Finding] = []
    for key, limit in budgets.items():
        if not isinstance(key, str) or not isinstance(limit, int):
            findings.append(
                Finding(
                    code="shell-budget-invalid",
                    severity="blocker",
                    path=MANIFEST_PATH,
                    detail=f"invalid shell budget entry: {key}",
                )
            )
            continue
        value = metrics.get(key)
        if isinstance(value, int) and value > limit:
            findings.append(
                Finding(
                    code="shell-budget-exceeded",
                    severity="blocker",
                    path=MANIFEST_PATH,
                    detail=f"{key}={value} exceeds budget {limit}",
                )
            )
    return findings


def _audit_forbidden_shell_patterns(root: Path, manifest: dict[str, object]) -> list[Finding]:
    entries = manifest.get("forbidden_shell_patterns", [])
    if not isinstance(entries, list):
        return []
    targets = [root / "bin/install-runtime.sh", *_shell_contract_files(root)]
    findings: list[Finding] = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        pattern = entry.get("pattern", "")
        reason = entry.get("reason", "forbidden shell pattern")
        if not isinstance(pattern, str) or not pattern:
            continue
        for path in targets:
            text = _read_text(path)
            if pattern in text:
                findings.append(
                    Finding(
                        code="forbidden-shell-pattern",
                        severity="blocker",
                        path=_relative(root, path),
                        detail=f"{reason}: {pattern}",
                    )
                )
    return findings


def _manifest_domains(manifest: dict[str, object]) -> dict[str, dict[str, object]]:
    domains = manifest.get("domain_ownership", {}) if isinstance(manifest, dict) else {}
    return {str(k): v for k, v in domains.items() if isinstance(v, dict)} if isinstance(domains, dict) else {}


def _audit_manifest_consistency(root: Path, manifest: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    if not manifest:
        return [Finding("manifest-missing-or-invalid", "blocker", MANIFEST_PATH, "manifest is missing or invalid")]
    if manifest.get("schema") != 1:
        findings.append(Finding("manifest-schema", "blocker", MANIFEST_PATH, "schema must be 1"))
    domains = _manifest_domains(manifest)
    expected = {"dispatch", "state", "profile", "session", "runtime", "notify", "doctor"}
    for domain in sorted(expected - set(domains)):
        findings.append(Finding("manifest-domain-missing", "blocker", MANIFEST_PATH, f"missing domain: {domain}"))
    for name, data in domains.items():
        rel = str(data.get("path", ""))
        if not rel or not (root / rel).is_file():
            findings.append(Finding("manifest-domain-path", "blocker", rel or MANIFEST_PATH, f"domain path missing: {name}"))
        if not isinstance(data.get("public_functions", []), list):
            findings.append(Finding("manifest-public-functions", "blocker", rel or MANIFEST_PATH, f"public_functions must be a list: {name}"))
    for entry in manifest.get("public_entrypoints", []):
        if isinstance(entry, dict) and isinstance(entry.get("path"), str) and not (root / entry["path"]).is_file():
            findings.append(Finding("manifest-entrypoint-path", "blocker", entry["path"], "public entrypoint path is missing"))
    return findings


def _function_defs_by_domain(root: Path, manifest: dict[str, object]) -> dict[str, str]:
    owners: dict[str, str] = {}
    for domain, data in _manifest_domains(manifest).items():
        path = root / str(data.get("path", ""))
        if not path.is_file():
            continue
        for match in _SHELL_FUNCTION_RE.finditer(_read_text(path)):
            owners[match.group(0).split("(", 1)[0]] = domain
    return owners


def _audit_domain_ownership(root: Path, manifest: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    domains = _manifest_domains(manifest)
    owners = _function_defs_by_domain(root, manifest)
    seen: dict[str, str] = {}
    for domain, data in domains.items():
        rel = str(data.get("path", ""))
        path = root / rel
        if not path.is_file():
            continue
        for match in _SHELL_FUNCTION_RE.finditer(_read_text(path)):
            fn = match.group(0).split("(", 1)[0]
            if fn in seen:
                findings.append(Finding("domain-function-duplicate", "blocker", rel, f"{fn} also defined in {seen[fn]}"))
            seen[fn] = rel
        for fn in data.get("public_functions", []):
            if isinstance(fn, str) and owners.get(fn) != domain:
                findings.append(Finding("domain-public-function-owner", "blocker", rel, f"{fn} not owned by {domain}"))
    loader = _read_text(root / "lib/codex-termux.sh")
    for domain in domains:
        if domain not in loader:
            findings.append(Finding("loader-domain-missing", "blocker", "lib/codex-termux.sh", f"loader does not source {domain}"))
    allow_map = manifest.get("allowed_private_cross_domain_calls", {}) if isinstance(manifest.get("allowed_private_cross_domain_calls", {}), dict) else {}
    global_allowed = set(allow_map.get("*", []))
    public = {fn for data in domains.values() for fn in data.get("public_functions", []) if isinstance(fn, str)}
    for domain, data in domains.items():
        rel = str(data.get("path", "")); path = root / rel
        if not path.is_file():
            continue
        allowed = set(global_allowed)
        if isinstance(allow_map.get(domain), list):
            allowed.update(allow_map[domain])
        body = _SHELL_FUNCTION_RE.sub("", _read_text(path))
        for call in sorted(set(_FUNCTION_CALL_RE.findall(body))):
            owner = owners.get(call)
            if owner and owner != domain and call not in public and call not in allowed:
                findings.append(Finding("private-cross-domain-call", "blocker", rel, f"{domain} calls private {owner} function {call}"))
    return findings


def _audit_protected_path_contracts(root: Path, manifest: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    install_runtime = _read_text(root / "bin/install-runtime.sh")
    shell = _shell_contract_text(root)
    for marker in (
        "codex_assert_managed_tree_target",
        "codex_rm_rf_managed",
        "cp \"$source_dir/lib/codex-termux.sh\" \"$CODEX_TERMUX_MANAGER_DIR/lib.sh\"",
        "cp -R \"$source_dir/lib/codex-termux\" \"$CODEX_TERMUX_MANAGER_DIR/codex-termux\"",
        "codex_try_verified_rollback",
    ):
        if marker not in install_runtime and marker not in shell:
            findings.append(Finding("protected-path-marker-missing", "blocker", "bin/install-runtime.sh", f"missing protected path marker: {marker}"))
    return findings


def _audit_public_entrypoints(root: Path, manifest: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    checks = (
        ("lib/codex-termux.sh", "codex_source_domain"),
        ("lib/codex-termux/dispatch.sh", "codex_main()"),
        ("lib/codex-termux/dispatch.sh", "codex_termux_main()"),
        ("bin/install-runtime.sh", 'codex_termux_doctor "$@"'),
        ("bin/install-runtime.sh", "install upstream [VERSION]"),
        ("bin/install-runtime.sh", "install rebuild"),
    )
    for rel, marker in checks:
        if marker not in _read_text(root / rel):
            findings.append(Finding("entrypoint-compat-marker-missing", "blocker", rel, f"missing marker: {marker}"))
    return findings

def _metrics(root: Path) -> dict[str, object]:
    lib = root / "lib/codex-termux.sh"
    domain_dir = root / DOMAIN_DIR
    install_runtime = root / "bin/install-runtime.sh"
    session_py = root / "tools/codex_termux/session.py"
    metrics: dict[str, object] = {}
    for key, path in (("lib_lines", lib), ("install_runtime_lines", install_runtime), ("session_py_lines", session_py)):
        metrics[key] = len(_read_text(path).splitlines()) if path.is_file() else 0
    shell = _shell_contract_text(root)
    metrics["lib_shell_functions"] = len(_SHELL_FUNCTION_RE.findall(shell))
    metrics["domain_files"] = len(list(domain_dir.glob("*.sh"))) if domain_dir.is_dir() else 0
    metrics["domain_shell_lines"] = sum(len(_read_text(path).splitlines()) for path in sorted(domain_dir.glob("*.sh"))) if domain_dir.is_dir() else 0
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
