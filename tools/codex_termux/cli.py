"""Single internal CLI for the Codex Termux wrapper."""

from __future__ import annotations

import argparse
import sys
from typing import Protocol

from . import (
    cli_activation,
    cli_artifacts,
    cli_doctor,
    cli_notify,
    cli_product,
    cli_profile,
    cli_repair,
    cli_runtime,
    cli_session,
    cli_store,
    cli_ui,
    cli_use,
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
    cli_runtime.add_commands(sub)
    cli_store.add_commands(sub)
    cli_activation.add_commands(sub)
    cli_use.add_commands(sub)
    cli_doctor.add_commands(sub)
    cli_notify.add_commands(sub)
    cli_profile.add_commands(sub)
    cli_session.add_commands(sub)
    cli_ui.add_commands(sub)
    return parser


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
