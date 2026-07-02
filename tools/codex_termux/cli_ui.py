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


def _ui_text(args: argparse.Namespace) -> int:
    print(ui.text(args.key, *args.args))
    return 0


def _ui_step_text(args: argparse.Namespace) -> int:
    print(ui.step_text(args.key, *args.args))
    return 0
