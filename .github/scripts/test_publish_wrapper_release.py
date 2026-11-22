#!/usr/bin/env python3
"""Fully offline GitHub API/gh negative tests: never creates a real Release."""
from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import hashlib
from io import StringIO
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest


spec = importlib.util.spec_from_file_location(
    "publish_wrapper_release", Path(__file__).with_name("publish_wrapper_release.py")
)
publish = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publish)

SOURCE = "a" * 40
OTHER = "b" * 40
OBJECT = "c" * 40
REPO = "humtr/codex"
TAG = "wrapper"


class FakeGh:
    def __init__(self, *, source=SOURCE, existing=True, annotated=False,
                 prerelease=False, fault=None):
        self.source = source
        self.ref = source if existing else None
        self.annotated = annotated
        self.fault = fault
        self.release = (
            {"tag_name": TAG, "target_commitish": OTHER, "body": "prior release",
             "prerelease": prerelease}
            if existing else None
        )
        self.assets = {}
        self.calls = []
        self.ref_reads = 0

    def _api_data(self, path):
        if path.endswith("/git/commits/" + SOURCE):
            if self.fault == "source-not-commit":
                return {"sha": OTHER}
            return {"sha": SOURCE}
        if path.endswith("/releases/tags/" + TAG):
            if self.release is None:
                return None
            return {
                **self.release,
                "assets": [
                    {"name": name, "size": len(data),
                     "digest": "sha256:" + hashlib.sha256(data).hexdigest()}
                    for name, data in self.assets.items()
                ],
            }
        if path.endswith("/git/ref/tags/" + TAG):
            self.ref_reads += 1
            if self.fault == "tag-lookup-error":
                raise RuntimeError("500")
            if self.ref is None:
                return None
            if self.fault == "drift-on-final-ref" and self.ref_reads >= 3:
                self.ref = OTHER
            if self.annotated:
                return {"object": {"sha": OBJECT, "type": "tag"}}
            return {"object": {"sha": self.ref, "type": "commit"}}
        if path.endswith("/git/tags/" + OBJECT):
            return {"object": {"sha": self.ref, "type": "commit"}}
        raise AssertionError(f"unexpected gh API read: {path}")

    def __call__(self, argv, *, text, capture_output):
        assert argv[0] == "gh"
        self.calls.append(argv)
        if argv[1] == "api":
            if "-X" in argv:
                assert argv[2:4] == ["-X", "PATCH"]
                if self.fault == "patch-error":
                    return SimpleNamespace(returncode=1, stdout="", stderr="gh: denied (HTTP 422)")
                if self.fault != "patch-ignored":
                    self.ref = argv[argv.index("-f") + 1].split("=", 1)[-1]
                return SimpleNamespace(returncode=0, stdout="{}", stderr="")
            path = argv[2]
            try:
                body = self._api_data(path)
            except RuntimeError:
                return SimpleNamespace(returncode=1, stdout="", stderr="gh: unavailable (HTTP 500)")
            if body is None:
                return SimpleNamespace(returncode=1, stdout="", stderr="gh: Not Found (HTTP 404)")
            import json
            return SimpleNamespace(returncode=0, stdout=json.dumps(body), stderr="")
        if argv[1:3] == ["release", "upload"]:
            for path in argv[4:]:
                if path.startswith("--"):
                    break
                file = Path(path)
                self.assets[file.name] = file.read_bytes()
                if self.fault == "partial-upload":
                    return SimpleNamespace(returncode=1, stdout="", stderr="gh: partial upload")
            if self.fault == "concurrent-tag-change":
                self.ref = OTHER
            return SimpleNamespace(returncode=0, stdout="", stderr="")
        if argv[1:3] == ["release", "edit"]:
            self.release["body"] = argv[argv.index("--notes") + 1]
            if self.fault == "notes-mismatch":
                self.release["body"] = "incorrect source/asset"
            if self.fault != "stale-target":
                self.release["target_commitish"] = argv[argv.index("--target") + 1]
            if self.fault != "prerelease-edit-ignored":
                if "--prerelease" in argv:
                    self.release["prerelease"] = True
                elif "--prerelease=false" in argv:
                    self.release["prerelease"] = False
            return SimpleNamespace(returncode=0, stdout="", stderr="")
        if argv[1:3] == ["release", "create"]:
            self.ref = self.source if self.ref is None else self.ref
            self.release = {
                "tag_name": TAG,
                "target_commitish": argv[argv.index("--target") + 1],
                "body": argv[argv.index("--notes") + 1],
                "prerelease": "--prerelease" in argv,
            }
            for path in argv[4:]:
                if path.startswith("--"):
                    break
                file = Path(path)
                self.assets[file.name] = file.read_bytes()
            return SimpleNamespace(returncode=0, stdout="", stderr="")
        if argv[1:3] == ["release", "download"]:
            name = argv[argv.index("--pattern") + 1]
            dest = Path(argv[argv.index("--dir") + 1]) / name
            value = self.assets[name]
            if name == publish.ARCHIVE and self.fault == "corrupt-readback":
                value += b"unauthorized-extra-byte"
            if name == "SHA256SUMS.sig" and self.fault == "corrupt-sig-readback":
                value = bytes([value[0] ^ 0xFF]) + value[1:]
            dest.write_bytes(value)
            return SimpleNamespace(returncode=0, stdout="", stderr="")
        raise AssertionError(f"unexpected fake gh invocation: {argv}")


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="wrapper-release-mock-")
        self.addCleanup(self.temp.cleanup)
        self.dist = Path(self.temp.name)
        archive = b"deterministic-candidate-release-bytes"
        digest = hashlib.sha256(archive).hexdigest()
        for name, data in {
            publish.ARCHIVE: archive,
            publish.ARCHIVE + ".sha256": f"{digest}  {publish.ARCHIVE}\n".encode(),
            "install.sh": b"#!/bin/sh\nexit 0\n",
            "SHA256SUMS": b"candidate-checksums\n",
        }.items():
            (self.dist / name).write_bytes(data)

    def make(self, fake, *, source=SOURCE, dispatch=True, prerelease=False):
        return publish.Publication(REPO, TAG, source, self.dist,
                                   dispatch=dispatch, prerelease=prerelease, runner=fake)

    def test_existing_release_readback_succeeds_even_if_target_commitish_is_stale(self):
        fake = FakeGh(fault="stale-target")
        warnings = StringIO()
        with redirect_stderr(warnings):
            self.make(fake).publish()
        self.assertIn("WARNING: target_commitish=", warnings.getvalue())
        self.assertEqual(fake.release["target_commitish"], OTHER)
        self.assertTrue(any("download" in argv for argv in fake.calls))
        self.assertEqual(len(fake.assets), 4)

    def test_existing_prerelease_is_updated_to_stable(self):
        fake = FakeGh(prerelease=True)
        output = StringIO()
        with redirect_stdout(output):
            self.make(fake, prerelease=False).publish()
        edit = next(argv for argv in fake.calls if argv[1:3] == ["release", "edit"])
        self.assertIn("--prerelease=false", edit)
        self.assertIs(fake.release["prerelease"], False)
        self.assertRegex(output.getvalue(), r"^RELEASE_VERIFIED .* prerelease=False\n$")

    def test_existing_stable_release_is_updated_to_prerelease(self):
        fake = FakeGh(prerelease=False)
        output = StringIO()
        with redirect_stdout(output):
            self.make(fake, prerelease=True).publish()
        edit = next(argv for argv in fake.calls if argv[1:3] == ["release", "edit"])
        self.assertIn("--prerelease", edit)
        self.assertIs(fake.release["prerelease"], True)
        self.assertRegex(output.getvalue(), r"^RELEASE_VERIFIED .* prerelease=True\n$")

    def test_release_prerelease_readback_mismatch_fails_closed(self):
        for expected in (True, False):
            with self.subTest(expected=expected):
                fake = FakeGh(prerelease=not expected, fault="prerelease-edit-ignored")
                with self.assertRaisesRegex(publish.PublicationError, "release prerelease mismatch"):
                    self.make(fake, prerelease=expected).publish()
                self.assertFalse(any(argv[1:3] == ["release", "download"] for argv in fake.calls))

    def test_annotated_tag_peels_to_commit(self):
        fake = FakeGh(annotated=True)
        self.make(fake).publish()
        self.assertEqual(fake.ref, SOURCE)
        self.assertTrue(any("/git/tags/" + OBJECT in str(call) for call in fake.calls))

    def test_absent_release_created_and_verified(self):
        fake = FakeGh(existing=False)
        self.make(fake).publish()
        self.assertEqual(fake.ref, SOURCE)
        self.assertEqual(len(fake.assets), 4)

    def test_tag_lookup_http_failure_is_not_absence(self):
        fake = FakeGh(fault="tag-lookup-error")
        with self.assertRaisesRegex(publish.PublicationError, "HTTP 500"):
            self.make(fake).publish()

    def test_tag_update_failure_does_not_succeed(self):
        fake = FakeGh(source=OTHER, fault="patch-error")
        with self.assertRaisesRegex(publish.PublicationError, "HTTP 422"):
            self.make(fake).publish()

    def test_patch_without_matching_readback_fails(self):
        fake = FakeGh(source=OTHER, fault="patch-ignored")
        with self.assertRaisesRegex(publish.PublicationError, "tag commit mismatch"):
            self.make(fake).publish()

    def test_wrong_archive_sha_rejected_before_api(self):
        (self.dist / (publish.ARCHIVE + ".sha256")).write_text("0" * 64 + "  " + publish.ARCHIVE + "\n")
        with self.assertRaisesRegex(publish.PublicationError, "archive SHA256SUMS"):
            self.make(FakeGh())

    def test_invalid_source_sha_and_source_api_mismatch_are_rejected(self):
        with self.assertRaisesRegex(publish.PublicationError, "invalid repository"):
            self.make(FakeGh(), source="short")
        with self.assertRaisesRegex(publish.PublicationError, "verified Git commit"):
            self.make(FakeGh(fault="source-not-commit")).publish()

    def test_release_notes_mismatch_is_not_silenced(self):
        with self.assertRaisesRegex(publish.PublicationError, "notes source/archive"):
            self.make(FakeGh(fault="notes-mismatch")).publish()

    def test_partial_upload_reports_failure(self):
        fake = FakeGh(fault="partial-upload")
        with self.assertRaisesRegex(publish.PublicationError, "partial upload"):
            self.make(fake).publish()
        self.assertEqual(len(fake.assets), 1)

    def test_uploaded_archive_byte_mismatch_rejected(self):
        with self.assertRaisesRegex(publish.PublicationError, "asset bytes differ"):
            self.make(FakeGh(fault="corrupt-readback")).publish()

    def test_concurrent_writer_tag_change_is_rejected(self):
        with self.assertRaisesRegex(publish.PublicationError, "tag commit mismatch"):
            self.make(FakeGh(fault="concurrent-tag-change")).publish()

    def test_tag_drift_after_successful_upload_is_detected(self):
        fake = FakeGh(fault="drift-on-final-ref")
        with self.assertRaisesRegex(publish.PublicationError, "tag commit mismatch"):
            self.make(fake).publish()

    def test_push_tag_event_mismatch_never_moves_tag(self):
        fake = FakeGh(source=OTHER)
        with self.assertRaisesRegex(publish.PublicationError, "tag/source mismatch"):
            self.make(fake, dispatch=False).publish()
        self.assertFalse(any("-X" in args for args in fake.calls))

    def test_annotated_tag_different_commit_is_preserved(self):
        fake = FakeGh(source=OTHER, annotated=True)
        with self.assertRaisesRegex(publish.PublicationError, "annotated tag targets"):
            self.make(fake).publish()
        self.assertFalse(any("-X" in args for args in fake.calls))

    def test_release_job_has_serialized_per_tag_concurrency(self):
        text = (Path(__file__).resolve().parents[1] / "workflows/wrapper.yml").read_text()
        self.assertIn("wrapper-publish-", text)
        self.assertIn("cancel-in-progress: false", text)

    def test_signed_publication_and_readback_verify_ed25519_signature(self):
        import subprocess
        priv = Path(self.temp.name) / "ed25519.priv.pem"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(priv)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        fake = FakeGh(existing=False)
        output = StringIO()
        with redirect_stdout(output):
            publish.Publication(
                REPO, TAG, SOURCE, self.dist,
                dispatch=True, prerelease=False,
                signing_key=priv, sequence=7, runner=fake,
            ).publish()
        self.assertIn("SHA256SUMS.sig", fake.assets)
        self.assertEqual(len(fake.assets), 5)
        self.assertRegex(output.getvalue(), r"signed=True sequence=7 key_sha256=[0-9a-f]{64}")

        bad_fake = FakeGh(existing=False, fault="corrupt-sig-readback")
        with self.assertRaisesRegex(publish.PublicationError, "asset bytes differ"):
            publish.Publication(
                REPO, TAG, SOURCE, self.dist,
                dispatch=True, prerelease=False,
                signing_key=priv, sequence=8, runner=bad_fake,
            ).publish()


if __name__ == "__main__":
    unittest.main(verbosity=2)
