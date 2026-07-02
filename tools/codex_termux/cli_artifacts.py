"""Artifact, package, and field helper commands for the internal CLI."""

from __future__ import annotations

import argparse
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import Protocol

from . import hashing, paths, registry, release, runtime_checks
from .errors import IntegrityError


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def add_commands(sub: SubparserCollection) -> None:
    hash_file = sub.add_parser("hash-file")
    hash_file.add_argument("--path", required=True)
    hash_file.set_defaults(func=lambda args: _print(hashing.sha256_file(Path(args.path))))

    tree_digest = sub.add_parser("tree-digest")
    tree_digest.add_argument("--path", required=True)
    tree_digest.set_defaults(func=lambda args: _print(hashing.tree_digest(Path(args.path))))

    store_id = sub.add_parser("store-id")
    for name in ("version", "sha256", "builder-sha256", "bwrap-sha256", "rg-sha256", "tree-sha256"):
        store_id.add_argument(f"--{name}", required=True)
    store_id.set_defaults(func=_store_id)

    resolve = sub.add_parser("resolve-path")
    resolve.add_argument("--path", required=True)
    resolve.set_defaults(func=lambda args: _print(paths.resolve_text(Path(args.path))))

    parent = sub.add_parser("parent-dir")
    parent.add_argument("--path", required=True)
    parent.set_defaults(func=lambda args: _print(paths.shell_parent_dir(args.path)))

    strip_slashes = sub.add_parser("strip-trailing-slashes")
    strip_slashes.add_argument("--path", required=True)
    strip_slashes.set_defaults(func=lambda args: _print(paths.strip_trailing_slashes(args.path)))

    managed = sub.add_parser("managed-tree-target-ok")
    for name in ("path", "label", "home", "prefix", "tmpdir", "root", "state"):
        managed.add_argument(f"--{name}", required=True)
    managed.set_defaults(func=_managed_tree_target_ok)

    validate_tarball = sub.add_parser("validate-tarball")
    validate_tarball.add_argument("--path", required=True)
    validate_tarball.set_defaults(func=_validate_tarball)

    release_package = sub.add_parser("release-package")
    release_package.add_argument("--package-root", required=True)
    release_package.add_argument("--out", required=True)
    release_package.set_defaults(func=_release_package)

    package_field = sub.add_parser("package-field")
    package_field.add_argument("--json-file", required=True)
    package_field.add_argument("--field", required=True)
    package_field.set_defaults(
        func=lambda args: _print(runtime_checks.extract_pack_field(Path(args.json_file), args.field))
    )

    package_spec = sub.add_parser("package-spec")
    package_spec.add_argument("--requested", default="")
    package_spec.add_argument("--default", required=True)
    package_spec.set_defaults(
        func=lambda args: _print(runtime_checks.package_spec(args.requested, args.default))
    )

    runtime_retention = sub.add_parser("runtime-retention-ok")
    runtime_retention.add_argument("--value", required=True)
    runtime_retention.set_defaults(func=_runtime_retention_ok)

    support_source = sub.add_parser("support-source-dir")
    support_source.add_argument("--manager-dir", required=True)
    support_source.add_argument("--runtime-dir", required=True)
    support_source.set_defaults(func=_support_source_dir)

    wrapper_metadata = sub.add_parser("wrapper-metadata-field")
    wrapper_metadata.add_argument("--manager-dir", required=True)
    wrapper_metadata.add_argument("--runtime-dir", required=True)
    wrapper_metadata.add_argument("--field", choices=("version", "commit"), required=True)
    wrapper_metadata.set_defaults(func=_wrapper_metadata_field)

    state_field = sub.add_parser("state-read-field")
    state_field.add_argument("--state-file", required=True)
    state_field.add_argument("--field", required=True)
    state_field.set_defaults(
        func=lambda args: _print(runtime_checks.state_field(Path(args.state_file), args.field))
    )

    runtime_date = sub.add_parser("registry-active-runtime-date")
    runtime_date.add_argument("--registry-file", required=True)
    runtime_date.set_defaults(
        func=lambda args: _print(registry.active_runtime_created_at(Path(args.registry_file)))
    )

    upstream_release_date = sub.add_parser("upstream-release-date")
    upstream_release_date.add_argument("--version", required=True)
    upstream_release_date.set_defaults(
        func=lambda args: _print(runtime_checks.upstream_release_date(sys.stdin.read(), args.version))
    )

    release_cache_read = sub.add_parser("upstream-release-cache-read")
    release_cache_read.add_argument("--cache", required=True)
    release_cache_read.add_argument("--version", required=True)
    release_cache_read.set_defaults(func=_upstream_release_cache_read)

    release_cache_write = sub.add_parser("upstream-release-cache-write")
    release_cache_write.add_argument("--cache", required=True)
    release_cache_write.add_argument("--version", required=True)
    release_cache_write.add_argument("--release-date", required=True)
    release_cache_write.set_defaults(func=_upstream_release_cache_write)

    display_date = sub.add_parser("display-runtime-date")
    display_date.add_argument("--value", default="")
    display_date.set_defaults(func=lambda args: _print(registry.display_runtime_date(args.value)))

    auto_mode = sub.add_parser("auto-update-mode")
    auto_mode.add_argument("--mode", default="")
    auto_mode.set_defaults(func=lambda args: _print(runtime_checks.normalize_auto_update_mode(args.mode)))

    auto_due = sub.add_parser("auto-update-due")
    auto_due.add_argument("--enabled", required=True)
    auto_due.add_argument("--mode", required=True)
    auto_due.add_argument("--now", required=True)
    auto_due.add_argument("--last", default="0")
    auto_due.add_argument("--interval", required=True)
    auto_due.set_defaults(func=_auto_update_due)

    failed_due = sub.add_parser("failed-auto-update-due")
    failed_due.add_argument("--record", default="")
    failed_due.add_argument("--version", required=True)
    failed_due.add_argument("--now", required=True)
    failed_due.add_argument("--interval", required=True)
    failed_due.set_defaults(func=_failed_auto_update_due)

    update_prompt = sub.add_parser("update-prompt-decision")
    update_prompt.add_argument("--choice", default="")
    update_prompt.set_defaults(
        func=lambda args: _print(runtime_checks.update_prompt_decision(args.choice))
    )


def _print(value: object) -> int:
    print(value)
    return 0


def _store_id(args: argparse.Namespace) -> int:
    return _print(
        paths.store_id(
            args.version,
            args.sha256,
            args.builder_sha256,
            args.bwrap_sha256,
            args.rg_sha256,
            args.tree_sha256,
        )
    )


def _managed_tree_target_ok(args: argparse.Namespace) -> int:
    paths.assert_managed_tree_target(
        args.path,
        args.label,
        home=args.home,
        prefix=args.prefix,
        tmpdir=args.tmpdir,
        root=args.root,
        state=args.state,
    )
    return 0


def _auto_update_due(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.auto_update_due(
        enabled=args.enabled,
        mode=args.mode,
        now=int(args.now),
        last=args.last,
        interval=int(args.interval),
    ) else 1


def _failed_auto_update_due(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.failed_auto_update_due(
        record=args.record,
        version=args.version,
        now=int(args.now),
        interval=int(args.interval),
    ) else 1


def _upstream_release_cache_read(args: argparse.Namespace) -> int:
    value = runtime_checks.read_upstream_release_cache(Path(args.cache), args.version)
    if value:
        print(value)
    return 0 if value else 1


def _upstream_release_cache_write(args: argparse.Namespace) -> int:
    runtime_checks.write_upstream_release_cache(Path(args.cache), args.version, args.release_date)
    return 0


def _runtime_retention_ok(args: argparse.Namespace) -> int:
    return 0 if runtime_checks.runtime_retention_ok(args.value) else 1


def _support_source_dir(args: argparse.Namespace) -> int:
    return _print(runtime_checks.support_source_dir(
        manager_dir=Path(args.manager_dir),
        runtime_dir=Path(args.runtime_dir),
    ))


def _wrapper_metadata_field(args: argparse.Namespace) -> int:
    return _print(runtime_checks.wrapper_metadata_field(
        manager_dir=Path(args.manager_dir),
        runtime_dir=Path(args.runtime_dir),
        field=args.field,
    ))


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


def _release_package(args: argparse.Namespace) -> int:
    out = Path(args.out)
    release.write_zip(Path(args.package_root), out)
    print(out)
    return 0
