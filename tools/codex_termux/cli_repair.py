"""Runtime diagnosis and repair command group for the internal CLI."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Protocol

from . import repair, runtime_checks


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    repair_diagnose = sub.add_parser("repair-diagnose")
    for name in (
        "managed-shell", "manager-dir", "public-codex", "marker",
        "runtime-dir", "runtime", "support-dir", "manifest-path", "builder",
        "state-path", "registry-path", "current", "verified", "raw",
        "raw-binary", "patch-policy", "wrapper-version", "wrapper-commit",
    ):
        repair_diagnose.add_argument(f"--{name}", required=True)
    repair_diagnose.add_argument("--field", choices=("action", "readiness-action"), default=None)
    repair_diagnose.set_defaults(func=_repair_diagnose)

    action_plan = sub.add_parser("runtime-action-plan")
    action_plan.add_argument("--action", required=True)
    action_plan.add_argument("--intent", choices=("readiness", "repair"), required=True)
    action_plan.add_argument(
        "--field",
        choices=("kind", "step", "refresh-after", "error", "exit-code"),
        required=True,
    )
    action_plan.set_defaults(func=_runtime_action_plan)

    runtime_layout = sub.add_parser("runtime-layout-ok")
    for name in ("runtime-dir", "runtime", "support-dir"):
        runtime_layout.add_argument(f"--{name}", required=True)
    runtime_layout.set_defaults(func=_runtime_layout_ok)

    support_layer = sub.add_parser("support-layer-ok")
    for name in ("managed-shell", "manager-dir", "public-codex", "marker"):
        support_layer.add_argument(f"--{name}", required=True)
    support_layer.set_defaults(func=_support_layer_ok)

    runtime = sub.add_parser("runtime-integrity")
    for name in ("runtime", "manifest-path", "builder", "state-path", "patch-policy"):
        runtime.add_argument(f"--{name}", required=True)
    runtime.set_defaults(func=_runtime_integrity)

    raw = sub.add_parser("raw-integrity")
    raw.add_argument("--raw-binary", required=True)
    raw.add_argument("--state-path", required=True)
    raw.set_defaults(func=_raw_integrity)

    metadata = sub.add_parser("runtime-metadata-current")
    for name in (
        "state-path", "registry-path", "current", "verified", "raw",
        "wrapper-version", "wrapper-commit",
    ):
        metadata.add_argument(f"--{name}", required=True)
    metadata.set_defaults(func=_runtime_metadata_current)


def _runtime_integrity(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_integrity_ok(
        runtime=Path(args.runtime),
        manifest_path=Path(args.manifest_path),
        builder=Path(args.builder),
        state_path=Path(args.state_path),
        patch_policy=args.patch_policy,
    ) else 1


def _raw_integrity(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.raw_integrity_ok(
        raw_binary=Path(args.raw_binary), state_path=Path(args.state_path)
    ) else 1


def _runtime_layout_ok(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_layout_ok(
        runtime_dir=Path(args.runtime_dir),
        runtime=Path(args.runtime),
        support_dir=Path(args.support_dir),
    ) else 1


def _support_layer_ok(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.support_layer_ok(
        managed_shell=Path(args.managed_shell),
        manager_dir=Path(args.manager_dir),
        public_codex=Path(args.public_codex),
        marker=args.marker,
    ) else 1


def _runtime_metadata_current(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_metadata_current(
        state_path=Path(args.state_path),
        registry_path=Path(args.registry_path),
        current=Path(args.current),
        verified=Path(args.verified),
        raw=Path(args.raw),
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
    ) else 1


def _repair_diagnose(args: argparse.Namespace) -> int:
    diagnosis = repair.diagnose(
        repair.RepairInputs(
            managed_shell=Path(args.managed_shell),
            manager_dir=Path(args.manager_dir),
            public_codex=Path(args.public_codex),
            marker=args.marker,
            runtime_dir=Path(args.runtime_dir),
            runtime=Path(args.runtime),
            support_dir=Path(args.support_dir),
            manifest_path=Path(args.manifest_path),
            builder=Path(args.builder),
            state_path=Path(args.state_path),
            registry_path=Path(args.registry_path),
            current=Path(args.current),
            verified=Path(args.verified),
            raw=Path(args.raw),
            raw_binary=Path(args.raw_binary),
            patch_policy=args.patch_policy,
            wrapper_version=args.wrapper_version,
            wrapper_commit=args.wrapper_commit,
        )
    )
    if args.field == "action":
        print(diagnosis.action)
    elif args.field == "readiness-action":
        print(diagnosis.readiness_action)
    else:
        print(json.dumps(diagnosis.to_dict(), ensure_ascii=True, sort_keys=True))
    return 0


def _runtime_action_plan(args: argparse.Namespace) -> int:
    print(repair.runtime_action_plan(args.action, args.intent).field(args.field))
    return 0
