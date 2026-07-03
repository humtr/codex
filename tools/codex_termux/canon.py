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
_SHELL_FUNCTION_NAME_RE = re.compile(r"^([A-Za-z0-9_][A-Za-z0-9_]*)\(\) \{", re.M)
_CLI_ADD_PARSER_RE = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*\.add_parser\(\"([A-Za-z0-9_-]+)\"")
_SHELL_HELPER_COMMAND_RE = re.compile(r"\bcodex_termux_cmd\s+([A-Za-z0-9][A-Za-z0-9_-]*)\b")

MANIFEST_PATH = "codex-wrapper.manifest.json"
DOMAIN_DIR = "lib/codex-termux"
_FUNCTION_CALL_RE = re.compile(r"\b(codex_[A-Za-z0-9_]+)\b")
SHELL_FUNCTION_CLASSES = {
    "dispatch",
    "prompt",
    "exec_fd",
    "file_mutation",
    "lock_temp",
    "env_setup",
    "termux_glue",
}


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


def _shell_classification_files(root: Path) -> list[Path]:
    return [
        path
        for path in [root / "bin/install-runtime.sh", *_shell_contract_files(root)]
        if path.is_file()
    ]


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
    findings.extend(_audit_product_repo_purity(root, manifest))
    findings.extend(_audit_python_module_ownership(root, manifest))
    findings.extend(_audit_helper_command_surface(metrics))
    classification_metrics, classification_findings = _audit_shell_function_classification(root, manifest)
    metrics.update(classification_metrics)
    findings.extend(classification_findings)
    metrics["target_budget_gaps"] = _target_budget_gaps(manifest, metrics)
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
    profile_shell = root / "lib/codex-termux/profile.sh"
    if not profile_shell.is_file():
        return []
    text = _read_text(profile_shell)
    markers = [marker for marker in PROFILE_SHELL_MODEL_MARKERS if marker in text]
    if not markers:
        return []
    return [
        Finding(
            code="profile-shell-model",
            severity="phase-c004",
            path="lib/codex-termux/profile.sh",
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


def _audit_product_repo_purity(root: Path, manifest: dict[str, object]) -> list[Finding]:
    findings: list[Finding] = []
    paths = manifest.get("product_forbidden_paths", [])
    if isinstance(paths, list):
        for entry in paths:
            if not isinstance(entry, str) or not entry:
                continue
            matches = sorted(root.glob(entry))
            for path in matches:
                findings.append(
                    Finding(
                        code="product-forbidden-path",
                        severity="blocker",
                        path=_relative(root, path),
                        detail=f"forbidden product-repo path exists: {entry}",
                    )
                )
    patterns = manifest.get("product_forbidden_patterns", [])
    if not isinstance(patterns, list):
        return findings
    skip = {MANIFEST_PATH, "tools/codex_termux/canon.py"}
    for entry in patterns:
        if not isinstance(entry, dict):
            continue
        pattern = entry.get("pattern", "")
        reason = entry.get("reason", "forbidden product-repo pattern")
        if not isinstance(pattern, str) or not pattern:
            continue
        for path in _source_files(root):
            relative = _relative(root, path)
            if relative in skip or _is_test_or_dev_only(relative):
                continue
            for lineno, line in enumerate(_read_text(path).splitlines(), start=1):
                if pattern in line:
                    findings.append(
                        Finding(
                            code="product-forbidden-pattern",
                            severity="blocker",
                            path=f"{relative}:{lineno}",
                            detail=f"{reason}: {pattern}",
                        )
                    )
    return findings


def _audit_python_module_ownership(root: Path, manifest: dict[str, object]) -> list[Finding]:
    ownership = manifest.get("python_module_ownership", {})
    if not isinstance(ownership, dict):
        return [
            Finding(
                code="python-module-ownership-missing",
                severity="blocker",
                path=MANIFEST_PATH,
                detail="python_module_ownership must be a mapping",
            )
        ]
    findings: list[Finding] = []
    py_files = sorted((root / "tools/codex_termux").glob("*.py"))
    owned_paths = {key for key, value in ownership.items() if isinstance(key, str) and isinstance(value, str) and value}
    for path in py_files:
        relative = _relative(root, path)
        if relative not in owned_paths:
            findings.append(
                Finding(
                    code="python-module-unowned",
                    severity="blocker",
                    path=relative,
                    detail="Python helper module lacks manifest ownership",
                )
            )
    for relative in sorted(owned_paths):
        if not (root / relative).is_file():
            findings.append(
                Finding(
                    code="python-module-owner-path-missing",
                    severity="blocker",
                    path=relative,
                    detail="python_module_ownership path does not exist",
                )
            )
    return findings


def _audit_helper_command_surface(metrics: dict[str, object]) -> list[Finding]:
    missing = metrics.get("unregistered_helper_commands", [])
    if not isinstance(missing, list) or not missing:
        return []
    return [
        Finding(
            code="helper-command-unregistered",
            severity="blocker",
            path="lib/codex-termux",
            detail="shell calls unregistered codex_termux helper commands: " + ", ".join(str(item) for item in missing),
        )
    ]


def _shell_function_definitions(root: Path) -> dict[str, str]:
    definitions: dict[str, str] = {}
    for path in _shell_classification_files(root):
        relative = _relative(root, path)
        for name in _SHELL_FUNCTION_NAME_RE.findall(_read_text(path)):
            definitions[name] = relative
    return definitions


def _audit_shell_function_classification(
    root: Path,
    manifest: dict[str, object],
) -> tuple[dict[str, object], list[Finding]]:
    findings: list[Finding] = []
    definitions = _shell_function_definitions(root)
    rules = manifest.get("shell_function_classification_rules", [])
    metrics: dict[str, object] = {
        "shell_classification_function_count": len(definitions),
        "shell_classification_class_counts": {},
    }
    if not isinstance(rules, list) or not rules:
        return metrics, [
            Finding(
                code="shell-function-classification-missing",
                severity="blocker",
                path=MANIFEST_PATH,
                detail="shell_function_classification_rules must classify protected shell functions",
            )
        ]

    compiled: list[tuple[int, str, list[re.Pattern[str]], str, list[str]]] = []
    for index, rule in enumerate(rules):
        if not isinstance(rule, dict):
            findings.append(Finding("shell-function-classification-rule", "blocker", MANIFEST_PATH, f"rule {index} is not an object"))
            continue
        class_name = rule.get("class", "")
        patterns = rule.get("patterns", [])
        reason = rule.get("reason", "")
        tests = rule.get("tests", [])
        if class_name not in SHELL_FUNCTION_CLASSES:
            findings.append(
                Finding(
                    "shell-function-classification-class",
                    "blocker",
                    MANIFEST_PATH,
                    f"rule {index} has unknown class: {class_name}",
                )
            )
            continue
        if not isinstance(reason, str) or not reason:
            findings.append(Finding("shell-function-classification-reason", "blocker", MANIFEST_PATH, f"rule {index} lacks a reason"))
        if not isinstance(tests, list) or not all(isinstance(item, str) and item for item in tests):
            findings.append(Finding("shell-function-classification-tests", "blocker", MANIFEST_PATH, f"rule {index} lacks test coverage references"))
        if not isinstance(patterns, list) or not patterns:
            findings.append(Finding("shell-function-classification-patterns", "blocker", MANIFEST_PATH, f"rule {index} lacks patterns"))
            continue
        compiled_patterns: list[re.Pattern[str]] = []
        for pattern in patterns:
            if not isinstance(pattern, str) or not pattern:
                findings.append(Finding("shell-function-classification-pattern", "blocker", MANIFEST_PATH, f"rule {index} has an empty pattern"))
                continue
            try:
                compiled_patterns.append(re.compile(pattern))
            except re.error as exc:
                findings.append(Finding("shell-function-classification-pattern", "blocker", MANIFEST_PATH, f"rule {index} invalid pattern {pattern!r}: {exc}"))
        compiled.append((index, str(class_name), compiled_patterns, str(reason), [str(item) for item in tests if isinstance(item, str)]))

    matches_by_function: dict[str, list[tuple[int, str]]] = {}
    used_rules: set[int] = set()
    class_counts = {class_name: 0 for class_name in sorted(SHELL_FUNCTION_CLASSES)}
    for function, relative in definitions.items():
        matches: list[tuple[int, str]] = []
        for index, class_name, patterns, _reason, _tests in compiled:
            if any(pattern.search(function) for pattern in patterns):
                matches.append((index, class_name))
        matches_by_function[function] = matches
        if len(matches) != 1:
            detail = "no matching classification rule" if not matches else "multiple classification rules: " + ", ".join(str(index) for index, _class in matches)
            findings.append(
                Finding(
                    "shell-function-classification",
                    "blocker",
                    relative,
                    f"{function}: {detail}",
                )
            )
            continue
        index, class_name = matches[0]
        used_rules.add(index)
        class_counts[class_name] += 1

    for index, class_name, _patterns, _reason, _tests in compiled:
        if index not in used_rules:
            findings.append(
                Finding(
                    "shell-function-classification-unused",
                    "blocker",
                    MANIFEST_PATH,
                    f"rule {index} ({class_name}) matches no shell functions",
                )
            )
    metrics["shell_classification_class_counts"] = class_counts
    metrics["shell_classification_unclassified"] = sorted(
        name for name, matches in matches_by_function.items() if not matches
    )
    metrics["shell_classification_ambiguous"] = sorted(
        name for name, matches in matches_by_function.items() if len(matches) > 1
    )
    return metrics, findings


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
    expected = {"dispatch", "state", "prompt", "profile", "use", "remove", "session", "runtime", "notify", "doctor"}
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

def _line_count(path: Path) -> int:
    return len(_read_text(path).splitlines()) if path.is_file() else 0


def _shell_function_count(path: Path) -> int:
    return len(_SHELL_FUNCTION_RE.findall(_read_text(path))) if path.is_file() else 0


def _cli_registered_commands(root: Path) -> list[str]:
    cli_dir = root / "tools/codex_termux"
    commands: set[str] = set()
    for path in [cli_dir / "cli.py", *sorted(cli_dir.glob("cli_*.py"))]:
        commands.update(_CLI_ADD_PARSER_RE.findall(_read_text(path)))
    return sorted(commands)


def _shell_helper_commands(root: Path) -> list[str]:
    commands: set[str] = set()
    for path in [root / "bin/install-runtime.sh", *_shell_contract_files(root)]:
        commands.update(_SHELL_HELPER_COMMAND_RE.findall(_read_text(path)))
    return sorted(commands)


def _target_budget_gaps(manifest: dict[str, object], metrics: dict[str, object]) -> dict[str, dict[str, int]]:
    targets = manifest.get("target_shell_budgets_95", {})
    if not isinstance(targets, dict):
        return {}
    gaps: dict[str, dict[str, int]] = {}
    for key, limit in targets.items():
        value = metrics.get(str(key))
        if isinstance(limit, int) and isinstance(value, int) and value > limit:
            gaps[str(key)] = {
                "value": value,
                "target": limit,
                "gap": value - limit,
            }
    return gaps


def _metrics(root: Path) -> dict[str, object]:
    lib = root / "lib/codex-termux.sh"
    domain_dir = root / DOMAIN_DIR
    install_runtime = root / "bin/install-runtime.sh"
    session_py = root / "tools/codex_termux/session.py"
    cli_py = root / "tools/codex_termux/cli.py"
    metrics: dict[str, object] = {}
    for key, path in (("lib_lines", lib), ("install_runtime_lines", install_runtime), ("session_py_lines", session_py)):
        metrics[key] = _line_count(path)
    shell = _shell_contract_text(root)
    metrics["lib_shell_functions"] = len(_SHELL_FUNCTION_RE.findall(shell))
    metrics["domain_files"] = len(list(domain_dir.glob("*.sh"))) if domain_dir.is_dir() else 0
    metrics["domain_shell_lines"] = sum(len(_read_text(path).splitlines()) for path in sorted(domain_dir.glob("*.sh"))) if domain_dir.is_dir() else 0
    metrics["notify_shell_functions"] = len(re.findall(r"^codex_notify[A-Za-z0-9_]*\(\) \{", shell, re.M))
    metrics["profile_shell_functions"] = len(re.findall(r"^codex_profile[A-Za-z0-9_]*\(\) \{", shell, re.M))
    shell_file_lines = {
        _relative(root, path): _line_count(path)
        for path in [lib, *sorted(domain_dir.glob("*.sh"))]
        if path.is_file()
    }
    shell_file_functions = {
        _relative(root, path): _shell_function_count(path)
        for path in [lib, *sorted(domain_dir.glob("*.sh"))]
        if path.is_file()
    }
    metrics["shell_file_lines"] = shell_file_lines
    metrics["shell_file_functions"] = shell_file_functions
    metrics["runtime_shell_lines"] = shell_file_lines.get("lib/codex-termux/runtime.sh", 0)
    metrics["state_shell_lines"] = shell_file_lines.get("lib/codex-termux/state.sh", 0)
    metrics["prompt_shell_lines"] = shell_file_lines.get("lib/codex-termux/prompt.sh", 0)
    metrics["notify_shell_lines"] = shell_file_lines.get("lib/codex-termux/notify.sh", 0)
    metrics["profile_shell_lines"] = shell_file_lines.get("lib/codex-termux/profile.sh", 0)
    metrics["cli_py_lines"] = _line_count(cli_py)
    registered = _cli_registered_commands(root)
    helper_commands = _shell_helper_commands(root)
    metrics["cli_registered_commands"] = registered
    metrics["cli_registered_command_count"] = len(registered)
    metrics["shell_helper_commands"] = helper_commands
    metrics["shell_helper_command_count"] = len(helper_commands)
    metrics["unregistered_helper_commands"] = sorted(set(helper_commands) - set(registered))
    return metrics


def _blocking_findings(findings: Iterable[Finding]) -> list[Finding]:
    return [finding for finding in findings if finding.severity == "blocker"]


def _phase_status(findings: Iterable[Finding]) -> dict[str, str]:
    active = {finding.severity for finding in findings}
    return {
        phase: "needs-canon" if severity in active else "ok"
        for phase, severity in PHASE_SEVERITIES.items()
    }
