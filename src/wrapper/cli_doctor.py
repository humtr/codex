"""Doctor command group for the internal Codex Termux CLI."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Protocol

from . import doctor, support_diagnostics


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    report = sub.add_parser("doctor-report")
    for name in (
        "runtime", "current-link", "verified-link", "raw-link", "manager-dir",
        "runtime-store-dir", "raw-store-dir", "raw-vendor", "resolv-conf", "cert-file",
        "state-file", "registry-file", "version", "raw-sha256", "runtime-sha256",
        "prefix", "runtime-builder", "patch-policy",
    ):
        report.add_argument(f"--{name}", required=True)
    report.set_defaults(func=_doctor_report)

    render = sub.add_parser("doctor-render")
    render.add_argument("--mode", choices=("human", "json"), default="human")
    render.set_defaults(func=_doctor_render)


def _doctor_report(args: argparse.Namespace) -> int:
    manager_dir = Path(args.manager_dir)
    report = doctor.build_report(
        doctor.DoctorInputs(
            runtime=Path(args.runtime),
            current_link=Path(args.current_link),
            verified_link=Path(args.verified_link),
            raw_link=Path(args.raw_link),
            manager_dir=manager_dir,
            runtime_store=Path(args.runtime_store_dir),
            raw_store=Path(args.raw_store_dir),
            raw_vendor=Path(args.raw_vendor),
            resolv_conf=Path(args.resolv_conf),
            cert_file=Path(args.cert_file),
            state_file=Path(args.state_file),
            registry_file=Path(args.registry_file),
            version=args.version,
            raw_sha256=args.raw_sha256,
            runtime_sha256=args.runtime_sha256,
            prefix=Path(args.prefix),
            runtime_builder=Path(args.runtime_builder),
            patch_policy=args.patch_policy,
        )
    )
    support_diagnostics.augment_report(report, manager_dir)
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


def _doctor_render(args: argparse.Namespace) -> int:
    report = json.load(sys.stdin)
    if args.mode == "json":
        print(json.dumps(report, ensure_ascii=True, sort_keys=True))
        return 0 if report.get("overallStatus") == "ok" else 1
    doctor.render_human(report)
    support_diagnostics.render_human(report, sys.stdout)
    return 0 if report.get("overallStatus") == "ok" else 1
