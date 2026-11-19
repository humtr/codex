#!/usr/bin/env python3
"""Publish and read back the Wrapper release at an exact source commit.

This script is invoked ONLY by the authorized release job, never by verification
or pull-request jobs. In-process gh is injectable for deterministic negative
tests. No retry of an uncertain tag mutation or upload is automatic.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ARCHIVE = "codex-wrapper-aarch64-linux-android.tar.gz"
ASSETS = (ARCHIVE, ARCHIVE + ".sha256", "install.sh", "SHA256SUMS")
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
                 runner=subprocess.run):
        if not REPO.fullmatch(repo) or not TAG.fullmatch(tag) or not HEX.fullmatch(source):
            raise PublicationError("invalid repository, tag, or exact 40-digit source SHA")
        self.repo, self.tag, self.source = repo, tag, source
        self.dist = Path(dist)
        self.dispatch, self.prerelease = dispatch, prerelease
        self.runner = runner
        self.inputs = {}
        for name in ASSETS:
            path = self.dist / name
            if not path.is_file() or path.is_symlink():
                raise PublicationError(f"missing or symlinked release input: {path}")
            self.inputs[name] = sha256(path)
        declared = (self.dist / (ARCHIVE + ".sha256")).read_text(encoding="ascii")
        expected_line = f"{self.inputs[ARCHIVE]}  {ARCHIVE}\n"
        if declared != expected_line:
            raise PublicationError("CI archive SHA256SUMS line does not match actual archive bytes")
        self.notes = (
            f"Termux codex wrapper ({self.tag}).\n\n"
            f"- Source commit: {self.source}\n"
            "- Target: aarch64-linux-android (Android API 24)\n"
            f"- Archive SHA-256: {expected_line}"
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
        for name in ASSETS:
            if names.count(name) != 1:
                raise PublicationError(f"release asset missing or duplicated: {name}")
        with tempfile.TemporaryDirectory(prefix="wrapper-release-readback-") as tmp:
            for name in ASSETS:
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
        # target_commitish is metadata, not the authoritative peeled tag commit.
        target = release.get("target_commitish")
        if target != self.source:
            print(f"WARNING: target_commitish={target!r}; peeled Git tag and bytes "
                  "have separate authoritative SHA verification.", file=sys.stderr)
        return target

    def publish(self):
        self.validate_source()
        release = self.api(f"releases/tags/{self.tag}", allow_404=True)
        current = self.commit_for_ref(allow_404=True)

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
                         *(str(self.dist / name) for name in ASSETS),
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
                         *(str(self.dist / name) for name in ASSETS), *flags)

        self.verify_tag()
        target = self.notes_and_asset_readback()
        self.verify_tag()  # Detect a tag move between upload and readback.
        print(f"RELEASE_VERIFIED repo={self.repo} tag={self.tag} "
              f"source={self.source} archive_sha256={self.inputs[ARCHIVE]} "
              f"target_commitish={target} prerelease={self.prerelease}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--dist", required=True)
    parser.add_argument("--dispatch", action="store_true")
    parser.add_argument("--prerelease", action="store_true")
    args = parser.parse_args()
    try:
        Publication(args.repo, args.tag, args.source_sha, args.dist,
                    dispatch=args.dispatch, prerelease=args.prerelease).publish()
    except (PublicationError, OSError, UnicodeError) as err:
        parser.exit(1, f"wrapper release: {err}\n")


if __name__ == "__main__":
    main()
