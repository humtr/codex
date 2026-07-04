"""Product validation and source-plan command group for the internal CLI."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Protocol

from . import canon, install_plan, source
from .errors import IntegrityError


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    termux_help = sub.add_parser("termux-help")
    termux_help.set_defaults(func=_termux_help)

    termux_version_help = sub.add_parser("termux-version-help")
    termux_version_help.set_defaults(func=_termux_version_help)

    validate = sub.add_parser("validate")
    validate.add_argument("--root", default=None)
    validate.set_defaults(func=_validate_wrapper)

    wrapper_source_missing = sub.add_parser("wrapper-source-missing")
    wrapper_source_missing.add_argument("--root", required=True)
    wrapper_source_missing.set_defaults(func=_wrapper_source_missing)

    validate_wrapper_source = sub.add_parser("validate-wrapper-source")
    validate_wrapper_source.add_argument("--root", required=True)
    validate_wrapper_source.set_defaults(func=_validate_wrapper_source)

    wrapper_source_commit = sub.add_parser("wrapper-source-commit")
    wrapper_source_commit.add_argument("--root", required=True)
    wrapper_source_commit.set_defaults(func=_wrapper_source_commit)

    wrapper_source_root = sub.add_parser("wrapper-source-root")
    wrapper_source_root.add_argument("--extract-root", required=True)
    wrapper_source_root.set_defaults(func=_wrapper_source_root)

    wrapper_source_env = sub.add_parser("wrapper-source-env")
    for name in ("repo", "ref", "token", "git-repo", "git-ref", "git-token", "release-token"):
        wrapper_source_env.add_argument(f"--{name}", default="")
    wrapper_source_env.set_defaults(func=_wrapper_source_env)

    wrapper_auth = sub.add_parser("wrapper-auth-token")
    for name in ("token", "git-token", "release-token", "github-token"):
        wrapper_auth.add_argument(f"--{name}", default="")
    wrapper_auth.add_argument("--allow-gh", choices=("0", "1"), default="0")
    wrapper_auth.set_defaults(func=_wrapper_auth_token)

    wrapper_source_plan = sub.add_parser("wrapper-source-plan")
    for name in ("repo", "ref", "release-url", "release-repo", "release-tag", "local-root"):
        wrapper_source_plan.add_argument(f"--{name}", default="")
    wrapper_source_plan.add_argument(
        "--field", choices=("kind", "git-url", "release-url", "label", "local-root"), default=None
    )
    wrapper_source_plan.set_defaults(func=_wrapper_source_plan)

    wrapper_source_plan_env = sub.add_parser("wrapper-source-plan-env")
    for name in ("repo", "ref", "release-url", "release-repo", "release-tag", "local-root"):
        wrapper_source_plan_env.add_argument(f"--{name}", default="")
    wrapper_source_plan_env.set_defaults(func=_wrapper_source_plan_env)

    install_usage = sub.add_parser("install-usage")
    install_usage.set_defaults(func=_install_usage)

    install_plan_cmd = sub.add_parser("install-plan")
    install_plan_cmd.add_argument("--command", required=True)
    install_plan_cmd.add_argument(
        "--field",
        choices=("action", "surface", "version", "exit-code", "error", "surface-message", "success-message"),
        default=None,
    )
    install_plan_cmd.add_argument("args", nargs=argparse.REMAINDER)
    install_plan_cmd.set_defaults(func=_install_plan)

    install_plan_env = sub.add_parser("install-plan-env")
    install_plan_env.add_argument("--command", required=True)
    install_plan_env.add_argument("args", nargs=argparse.REMAINDER)
    install_plan_env.set_defaults(func=_install_plan_env)

    canon_audit = sub.add_parser("canon-audit")
    canon_audit.add_argument("--root", default=None)
    canon_audit.add_argument("--strict", action="store_true")
    canon_audit.set_defaults(func=_canon_audit)


def _print_ok() -> int:
    print("codex_termux: ok")
    return 0


def _termux_help(args: argparse.Namespace) -> int:
    print(_TERMUX_HELP_TEXT, end="")
    return 0


def _termux_version_help(args: argparse.Namespace) -> int:
    print(_TERMUX_VERSION_HELP_TEXT, end="")
    return 0


def _install_usage(args: argparse.Namespace) -> int:
    print(_INSTALL_USAGE_TEXT, end="")
    return 0


def _validate_wrapper(args: argparse.Namespace) -> int:
    root = _validate_root(Path(args.root) if args.root else None)
    _validate_required_layout(root)
    _validate_no_removed_contracts(root)
    _validate_wrapper_version(root)
    _validate_resolver_contract(root)
    _validate_profile_contract(root)
    return _print_ok()


def _canon_audit(args: argparse.Namespace) -> int:
    root = _validate_root(Path(args.root) if args.root else None)
    report = canon.audit(root)
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    if args.strict and report.get("status") != "ok":
        return 1
    return 0


def _validate_root(requested: Path | None) -> Path:
    candidates: list[Path] = []
    if requested is not None:
        candidates.append(requested)
    candidates.append(Path.cwd())
    try:
        candidates.append(Path(__file__).resolve().parents[2])
    except IndexError:
        pass
    for candidate in candidates:
        root = candidate.resolve()
        if (
            (root / "bin/install-runtime.sh").is_file()
            and (root / "lib/codex-termux.sh").is_file()
            and (root / "tools/codex_termux/cli.py").is_file()
            and (root / "config/wrapper-version.env").is_file()
        ):
            return root
    raise IntegrityError("wrapper source root not found")


def _validate_required_layout(root: Path) -> None:
    for relative in source.missing_wrapper_source_paths(root):
        raise IntegrityError(f"required wrapper path is missing: {relative}")


def _wrapper_source_missing(args: argparse.Namespace) -> int:
    for relative in source.missing_wrapper_source_paths(Path(args.root)):
        print(relative)
    return 0


def _validate_wrapper_source(args: argparse.Namespace) -> int:
    return 0 if source.is_wrapper_source(Path(args.root)) else 1


def _wrapper_source_commit(args: argparse.Namespace) -> int:
    print(source.source_commit(Path(args.root)))
    return 0


def _wrapper_source_root(args: argparse.Namespace) -> int:
    print(source.find_extracted_wrapper_source(Path(args.extract_root)))
    return 0


def _wrapper_source_env(args: argparse.Namespace) -> int:
    values = {
        source.env_key("REPO"): args.repo,
        source.env_key("REF"): args.ref,
        source.env_key("TOKEN"): args.token,
        source.env_key("GIT_REPO"): args.git_repo,
        source.env_key("GIT_REF"): args.git_ref,
        source.env_key("GIT_TOKEN"): args.git_token,
        source.env_key("RELEASE_TOKEN"): args.release_token,
    }
    text = source.source_env_exports(values)
    if text:
        print(text)
    return 0


def _wrapper_auth_token(args: argparse.Namespace) -> int:
    token = source.auth_token(
        {
            source.env_key("TOKEN"): args.token,
            source.env_key("GIT_TOKEN"): args.git_token,
            source.env_key("RELEASE_TOKEN"): args.release_token,
            "GITHUB_TOKEN": args.github_token,
        },
        allow_gh=args.allow_gh == "1",
    )
    if not token:
        return 1
    print(token)
    return 0


def _install_plan(args: argparse.Namespace) -> int:
    result = install_plan.plan(args.command, list(args.args)).to_dict()
    if args.field:
        key = args.field.replace("-", "_")
        print(result[key])
    else:
        print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


def _install_plan_env(args: argparse.Namespace) -> int:
    print(install_plan.plan_shell_exports(args.command, list(args.args)))
    return 0


def _wrapper_source_plan(args: argparse.Namespace) -> int:
    result = source.wrapper_source_plan(
        repo=args.repo,
        ref=args.ref,
        release_url=args.release_url,
        release_repo=args.release_repo,
        release_tag=args.release_tag,
        local_root=args.local_root,
    ).to_dict()
    if args.field:
        print(result[args.field.replace("-", "_")])
    else:
        print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


def _wrapper_source_plan_env(args: argparse.Namespace) -> int:
    plan = source.wrapper_source_plan(
        repo=args.repo,
        ref=args.ref,
        release_url=args.release_url,
        release_repo=args.release_repo,
        release_tag=args.release_tag,
        local_root=args.local_root,
    )
    print(source.wrapper_source_plan_exports(plan))
    return 0


_TERMUX_HELP_TEXT = """Codex Termux wrapper commands

Usage:
  codex termux <command> [args...]

Commands:
  help                         Print this wrapper command help.
  install [VERSION]            Install support files and a fresh patched runtime.
  install support              Refresh support files and the public launcher only.
  install upstream [VERSION]   Install selected/latest upstream package with current support.
  install rebuild              Rebuild a patched runtime from the cached raw package.
  update [VERSION]             Refresh support files and runtime; same as install.
  repair                       Diagnose and repair the managed installation.
  doctor [--json]              Check launcher, runtime resources, resolver, CA, DNS patch, and state.
  version                      Print upstream, runtime, and wrapper version/date rows.
  use [--list|SELECTION]       List or promote cached/remote runtimes.
  profile [list|NAME]          List profiles or launch with an explicit CODEX_HOME profile.
  session [PROFILE] [--all]    Pick and resume discovered Codex sessions across profiles.
  notify [options]             Configure notification/toast hooks.
  remove                       Remove managed launcher/runtime and restore launcher backups.

Top-level codex arguments are reserved for upstream Codex. Use "codex termux help" for wrapper operations.
"""

_TERMUX_VERSION_HELP_TEXT = """Usage: codex termux version

Prints upstream Codex and managed wrapper/runtime version rows.
"""

_INSTALL_USAGE_TEXT = """Usage: bash bin/install-runtime.sh [install|update|repair|remove|doctor] [ARGS]

install [VERSION]           Install support files, launcher, and a fresh patched runtime.
install support             Refresh support files and the launcher only.
install upstream [VERSION]  Install a fresh patched runtime from upstream raw.
install rebuild             Refresh support files and rebuild patched runtime from cached raw.
update [VERSION]            Same as install [VERSION]: refresh support and patched runtime.
repair                      Diagnose and repair the managed installation; does not update by default.
remove                      Remove the managed launcher/runtime and restore a launcher backup.
doctor                      Run wrapper diagnostics. Use: doctor --json for machine output.
"""


def _removed_contract_terms() -> tuple[str, ...]:
    return (
        "".join(("codex", "_native")),
        "".join(("CODEX", "_NATIVE")),
        "".join(("codex/", "native")),
        "".join(("codex", " ", "native")),
        "".join(("native", ".lock")),
        "".join(("CODEX_TERMUX", "_RESOLVER_FD")),
        "".join(("CODEX_TERMUX", "_SHARED_PLUGINS_DIR")),
        "".join(("codex_profile", "_share_plugins")),
    )


def _source_files(root: Path) -> list[Path]:
    ignored_dirs = {".git", "__pycache__", ".pytest_cache", ".mypy_cache"}
    ignored_suffixes = {".pyc", ".zip", ".tgz", ".tar", ".gz"}
    files: list[Path] = []
    for path in root.rglob("*"):
        if any(part in ignored_dirs for part in path.parts):
            continue
        if not path.is_file():
            continue
        if path.suffix in ignored_suffixes:
            continue
        files.append(path)
    return files


def _validate_no_removed_contracts(root: Path) -> None:
    terms = _removed_contract_terms()
    for path in _source_files(root):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for term in terms:
            if term in text:
                relative = path.relative_to(root)
                raise IntegrityError(f"removed wrapper contract remains in {relative}")


def _parse_env_file(path: Path) -> dict[str, str]:
    data: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            raise IntegrityError(f"invalid env metadata line: {path.name}")
        key, value = line.split("=", 1)
        data[key] = value
    return data


def _validate_wrapper_version(root: Path) -> None:
    data = _parse_env_file(root / "config/wrapper-version.env")
    required = (
        "CODEX_TERMUX_WRAPPER_VERSION",
        "CODEX_TERMUX_WRAPPER_CHANNEL",
        "CODEX_TERMUX_WRAPPER_REPO",
    )
    for key in required:
        if not data.get(key):
            raise IntegrityError(f"wrapper metadata is missing: {key}")
    if "/" not in data["CODEX_TERMUX_WRAPPER_REPO"]:
        raise IntegrityError("wrapper repository metadata must be OWNER/REPO")


def _shell_contract_text(root: Path) -> str:
    parts = []
    for path in [root / "lib/codex-termux.sh", *sorted((root / "lib/codex-termux").glob("*.sh"))]:
        if path.is_file():
            parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def _validate_resolver_contract(root: Path) -> None:
    shell = _shell_contract_text(root)
    builder = (root / "tools/build-runtime.py").read_text(encoding="utf-8")
    shell_policy = 'CODEX_TERMUX_PATCH_POLICY="${CODEX_TERMUX_PATCH_POLICY:-termux-fd-remap-v1}"'
    builder_policy = 'PATCH_POLICY = "termux-fd-remap-v1"'
    resolver_target = 'b"/etc/resolv.conf": b"/proc/self/fd/33"'
    system_config_target = 'b"/etc/codex/config.toml": b"/dev/fd/34/config.toml"'
    runtime_fd = '33<"$CODEX_TERMUX_RESOLV_CONF"'
    system_config_fd = '34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"'
    if shell_policy not in shell:
        raise IntegrityError("shell patch policy contract changed")
    if builder_policy not in builder:
        raise IntegrityError("builder patch policy contract changed")
    if resolver_target not in builder:
        raise IntegrityError("builder resolver target contract changed")
    if system_config_target not in builder:
        raise IntegrityError("builder system config target contract changed")
    if runtime_fd not in shell:
        raise IntegrityError("runtime launcher fd33 contract changed")
    if system_config_fd not in shell:
        raise IntegrityError("runtime launcher fd34 contract changed")


def _validate_profile_contract(root: Path) -> None:
    shell = _shell_contract_text(root)
    session_py = (root / "tools/codex_termux/session.py").read_text(encoding="utf-8")
    profile_root = 'CODEX_TERMUX_PROFILE_ROOT="${CODEX_TERMUX_PROFILE_ROOT:-$CODEX_TERMUX_HOME/.codex-profiles}"'
    required_python_model = (
        "def validate_profile_name(",
        "def profile_dir(",
        'return get_codex_termux_home() / ".codex"',
        "def write_recent_profile(",
        "def read_recent_profile(",
        "def profile_menu_ids(",
    )
    guarded_export = "\n".join((
        'if ! codex_termux_cmd profile-is-default --profile "$profile"; then',
        '        export CODEX_HOME="$profile_dir"',
        '    fi',
    ))
    required_shell_calls = (
        'codex_termux_cmd profile-validate --profile "$profile"',
        'codex_termux_cmd profile-dir --profile "$profile"',
        'codex_termux_cmd profile-dir --profile "$recent_profile"',
        "codex_termux_cmd prompt-choice-action",
    )
    if profile_root not in shell:
        raise IntegrityError("custom profile root contract changed")
    for marker in required_python_model:
        if marker not in session_py:
            raise IntegrityError(f"profile model owner contract changed: {marker}")
    for marker in required_shell_calls:
        if marker not in shell:
            raise IntegrityError(f"profile shell command contract changed: {marker}")
    if guarded_export not in shell:
        raise IntegrityError("custom profile CODEX_HOME guard changed")
    if 'if [ ! -t 0 ] || [ ! -t 2 ]; then' not in shell:
        raise IntegrityError("missing custom profile non-tty guard changed")
    if 'codex_fail "$(codex_ui_text_get missing_profile "$display")"' not in shell:
        raise IntegrityError("missing custom profile refusal message changed")
