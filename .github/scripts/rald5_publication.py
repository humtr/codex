#!/usr/bin/env python3
import argparse
import hashlib
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

GENERATION_RE = re.compile(r"local-[a-z0-9][a-z0-9._-]{0,126}\Z")
HEX64_RE = re.compile(r"[0-9a-f]{64}\Z")
DOWNLOAD_SIZE_FORMAT = "codex-download-size-v1"
DOWNLOAD_SIZE_RELATIVE = "compat/download-size-v1"
DOWNLOAD_SIZE_SIGNATURE_RELATIVE = "compat/download-size-v1.sig"
DOWNLOAD_SIZE_ASSET = "download-size-v1"
DOWNLOAD_SIZE_SIGNATURE_ASSET = "download-size-v1.sig"
EXPECTED_FILES = {
    "codex-code-mode-host": "0755",
    "core": "0755",
    "generation.meta": "0644",
    "helpers/0": "0755",
    "helpers/1": "0755",
    "manager": "0755",
    "runtime": "0755",
}
EXTENDED_FILES = dict(sorted({**EXPECTED_FILES, "helpers/2": "0755"}.items()))
FLAT_ASSETS = {
    "codex-code-mode-host": "codex-code-mode-host",
    "core": "core",
    "generation.meta": "generation.meta",
    "helpers/0": "helper-0",
    "helpers/1": "helper-1",
    "helpers/2": "helper-2",
    "manager": "manager",
    "runtime": "runtime",
}


class ValidationError(Exception):
    pass


def fail(message: str) -> None:
    raise ValidationError(message)


def read_text(path: Path, limit: int) -> str:
    data = path.read_bytes()
    if len(data) > limit:
        fail(f"{path.name} exceeds byte bound")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ValidationError(f"{path.name} is not UTF-8") from exc
    if not text.endswith("\n"):
        fail(f"{path.name} must end with newline")
    return text


def positive_decimal(value: str, label: str) -> int:
    if not value or not value.isascii() or not value.isdigit() or value.startswith("0"):
        fail(f"{label} is not canonical positive decimal")
    parsed = int(value)
    if parsed <= 0 or parsed >= 1 << 64:
        fail(f"{label} is out of range")
    return parsed


def nonnegative_decimal(value: str, label: str) -> int:
    if (
        not value
        or not value.isascii()
        or not value.isdigit()
        or (value.startswith("0") and value != "0")
    ):
        fail(f"{label} is not canonical nonnegative decimal")
    parsed = int(value)
    if parsed >= 1 << 64:
        fail(f"{label} is out of range")
    return parsed


def safe_generation(value: str) -> str:
    if not GENERATION_RE.fullmatch(value):
        fail("generation id is not canonical")
    return value


def canonical_release_base(generation: str) -> str:
    return f"https://humtr.github.io/codex/{generation}/"


def parse_index(path: Path) -> dict[str, str]:
    lines = read_text(path, 16 * 1024).splitlines()
    if len(lines) != 4 or lines[0] != "codex-update-index-v1":
        fail("update index shape is invalid")
    expected = ["channel", "generation_id", "release_base"]
    values: dict[str, str] = {}
    for line, key in zip(lines[1:], expected, strict=True):
        parts = line.split("\t")
        if len(parts) != 2 or parts[0] != key or not parts[1]:
            fail(f"update index {key} record is invalid")
        values[key] = parts[1]
    if values["channel"] != "stable":
        fail("update index channel is not stable")
    generation = safe_generation(values["generation_id"])
    if values["release_base"] != canonical_release_base(generation):
        fail("update index release base is not canonical")
    return values


def parse_manifest(path: Path) -> tuple[dict[str, str], list[tuple[str, str, str]]]:
    lines = read_text(path, 128 * 1024).splitlines()
    if not lines or lines[0] != "codex-release-v4":
        fail("release manifest format is invalid")
    keys = [
        "generation_id",
        "release_sequence",
        "channel",
        "expected_platform",
        "expected_architecture",
        "core_api_identity",
        "persistent_schema_identity",
        "release_public_key",
        "file_count",
    ]
    if len(lines) < 1 + len(keys):
        fail("release manifest is incomplete")
    values: dict[str, str] = {}
    for line, key in zip(lines[1 : 1 + len(keys)], keys, strict=True):
        parts = line.split("\t")
        if len(parts) != 2 or parts[0] != key or not parts[1]:
            fail(f"release manifest {key} record is invalid")
        values[key] = parts[1]
    safe_generation(values["generation_id"])
    positive_decimal(values["release_sequence"], "release sequence")
    if values["channel"] != "stable":
        fail("release manifest channel is not stable")
    if values["expected_platform"] != "android" or values["expected_architecture"] != "aarch64":
        fail("release manifest platform is not Android/AArch64")
    if values["core_api_identity"] != "core-api-v1" or values["persistent_schema_identity"] != "schema-v1":
        fail("release manifest compatibility identity is invalid")
    if not HEX64_RE.fullmatch(values["release_public_key"]):
        fail("release public key is not canonical")
    count = positive_decimal(values["file_count"], "file count")
    records = lines[1 + len(keys) :]
    if len(records) != count or count not in (len(EXPECTED_FILES), len(EXTENDED_FILES)):
        fail("release file count is invalid")
    inventory = EXTENDED_FILES if count == len(EXTENDED_FILES) else EXPECTED_FILES
    files: list[tuple[str, str, str]] = []
    seen: set[str] = set()
    for line in records:
        parts = line.split("\t")
        if len(parts) != 4 or parts[0] != "file":
            fail("release file record is invalid")
        rel, digest, mode = parts[1:]
        if rel in seen or rel not in inventory:
            fail("release file inventory path is invalid")
        if not HEX64_RE.fullmatch(digest) or mode != inventory[rel]:
            fail("release file inventory digest or mode is invalid")
        seen.add(rel)
        files.append((rel, digest, mode))
    if [rel for rel, _, _ in files] != list(inventory):
        fail("release file inventory ordering is invalid")
    return values, files


def run_checked(argv: list[str]) -> None:
    try:
        subprocess.run(argv, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except (OSError, subprocess.CalledProcessError) as exc:
        raise ValidationError("OpenSSL verification failed") from exc


def raw_public_key(openssl: str, public_key: Path, scratch: Path) -> str:
    der = scratch / "public.der"
    run_checked([openssl, "pkey", "-pubin", "-in", str(public_key), "-outform", "DER", "-out", str(der)])
    data = der.read_bytes()
    der.unlink(missing_ok=True)
    prefix = bytes.fromhex("302a300506032b6570032100")
    if len(data) != 44 or not data.startswith(prefix):
        fail("public key is not canonical Ed25519 SPKI")
    return data[len(prefix) :].hex()


def verify_signature(openssl: str, public_key: Path, data: Path, signature: Path) -> None:
    if signature.stat().st_size != 64:
        fail("signature length is invalid")
    run_checked([
        openssl,
        "pkeyutl",
        "-verify",
        "-pubin",
        "-inkey",
        str(public_key),
        "-rawin",
        "-in",
        str(data),
        "-sigfile",
        str(signature),
    ])


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def verify_download_size_sidecar(
    sidecar: Path,
    signature: Path,
    release_dir: Path,
    public_key: Path,
    openssl: str,
    files: list[tuple[str, str, str]],
) -> None:
    require_regular(sidecar)
    require_regular(signature)
    verify_signature(openssl, public_key, sidecar, signature)
    lines = read_text(sidecar, 128 * 1024).splitlines()
    if len(lines) != 4 + len(files) or lines[0] != DOWNLOAD_SIZE_FORMAT:
        fail("download-size sidecar shape is invalid")
    manifest_parts = lines[1].split("\t")
    count_parts = lines[2].split("\t")
    total_parts = lines[3].split("\t")
    if (
        len(manifest_parts) != 2
        or manifest_parts[0] != "manifest_sha256"
        or not HEX64_RE.fullmatch(manifest_parts[1])
        or manifest_parts[1] != sha256(release_dir / "release.manifest")
    ):
        fail("download-size sidecar manifest binding is invalid")
    if (
        len(count_parts) != 2
        or count_parts[0] != "file_count"
        or positive_decimal(count_parts[1], "download-size file count") != len(files)
    ):
        fail("download-size sidecar file count is invalid")
    if len(total_parts) != 2 or total_parts[0] != "total_bytes":
        fail("download-size sidecar total record is invalid")
    declared_total = nonnegative_decimal(total_parts[1], "download-size total")
    if declared_total > 1024 * 1024 * 1024:
        fail("download-size sidecar total exceeds the release byte bound")
    observed_total = 0
    for line, (rel, _, _) in zip(lines[4:], files, strict=True):
        parts = line.split("\t")
        if len(parts) != 3 or parts[0] != "file" or parts[1] != rel:
            fail("download-size sidecar inventory does not match signed manifest")
        declared_size = nonnegative_decimal(parts[2], f"download-size {rel} size")
        actual_size = (release_dir / rel).stat().st_size
        if declared_size != actual_size:
            fail(f"download-size sidecar size does not match {rel}")
        observed_total += declared_size
        if observed_total >= 1 << 64:
            fail("download-size sidecar total is out of range")
    if observed_total != declared_total:
        fail("download-size sidecar total does not match file sizes")


def require_regular(path: Path) -> None:
    if path.is_symlink() or not path.is_file():
        fail(f"{path} must be a regular non-symlink file")


def exact_entries(directory: Path, names: set[str]) -> None:
    if directory.is_symlink() or not directory.is_dir():
        fail(f"{directory} must be a real directory")
    actual = {entry.name for entry in directory.iterdir()}
    if actual != names:
        fail(f"{directory} contains unexpected entries")


def verify_release_tree(
    release_dir: Path,
    index: Path,
    index_sig: Path,
    public_key: Path,
    generation: str,
    sequence: str,
    release_base: str,
    openssl: str,
    scratch: Path,
    index_inside_release: bool = False,
    require_download_size: bool = False,
) -> tuple[dict[str, str], list[tuple[str, str, str]]]:
    generation = safe_generation(generation)
    positive_decimal(sequence, "release sequence")
    if release_base != canonical_release_base(generation):
        fail("requested release base is not canonical")
    for path in [index, index_sig, public_key, release_dir / "release.manifest", release_dir / "release.sig"]:
        require_regular(path)
    verify_signature(openssl, public_key, index, index_sig)
    verify_signature(openssl, public_key, release_dir / "release.manifest", release_dir / "release.sig")
    index_values = parse_index(index)
    manifest_values, files = parse_manifest(release_dir / "release.manifest")
    if index_values["generation_id"] != generation or index_values["release_base"] != release_base:
        fail("update index does not match requested candidate")
    if manifest_values["generation_id"] != generation or manifest_values["release_sequence"] != sequence:
        fail("release manifest does not match requested candidate")
    if manifest_values["release_public_key"] != raw_public_key(openssl, public_key, scratch):
        fail("release manifest authority does not match public key")
    download_size_present = (release_dir / "compat").exists()
    if require_download_size and not download_size_present:
        fail("download-size sidecar is required")
    expected_release_entries = {"release.manifest", "release.sig", "helpers"} | {
        rel for rel, _, _ in files if "/" not in rel
    }
    if download_size_present:
        expected_release_entries.add("compat")
    if index_inside_release:
        expected_release_entries |= {"update-index-v1", "update-index-v1.sig"}
    exact_entries(release_dir, expected_release_entries)
    exact_entries(release_dir / "helpers", {
        rel.split("/")[1] for rel, _, _ in files if rel.startswith("helpers/")
    })
    if download_size_present:
        exact_entries(release_dir / "compat", {"download-size-v1", "download-size-v1.sig"})
        verify_download_size_sidecar(
            release_dir / DOWNLOAD_SIZE_RELATIVE,
            release_dir / DOWNLOAD_SIZE_SIGNATURE_RELATIVE,
            release_dir,
            public_key,
            openssl,
            files,
        )
    for rel, digest, mode in files:
        path = release_dir / rel
        require_regular(path)
        if sha256(path) != digest:
            fail(f"signed digest mismatch for {rel}")
        actual_mode = f"{path.stat().st_mode & 0o7777:04o}"
        if actual_mode != mode:
            fail(f"signed mode mismatch for {rel}")
    return manifest_values, files


def prepare_assets(args: argparse.Namespace) -> None:
    signed_root = Path(args.signed_root).resolve()
    public_key = Path(args.public_key).resolve()
    output = Path(args.output).resolve()
    generation = safe_generation(args.generation)
    if output.exists():
        fail("publication asset output already exists")
    exact_entries(
        signed_root,
        {
            "releases",
            "update-index-v1",
            "update-index-v1.sig",
            DOWNLOAD_SIZE_ASSET,
            DOWNLOAD_SIZE_SIGNATURE_ASSET,
        },
    )
    exact_entries(signed_root / "releases", {generation})
    scratch = signed_root / ".rald5-publication-scratch"
    if scratch.exists():
        fail("publication scratch path already exists")
    scratch.mkdir(mode=0o700)
    try:
        _, files = verify_release_tree(
            signed_root / "releases" / generation,
            signed_root / "update-index-v1",
            signed_root / "update-index-v1.sig",
            public_key,
            generation,
            args.release_sequence,
            args.release_base,
            args.openssl,
            scratch,
        )
        output.mkdir(mode=0o700)
        release_dir = signed_root / "releases" / generation
        verify_download_size_sidecar(
            signed_root / DOWNLOAD_SIZE_ASSET,
            signed_root / DOWNLOAD_SIZE_SIGNATURE_ASSET,
            release_dir,
            public_key,
            args.openssl,
            files,
        )
        for rel, _, _ in files:
            shutil.copyfile(release_dir / rel, output / FLAT_ASSETS[rel])
        shutil.copyfile(release_dir / "release.manifest", output / "release.manifest")
        shutil.copyfile(release_dir / "release.sig", output / "release.sig")
        shutil.copyfile(signed_root / DOWNLOAD_SIZE_ASSET, output / DOWNLOAD_SIZE_ASSET)
        shutil.copyfile(
            signed_root / DOWNLOAD_SIZE_SIGNATURE_ASSET,
            output / DOWNLOAD_SIZE_SIGNATURE_ASSET,
        )
        shutil.copyfile(signed_root / "update-index-v1", output / "candidate-update-index-v1")
        shutil.copyfile(signed_root / "update-index-v1.sig", output / "candidate-update-index-v1.sig")
        shutil.copyfile(public_key, output / "update-public-key.pem")
        exact_entries(output, {FLAT_ASSETS[rel] for rel, _, _ in files} | {
            "release.manifest",
            "release.sig",
            DOWNLOAD_SIZE_ASSET,
            DOWNLOAD_SIZE_SIGNATURE_ASSET,
            "candidate-update-index-v1",
            "candidate-update-index-v1.sig",
            "update-public-key.pem",
        })
        print(f"generation_id={generation}")
        print(f"release_sequence={args.release_sequence}")
        print(f"asset_count={len(list(output.iterdir()))}")
    except Exception:
        if output.exists():
            shutil.rmtree(output)
        raise
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


def restore_assets(args: argparse.Namespace) -> None:
    assets = Path(args.assets).resolve()
    public_key = Path(args.public_key).resolve()
    root = Path(args.root).resolve()
    generation = safe_generation(args.generation)
    destination = root / generation
    if destination.exists() or destination.is_symlink():
        fail("publication generation output already exists")
    require_regular(public_key)
    require_regular(assets / "update-public-key.pem")
    if (assets / "update-public-key.pem").read_bytes() != public_key.read_bytes():
        fail("publication asset authority does not match trusted public key")
    require_regular(assets / "release.manifest")
    _, files = parse_manifest(assets / "release.manifest")
    metadata = {
        "release.manifest": "release.manifest",
        "release.sig": "release.sig",
        DOWNLOAD_SIZE_ASSET: DOWNLOAD_SIZE_RELATIVE,
        DOWNLOAD_SIZE_SIGNATURE_ASSET: DOWNLOAD_SIZE_SIGNATURE_RELATIVE,
        "candidate-update-index-v1": "update-index-v1",
        "candidate-update-index-v1.sig": "update-index-v1.sig",
    }
    expected = {FLAT_ASSETS[rel] for rel, _, _ in files} | set(metadata) | {"update-public-key.pem"}
    exact_entries(assets, expected)
    for name in expected:
        require_regular(assets / name)
    root.mkdir(parents=True, exist_ok=True)
    destination.mkdir(mode=0o700)
    try:
        for rel, _, mode in files:
            path = destination / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(assets / FLAT_ASSETS[rel], path)
            path.chmod(int(mode, 8))
        for name, rel in metadata.items():
            path = destination / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(assets / name, path)
            path.chmod(0o644)
        with tempfile.TemporaryDirectory(prefix="publication-verify-", dir=root) as temporary:
            verify_release_tree(
                destination, destination / "update-index-v1",
                destination / "update-index-v1.sig", public_key,
                generation, args.release_sequence, args.release_base, args.openssl,
                Path(temporary), index_inside_release=True, require_download_size=True,
            )
        print(f"verified_generation={generation}")
        print(f"verified_sequence={args.release_sequence}")
    except Exception:
        shutil.rmtree(destination)
        raise


def verify_public(args: argparse.Namespace) -> None:
    root = Path(args.root).resolve()
    public_key = Path(args.public_key).resolve()
    generation = safe_generation(args.generation)
    release_dir = root / generation
    scratch = root / ".rald5-publication-scratch"
    if scratch.exists():
        fail("publication scratch path already exists")
    scratch.mkdir(mode=0o700)
    try:
        verify_release_tree(
            release_dir,
            release_dir / "update-index-v1",
            release_dir / "update-index-v1.sig",
            public_key,
            generation,
            args.release_sequence,
            args.release_base,
            args.openssl,
            scratch,
            index_inside_release=True,
            require_download_size=args.require_download_size,
        )
        print(f"verified_generation={generation}")
        print(f"verified_sequence={args.release_sequence}")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--generation", required=True)
    common.add_argument("--release-sequence", required=True)
    common.add_argument("--release-base", required=True)
    common.add_argument("--public-key", required=True)
    common.add_argument("--openssl", default="openssl")
    prepare = sub.add_parser("prepare-assets", parents=[common])
    prepare.add_argument("--signed-root", required=True)
    prepare.add_argument("--output", required=True)
    restore = sub.add_parser("restore-assets", parents=[common])
    restore.add_argument("--assets", required=True)
    restore.add_argument("--root", required=True)
    verify = sub.add_parser("verify-public", parents=[common])
    verify.add_argument("--root", required=True)
    verify.add_argument("--require-download-size", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.command == "prepare-assets":
            prepare_assets(args)
        elif args.command == "restore-assets":
            restore_assets(args)
        elif args.command == "verify-public":
            verify_public(args)
        else:
            raise AssertionError(args.command)
    except (ValidationError, OSError) as exc:
        print(f"rald5 publication: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
