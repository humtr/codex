"""Bind a downloaded native frontend to its successful source qualification."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
import subprocess
import sys

from rald3_preflight import bounded_read, no_duplicate_object
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "frontend/manager-tui"))
from check_command_boundary import nonzero_tests

UPSTREAM = "979011409de0a60b52f179721948e65531d26144"
VERSION = "0.161.0"
INPUTS = (
    ".github/workflows/manager-tui-native.yml",
    "frontend/manager-tui/native.patch",
    "frontend/manager-tui/normalize_lock.py",
    "frontend/manager-tui/test_normalize_lock.py",
    "frontend/manager-tui/check_command_boundary.py",
    "frontend/manager-tui/test_command_boundary.py",
)
FILES = {"codex", "source.txt", "version.txt", "help.txt", "sha256.txt",
         "lock-audit.json", "native.patch"}
LOCK = {
    "external_packages_unchanged": 1314,
    "workspace_versions_changed": 160,
    "original_lock_sha256": "3206e2fdb53a3498758ce2f2972f19fae26beb261befd65cc0eb9e02efe23d52",
    "normalized_lock_sha256": "4e43cd81ed341c32bde170917f38c3e79702cbc69d1f96744ab1e09f9e80a601",
}


def metadata(path):
    return json.loads(bounded_read(path, 256 * 1024, "qualification metadata"),
                      object_pairs_hook=no_duplicate_object)


def verify(args):
    if args.version != VERSION or not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        raise ValueError("no qualified native pair for this backend/source")
    run = metadata(args.run_json)
    expected = {"id": args.run_id, "path": INPUTS[0], "event": "push",
                "head_branch": "rewrite/rust-core", "head_sha": args.source_sha,
                "status": "completed", "conclusion": "success"}
    if any(run.get(k) != v for k, v in expected.items()) or run.get("repository", {}).get("full_name") != "humtr/codex":
        raise ValueError("native qualification run is not the pinned successful authority")
    for relative in INPUTS:
        blobs = [subprocess.check_output(
            ["git", "-C", str(args.source), "rev-parse", f"{revision}:{relative}"],
            stderr=subprocess.DEVNULL) for revision in ("HEAD", args.source_sha)]
        if blobs[0] != blobs[1]:
            raise ValueError("maintained native input differs from qualified source: " + relative)
    if args.artifact.is_symlink() or not args.artifact.is_dir():
        raise ValueError("native artifact root is not an owned directory")
    if {p.name for p in args.artifact.iterdir()} != FILES:
        raise ValueError("native artifact inventory is incomplete or extended")
    for name in FILES:
        path = args.artifact / name
        if not stat.S_ISREG(path.lstat().st_mode):
            raise ValueError("native artifact contains a nonregular asset")
    if (args.artifact / "codex").stat().st_size > 512 * 1024 * 1024:
        raise ValueError("native executable exceeds the builder bound")
    if bounded_read(args.artifact / "version.txt", 128, "native version") != f"codex-cli {VERSION}\n".encode():
        raise ValueError("native/backend version mismatch")
    source = (f"upstream_sha={UPSTREAM}\nsource_sha={args.source_sha}\n"
              "target=aarch64-unknown-linux-musl\n"
              "profile=release opt-level=z codegen-units=1 strip=symbols\n")
    if bounded_read(args.artifact / "source.txt", 1024, "native source") != source.encode():
        raise ValueError("native source/target/profile differs from qualification")
    if metadata(args.artifact / "lock-audit.json") != LOCK:
        raise ValueError("native dependency audit differs from qualification")
    digest = hashlib.sha256()
    with (args.artifact / "codex").open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    digest = digest.hexdigest()
    if digest != args.sha256:
        raise ValueError("native executable differs from the pinned qualified digest")
    checksum = bounded_read(args.artifact / "sha256.txt", 4096, "native checksum").decode()
    if not re.fullmatch(re.escape(digest) + r"  [^\r\n\x00]+/codex\n", checksum):
        raise ValueError("native executable digest differs from qualified artifact")
    proof = args.proof / "_temp"
    if args.proof.is_symlink() or proof.is_symlink():
        raise ValueError("native proof root is not an owned directory")
    log = bounded_read(proof / "profile-tui-focused.log", 16 * 1024 * 1024, "native tests").decode()
    if nonzero_tests(log) != 27:
        raise ValueError("actual native regression set is not qualified")
    formatted = bounded_read(proof / "profile-tui-formatted.patch", 1024 * 1024, "formatted native patch")
    linted = bounded_read(proof / "profile-tui-linted.patch", 1024 * 1024, "linted native patch")
    if formatted != linted or formatted != bounded_read(args.artifact / "native.patch", 1024 * 1024, "native patch"):
        raise ValueError("native format/lint/artifact source differs")
    return digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--run-json", type=Path, required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--proof", type=Path, required=True)
    args = parser.parse_args()
    try:
        print("manager_tui_sha256\t" + verify(args))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, "native qualification refused: " + str(error) + "\n")


if __name__ == "__main__":
    main()
