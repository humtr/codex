"""Session command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from typing import Protocol

from . import session


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    list_cmd = sub.add_parser("session-list")
    list_cmd.set_defaults(func=_session_list)

    select_cmd = sub.add_parser("session-select")
    select_cmd.add_argument("--choice", required=True)
    select_cmd.add_argument("--target-profile", required=True)
    select_cmd.set_defaults(func=_session_select)

    plan_env = sub.add_parser("session-plan-env")
    plan_env.add_argument("--plan", required=True)
    plan_env.set_defaults(func=_session_plan_env)

    share_cmd = sub.add_parser("session-share")
    share_cmd.add_argument("--source-path", required=True)
    share_cmd.add_argument("--source-profile", required=True)
    share_cmd.add_argument("--target-profile", required=True)
    share_cmd.set_defaults(func=_session_share)

    boundary_cmd = sub.add_parser("session-boundary-check")
    boundary_cmd.add_argument("--source-profile", required=True)
    boundary_cmd.add_argument("--target-profile", required=True)
    boundary_cmd.set_defaults(func=_session_boundary_check)

    tui_cmd = sub.add_parser("session-tui")
    tui_cmd.add_argument("--output", required=True)
    tui_cmd.add_argument("--all", action="store_true")
    tui_cmd.set_defaults(func=lambda args: session.session_tui_command(args.output, args.all))


def _session_list(args: argparse.Namespace) -> int:
    rows = session.discover_sessions()
    for r in rows[:20]:
        disp_time = session.format_datetime(r.updated_at)
        fields = [
            r.native_session_ref,
            r.source_profile,
            disp_time,
            r.workdir,
            r.title,
        ]
        print("\x1f".join(fields))
    return 0


def _session_select(args: argparse.Namespace) -> int:
    session.session_select(args.choice, args.target_profile)
    return 0


def _session_plan_env(args: argparse.Namespace) -> int:
    print(session.session_plan_exports(args.plan))
    return 0


def _session_share(args: argparse.Namespace) -> int:
    try:
        session.share_session(args.source_path, args.source_profile, args.target_profile)
    except session.SessionBoundaryError as exc:
        raise SystemExit(f"ERROR: {exc}") from exc
    return 0


def _session_boundary_check(args: argparse.Namespace) -> int:
    try:
        session.require_session_boundary(args.source_profile, args.target_profile)
    except session.SessionBoundaryError as exc:
        raise SystemExit(f"ERROR: {exc}") from exc
    return 0
