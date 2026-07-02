"""Profile command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from typing import Protocol

from . import session


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    validate = sub.add_parser("profile-validate")
    validate.add_argument("--profile", default="")
    validate.set_defaults(func=_profile_validate)

    profile_dir_cmd = sub.add_parser("profile-dir")
    profile_dir_cmd.add_argument("--profile", default="default")
    profile_dir_cmd.set_defaults(func=lambda args: _print(str(session.profile_dir(args.profile))))

    display = sub.add_parser("profile-display-name")
    display.add_argument("--profile", default="default")
    display.set_defaults(func=lambda args: _print(session.profile_display_name(args.profile)))

    is_default = sub.add_parser("profile-is-default")
    is_default.add_argument("--profile", default="default")
    is_default.set_defaults(func=lambda args: 0 if session.is_default_profile(args.profile) else 1)

    choice = sub.add_parser("profile-choice-to-name")
    choice.add_argument("--choice", default="")
    choice.set_defaults(func=lambda args: _print(session.normalize_profile_choice(args.choice)))

    select_choice = sub.add_parser("profile-menu-choice")
    select_choice.add_argument("--choice", default="")
    select_choice.set_defaults(func=lambda args: _print(session.resolve_profile_menu_choice(args.choice)))

    create_confirm = sub.add_parser("profile-create-confirmed")
    create_confirm.add_argument("--choice", default="")
    create_confirm.set_defaults(func=lambda args: 0 if session.profile_create_confirmed(args.choice) else 1)

    write_recent = sub.add_parser("profile-write-recent")
    write_recent.add_argument("--profile", default="default")
    write_recent.set_defaults(func=_profile_write_recent)

    read_recent = sub.add_parser("profile-read-recent")
    read_recent.set_defaults(func=lambda args: _print(session.read_recent_profile()))

    list_cmd = sub.add_parser("profile-list")
    list_cmd.add_argument("--include-default", action="store_true")
    list_cmd.set_defaults(func=_profile_list)

    menu = sub.add_parser("profile-menu-ids")
    menu.set_defaults(func=_profile_menu_ids)

    menu_render = sub.add_parser("profile-menu-render")
    menu_render.add_argument("--interactive", choices=("0", "1"), default="0")
    menu_render.set_defaults(func=_profile_menu_render)


def _print(value: object) -> int:
    print(value)
    return 0


def _profile_validate(args: argparse.Namespace) -> int:
    return 0 if session.validate_profile_name(args.profile) else 1


def _profile_write_recent(args: argparse.Namespace) -> int:
    session.write_recent_profile(args.profile)
    return 0


def _profile_list(args: argparse.Namespace) -> int:
    if args.include_default:
        print("default")
    for profile in session.list_profiles():
        print(profile)
    return 0


def _profile_menu_ids(args: argparse.Namespace) -> int:
    for profile in session.profile_menu_ids():
        print(profile)
    return 0


def _profile_menu_render(args: argparse.Namespace) -> int:
    print(session.render_profile_menu(interactive=args.interactive == "1"))
    return 0
