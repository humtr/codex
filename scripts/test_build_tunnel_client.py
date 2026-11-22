#!/usr/bin/env python3
import unittest
import urllib.error

from scripts.build_tunnel_client import (
    fetch_latest_release,
    parse_redirect_tag,
    select_release,
)


class ReleaseSelection(unittest.TestCase):
    def test_latest_resolves_official_stable_without_silent_old_fallback(self):
        recipe = {"version": "v0.0.21"}
        self.assertEqual(
            select_release(
                "latest",
                recipe,
                lambda: {"tag_name": "v0.0.21", "draft": False, "prerelease": False},
            ),
            "v0.0.21",
        )
        with self.assertRaisesRegex(ValueError, "no older build was selected"):
            select_release("latest", recipe, lambda: {"tag_name": "v0.0.22"})
        for field in ("draft", "prerelease"):
            with self.assertRaisesRegex(ValueError, "not stable"):
                select_release(
                    "latest", recipe, lambda: {"tag_name": "v0.0.21", field: True}
                )
        with self.assertRaisesRegex(ValueError, "outside the validated recipe"):
            select_release("latest", recipe, lambda: {})

    def test_exact_validated_release_needs_no_discovery_request(self):
        def forbidden():
            self.fail("explicit source version must not discover another release")

        self.assertEqual(
            select_release("v0.0.21", {"version": "v0.0.21"}, forbidden), "v0.0.21"
        )
        with self.assertRaisesRegex(ValueError, "outside the validated recipe"):
            select_release("../../other-repo", {"version": "v0.0.21"}, forbidden)

    def test_github_api_403_falls_back_to_web_redirect(self):
        def rate_limited(_url):
            raise urllib.error.HTTPError(_url, 403, "rate limit exceeded", {}, None)

        res = fetch_latest_release(
            api_opener=rate_limited,
            web_fallback=lambda: {
                "tag_name": "v0.0.21",
                "draft": False,
                "prerelease": False,
                "via_redirect": True,
            },
        )
        self.assertEqual(res["tag_name"], "v0.0.21")
        self.assertTrue(res["via_redirect"])

    def test_parse_redirect_tag_validates_semver_tag(self):
        self.assertEqual(
            parse_redirect_tag("https://github.com/openai/tunnel-client/releases/tag/v0.0.21"),
            "v0.0.21",
        )
        with self.assertRaises(ValueError):
            parse_redirect_tag("https://github.com/openai/tunnel-client/releases/tag/v0.0.21-rc1")
        with self.assertRaises(ValueError):
            parse_redirect_tag("https://example.com/unexpected")


if __name__ == "__main__":
    unittest.main()
