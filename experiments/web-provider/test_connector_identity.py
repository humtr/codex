"""Offline selector/CLI checks; never start a browser or submit a Chat message."""
import json
import subprocess
import sys
import unittest
from pathlib import Path

SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE))
from live_core_probe import connector_config


class ConnectorIdentityTests(unittest.TestCase):
    def test_display_names_are_explicit_and_not_protocol_versions(self):
        for name in ('Codex Chat Bridge', '개발 연결', 'Local tools', 'Codex Native2', 'bridge "quoted"'):
            with self.subTest(name=name):
                self.assertEqual(connector_config(name), {'appName': name, 'automaticAppName': name})
                self.assertEqual(json.loads(json.dumps(connector_config(name)))['appName'], name)

    def test_invalid_selectors_fail_before_any_device_action(self):
        for name in (None, '', ' ', ' bridge', 'bridge ', 'a' * 81, 'a\nb', 'a\x00b', 'a\x7fb'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                connector_config(name)

    def test_browser_entrypoints_require_an_explicit_name_before_accessing_state(self):
        for script in ('live_core_probe.py', 'fresh_login_probe.py', 'profile_reuse_probe.py'):
            with self.subTest(script=script):
                result = subprocess.run([sys.executable, '-B', str(SOURCE / script)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 2)
                self.assertIn('--connector-name', result.stderr)


if __name__ == '__main__':
    unittest.main()
