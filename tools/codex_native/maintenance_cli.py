"""Internal CLI commands for wrapper maintenance domains."""

from __future__ import annotations

import argparse
import json
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import Protocol

from . import (
    doctor_render,
    doctor_report,
    migration,
    paths,
    prune,
    registry,
    runtime_checks,
    use,
)
from .errors import IntegrityError


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(subparsers: SubparserCollection) -> None:
    _add_prune(subparsers)
    _add_migration(subparsers)
    _add_use(subparsers)
    _add_doctor(subparsers)
    _add_runtime_checks(subparsers)
    _add_misc(subparsers)


def _add_prune(subparsers: SubparserCollection) -> None:
    parser = subparsers.add_parser("store-prune")
    for name in (
        "runtime-store-dir",
        "raw-store-dir",
        "registry-file",
        "state-file",
        "runtime-builder",
        "patch-policy",
        "retention",
        "current-link",
        "verified-link",
        "raw-link",
    ):
        parser.add_argument(f"--{name}", required=True)
    parser.set_defaults(func=_prune)


def _add_migration(subparsers: SubparserCollection) -> None:
    parser = subparsers.add_parser("legacy-store-migrate")
    for name in (
        "legacy-store-dir",
        "runtime-store-dir",
        "raw-store-dir",
        "registry-file",
        "runtime-builder",
        "manager-dir",
        "patch-policy",
        "report-file",
        "completed-at",
    ):
        parser.add_argument(f"--{name}", required=True)
    parser.set_defaults(func=_migrate)


def _add_use(subparsers: SubparserCollection) -> None:
    render = subparsers.add_parser("use-render")
    _add_use_common(render)
    render.add_argument("--mode", required=True)
    render.add_argument("--interactive-limit", required=True)
    render.set_defaults(func=_use_render)

    select = subparsers.add_parser("use-select")
    _add_use_common(select)
    select.add_argument("--choice", required=True)
    select.set_defaults(func=_use_select)


def _add_use_common(parser: argparse.ArgumentParser) -> None:
    for name in (
        "registry-file",
        "latest",
        "runtime-store-dir",
        "runtime-builder",
        "patch-policy",
    ):
        parser.add_argument(f"--{name}", required=True)


def _add_misc(subparsers: SubparserCollection) -> None:
    store_id = subparsers.add_parser("store-id")
    for name in ("version", "sha256", "builder-sha256", "bwrap-sha256", "rg-sha256"):
        store_id.add_argument(f"--{name}", required=True)
    store_id.add_argument("--tree-sha256", default="")
    store_id.set_defaults(func=_store_id)

    resolve = subparsers.add_parser("resolve-path")
    resolve.add_argument("--path", required=True)
    resolve.set_defaults(func=_resolve_path)

    tarball = subparsers.add_parser("validate-tarball")
    tarball.add_argument("--path", required=True)
    tarball.set_defaults(func=_validate_tarball)


def _add_doctor(subparsers: SubparserCollection) -> None:
    report = subparsers.add_parser("doctor-report")
    for name in (
        "runtime", "current-link", "verified-link", "raw-link", "manager-dir",
        "runtime-store-dir", "raw-store-dir", "raw-vendor", "resolv-conf",
        "cert-file", "state-file", "registry-file", "migration-report-file",
        "legacy-store-dir", "version", "raw-sha256", "runtime-sha256", "prefix",
        "runtime-builder", "patch-policy", "network-json",
    ):
        report.add_argument(f"--{name}", required=True)
    report.set_defaults(func=_doctor_report)
    render = subparsers.add_parser("doctor-render")
    render.add_argument("--mode", choices=("human", "json"), default="human")
    render.set_defaults(func=_doctor_render)
    probe = subparsers.add_parser("doctor-socket-probe")
    probe.set_defaults(func=_doctor_socket_probe)
    network = subparsers.add_parser("doctor-network-boundary")
    for name in ("baseline-json", "off-json", "on-json", "reset-json"):
        network.add_argument(f"--{name}", required=True)
    for name in ("baseline-exit", "off-exit", "on-exit", "reset-exit"):
        network.add_argument(f"--{name}", required=True)
    network.set_defaults(func=_doctor_network_boundary)


def _add_runtime_checks(subparsers: SubparserCollection) -> None:
    package_field = subparsers.add_parser("package-field")
    package_field.add_argument("--json-file", required=True)
    package_field.add_argument("--field", required=True)
    package_field.set_defaults(func=_package_field)
    runtime_integrity = subparsers.add_parser("runtime-integrity")
    for name in ("runtime", "manifest-path", "builder", "state-path", "patch-policy"):
        runtime_integrity.add_argument(f"--{name}", required=True)
    runtime_integrity.set_defaults(func=_runtime_integrity)
    raw_integrity = subparsers.add_parser("raw-integrity")
    for name in ("raw-binary", "state-path"):
        raw_integrity.add_argument(f"--{name}", required=True)
    raw_integrity.set_defaults(func=_raw_integrity)
    metadata = subparsers.add_parser("runtime-metadata-current")
    for name in (
        "state-path", "registry-path", "current", "verified", "raw",
        "wrapper-version", "wrapper-commit",
    ):
        metadata.add_argument(f"--{name}", required=True)
    metadata.set_defaults(func=_runtime_metadata_current)
    commands = subparsers.add_parser("parse-upstream-commands")
    commands.set_defaults(func=_parse_upstream_commands)
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
        raw_link=Path(args.raw_link),
    )
    print(json.dumps(result, ensure_ascii=True, sort_keys=True))
    return 0


def _migrate(args: argparse.Namespace) -> int:
    report = migration.migrate_legacy_store_cache(
        legacy_store=Path(args.legacy_store_dir),
        runtime_store=Path(args.runtime_store_dir),
        raw_store=Path(args.raw_store_dir),
        registry_file=Path(args.registry_file),
        runtime_builder=Path(args.runtime_builder),
        manager_dir=Path(args.manager_dir),
        patch_policy=args.patch_policy,
        report_file=Path(args.report_file),
        completed_at=args.completed_at,
    )
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


def _use_rows(args: argparse.Namespace) -> list[dict[str, str]]:
    return use.runtime_rows_from_registry(
        registry_file=Path(args.registry_file),
        latest=args.latest,
        runtime_store_dir=Path(args.runtime_store_dir),
        runtime_builder=Path(args.runtime_builder),
        patch_policy=args.patch_policy,
    )


def _use_render(args: argparse.Namespace) -> int:
    return use.render_runtime_rows(
        _use_rows(args),
        mode=args.mode,
        interactive_limit=int(args.interactive_limit),
    )


def _use_select(args: argparse.Namespace) -> int:
    row = registry.resolve_runtime_selection(
        registry_file=Path(args.registry_file),
        choice=args.choice,
        latest=args.latest,
        runtime_store_dir=Path(args.runtime_store_dir),
        runtime_builder=Path(args.runtime_builder),
        patch_policy=args.patch_policy,
    )
    print(use.selection_fields(row))
    return 0


def _store_id(args: argparse.Namespace) -> int:
    print(
        paths.store_id(
            args.version,
            args.sha256,
            args.builder_sha256,
            args.bwrap_sha256,
            args.rg_sha256,
            args.tree_sha256,
        )
    )
    return 0


def _resolve_path(args: argparse.Namespace) -> int:
    print(paths.resolve_text(Path(args.path)))
    return 0


def _validate_tarball(args: argparse.Namespace) -> int:
    with tarfile.open(args.path, "r:gz") as handle:
        for member in handle.getmembers():
            path = PurePosixPath(member.name)
            if not member.name or member.name.startswith("/") or path.is_absolute():
                raise IntegrityError(f"unsafe tar entry path: {member.name}")
            if ".." in path.parts or member.issym() or member.islnk():
                raise IntegrityError(f"unsafe tar entry: {member.name}")
            if member.ischr() or member.isblk() or member.isfifo() or member.isdev():
                raise IntegrityError(f"unsafe tar special entry: {member.name}")
    print("ok")
    return 0


def _doctor_report(args: argparse.Namespace) -> int:
    report = doctor_report.build_doctor_report(
        doctor_report.DoctorReportInputs(
            runtime=Path(args.runtime),
            current_link=Path(args.current_link),
            verified_link=Path(args.verified_link),
            raw_link=Path(args.raw_link),
            manager_dir=Path(args.manager_dir),
            runtime_store=Path(args.runtime_store_dir),
            raw_store=Path(args.raw_store_dir),
            raw_vendor=Path(args.raw_vendor),
            resolv_conf=Path(args.resolv_conf),
            cert_file=Path(args.cert_file),
            state_file=Path(args.state_file),
            registry_file=Path(args.registry_file),
            migration_report=Path(args.migration_report_file),
            legacy_store=Path(args.legacy_store_dir),
            version=args.version,
            raw_sha256=args.raw_sha256,
            runtime_sha256=args.runtime_sha256,
            prefix=Path(args.prefix),
            runtime_builder=Path(args.runtime_builder),
            patch_policy=args.patch_policy,
            network_boundary=json.loads(args.network_json),
        )
    )
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


def _doctor_render(args: argparse.Namespace) -> int:
    report = json.load(sys.stdin)
    if args.mode == "json":
        return doctor_render.render_json_report(report)
    return doctor_render.render_human_doctor(report)


def _doctor_socket_probe(_args: argparse.Namespace) -> int:
    print(json.dumps(doctor_report.socket_probe(), ensure_ascii=True, sort_keys=True))
    return 0


def _doctor_network_boundary(args: argparse.Namespace) -> int:
    report = doctor_report.build_network_boundary_report(
        baseline=json.loads(args.baseline_json),
        network_off=json.loads(args.off_json),
        network_on=json.loads(args.on_json),
        network_reset=json.loads(args.reset_json),
        exit_codes=(
            int(args.baseline_exit), int(args.off_exit), int(args.on_exit), int(args.reset_exit)
        ),
    )
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


def _package_field(args: argparse.Namespace) -> int:
    print(runtime_checks.extract_pack_field(Path(args.json_file), args.field))
    return 0


def _runtime_integrity(args: argparse.Namespace) -> int:
    ok = runtime_checks.runtime_integrity_ok(
        runtime=Path(args.runtime),
        manifest_path=Path(args.manifest_path),
        builder=Path(args.builder),
        state_path=Path(args.state_path),
        patch_policy=args.patch_policy,
    )
    return 0 if ok else 1


def _raw_integrity(args: argparse.Namespace) -> int:
    ok = runtime_checks.raw_integrity_ok(
        raw_binary=Path(args.raw_binary), state_path=Path(args.state_path)
    )
    return 0 if ok else 1


def _runtime_metadata_current(args: argparse.Namespace) -> int:
    ok = runtime_checks.runtime_metadata_current(
        state_path=Path(args.state_path),
        registry_path=Path(args.registry_path),
        current=Path(args.current),
        verified=Path(args.verified),
        raw=Path(args.raw),
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
    )
    return 0 if ok else 1


def _parse_upstream_commands(_args: argparse.Namespace) -> int:
    for name in runtime_checks.parse_upstream_commands(sys.stdin.read()):
        print(name)
    return 0
