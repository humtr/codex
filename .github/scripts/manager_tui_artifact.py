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
from check_upstream_features import verify as verify_upstream_features

from upstream_release import load as load_release, build_recipe
from qualification_signal import native_recipe
RELEASE_INPUT = Path(__file__).resolve().parents[2] / 'release/upstream.json'
# Kept as fixture conveniences; verify() reads the selected source authority.
_INPUT = load_release(RELEASE_INPUT)
UPSTREAM = _INPUT['upstream']['source']
VERSION = _INPUT['upstream']['version']
INPUTS = (
    ".github/workflows/manager-tui-native.yml",
    "frontend/manager-tui/native.patch",
    ".github/scripts/upstream_release.py",
    "frontend/manager-tui/src",
    "frontend/manager-tui/apply.py",
    "frontend/manager-tui/test_apply.py",
    "frontend/manager-tui/normalize_lock.py",
    "frontend/manager-tui/test_normalize_lock.py",
    "frontend/manager-tui/check_command_boundary.py",
    "frontend/manager-tui/test_command_boundary.py",
    "frontend/manager-tui/check_upstream_features.py",
    "frontend/manager-tui/test_upstream_features.py",
)
FILES = {"codex", "source.txt", "version.txt", "help.txt", "sha256.txt",
         "lock-audit.json", "native.patch"}
LOCK = _INPUT['qualification']['lock_audit']


def metadata(path):
    return json.loads(bounded_read(path, 256 * 1024, "qualification metadata"),
                      object_pairs_hook=no_duplicate_object)


def verify_recipe(source, source_sha):
    inputs = load_release(source / 'release/upstream.json')
    for relative in INPUTS:
        blobs = [subprocess.check_output(
            ["git", "-C", str(source), "rev-parse", f"{revision}:{relative}"],
            stderr=subprocess.DEVNULL) for revision in ("HEAD", source_sha)]
        if blobs[0] != blobs[1]:
            if relative == INPUTS[0]:
                workflows=[subprocess.check_output(['git','-C',str(source),'show',revision+':'+relative]).decode() for revision in ('HEAD',source_sha)]
                if native_recipe(workflows[0]) == native_recipe(workflows[1]):
                    continue
            raise ValueError("maintained native input differs from qualified source: " + relative)
    qualified_input = subprocess.check_output(
        ['git', '-C', str(source), 'show', source_sha + ':release/upstream.json'],
        stderr=subprocess.DEVNULL)
    # Parse through the same strict loader without modifying a source checkout.
    import tempfile
    with tempfile.TemporaryDirectory(prefix='native-input-') as temporary:
        path = Path(temporary) / 'upstream.json'
        path.write_bytes(qualified_input)
        if build_recipe(load_release(path)) != build_recipe(inputs):
            raise ValueError('native release recipe differs from qualified source')

def verify(args):
    inputs = load_release(args.source / 'release/upstream.json')
    version = inputs['upstream']['version']
    upstream = inputs['upstream']['source']
    if args.version != version or not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
        raise ValueError("no qualified native pair for this backend/source")
    run = metadata(args.run_json)
    expected = {"id": args.run_id, "path": INPUTS[0],
                "head_branch": "rewrite/rust-core", "head_sha": args.source_sha,
                "status": "completed", "conclusion": "success"}
    if (any(run.get(k) != v for k, v in expected.items())
            or run.get("event") not in {"push", "workflow_dispatch"}
            or run.get("repository", {}).get("full_name") != "humtr/codex"):
        raise ValueError("native qualification run is not the pinned successful authority")
    verify_recipe(args.source, args.source_sha)
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
    if bounded_read(args.artifact / "version.txt", 128, "native version") != f"codex-cli {version}\n".encode():
        raise ValueError("native/backend version mismatch")
    source = (f"upstream_sha={upstream}\nsource_sha={args.source_sha}\n"
              "target=aarch64-unknown-linux-musl\n"
              "profile=release opt-level=z codegen-units=1 strip=symbols\n")
    if bounded_read(args.artifact / "source.txt", 1024, "native source") != source.encode():
        raise ValueError("native source/target/profile differs from qualification")
    if metadata(args.artifact / "lock-audit.json") != inputs['qualification']['lock_audit']:
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
    if nonzero_tests(log) != inputs['qualification']['focused_tests']:
        raise ValueError("actual native regression set is not qualified")
    verify_upstream_features(proof)
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
