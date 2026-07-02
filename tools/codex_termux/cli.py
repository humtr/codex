"""Single internal CLI for the Codex Termux wrapper."""

from __future__ import annotations

import argparse
import json
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import Protocol

from . import activation, canon, cli_doctor, cli_notify, cli_profile, cli_session, hashing, install_plan, paths, prune, registry, release, repair, runtime_checks, source, use
from .errors import CodexTermuxError, IntegrityError
from .schemas import ActivationPlan


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python3 -m codex_termux.cli",
        description="Internal helper interface for the Codex Termux wrapper.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate")
    validate.add_argument("--root", default=None)
    validate.set_defaults(func=_validate_wrapper)

    wrapper_source_missing = sub.add_parser("wrapper-source-missing")
    wrapper_source_missing.add_argument("--root", required=True)
    wrapper_source_missing.set_defaults(func=_wrapper_source_missing)

    validate_wrapper_source = sub.add_parser("validate-wrapper-source")
    validate_wrapper_source.add_argument("--root", required=True)
    validate_wrapper_source.set_defaults(func=_validate_wrapper_source)

    wrapper_source_plan = sub.add_parser("wrapper-source-plan")
    for name in ("repo", "ref", "release-url", "release-repo", "release-tag", "local-root"):
        wrapper_source_plan.add_argument(f"--{name}", default="")
    wrapper_source_plan.add_argument("--field", choices=("kind", "git-url", "release-url", "label", "local-root"), default=None)
    wrapper_source_plan.set_defaults(func=_wrapper_source_plan)

    install_plan_cmd = sub.add_parser("install-plan")
    install_plan_cmd.add_argument("--command", required=True)
    install_plan_cmd.add_argument("--field", choices=("action", "surface", "version", "exit-code", "error"), default=None)
    install_plan_cmd.add_argument("args", nargs=argparse.REMAINDER)
    install_plan_cmd.set_defaults(func=_install_plan)

    canon_audit = sub.add_parser("canon-audit")
    canon_audit.add_argument("--root", default=None)
    canon_audit.add_argument("--strict", action="store_true")
    canon_audit.set_defaults(func=_canon_audit)
    _add_hash_and_package(sub)
    _add_runtime_checks(sub)
    _add_store_commands(sub)
    _add_activation_commands(sub)
    _add_use_commands(sub)
    cli_doctor.add_commands(sub)
    cli_notify.add_commands(sub)
    cli_profile.add_commands(sub)
    cli_session.add_commands(sub)
    return parser


def _add_hash_and_package(sub: SubparserCollection) -> None:
    hash_file = sub.add_parser("hash-file")
    hash_file.add_argument("--path", required=True)
    hash_file.set_defaults(func=lambda args: _print(hashing.sha256_file(Path(args.path))))

    tree_digest = sub.add_parser("tree-digest")
    tree_digest.add_argument("--path", required=True)
    tree_digest.set_defaults(func=lambda args: _print(hashing.tree_digest(Path(args.path))))

    store_id = sub.add_parser("store-id")
    for name in ("version", "sha256", "builder-sha256", "bwrap-sha256", "rg-sha256", "tree-sha256"):
        store_id.add_argument(f"--{name}", required=True)
    store_id.set_defaults(func=_store_id)

    resolve = sub.add_parser("resolve-path")
    resolve.add_argument("--path", required=True)
    resolve.set_defaults(func=lambda args: _print(paths.resolve_text(Path(args.path))))

    validate_tarball = sub.add_parser("validate-tarball")
    validate_tarball.add_argument("--path", required=True)
    validate_tarball.set_defaults(func=_validate_tarball)

    release_package = sub.add_parser("release-package")
    release_package.add_argument("--package-root", required=True)
    release_package.add_argument("--out", required=True)
    release_package.set_defaults(func=_release_package)

    package_field = sub.add_parser("package-field")
    package_field.add_argument("--json-file", required=True)
    package_field.add_argument("--field", required=True)
    package_field.set_defaults(
        func=lambda args: _print(runtime_checks.extract_pack_field(Path(args.json_file), args.field))
    )

    state_field = sub.add_parser("state-read-field")
    state_field.add_argument("--state-file", required=True)
    state_field.add_argument("--field", required=True)
    state_field.set_defaults(
        func=lambda args: _print(runtime_checks.state_field(Path(args.state_file), args.field))
    )

    runtime_date = sub.add_parser("registry-active-runtime-date")
    runtime_date.add_argument("--registry-file", required=True)
    runtime_date.set_defaults(
        func=lambda args: _print(registry.active_runtime_created_at(Path(args.registry_file)))
    )

    upstream_release_date = sub.add_parser("upstream-release-date")
    upstream_release_date.add_argument("--version", required=True)
    upstream_release_date.set_defaults(
        func=lambda args: _print(runtime_checks.upstream_release_date(sys.stdin.read(), args.version))
    )


def _add_runtime_checks(sub: SubparserCollection) -> None:
    repair_diagnose = sub.add_parser("repair-diagnose")
    for name in (
        "managed-shell", "manager-dir", "public-codex", "marker",
        "runtime-dir", "runtime", "support-dir", "manifest-path", "builder",
        "state-path", "registry-path", "current", "verified", "raw",
        "raw-binary", "patch-policy", "wrapper-version", "wrapper-commit",
    ):
        repair_diagnose.add_argument(f"--{name}", required=True)
    repair_diagnose.add_argument("--field", choices=("action", "readiness-action"), default=None)
    repair_diagnose.set_defaults(func=_repair_diagnose)

    runtime_layout = sub.add_parser("runtime-layout-ok")
    for name in ("runtime-dir", "runtime", "support-dir"):
        runtime_layout.add_argument(f"--{name}", required=True)
    runtime_layout.set_defaults(func=_runtime_layout_ok)

    support_layer = sub.add_parser("support-layer-ok")
    for name in ("managed-shell", "manager-dir", "public-codex", "marker"):
        support_layer.add_argument(f"--{name}", required=True)
    support_layer.set_defaults(func=_support_layer_ok)

    runtime = sub.add_parser("runtime-integrity")
    for name in ("runtime", "manifest-path", "builder", "state-path", "patch-policy"):
        runtime.add_argument(f"--{name}", required=True)
    runtime.set_defaults(func=_runtime_integrity)

    raw = sub.add_parser("raw-integrity")
    raw.add_argument("--raw-binary", required=True)
    raw.add_argument("--state-path", required=True)
    raw.set_defaults(func=_raw_integrity)

    metadata = sub.add_parser("runtime-metadata-current")
    for name in (
        "state-path", "registry-path", "current", "verified", "raw",
        "wrapper-version", "wrapper-commit",
    ):
        metadata.add_argument(f"--{name}", required=True)
    metadata.set_defaults(func=_runtime_metadata_current)


def _add_store_commands(sub: SubparserCollection) -> None:
    prune_cmd = sub.add_parser("store-prune")
    for name in (
        "runtime-store-dir", "raw-store-dir", "registry-file", "state-file",
        "runtime-builder", "patch-policy", "retention", "current-link", "verified-link", "raw-link",
    ):
        prune_cmd.add_argument(f"--{name}", required=True)
    prune_cmd.add_argument("--protect-runtime-path", action="append", default=[])
    prune_cmd.set_defaults(func=_prune)


def _add_activation_commands(sub: SubparserCollection) -> None:
    commit = sub.add_parser("activation-commit")
    _add_activation_common(commit)
    for name in (
        "candidate-runtime", "candidate-raw", "runtime-target", "raw-target",
        "version", "raw-sha256", "runtime-sha256", "package-spec",
    ):
        commit.add_argument(f"--{name}", required=True)
    commit.add_argument("--cleanup-runtime-source", action="store_true")
    commit.add_argument("--cleanup-raw-source", action="store_true")
    commit.set_defaults(func=_activation_commit)

    restore = sub.add_parser("activation-restore-verified")
    _add_activation_common(restore)
    restore.set_defaults(func=_activation_restore_verified)


def _add_activation_common(parser: argparse.ArgumentParser) -> None:
    for name in (
        "current-link", "verified-link", "raw-link", "state-file", "registry-file",
        "runtime-store-dir", "raw-store-dir", "wrapper-version", "wrapper-commit", "updated-at",
        "shell-bin", "shell-lib", "home", "prefix", "manager-dir", "runtime-builder",
        "resolv-conf", "cert-file", "cert-dir", "patch-policy",
    ):
        parser.add_argument(f"--{name}", required=True)


def _add_use_commands(sub: SubparserCollection) -> None:
    render = sub.add_parser("use-render")
    _add_use_common(render)
    render.add_argument("--mode", required=True)
    render.add_argument("--interactive-limit", required=True)
    render.set_defaults(func=_use_render)

    select = sub.add_parser("use-select")
    _add_use_common(select)
    select.add_argument("--choice", required=True)
    select.set_defaults(func=_use_select)


def _add_use_common(parser: argparse.ArgumentParser) -> None:
    for name in ("registry-file", "latest", "runtime-store-dir", "runtime-builder", "patch-policy"):
        parser.add_argument(f"--{name}", required=True)


def _print(value: object) -> int:
    print(value)
    return 0


def _print_ok() -> int:
    print("codex_termux: ok")
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


def _install_plan(args: argparse.Namespace) -> int:
    result = install_plan.plan(args.command, list(args.args)).to_dict()
    if args.field:
        key = args.field.replace("-", "_")
        print(result[key])
    else:
        print(json.dumps(result, ensure_ascii=True, sort_keys=True))
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
        'if ! codex_profile_default_p "$profile"; then',
        '        export CODEX_HOME="$profile_dir"',
        '    fi',
    ))
    required_shell_facade = (
        "codex_profile_name_valid()",
        "codex_profile_home_dir()",
        "codex_profile_recent_read()",
        "codex_profile_recent_write()",
        "codex_profile_menu_items()",
    )
    if profile_root not in shell:
        raise IntegrityError("custom profile root contract changed")
    for marker in required_python_model:
        if marker not in session_py:
            raise IntegrityError(f"profile model owner contract changed: {marker}")
    for marker in required_shell_facade:
        if marker not in shell:
            raise IntegrityError(f"profile shell facade contract changed: {marker}")
    if guarded_export not in shell:
        raise IntegrityError("custom profile CODEX_HOME guard changed")
    if 'if [ ! -t 0 ] || [ ! -t 2 ]; then' not in shell:
        raise IntegrityError("missing custom profile non-tty guard changed")
    if 'codex_fail "$(codex_ui_text_get missing_profile "$display")"' not in shell:
        raise IntegrityError("missing custom profile refusal message changed")

def _store_id(args: argparse.Namespace) -> int:
    return _print(
        paths.store_id(
            args.version,
            args.sha256,
            args.builder_sha256,
            args.bwrap_sha256,
            args.rg_sha256,
            args.tree_sha256,
        )
    )


def _validate_tarball(args: argparse.Namespace) -> int:
    with tarfile.open(args.path, "r:gz") as handle:
        for member in handle.getmembers():
            path = PurePosixPath(member.name)
            if not member.name or member.name.startswith("/") or path.is_absolute():
                raise IntegrityError(f"unsafe tar entry path: {member.name}")
            if ".." in path.parts or member.issym() or member.islnk():
                raise IntegrityError(f"unsafe tar entry: {member.name}")
            if member.ischr() or member.isblk() or member.isfifo() or member.isdev():
                raise IntegrityError(f"unsafe tar special entry: {member.name}")
    print("ok")
    return 0


def _release_package(args: argparse.Namespace) -> int:
    out = Path(args.out)
    release.write_zip(Path(args.package_root), out)
    print(out)
    return 0


def _runtime_integrity(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_integrity_ok(
        runtime=Path(args.runtime),
        manifest_path=Path(args.manifest_path),
        builder=Path(args.builder),
        state_path=Path(args.state_path),
        patch_policy=args.patch_policy,
    ) else 1


def _raw_integrity(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.raw_integrity_ok(
        raw_binary=Path(args.raw_binary), state_path=Path(args.state_path)
    ) else 1


def _runtime_layout_ok(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_layout_ok(
        runtime_dir=Path(args.runtime_dir),
        runtime=Path(args.runtime),
        support_dir=Path(args.support_dir),
    ) else 1


def _support_layer_ok(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.support_layer_ok(
        managed_shell=Path(args.managed_shell),
        manager_dir=Path(args.manager_dir),
        public_codex=Path(args.public_codex),
        marker=args.marker,
    ) else 1


def _runtime_metadata_current(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_metadata_current(
        state_path=Path(args.state_path),
        registry_path=Path(args.registry_path),
        current=Path(args.current),
        verified=Path(args.verified),
        raw=Path(args.raw),
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
    ) else 1


def _repair_diagnose(args: argparse.Namespace) -> int:
    diagnosis = repair.diagnose(
        repair.RepairInputs(
            managed_shell=Path(args.managed_shell),
            manager_dir=Path(args.manager_dir),
            public_codex=Path(args.public_codex),
            marker=args.marker,
            runtime_dir=Path(args.runtime_dir),
            runtime=Path(args.runtime),
            support_dir=Path(args.support_dir),
            manifest_path=Path(args.manifest_path),
            builder=Path(args.builder),
            state_path=Path(args.state_path),
            registry_path=Path(args.registry_path),
            current=Path(args.current),
            verified=Path(args.verified),
            raw=Path(args.raw),
            raw_binary=Path(args.raw_binary),
            patch_policy=args.patch_policy,
            wrapper_version=args.wrapper_version,
            wrapper_commit=args.wrapper_commit,
        )
    )
    if args.field == "action":
        print(diagnosis.action)
    elif args.field == "readiness-action":
        print(diagnosis.readiness_action)
    else:
        print(json.dumps(diagnosis.to_dict(), ensure_ascii=True, sort_keys=True))
    return 0


def _prune(args: argparse.Namespace) -> int:
    result = prune.build_and_apply_prune(
        runtime_store=Path(args.runtime_store_dir),
        raw_store=Path(args.raw_store_dir),
        registry_file=Path(args.registry_file),
        state_file=Path(args.state_file),
        builder=Path(args.runtime_builder),
        policy=args.patch_policy,
        retention=int(args.retention),
        current_link=Path(args.current_link),
        verified_link=Path(args.verified_link),
        protected_runtime_paths=[Path(item) for item in args.protect_runtime_path],
        raw_link=Path(args.raw_link),
    )
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0 if result.get("status") == "ok" else 1


def _activation_commit(args: argparse.Namespace) -> int:
    result = activation.commit(
        _activation_plan(
            args,
            candidate_runtime=Path(args.candidate_runtime),
            candidate_raw=Path(args.candidate_raw),
            runtime_target=Path(args.runtime_target),
            raw_target=Path(args.raw_target),
            version=args.version,
            raw_sha256=args.raw_sha256,
            runtime_sha256=args.runtime_sha256,
            package_spec=args.package_spec,
            cleanup_runtime_source=args.cleanup_runtime_source,
            cleanup_raw_source=args.cleanup_raw_source,
        )
    )
    print(result.tuple_id)
    return 0


def _activation_restore_verified(args: argparse.Namespace) -> int:
    result = activation.restore_verified(
        _activation_plan(
            args,
            candidate_runtime=Path(args.current_link),
            candidate_raw=Path(args.raw_link),
            runtime_target=Path(args.runtime_store_dir),
            raw_target=Path(args.raw_store_dir),
            version="verified",
            raw_sha256="verified",
            runtime_sha256="verified",
            package_spec="verified",
            cleanup_runtime_source=False,
            cleanup_raw_source=False,
        )
    )
    print(result.tuple_id)
    return 0


def _activation_plan(args: argparse.Namespace, **kwargs: object) -> ActivationPlan:
    current = Path(args.current_link)
    raw = Path(args.raw_link)
    return ActivationPlan(
        current_link=current,
        verified_link=Path(args.verified_link),
        raw_link=raw,
        state_file=Path(args.state_file),
        registry_file=Path(args.registry_file),
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
        updated_at=args.updated_at,
        shell_bin=Path(args.shell_bin),
        shell_lib=Path(args.shell_lib),
        probe_env={
            "HOME": args.home,
            "PREFIX": args.prefix,
            "CODEX_TERMUX_HOME": args.home,
            "CODEX_TERMUX_PREFIX": args.prefix,
            "CODEX_TERMUX_ROOT": str(current.parent),
            "CODEX_TERMUX_MANAGER_DIR": args.manager_dir,
            "CODEX_TERMUX_RUNTIME_DIR": str(current),
            "CODEX_TERMUX_CURRENT_LINK": str(current),
            "CODEX_TERMUX_VERIFIED_LINK": args.verified_link,
            "CODEX_TERMUX_RAW_DIR": str(raw),
            "CODEX_TERMUX_RAW_VENDOR": str(raw / "vendor/aarch64-unknown-linux-musl"),
            "CODEX_TERMUX_RUNTIME": str(current / "codex"),
            "CODEX_TERMUX_STATE_DIR": str(Path(args.state_file).parent),
            "CODEX_TERMUX_STATE_FILE": args.state_file,
            "CODEX_TERMUX_REGISTRY_FILE": args.registry_file,
            "CODEX_TERMUX_STORE_DIR": str(Path(args.runtime_store_dir).parent),
            "CODEX_TERMUX_RUNTIME_STORE_DIR": args.runtime_store_dir,
            "CODEX_TERMUX_RAW_STORE_DIR": args.raw_store_dir,
            "CODEX_TERMUX_RUNTIME_BUILDER": args.runtime_builder,
            "CODEX_TERMUX_RESOLV_CONF": args.resolv_conf,
            "CODEX_TERMUX_CERT_FILE": args.cert_file,
            "CODEX_TERMUX_CERT_DIR": args.cert_dir,
            "CODEX_TERMUX_PATCH_POLICY": args.patch_policy,
        },
        **kwargs,  # type: ignore[arg-type]
    )


def _use_rows(args: argparse.Namespace) -> list[dict[str, str]]:
    return use.runtime_rows_from_registry(
        registry_file=Path(args.registry_file),
        latest=args.latest,
        runtime_store_dir=Path(args.runtime_store_dir),
        runtime_builder=Path(args.runtime_builder),
        patch_policy=args.patch_policy,
    )


def _use_render(args: argparse.Namespace) -> int:
    return use.render_runtime_rows(
        _use_rows(args),
        mode=args.mode,
        interactive_limit=int(args.interactive_limit),
    )


def _use_select(args: argparse.Namespace) -> int:
    row = registry.resolve_runtime_selection(
        registry_file=Path(args.registry_file),
        choice=args.choice,
        latest=args.latest,
        runtime_store_dir=Path(args.runtime_store_dir),
        runtime_builder=Path(args.runtime_builder),
        patch_policy=args.patch_policy,
    )
    print(use.selection_fields(row))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except CodexTermuxError as exc:
        print(f"codex_termux: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
