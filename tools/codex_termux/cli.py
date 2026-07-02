"""Single internal CLI for the Codex Termux wrapper."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Protocol

from . import cli_activation, cli_artifacts, cli_doctor, cli_notify, cli_product, cli_profile, cli_session, cli_ui, prune, registry, repair, runtime_checks, use
from .errors import CodexTermuxError


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python3 -m codex_termux.cli",
        description="Internal helper interface for the Codex Termux wrapper.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    cli_product.add_commands(sub)
    cli_artifacts.add_commands(sub)
    _add_runtime_checks(sub)
    _add_store_commands(sub)
    cli_activation.add_commands(sub)
    _add_use_commands(sub)
    cli_doctor.add_commands(sub)
    cli_notify.add_commands(sub)
    cli_profile.add_commands(sub)
    cli_session.add_commands(sub)
    cli_ui.add_commands(sub)
    return parser


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
