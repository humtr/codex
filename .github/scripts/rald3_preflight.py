#!/usr/bin/env python3
"""Strict, dependency-free RALD-3 hosted-producer preflight helpers."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
import sys
from urllib.parse import quote, urlsplit

UPSTREAM_ASSET = "codex-package-aarch64-unknown-linux-musl.tar.gz"
DEFERRED_MARKER = ".manager-probe-deferred"
DEFERRED_MARKER_BYTES = b"codex-manager-probe-deferred-v1\n"
GENERATION_FORMAT = "codex-local-generation-v2"
PACKAGE_IDENTITY = "openai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz"
R10_METADATA = "r10-browser-helper-bridge-v1"
MAX_UPSTREAM_METADATA = 128 * 1024
MAX_INDEX = 16 * 1024
MAX_MANIFEST = 128 * 1024
MAX_DESCRIPTOR = 64 * 1024
LOWER_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GENERATION_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,511}\Z")


class PreflightError(ValueError):
    pass


def fail(message: str) -> "None":
    raise PreflightError(message)


def bounded_read(path: Path, maximum: int, label: str) -> bytes:
    try:
        st = path.lstat()
    except OSError as exc:
        fail(f"{label} is unavailable: {exc}")
    if not stat.S_ISREG(st.st_mode):
        fail(f"{label} is not a regular file")
    if st.st_size > maximum:
        fail(f"{label} exceeds its byte bound")
    data = path.read_bytes()
    if len(data) > maximum:
        fail(f"{label} exceeds its byte bound")
    return data


def text_lines(path: Path, maximum: int, label: str) -> list[str]:
    data = bounded_read(path, maximum, label)
    if b"\r" in data:
        fail(f"{label} has unsupported line endings")
    if not data.endswith(b"\n"):
        fail(f"{label} is missing its final newline")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        fail(f"{label} is not UTF-8")
    return text[:-1].split("\n")


def stable_version(value: str) -> tuple[int, int, int]:
    if len(value) > 64 or not re.fullmatch(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)", value):
        fail("stable version is invalid")
    numbers = tuple(int(component) for component in value.split("."))
    if any(number > (1 << 63) - 1 for number in numbers):
        fail("stable version is outside its bound")
    return numbers  # type: ignore[return-value]


def lower_sha256(value: str, label: str) -> str:
    if not LOWER_SHA256.fullmatch(value):
        fail(f"{label} is not canonical lowercase SHA-256")
    return value


def generation_id(value: str, label: str = "generation id") -> str:
    if value in {".", ".."} or not GENERATION_ID.fullmatch(value):
        fail(f"{label} is invalid")
    return value


def positive_decimal(value: str, label: str) -> int:
    if not re.fullmatch(r"[1-9][0-9]*", value):
        fail(f"{label} is invalid")
    number = int(value)
    if number > (1 << 64) - 1:
        fail(f"{label} is outside its bound")
    return number


class NoDuplicateObject(dict):
    pass


def no_duplicate_object(pairs: list[tuple[str, object]]) -> NoDuplicateObject:
    result: NoDuplicateObject = NoDuplicateObject()
    for key, value in pairs:
        if key in result:
            fail(f"JSON object contains duplicate key {key!r}")
        result[key] = value
    return result


def parse_upstream(path: Path) -> tuple[str, str]:
    raw = bounded_read(path, MAX_UPSTREAM_METADATA, "upstream metadata")
    try:
        value = json.loads(raw, object_pairs_hook=no_duplicate_object)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        fail(f"upstream metadata is malformed JSON: {exc}")
    if not isinstance(value, dict):
        fail("upstream metadata root is not an object")
    tag = value.get("tag_name")
    assets = value.get("assets")
    if not isinstance(tag, str) or not tag.startswith("rust-v"):
        fail("upstream metadata has an unsupported tag")
    version = tag.removeprefix("rust-v")
    stable_version(version)
    if not isinstance(assets, list):
        fail("upstream metadata has no asset list")
    matched: list[str] = []
    for asset in assets:
        if not isinstance(asset, dict):
            fail("upstream metadata asset is not an object")
        if asset.get("name") == UPSTREAM_ASSET:
            digest = asset.get("digest")
            if not isinstance(digest, str) or not digest.startswith("sha256:"):
                fail("upstream target package digest is missing")
            matched.append(lower_sha256(digest.removeprefix("sha256:"), "upstream package digest"))
    if len(matched) != 1:
        fail("upstream metadata must contain exactly one target package")
    return version, matched[0]


def canonical_https_base(value: str, generation: str) -> str:
    if len(value) > 4096 or any(ord(char) < 0x21 or ord(char) > 0x7e for char in value):
        fail("release base URL is not canonical ASCII")
    parsed = urlsplit(value)
    if parsed.scheme != "https" or not parsed.netloc or parsed.query or parsed.fragment or "\\" in value:
        fail("release base URL is not canonical HTTPS")
    if not value.endswith("/"):
        fail("release base URL must end with slash")
    final_component = parsed.path.rstrip("/").rsplit("/", 1)[-1]
    expected = quote(generation, safe="-._~")
    if final_component != expected:
        fail("release base URL does not match generation id")
    return value


def parse_update_index(path: Path) -> tuple[str, str]:
    lines = text_lines(path, MAX_INDEX, "update index")
    if len(lines) != 4 or lines[0] != "codex-update-index-v1":
        fail("update index format is unsupported")
    expected = ["channel", "generation_id", "release_base"]
    values: dict[str, str] = {}
    for line, name in zip(lines[1:], expected, strict=True):
        parts = line.split("\t")
        if len(parts) != 2 or parts[0] != name or not parts[1] or any(ord(c) < 0x20 for c in parts[1]):
            fail(f"update index field {name} is invalid")
        values[name] = parts[1]
    if values["channel"] != "stable":
        fail("update index channel is unsupported")
    generation = generation_id(values["generation_id"], "update index generation id")
    base = canonical_https_base(values["release_base"], generation)
    return generation, base


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_descriptor(path: Path) -> dict[str, str]:
    lines = text_lines(path, MAX_DESCRIPTOR, "generation descriptor")
    if not lines or lines[0] != GENERATION_FORMAT:
        fail("generation descriptor format is unsupported")
    result: dict[str, str] = {}
    for line in lines[1:]:
        parts = line.split("\t")
        if len(parts) == 2:
            key, value = parts
            if key in result or not key or not value:
                fail("generation descriptor field is invalid")
            result[key] = value
        elif len(parts) == 3 and parts[0] == "helper":
            key = f"helper:{parts[1]}"
            if key in result or not parts[1] or not parts[2]:
                fail("generation descriptor helper is invalid")
            result[key] = parts[2]
        else:
            fail("generation descriptor line is invalid")
    required = {
        "generation_id", "upstream_package_identity", "upstream_package_version",
        "source_artifact_digest", "expected_platform", "expected_architecture",
        "patch_policy_id", "runtime_digest", "core_artifact_digest",
        "manager_artifact_digest", "core_api_identity", "persistent_schema_identity",
        "qualification", "creation_metadata", "upstream_doctor", "helper_count",
    }
    if not required.issubset(result):
        fail("generation descriptor is incomplete")
    generation_id(result["generation_id"], "descriptor generation id")
    stable_version(result["upstream_package_version"])
    if result["upstream_package_identity"] != PACKAGE_IDENTITY:
        fail("generation descriptor package identity is invalid")
    if result["expected_platform"] != "android" or result["expected_architecture"] != "aarch64":
        fail("generation descriptor platform binding is invalid")
    if result["patch_policy_id"] != "termux-fd-remap-v1" or result["qualification"] != "qualified":
        fail("generation descriptor qualification binding is invalid")
    for field in ["source_artifact_digest", "runtime_digest", "core_artifact_digest"]:
        lower_sha256(result[field], field)
    if result["manager_artifact_digest"] != "-":
        lower_sha256(result["manager_artifact_digest"], "manager artifact digest")
    return result


def verify_candidate_payload(
    root: Path, expected_version: str, *, deferred_manager_probe: bool
) -> dict[str, str]:
    stable_version(expected_version)
    if not root.is_dir() or root.is_symlink():
        fail("candidate root is not a real directory")
    marker = root / DEFERRED_MARKER
    if deferred_manager_probe:
        marker_data = bounded_read(
            marker, len(DEFERRED_MARKER_BYTES), "deferred Manager probe marker"
        )
        if marker_data != DEFERRED_MARKER_BYTES:
            fail("deferred Manager probe marker is invalid")
        if stat.S_IMODE(marker.lstat().st_mode) != 0o644:
            fail("deferred Manager probe marker mode is invalid")
    else:
        try:
            marker.lstat()
        except FileNotFoundError:
            pass
        except OSError as exc:
            fail(f"inspect deferred Manager probe marker: {exc}")
        else:
            fail("qualified candidate still has deferred Manager probe marker")
    descriptor = parse_descriptor(root / "generation.meta")
    if descriptor["upstream_package_version"] != expected_version:
        fail("candidate upstream version does not match")
    if descriptor["manager_artifact_digest"] == "-":
        fail("hosted candidate is missing Manager binding")
    expected_files = {
        "runtime": descriptor["runtime_digest"],
        "core": descriptor["core_artifact_digest"],
        "manager": descriptor["manager_artifact_digest"],
    }
    for name, expected_digest in expected_files.items():
        path = root / name
        try:
            st = path.lstat()
        except OSError as exc:
            fail(f"candidate {name} is unavailable: {exc}")
        if not stat.S_ISREG(st.st_mode) or stat.S_IMODE(st.st_mode) != 0o755:
            fail(f"candidate {name} is not a mode-0755 regular file")
        if sha256_file(path) != expected_digest:
            fail(f"candidate {name} digest does not match descriptor")
    return descriptor


def verify_candidate(root: Path, expected_version: str) -> dict[str, str]:
    return verify_candidate_payload(root, expected_version, deferred_manager_probe=True)


def verify_qualified_candidate(root: Path, expected_version: str) -> dict[str, str]:
    return verify_candidate_payload(root, expected_version, deferred_manager_probe=False)


def next_release_sequence(value: str) -> int:
    current = positive_decimal(value, "release sequence")
    if current == (1 << 64) - 1:
        fail("release sequence cannot advance")
    return current + 1


def parse_release_manifest(path: Path, wanted_path: str) -> tuple[str, int, str, str]:
    lines = text_lines(path, MAX_MANIFEST, "release manifest")
    if len(lines) < 10 or lines[0] not in {"codex-release-v3", "codex-release-v4"}:
        fail("release manifest format is unsupported")
    headers = [
        "generation_id", "release_sequence", "channel", "expected_platform",
        "expected_architecture", "core_api_identity", "persistent_schema_identity",
        "release_public_key", "file_count",
    ]
    values: dict[str, str] = {}
    for index, name in enumerate(headers, start=1):
        parts = lines[index].split("\t")
        if len(parts) != 2 or parts[0] != name or not parts[1]:
            fail(f"release manifest field {name} is invalid")
        values[name] = parts[1]
    generation = generation_id(values["generation_id"], "release manifest generation id")
    sequence = positive_decimal(values["release_sequence"], "release sequence")
    if values["channel"] != "stable" or values["expected_platform"] != "android" or values["expected_architecture"] != "aarch64":
        fail("release manifest channel/platform binding is invalid")
    count = positive_decimal(values["file_count"], "release file count")
    if count > 4096 or len(lines) != 10 + count:
        fail("release manifest file inventory length is invalid")
    previous = ""
    wanted: tuple[str, str] | None = None
    for line in lines[10:]:
        parts = line.split("\t")
        if len(parts) != 4 or parts[0] != "file":
            fail("release file inventory entry is invalid")
        rel, digest, mode = parts[1:]
        if not rel or rel.startswith("/") or ".." in Path(rel).parts or rel <= previous:
            fail("release file inventory path/order is invalid")
        previous = rel
        lower_sha256(digest, "release file digest")
        if mode not in {"0644", "0755"}:
            fail("release file mode is invalid")
        if rel == wanted_path:
            if wanted is not None:
                fail("release file inventory contains duplicate wanted path")
            wanted = (digest, mode)
    if wanted is None:
        fail("release file inventory does not contain requested path")
    return generation, sequence, wanted[0], wanted[1]


def print_kv(**values: object) -> None:
    for key, value in values.items():
        print(f"{key}={value}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    subs = parser.add_subparsers(dest="command", required=True)
    p = subs.add_parser("upstream")
    p.add_argument("file", type=Path)
    p = subs.add_parser("index")
    p.add_argument("file", type=Path)
    p = subs.add_parser("descriptor")
    p.add_argument("file", type=Path)
    p = subs.add_parser("manifest-entry")
    p.add_argument("file", type=Path)
    p.add_argument("path")
    p = subs.add_parser("compare")
    p.add_argument("current")
    p.add_argument("upstream")
    p = subs.add_parser("candidate")
    p.add_argument("root", type=Path)
    p.add_argument("expected_version")
    p = subs.add_parser("qualified-candidate")
    p.add_argument("root", type=Path)
    p.add_argument("expected_version")
    p = subs.add_parser("next-sequence")
    p.add_argument("current")
    args = parser.parse_args(argv)
    try:
        if args.command == "upstream":
            version, digest = parse_upstream(args.file)
            print_kv(version=version, archive_sha256=digest)
        elif args.command == "index":
            generation, base = parse_update_index(args.file)
            print_kv(generation_id=generation, release_base=base)
        elif args.command == "descriptor":
            d = parse_descriptor(args.file)
            print_kv(generation_id=d["generation_id"], version=d["upstream_package_version"], source_sha256=d["source_artifact_digest"])
        elif args.command == "manifest-entry":
            generation, sequence, digest, mode = parse_release_manifest(args.file, args.path)
            print_kv(generation_id=generation, release_sequence=sequence, sha256=digest, mode=mode)
        elif args.command == "compare":
            current = stable_version(args.current)
            upstream = stable_version(args.upstream)
            if upstream < current:
                fail("official upstream stable is older than authenticated wrapper stable")
            print_kv(candidate=("true" if upstream > current else "false"))
        elif args.command == "candidate":
            d = verify_candidate(args.root, args.expected_version)
            print_kv(generation_id=d["generation_id"], version=d["upstream_package_version"])
        elif args.command == "qualified-candidate":
            d = verify_qualified_candidate(args.root, args.expected_version)
            print_kv(generation_id=d["generation_id"], version=d["upstream_package_version"])
        elif args.command == "next-sequence":
            print_kv(release_sequence=next_release_sequence(args.current))
        return 0
    except (PreflightError, OSError) as exc:
        print(f"rald3 preflight: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
