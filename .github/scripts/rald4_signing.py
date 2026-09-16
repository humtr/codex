#!/usr/bin/env python3
"""Fail-closed RALD-4 secret-backed candidate signing boundary."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile

SECRET_NAME = "CODEX_RELEASE_SIGNING_KEY"
PRIVATE_KEY_MAX_BYTES = 16 * 1024
PUBLIC_KEY_MAX_BYTES = 16 * 1024
ED25519_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
GENERATION_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._~-]{0,511}\Z")
POSITIVE_DECIMAL = re.compile(r"[1-9][0-9]*\Z")


class SigningError(ValueError):
    pass


def fail(message: str) -> "None":
    raise SigningError(message)


def require_absolute(path: Path, label: str) -> Path:
    if not path.is_absolute() or any(part in {".", ".."} for part in path.parts):
        fail(f"{label} must be a canonical absolute path")
    return path


def require_real_directory(path: Path, label: str) -> Path:
    require_absolute(path, label)
    try:
        metadata = path.lstat()
    except OSError as exc:
        fail(f"{label} is unavailable: {exc}")
    if not stat.S_ISDIR(metadata.st_mode) or path.is_symlink():
        fail(f"{label} is not a real directory")
    try:
        resolved = path.resolve(strict=True)
    except OSError as exc:
        fail(f"{label} cannot be resolved: {exc}")
    if resolved != path:
        fail(f"{label} contains a symlink")
    return path


def require_regular_file(path: Path, maximum: int, label: str, executable: bool = False) -> Path:
    require_absolute(path, label)
    try:
        metadata = path.lstat()
    except OSError as exc:
        fail(f"{label} is unavailable: {exc}")
    if not stat.S_ISREG(metadata.st_mode) or path.is_symlink():
        fail(f"{label} is not a regular file")
    if metadata.st_size == 0 or metadata.st_size > maximum:
        fail(f"{label} is outside its byte bound")
    if executable and metadata.st_mode & 0o111 == 0:
        fail(f"{label} is not executable")
    return path


def run_quiet(argv: list[str], label: str) -> bytes:
    try:
        result = subprocess.run(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env={},
            check=False,
        )
    except OSError as exc:
        fail(f"{label} could not start: {exc}")
    if result.returncode != 0:
        fail(f"{label} failed")
    return result.stdout


def ed25519_raw_public(openssl: Path, key: Path, *, private: bool) -> bytes:
    argv = [str(openssl), "pkey"]
    if not private:
        argv.append("-pubin")
    argv.extend(["-in", str(key), "-pubout", "-outform", "DER"])
    der = run_quiet(argv, "derive Ed25519 public key")
    if len(der) != len(ED25519_SPKI_PREFIX) + 32 or not der.startswith(ED25519_SPKI_PREFIX):
        fail("signing authority is not Ed25519")
    return der[len(ED25519_SPKI_PREFIX) :]


def candidate_generation_id(candidate: Path) -> str:
    marker = candidate / ".manager-probe-deferred"
    try:
        marker.lstat()
    except FileNotFoundError:
        pass
    except OSError as exc:
        fail(f"inspect deferred Manager probe marker: {exc}")
    else:
        fail("candidate Manager probe is still deferred")
    descriptor = candidate / "generation.meta"
    require_regular_file(descriptor, 64 * 1024, "candidate generation descriptor")
    try:
        data = descriptor.read_bytes()
    except OSError as exc:
        fail(f"read candidate generation descriptor: {exc}")
    if b"\r" in data or not data.endswith(b"\n"):
        fail("candidate generation descriptor line endings are invalid")
    try:
        lines = data.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        fail("candidate generation descriptor is not UTF-8")
    if len(lines) < 2 or lines[0] != "codex-local-generation-v2":
        fail("candidate generation descriptor format is unsupported")
    fields: dict[str, str] = {}
    for line in lines[1:]:
        parts = line.split("\t")
        if len(parts) == 2 and parts[0] not in fields:
            fields[parts[0]] = parts[1]
    generation = fields.get("generation_id", "")
    if not GENERATION_ID.fullmatch(generation):
        fail("candidate generation identity is invalid")
    return generation


def validate_sequence(value: str) -> str:
    if not POSITIVE_DECIMAL.fullmatch(value):
        fail("release sequence is invalid")
    number = int(value)
    if number > (1 << 64) - 1:
        fail("release sequence is outside its bound")
    return value


def validate_release_base(value: str, generation: str) -> str:
    expected_suffix = f"/{generation}/"
    if (
        len(value) > 4096
        or not value.startswith("https://")
        or not value.endswith(expected_suffix)
        or any(ord(char) < 0x21 or ord(char) > 0x7e for char in value)
        or any(char in value for char in "?#\\\\")
    ):
        fail("release base is invalid")
    return value


def write_private_key(runner_temp: Path, secret: str) -> tuple[Path, Path]:
    encoded = secret.encode("utf-8")
    if not encoded or len(encoded) > PRIVATE_KEY_MAX_BYTES:
        fail("release signing secret is absent or outside its byte bound")
    key_dir = Path(tempfile.mkdtemp(prefix=".rald4-signing-", dir=runner_temp))
    key_dir.chmod(0o700)
    key_path = key_dir / "release-key.pem"
    fd = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
    except Exception:
        try:
            os.close(fd)
        except OSError:
            pass
        raise
    if stat.S_IMODE(key_path.lstat().st_mode) != 0o600:
        fail("release signing key file mode is not private")
    return key_dir, key_path


def parse_manifest(path: Path) -> dict[str, str]:
    require_regular_file(path, 128 * 1024, "signed release manifest")
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as exc:
        fail(f"read signed release manifest: {exc}")
    if len(lines) < 10 or lines[0] not in {"codex-release-v3", "codex-release-v4"}:
        fail("signed release manifest format is unsupported")
    result: dict[str, str] = {}
    for line in lines[1:10]:
        parts = line.split("\t")
        if len(parts) != 2 or parts[0] in result or not parts[1]:
            fail("signed release manifest header is malformed")
        result[parts[0]] = parts[1]
    return result


def verify_signature(openssl: Path, public_key: Path, payload: Path, signature: Path) -> None:
    require_regular_file(signature, 64, "candidate signature")
    if signature.stat().st_size != 64:
        fail("candidate signature size is invalid")
    run_quiet(
        [
            str(openssl),
            "pkeyutl",
            "-verify",
            "-pubin",
            "-inkey",
            str(public_key),
            "-rawin",
            "-in",
            str(payload),
            "-sigfile",
            str(signature),
        ],
        "independent candidate signature verification",
    )


def verify_signed_output(
    output: Path,
    generation: str,
    release_sequence: str,
    release_base: str,
    public_raw: bytes,
    accepted_public_key: Path,
    openssl: Path,
) -> None:
    release_dir = output / "releases" / generation
    require_real_directory(output, "signed output")
    require_real_directory(output / "releases", "signed release root")
    require_real_directory(release_dir, "signed generation root")
    manifest = release_dir / "release.manifest"
    signature = release_dir / "release.sig"
    fields = parse_manifest(manifest)
    if fields.get("generation_id") != generation:
        fail("signed release manifest generation does not match candidate")
    if fields.get("release_sequence") != release_sequence:
        fail("signed release manifest sequence does not match request")
    if fields.get("release_public_key") != public_raw.hex():
        fail("signed release manifest public authority does not match accepted key")
    verify_signature(openssl, accepted_public_key, manifest, signature)
    index = output / "update-index-v1"
    index_sig = output / "update-index-v1.sig"
    require_regular_file(index, 16 * 1024, "signed update index")
    expected_index = (
        "codex-update-index-v1\n"
        "channel\tstable\n"
        f"generation_id\t{generation}\n"
        f"release_base\t{release_base}\n"
    ).encode()
    try:
        actual_index = index.read_bytes()
    except OSError as exc:
        fail(f"read signed update index: {exc}")
    if actual_index != expected_index:
        fail("signed update index does not match candidate request")
    verify_signature(openssl, accepted_public_key, index, index_sig)


def sign(args: argparse.Namespace) -> str:
    candidate = require_real_directory(args.candidate, "qualified candidate")
    generation = candidate_generation_id(candidate)
    accepted_public_key = require_regular_file(
        args.accepted_public_key, PUBLIC_KEY_MAX_BYTES, "accepted update public key"
    )
    openssl = require_regular_file(args.openssl, 128 * 1024 * 1024, "OpenSSL", executable=True)
    builder = require_regular_file(
        args.builder, 128 * 1024 * 1024, "release builder", executable=True
    )
    runner_temp = require_real_directory(args.runner_temp, "runner temporary root")
    release_sequence = validate_sequence(args.release_sequence)
    release_base = validate_release_base(args.release_base, generation)
    output = require_absolute(args.output, "signed output")
    try:
        output.lstat()
    except FileNotFoundError:
        pass
    except OSError as exc:
        fail(f"inspect signed output: {exc}")
    else:
        fail("signed output already exists")
    require_real_directory(output.parent, "signed output parent")

    accepted_raw = ed25519_raw_public(openssl, accepted_public_key, private=False)
    secret = os.environ.get(SECRET_NAME)
    if not secret:
        fail("release signing secret is absent")

    key_dir: Path | None = None
    try:
        key_dir, key_path = write_private_key(runner_temp, secret)
        derived_raw = ed25519_raw_public(openssl, key_path, private=True)
        if derived_raw != accepted_raw:
            fail("release signing key does not match accepted update authority")
        try:
            result = subprocess.run(
                [
                    str(builder),
                    "publish",
                    "--generation",
                    str(candidate),
                    "--release-sequence",
                    release_sequence,
                    "--release-base",
                    release_base,
                    "--private-key",
                    str(key_path),
                    "--openssl",
                    str(openssl),
                    "--output",
                    str(output),
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env={},
                check=False,
            )
        except OSError as exc:
            fail(f"candidate signing could not start: {exc}")
        if result.returncode != 0:
            fail("candidate signing failed")
        verify_signed_output(
            output,
            generation,
            release_sequence,
            release_base,
            accepted_raw,
            accepted_public_key,
            openssl,
        )
    finally:
        if key_dir is not None:
            shutil.rmtree(key_dir, ignore_errors=False)
    return generation


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--accepted-public-key", type=Path, required=True)
    parser.add_argument("--release-sequence", required=True)
    parser.add_argument("--release-base", required=True)
    parser.add_argument("--builder", type=Path, required=True)
    parser.add_argument("--openssl", type=Path, required=True)
    parser.add_argument("--runner-temp", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        generation = sign(args)
    except (SigningError, OSError) as exc:
        print(f"rald4 signing: {exc}", file=sys.stderr)
        return 1
    print(f"signed_generation={generation}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
