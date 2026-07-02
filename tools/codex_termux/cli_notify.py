"""Notify command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
import os
import sys
from typing import Protocol

from . import notify


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    hook = sub.add_parser("notify-hook")
    hook.add_argument(
        "--action",
        choices=("all", "canonical", "valid", "normalize", "status-message", "list", "parse-selection"),
        required=True,
    )
    hook.add_argument("--value", default="")
    hook.set_defaults(func=_notify_hook)

    channel = sub.add_parser("notify-channel")
    channel.add_argument("--action", choices=("parse", "needs-gravity"), required=True)
    channel.add_argument("--value", default="")
    channel.set_defaults(func=_notify_channel)

    config = sub.add_parser("notify-config-env")
    for name in (
        "content-chars", "preserve-newlines", "toast-gravity", "toast-short",
        "toast-background", "toast-color", "group", "channel", "hooks", "pretooluse",
    ):
        config.add_argument(f"--{name}", required=True)
    config.set_defaults(func=_notify_config_env)

    system = sub.add_parser("notify-system-config")
    system.add_argument("--hooks", required=True)
    system.add_argument("--turn-notify", required=True)
    system.set_defaults(func=_notify_system_config)

    command = sub.add_parser("notify-command-config")
    command.add_argument("--field", choices=("config-file", "config-env"), required=True)
    command.add_argument("args", nargs=argparse.REMAINDER)
    command.set_defaults(func=_notify_command_config)


def _notify_hook(args: argparse.Namespace) -> int:
    if args.action == "all":
        for hook in notify.HOOKS:
            print(hook)
        return 0
    if args.action == "canonical":
        print(notify.canonical_hook(args.value))
        return 0
    if args.action == "valid":
        return 0 if notify.hook_valid(args.value) else 1
    if args.action == "normalize":
        print(notify.normalize_hooks(args.value or "Stop"))
        return 0
    if args.action == "status-message":
        print(notify.status_message(args.value))
        return 0
    if args.action == "list":
        for hook in notify.hook_list(args.value or "Stop"):
            print(hook)
        return 0
    if args.action == "parse-selection":
        try:
            print(notify.parse_hook_selection(args.value))
        except notify.NotifyConfigError as exc:
            print(f"codex_termux: {exc}", file=sys.stderr)
            return 64
        return 0
    raise AssertionError(args.action)


def _notify_channel(args: argparse.Namespace) -> int:
    try:
        if args.action == "parse":
            print(notify.parse_channel_selection(args.value))
            return 0
        if args.action == "needs-gravity":
            return 0 if notify.channel_needs_gravity(args.value) else 1
    except notify.NotifyConfigError as exc:
        print(f"codex_termux: {exc}", file=sys.stderr)
        return 64
    raise AssertionError(args.action)


def _notify_config_env(args: argparse.Namespace) -> int:
    settings = notify.NotifySettings(
        content_chars=args.content_chars,
        preserve_newlines=args.preserve_newlines,
        toast_gravity=args.toast_gravity,
        toast_short=args.toast_short,
        toast_background=args.toast_background,
        toast_color=args.toast_color,
        group=args.group,
        channel=args.channel,
        hooks=args.hooks,
        pretooluse=args.pretooluse,
    )
    try:
        print(notify.render_config_env(settings), end="")
    except notify.NotifyConfigError as exc:
        print(f"codex_termux: {exc}", file=sys.stderr)
        return 64
    return 0


def _notify_system_config(args: argparse.Namespace) -> int:
    print(notify.render_system_config(hooks=args.hooks, turn_notify=args.turn_notify), end="")
    return 0


def _notify_command_config(args: argparse.Namespace) -> int:
    try:
        config = notify.parse_command_config(args.args, os.environ)
    except notify.NotifyConfigError as exc:
        print(f"codex_termux: {exc}", file=sys.stderr)
        return 64
    if args.field == "config-file":
        print(config.config_file)
    else:
        print(notify.render_config_env(config.settings), end="")
    return 0
