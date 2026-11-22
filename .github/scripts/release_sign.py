#!/usr/bin/env python3
"""Universal lightweight Ed25519 release signing and verification (SHA256SUMS.sig).

Shared by Wrapper (`codex`) and all independent Stage 2 components (`codex-ext`,
`termux-shell-mcp`, `termux-codex-mcp`, `chatgpt-provider`, `termux-chatgpt-mcp`,
`tunnel-client`). Uses only Python stdlib and OpenSSL `pkeyutl` (Ed25519).
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

FORMAT_ID = "codex-release-sums-v1"
COMPONENT_RE = re.compile(r"[a-z0-9][a-z0-9._-]{0,63}\Z")
COMMIT_RE = re.compile(r"[0-9a-f]{40}\Z")
HEX64_RE = re.compile(r"[0-9a-f]{64}\Z")
SEQ_RE = re.compile(r"[1-9][0-9]{0,18}\Z")
FILENAME_RE = re.compile(r"[A-Za-z0-9._-]{1,128}\Z")

MAX_SUMS_BYTES = 64 * 1024
MAX_PUBKEY_BYTES = 4 * 1024
ED25519_SIG_BYTES = 64


class SigningError(RuntimeError):
    pass


def require_regular(path: Path, label: str, max_bytes: int | None = None) -> bytes:
    if path.is_symlink() or not path.is_file():
        raise SigningError(f"{label} must be a regular non-symlink file: {path}")
    size = path.stat().st_size
    if size <= 0:
        raise SigningError(f"{label} must not be empty: {path}")
    if max_bytes is not None and size > max_bytes:
        raise SigningError(f"{label} exceeds {max_bytes} bytes: {path}")
    return path.read_bytes()


def sha256_file(path: Path) -> str:
    if path.is_symlink() or not path.is_file():
        raise SigningError(f"asset must be a regular non-symlink file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_openssl(openssl: str, argv: list[str], label: str) -> bytes:
    result = subprocess.run(
        [openssl, *argv],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        err = result.stderr.decode("utf-8", "replace").strip()[:300]
        raise SigningError(f"openssl {label} failed ({result.returncode}): {err}")
    return result.stdout


def derive_public_key_pem(openssl: str, private_key: Path) -> bytes:
    require_regular(private_key, "private key", MAX_PUBKEY_BYTES)
    pub = run_openssl(
        openssl,
        ["pkey", "-in", str(private_key), "-pubout"],
        "derive public key",
    )
    if b"BEGIN PUBLIC KEY" not in pub or len(pub) > MAX_PUBKEY_BYTES:
        raise SigningError("derived public key PEM is invalid")
    return pub


def verify_ed25519_file(openssl: str, pubkey: Path, data_path: Path, sig_path: Path) -> None:
    sig = require_regular(sig_path, "signature", ED25519_SIG_BYTES)
    if len(sig) != ED25519_SIG_BYTES:
        raise SigningError(
            f"Ed25519 signature must be {ED25519_SIG_BYTES} bytes, got {len(sig)}"
        )
    require_regular(pubkey, "public key", MAX_PUBKEY_BYTES)
    run_openssl(
        openssl,
        [
            "pkeyutl",
            "-verify",
            "-pubin",
            "-inkey",
            str(pubkey),
            "-rawin",
            "-in",
            str(data_path),
            "-sigfile",
            str(sig_path),
        ],
        "Ed25519 verify",
    )


def sign_ed25519_file(openssl: str, privkey: Path, data_path: Path, sig_out: Path) -> None:
    require_regular(privkey, "private key", MAX_PUBKEY_BYTES)
    run_openssl(
        openssl,
        [
            "pkeyutl",
            "-sign",
            "-inkey",
            str(privkey),
            "-rawin",
            "-in",
            str(data_path),
            "-out",
            str(sig_out),
        ],
        "Ed25519 sign",
    )
    sig = require_regular(sig_out, "generated signature", ED25519_SIG_BYTES)
    if len(sig) != ED25519_SIG_BYTES:
        raise SigningError(f"generated Ed25519 signature has unexpected length {len(sig)}")


def validate_filename(name: str) -> str:
    if not FILENAME_RE.fullmatch(name) or name in (".", ".."):
        raise SigningError(f"invalid release asset filename: {name!r}")
    return name


def render_sums(component: str, sequence: int, commit: str, entries: dict[str, str]) -> bytes:
    if not COMPONENT_RE.fullmatch(component):
        raise SigningError(f"invalid component name: {component!r}")
    if not isinstance(sequence, int) or sequence < 1:
        raise SigningError(f"sequence must be a positive integer, got {sequence!r}")
    if not COMMIT_RE.fullmatch(commit):
        raise SigningError(f"commit must be 40 lowercase hex chars, got {commit!r}")
    if not entries:
        raise SigningError("release must list at least one asset")
    lines = [
        f"# format={FORMAT_ID}",
        f"# component={component}",
        f"# sequence={sequence}",
        f"# commit={commit}",
    ]
    for name in sorted(entries):
        validate_filename(name)
        digest = entries[name]
        if not HEX64_RE.fullmatch(digest):
            raise SigningError(f"invalid SHA-256 for {name}: {digest!r}")
        lines.append(f"{digest}  {name}")
    payload = ("\n".join(lines) + "\n").encode("ascii")
    if len(payload) > MAX_SUMS_BYTES:
        raise SigningError("rendered SHA256SUMS exceeds size bound")
    return payload


def parse_sums(raw: bytes) -> dict:
    if len(raw) == 0 or len(raw) > MAX_SUMS_BYTES:
        raise SigningError("SHA256SUMS is empty or exceeds size bound")
    try:
        text = raw.decode("ascii")
    except UnicodeDecodeError as exc:
        raise SigningError("SHA256SUMS must be ASCII") from exc
    if not text.endswith("\n") or "\r" in text:
        raise SigningError("SHA256SUMS must use LF line endings")
    lines = text.splitlines()
    if len(lines) < 5:
        raise SigningError("SHA256SUMS missing required headers or entries")

    expected_headers = ("format", "component", "sequence", "commit")
    headers = {}
    for idx, key in enumerate(expected_headers):
        prefix = f"# {key}="
        if not lines[idx].startswith(prefix):
            raise SigningError(f"SHA256SUMS line {idx + 1} must start with {prefix!r}")
        headers[key] = lines[idx][len(prefix):]

    if headers["format"] != FORMAT_ID:
        raise SigningError(f"unsupported SHA256SUMS format: {headers['format']!r}")
    if not COMPONENT_RE.fullmatch(headers["component"]):
        raise SigningError(f"invalid component header: {headers['component']!r}")
    if not SEQ_RE.fullmatch(headers["sequence"]):
        raise SigningError(f"invalid sequence header: {headers['sequence']!r}")
    if not COMMIT_RE.fullmatch(headers["commit"]):
        raise SigningError(f"invalid commit header: {headers['commit']!r}")

    entries: dict[str, str] = {}
    for line_no, line in enumerate(lines[4:], start=5):
        if line.startswith("#") or "  " not in line:
            raise SigningError(f"malformed SHA256SUMS line {line_no}")
        digest, name = line.split("  ", 1)
        if not HEX64_RE.fullmatch(digest):
            raise SigningError(f"malformed SHA-256 on line {line_no}")
        validate_filename(name)
        if name in entries:
            raise SigningError(f"duplicate SHA256SUMS entry: {name}")
        entries[name] = digest

    if not entries:
        raise SigningError("SHA256SUMS lists zero files")
    return {
        "format": headers["format"],
        "component": headers["component"],
        "sequence": int(headers["sequence"]),
        "commit": headers["commit"],
        "files": entries,
    }


def sign_release(
    *,
    component: str,
    sequence: int,
    commit: str,
    dist: Path,
    files: list[str],
    private_key: Path,
    openssl: str = "openssl",
) -> dict:
    if not dist.is_dir() or dist.is_symlink():
        raise SigningError(f"dist must be a real directory: {dist}")
    entries: dict[str, str] = {}
    for name in files:
        validate_filename(name)
        if name in ("SHA256SUMS", "SHA256SUMS.sig"):
            raise SigningError(f"cannot include {name} inside SHA256SUMS")
        if name in entries:
            raise SigningError(f"duplicate file argument: {name}")
        entries[name] = sha256_file(dist / name)

    sums_bytes = render_sums(component, sequence, commit, entries)
    sums_path = dist / "SHA256SUMS"
    sig_path = dist / "SHA256SUMS.sig"
    sums_path.write_bytes(sums_bytes)
    sign_ed25519_file(openssl, private_key, sums_path, sig_path)

    # Immediately self-verify using derived public key.
    pub_pem = derive_public_key_pem(openssl, private_key)
    with tempfile.TemporaryDirectory(prefix="codex-sign-selfverify-") as tmp:
        pub_path = Path(tmp) / "pub.pem"
        pub_path.write_bytes(pub_pem)
        verified = verify_release(
            component=component,
            dist=dist,
            public_key=pub_path,
            min_sequence=sequence,
            expected_key_sha256=hashlib.sha256(pub_pem).hexdigest(),
            openssl=openssl,
        )
    return verified


def sign_key_rotation(
    *,
    old_private_key: Path,
    new_public_key: Path,
    output_sig: Path,
    openssl: str = "openssl",
) -> str:
    new_pub = require_regular(new_public_key, "new public key", MAX_PUBKEY_BYTES)
    if b"BEGIN PUBLIC KEY" not in new_pub:
        raise SigningError("new public key must be a PEM public key")
    sign_ed25519_file(openssl, old_private_key, new_public_key, output_sig)
    old_pub = derive_public_key_pem(openssl, old_private_key)
    with tempfile.TemporaryDirectory(prefix="codex-rot-selfverify-") as tmp:
        old_pub_path = Path(tmp) / "old.pem"
        old_pub_path.write_bytes(old_pub)
        verify_ed25519_file(openssl, old_pub_path, new_public_key, output_sig)
    return hashlib.sha256(new_pub).hexdigest()


def verify_release(
    *,
    component: str,
    dist: Path,
    public_key: Path,
    min_sequence: int = 1,
    expected_key_sha256: str | None = None,
    openssl: str = "openssl",
) -> dict:
    if not COMPONENT_RE.fullmatch(component):
        raise SigningError(f"invalid expected component: {component!r}")
    if not dist.is_dir() or dist.is_symlink():
        raise SigningError(f"dist must be a real directory: {dist}")
    pub_bytes = require_regular(public_key, "public key", MAX_PUBKEY_BYTES)
    pinned_sha = hashlib.sha256(pub_bytes).hexdigest()
    if expected_key_sha256 is not None:
        if not HEX64_RE.fullmatch(expected_key_sha256):
            raise SigningError("expected_key_sha256 must be 64 lowercase hex chars")
        if pinned_sha != expected_key_sha256:
            raise SigningError(
                f"public key SHA-256 mismatch: expected {expected_key_sha256}, got {pinned_sha}"
            )

    active_key = public_key
    active_key_sha = pinned_sha
    rotated = False
    rot_key = dist / "release-public-key.pem"
    rot_sig = dist / "release-public-key.pem.sig"
    if rot_key.exists() or rot_key.is_symlink() or rot_sig.exists() or rot_sig.is_symlink():
        new_pub = require_regular(rot_key, "rotated public key", MAX_PUBKEY_BYTES)
        if b"BEGIN PUBLIC KEY" not in new_pub:
            raise SigningError("rotated public key is not a PEM public key")
        new_pub_sha = hashlib.sha256(new_pub).hexdigest()
        if new_pub_sha == pinned_sha:
            # Verifier already trusts the rotated key; allow idempotent re-verification
            # while the release still carries the old-key -> new-key rotation signature.
            active_key = public_key
            active_key_sha = pinned_sha
            rotated = False
        else:
            verify_ed25519_file(openssl, public_key, rot_key, rot_sig)
            active_key = rot_key
            active_key_sha = new_pub_sha
            rotated = True

    sums_path = dist / "SHA256SUMS"
    sig_path = dist / "SHA256SUMS.sig"
    sums_raw = require_regular(sums_path, "SHA256SUMS", MAX_SUMS_BYTES)
    verify_ed25519_file(openssl, active_key, sums_path, sig_path)
    parsed = parse_sums(sums_raw)

    if parsed["component"] != component:
        raise SigningError(
            f"signed component mismatch: expected {component!r}, got {parsed['component']!r}"
        )
    if parsed["sequence"] < min_sequence:
        raise SigningError(
            f"signed sequence rollback rejected: got {parsed['sequence']}, minimum {min_sequence}"
        )
    for name, expected_digest in parsed["files"].items():
        actual = sha256_file(dist / name)
        if actual != expected_digest:
            raise SigningError(
                f"asset SHA-256 mismatch for {name}: expected {expected_digest}, got {actual}"
            )

    return {
        **parsed,
        "rotated": rotated,
        "active_key_sha256": active_key_sha,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_sign = sub.add_parser("sign", help="generate and sign SHA256SUMS + SHA256SUMS.sig")
    p_sign.add_argument("--component", required=True)
    p_sign.add_argument("--sequence", required=True, type=int)
    p_sign.add_argument("--commit", required=True)
    p_sign.add_argument("--dist", required=True, type=Path)
    p_sign.add_argument("--files", required=True, nargs="+")
    p_sign.add_argument("--private-key", required=True, type=Path)
    p_sign.add_argument("--openssl", default="openssl")

    p_rot = sub.add_parser("sign-rotation", help="sign a replacement Ed25519 public key")
    p_rot.add_argument("--old-private-key", required=True, type=Path)
    p_rot.add_argument("--new-public-key", required=True, type=Path)
    p_rot.add_argument("--output", required=True, type=Path)
    p_rot.add_argument("--openssl", default="openssl")

    p_ver = sub.add_parser("verify", help="verify SHA256SUMS.sig, sequence, and asset digests")
    p_ver.add_argument("--component", required=True)
    p_ver.add_argument("--dist", required=True, type=Path)
    p_ver.add_argument("--public-key", required=True, type=Path)
    p_ver.add_argument("--min-sequence", type=int, default=1)
    p_ver.add_argument("--expected-key-sha256", default=None)
    p_ver.add_argument("--openssl", default="openssl")

    args = parser.parse_args(argv)
    try:
        if args.cmd == "sign":
            res = sign_release(
                component=args.component,
                sequence=args.sequence,
                commit=args.commit,
                dist=args.dist,
                files=args.files,
                private_key=args.private_key,
                openssl=args.openssl,
            )
            print(json.dumps(res, sort_keys=True))
        elif args.cmd == "sign-rotation":
            key_sha = sign_key_rotation(
                old_private_key=args.old_private_key,
                new_public_key=args.new_public_key,
                output_sig=args.output,
                openssl=args.openssl,
            )
            print(json.dumps({"new_key_sha256": key_sha}, sort_keys=True))
        elif args.cmd == "verify":
            res = verify_release(
                component=args.component,
                dist=args.dist,
                public_key=args.public_key,
                min_sequence=args.min_sequence,
                expected_key_sha256=args.expected_key_sha256,
                openssl=args.openssl,
            )
            print(json.dumps(res, sort_keys=True))
    except (SigningError, OSError) as err:
        print(f"release_sign: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
