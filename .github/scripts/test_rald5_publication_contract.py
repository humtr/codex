#!/usr/bin/env python3
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import rald5_publication as publication  # noqa: E402

GENERATION = "local-rald5-contract-1"


def manifest_lines() -> list[str]:
    lines = [
        "codex-release-v4",
        f"generation_id\t{GENERATION}",
        "release_sequence\t11",
        "channel\tstable",
        "expected_platform\tandroid",
        "expected_architecture\taarch64",
        "core_api_identity\tcore-api-v1",
        "persistent_schema_identity\tschema-v1",
        f"release_public_key\t{'a' * 64}",
        "file_count\t7",
    ]
    for rel, mode in publication.EXPECTED_FILES.items():
        lines.append(f"file\t{rel}\t{'b' * 64}\t{mode}")
    return lines


class Rald5PublicationContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="rald5-contract-")
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_manifest(self, lines: list[str]) -> Path:
        path = self.root / "release.manifest"
        path.write_text("\n".join(lines) + "\n")
        return path

    def test_missing_release_entry_is_rejected(self) -> None:
        release = self.root / "release"
        release.mkdir()
        (release / "core").write_bytes(b"core")
        with self.assertRaises(publication.ValidationError):
            publication.exact_entries(release, {"core", "runtime"})

    def test_manifest_metadata_order_is_exact(self) -> None:
        lines = manifest_lines()
        lines[4], lines[5] = lines[5], lines[4]
        with self.assertRaisesRegex(publication.ValidationError, "expected_platform"):
            publication.parse_manifest(self.write_manifest(lines))

    def test_manifest_signed_file_order_is_exact(self) -> None:
        lines = manifest_lines()
        lines[-1], lines[-2] = lines[-2], lines[-1]
        with self.assertRaisesRegex(publication.ValidationError, "ordering"):
            publication.parse_manifest(self.write_manifest(lines))


if __name__ == "__main__":
    unittest.main()
