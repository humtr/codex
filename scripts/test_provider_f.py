import unittest
import tempfile
import json
import io
from scripts.qualify_provider_chat_entry import control, cli_final_visible, observer_env, require_observer_account, prepare_fixture_observer_profile
from pathlib import Path
from unittest.mock import patch
from scripts.qualify_provider_f import Batch, creation_call_id, input_completed, require_host_capabilities, followup_branch, run


class FDriverTests(unittest.TestCase):
    def test_creation_keys_are_stable_within_a_run_and_distinct_between_runs(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = Path(directory) / "first", Path(directory) / "second"
            first.mkdir()
            second.mkdir()
            key = creation_call_id(first, "managed-mcp")
            self.assertEqual(key, creation_call_id(first / ".", "managed-mcp"))
            self.assertNotEqual(key, creation_call_id(second, "managed-mcp"))
            self.assertNotEqual(key, creation_call_id(first, "managed-local"))
            self.assertLessEqual(len(key), 160)

    def test_fixture_observer_profile_is_explicit_idempotent_and_preserves_existing_files(self):
        import os
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as directory:
            root=Path(directory);home=root/'role';home.mkdir()
            routing=b'# codex-chatgpt-provider generated v1\nmodel_provider="chatgpt_desktop"\n[model_providers.chatgpt_desktop]\nrequires_openai_auth=false\n'
            self.assertFalse(prepare_fixture_observer_profile(home,root,routing)['reused'])
            self.assertTrue(prepare_fixture_observer_profile(home,root,routing)['reused'])
            (home/'config.toml').write_bytes(b'user settings')
            with self.assertRaisesRegex(AssertionError,'existing_fixture_config_preserved'):
                prepare_fixture_observer_profile(home,root,routing)
            self.assertEqual((home/'config.toml').read_bytes(),b'user settings')
            with self.assertRaises(AssertionError):prepare_fixture_observer_profile(root,home,routing)

    def test_cli_preflight_uses_native_auth_requirement_without_copying_credentials(self):
        require_observer_account({'requiresOpenaiAuth':False,'account':None})
        require_observer_account({'requiresOpenaiAuth':True,'account':{'type':'chatgpt'}})
        with self.assertRaisesRegex(AssertionError,'observer_native_login_required'):
            require_observer_account({'requiresOpenaiAuth':True,'account':None})

    def test_cli_final_can_repeat_its_opening_layout_without_losing_any_content(self):
        final='<row align="center">\nFirst result\n</row>\n<row align="center">\nSecond result\n</row>'
        self.assertTrue(cli_final_visible(final,final))
        self.assertFalse(cli_final_visible(final.replace('First result','Missing'),final))
        self.assertFalse(cli_final_visible(final.replace('Second result','Missing'),final))

    def test_cli_final_preserves_mixed_inline_code_rendering_and_exact_fenced_code(self):
        final='**`receipt.json` recorded**\n```text\na = `literal`\n```'
        self.assertTrue(cli_final_visible('**`receipt.json` recorded**\na = `literal`',final))
        self.assertTrue(cli_final_visible('receipt.json recorded\na = `literal`',final))
        self.assertFalse(cli_final_visible('receipt.json recorded\na = literal',final))

    def test_cli_final_renders_paired_asterisks_without_losing_regex_or_literal_code(self):
        final='NAME matches [a-z][a-z0-9]*(?:-[a-z0-9]+)*, limit 2**63-1.\nCode: `a*b*c`.'
        rendered='NAME matches [a-z][a-z0-9](?:-[a-z0-9]+), limit 2**63-1.\nCode: a*b*c.'
        self.assertTrue(cli_final_visible(rendered,final))
        self.assertFalse(cli_final_visible(rendered.replace('0-9','0-8'),final))
        self.assertFalse(cli_final_visible(rendered.replace('2**63-1','263-1'),final))
        self.assertFalse(cli_final_visible(rendered.replace('a*b*c','abc'),final))

    def test_cli_observer_uses_actual_execution_home_without_mutating_caller_environment(self):
        env={'CODEX_HOME':'/base/home','PATH':'/owned/bin'}
        actual=observer_env(env,{'execution':{'home':'/role/home'}})
        self.assertEqual(actual,{'CODEX_HOME':'/role/home','PATH':'/owned/bin'})
        self.assertEqual(env['CODEX_HOME'],'/base/home')

    def test_source_presentation_failure_retains_elapsed_time_without_resubmission(self):
        batch=object.__new__(Batch);batch.client=unittest.mock.Mock();batch.steps=[]
        with tempfile.TemporaryDirectory() as directory:
            batch.root=Path(directory)
            with patch.object(batch,'source_wait',side_effect=RuntimeError('title_preservation_unconfirmed')):
                with self.assertRaisesRegex(RuntimeError,'title_preservation'):
                    batch.source_task('original','task','DONE',1)
            self.assertEqual(batch.client.call.call_args_list,[unittest.mock.call('snapshot'),
                unittest.mock.call('submit',{'step':1,'text':'task'})])
            self.assertEqual(len(batch.steps),1)
            self.assertEqual(batch.steps[0]['outcome'],'submission_or_observation_failed')
            self.assertGreaterEqual(batch.steps[0]['elapsed_seconds'],0)

    def test_released_source_reopens_once_before_first_submission(self):
        batch=object.__new__(Batch);batch.client=unittest.mock.Mock();batch.steps=[]
        batch.client.call.return_value={'source_released':True,'submissions':[],'requests':[]}
        with tempfile.TemporaryDirectory() as directory:
            batch.root=Path(directory)
            with patch.object(batch,'source_wait',return_value={'saved':True}):
                self.assertEqual(batch.source_task('direct','task','DONE',1),{'saved':True})
                with self.assertRaises(FileExistsError):batch.source_task('direct','task','DONE',1)
            self.assertEqual(batch.client.call.call_args_list,[unittest.mock.call('snapshot'),
                unittest.mock.call('new_source'),unittest.mock.call('submit',{'step':1,'text':'task'})])

    def test_uncertain_source_snapshot_never_opens_or_submits(self):
        batch=object.__new__(Batch);batch.client=unittest.mock.Mock();batch.steps=[]
        batch.client.call.side_effect=RuntimeError('observation_unknown')
        with tempfile.TemporaryDirectory() as directory:
            batch.root=Path(directory)
            with self.assertRaisesRegex(RuntimeError,'observation_unknown'):
                batch.source_task('direct','task','DONE',1)
            batch.client.call.assert_called_once_with('snapshot')

    def test_static_inputs_are_rehashed_on_change_and_forced_final_review(self):
        import os
        import hashlib
        from scripts.qualify_provider_f import sha
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);target=root/'static';target.write_bytes(b'original')
            (root/'sealed-inputs.json').write_text(json.dumps({'static':hashlib.sha256(target.read_bytes()).hexdigest()}))
            batch=object.__new__(Batch);batch.root=root;batch.plan={};batch.deadline=float('inf')
            with patch('scripts.qualify_provider_f.sha',wraps=sha) as hashed:
                batch.check();batch.check();self.assertEqual(hashed.call_count,1)
                batch.check(force=True);self.assertEqual(hashed.call_count,2)
                import time
                old=target.stat();time.sleep(.02)  # Cross the filesystem timestamp tick.
                target.write_bytes(b'modified');os.utime(target,ns=(old.st_atime_ns,old.st_mtime_ns))
                with self.assertRaisesRegex(RuntimeError,'sealed_input_changed'):
                    batch.check()

    def test_wall_deadline_expires_even_when_monotonic_time_has_not_reached_its_limit(self):
        batch=object.__new__(Batch);batch.plan={'deadline_unix_ms':90000};batch.deadline=1000
        with patch('scripts.qualify_provider_f.time.monotonic',return_value=10), \
                patch('scripts.qualify_provider_f.time.time',return_value=100), \
                self.assertRaisesRegex(RuntimeError,'batch_deadline_exhausted'):
            batch.check()

    def test_natural_source_completion_is_not_mistaken_for_cancelled_stream(self):
        batch=object.__new__(Batch)
        with tempfile.TemporaryDirectory() as directory:
            batch.root=Path(directory);batch.plan={};batch.client=unittest.mock.Mock()
            snapshot={'accepted':True,'pending':False,'streamActive':False,'localStreamStopped':True,'terminals':['done'],
                'messages':[{'author':{'role':'assistant'},'end_turn':True,'content':{'parts':['COMPLETE']}}]}
            batch.client.call.side_effect=lambda action,*args: snapshot if action=='snapshot' else {'saved':True}
            with patch.object(batch,'check'),patch.object(batch,'drain'),patch.object(batch,'observe_current'):
                self.assertEqual(batch.source_wait('owned','COMPLETE'),{'saved':True})

    def test_aborted_local_source_is_preserved_without_waiting_for_total_deadline(self):
        batch=object.__new__(Batch)
        with tempfile.TemporaryDirectory() as directory:
            batch.root=Path(directory);batch.plan={};batch.client=unittest.mock.Mock()
            batch.client.call.return_value={'localStreamStopped':True,'terminals':['aborted']}
            with patch.object(batch,'check'),patch.object(batch,'drain'),patch.object(batch,'observe_current'), \
                    self.assertRaisesRegex(RuntimeError,'source_stopped_before_final_receipt'):
                batch.source_wait('owned','COMPLETE')

    def test_snapshot_rewrites_and_stream_flags_do_not_reset_stalled_activity_budget(self):
        batch=object.__new__(Batch);batch.root=Path('/owned');batch.plan={'max_no_progress_seconds':10}
        with patch('scripts.qualify_provider_f.native_work',return_value=[]), \
                patch('scripts.qualify_provider_f.time.monotonic',side_effect=[0,9,11]):
            batch.check_progress({'messages':[],'streamActive':True})
            batch.check_progress({'messages':[],'streamActive':False})
            with self.assertRaisesRegex(RuntimeError,'activity_stalled'):
                batch.check_progress({'messages':[],'streamActive':True})

    def test_new_public_output_resets_activity_budget_without_declaring_completion(self):
        batch=object.__new__(Batch);batch.root=Path('/owned');batch.plan={'max_no_progress_seconds':10}
        with patch('scripts.qualify_provider_f.native_work',return_value=[]), \
                patch('scripts.qualify_provider_f.time.monotonic',side_effect=[0,9,18]):
            batch.check_progress({'messages':[]})
            batch.check_progress({'messages':[{'id':'new-public-output'}]})
            batch.check_progress({'messages':[{'id':'new-public-output'}]})
        self.assertEqual(batch._progress[1],9)

    def test_observation_failure_preserves_already_completed_native_work_without_stop(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            batch = unittest.mock.Mock()
            batch.plan, batch.steps, batch.expected_stops = {}, [], set()
            batch.check.side_effect = RuntimeError("observation budget exhausted")
            batch.client.call.return_value = {}
            with patch("scripts.qualify_provider_f.Batch",return_value=batch), \
                    patch("scripts.qualify_provider_f.native_work",return_value=[{"id":"done","phase":"completed","pending":[]},{"id":"unknown-stop","phase":"stop_requested","pending":[]}]), \
                    patch("scripts.qualify_provider_f.control") as control_request, \
                    patch("scripts.qualify_provider_f.notify"):
                run(root)
            control_request.assert_not_called()
            self.assertEqual(json.loads((root / "failure-owned-native-stops-private.json").read_text()),
                [{"work_id":"done","state":"completed_preserved","native_stop_requested":False},
                 {"work_id":"unknown-stop","state":"stop_requested_preserved","native_stop_requested":False}])

    def test_saved_retry_wait_makes_no_saved_get_or_mutation_and_keeps_local_protocol_responsive(self):
        batch = object.__new__(Batch)
        batch.deadline = 1000
        batch.client = unittest.mock.Mock()
        batch.client.call.return_value = {}
        clock = [0.0]
        def advance(seconds):
            clock[0] += seconds
        with patch.object(batch,"check"), patch.object(batch,"drain"), \
                patch.object(batch,"observe_current") as observe, \
                patch.object(batch,"rpc") as rpc, \
                patch("scripts.qualify_provider_f.time.monotonic",side_effect=lambda:clock[0]), \
                patch("scripts.qualify_provider_f.time.sleep",side_effect=advance):
            batch.wait_for_saved_retry(60,"original")
        self.assertEqual(clock[0],60)
        self.assertEqual(observe.call_count,12)
        rpc.assert_not_called()
        self.assertTrue(all(c.args==("saved_snapshot",{"input_id":"original"}) for c in batch.client.call.call_args_list))

    def test_completed_work_with_original_observer_is_not_polled_again(self):
        batch = object.__new__(Batch)
        batch.root, batch.plan, batch.observers = Path("/owned"), {}, {"done":object()}
        with patch("scripts.qualify_provider_f.native_work",return_value=[{"id":"done","phase":"completed"},{"id":"active","phase":"running"}]):
            self.assertEqual([r["id"] for r in batch.current_records()],["active"])

    def test_original_cli_code_fences_do_not_require_language_labels_or_remove_code_comments(self):
        text='## Done\n\n```bash\necho ok\n# actual comment\n```\n\nPassed.'
        self.assertTrue(cli_final_visible('Done\necho ok\n# actual comment\nPassed.',text))
        self.assertFalse(cli_final_visible('Done\necho ok\nactual comment\nPassed.',text))

    def test_continuation_runs_unstarted_delegation_once_in_selected_order(self):
        for general_first in [True, False]:
            with self.subTest(general_first=general_first), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name in ["development", "general", "followup"]:
                    (root / ("task-" + name + ".txt")).write_text("owned task")
                batch = unittest.mock.Mock()
                batch.plan = {"resume_source":{"conversation_id":"saved","work_id":"work"},
                    "general_first":general_first,"followup_source_branch":"public-final","assignment_budget":{}}
                batch.steps, batch.control_observations = [], 0
                batch.read.return_value = [{"cursor":{},"messages":[]}]
                batch.session.side_effect = [{"input_id":"original"}, {"complete":True,"input":{"phase":"completed"}}]
                batch.client.call.return_value = {}
                with patch("scripts.qualify_provider_f.Batch",return_value=batch), \
                        patch("scripts.qualify_provider_f.audit",return_value=[]), \
                        patch("scripts.qualify_provider_f.notify"), \
                        patch("scripts.qualify_provider_f.time.monotonic",side_effect=[0,3]):
                    run(root)
                self.assertTrue(json.loads((root / "result.json").read_text())["automated_gates"])
                batch.source_task.assert_called_once_with("general-mcp","owned task","F_GENERAL_COMPLETE",2)
                calls = [c[0] for c in batch.method_calls]
                self.assertEqual(calls.index("source_task") < calls.index("session"),general_first)
                opens = [c for c in batch.client.call.call_args_list if c.args[0] == "new_source"]
                self.assertEqual(len(opens),0 if general_first else 1)

    def test_followup_selects_explicit_public_final_preserving_hidden_terminal(self):
        public = {"id":"public-final", "role":"assistant", "channel":"final", "end_turn":True,
                  "content":{"parts":["finished"]}, "unavailable":None}
        hidden = {**public,"id":"hidden-terminal","content":None,"unavailable":"hidden"}
        self.assertEqual(followup_branch([{"messages":[public,hidden]}]), "public-final")
        with self.assertRaisesRegex(AssertionError,"settled_public_followup_branch_unavailable"):
            followup_branch([{"messages":[hidden]}])

    def test_followup_observes_current_assignments_without_old_preflight_or_completed_work(self):
        batch = object.__new__(Batch)
        batch.root = Path("/owned-fixture")
        batch.plan = {"completed_prior_work":{"old":{}},"preflight_stopped_work":["empty"]}
        with patch("scripts.qualify_provider_f.native_work",return_value=[{"id":i} for i in ["old","empty","direct","new"]]):
            self.assertEqual([r["id"] for r in batch.current_records()],["direct","new"])


    def test_required_saved_input_capability_is_checked_before_inference(self):
        for value in [{}, {"host": None}, {"host": {"capabilities": {"saved_input": False}}}]:
            with self.assertRaisesRegex(AssertionError, "required_host_capability_missing:saved_input"):
                require_host_capabilities(value, ["saved_input"])
        require_host_capabilities({"host": {"capabilities": {"saved_input": True, "saved_read": True}}}, ["saved_input", "saved_read"])

    def test_original_cli_renders_every_markdown_table_cell_in_order(self):
        final = "## Completed\n\n| File | Result |\n|---|---|\n| `test_original.py` | **unchanged** |\n\nAll 12 tests passed."
        screen = "earlier unrelated output\n\x1b[1mCompleted\x1b[0m\nFile │ Result\ntest_original.py │ unchanged\nAll 12 tests passed."
        self.assertTrue(cli_final_visible(screen, final))
        self.assertFalse(cli_final_visible(screen.replace("unchanged", "changed"), final))
        self.assertFalse(cli_final_visible(screen.replace("test_original.py │ unchanged", "unchanged │ test_original.py"), final))
        self.assertFalse(cli_final_visible("unchanged\n" + screen.replace(" │ unchanged", ""), final))

    def test_unavailable_host_observation_is_not_an_approval_or_error(self):
        batch = object.__new__(Batch)
        for status in [{}, {"managed_lane": None, "host_observation": None},
                       {"managed_lane": {"host_block": None}, "host_observation": None},
                       {"managed_lane": {}, "host_observation": {}}]:
            with patch("scripts.qualify_provider_f.subprocess.check_output") as token, \
                    patch("scripts.qualify_provider_f.urllib.request.urlopen") as send:
                batch.approve({"thread_id": "owned-thread"}, status)
                token.assert_not_called()
                send.assert_not_called()

    def test_managed_cycle_accepts_real_null_observation_shape_without_supervisory_polling(self):
        batch = object.__new__(Batch)
        with tempfile.TemporaryDirectory() as temporary:
            batch.root = Path(temporary)
            batch.plan = {"assignment_budget": {"max_parallel": 2, "max_app_starts": 12,
                "max_native_calls": 100, "max_elapsed_seconds": 1200}}
            batch.works, batch.steps = {}, []
            work = {"work_id": "a" * 48, "thread_id": "owned-thread", "controller": "fake-controller"}
            status = {"managed_lane": None, "host_observation": None, "pending_calls": [], "human_requests": []}
            replies = [work.copy(), {"controller": "new-fake-controller"},
                {"wait_reason": "pending", "status": status},
                {"wait_reason": "completed", "status": {**status, "completed": True}}]
            with patch.object(batch, "session", side_effect=replies) as request, \
                    patch.object(batch, "observe"), patch.object(batch, "event"), \
                    patch.object(batch, "check"), patch.object(batch, "drain"), \
                    patch("scripts.qualify_provider_f.subprocess.check_output") as token:
                result = batch.managed("mcp", "Owned synthetic task")
                self.assertEqual(result["controller"], "new-fake-controller")
                self.assertEqual([c.kwargs["action"] for c in request.call_args_list], ["create", "start", "wait", "wait"])
                self.assertEqual(len(batch.steps), 1)
                token.assert_not_called()
                self.assertTrue((batch.root / "mcp-intent.json").is_file())
                self.assertTrue((batch.root / "mcp-complete-private.json").is_file())

    def test_source_wait_handles_current_worker_confirmation_without_supervisor(self):
        batch = object.__new__(Batch)
        batch.plan = {}
        with tempfile.TemporaryDirectory() as directory:
            batch.root = Path(directory)
            batch.client = unittest.mock.Mock()
            batch.client.call.side_effect = [{"accepted":True,"pending":False,"streamActive":False,
                "messages":[{"author":{"role":"assistant"},"end_turn":True,
                    "content":{"parts":["COMPLETE"]}}]}, {"saved":True}]
            with patch.object(batch,"check"), patch.object(batch,"drain"), \
                    patch.object(batch,"current_records",return_value=[{"id":"current","thread":"thread","native_turn":"turn"}]), \
                    patch.object(batch,"observe"), patch.object(batch,"rpc",return_value={"host_observation":{"waiting_for":"mcp_host_confirmation_required"}}), \
                    patch.object(batch,"approve") as approve:
                self.assertEqual(batch.source_wait("general","COMPLETE"),{"saved":True})
                approve.assert_called_once()
                self.assertEqual(approve.call_args.args[0]["work_id"],"current")

    def test_observer_waits_for_explicit_native_claim_to_load_thread(self):
        batch = object.__new__(Batch)
        batch.plan, batch.observers = {}, {}
        batch.root = Path("/owned-fixture")
        with patch("scripts.qualify_provider_f.native_work", return_value=[{"id":"work","native_turn":"turn"}]), \
                patch.object(batch,"rpc",return_value={"native_status":{"type":"notLoaded"}}), \
                patch("scripts.qualify_provider_f.subprocess.Popen") as spawn:
            batch.observe({"work_id":"work"})
            spawn.assert_not_called()

    def test_observer_rebinds_changed_execution_owner_without_stopping_backend(self):
        batch = object.__new__(Batch)
        batch.plan, batch.info, batch.env = {}, {"workspace":"/owned-workspace"}, {}
        old = unittest.mock.Mock()
        old.poll.return_value = None
        batch.observers = {"work":(old,11)}
        with tempfile.TemporaryDirectory() as directory:
            batch.root = Path(directory)
            (batch.root/'work-cli-binding.json').write_text(json.dumps({"execution":{"directory":"old-owner"}}))
            record = {"id":"work","thread":"thread","native_turn":"turn","execution":{"directory":"new-owner","home":"/role/home"}}
            with patch("scripts.qualify_provider_f.native_work",return_value=[record]), \
                    patch.object(batch,"rpc",return_value={"native_status":{"type":"idle"}}), \
                    patch.object(batch,"drain"), patch("scripts.qualify_provider_f.pty.openpty",return_value=(99,100)), \
                    patch("scripts.qualify_provider_f.fcntl.ioctl"), patch("scripts.qualify_provider_f.os.close"), \
                    patch("scripts.qualify_provider_f.observer_argv",return_value=["/owned/core","--remote","unix:///new-owner/s"]), \
                    patch("scripts.qualify_provider_f.subprocess.Popen") as spawn:
                batch.observe({"work_id":"work"})
                old.terminate.assert_called_once()
                self.assertIs(batch.observers['work'][0],spawn.return_value)
                self.assertEqual(spawn.call_args.kwargs['env']['CODEX_HOME'],'/role/home')
                self.assertEqual(json.loads((batch.root/'work-cli-binding.json').read_text())['execution'],record['execution'])
                self.assertEqual(len(list(batch.root.glob('work-cli-binding-prior-*.json'))),1)

    def test_transient_saved_read_is_observed_afresh_without_retrying_effects(self):
        batch = object.__new__(Batch)
        batch.plan, batch.info, batch.env = {"max_control_observations":3}, {}, {}
        batch.control_observations = 0
        with tempfile.TemporaryDirectory() as directory:
            batch.root = Path(directory)
            def first_failed(*args, **kwargs):
                kwargs['evidence_path'].write_text(json.dumps({'isError':True,'content':[{'text':json.dumps({'error':'app_bridge_operation_failed'})}]}))
                if batch.control_observations == 1:
                    raise AssertionError('read rejected')
                return {'complete':False}
            with patch.object(batch,'check'),patch.object(batch,'event'),patch('scripts.qualify_provider_f.time.sleep'), \
                    patch('scripts.qualify_provider_f.control',side_effect=first_failed) as request:
                self.assertEqual(batch.rpc('codex_sessions',{'action':'chat_result','input_id':'owned'}),{'complete':False})
                self.assertEqual(request.call_count,2)
                batch.control_observations=0
                with self.assertRaisesRegex(AssertionError,'read rejected'):
                    batch.rpc('codex_sessions',{'action':'start'})
                self.assertEqual(request.call_count,3)

    def test_source_receipt_requires_saved_original_completion(self):
        for value in [{}, {"phase": "completed"}, {"complete": True}, {"complete": True, "input": None},
                      {"complete": True, "input": {"phase": "delivery_unknown"}},
                      {"complete": False, "input": {"phase": "completed"}}]:
            self.assertFalse(input_completed(value))
        self.assertTrue(input_completed({"complete": True, "input": {"phase": "completed"}}))

    def test_control_error_is_preserved_and_never_retried(self):
        batch = object.__new__(Batch)
        batch.control_observations = 0
        batch.plan = {"max_control_observations": 2}
        batch.info, batch.env = {}, {}
        batch.root = Path("/owned-fixture")
        with patch.object(batch, "check"), patch.object(batch, "event") as events, \
                patch("scripts.qualify_provider_f.control", side_effect=RuntimeError("uncertain")) as request:
            with self.assertRaisesRegex(RuntimeError, "uncertain"):
                batch.rpc("codex_sessions", {"action": "start"})
            request.assert_called_once()
            self.assertEqual(request.call_args.kwargs["evidence_path"], batch.root / "control-0001-private.json")
            events.assert_called_once()
            self.assertEqual(events.call_args.args[0], "control_failed_preserved")

    def test_active_stop_requires_real_stream_and_fresh_terminal_proof(self):
        batch = object.__new__(Batch)
        with tempfile.TemporaryDirectory() as temporary:
            batch.root = Path(temporary)
            (batch.root / "task-server-stop.txt").write_text("owned task")
            (batch.root / "w-server-stop").mkdir()
            (batch.root / "w-server-stop/F_STOP_READY").touch()
            batch.deadline = float("inf")
            batch.plan = {"stop_probe_timeout_seconds": 10}
            batch.expected_stops, batch.steps = set(), []
            work = {"work_id": "a" * 48}
            status = {"managed_lane": {"conversation_id": "owned-chat"}, "completed": False}
            stop = {"native_stop_confirmed": True, "host_stop_confirmed": True, "remaining_jobs": []}
            client = unittest.mock.Mock()
            batch.client = client
            client.call.side_effect = [
                {"streamActive": True, "stopRequested": False}, {"status": "IS_STREAMING"},
                {"serverStopState": "terminal_observed", "streamActive": False, "pending": False}, {"status": "COMPLETE"}]
            with patch.object(batch, "created", return_value=work), \
                    patch.object(batch, "start", return_value={"monotonic_ns": 1}), \
                    patch.object(batch, "check"), patch.object(batch, "drain"), patch.object(batch, "approve"), \
                    patch.object(batch, "rpc", side_effect=[status, stop]) as rpc:
                batch.active_server_stop()
            self.assertEqual([c.args[0] for c in rpc.call_args_list], ["codex_status", "codex_stop"])
            self.assertIn(work["work_id"], batch.expected_stops)
            self.assertTrue(batch.steps[0]["expected_cancellation"])
            evidence = json.loads((batch.root / "active-server-stop-after.json").read_text())
            self.assertEqual(evidence["server"]["status"], "COMPLETE")

    def test_observation_budget_checks_before_dispatch(self):
        batch = object.__new__(Batch)
        batch.control_observations = 2
        batch.plan = {"max_control_observations": 2}
        batch.info, batch.env = {}, {}
        with patch.object(batch, "check"), patch("scripts.qualify_provider_f.control") as request:
            with self.assertRaisesRegex(AssertionError, "budget_exhausted"):
                batch.rpc("codex_status", {})
            request.assert_not_called()

    def test_rejected_original_control_response_is_retained_without_replay(self):
        with tempfile.TemporaryDirectory() as temporary:
            evidence = Path(temporary) / "original-rejected-private.json"
            result = {"isError": True, "content": [{"type": "text", "text": '{"error":"native_request_rejected"}'}]}
            with patch("scripts.qualify_provider_chat_entry.subprocess.check_output", return_value="owned-fixture"), \
                    patch("scripts.qualify_provider_chat_entry.urllib.request.urlopen", return_value=io.StringIO(json.dumps({"result": result}))) as send:
                with self.assertRaisesRegex(AssertionError, "codex_status rejected"):
                    control({"binary": "fixture", "state": "fixture", "port": 1}, {}, "codex_status", {}, evidence_path=evidence)
            send.assert_called_once()
            self.assertEqual(json.loads(evidence.read_text()), result)


if __name__ == "__main__":
    unittest.main()
