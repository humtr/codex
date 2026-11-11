"""The actual Native compiler qualifier must reject missing or borrowed proof."""
import copy
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).parent))
import qualify_provider_native_compiler as q
import qualify_provider_native_trial as trial


class CandidateInstallationBoundary(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='native-candidate-boundary-')
        self.addCleanup(self.temporary.cleanup)
        self.prefix = Path(self.temporary.name)
        self.case = self.prefix / 'candidate'
        self.case.mkdir(mode=0o700)
        self.home = self.case / 'h'
        self.home.mkdir(mode=0o700)
        self.core = self.case / 'p/bin/codex'
        self.core.parent.mkdir(parents=True)
        self.core.write_bytes(b'owned-core')
        self.core.chmod(0o755)
        (self.prefix / 'bin').mkdir()
        (self.prefix / 'bin/codex').write_bytes(b'operating-core-sentinel')
        self.generation = self.home / '.local/lib/codex/core/generations/owned-candidate'
        self.generation.mkdir(parents=True)
        (self.generation / 'core').write_bytes(self.core.read_bytes())
        (self.generation / 'runtime').write_bytes(b'owned-runtime')
        self.fields = {'core_artifact_digest': q.sha(self.core), 'runtime_digest': q.sha(self.generation / 'runtime')}
        marker = self.case / 'owned-fixture'
        marker.write_text('owned test installation')
        marker.chmod(0o600)
        self.public = self.case / 'fixture-public.pem'
        private = self.case / 'fixture-key.pem'
        subprocess.run(['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', str(private)], check=True, capture_output=True)
        subprocess.run(['openssl', 'pkey', '-in', str(private), '-pubout', '-out', str(self.public)], check=True, capture_output=True)
        private.chmod(0o600)
        manifest = self.generation / 'release.manifest'
        manifest.write_bytes(b'owned candidate manifest')
        subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(private), '-rawin', '-in', str(manifest),
            '-out', str(self.generation / 'release.sig')], check=True, capture_output=True)
        self.activation = self.home / '.local/share/codex/core/activation-state'
        self.activation.parent.mkdir(parents=True)
        self.activation.write_text(q.fixture_activation_state(self.generation.name, self.public))
        self.activation.chmod(0o600)

    def select(self, **changed):
        values = dict(core=self.core, generation=self.generation, home=self.home, prefix=self.prefix, candidate_root=self.case)
        values.update(changed)
        # The closed inventory has its own full parser/digest suite. These
        # negatives isolate the additional candidate boundary and real signature.
        with patch.object(q, 'local_generation_inventory', return_value=(self.fields, [])) as inventory:
            result = q.selected_installation(**values)
            inventory.assert_called_once_with(self.generation)
            return result

    def test_private_fixture_accepts_exact_digests_and_ephemeral_signature(self):
        self.assertEqual(self.select(), 'candidate-native-compiler')
        self.assertEqual((self.prefix / 'bin/codex').read_bytes(), b'operating-core-sentinel')

    def test_operating_mode_does_not_accept_candidate_entrypoint(self):
        with self.assertRaisesRegex(AssertionError, 'actual installed Core'):
            q.selected_installation(self.core, self.generation, self.home, self.prefix)

    def test_foreign_home_alias_and_nonprivate_root_fail_before_launch(self):
        with self.assertRaisesRegex(AssertionError, 'candidate HOME'):
            self.select(home=self.prefix)
        alias = self.prefix / 'alias'
        alias.symlink_to(self.case, target_is_directory=True)
        with self.assertRaisesRegex(AssertionError, 'alias'):
            self.select(candidate_root=alias)
        self.case.chmod(0o755)
        with self.assertRaisesRegex(AssertionError, 'private'):
            self.select()

    def test_changed_core_runtime_and_activation_fail_without_repair(self):
        self.core.chmod(0o777)
        with self.assertRaisesRegex(AssertionError, 'Core executable'):
            self.select()
        self.core.chmod(0o755)
        self.core.write_bytes(b'changed core')
        with self.assertRaisesRegex(AssertionError, 'Core digest'):
            self.select()
        self.core.write_bytes(b'owned-core')
        (self.generation / 'runtime').write_bytes(b'changed runtime')
        with self.assertRaisesRegex(AssertionError, 'Runtime digest'):
            self.select()
        (self.generation / 'runtime').write_bytes(b'owned-runtime')
        self.activation.write_text(self.activation.read_text().replace('current=owned-candidate', 'current=other'))
        with self.assertRaisesRegex(AssertionError, 'ephemeral fixture key'):
            self.select()
        self.assertIn('current=other', self.activation.read_text())

    def test_foreign_marker_key_and_invalid_signature_are_rejected(self):
        marker = self.case / 'owned-fixture'
        marker.unlink()
        marker.symlink_to(self.core)
        with self.assertRaisesRegex(AssertionError, 'marker'):
            self.select()
        marker.unlink()
        marker.write_text('owned')
        marker.chmod(0o600)
        self.public.chmod(0o666)
        with self.assertRaisesRegex(AssertionError, 'public key'):
            self.select()
        self.public.chmod(0o600)
        (self.generation / 'release.manifest').write_bytes(b'changed manifest')
        with self.assertRaises(subprocess.CalledProcessError):
            self.select()


class NativeCompilerEvidence(unittest.TestCase):
    def setUp(self):
        self.home = Path('/data/data/com.termux/files/home')
        self.prefix = Path('/data/data/com.termux/files/usr')
        self.workspace = self.prefix / 'tmp/nc-owned/w'
        self.runtime = self.home / '.local/lib/codex/core/generations/full-owned/runtime'
        self.owner = {'directory': str(self.home / '.local/share/codex/core/catalogs/0123456789abcdef'),
                      'home': str(self.workspace.parent / 'h/.codex'),
                      'process': {'pid': 123, 'start': '456', 'image': str(self.runtime)}}
        self.record = {'execution': self.owner, 'thread': 'native-thread', 'native_turn': 'native-turn'}
        self.metadata = {'schema': 2, 'hook_free_tools': ['exec_command'], 'pid': 123, 'start': '456', 'thread': 'native-thread',
                         'turn': 'native-turn', 'cwd': str(self.workspace), 'prefix': str(self.prefix),
                         'home': str(self.home), 'shell': str(self.prefix / 'bin/bash'),
                         'variables': ['HOME', 'PREFIX', 'PATH'], 'no_default_config': False}
        self.result = {'type': 'function_call_output', 'call_id': 'native-call',
                       'output': 'Chunk ID: original\nProcess running with session ID 76\nOutput:\n'}
        self.invocation = {'id': 'native-call', 'offered': True, 'result': self.result,
                           'effect': {'Paths': [
                               {'path': str(self.workspace / 'a.c'), 'operation': 'Input', 'base': 'source-digest'},
                               {'path': str(self.workspace / 'a.out'), 'operation': 'Add', 'base': None},
                               {'path': str(self.prefix / 'bin/clang-21'), 'external_input': True,
                                'operation': 'MetadataInput', 'base': 'actual-installed-identity'}]}}

    def proof(self, turn):
        return q.execution_proof(self.record, turn, home=self.home, prefix=self.prefix,
                                 workspace=self.workspace, runtime=self.runtime, observed=self.owner['process'])

    def test_actual_owner_and_same_native_thread_turn_are_required(self):
        self.assertEqual(self.proof({'execution': self.metadata}), self.metadata)
        with self.assertRaisesRegex(AssertionError, 'absent'):
            self.proof({})
        for key, value in [('schema', True), ('pid', 124), ('start', '457'), ('thread', 'other-thread'),
                           ('turn', 'other-turn'), ('cwd', '/another/workspace'), ('prefix', '/fake/prefix'),
                           ('home', '/fake/home'), ('shell', '/another/bash'), ('no_default_config', True)]:
            with self.subTest(key=key), self.assertRaises(AssertionError):
                self.proof({'execution': dict(self.metadata, **{key: value})})
        with self.assertRaises(AssertionError):
            self.proof({'execution': dict(self.metadata, variables=['CLANG_NO_DEFAULT_CONFIG'])})

    def test_tool_specific_hook_proof_requires_selected_exec_and_rejects_legacy(self):
        scoped = dict(self.metadata, schema=2, hook_free_tools=['exec_command'])
        self.assertEqual(self.proof({'execution': scoped}), scoped)
        self.assertEqual(self.proof({'execution': self.metadata}), self.metadata)
        for tools in (None, [], ['apply_patch'], 'exec_command', ['exec_command', 'unknown']):
            with self.subTest(tools=tools), self.assertRaises(AssertionError):
                self.proof({'execution': dict(scoped, hook_free_tools=tools)})
        with self.assertRaises(AssertionError):
            self.proof({'execution': dict(self.metadata, schema=1, hook_free_tools=[])})

    def test_native_image_and_private_execution_home_cannot_be_borrowed(self):
        for key, value in [('home', str(self.home / '.codex')), ('directory', str(self.workspace / 'catalog'))]:
            with self.subTest(key=key), self.assertRaises(AssertionError):
                q.execution_proof(dict(self.record, execution=dict(self.owner, **{key: value})),
                    {'execution': self.metadata}, home=self.home, prefix=self.prefix,
                    workspace=self.workspace, runtime=self.runtime, observed=self.owner['process'])
        with self.assertRaises(AssertionError):
            q.execution_proof(self.record, {'execution': self.metadata}, home=self.home, prefix=self.prefix,
                workspace=self.workspace, runtime=self.runtime, observed=dict(self.owner['process'], start='new-process'))

    def test_bounded_claim_has_real_source_output_and_installed_dependencies(self):
        q.compiler_claim(self.invocation, self.workspace, 'a', 'source-digest')
        for effect in ('Opaque', 'Read', None, {'Paths': []}, {'Paths': self.invocation['effect']['Paths'][:2]}):
            with self.subTest(effect=effect), self.assertRaises(AssertionError):
                q.compiler_claim(dict(self.invocation, effect=effect), self.workspace, 'a', 'source-digest')
        for label, digest in [('b', 'source-digest'), ('a', 'changed-source')]:
            with self.subTest(label=label, digest=digest), self.assertRaises(AssertionError):
                q.compiler_claim(self.invocation, self.workspace, label, digest)

    def test_original_native_result_and_call_identity_are_immutable(self):
        self.assertEqual(q.original_result({'result': self.result}, self.invocation), self.result)
        for result in [dict(self.result, output='same execution, rewritten result'), dict(self.result, call_id='other')]:
            with self.subTest(result=result), self.assertRaises(AssertionError):
                q.original_result({'result': result}, self.invocation)
        with self.assertRaises(AssertionError):
            q.original_result({'result': self.result}, dict(self.invocation, offered=False))

    def test_running_session_cannot_be_inferred_from_a_completed_command(self):
        self.assertEqual(q.running_session(self.result), 76)
        terminal = dict(self.result, output='Chunk ID: original\nProcess exited with code 0\nOutput:\n')
        self.assertTrue(q.completed(terminal))
        with self.assertRaises(AssertionError):
            q.running_session(terminal)
        self.assertFalse(q.completed(self.result))
        for output in ['Process exited with code 1\n', 'Process exited with code 0\nProcess exited with code 1\n']:
            with self.subTest(output=output), self.assertRaises(AssertionError):
                q.completed(dict(self.result, output=output))

    def test_both_actual_owners_and_sessions_must_hold_bounded_claims(self):
        rows = [{'owner_work': 'work-a', 'call': 'call-a', 'process_session': 76,
                 'resource_kind': 'bounded', 'pending_result': True},
                {'owner_work': 'work-b', 'call': 'call-b', 'process_session': 77,
                 'resource_kind': 'bounded', 'pending_result': True}]
        expected = {'work-a': {'id': 'call-a', 'session': 76}, 'work-b': {'id': 'call-b', 'session': 77}}
        self.assertEqual(q.simultaneous_claims({'native_effects': {'active': rows}}, expected), rows)
        for key, value in [('owner_work', 'other-work'), ('call', 'other-call'), ('process_session', None),
                           ('resource_kind', 'opaque'), ('pending_result', False)]:
            changed = copy.deepcopy(rows)
            changed[1][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError):
                q.simultaneous_claims({'native_effects': {'active': changed}}, expected)

    def test_native_history_requires_one_original_successful_command(self):
        shell, command = self.prefix / 'bin/bash', 'clang -O0 a.c -o a.out'
        item = {'type': 'commandExecution', 'id': 'native-call', 'status': 'completed', 'exitCode': 0,
                'cwd': str(self.workspace), 'command': str(shell) + " -c 'clang -O0 a.c -o a.out'",
                'aggregatedOutput': None, 'startedAtMs': 1000, 'completedAtMs': 2000}
        thread = {'turns': [{'items': [item]}]}
        self.assertEqual(q.command_item(thread, command, shell, self.workspace, 'native-call', ''), item)
        for changed in [dict(item, exitCode=True), dict(item, status='inProgress'),
                        dict(item, id='other-call'), dict(item, command=str(shell) + " -lc 'clang -O0 a.c -o a.out'"),
                        dict(item, completedAtMs=None), dict(item, startedAtMs=2001)]:
            with self.subTest(item=changed), self.assertRaises(AssertionError):
                q.command_item({'turns': [{'items': [changed]}]}, command, shell, self.workspace, 'native-call', '')
        with self.assertRaises(AssertionError):
            q.command_item({'turns': [{'items': [item, item]}]}, command, shell, self.workspace, 'native-call', '')
        immediate = dict(item, startedAtMs=2000)
        self.assertEqual(q.command_item({'turns': [{'items': [immediate]}]}, command, shell, self.workspace,
                                       'native-call', ''), immediate)

    def test_history_nullable_output_must_match_original_framed_receipt(self):
        shell, command = self.prefix / 'bin/bash', 'clang -O0 a.c -o a.out'
        item = {'type': 'commandExecution', 'id': 'native-call', 'status': 'completed', 'exitCode': 0,
                'cwd': str(self.workspace), 'command': str(shell) + " -c 'clang -O0 a.c -o a.out'",
                'aggregatedOutput': None, 'startedAtMs': 1000, 'completedAtMs': 2000}
        silent = dict(self.result, output='Chunk ID: original\nProcess exited with code 0\nOutput:\n')
        self.assertTrue(q.completed(silent))
        self.assertEqual(q.command_output(silent), '')
        def verify(row, output):
            return q.command_item({'turns': [{'items': [row]}]}, command, shell, self.workspace, 'native-call', output)
        self.assertEqual(verify(item, q.command_output(silent)), item)
        explicit = dict(item, aggregatedOutput='actual output\n')
        loud = dict(silent, output=silent['output'] + 'actual output\n')
        self.assertEqual(verify(explicit, q.command_output(loud)), explicit)
        for changed, output in [(item, 'actual output\n'), (explicit, ''), (item, None),
                                (dict(item, aggregatedOutput=''), ''),
                                ({key: value for key, value in item.items() if key != 'aggregatedOutput'}, '')]:
            with self.subTest(item=changed, output=output), self.assertRaises(AssertionError):
                verify(changed, output)
        for invalid in [None, '', 'Process exited with code 0\n', 'missing framing']:
            with self.subTest(output=invalid), self.assertRaises(AssertionError):
                q.command_output(dict(silent, output=invalid))

    def test_actual_intervals_must_overlap_not_only_http_admission(self):
        self.assertEqual(q.overlapping_commands([{'startedAtMs': 100, 'completedAtMs': 200},
                                                {'startedAtMs': 150, 'completedAtMs': 250}]), 50)
        with self.assertRaises(AssertionError):
            q.overlapping_commands([{'startedAtMs': 100, 'completedAtMs': 200},
                                    {'startedAtMs': 200, 'completedAtMs': 250}])

    def test_terminal_result_waits_only_for_the_same_native_watcher_publication(self):
        shell, command = self.prefix / 'bin/bash', 'clang -O0 a.c -o a.out'
        item = {'type': 'commandExecution', 'id': 'native-call', 'status': 'completed', 'exitCode': 0,
                'cwd': str(self.workspace), 'command': str(shell) + " -c 'clang -O0 a.c -o a.out'",
                'aggregatedOutput': None, 'startedAtMs': 1000, 'completedAtMs': 2000}
        def observe(items):
            return q.published_command({'turns': [{'items': items}]}, command, shell, self.workspace, 'native-call', '')
        self.assertIsNone(observe([]))
        self.assertIsNone(observe([dict(item, status='inProgress', exitCode=None, completedAtMs=None)]))
        self.assertIsNone(observe([dict(item, completedAtMs=None)]))
        self.assertEqual(observe([item]), item)
        for items in ([dict(item, status='failed', exitCode=1)], [dict(item, id='other-call')], [item, item]):
            with self.subTest(items=items), self.assertRaises(AssertionError):
                observe(items)

    def test_fixed_sources_are_distinct_finite_plain_c_under_the_actual_snapshot_bound(self):
        sources = [q.plain_source(label) for label in ('a', 'b')]
        self.assertNotEqual(*sources)
        for source in sources:
            self.assertLess(len(source.encode()), 4 * 1024 * 1024)
            self.assertNotIn('#', source)
            self.assertNotIn('\\', source)
            self.assertNotIn('asm', source)
            self.assertNotIn('__', source)
            self.assertEqual(source.count('int main(void)'), 1)

    def test_fixture_paths_cannot_reach_external_storage_by_name_or_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home, prefix, outside = root / 'home', root / 'prefix', root / 'outside'
            for path in (home, prefix, outside):
                path.mkdir()
            self.assertEqual(q.termux_path(prefix, home, prefix), prefix)
            (home / 'alias').symlink_to(outside, target_is_directory=True)
            for path in (outside, home / 'alias'):
                with self.subTest(path=path), self.assertRaises(AssertionError):
                    q.termux_path(path, home, prefix)

    def test_scoped_cli_cannot_accidentally_run_full_or_live_qualification(self):
        script = Path(__file__).with_name('qualify_provider_operator.py')
        result = subprocess.run([sys.executable, '-B', str(script), '--native-compiler-only', '--inputs', '/missing'],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('scoped Native proof', result.stderr)

    def test_cleanup_rejects_reused_pid_after_acquiring_the_kernel_handle(self):
        libc = Mock()
        raw = '123 (same-runtime) ' + ' '.join(['S'] + ['0'] * 18 + ['456'])
        with patch.object(trial, 'open_pidfd', return_value=55) as opened, \
                patch.object(trial.ctypes, 'CDLL', return_value=libc), \
                patch.object(Path, 'resolve', return_value=self.runtime), \
                patch.object(Path, 'read_text', return_value=raw), \
                patch.object(trial.os, 'close') as closed:
            with self.assertRaisesRegex(AssertionError, 'birth identity'):
                trial.stop_owned_pid(123, self.runtime, expected_start='an-earlier-process')
            opened.assert_called_once_with(123)
            libc.syscall.assert_not_called()
            closed.assert_called_once_with(55)

    def test_cleanup_requires_native_jobs_and_requested_host_confirmation(self):
        receipt = {'native_stop_confirmed': True, 'remaining_jobs': [], 'host_stop_requested': False}
        self.assertTrue(q.confirmed_native_stop(receipt))
        self.assertTrue(q.confirmed_native_stop(dict(receipt, host_stop_requested=True, host_stop_confirmed=True)))
        for value in (None, {}, dict(receipt, native_stop_confirmed=False),
                      dict(receipt, native_stop_confirmed=1), dict(receipt, remaining_jobs=None),
                      dict(receipt, remaining_jobs=[{'id': 'original-job'}]),
                      dict(receipt, host_stop_requested=None),
                      dict(receipt, host_stop_requested=True, host_stop_confirmed=False)):
            with self.subTest(value=value):
                self.assertFalse(q.confirmed_native_stop(value))

    def test_failed_cleanup_keeps_original_service_and_native_owners_until_confirmation(self):
        work = {'work_id': 'original-work', 'thread_id': 'original-thread'}
        receipt = {'native_stop_confirmed': True, 'remaining_jobs': [], 'host_stop_requested': False}
        settled = {**work, 'phase': 'stopped', 'native_effects': {'active': [], 'unconfirmed_effects': 0}}
        for replies in ([dict(receipt, native_stop_confirmed=False)],
                        [dict(receipt, remaining_jobs=None)],
                        [dict(receipt, remaining_jobs=[{'id': 'original-job'}])],
                        [TimeoutError('original Stop delivery is unknown')],
                        [receipt, dict(settled, native_effects={'active': [{}], 'unconfirmed_effects': 1})],
                        [receipt, dict(settled, native_effects={})]):
            with self.subTest(replies=replies), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                client, service = Mock(), Mock()
                client.call.side_effect = replies
                with patch.object(q, 'stop_owned_pid') as stop_owner:
                    errors = q.cleanup_compiler_fixture(client, {'a': work}, service, root,
                        False, {123: self.owner}, Path(self.owner['home']), self.runtime)
                self.assertTrue(errors)
                self.assertFalse((root / 'stop-provider').exists())
                service.poll.assert_not_called()
                service.wait.assert_not_called()
                service.terminate.assert_not_called()
                stop_owner.assert_not_called()
                self.assertEqual(client.call.call_args_list[0].args, ('codex_stop', {'work_id': 'original-work'}))
                self.assertEqual(sum(call.args[0] == 'codex_stop' for call in client.call.call_args_list), 1)
                self.assertTrue((root / 'cleanup-unconfirmed.json').is_file())
        with tempfile.TemporaryDirectory() as directory:
            root, client, service = Path(directory), Mock(), Mock()
            self.assertTrue(q.cleanup_compiler_fixture(client, {}, service, root, False, {}, self.home, self.runtime))
            client.call.assert_not_called()
            service.poll.assert_not_called()
            self.assertFalse((root / 'stop-provider').exists())

    def test_confirmed_cleanup_stops_only_the_owned_fixture_service(self):
        work = {'work_id': 'original-work', 'thread_id': 'original-thread'}
        receipt = {'native_stop_confirmed': True, 'remaining_jobs': [], 'host_stop_requested': False}
        status = {**work, 'phase': 'stopped', 'native_effects': {'active': [], 'unconfirmed_effects': 0}}
        for accepted in (False, True):
            with self.subTest(accepted=accepted), tempfile.TemporaryDirectory() as directory:
                root, client, service = Path(directory), Mock(), Mock()
                client.call.side_effect = [receipt, status]
                service.poll.return_value = None
                with patch.object(q, 'stop_owned_pid') as stop_owner:
                    self.assertEqual(q.cleanup_compiler_fixture(client, {'a': work}, service, root,
                        accepted, {}, self.home, self.runtime), [])
                self.assertTrue((root / 'stop-provider').is_file())
                service.wait.assert_called_once_with(timeout=30)
                service.terminate.assert_not_called()
                stop_owner.assert_not_called()
                if accepted:
                    client.call.assert_not_called()
                else:
                    self.assertEqual([call.args[0] for call in client.call.call_args_list], ['codex_stop', 'codex_status'])

    def test_partial_allocation_receipt_loss_preserves_known_and_unknown_owners(self):
        # The first work is known, but the second create may have reached its
        # original Native owner even though its reply was lost.
        known = {'a': {'work_id': 'known-work', 'thread_id': 'known-thread'}}
        for accepted in (False, True):
            with self.subTest(accepted=accepted), tempfile.TemporaryDirectory() as directory:
                root, client, service = Path(directory), Mock(), Mock()
                with patch.object(q, 'stop_owned_pid') as stop_owner:
                    errors = q.cleanup_compiler_fixture(client, known, service, root,
                        accepted, {123: self.owner}, Path(self.owner['home']), self.runtime,
                        creation_pending=True)
                self.assertEqual(errors, ['fixture_work_admission_unconfirmed'])
                client.call.assert_not_called()
                service.poll.assert_not_called()
                service.wait.assert_not_called()
                service.terminate.assert_not_called()
                stop_owner.assert_not_called()
                self.assertFalse((root / 'stop-provider').exists())

    def test_cleanup_keeps_legacy_callers_and_accepts_the_exact_start_identity(self):
        raw = '123 (same-runtime) ' + ' '.join(['S'] + ['0'] * 18 + ['456'])
        for expected in (None, '456'):
            libc, poller = Mock(), Mock()
            libc.syscall.return_value = 0
            poller.select.return_value = [('kernel-confirmed-exit', 1)]
            with self.subTest(expected=expected), patch.object(trial, 'open_pidfd', return_value=55), \
                    patch.object(trial.ctypes, 'CDLL', return_value=libc), \
                    patch.object(Path, 'resolve', return_value=self.runtime), \
                    patch.object(Path, 'read_text', return_value=raw), \
                    patch.object(trial.selectors, 'DefaultSelector', return_value=poller), \
                    patch.object(trial.os, 'close'):
                self.assertEqual(trial.stop_owned_pid(123, self.runtime, expected),
                    {'pid': 123, 'start_ticks': '456', 'kernel_stop_confirmed': True})
                libc.syscall.assert_called_once()
                poller.close.assert_called_once()


if __name__ == '__main__':
    unittest.main()
