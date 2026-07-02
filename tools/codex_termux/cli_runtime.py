"""Runtime command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
from typing import Protocol

from . import runtime_env


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    env_plan = sub.add_parser("runtime-env-plan")
    for name in (
        "runtime-dir", "runtime-exe", "tmpdir", "cert-file", "cert-dir",
        "prefix", "path", "browser", "ssl-cert-file", "ssl-cert-dir", "bwrap-quiet",
    ):
        env_plan.add_argument(f"--{name}", required=True)
    for name in ("home", "xdg-config-home", "xdg-cache-home", "xdg-data-home", "godebug"):
        env_plan.add_argument(f"--{name}", default="")
    env_plan.add_argument("--set-home", choices=("0", "1"), default="0")
    env_plan.add_argument("--termux-open-url", choices=("0", "1"), default="0")
    env_plan.set_defaults(func=_runtime_env_plan)


def _runtime_env_plan(args: argparse.Namespace) -> int:
    print(runtime_env.shell_exports(
        runtime_dir=args.runtime_dir,
        runtime_exe=args.runtime_exe,
        set_home=args.set_home == "1",
        home=args.home,
        tmpdir=args.tmpdir,
        cert_file=args.cert_file,
        cert_dir=args.cert_dir,
        prefix=args.prefix,
        path=args.path,
        browser=args.browser,
        ssl_cert_file=args.ssl_cert_file,
        ssl_cert_dir=args.ssl_cert_dir,
        xdg_config_home=args.xdg_config_home,
        xdg_cache_home=args.xdg_cache_home,
        xdg_data_home=args.xdg_data_home,
        godebug=args.godebug,
        bwrap_quiet=args.bwrap_quiet,
        termux_open_url_available=args.termux_open_url == "1",
    ))
    return 0
