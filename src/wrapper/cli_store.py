"""Runtime store command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Protocol

from . import prune


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    prune_cmd = sub.add_parser("store-prune")
    for name in (
        "runtime-store-dir", "raw-store-dir", "registry-file", "state-file",
        "runtime-builder", "patch-policy", "retention", "current-link", "verified-link", "raw-link",
    ):
        prune_cmd.add_argument(f"--{name}", required=True)
    prune_cmd.add_argument("--protect-runtime-path", action="append", default=[])
    prune_cmd.set_defaults(func=_prune)


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
