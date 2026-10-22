import json
import os
import pty
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from scripts.qualification_diagnostics import (
    AndroidLogCapture, DispatchLedger, Recorder, candidate_info, classify,
    listener_port, observer_keeper, process_fact, release_observer_keeper, sample,
    terminal_contains, native_thread_settled,
)


class Diagnostics(unittest.TestCase):
    def test_terminal_item_without_confirmed_completion_is_not_a_pass_or_exception(self):
        state = {'status': {'type': 'idle'}, 'turns': [
            {'status': 'interrupted', 'items': [{'type': 'agentMessage', 'phase': 'final_answer'}]}]}
        for status in ['interrupted', 'failed', 'inProgress']:
            state['turns'][-1]['status'] = status
            self.assertFalse(native_thread_settled(state, 1))
        state['turns'][-1]['status'] = 'completed'
        self.assertTrue(native_thread_settled(state, 1))
        self.assertFalse(native_thread_settled(state, 2))
        state['status']['type'] = 'active'
        self.assertFalse(native_thread_settled(state, 1))
        self.assertFalse(native_thread_settled({'status': {'type': 'idle'}, 'turns': []}, 0))

    def test_terminal_labels_split_by_real_cursor_updates_are_not_missing(self):
        output = '\x1b]0;private window title\x07\x1b[42;3H2. ChatGPT\x1b[42;14H·\x1b[42;16HGPT-6\x1b[42;29Hdescription'
        self.assertTrue(terminal_contains(output, 'ChatGPT · GPT-6'))
        self.assertFalse(terminal_contains(output, 'ChatGPT · GPT-6 Pro'))
        self.assertFalse(terminal_contains(output, 'private window title'))

    def test_candidate_endpoint_replaces_the_inherited_original_port(self):
        ready = {'base_url': 'http://127.0.0.1:34061/v1'}
        self.assertEqual(listener_port(ready), 34061)
        result = candidate_info({'port': 45475, 'profile': 'kept'}, ready,
            binary='/tmp/candidate', state='/tmp/state', pid=123,
            desktop_session='/tmp/relay')
        self.assertEqual(result['port'], 34061)
        self.assertEqual(result['profile'], 'kept')
        with self.assertRaises(ValueError):
            listener_port({'base_url': 'http://0.0.0.0:34061/v1'})

    def test_dispatch_ledger_counts_managed_and_local_paths_without_zero_inference(self):
        ledger = DispatchLedger()
        self.assertIsNone(ledger.observe(0, [('work-a', None)])['actual_chat_dispatches'])
        known = ledger.observe(0, [('work-a', 1)])
        self.assertEqual(known['actual_chat_dispatches'], 1)
        self.assertEqual(known['managed_app_dispatches'], 1)
        # Repeated observations of one journal count its high-water mark once.
        repeated = ledger.observe(0, [('work-a', 1)])
        self.assertEqual(repeated['actual_chat_dispatches'], 1)
        combined = ledger.observe(2, [('work-a', 1), ('work-b', 2)])
        self.assertEqual(combined['local_source_requests'], 2)
        self.assertEqual(combined['managed_app_dispatches'], 3)
        self.assertEqual(combined['actual_chat_dispatches'], 5)
        with self.assertRaises(ValueError):
            ledger.observe(-1, [])

    def test_observer_keeper_holds_pty_and_input_pipes_until_nonce_release(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as root:
            Path(root).chmod(0o700)
            master, slave = pty.openpty()
            tui = subprocess.Popen([sys.executable, '-c',
                'import sys,time; print("keeper-pty-live",flush=True); time.sleep(30)'],
                stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            input_read, input_write = os.pipe()
            output_read, output_write = os.pipe()
            client = subprocess.Popen([sys.executable, '-c',
                'import sys; sys.stdin.read()'], stdin=input_read, stdout=output_write,
                stderr=subprocess.DEVNULL, start_new_session=True)
            os.close(slave)
            os.close(input_read)
            os.close(output_write)
            keeper = None
            try:
                keeper, receipt = observer_keeper(root, terminal_fd=master,
                    input_stdout_fd=output_read, input_stdin_fd=input_write,
                    terminal_log=Path(root) / 'tui.log', thread_id='thread')
                os.close(master)
                master = -1
                os.close(output_read)
                output_read = -1
                os.close(input_write)
                input_write = -1
                time.sleep(.35)
                self.assertIsNone(tui.poll())
                self.assertIsNone(client.poll())
                with self.assertRaises(ValueError):
                    release_observer_keeper(root, receipt['nonce'],
                        {'thread_id': 'thread', 'thread_status': 'running',
                         'last_turn_status': 'inProgress', 'pending_calls': []})
                with self.assertRaises(ValueError):
                    release_observer_keeper(root, receipt['nonce'],
                        {'thread_id': 'unrelated-thread', 'thread_status': 'idle',
                         'last_turn_status': 'completed', 'pending_calls': []})
                release_observer_keeper(root, receipt['nonce'],
                    {'thread_id': 'thread', 'thread_status': 'idle',
                     'last_turn_status': 'completed', 'pending_calls': []})
                self.assertEqual(keeper.wait(timeout=3), 0)
            finally:
                for descriptor in (master, output_read, input_write):
                    if descriptor >= 0:
                        os.close(descriptor)
                if keeper is not None and keeper.poll() is None:
                    keeper.terminate()
                    keeper.wait(timeout=3)
                for child in (tui, client):
                    if child.poll() is None:
                        child.terminate()
                        child.wait(timeout=3)

    def test_observer_keeper_can_preserve_a_terminal_without_fake_input_pipes(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as root:
            master, slave = pty.openpty()
            keeper = None
            try:
                keeper, receipt = observer_keeper(root, terminal_fd=master,
                    terminal_log=Path(root) / 'tui.log', thread_id='terminal-only')
                os.close(master)
                master = -1
                os.write(slave, b'terminal-only-output\n')
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    log = Path(root) / 'tui.log'
                    if log.exists() and b'terminal-only-output' in log.read_bytes():
                        break
                    time.sleep(.02)
                self.assertIn(b'terminal-only-output', log.read_bytes())
                self.assertIsNone(receipt['input_stdin_fd'])
                release_observer_keeper(root, receipt['nonce'],
                    {'thread_id': 'terminal-only', 'thread_status': 'idle',
                     'last_turn_status': 'completed', 'pending_calls': []})
                self.assertEqual(keeper.wait(timeout=3), 0)
            finally:
                if master >= 0:
                    os.close(master)
                os.close(slave)
                if keeper is not None and keeper.poll() is None:
                    keeper.terminate()
                    keeper.wait(timeout=3)

    def test_android_log_is_private_bounded_filtered_and_owned(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as root:
            with patch('scripts.qualification_diagnostics.subprocess.Popen') as launch:
                capture = AndroidLogCapture(root, executable='/owned/logcat')
                argv = launch.call_args.args[0]
                self.assertEqual(argv[argv.index('-r') + 1], '256')
                self.assertEqual(argv[argv.index('-n') + 1], '2')
                self.assertIn('*:S', argv)
                self.assertIn('AndroidRuntime:E', argv)
                self.assertEqual((Path(root) / 'android.log').stat().st_mode & 0o777, 0o600)
                launch.return_value.poll.return_value = None
                capture.close()
                launch.return_value.terminate.assert_called_once()
                with self.assertRaises(FileExistsError):
                    AndroidLogCapture(root, executable='/owned/logcat')

    def test_android_log_close_preserves_an_already_exited_child(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as root:
            with patch('scripts.qualification_diagnostics.subprocess.Popen') as launch:
                launch.return_value.poll.return_value = 1
                capture = AndroidLogCapture(root, executable='/owned/logcat')
                capture.close()
                launch.return_value.terminate.assert_not_called()

    def test_current_process_has_identity_resource_and_ancestry_without_arguments(self):
        fact = process_fact(Path('/proc') / str(os.getpid()))
        self.assertEqual(fact['pid'], os.getpid())
        self.assertEqual(fact['ppid'], os.getppid())
        self.assertGreater(fact['rss_kib'], 0)
        self.assertNotIn('argv', fact)
        self.assertNotIn('environment', fact)

    def test_snapshot_keeps_app_identity_and_all_owned_process_counts(self):
        fact = sample()
        self.assertGreater(fact['owned_process_count'], 0)
        self.assertTrue(any(p['pid'] == os.getpid() for p in fact['processes']))
        self.assertGreater(fact['owned_rss_kib'], 0)

    def test_restart_classification_does_not_invent_oom_or_signal(self):
        before = {'boot_id': 'a', 'apps': [{'name': 'com.termux', 'pid': 1, 'start': '10'}]}
        self.assertEqual(classify(before, before), 'worker_recording_interrupted_cause_unknown')
        self.assertEqual(classify(before, before, closed=True), 'recorder_closed')
        self.assertEqual(classify(before, dict(before, boot_id='b')), 'device_boot_changed')
        self.assertEqual(classify(before, {'boot_id': 'a', 'apps': []}), 'termux_app_identity_changed')
        changed = {'boot_id': 'a', 'apps': [{'name': 'com.termux', 'pid': 1, 'start': '20'}]}
        self.assertEqual(classify(before, changed), 'termux_app_identity_changed')

    def test_stages_are_durable_and_sampling_is_bounded(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as root:
            recorder = Recorder(root, interval=60, max_sample_bytes=1)
            recorder.event('enter_written', native_thread='owned')
            row = json.loads((Path(root) / 'last-stage.json').read_text())
            self.assertEqual(row['phase'], 'enter_written')
            self.assertFalse(recorder.capture())
            recorder.close('test_complete')
            rows = [json.loads(s) for s in (Path(root) / 'stages.jsonl').read_text().splitlines()]
            self.assertEqual(rows[-1]['phase'], 'recorder_closed')
            self.assertEqual([p['sequence'] for p in rows], list(range(1, len(rows) + 1)))
            self.assertEqual(len((Path(root) / 'resources.jsonl').read_text().splitlines()), 0)


if __name__ == '__main__':
    unittest.main()
