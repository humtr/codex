#!/usr/bin/env python3
"""Fail-closed signed GitHub Release publisher for Stage 2 standalone components.

Mirrors the safety invariants of `.github/scripts/publish_wrapper_release.py`:
- Signs `SHA256SUMS` + `SHA256SUMS.sig` with Ed25519 and verifies locally before any upload.
- Checks existing GitHub Releases via API (distinguishing 404 from API errors).
- Fails closed if an existing release belongs to a different component (prevents cross-component tag clobbering).
- Fails closed if the existing release's `SHA256SUMS` cannot be downloaded/verified or if `sequence <= previous_sequence`.
- Moves lightweight Git tag refs cleanly on `workflow_dispatch` re-releases (refusing to overwrite annotated tags).
- Performs mandatory post-upload readback verification against the pinned repository public key.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

HEX_SHA = re.compile(r"^[0-9a-f]{40}$")


class Stage2PublicationError(RuntimeError):
    pass


def _load_release_sign():
    script = Path(__file__).resolve().with_name("release_sign.py")
    spec = importlib.util.spec_from_file_location("release_sign", script)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    spec.loader.exec_module(mod)
    return mod


release_sign = _load_release_sign()


class Stage2Publisher:
    def __init__(
        self,
        *,
        repo: str,
        tag: str,
        component: str,
        source_sha: str,
        dist: Path,
        tarball: str,
        title: str,
        notes: str,
        sequence: int,
        signing_key: Path,
        public_key: Path,
        prerelease: bool = False,
        runner=None,
    ):
        if not HEX_SHA.fullmatch(source_sha):
            raise Stage2PublicationError(f"invalid source commit SHA: {source_sha!r}")
        if sequence < 1:
            raise Stage2PublicationError(f"sequence must be >= 1, got {sequence}")
        self.repo = repo
        self.tag = tag
        self.component = component
        self.source_sha = source_sha
        self.dist = dist
        self.tarball = tarball
        self.title = title
        self.notes = notes
        self.sequence = sequence
        self.signing_key = signing_key
        self.public_key = public_key
        self.prerelease = prerelease
        self._runner = runner

    def _run_gh(self, *args: str, allow_404: bool = False) -> str | None:
        argv = ["gh", *args]
        if self._runner is not None:
            res = self._runner(argv)
        else:
            res = subprocess.run(argv, capture_output=True, text=True)
        if res.returncode != 0:
            combined = f"{res.stderr or ''}\n{res.stdout or ''}"
            if allow_404 and ("404" in combined or "Not Found" in combined or "release not found" in combined):
                return None
            raise Stage2PublicationError(
                f"{' '.join(argv[:5])} failed ({res.returncode}): {combined.strip()[:400]}"
            )
        return res.stdout

    def _api_json(self, path: str, *, allow_404: bool = False) -> dict | None:
        raw = self._run_gh("api", f"repos/{self.repo}/{path}", allow_404=allow_404)
        if raw is None:
            return None
        try:
            data = json.loads(raw)
        except (ValueError, TypeError) as exc:
            raise Stage2PublicationError(f"malformed GitHub API JSON for {path}") from exc
        if not isinstance(data, dict):
            raise Stage2PublicationError(f"unexpected GitHub API response type for {path}")
        return data

    def _resolve_tag_commit(self, *, allow_404: bool = False) -> tuple[str, str] | None:
        ref = self._api_json(f"git/ref/tags/{self.tag}", allow_404=allow_404)
        if ref is None:
            return None
        obj = ref.get("object", {})
        root_type = obj.get("type", "")
        for _ in range(5):
            kind, sha = obj.get("type"), obj.get("sha", "")
            if not HEX_SHA.fullmatch(sha):
                raise Stage2PublicationError(f"tag ref contains malformed SHA: {sha!r}")
            if kind == "commit":
                return sha, root_type
            if kind != "tag":
                raise Stage2PublicationError(f"unsupported Git tag object type: {kind!r}")
            tag_obj = self._api_json(f"git/tags/{sha}")
            assert tag_obj is not None
            obj = tag_obj.get("object", {})
        raise Stage2PublicationError("annotated tag chain exceeded max depth")

    def enforce_existing_release_guards(self, existing: dict) -> None:
        asset_names = {a.get("name") for a in existing.get("assets", [])}
        if "SHA256SUMS" not in asset_names:
            raise Stage2PublicationError(
                f"existing release '{self.tag}' is missing SHA256SUMS; refusing to clobber unverified release"
            )
        with tempfile.TemporaryDirectory(prefix="stage2-prev-seq-") as tmp:
            self._run_gh(
                "release",
                "download",
                self.tag,
                "--repo",
                self.repo,
                "--pattern",
                "SHA256SUMS",
                "--dir",
                tmp,
            )
            sums_path = Path(tmp) / "SHA256SUMS"
            if not sums_path.is_file() or sums_path.is_symlink():
                raise Stage2PublicationError("failed to read downloaded existing SHA256SUMS")
            try:
                prev = release_sign.parse_sums(sums_path.read_bytes())
            except release_sign.SigningError as exc:
                raise Stage2PublicationError(
                    f"existing release SHA256SUMS is invalid: {exc}"
                ) from exc
            if prev["component"] != self.component:
                raise Stage2PublicationError(
                    f"refusing cross-component tag clobber: tag '{self.tag}' belongs to "
                    f"component '{prev['component']}', expected '{self.component}'"
                )
            if self.sequence <= prev["sequence"]:
                raise Stage2PublicationError(
                    f"{self.component} sequence {self.sequence} must exceed published sequence {prev['sequence']}"
                )

    def publish(self) -> None:
        tar_path = self.dist / self.tarball
        sha_path = self.dist / f"{self.tarball}.sha256"
        if not tar_path.is_file() or not sha_path.is_file():
            raise Stage2PublicationError(f"missing release artifacts in {self.dist}")

        # 1. Sign and locally verify BEFORE touching GitHub.
        release_sign.sign_release(
            component=self.component,
            sequence=self.sequence,
            commit=self.source_sha,
            dist=self.dist,
            files=[self.tarball],
            private_key=self.signing_key,
        )
        release_sign.verify_release(
            component=self.component,
            dist=self.dist,
            public_key=self.public_key,
            min_sequence=self.sequence,
        )

        assets = [
            str(self.dist / self.tarball),
            str(self.dist / f"{self.tarball}.sha256"),
            str(self.dist / "SHA256SUMS"),
            str(self.dist / "SHA256SUMS.sig"),
        ]

        # 2. Check existing release & tag ref fail-closed.
        existing = self._api_json(f"releases/tags/{self.tag}", allow_404=True)
        current_ref = self._resolve_tag_commit(allow_404=True)

        if existing is not None:
            self.enforce_existing_release_guards(existing)
            if current_ref is None:
                raise Stage2PublicationError(f"existing release '{self.tag}' has no Git tag ref")
            if current_ref[0] != self.source_sha:
                if current_ref[1] != "commit":
                    raise Stage2PublicationError(
                        f"annotated tag '{self.tag}' targets {current_ref[0]}; refusing overwrite"
                    )
                self._run_gh(
                    "api",
                    "-X",
                    "PATCH",
                    f"repos/{self.repo}/git/refs/tags/{self.tag}",
                    "-f",
                    f"sha={self.source_sha}",
                    "-F",
                    "force=true",
                )
            self._run_gh("release", "upload", self.tag, *assets, "--repo", self.repo, "--clobber")
            edit_args = [
                "release",
                "edit",
                self.tag,
                "--repo",
                self.repo,
                "--target",
                self.source_sha,
                "--notes",
                self.notes,
                "--prerelease" if self.prerelease else "--prerelease=false",
            ]
            self._run_gh(*edit_args)
        else:
            if current_ref is not None and current_ref[0] != self.source_sha:
                raise Stage2PublicationError(
                    f"unpublished existing tag '{self.tag}' targets {current_ref[0]} != {self.source_sha}"
                )
            create_args = [
                "release",
                "create",
                self.tag,
                *assets,
                "--repo",
                self.repo,
                "--target",
                self.source_sha,
                "--title",
                self.title,
                "--notes",
                self.notes,
            ]
            if self.prerelease:
                create_args.append("--prerelease")
            self._run_gh(*create_args)

        # 3. Verify peeled tag ref & mandatory post-upload signed readback.
        verified_ref = self._resolve_tag_commit(allow_404=False)
        if not verified_ref or verified_ref[0] != self.source_sha:
            raise Stage2PublicationError(
                f"post-publish tag commit mismatch: expected {self.source_sha}, got {verified_ref}"
            )
        with tempfile.TemporaryDirectory(prefix="stage2-readback-") as rb:
            for name in (self.tarball, "SHA256SUMS", "SHA256SUMS.sig"):
                self._run_gh(
                    "release",
                    "download",
                    self.tag,
                    "--repo",
                    self.repo,
                    "--pattern",
                    name,
                    "--dir",
                    rb,
                )
            release_sign.verify_release(
                component=self.component,
                dist=Path(rb),
                public_key=self.public_key,
                min_sequence=self.sequence,
            )
        print(
            f"STAGE2_RELEASE_VERIFIED component={self.component} tag={self.tag} "
            f"commit={self.source_sha} sequence={self.sequence}"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--component", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--tarball", required=True)
    parser.add_argument("--title", required=True)
    parser.add_argument("--notes", required=True)
    parser.add_argument("--sequence", type=int, required=True)
    parser.add_argument("--signing-key", type=Path, required=True)
    parser.add_argument("--public-key", type=Path, required=True)
    parser.add_argument("--prerelease", action="store_true")
    args = parser.parse_args(argv)
    try:
        Stage2Publisher(
            repo=args.repo,
            tag=args.tag,
            component=args.component,
            source_sha=args.source_sha,
            dist=args.dist,
            tarball=args.tarball,
            title=args.title,
            notes=args.notes,
            sequence=args.sequence,
            signing_key=args.signing_key,
            public_key=args.public_key,
            prerelease=args.prerelease,
        ).publish()
        return 0
    except (Stage2PublicationError, release_sign.SigningError, OSError) as err:
        print(f"stage2 release: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
