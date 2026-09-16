#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("rald3_preflight", HERE / "rald3_preflight.py")
assert SPEC and SPEC.loader
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class PreflightTests(unittest.TestCase):
    def test_upstream_metadata_is_strict_and_target_bound(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "latest.json"
            sha = "a" * 64
            path.write_text(json.dumps({
                "tag_name": "rust-v0.155.0",
                "assets": [{"name": M.UPSTREAM_ASSET, "digest": f"sha256:{sha}"}],
            }))
            self.assertEqual(M.parse_upstream(path), ("0.155.0", sha))
            path.write_text('{"tag_name":"rust-v0.155.0","tag_name":"rust-v0.156.0","assets":[]}')
            with self.assertRaises(M.PreflightError):
                M.parse_upstream(path)

    def test_index_requires_generation_bound_https_base(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "index"
            path.write_text(
                "codex-update-index-v1\nchannel\tstable\n"
                "generation_id\tgen-1\n"
                "release_base\thttps://example.test/releases/gen-1/\n"
            )
            self.assertEqual(M.parse_update_index(path), ("gen-1", "https://example.test/releases/gen-1/"))
            path.write_text(path.read_text().replace("gen-1/", "other/"))
            with self.assertRaises(M.PreflightError):
                M.parse_update_index(path)

    def test_semver_compare_is_monotonic(self) -> None:
        self.assertLess(M.stable_version("0.154.0"), M.stable_version("0.155.0"))
        with self.assertRaises(M.PreflightError):
            M.stable_version("0.155.0-rc.1")

    def test_candidate_requires_deferred_marker_and_bound_digests(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            files = {"runtime": b"runtime", "core": b"core", "manager": b"manager"}
            for name, data in files.items():
                path = root / name
                path.write_bytes(data)
                path.chmod(0o755)
            marker = root / M.DEFERRED_MARKER
            marker.write_bytes(M.DEFERRED_MARKER_BYTES)
            marker.chmod(0o644)
            descriptor = "\n".join([
                M.GENERATION_FORMAT,
                "generation_id\tgen-1",
                f"upstream_package_identity\t{M.PACKAGE_IDENTITY}",
                "upstream_package_version\t0.155.0",
                f"source_artifact_digest\t{'a' * 64}",
                "expected_platform\tandroid",
                "expected_architecture\taarch64",
                "patch_policy_id\ttermux-fd-remap-v1",
                "patch_report\tx",
                f"runtime_digest\t{digest(files['runtime'])}",
                f"core_artifact_digest\t{digest(files['core'])}",
                f"manager_artifact_digest\t{digest(files['manager'])}",
                "core_api_identity\tcore-api-v1",
                "persistent_schema_identity\tschema-v1",
                "qualification\tqualified",
                f"creation_metadata\t{M.R10_METADATA}",
                "upstream_doctor\tsupported",
                "helper_count\t2",
                f"helper\ttermux-browser-open-v1\t{'b' * 64}",
                f"helper\ttermux-browser-manual-v1\t{'c' * 64}",
                "",
            ])
            (root / "generation.meta").write_text(descriptor)
            self.assertEqual(M.verify_candidate(root, "0.155.0")["generation_id"], "gen-1")
            with self.assertRaises(M.PreflightError):
                M.verify_qualified_candidate(root, "0.155.0")
            marker.unlink()
            self.assertEqual(
                M.verify_qualified_candidate(root, "0.155.0")["generation_id"], "gen-1"
            )
            with self.assertRaises(M.PreflightError):
                M.verify_candidate(root, "0.155.0")
            (root / "manager").write_bytes(b"tampered")
            (root / "manager").chmod(0o755)
            with self.assertRaises(M.PreflightError):
                M.verify_candidate(root, "0.155.0")

    def test_next_release_sequence_is_bounded(self) -> None:
        self.assertEqual(M.next_release_sequence("10"), 11)
        with self.assertRaises(M.PreflightError):
            M.next_release_sequence(str((1 << 64) - 1))
        with self.assertRaises(M.PreflightError):
            M.next_release_sequence("0")

    def test_manifest_entry_is_exact(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "release.manifest"
            sha = "d" * 64
            path.write_text("\n".join([
                "codex-release-v4",
                "generation_id\tgen-1",
                "release_sequence\t10",
                "channel\tstable",
                "expected_platform\tandroid",
                "expected_architecture\taarch64",
                "core_api_identity\tcore-api-v1",
                "persistent_schema_identity\tschema-v1",
                f"release_public_key\t{'e' * 64}",
                "file_count\t1",
                f"file\tgeneration.meta\t{sha}\t0644",
                "",
            ]))
            self.assertEqual(M.parse_release_manifest(path, "generation.meta"), ("gen-1", 10, sha, "0644"))


if __name__ == "__main__":
    unittest.main()
