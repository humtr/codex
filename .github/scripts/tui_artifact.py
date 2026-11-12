"""Bind a downloaded native frontend to its successful source qualification."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import stat
import subprocess
import sys

from release_preflight import bounded_read, no_duplicate_object
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "frontend/tui"))
from check_command_boundary import nonzero_tests
from check_upstream_features import verify as verify_upstream_features

from upstream_release import load as load_release, build_recipe
from qualification_signal import native_recipe
from native_completion_results import SUITES as COMPLETION_SUITES, verify as verify_completion_results
RELEASE_INPUT = Path(__file__).resolve().parents[2] / 'release/upstream.json'
# Kept as fixture conveniences; verify() reads the selected source authority.
_INPUT = load_release(RELEASE_INPUT)
UPSTREAM = _INPUT['upstream']['source']
VERSION = _INPUT['upstream']['version']
INPUTS = (
    ".github/workflows/manager-tui-native.yml",
    "frontend/tui/native.patch",
    "crates/release-builder/src/source.rs",
    "crates/release-builder/src/source/patch.rs",
    "crates/release-builder/src/source/syntax.rs",
    "crates/release-builder/src/source/syntax/permissions.rs",
    "crates/release-builder/src/runtime.rs",
    "crates/release-builder/src/native_route.patch",
    "crates/release-builder/src/lib.rs",
    "crates/release-builder/Cargo.toml",
    "Cargo.lock",
    ".github/scripts/upstream_release.py",
    ".github/scripts/native_completion_results.py",
    ".github/scripts/tui_regression_results.py",
    "frontend/tui/src",
    "frontend/tui/apply.py",
    "frontend/tui/test_apply.py",
    "frontend/tui/check_command_boundary.py",
    "frontend/tui/test_command_boundary.py",
    "frontend/tui/check_upstream_features.py",
    "frontend/tui/test_upstream_features.py",
)
FILES = {"codex", "source.txt", "version.txt", "help.txt", "sha256.txt",
         "lock-audit.json", "native.patch"}
LOCK = _INPUT['qualification']['lock_audit']


def metadata(path):
    return json.loads(bounded_read(path, 256 * 1024, "qualification metadata"),
                      object_pairs_hook=no_duplicate_object)


def verify_focused(log, workflow):
    count = nonzero_tests(log)
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', log)
    passed = re.findall(r'(?m)^[ \t]*PASS[ \t]+\[[^\]\n]+\][ \t]+'
                        r'(?:\([ \t]*\d+[ \t]*/[ \t]*\d+[ \t]*\)[ \t]+)?'
                        r'codex-tui[ \t]+([^\s]+)[ \t]*$', text)
    if len(passed) != count or len(set(passed)) != count:
        raise ValueError('native focused execution records differ from the summary')
    commands = [line for line in workflow.splitlines()
                if 'just test ' in line and 'profile-tui-focused.log' in line]
    if len(commands) != 1:
        raise ValueError('native focused test selection is ambiguous or missing')
    filters = re.findall(r"-E '([^'\n]+)'", commands[0])
    if len(filters) != 1:
        raise ValueError('unsupported native focused test selection')
    scopes = re.findall(r'test\(([A-Za-z0-9_]+)\)', filters[0])
    remainder = re.sub(r'test\([A-Za-z0-9_]+\)', '', filters[0])
    if not scopes or remainder.strip(' |\t'):
        raise ValueError('unsupported native focused test selection')
    missing = [scope for scope in scopes if not any(scope in name for name in passed)]
    if missing:
        raise ValueError('native focused proof is missing scopes: '+', '.join(missing))



def verify_completion(proof, workflow):
    # Retained generations have no completion-proof contract. Once the exact
    # qualified workflow declares it, all six bounded logs are mandatory.
    commands = [line for line in workflow.splitlines()
                if 'native_completion_results.py' in line and not line.lstrip().startswith('#')]
    if not commands:
        return 0
    observed = []
    for command in commands:
        matches = re.findall(r'native_completion_results[.]py" ([a-z-]+) "\$RUNNER_TEMP/(native-provider-completion-[a-z-]+[.]log)"', command)
        if len(matches) != 1:
            raise ValueError('native completion proof selection is ambiguous or missing')
        observed.append(matches[0])
    expected = {(suite, f'native-provider-completion-{suite}.log') for suite in COMPLETION_SUITES}
    if len(observed) != len(expected) or set(observed) != expected:
        raise ValueError('native completion proof suite inventory differs')
    count = 0
    for suite, filename in observed:
        log = bounded_read(proof / filename, 16 * 1024 * 1024, 'native completion tests').decode()
        nonzero_tests(log)
        count += verify_completion_results(log, suite)
    return count


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
        qualified = load_release(path)
        # Retained schema1 readers require this field. Current proof admission
        # uses declared scope coverage, independently of additional upstream tests.
        qualified['qualification']['focused_tests'] = inputs['qualification']['focused_tests']
        if build_recipe(qualified) != build_recipe(inputs):
            raise ValueError('native release recipe differs from qualified source')

def verify(args):
    inputs = load_release(args.source / 'release/upstream.json')
    version = inputs['upstream']['version']
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
    return verify_payload(artifact=args.artifact, proof=args.proof,
                          source_sha=args.source_sha, sha256=args.sha256,
                          inputs=inputs, workflow=args.source / INPUTS[0])


def verify_payload(*, artifact, proof, source_sha, sha256, inputs, workflow):
    """Check artifact/proof contents against expectations, without admitting them.

    This read-only check establishes neither run authority nor Git recipe
    equivalence. Callers must independently validate the actual run, selected
    source recipe and expected digest. The public verifier does so before
    calling this function; its branch and successful-run requirements remain
    mandatory for public admission.
    """
    version = inputs['upstream']['version']
    upstream = inputs['upstream']['source']
    if artifact.is_symlink() or not artifact.is_dir():
        raise ValueError("native artifact root is not an owned directory")
    if {p.name for p in artifact.iterdir()} != FILES:
        raise ValueError("native artifact inventory is incomplete or extended")
    for name in FILES:
        path = artifact / name
        if not stat.S_ISREG(path.lstat().st_mode):
            raise ValueError("native artifact contains a nonregular asset")
    if (artifact / "codex").stat().st_size > 512 * 1024 * 1024:
        raise ValueError("native executable exceeds the builder bound")
    if bounded_read(artifact / "version.txt", 128, "native version") != f"codex-cli {version}\n".encode():
        raise ValueError("native/backend version mismatch")
    source = (f"upstream_sha={upstream}\nsource_sha={source_sha}\n"
              "target=aarch64-unknown-linux-musl\n"
              "profile=release opt-level=z codegen-units=1 strip=symbols\n")
    if bounded_read(artifact / "source.txt", 1024, "native source") != source.encode():
        raise ValueError("native source/target/profile differs from qualification")
    if metadata(artifact / "lock-audit.json") != inputs['qualification']['lock_audit']:
        raise ValueError("native dependency audit differs from qualification")
    digest = hashlib.sha256()
    with (artifact / "codex").open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    digest = digest.hexdigest()
    if digest != sha256:
        raise ValueError("native executable differs from the pinned qualified digest")
    checksum = bounded_read(artifact / "sha256.txt", 4096, "native checksum").decode()
    if not re.fullmatch(re.escape(digest) + r"  [^\r\n\x00]+/codex\n", checksum):
        raise ValueError("native executable digest differs from qualified artifact")
    proof_root = proof
    proof = proof_root / "_temp"
    if proof_root.is_symlink() or proof.is_symlink():
        raise ValueError("native proof root is not an owned directory")
    log = bounded_read(proof / "profile-tui-focused.log", 16 * 1024 * 1024, "native tests").decode()
    verify_focused(log, workflow.read_text())
    verify_completion(proof, workflow.read_text())
    verify_upstream_features(proof)
    formatted = bounded_read(proof / "profile-tui-formatted.patch", 1024 * 1024, "formatted native patch")
    linted = bounded_read(proof / "profile-tui-linted.patch", 1024 * 1024, "linted native patch")
    if formatted != linted or formatted != bounded_read(artifact / "native.patch", 1024 * 1024, "native patch"):
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
        # Retained external CLI record key; signed consumers keep their identity.
        print("manager_tui_sha256\t" + verify(args))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, "native qualification refused: " + str(error) + "\n")


if __name__ == "__main__":
    main()
