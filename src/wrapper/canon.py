"""Role-oriented canonicalization audit.

The mature audit engine is retained in ``_legacy_canon`` while this module
supplies layout discovery and compatibility rules. This keeps policy behavior
stable while paths move from language-oriented directories to role-oriented
layers.
"""

from __future__ import annotations

import copy
from pathlib import Path

from . import _legacy_canon as _legacy


Finding = _legacy.Finding
WRAPPER_TOP_LEVEL_COMMANDS = _legacy.WRAPPER_TOP_LEVEL_COMMANDS
REMOVED_CONTRACT_TERMS = _legacy.REMOVED_CONTRACT_TERMS


def _shell_contract_files(root: Path) -> list[Path]:
    files = [root / "lib/codex-termux.sh"]
    shell_dir = root / "shell"
    if shell_dir.is_dir():
        files.extend(sorted(path for path in shell_dir.glob("*.sh") if path.name != "loader.sh"))
        files.insert(1, shell_dir / "loader.sh")
    else:
        legacy_dir = root / "lib/codex-termux"
        files.extend(sorted(legacy_dir.glob("*.sh")))
    return [path for path in files if path.is_file()]


def _shell_classification_files(root: Path) -> list[Path]:
    return [
        path
        for path in [root / "bin/install-runtime.sh", *_shell_contract_files(root)]
        if path.is_file()
    ]


def _layout_manifest(root: Path, manifest: dict[str, object]) -> dict[str, object]:
    data = copy.deepcopy(manifest)
    domains = data.get("domain_ownership")
    if isinstance(domains, dict) and (root / "shell").is_dir():
        for name, entry in domains.items():
            if isinstance(entry, dict) and (root / "shell" / f"{name}.sh").is_file():
                entry["path"] = f"shell/{name}.sh"
    return data


def _audit_manifest_consistency(root: Path, manifest: dict[str, object]) -> list[Finding]:
    return _ORIGINAL_MANIFEST_CONSISTENCY(root, _layout_manifest(root, manifest))


def _audit_domain_ownership(root: Path, manifest: dict[str, object]) -> list[Finding]:
    return _ORIGINAL_DOMAIN_OWNERSHIP(root, _layout_manifest(root, manifest))


def _audit_python_module_ownership(root: Path, manifest: dict[str, object]) -> list[Finding]:
    ownership = manifest.get("python_module_ownership", {})
    if not isinstance(ownership, dict):
        return [
            Finding(
                "python-module-ownership-missing",
                "blocker",
                _legacy.MANIFEST_PATH,
                "python_module_ownership must be a mapping",
            )
        ]
    findings: list[Finding] = []
    owned_paths = {
        key
        for key, value in ownership.items()
        if isinstance(key, str) and isinstance(value, str) and value
    }
    package_root = root / "src/wrapper"
    for path in sorted(package_root.rglob("*.py")):
        relative = str(path.relative_to(root))
        if relative not in owned_paths:
            findings.append(
                Finding(
                    "python-module-unowned",
                    "blocker",
                    relative,
                    "Python helper module lacks manifest ownership",
                )
            )
    for relative in sorted(owned_paths):
        if not (root / relative).is_file():
            findings.append(
                Finding(
                    "python-module-owner-path-missing",
                    "blocker",
                    relative,
                    "python_module_ownership path does not exist",
                )
            )
    return findings


def _audit_prompt_state_boundary(root: Path) -> list[Finding]:
    findings: list[Finding] = []
    prompt_state = "CODEX_PROMPT_CHOICE_RESULT"
    allowed_path = root / "shell/prompt.sh"
    if not allowed_path.is_file():
        allowed_path = root / "lib/codex-termux/prompt.sh"
    for path in _shell_contract_files(root):
        if path == allowed_path:
            continue
        if prompt_state in _legacy._read_text(path):
            findings.append(
                Finding(
                    "prompt-state-leak",
                    "blocker",
                    str(path.relative_to(root)),
                    f"{prompt_state} must stay private to {allowed_path.relative_to(root)}; use codex_prompt_result",
                )
            )
    return findings


def _audit_profile_shell_model(root: Path) -> list[Finding]:
    profile_shell = root / "shell/profile.sh"
    if not profile_shell.is_file():
        profile_shell = root / "lib/codex-termux/profile.sh"
    if not profile_shell.is_file():
        return []
    text = _legacy._read_text(profile_shell)
    markers = [marker for marker in _legacy.PROFILE_SHELL_MODEL_MARKERS if marker in text]
    if not markers:
        return []
    return [
        Finding(
            "profile-shell-model",
            "phase-c004",
            str(profile_shell.relative_to(root)),
            "profile model still lives in shell: " + ", ".join(markers),
        )
    ]


def _audit_protected_path_contracts(root: Path, manifest: dict[str, object]) -> list[Finding]:
    install_runtime = _legacy._read_text(root / "bin/install-runtime.sh")
    shell = "\n".join(_legacy._read_text(path) for path in _shell_contract_files(root))
    required = (
        "prepare_support_install",
        "rollback_support_install",
        "commit_support_install",
        "codex_assert_managed_tree_target",
        "codex_rm_rf_managed",
        "codex_try_verified_rollback",
    )
    return [
        Finding(
            "protected-path-marker-missing",
            "blocker",
            "bin/install-runtime.sh",
            f"missing protected path marker: {marker}",
        )
        for marker in required
        if marker not in install_runtime and marker not in shell
    ]


def _audit_public_entrypoints(root: Path, manifest: dict[str, object]) -> list[Finding]:
    dispatch = root / "shell/dispatch.sh"
    if not dispatch.is_file():
        dispatch = root / "lib/codex-termux/dispatch.sh"
    checks = (
        (root / "lib/codex-termux.sh", "shell/loader.sh"),
        (dispatch, "codex_main()"),
        (dispatch, "codex_termux_main()"),
        (root / "bin/install-runtime.sh", 'codex_termux_doctor "$@"'),
        (root / "bin/install-runtime.sh", "install upstream [VERSION]"),
        (root / "bin/install-runtime.sh", "install rebuild"),
    )
    findings: list[Finding] = []
    for path, marker in checks:
        if marker not in _legacy._read_text(path):
            findings.append(
                Finding(
                    "entrypoint-compat-marker-missing",
                    "blocker",
                    str(path.relative_to(root)),
                    f"missing marker: {marker}",
                )
            )
    return findings


def _cli_registered_commands(root: Path) -> list[str]:
    cli_dir = root / "src/wrapper"
    if not (cli_dir / "cli.py").is_file():
        cli_dir = root / "tools/codex_termux"
    commands: set[str] = set()
    for path in [cli_dir / "cli.py", *sorted(cli_dir.glob("cli_*.py"))]:
        commands.update(_legacy._CLI_ADD_PARSER_RE.findall(_legacy._read_text(path)))
    return sorted(commands)


def _metrics(root: Path) -> dict[str, object]:
    metrics = _ORIGINAL_METRICS(root)
    shell_lines = metrics.get("shell_file_lines", {})
    shell_functions = metrics.get("shell_file_functions", {})
    if isinstance(shell_lines, dict):
        for domain in ("build", "ui", "fs", "runtime", "state", "prompt", "notify", "profile"):
            metrics[f"{domain}_shell_lines"] = int(shell_lines.get(f"shell/{domain}.sh", 0))
    if isinstance(shell_functions, dict):
        metrics["notify_shell_functions"] = int(shell_functions.get("shell/notify.sh", 0))
        metrics["profile_shell_functions"] = int(shell_functions.get("shell/profile.sh", 0))
    metrics["cli_py_lines"] = _legacy._line_count(root / "src/wrapper/cli.py")
    metrics["session_py_lines"] = _legacy._line_count(root / "src/wrapper/session.py")
    return metrics


_ORIGINAL_MANIFEST_CONSISTENCY = _legacy._audit_manifest_consistency
_ORIGINAL_DOMAIN_OWNERSHIP = _legacy._audit_domain_ownership
_ORIGINAL_METRICS = _legacy._metrics
_legacy.DOMAIN_DIR = "shell"
_legacy._shell_contract_files = _shell_contract_files
_legacy._shell_classification_files = _shell_classification_files
_legacy._audit_manifest_consistency = _audit_manifest_consistency
_legacy._audit_domain_ownership = _audit_domain_ownership
_legacy._audit_python_module_ownership = _audit_python_module_ownership
_legacy._audit_prompt_state_boundary = _audit_prompt_state_boundary
_legacy._audit_profile_shell_model = _audit_profile_shell_model
_legacy._audit_protected_path_contracts = _audit_protected_path_contracts
_legacy._audit_public_entrypoints = _audit_public_entrypoints
_legacy._cli_registered_commands = _cli_registered_commands
_legacy._metrics = _metrics


def audit(root: Path) -> dict[str, object]:
    return _legacy.audit(root)
