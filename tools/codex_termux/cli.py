"""Single internal CLI for the Codex Termux wrapper."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Protocol

from . import (
    cli_activation,
    cli_artifacts,
    cli_doctor,
    cli_notify,
    cli_product,
    cli_profile,
    cli_repair,
    cli_session,
    cli_ui,
    prune,
    registry,
    use,
)
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
    cli_repair.add_commands(sub)
    _add_store_commands(sub)
    cli_activation.add_commands(sub)
    _add_use_commands(sub)
    cli_doctor.add_commands(sub)
    cli_notify.add_commands(sub)
    cli_profile.add_commands(sub)
    cli_session.add_commands(sub)
    cli_ui.add_commands(sub)
    return parser


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

    select_env = sub.add_parser("use-select-env")
    _add_use_common(select_env)
    select_env.add_argument("--choice", required=True)
    select_env.set_defaults(func=_use_select_env)


def _add_use_common(parser: argparse.ArgumentParser) -> None:
    for name in ("registry-file", "latest", "runtime-store-dir", "runtime-builder", "patch-policy"):
        parser.add_argument(f"--{name}", required=True)


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


def _use_select_env(args: argparse.Namespace) -> int:
    row = registry.resolve_runtime_selection(
        registry_file=Path(args.registry_file),
        choice=args.choice,
        latest=args.latest,
        runtime_store_dir=Path(args.runtime_store_dir),
        runtime_builder=Path(args.runtime_builder),
        patch_policy=args.patch_policy,
    )
    print(use.selection_plan_exports(row))
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
