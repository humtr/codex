"""UI text command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from typing import Protocol

from . import ui


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    text = sub.add_parser("ui-text")
    text.add_argument("--key", required=True)
    text.add_argument("args", nargs=argparse.REMAINDER)
    text.set_defaults(func=_ui_text)

    step_text = sub.add_parser("ui-step-text")
    step_text.add_argument("--key", required=True)
    step_text.add_argument("args", nargs=argparse.REMAINDER)
    step_text.set_defaults(func=_ui_step_text)

    status = sub.add_parser("ui-status-text")
    status.add_argument("--message", required=True)
    status.set_defaults(func=_ui_status_text)

    fmt = sub.add_parser("ui-format")
    fmt.add_argument("--kind", required=True)
    fmt.add_argument("--value", default="")
    fmt.add_argument("--color", choices=("0", "1"), default="0")
    fmt.set_defaults(func=_ui_format)


def _ui_text(args: argparse.Namespace) -> int:
    print(ui.text(args.key, *args.args))
    return 0


def _ui_step_text(args: argparse.Namespace) -> int:
    print(ui.step_text(args.key, *args.args))
    return 0


def _ui_status_text(args: argparse.Namespace) -> int:
    print(ui.status_text(args.message))
    return 0


def _ui_format(args: argparse.Namespace) -> int:
    print(ui.format_text(args.kind, args.value, color=args.color == "1"))
    return 0
