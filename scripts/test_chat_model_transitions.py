"""Unit-level contract checks for the isolated TUI transition fixture."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from qualify_chat_model_transitions import fixture_auth_command


class FixtureSynchronizeContract(unittest.TestCase):
    def test_catalog_observation_requires_the_exact_owned_state(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            auth = root / 'fixture-auth'
            auth.write_text(fixture_auth_command(root, root / 'real-provider'))
            auth.chmod(0o700)
            reply = {'schema': 'codex-chatgpt-app-models-v1', 'inference_submitted': False}
            (root / 'synthetic-live-models.json').write_text(json.dumps(reply))
            state = str(root / 'state')
            accepted = subprocess.run([str(auth), 'host', 'models', '--state', state],
                                      capture_output=True, text=True, timeout=5)
            self.assertEqual(accepted.returncode, 0, accepted.stderr)
            self.assertEqual(json.loads(accepted.stdout), reply)
            for args in [
                ('host', 'models', '--state', str(root / 'other')),
                ('host', 'ensure', '--state', state),
                ('host', 'models', '--state', state, '--refresh', 'false'),
            ]:
                rejected = subprocess.run([str(auth), *args], capture_output=True, text=True, timeout=5)
                self.assertNotEqual(rejected.returncode, 0)
                self.assertEqual(rejected.stdout, '')

    def test_exact_thread_and_state_are_required_before_ready(self):
        thread = '01a1c381-aaaa-7aaa-8aaa-123456789012'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            auth = root / 'fixture-auth'
            auth.write_text(fixture_auth_command(root, root / 'real-provider'))
            auth.chmod(0o700)
            commands = root / 'synthetic-work-synchronizations.txt'
            state = str(root / 'state')

            def invoke(*args):
                return subprocess.run([str(auth), *args], capture_output=True, text=True,
                                      timeout=5, check=False)

            permitted = invoke('work', 'synchronize', '--thread-id', thread, '--state', state)
            self.assertEqual(permitted.returncode, 0, permitted.stderr)
            self.assertEqual(json.loads(permitted.stdout), {
                'thread_id': thread, 'status': 'ready', 'inference_submitted': False})
            self.assertEqual(commands.read_text().splitlines(), [thread])

            for args in [
                ('work', 'synchronize', '--thread-id', thread, '--state', str(root / 'other')),
                ('work', 'synchronize', '--thread-id', 'bad-thread', '--state', state),
                ('work', 'synchronize', '--thread-id', thread, '--state', state, '--extra'),
                ('work', 'start', '--thread-id', thread, '--state', state),
                ('work', 'synchronize', '--state', state, '--thread-id', thread),
            ]:
                with self.subTest(args=args):
                    rejected = invoke(*args)
                    self.assertNotEqual(rejected.returncode, 0)
                    self.assertEqual(rejected.stdout, '')
                    self.assertEqual(commands.read_text().splitlines(), [thread],
                                     'rejected CLI must have no synthetic ready effect')

            self.assertEqual(invoke('token').stdout, 'owned-inert-fixture\n')
            self.assertNotEqual(invoke('not-a-provider-command').returncode, 0)
            self.assertEqual(commands.read_text().splitlines(), [thread])


if __name__ == '__main__':
    unittest.main()
