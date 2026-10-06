import unittest
from adapt_frontend import adapt, policy


class FrontendPolicyTests(unittest.TestCase):
    def test_exact_existing_policy_only_changes_54_bytes(self):
        raw = b'ELF-owned-prefix\0' + b'\0'.join(old for old, _, count in policy() for _ in range(count)) + b'\0suffix'
        adapted = adapt(raw)
        self.assertEqual(len(raw), len(adapted))
        self.assertEqual(sum(a != b for a, b in zip(raw, adapted)), 54)
        self.assertTrue(adapted.startswith(b'ELF-owned-prefix\0') and adapted.endswith(b'\0suffix'))

    def test_missing_extra_or_prepatched_sources_fail_closed(self):
        raw = b'\0'.join(old for old, _, count in policy() for _ in range(count))
        old, new, _ = policy()[0]
        for bad in [raw.replace(old, b'X' * len(old), 1), raw + old, raw.replace(old, new, 1)]:
            with self.subTest(kind=bad[:20]), self.assertRaises(AssertionError):
                adapt(bad)


if __name__ == '__main__':
    unittest.main()
