#!/usr/bin/env python3
"""Publish and read back the Wrapper release at an exact source commit.

This script is invoked ONLY by the authorized release job, never by verification
or pull-request jobs. In-process gh is injectable for deterministic negative
tests. No retry of an uncertain tag mutation or upload is automatic.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

_SIGN_SPEC = importlib.util.spec_from_file_location(
    "release_sign", Path(__file__).with_name("release_sign.py")
)
release_sign = importlib.util.module_from_spec(_SIGN_SPEC)
_SIGN_SPEC.loader.exec_module(release_sign)

ARCHIVE = "codex-wrapper-aarch64-linux-android.tar.gz"
ASSETS = (ARCHIVE, ARCHIVE + ".sha256", "install.sh", "SHA256SUMS")
SIGNED_EXTRA_ASSETS = ("SHA256SUMS.sig",)
ROTATION_ASSETS = ("release-public-key.pem", "release-public-key.pem.sig")
HEX = re.compile(r"[0-9a-f]{40}\Z")
HASH = re.compile(r"[0-9a-f]{64}\Z")
TAG = re.compile(r"[A-Za-z0-9._-]+\Z")
REPO = re.compile(r"[A-Za-z0-9._-]+/[A-Za-z0-9._-]+\Z")


class PublicationError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as src:
        for block in iter(lambda: src.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


class Publication:
    def __init__(self, repo, tag, source, dist, dispatch=False, prerelease=False,
                 signing_key=None, sequence=None, rotation_key=None, rotation_sig=None,
                 openssl="openssl", runner=subprocess.run, public_key=None,
                 allow_unsigned_migration=False):
        if not REPO.fullmatch(repo) or not TAG.fullmatch(tag) or not HEX.fullmatch(source):
            raise PublicationError("invalid repository, tag, or exact 40-digit source SHA")
        self.repo, self.tag, self.source = repo, tag, source
        self.dist = Path(dist)
        self.dispatch, self.prerelease = dispatch, prerelease
        self.runner = runner
        self.openssl = openssl
        self.trusted_key = Path(public_key) if public_key is not None else (
            Path(__file__).resolve().parents[2] / "release" / "release-public-key.pem"
        )
        self.allow_unsigned_migration = allow_unsigned_migration
        self.signed = signing_key is not None
        self.sequence = sequence
        self.has_rotation = rotation_key is not None
        self.public_key_pem = None
        self.public_key_sha256 = None
        self.assets = list(ASSETS)

        if self.signed:
            if not isinstance(sequence, int) or sequence < 1:
                raise PublicationError("--sequence (>= 1) is required with --signing-key")
            key_path = Path(signing_key)
            try:
                self.public_key_pem = release_sign.derive_public_key_pem(self.openssl, key_path)
                self.public_key_sha256 = hashlib.sha256(self.public_key_pem).hexdigest()
                if (rotation_key is None) != (rotation_sig is None):
                    raise PublicationError(
                        "--rotation-key and --rotation-sig must be provided together"
                    )
                trusted = release_sign.require_regular(self.trusted_key, "trusted public key", release_sign.MAX_PUBKEY_BYTES)
                if rotation_key is None and trusted != self.public_key_pem:
                    raise PublicationError("signing key does not match trusted public key")
                if rotation_key is not None:
                    rot_k = release_sign.require_regular(
                        Path(rotation_key), "rotation public key", release_sign.MAX_PUBKEY_BYTES
                    )
                    if rot_k != self.public_key_pem:
                        raise PublicationError(
                            "--rotation-key does not match the public key derived from --signing-key"
                        )
                    rot_s = release_sign.require_regular(
                        Path(rotation_sig), "rotation signature", release_sign.ED25519_SIG_BYTES
                    )
                    if len(rot_s) != release_sign.ED25519_SIG_BYTES:
                        raise PublicationError(
                            f"--rotation-sig must be {release_sign.ED25519_SIG_BYTES} bytes"
                        )
                    if trusted == rot_k:
                        raise PublicationError("rotation requires the previous trusted public key")
                    release_sign.verify_ed25519_file(self.openssl, self.trusted_key, Path(rotation_key), Path(rotation_sig))
                    for name in ROTATION_ASSETS:
                        release_sign.check_output(self.dist / name)
                    release_sign.atomic_write(self.dist / ROTATION_ASSETS[0], rot_k)
                    release_sign.atomic_write(self.dist / ROTATION_ASSETS[1], rot_s)
                    self.assets.extend(ROTATION_ASSETS)
                release_sign.sign_release(
                    component="wrapper",
                    sequence=sequence,
                    commit=source,
                    dist=self.dist,
                    files=[ARCHIVE, "install.sh"],
                    private_key=key_path,
                    openssl=self.openssl,
                )
                release_sign.verify_release(component="wrapper", dist=self.dist,
                                            public_key=self.trusted_key, min_sequence=sequence,
                                            openssl=self.openssl)
            except release_sign.SigningError as exc:
                raise PublicationError(f"release signing failed: {exc}") from exc
            self.assets.extend(SIGNED_EXTRA_ASSETS)
        elif sequence is not None or rotation_key is not None or rotation_sig is not None:
            raise PublicationError("--sequence and rotation flags require --signing-key")

        self.inputs = {}
        for name in self.assets:
            path = self.dist / name
            if not path.is_file() or path.is_symlink():
                raise PublicationError(f"missing or symlinked release input: {path}")
            self.inputs[name] = sha256(path)
        declared = (self.dist / (ARCHIVE + ".sha256")).read_text(encoding="ascii")
        expected_line = f"{self.inputs[ARCHIVE]}  {ARCHIVE}\n"
        if declared != expected_line:
            raise PublicationError("CI archive SHA256SUMS line does not match actual archive bytes")
        sig_note = (
            f"- Signature: Ed25519 (`SHA256SUMS.sig`, sequence {self.sequence}, "
            f"key sha256 `{self.public_key_sha256}`)\n"
            if self.signed
            else ""
        )
        self.notes = (
            f"Termux codex wrapper ({self.tag}).\n\n"
            f"- Source commit: {self.source}\n"
            "- Target: aarch64-linux-android (Android API 24)\n"
            f"- Archive SHA-256: {expected_line}"
            f"{sig_note}"
            "- Install in Termux:\n"
            f"  curl -fsSL https://github.com/{self.repo}/releases/download/"
            f"{self.tag}/install.sh | sh\n"
        )

    def command(self, *args, allow_404=False):
        argv = ["gh", *args]
        result = self.runner(argv, text=True, capture_output=True)
        if result.returncode:
            if allow_404 and "(HTTP 404)" in result.stderr:
                return None
            err = (result.stderr or result.stdout).strip()[:400]
            raise PublicationError(f"{' '.join(argv[:5])} failed ({result.returncode}): {err}")
        return result.stdout

    def api(self, path, *, allow_404=False):
        raw = self.command("api", f"repos/{self.repo}/{path}", allow_404=allow_404)
        if raw is None:
            return None
        try:
            return json.loads(raw)
        except (ValueError, TypeError) as exc:
            raise PublicationError(f"GitHub API returned malformed JSON for {path}") from exc

    def commit_for_ref(self, *, allow_404=False):
        reference = self.api(f"git/ref/tags/{self.tag}", allow_404=allow_404)
        if reference is None:
            return None
        obj = reference.get("object", {})
        # Annotated tag objects are not commits. Resolve tags recursively,
        # but never re-point a signed/annotated object to an unsigned commit.
        for _ in range(5):
            kind, sha = obj.get("type"), obj.get("sha", "")
            if not HEX.fullmatch(sha):
                raise PublicationError("tag ref contains malformed SHA")
            if kind == "commit":
                return sha, reference["object"].get("type")
            if kind != "tag":
                raise PublicationError(f"tag refers to unsupported Git object type: {kind}")
            tag_object = self.api(f"git/tags/{sha}")
            obj = tag_object.get("object", {})
        raise PublicationError("annotated tag chain exceeds safe resolution depth")

    def verify_tag(self):
        observed = self.commit_for_ref()
        if not observed or observed[0] != self.source:
            raise PublicationError(f"tag commit mismatch: expected {self.source}, got {observed}")
        return observed

    def validate_source(self):
        # Avoid accepting a tag object, tree, or nonexistent source SHA.
        raw = self.api(f"git/commits/{self.source}")
        if not isinstance(raw, dict) or raw.get("sha") != self.source:
            raise PublicationError("publication source is not the verified Git commit")

    def notes_and_asset_readback(self):
        release = self.api(f"releases/tags/{self.tag}")
        if release.get("tag_name") != self.tag:
            raise PublicationError("release tag_name changed during publication")
        if release.get("body") != self.notes:
            raise PublicationError("release notes source/archive SHA mismatch")
        prerelease = bool(release.get("prerelease"))
        if prerelease != self.prerelease:
            raise PublicationError(
                f"release prerelease mismatch: expected {self.prerelease}, got {prerelease}"
            )
        assets = release.get("assets", [])
        names = [asset.get("name") for asset in assets]
        for name in self.assets:
            if names.count(name) != 1:
                raise PublicationError(f"release asset missing or duplicated: {name}")
        if not self.has_rotation:
            for stale in ROTATION_ASSETS:
                if stale in names:
                    raise PublicationError(
                        f"unexpected stale key-rotation asset on non-rotation release: {stale}"
                    )
        with tempfile.TemporaryDirectory(prefix="wrapper-release-readback-") as tmp:
            for name in self.assets:
                # Separate exact requests to avoid unintended wildcard matches.
                self.command("release", "download", self.tag, "--repo", self.repo,
                             "--pattern", name, "--dir", tmp)
                path = Path(tmp) / name
                if not path.is_file() or path.is_symlink():
                    raise PublicationError(f"downloaded release asset is missing: {name}")
                observed_hash = sha256(path)
                if observed_hash != self.inputs[name]:
                    raise PublicationError(f"uploaded release asset bytes differ: {name}")
                asset = next(asset for asset in assets if asset["name"] == name)
                declared_api_hash = asset.get("digest")
                if declared_api_hash and declared_api_hash != f"sha256:{observed_hash}":
                    raise PublicationError(f"release API asset digest mismatch: {name}")
                if asset.get("size") != path.stat().st_size:
                    raise PublicationError(f"release API asset size mismatch: {name}")
            if self.signed:
                try:
                    parsed = release_sign.verify_release(
                        component="wrapper", dist=Path(tmp), public_key=self.trusted_key,
                        min_sequence=self.sequence, openssl=self.openssl,
                    )
                    if parsed["component"] != "wrapper" or parsed["sequence"] != self.sequence or parsed["commit"] != self.source:
                        raise PublicationError("signed readback metadata mismatch")
                    for fname, expected in parsed["files"].items():
                        if sha256(Path(tmp) / fname) != expected:
                            raise PublicationError(f"signed readback digest mismatch: {fname}")
                except release_sign.SigningError as exc:
                    raise PublicationError(f"signed readback verification failed: {exc}") from exc
        # target_commitish is metadata, not the authoritative peeled tag commit.
        target = release.get("target_commitish")
        if target != self.source:
            print(f"WARNING: target_commitish={target!r}; peeled Git tag and bytes "
                  "have separate authoritative SHA verification.", file=sys.stderr)
        return target

    def enforce_monotonic_sequence(self, existing_release):
        if existing_release is None:
            return
        names = [asset.get("name") for asset in existing_release.get("assets", [])]
        if any(names.count(name) > 1 for name in ("SHA256SUMS", "SHA256SUMS.sig", *ROTATION_ASSETS)):
            raise PublicationError("duplicate existing release security assets")
        signed = "SHA256SUMS.sig" in names
        rotated = any(name in names for name in ROTATION_ASSETS)
        if ((signed and "SHA256SUMS" not in names)
                or (rotated and (not signed or not all(name in names for name in ROTATION_ASSETS)))):
            raise PublicationError("incomplete existing signed release")
        if signed and not self.signed:
            raise PublicationError("refusing signed release downgrade")
        if rotated and not self.has_rotation:
            raise PublicationError("existing rotation assets require reconciliation")
        if not signed:
            if "- Signature: Ed25519" in (existing_release.get("body") or ""):
                raise PublicationError("existing release advertises a missing signature")
            if any(name not in ASSETS for name in names):
                raise PublicationError("unrecognized existing unsigned release assets")
            if "SHA256SUMS" in names:
                with tempfile.TemporaryDirectory(prefix="wrapper-legacy-") as tmp:
                    self.command("release", "download", self.tag, "--repo", self.repo,
                                 "--pattern", "SHA256SUMS", "--dir", tmp)
                    raw = release_sign.require_regular(Path(tmp) / "SHA256SUMS", "existing manifest", release_sign.MAX_SUMS_BYTES)
                    if raw.lstrip().startswith(b"#"):
                        raise PublicationError("existing signed-format manifest lacks signature")
            if self.signed and not self.allow_unsigned_migration:
                raise PublicationError("unsigned migration requires explicit approval")
            return
        with tempfile.TemporaryDirectory(prefix="wrapper-release-prev-seq-") as tmp:
            for name in ("SHA256SUMS", "SHA256SUMS.sig", *(ROTATION_ASSETS if rotated else ())):
                self.command("release", "download", self.tag, "--repo", self.repo,
                             "--pattern", name, "--dir", tmp)
            try:
                active_key = self.trusted_key
                if rotated:
                    replacement = Path(tmp) / ROTATION_ASSETS[0]
                    release_sign.require_regular(replacement, "rotation key", release_sign.MAX_PUBKEY_BYTES)
                    release_sign.verify_ed25519_file(self.openssl, active_key, replacement, Path(tmp) / ROTATION_ASSETS[1])
                    active_key = replacement
                prev = release_sign.verify_manifest(self.openssl, active_key, Path(tmp))
            except release_sign.SigningError as exc:
                raise PublicationError(f"existing signed manifest verification failed: {exc}") from exc
            if prev["component"] != "wrapper":
                raise PublicationError("refusing cross-component tag clobber")
            if self.sequence <= prev["sequence"]:
                raise PublicationError(
                    f"signed sequence {self.sequence} must exceed existing published sequence {prev['sequence']}"
                )

    def publish(self):
        self.validate_source()
        release = self.api(f"releases/tags/{self.tag}", allow_404=True)
        current = self.commit_for_ref(allow_404=True)
        self.enforce_monotonic_sequence(release)

        if release is not None:
            if current is None:
                raise PublicationError("existing Release has no Git tag ref")
            if current[0] != self.source:
                if not self.dispatch:
                    raise PublicationError("tag/source mismatch on tag event; refusing upload")
                if current[1] != "commit":
                    raise PublicationError(
                        "annotated tag targets another commit; preserving tag object (manual reconciliation)"
                    )
                # Explicit workflow_dispatch publication may move a lightweight
                # tag; GitHub's ref protection must authorize the mutation.
                self.command("api", "-X", "PATCH",
                             f"repos/{self.repo}/git/refs/tags/{self.tag}",
                             "-f", f"sha={self.source}", "-F", "force=true")
                self.verify_tag()
            self.verify_tag()
            self.command("release", "upload", self.tag,
                         *(str(self.dist / name) for name in self.assets),
                         "--repo", self.repo, "--clobber")
            self.command("release", "edit", self.tag, "--repo", self.repo,
                         "--target", self.source, "--notes", self.notes,
                         "--prerelease" if self.prerelease else "--prerelease=false")
        else:
            if current is not None and current[0] != self.source:
                raise PublicationError("unpublished existing tag differs from source")
            flags = ["--repo", self.repo, "--target", self.source,
                     "--title", f"Termux Codex Wrapper ({self.tag})",
                     "--notes", self.notes]
            if self.prerelease:
                flags.append("--prerelease")
            self.command("release", "create", self.tag,
                         *(str(self.dist / name) for name in self.assets), *flags)

        self.verify_tag()
        target = self.notes_and_asset_readback()
        self.verify_tag()  # Detect a tag move between upload and readback.
        extra = (
            f" signed=True sequence={self.sequence} key_sha256={self.public_key_sha256}"
            if self.signed
            else ""
        )
        print(f"RELEASE_VERIFIED repo={self.repo} tag={self.tag} "
              f"source={self.source} archive_sha256={self.inputs[ARCHIVE]} "
              f"target_commitish={target} prerelease={self.prerelease}{extra}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--dist", required=True)
    parser.add_argument("--dispatch", action="store_true")
    parser.add_argument("--prerelease", action="store_true")
    parser.add_argument("--signing-key", default=None)
    parser.add_argument("--sequence", type=int, default=None)
    parser.add_argument("--public-key", type=Path, default=None)
    parser.add_argument("--allow-unsigned-migration", action="store_true")
    parser.add_argument("--rotation-key", default=None)
    parser.add_argument("--rotation-sig", default=None)
    parser.add_argument("--openssl", default="openssl")
    args = parser.parse_args()
    try:
        Publication(
            args.repo,
            args.tag,
            args.source_sha,
            args.dist,
            dispatch=args.dispatch,
            prerelease=args.prerelease,
            signing_key=args.signing_key,
            sequence=args.sequence,
            rotation_key=args.rotation_key,
            rotation_sig=args.rotation_sig,
            openssl=args.openssl,
            public_key=args.public_key,
            allow_unsigned_migration=args.allow_unsigned_migration,
        ).publish()
    except (PublicationError, release_sign.SigningError, OSError, UnicodeError) as err:
        parser.exit(1, f"wrapper release: {err}\n")


if __name__ == "__main__":
    main()
