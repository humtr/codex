#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys
import unittest

SCRIPT = Path(__file__).with_name("rald5_promotion.py")
EXPECTED = "1" * 40
NEW = "2" * 40
OTHER = "3" * 40


def run(actual: str, rc: str = "1") -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--expected",
            EXPECTED,
            "--new-commit",
            NEW,
            "--actual",
            actual,
            "--update-rc",
            rc,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


class Rald5PromotionTests(unittest.TestCase):
    def test_observed_new_commit_is_committed_even_after_ambiguous_api_rc(self) -> None:
        result = run(NEW, "1")
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"promotion_result=committed", result.stdout)
        self.assertIn(b"ref_update_rc=1", result.stdout)

    def test_expected_ref_after_update_is_failure(self) -> None:
        result = run(EXPECTED)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"CAS did not advance", result.stderr)

    def test_concurrent_ref_is_failure_even_if_candidate_bytes_could_match(self) -> None:
        result = run(OTHER)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"CAS race", result.stderr)

    def test_indeterminate_ref_value_is_failure(self) -> None:
        result = run("unknown")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"actual ref is not a canonical commit SHA", result.stderr)

    def test_noncanonical_update_result_is_failure(self) -> None:
        result = run(NEW, "01")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not canonical", result.stderr)


if __name__ == "__main__":
    unittest.main()
