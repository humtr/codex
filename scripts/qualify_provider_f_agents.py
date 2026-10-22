"""Bounded F agent fixture controls and terminal audit, using the common driver.

These hooks only answer the declared fixture question and inject its declared
Stop/barrier. Development, review, correction and integration use upstream tools.
"""
import datetime
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path


def role(record):
    return (record.get('assignment') or {}).get('role')


def validate_control_arguments(tools, name, args):
    schema = next(v['inputSchema'] for v in tools if v['name'] == name)
    assert set(args) <= set(schema['properties']), 'fixture_unknown_control_argument:' + name
    assert set(schema.get('required', [])) <= set(args), 'fixture_required_control_argument:' + name


def terminal_record(record, cancelled):
    if cancelled:
        # Stop preserves the original offered call as cancelled history. A
        # missing result must remain visible; it is not a live job or success.
        return record['phase'] == 'stopped'
    return not record['pending'] and record['phase'] in ('completed','stopped')


def canonical_item(item):
    """Canonical rollout uses Rust enum/snake fields; API history uses camel case."""
    value = dict(item)
    value['type'] = {'CommandExecution':'commandExecution', 'AgentMessage':'agentMessage'}.get(value.get('type'), value.get('type'))
    for original, public in [('aggregated_output','aggregatedOutput'),('exit_code','exitCode')]:
        if original in value:
            value[public] = value[original]
    return value


def observe(batch, record, status):
    from qualify_provider_chat_entry import native_work
    from qualify_provider_f import save, stamp
    if role(record) == 'agent-coordinator':
        for question in status.get('human_requests', []):
            offered = question.get('params', {}).get('questions', [])
            assert len(offered) == 1 and offered[0]['id'] == 'f_validation_policy', 'unexpected_fixture_question'
            labels = [v['label'] for v in offered[0].get('options', [])]
            assert 'Strict (Recommended)' in labels, 'declared_fixture_choice_missing'
            key = hashlib.sha256(json.dumps(question['id'], sort_keys=True).encode()).hexdigest()
            intent = batch.root / ('question-answer-' + key + '-intent.json')
            if intent.exists():
                continue  # Fresh status may reconcile this answer; never resend unknown delivery.
            args = {'work_id':record['id'], 'controller':record['capability'],
                'request_id':json.dumps(question['id'],separators=(',', ':')),
                'answers':{'f_validation_policy':{'answers':['Strict (Recommended)']}}}
            save(intent, {'request_id':args['request_id'], **stamp()}, True)
            value = batch.rpc('codex_answer', args)
            save(batch.root / ('question-answer-' + key + '-private.json'), value, True)
            batch.event('declared_native_question_answered', work_id=record['id'])
    if role(record) != 'parallel-writer-a':
        return
    fixture = Path(batch.plan['agent_fixture']['root'])
    ready_a = fixture / 'writer-a/READY.signal'
    ready_b = fixture / 'writer-b/BEFORE_STOP.signal'
    intent = batch.root / 'declared-writer-stop-intent.json'
    if intent.exists() or not ready_a.exists() or not ready_b.exists():
        return
    sibling = next((r for r in native_work(batch.root) if role(r) == 'parallel-writer-b'
        and Path(r['workspace']) == fixture / 'writer-b'), None)
    if sibling is None or sibling['phase'] not in ('running', 'waiting_native'):
        return
    before = batch.rpc('codex_status', {'work_id':sibling['id']})
    assert not before['completed'], 'sibling_finished_before_declared_stop'
    save(intent, {'work_id':record['id'], 'sibling_work_id':sibling['id'],
        'both_native_workspaces_observed_active':True, **stamp()}, True)
    # This one declared cancellation is not a failed request retry.
    batch.expected_stops.add(record['id'])
    stopped = batch.rpc('codex_stop', {'work_id':record['id']})
    save(batch.root / 'declared-writer-stop-private.json', stopped, True)
    assert stopped['native_stop_confirmed'] and not stopped['remaining_jobs'], 'declared_native_stop_unconfirmed'
    # A test-protocol signal, never attributed to a model's implementation.
    signal = fixture / 'writer-b/RELEASE.signal'
    with signal.open('x') as stream:
        stream.write('Runner confirmed sibling native Stop; continue this independent work.\n')
    save(batch.root / 'sibling-release.json', {'work_id':sibling['id'],
        'native_stop_confirmed':True,'host_stop_confirmed':stopped['host_stop_confirmed'], **stamp()}, True)
    batch.event('declared_stop_released_independent_sibling', work_id=record['id'], sibling=sibling['id'])


def perform(root, batch):
    from qualify_provider_f import save
    receipt_check = bool(batch.plan.get('agent_receipt_check'))
    continued = bool(batch.plan.get('agent_continuation'))
    marker = 'F_AGENT_RECEIPTS_DONE' if receipt_check else 'F_AGENTS_CONTINUATION_DONE' if continued else 'F_AGENTS_DONE'
    source = batch.source_task('agents', (root / 'task-agents.txt').read_text(), marker, 1)
    batch.client.call('release_source')
    # End-only output flush and audit; no supervisory model is used between roles.
    until = time.monotonic() + 2
    while time.monotonic() < until:
        batch.drain()
        time.sleep(.05)
    batch.check(force=True)
    report = audit_receipt_check(root, batch, source) if receipt_check else audit_continuation(root, batch, source) if continued else audit(root, batch, source)
    batch.check(force=True)
    save(root / 'agent-terminal-audit.json', report, True)
    return report


def receipt_assignments(records):
    roles = {role(r):r for r in records}
    assert len(records) == len(roles) == 3 and set(roles) == {'receipt-root','receipt-verifier','receipt-dependent'}, 'receipt_roles_incomplete'
    assert all(r['phase'] == 'stopped' and not r['pending'] for r in records), 'receipt_tree_unsettled'
    verifier = roles['receipt-verifier']
    evaluation = verifier['assignment'].get('evaluation') or {}
    assert evaluation.get('decision') == 'accepted' and evaluation.get('findings'), 'receipt_verifier_not_evaluated'
    assert verifier['id'] in roles['receipt-dependent']['assignment']['dependencies'], 'receipt_dependency_missing'
    return roles


def audit_receipt_check(root, batch, source, *, cli_root=None):
    from qualify_provider_chat_entry import native_work, cli_final_visible
    from qualify_provider_f import sha
    fixture = Path(batch.plan['agent_fixture']['root'])
    roles = receipt_assignments([r for r in native_work(root) if Path(r['workspace']).parent == fixture])
    for row in json.loads((root/'prior-proof.json').read_text())['native_prefixes']:
        with Path(row['path']).open('rb') as stream:
            assert hashlib.sha256(stream.read(row['bytes'])).hexdigest() == row['sha256'], 'prior_native_history_changed'
    reports = []
    for name, record in roles.items():
        logs = list((Path(record['execution']['home'])/'sessions').rglob('*'+record['thread']+'.jsonl'))
        assert len(logs) == 1
        rows = [json.loads(v) for v in logs[0].read_text().splitlines()]
        ended = [v['payload'] for v in rows if v['type']=='event_msg' and v['payload'].get('type')=='task_complete']
        calls = {v['payload']['call_id'] for v in rows if v['type']=='response_item' and v['payload'].get('type') in ('function_call','custom_tool_call')}
        returns = {v['payload']['call_id'] for v in rows if v['type']=='response_item' and v['payload'].get('type') in ('function_call_output','custom_tool_call_output')}
        cli = (cli_root or root)/(record['id']+'-original-cli.txt')
        assert ended and not ended[-1].get('error') and isinstance(ended[-1].get('last_agent_message'),str), 'receipt_native_final_failed:'+name
        assert calls and not calls-returns, 'receipt_native_effect_unconfirmed:'+name
        assert cli.is_file() and cli_final_visible(cli.read_text(errors='replace'),ended[-1]['last_agent_message']), 'receipt_cli_final_missing:'+name
        reports.append({'role':name,'work_id':record['id'],'thread_id':record['thread'],'native_calls':len(calls),'unreturned_native_calls':0,'rollout_sha256':sha(logs[0]),'original_cli':str(cli),'original_cli_sha256':sha(cli)})
    assert 'F_AGENT_RECEIPTS_DONE' in json.dumps(source.get('public_messages',[]),ensure_ascii=False)
    return {'automated_fixture_gates':True,'scope':'evaluated correction receipt and dependent release; prior development preserved',
        'reports':reports,'evaluated_correction_receipt':True,'dependent_native_completion':True,
        'tree_work_count':3,'whole_tree_durable_stop_confirmed':True,'full_f_accepted':False}


def audit_continuation(root, batch, source):
    from qualify_provider_chat_entry import native_work, cli_final_visible
    from qualify_provider_f import sha
    fixture = Path(batch.plan['agent_fixture']['root'])
    roles = {role(r):r for r in native_work(root) if Path(r['workspace']).parent == fixture}
    assert len(roles) == 8 and all(r['phase'] == 'stopped' for r in roles.values()), 'continued_tree_stop_unsettled'
    prior = json.loads((root / 'prior-proof.json').read_text())
    for row in prior['native_prefixes']:
        with Path(row['path']).open('rb') as stream:
            assert hashlib.sha256(stream.read(row['bytes'])).hexdigest() == row['sha256'], 'prior_native_history_changed'
    reports = []
    for name in ('agent-coordinator','agent-integrator','correction-verifier','tree-stop-canary'):
        record = roles[name]
        logs = list((Path(record['execution']['home'])/'sessions').rglob('*'+record['thread']+'.jsonl'))
        assert len(logs) == 1
        rows = [json.loads(v) for v in logs[0].read_text().splitlines()]
        ended = [v['payload'] for v in rows if v['type']=='event_msg' and v['payload'].get('type')=='task_complete']
        calls = {v['payload']['call_id'] for v in rows if v['type']=='response_item' and v['payload'].get('type') in ('function_call','custom_tool_call')}
        returns = {v['payload']['call_id'] for v in rows if v['type']=='response_item' and v['payload'].get('type') in ('function_call_output','custom_tool_call_output')}
        cli = root/(record['id']+'-original-cli.txt')
        assert cli.is_file() and cli.stat().st_size
        if name != 'tree-stop-canary':
            assert ended and not ended[-1].get('error') and isinstance(ended[-1].get('last_agent_message'),str), 'continued_native_turn_failed:'+name
            assert not calls-returns and not record['pending']
            assert cli_final_visible(cli.read_text(errors='replace'),ended[-1]['last_agent_message']), 'continued_original_cli_final_missing:'+name
        if name == 'agent-integrator':
            assert len(ended) >= 4 and record['thread'] == prior['integrator_thread'], 'same_native_transfer_continuity_missing'
        reports.append({'role':name,'work_id':record['id'],'thread_id':record['thread'],
            'completed_turns':len([v for v in ended if not v.get('error')]),
            'native_calls':len(calls),'unreturned_native_calls':len(calls-returns),
            'native_turn_timings':[{k:v.get(k) for k in ('turn_id','started_at','completed_at','duration_ms','error')} for v in ended],
            'original_cli':str(cli),'rollout_sha256':sha(logs[0])})
    workspace = fixture/'integrator'
    assert (workspace/'test_original.py').read_bytes() == (fixture/'baseline/test_original.py').read_bytes()
    assert (workspace/'test_review_regression.py').read_bytes() == (fixture/'reviewer/test_review_regression.py').read_bytes()
    p = subprocess.run([sys.executable,'-B','-m','unittest','discover','-v'],cwd=workspace,env=batch.env,capture_output=True,timeout=60)
    (root/'continued-integrated-tests.log').write_bytes(p.stdout+p.stderr)
    assert p.returncode == 0 and b'Ran 30 tests' in p.stderr, 'continued_integrated_tests_failed'
    subprocess.run(['git','diff','--check'],cwd=workspace,env=batch.env,check=True)
    assert not (fixture/'writer-a/SHOULD_NOT_APPEAR.signal').exists()
    assert (fixture/'writer-b/AFTER_STOP.signal').is_file()
    assert 'F_AGENTS_CONTINUATION_DONE' in json.dumps(source.get('public_messages',[]),ensure_ascii=False)
    return {'automated_fixture_gates':True,'scope':'remaining F1/F2/F4/F5 continuation; original failed stream preserved',
        'reports':reports,'preserved_prior_native_prefixes':len(prior['native_prefixes']),
        'original_corrector_stream_accepted':False,'new_correction_verifier_completed':True,
        'tree_work_count':len(roles),'whole_tree_durable_stop_confirmed':True,
        'app_requests':len(source.get('requests',[])),'billing_observation':'unknown',
        'supervisory_model_calls_between_roles':0,'full_f_accepted':False}


def audit(root, batch, source):
    from qualify_provider_chat_entry import native_work, cli_final_visible
    from qualify_provider_f import save, sha
    fixture = Path(batch.plan['agent_fixture']['root'])
    records = [r for r in native_work(root) if Path(r['workspace']).parent == fixture]
    roles = {role(r):r for r in records}
    assert len(records) == len(roles), 'duplicate_agent_role'
    expected = {'agent-coordinator','parallel-writer-a','parallel-writer-b','agent-integrator',
        'independent-reviewer','agent-corrector','tree-stop-canary'}
    assert set(roles) == expected, 'agent_roles_missing_or_unexpected'
    reports, all_items = [], {}
    for name, record in roles.items():
        rollouts = list((Path(record['execution']['home']) / 'sessions').rglob('*' + record['thread'] + '.jsonl'))
        assert len(rollouts) == 1
        rows = [json.loads(line) for line in rollouts[0].read_text().splitlines()]
        items = [canonical_item(row['payload']['item']) for row in rows if row['type'] == 'event_msg'
            and row.get('payload', {}).get('type') == 'item_completed']
        all_items[name] = items
        completed = [row for row in rows
            if row['type'] == 'event_msg' and row.get('payload', {}).get('type') == 'task_complete']
        finals = [row['payload']['last_agent_message'] for row in completed
            if not row['payload'].get('error') and isinstance(row['payload'].get('last_agent_message'),str)]
        cancelled = name in ('parallel-writer-a','tree-stop-canary')
        calls = {row['payload']['call_id'] for row in rows if row['type'] == 'response_item'
            and row['payload'].get('type') in ('function_call','custom_tool_call')}
        returned = {row['payload']['call_id'] for row in rows if row['type'] == 'response_item'
            and row['payload'].get('type') in ('function_call_output','custom_tool_call_output')}
        cli = root / (record['id'] + '-original-cli.txt')
        assert cli.exists() and cli.stat().st_size, 'original_agent_cli_missing:' + name
        if not cancelled:
            assert completed and not completed[-1]['payload'].get('error'), 'agent_native_turn_failed:' + name
            assert finals and cli_final_visible(cli.read_text(errors='replace'), finals[-1]), 'agent_final_not_in_original_cli:' + name
            assert not calls - returned, 'agent_unreturned_native_call:' + name
        assert terminal_record(record, cancelled), 'agent_terminal_unsettled'
        save(root / (name + '-native-items-private.json'), items, True)
        reports.append({'role':name,'work_id':record['id'],'thread_id':record['thread'],
            'expected_cancellation':cancelled,'canonical_completed_turns':len(finals),
            'canonical_failed_turns':len([row for row in completed if row['payload'].get('error')]),
            'native_calls':len(calls),'unreturned_native_calls':len(calls-returned),
            'retained_pending_call_ids':record['pending'] if cancelled else [],
            'canonical_turn_events':[{'timestamp':row.get('timestamp'),'event':row['payload']['type'],
                'turn_id':row['payload'].get('turn_id')} for row in rows
                if row['type'] == 'event_msg' and row.get('payload',{}).get('type')
                in ('task_started','task_complete','turn_aborted')],
            'original_cli':str(cli),'canonical_rollout_sha256':sha(rollouts[0])})
    # Actual upstream command results, not a coordinator's assertions.
    merged = [v for v in all_items['agent-integrator'] if v.get('type') == 'commandExecution']
    assert any('CONFLICT' in str(v.get('aggregatedOutput','')) and 'README.md' in str(v.get('aggregatedOutput',''))
        and v.get('exitCode') == 1 for v in merged), 'actual_merge_conflict_not_observed'
    reviewed = [v for v in all_items['independent-reviewer'] if v.get('type') == 'commandExecution']
    assert any('FAILED' in str(v.get('aggregatedOutput','')) and v.get('exitCode') == 1 for v in reviewed), 'independent_failing_regression_missing'
    findings = json.loads((fixture / 'reviewer/review.json').read_text())
    assert findings['decision'] == 'request_changes' and findings['findings'], 'concrete_review_missing'
    assert (fixture / 'reviewer/test_review_regression.py').read_bytes() == (fixture / 'integrator/test_review_regression.py').read_bytes()
    final_workspace = fixture / 'integrator'
    assert (final_workspace / 'test_original.py').read_bytes() == (fixture / 'baseline/test_original.py').read_bytes()
    p = subprocess.run([sys.executable,'-B','-m','unittest','discover','-v'], cwd=final_workspace,
        capture_output=True, timeout=60, env=batch.env)
    (root / 'integrated-tests.log').write_bytes(p.stdout+p.stderr)
    assert p.returncode == 0 and b'Ran 0 tests' not in p.stderr, 'integrated_suite_failed'
    checks = '''import json,subprocess,sys
from api import balance_after
values=[-2,5];assert balance_after(4,values)==7 and values==[-2,5]
for b,v in [(True,[]),(-1,[]),(1,[False]),(1,[-2,3]),(1.0,[])]:
 try:balance_after(b,v)
 except ValueError:pass
 else:raise AssertionError('invalid API accepted')
for data in ['[true]','[1.5]','["2"]','{}','{']:
 p=subprocess.run([sys.executable,'-B','cli.py','--balance','4'],input=data,text=True,capture_output=True)
 assert p.returncode==2 and not p.stdout.strip() and 'Traceback' not in p.stderr
p=subprocess.run([sys.executable,'-B','cli.py','--balance','4'],input='[-2,5]',text=True,capture_output=True)
assert p.returncode==0 and json.loads(p.stdout)=={'balance':7}
'''
    p = subprocess.run([sys.executable,'-B','-c',checks],cwd=final_workspace,env=batch.env,capture_output=True,timeout=60)
    (root / 'independent-agent-contract.log').write_bytes(p.stdout+p.stderr)
    assert p.returncode == 0, 'independent_agent_contract_failed'
    subprocess.run(['git','diff','--check'],cwd=final_workspace,env=batch.env,check=True)
    stopped = json.loads((root / 'declared-writer-stop-private.json').read_text())
    assert stopped['native_stop_confirmed'] and stopped['host_stop_confirmed'] and not stopped['remaining_jobs']
    assert (fixture / 'writer-b/AFTER_STOP.signal').is_file(), 'sibling_did_not_continue'
    assert not (fixture / 'writer-a/SHOULD_NOT_APPEAR.signal').exists(), 'cancelled_writer_wrote_after_fence'
    # Durable stopped is committed only after native/jobs and applicable host
    # confirmation. Confirm the whole declared tree, without repeating Stop.
    assert all(r['phase'] == 'stopped' and terminal_record(r, name in ('parallel-writer-a','tree-stop-canary'))
        for name,r in roles.items()), 'whole_tree_stop_unsettled'
    assert list(root.glob('question-answer-*-private.json')), 'real_native_question_missing'
    question_items = json.dumps(all_items['agent-coordinator'],ensure_ascii=False)
    assert 'Strict (Recommended)' in question_items, 'original_native_question_answer_missing'
    assert len([v for v in all_items['agent-integrator'] if v.get('type') == 'agentMessage' and v.get('phase') == 'final_answer']) >= 3, 'same_thread_transfer_continuations_missing'
    public = json.dumps(source.get('public_messages',[]),ensure_ascii=False)
    assert 'F_AGENTS_DONE' in public
    return {'automated_fixture_gates':True,'reports':reports,'independent_review_finding':findings,
        'actual_merge_conflict':True,'correction_and_integration_checked':True,'sibling_continued_after_scoped_stop':True,
        'whole_tree_durable_stop_confirmed':True,'tree_work_count':len(roles),
        'real_native_question_return':True,'app_requests':len(source.get('requests',[])),
        'supervisory_model_calls_between_roles':0,'billing_observation':'unknown',
        'full_f_accepted':False,'scope':'F1/F2/F4/F5 bounded live roles; final independent review and remaining F cases still required'}
