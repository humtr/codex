"""Runtime use command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Protocol

from . import registry, use


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    command_plan = sub.add_parser("use-command-plan-env")
    command_plan.add_argument("--arg", action="append", default=[])
    command_plan.set_defaults(func=_use_command_plan_env)

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


def _use_command_plan_env(args: argparse.Namespace) -> int:
    print(use.command_plan_exports(args.arg))
    return 0


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
