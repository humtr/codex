import unittest

from scripts.build_tunnel_client import select_release


class ReleaseSelection(unittest.TestCase):
    def test_latest_resolves_official_stable_without_silent_old_fallback(self):
        recipe = {'version': 'v0.0.18'}
        self.assertEqual(select_release('latest', recipe, lambda: {
            'tag_name': 'v0.0.18', 'draft': False, 'prerelease': False}), 'v0.0.18')
        with self.assertRaisesRegex(ValueError, 'no older build was selected'):
            select_release('latest', recipe, lambda: {'tag_name': 'v0.0.19'})
        for field in ('draft', 'prerelease'):
            with self.assertRaisesRegex(ValueError, 'not stable'):
                select_release('latest', recipe, lambda: {'tag_name': 'v0.0.18', field: True})
        with self.assertRaisesRegex(ValueError, 'outside the validated recipe'):
            select_release('latest', recipe, lambda: {})

    def test_exact_validated_release_needs_no_discovery_request(self):
        def forbidden():
            self.fail('explicit source version must not discover another release')
        self.assertEqual(select_release('v0.0.18', {'version': 'v0.0.18'}, forbidden), 'v0.0.18')
        with self.assertRaisesRegex(ValueError, 'outside the validated recipe'):
            select_release('../../other-repo', {'version': 'v0.0.18'}, forbidden)


if __name__ == '__main__':
    unittest.main()
