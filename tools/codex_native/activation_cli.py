"""Internal CLI adapter for activation transactions."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Protocol

from . import activation
from .schemas import ActivationPlan


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(subparsers: SubparserCollection) -> None:
    commit = subparsers.add_parser("activation-commit")
    _add_common_args(commit)
    for name in (
        "candidate-runtime",
        "candidate-raw",
        "runtime-target",
        "raw-target",
        "version",
        "raw-sha256",
        "runtime-sha256",
        "package-spec",
    ):
        commit.add_argument(f"--{name}", required=True)
    commit.add_argument("--cleanup-runtime-source", action="store_true")
    commit.add_argument("--cleanup-raw-source", action="store_true")
    commit.set_defaults(func=_commit)

    restore = subparsers.add_parser("activation-restore-verified")
    _add_common_args(restore)
    restore.set_defaults(func=_restore_verified)


def _add_common_args(parser: argparse.ArgumentParser) -> None:
    for name in (
        "current-link",
        "verified-link",
        "raw-link",
        "state-file",
        "registry-file",
        "runtime-store-dir",
        "raw-store-dir",
        "wrapper-version",
        "wrapper-commit",
        "updated-at",
        "shell-bin",
        "shell-lib",
        "home",
        "prefix",
        "manager-dir",
        "runtime-builder",
        "resolv-conf",
        "cert-file",
        "cert-dir",
        "patch-policy",
    ):
        parser.add_argument(f"--{name}", required=True)


def _commit(args: argparse.Namespace) -> int:
    result = activation.commit(
        _plan(
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


def _restore_verified(args: argparse.Namespace) -> int:
    result = activation.restore_verified(
        _plan(
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


def _plan(
    args: argparse.Namespace,
    *,
    candidate_runtime: Path,
    candidate_raw: Path,
    runtime_target: Path,
    raw_target: Path,
    version: str,
    raw_sha256: str,
    runtime_sha256: str,
    package_spec: str,
    cleanup_runtime_source: bool,
    cleanup_raw_source: bool,
) -> ActivationPlan:
    return ActivationPlan(
        candidate_runtime=candidate_runtime,
        candidate_raw=candidate_raw,
        runtime_target=runtime_target,
        raw_target=raw_target,
        current_link=Path(args.current_link),
        verified_link=Path(args.verified_link),
        raw_link=Path(args.raw_link),
        state_file=Path(args.state_file),
        registry_file=Path(args.registry_file),
        version=version,
        raw_sha256=raw_sha256,
        runtime_sha256=runtime_sha256,
        package_spec=package_spec,
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
        updated_at=args.updated_at,
        shell_bin=Path(args.shell_bin),
        shell_lib=Path(args.shell_lib),
        probe_env=_probe_env(args),
        cleanup_runtime_source=cleanup_runtime_source,
        cleanup_raw_source=cleanup_raw_source,
    )


def _probe_env(args: argparse.Namespace) -> dict[str, str]:
    current = Path(args.current_link)
    raw = Path(args.raw_link)
    return {
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
    }
