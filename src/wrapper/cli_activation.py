"""Runtime activation command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Protocol

from . import activation
from .schemas import ActivationPlan


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
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
