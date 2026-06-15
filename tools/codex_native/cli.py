"""Internal command line entrypoint for Codex native helpers."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Protocol

from . import activation_cli, hashing, maintenance_cli, registry, state, store
from .errors import CodexNativeError


class SubparserCollection(Protocol):
    def add_parser(self, name: str, **kwargs: object) -> argparse.ArgumentParser: ...


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python3 -m codex_native.cli",
        description="Internal Codex native helper interface.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser(
        "validate",
        help=argparse.SUPPRESS,
        description="Run a minimal internal package diagnostic.",
    )
    validate.set_defaults(func=_validate)
    _add_state_commands(subparsers)
    _add_registry_commands(subparsers)
    _add_store_commands(subparsers)
    activation_cli.add_commands(subparsers)
    maintenance_cli.add_commands(subparsers)
    return parser


def _add_state_commands(subparsers: SubparserCollection) -> None:
    state_read = subparsers.add_parser("state-read-field")
    state_read.add_argument("--state-file", required=True)
    state_read.add_argument("--field", required=True)
    state_read.set_defaults(func=_state_read_field)

    state_write = subparsers.add_parser("state-write")
    state_write.add_argument("--state-file", required=True)
    state_write.add_argument("--version", required=True)
    state_write.add_argument("--raw-sha256", required=True)
    state_write.add_argument("--runtime-sha256", required=True)
    state_write.add_argument("--package-spec", required=True)
    state_write.add_argument("--active-tuple-id", required=True)
    state_write.add_argument("--wrapper-version", required=True)
    state_write.add_argument("--wrapper-commit", required=True)
    state_write.add_argument("--updated-at", required=True)
    state_write.add_argument("--verified-tuple-id", required=True)
    state_write.add_argument("--verified-at", required=True)
    state_write.set_defaults(func=_state_write)


def _add_registry_commands(subparsers: SubparserCollection) -> None:
    registry_record = subparsers.add_parser("registry-record")
    registry_record.add_argument("--registry-file", required=True)
    registry_record.add_argument("--version", required=True)
    registry_record.add_argument("--raw-sha256", required=True)
    registry_record.add_argument("--runtime-sha256", required=True)
    registry_record.add_argument("--package-spec", required=True)
    registry_record.add_argument("--runtime-path", required=True)
    registry_record.add_argument("--wrapper-version", required=True)
    registry_record.add_argument("--wrapper-commit", required=True)
    registry_record.add_argument("--runtime-store-dir", required=True)
    registry_record.add_argument("--updated-at", required=True)
    registry_record.add_argument("--smoke-tested-at", required=True)
    registry_record.add_argument("--raw-path", required=True)
    registry_record.set_defaults(func=_registry_record)

    tuple_for_path = subparsers.add_parser("registry-tuple-for-runtime-path")
    tuple_for_path.add_argument("--registry-file", required=True)
    tuple_for_path.add_argument("--runtime-path", required=True)
    tuple_for_path.set_defaults(func=_registry_tuple_for_runtime_path)

    tuple_fields = subparsers.add_parser("registry-tuple-state-fields")
    tuple_fields.add_argument("--registry-file", required=True)
    tuple_fields.add_argument("--tuple-id", required=True)
    tuple_fields.set_defaults(func=_registry_tuple_state_fields)


def _add_store_commands(subparsers: SubparserCollection) -> None:
    hash_file = subparsers.add_parser("hash-file")
    hash_file.add_argument("--path", required=True)
    hash_file.set_defaults(func=_hash_file)

    tree_digest = subparsers.add_parser("tree-digest")
    tree_digest.add_argument("--path", required=True)
    tree_digest.set_defaults(func=_tree_digest)

    publish_tree = subparsers.add_parser("store-publish-tree")
    publish_tree.add_argument("--source-dir", required=True)
    publish_tree.add_argument("--target-dir", required=True)
    publish_tree.set_defaults(func=_store_publish_tree)

    publish_runtime = subparsers.add_parser("store-publish-runtime")
    publish_runtime.add_argument("--source-dir", required=True)
    publish_runtime.add_argument("--target-dir", required=True)
    publish_runtime.add_argument("--expected-sha256", required=True)
    publish_runtime.set_defaults(func=_store_publish_runtime)

    publish_raw = subparsers.add_parser("store-publish-raw")
    publish_raw.add_argument("--source-dir", required=True)
    publish_raw.add_argument("--target-dir", required=True)
    publish_raw.add_argument("--expected-sha256", required=True)
    publish_raw.set_defaults(func=_store_publish_raw)


def _validate(_args: argparse.Namespace) -> int:
    print("codex_native: ok")
    return 0


def _state_read_field(args: argparse.Namespace) -> int:
    print(state.read_field(Path(args.state_file), args.field))
    return 0


def _state_write(args: argparse.Namespace) -> int:
    state.write(
        state_file=Path(args.state_file),
        version=args.version,
        raw_sha256=args.raw_sha256,
        runtime_sha256=args.runtime_sha256,
        package_spec=args.package_spec,
        active_tuple_id=args.active_tuple_id,
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
        updated_at=args.updated_at,
        verified_tuple_id=args.verified_tuple_id,
        verified_at=args.verified_at,
    )
    return 0


def _registry_record(args: argparse.Namespace) -> int:
    tuple_id = registry.record(
        registry_file=Path(args.registry_file),
        version=args.version,
        raw_sha256=args.raw_sha256,
        runtime_sha256=args.runtime_sha256,
        package_spec=args.package_spec,
        runtime_path=args.runtime_path,
        wrapper_version=args.wrapper_version,
        wrapper_commit=args.wrapper_commit,
        runtime_store_dir=Path(args.runtime_store_dir),
        updated_at=args.updated_at,
        smoke_tested_at=args.smoke_tested_at,
        raw_path=args.raw_path,
    )
    print(tuple_id)
    return 0


def _registry_tuple_for_runtime_path(args: argparse.Namespace) -> int:
    print(
        registry.tuple_for_runtime_path(
            Path(args.registry_file),
            args.runtime_path,
        )
    )
    return 0


def _registry_tuple_state_fields(args: argparse.Namespace) -> int:
    print(registry.tuple_state_fields(Path(args.registry_file), args.tuple_id))
    return 0


def _hash_file(args: argparse.Namespace) -> int:
    print(hashing.sha256_file(Path(args.path)))
    return 0


def _tree_digest(args: argparse.Namespace) -> int:
    print(hashing.tree_digest(Path(args.path)))
    return 0


def _store_publish_tree(args: argparse.Namespace) -> int:
    print(
        store.publish_immutable_tree(
            Path(args.source_dir),
            Path(args.target_dir),
        )
    )
    return 0


def _store_publish_runtime(args: argparse.Namespace) -> int:
    print(
        store.publish_runtime_artifact(
            Path(args.source_dir),
            Path(args.target_dir),
            args.expected_sha256,
        )
    )
    return 0


def _store_publish_raw(args: argparse.Namespace) -> int:
    print(
        store.publish_raw_artifact(
            Path(args.source_dir),
            Path(args.target_dir),
            args.expected_sha256,
        )
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except CodexNativeError as exc:
        print(f"codex_native: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
