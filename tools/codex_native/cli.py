"""Single internal CLI for the Termux Codex native wrapper."""

from __future__ import annotations

import argparse
import json
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import Protocol

from . import activation, doctor, hashing, paths, prune, registry, runtime_checks, use
from .errors import CodexNativeError, IntegrityError
from .schemas import ActivationPlan


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python3 -m codex_native.cli",
        description="Internal helper interface for the Codex Termux wrapper.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("validate").set_defaults(func=lambda _args: _print_ok())
    _add_hash_and_package(sub)
    _add_runtime_checks(sub)
    _add_store_commands(sub)
    _add_activation_commands(sub)
    _add_use_commands(sub)
    _add_doctor_commands(sub)
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


def _add_runtime_checks(sub: SubparserCollection) -> None:
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


def _add_doctor_commands(sub: SubparserCollection) -> None:
    report = sub.add_parser("doctor-report")
    for name in (
        "runtime", "current-link", "verified-link", "raw-link", "manager-dir",
        "runtime-store-dir", "raw-store-dir", "raw-vendor", "resolv-conf", "cert-file",
        "state-file", "registry-file", "version", "raw-sha256", "runtime-sha256",
        "prefix", "runtime-builder", "patch-policy",
    ):
        report.add_argument(f"--{name}", required=True)
    report.set_defaults(func=_doctor_report)

    render = sub.add_parser("doctor-render")
    render.add_argument("--mode", choices=("human", "json"), default="human")
    render.set_defaults(func=_doctor_render)


def _print(value: object) -> int:
    print(value)
    return 0


def _print_ok() -> int:
    print("codex_native: ok")
    return 0


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
            "CODEX_NATIVE_HOME": args.home,
            "CODEX_NATIVE_PREFIX": args.prefix,
            "CODEX_NATIVE_NATIVE_ROOT": str(current.parent),
            "CODEX_NATIVE_MANAGER_DIR": args.manager_dir,
            "CODEX_NATIVE_RUNTIME_DIR": str(current),
            "CODEX_NATIVE_CURRENT_LINK": str(current),
            "CODEX_NATIVE_VERIFIED_LINK": args.verified_link,
            "CODEX_NATIVE_RAW_DIR": str(raw),
            "CODEX_NATIVE_RAW_VENDOR": str(raw / "vendor/aarch64-unknown-linux-musl"),
            "CODEX_NATIVE_RUNTIME": str(current / "codex"),
            "CODEX_NATIVE_STATE_DIR": str(Path(args.state_file).parent),
            "CODEX_NATIVE_STATE_FILE": args.state_file,
            "CODEX_NATIVE_REGISTRY_FILE": args.registry_file,
            "CODEX_NATIVE_STORE_DIR": str(Path(args.runtime_store_dir).parent),
            "CODEX_NATIVE_RUNTIME_STORE_DIR": args.runtime_store_dir,
            "CODEX_NATIVE_RAW_STORE_DIR": args.raw_store_dir,
            "CODEX_NATIVE_RUNTIME_BUILDER": args.runtime_builder,
            "CODEX_NATIVE_RESOLV_CONF": args.resolv_conf,
            "CODEX_NATIVE_CERT_FILE": args.cert_file,
            "CODEX_NATIVE_CERT_DIR": args.cert_dir,
            "CODEX_NATIVE_PATCH_POLICY": args.patch_policy,
        },
        **kwargs,  # type: ignore[arg-type]
    )


def _use_rows(args: argparse.Namespace) -> list[dict[str, str]]:
    return use.runtime_rows_from_registry(
        registry_file=Path(args.registry_file),
        latest=args.latest,
        runtime_store=Path(args.runtime_store_dir),
        builder=Path(args.runtime_builder),
        policy=args.patch_policy,
    )


def _use_render(args: argparse.Namespace) -> int:
    return use.render_runtime_rows(
        _use_rows(args),
        mode=args.mode,
        interactive_limit=int(args.interactive_limit),
    )


def _use_select(args: argparse.Namespace) -> int:
    row = registry.resolve_runtime_selection(_use_rows(args), args.choice)
    print(use.selection_fields(row))
    return 0


def _doctor_report(args: argparse.Namespace) -> int:
    report = doctor.build_report(
        doctor.DoctorInputs(
            runtime=Path(args.runtime),
            current_link=Path(args.current_link),
            verified_link=Path(args.verified_link),
            raw_link=Path(args.raw_link),
            manager_dir=Path(args.manager_dir),
            runtime_store=Path(args.runtime_store_dir),
            raw_store=Path(args.raw_store_dir),
            raw_vendor=Path(args.raw_vendor),
            resolv_conf=Path(args.resolv_conf),
            cert_file=Path(args.cert_file),
            state_file=Path(args.state_file),
            registry_file=Path(args.registry_file),
            version=args.version,
            raw_sha256=args.raw_sha256,
            runtime_sha256=args.runtime_sha256,
            prefix=Path(args.prefix),
            runtime_builder=Path(args.runtime_builder),
            patch_policy=args.patch_policy,
        )
    )
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


def _doctor_render(args: argparse.Namespace) -> int:
    report = json.load(sys.stdin)
    if args.mode == "json":
        print(json.dumps(report, ensure_ascii=True, sort_keys=True))
        return 0
    return doctor.render_human(report)


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except CodexNativeError as exc:
        print(f"codex_native: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
