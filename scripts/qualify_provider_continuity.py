"""Actual Android queue/history/TUI proof using saved public Chat fixtures; no Chat inference."""
import argparse
import codecs
import concurrent.futures
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
import urllib.request

from qualify_permission_lifecycle import fixture, rpc as native_rpc, server_paths
from qualify_provider_operator import TOKEN, fixture_auth
from qualify_tasks import Ws


def probe(args):
    display = stream = None
    if args.provider_binary:
        import pyte
        from wcwidth import wcswidth
        display = pyte.Screen(160, 80)
        stream = pyte.Stream(display)
    # Core's Unix socket uses this root; retain the existing short fixture form.
    root = Path(tempfile.mkdtemp(prefix='ct-', dir=os.environ['TMPDIR']))
    home, prefix, env = fixture(root, args.core, args.frontend)
    if args.fresh_auth_home:
        auth = home / '.codex/auth.json'
        assert json.loads(auth.read_text()) == {
            'OPENAI_API_KEY': 'owned-inert-fixture', 'auth_mode': 'apikey'}
        auth.unlink()
        env.pop('OPENAI_API_KEY', None)
    config=home/'.codex/config.toml'
    config.write_text(config.read_text()+'\n[projects.'+json.dumps(str(root/'w'))+']\ntrust_level = "trusted"\n')
    for name in ['owned-fixture', 'model-adapter-fixture', 'bound-source-fixture']:
        (root / name).write_text('Owned synthetic saved-source continuity proof\n')
    if args.direct:
        (root / 'source-project-fixture').write_text('Owned project for explicit Provider route selection\n')
    auth = root / 'fixture-auth'
    fixture_auth(auth, args.provider_binary)
    env['CODEX_PROVIDER_OPERATOR_FIXTURE'] = str(root)
    output = (root / 'provider-test.log').open('wb')
    service = subprocess.Popen([str(args.service_test), '--ignored', '--exact',
        'synthetic_operator_service', '--nocapture'], env=env, stdout=output,
        stderr=subprocess.STDOUT, cwd=root / 'w')
    terminal = None
    master = None
    screen = ''
    decoder = codecs.getincrementaldecoder('utf-8')(errors='replace')
    try:
        def drain():
            nonlocal screen
            if master is not None:
                while select.select([master], [], [], 0)[0]:
                    try:
                        text = decoder.decode(os.read(master, 65536))
                        screen += text
                        if stream is not None:
                            stream.feed(text)
                    except OSError:
                        break
            return screen

        def wait(predicate, description, timeout=40):
            until = time.monotonic() + timeout
            while time.monotonic() < until:
                drain()
                assert service.poll() is None, (root / 'provider-test.log').read_text()
                if terminal is not None:
                    assert terminal.poll() is None, 'TUI exited; inspect ' + str(root)
                result = predicate()
                if result:
                    return result
                time.sleep(.05)
            try:
                (root/'failed-native-observation.json').write_text(json.dumps(
                    native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True}),indent=2))
                (root/'failed-provider-observation.json').write_text(json.dumps(
                    rpc('codex_status',{'work_id':work['work_id']}),indent=2))
            except (NameError, OSError, AssertionError):
                pass
            raise AssertionError(description + '; inspect ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists(), 'Provider readiness')
        port = json.loads((root / 'provider-ready.json').read_text())['port']
        seq = 0

        def rpc(name, arguments, expect_error=False, observation_pending=False):
            nonlocal seq
            seq += 1
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps({'jsonrpc':'2.0','id':seq,'method':'tools/call',
                    'params':{'name':name,'arguments':arguments}}).encode(),
                headers={'Content-Type':'application/json','Authorization':'Bearer ' + TOKEN})
            with urllib.request.urlopen(request, timeout=50) as response:
                result = json.load(response)['result']
            value = json.loads(result['content'][0]['text'])
            if observation_pending and name=='codex_status' and result['isError'] and value.get('error') in ['bound_chat_observation_busy','bound_chat_observation_superseded']:
                # Status can race the same source reconciliation; no input or
                # effect was submitted by this read. Preserve the actual cause.
                with (root/'pending-status-observations.jsonl').open('a') as out:
                    out.write(json.dumps(value)+'\n')
                return None
            if result['isError'] != expect_error:
                with (root/'unexpected-control-errors.jsonl').open('a') as record:
                    record.write(json.dumps({'tool':name,'call_id':arguments.get('call_id'),'value':value})+'\n')
            assert result['isError']==expect_error, value
            return value

        def bridge_commands():
            path = root/'bridge-commands.json'
            # Direct Native startup need not acquire or call any Chat host.
            return json.loads(path.read_text()) if path.exists() else []

        created_before_tui = None
        if args.native_created:
            created_before_tui = rpc('codex_sessions',{'action':'create','reasoning':'direct',
                'call_id':'native-before-tui-create'})
            thread = created_before_tui['thread_id']
            started = rpc('codex_sessions',{'action':'start','work_id':created_before_tui['work_id'],
                'controller':created_before_tui['controller'],'call_id':'native-before-tui-start',
                'task':'Prepare this Native work before opening its ordinary TUI resume.'})
            bound = {'work_id':created_before_tui['work_id'],'controller':started['controller']}
            wait(lambda:rpc('codex_status',{'work_id':bound['work_id']})['phase']=='running',
                 'Native-created work starts without a TUI')
            context=rpc('codex_attach',bound)
            assert rpc('codex_attach',{**bound,'accept':context['receipt']})['accepted']
            rpc('codex_finish',{**bound,'call_id':'native-before-tui-final','text':'NATIVE_BEFORE_TUI_BASELINE'})
            assert rpc('codex_status',{'work_id':bound['work_id']})['phase']=='completed'
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 80 if display else 44, 160, 0, 0))
        screen_args = [] if display else ['--no-alt-screen']
        resume_args = ['resume',created_before_tui['thread_id']] if created_before_tui else []
        terminal = subprocess.Popen([str(prefix / 'bin/codex'), *screen_args,
            '--profile','chatgpt','-c','tui.animations=false',*resume_args], env=env, cwd=root / 'w',
            stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        wait(lambda: 'NATIVE_BEFORE_TUI_BASELINE' in screen if created_before_tui else 'GPT-6' in screen, 'initial TUI')
        wait(lambda: native_rpc(home,'thread/loaded/list',{}).get('data'), 'native thread ready before typing')
        def submit(text):
            os.write(master,text.encode())
            # A separate Enter is intentional: upstream recognizes rapid text
            # plus Enter as a paste and protects it as an unsubmitted draft.
            if display:
                wait(lambda: text in '\n'.join(display.display), 'typed input rendered before separate Enter')
            else:
                time.sleep(.2)
            os.write(master,b'\r')
        if not created_before_tui:
            submit('Initial native continuity input')
        work = wait(lambda: next(iter(rpc('codex_sessions',{'action':'list'})['work']),None), 'registered native work')
        thread = work['thread_id']
        if created_before_tui:
            assert work['work_id']==created_before_tui['work_id'] and thread==created_before_tui['thread_id']
            wait(lambda:'NATIVE_BEFORE_TUI_BASELINE' in screen,'ordinary TUI resume retains the Native-created baseline')

        def history():
            rows, cursor = [], None
            while True:
                page = native_rpc(home,'thread/items/list',{'threadId':thread,'limit':100,'sortDirection':'asc','cursor':cursor})
                rows.extend(page['data'])
                cursor = page.get('nextCursor')
                if cursor is None:
                    return rows

        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'initial native completion')
        initial = bridge_commands()
        submitted = next((row for row in reversed(initial) if row['op']=='submit'),None)
        assert submitted is not None or created_before_tui
        anchor = 'owned-adapter-' + submitted['round'] if submitted else 'native-created-view-baseline'
        # A real source pass must survive more than one public delivery page
        # without refetching the same whole conversation for each200 messages.
        saved = [{'id':f'retained-public-{index}','role':'assistant','channel':'commentary',
            'status':'finished_successfully','content':{'content_type':'text','parts':['Retained original public history']}}
            for index in range(450)] + [{'id':anchor,'role':'assistant','channel':'final','status':'finished_successfully',
            'end_turn':True,'content':{'content_type':'text','parts':['OWNED_MANAGED_ROUND_COMPLETE']}}]
        baseline = history()
        if args.bridge_first:
            # The real CLI just completed Provider-origin execution. The next
            # requests arrive through authenticated MCP, not saved Chat reads.
            def source_reads():
                return [row for row in bridge_commands()
                        if row['op'] in ['chat_read','chat_page','chat_list']]
            before_reads = source_reads()
            assert before_reads == [], before_reads
            before_submits = sum(row['op']=='submit' for row in bridge_commands())
            admissions = []
            for index in [1, 2]:
                before_turns = native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']
                if index == 2:
                    terminal.terminate()
                    terminal.wait(timeout=15)
                    terminal = None
                    os.close(master)
                    master = None
                    (root/'stop-provider').touch()
                    service.wait(timeout=30)
                    assert service.returncode == 0
                    output.close()
                    (root/'stop-provider').unlink()
                    (root/'provider-ready.json').unlink()
                    if args.replace_native_owner:
                        from qualify_provider_d3 import stop_owned_pid
                        paths=server_paths(home)
                        assert len(paths)==1, 'one completed owned Native server required'
                        retired_owner=int(paths[0].with_name('pid').read_text())
                        retired=stop_owned_pid(retired_owner,home/'.local/lib/codex/core/generations/local-permission-fixture/runtime')
                        assert retired['kernel_stop_confirmed']
                        (root/'retired-completed-native-owner.json').write_text(json.dumps(retired,indent=2))
                        assert not server_paths(home), 'old native owner still alive'
                    env['CODEX_PROVIDER_OPERATOR_PORT'] = str(port)
                    output = (root/'provider-restarted-test.log').open('wb')
                    service = subprocess.Popen([str(args.service_test),'--ignored','--exact',
                        'synthetic_operator_service','--nocapture'],env=env,stdout=output,
                        stderr=subprocess.STDOUT,cwd=root/'w')
                    wait(lambda:(root/'provider-ready.json').exists(),'Bridge-first service restart')
                    assert json.loads((root/'provider-ready.json').read_text())['port']==port
                    if not args.replace_native_owner:
                        paths=server_paths(home)
                        assert len(paths)==1, 'one verified owned Native server required'
                        observer=Ws(str(paths[0].resolve()),uri_path='/rpc')
                        try:
                            state=observer.call('thread/read',{'threadId':thread,'includeTurns':False})
                            assert state['thread']['status']['type']=='notLoaded', 'restart did not exercise an unloaded work'
                        finally:
                            observer.close()
                command_args = {'action':'join','work_id':work['work_id'],
                    'task':f'BRIDGE_ORIGINAL_INPUT_{index}', 'call_id':f'bridge-input-{index}'}
                with concurrent.futures.ThreadPoolExecutor(max_workers=2) as clients:
                    first = clients.submit(rpc,'codex_sessions',command_args)
                    second = clients.submit(rpc,'codex_sessions',command_args)
                    started, retried = first.result(timeout=40), second.result(timeout=40)
                assert started==retried and started['started'] and started['inference_submitted'] is False
                assert started['thread_id']==thread and started['input_origin']=='bridge_arguments'
                if index==2 and args.replace_native_owner:
                    paths=server_paths(home)
                    assert len(paths)==1
                    assert int(paths[0].with_name('pid').read_text())!=retired_owner
                    assert before_turns==native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns'][:-1]
                rejected = rpc('codex_sessions',{**command_args,'task':'CHANGED_INPUT'},True)
                assert rejected['error']=='operator_call_changed', rejected
                bound = {'work_id':work['work_id'],'controller':started['controller']}
                wait(lambda:rpc('codex_status',{'work_id':work['work_id']})['phase']=='running','Bridge-owned Native turn')
                if index==2:
                    master,slave=pty.openpty()
                    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',80 if display else 44,160,0,0))
                    terminal=subprocess.Popen([str(prefix/'bin/codex'),*screen_args,
                        '--profile','chatgpt','-c','tui.animations=false','resume',thread],env=env,cwd=root/'w',
                        stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
                    os.close(slave)
                    wait(lambda:f'BRIDGE_ORIGINAL_INPUT_{index}' in '\n'.join(display.display) if display else f'BRIDGE_ORIGINAL_INPUT_{index}' in screen,
                         'reopened TUI sees the input admitted before it attached')
                busy = rpc('codex_sessions',{**command_args,'call_id':f'overlap-{index}'},True)
                assert busy['error']=='native_writer_unsettled', busy
                assert rpc('codex_tools',bound,True)['error']=='operator_controller_fenced'
                context = rpc('codex_attach',bound)
                assert context['accepted'] is False
                assert f'BRIDGE_ORIGINAL_INPUT_{index}' in json.dumps(context['incremental_history'])
                accepted = rpc('codex_attach',{**bound,'accept':context['receipt']})
                assert accepted['accepted'] is True
                tools = rpc('codex_tools',bound)['tools']
                name = next(name for name in tools if name.rsplit('.',1)[-1]=='exec_command')
                effect = {**bound,'call_id':f'bridge-effect-{index}','name':name,
                    'input':json.dumps({'cmd':f'printf "bridge-effect-{index}\\n" >> bridge-effects.txt; printf BRIDGE_RESULT_{index}',
                                      'login':False,'max_output_tokens':1000,'yield_time_ms':1000}),
                    'report':f'BRIDGE_PUBLIC_PROGRESS_{index}'}
                rejected = rpc('codex_execute',{**effect,'controller':work['controller']},True) if 'controller' in work else None
                if rejected is not None:
                    assert rejected['error']=='operator_controller_fenced', rejected
                result = rpc('codex_execute',effect)
                assert f'BRIDGE_RESULT_{index}' in json.dumps(result)
                assert rpc('codex_execute',effect)['result']==result['result']
                final = {**bound,'call_id':f'bridge-final-{index}','text':f'BRIDGE_SUBMITTED_FINAL_{index}'}
                rpc('codex_finish',final)
                assert rpc('codex_finish',final)['reused']
                wait(lambda:rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed','Bridge completion')
                wait(lambda:all(marker in screen for marker in [f'BRIDGE_ORIGINAL_INPUT_{index}', f'BRIDGE_PUBLIC_PROGRESS_{index}', f'BRIDGE_RESULT_{index}', f'BRIDGE_SUBMITTED_FINAL_{index}']),'Bridge input/result/progress/final live TUI')
                if display:
                    wait(lambda:f'BRIDGE_SUBMITTED_FINAL_{index}' in '\n'.join(display.display),
                         'Bridge final rendered in the active TUI viewport')
                after_turns = native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']
                assert after_turns[:-1]==before_turns and len(after_turns)==len(before_turns)+1
                assert sum(row['item']['type']=='userMessage' and f'BRIDGE_ORIGINAL_INPUT_{index}' in json.dumps(row['item']) for row in history())==1
                assert rpc('codex_sessions',command_args)==started
                work.update(controller=bound['controller'])
                admissions.append({'native_turn':started['native_turn'],'input':command_args['task']})
            assert (root/'w/bridge-effects.txt').read_text()=='bridge-effect-1\nbridge-effect-2\n'
            # More than the former2s timer interval, including after restart.
            # No request is sent by this idle interval.
            until = time.monotonic()+4.5
            while time.monotonic()<until:
                drain()
                time.sleep(.05)
            assert source_reads()==before_reads
            barrier = rpc('codex_sessions',{'action':'synchronize','thread_id':thread})
            assert barrier['status']=='ready' and not barrier['inference_submitted']
            assert source_reads()==before_reads
            assert sum(row['op']=='submit' for row in bridge_commands())==before_submits
            if not created_before_tui:
                bridge_prefix = native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']
                submit('CLI_AFTER_BRIDGE_INPUT')
                wait(lambda:sum(row['op']=='submit' for row in bridge_commands())==before_submits+1,
                     'new CLI input starts Provider after Bridge Direct work')
                wait(lambda:rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed','CLI after Bridge completes')
                after_cli = native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']
                assert after_cli[:-1]==bridge_prefix and len(after_cli)==len(bridge_prefix)+1
                assert rpc('codex_status',{'work_id':work['work_id']})['selection']['reasoning']=='provider'
                assert source_reads()==before_reads
            (root/'bridge-final-native-history.json').write_text(json.dumps(history(),indent=2))
            args.report.write_text(json.dumps({'accepted':True,'fixture':str(root),'thread_id':thread,
                'bridge_admissions':admissions,'provider_to_direct_same_work':not bool(created_before_tui),'native_created_before_ordinary_tui_resume':bool(created_before_tui),
                'context_before_effect':True,'concurrent_identical_join_once':True,'changed_input_rejected':True,
                'busy_work_rejected':True,'original_native_prefix_unchanged':True,'exact_effect_and_finish_retry':True,
                'service_restart_same_work':True,'saved_chat_reads':len(source_reads()),
                'join_after_all_clients_closed_and_thread_unloaded':True,
                'bridge_additional_primary_submissions':0,'cli_followup_primary_submissions':0 if created_before_tui else 1,
                'cli_after_bridge_same_work_completed':not bool(created_before_tui),'native_created_view_was_read_only':'read-only' in screen.lower(),'actual_native_core_and_live_tui':True,
                'bridge_finals_in_active_viewport':bool(display),
                'actual_web_or_android_chat_client':False,'supplied_completion_is_not_observed_chat_final':True,
                'fresh_native_home_without_openai_credentials':bool(args.fresh_auth_home),
                'completed_native_owner_replaced_before_join':bool(args.replace_native_owner)},indent=2)+'\n')
            origin = 'Native-created work→ordinary TUI resume' if created_before_tui else 'Provider→Direct same work'
            print(f'Bridge-first: {origin}, concurrent retry once, context gate, actual effects/results, public progress/final in live TUI, restart and zero saved-Chat reads',flush=True)
            return
        def select_route(reasoning, label):
            native_before = history()
            source = rpc('codex_history', {'work_id': work['work_id']})
            claimed = rpc('codex_sessions', {'action': 'claim', 'work_id': work['work_id'],
                'call_id': label + '-claim', 'cursor': source['cursor'], 'context_cursor': source['context_cursor']})
            bound = {'work_id': work['work_id'], 'controller': claimed['controller']}
            offered = rpc('codex_sessions', {**bound, 'action': 'transfer', 'call_id': label + '-offer',
                'reasoning': reasoning, 'return_path': 'mcp', 'cursor': source['cursor'],
                'context_cursor': source['context_cursor'], 'text': 'Continue this same verified development work.'})
            accepted = rpc('codex_sessions', {'work_id': work['work_id'], 'controller': offered['controller'],
                'action': 'transfer_accept', 'call_id': label + '-accept', 'transfer_receipt': offered['offer']['receipt'],
                'cursor': source['cursor'], 'context_cursor': source['context_cursor']})
            assert accepted['accepted'] and accepted['selection']['reasoning'] == reasoning
            assert history() == native_before, 'route transfer changed original native items'
        if args.direct:
            select_route('direct', 'direct-source')
        unbind_command = None
        unbind_result = None
        if args.binding:
            cursor=rpc('codex_history',{'work_id':work['work_id']})
            unbind_command=[str(args.provider_binary),'work','unbind','--work-id',work['work_id'],
                '--call-id','owned-unbind','--cursor',json.dumps(cursor['cursor']),
                '--context-cursor',json.dumps(cursor['context_cursor']),'--state',str(root/'provider-state')]
            detached=subprocess.run(unbind_command,env=env,capture_output=True,text=True,timeout=40)
            (root/'unbind-result.json').write_text(detached.stdout)
            assert detached.returncode==0, detached.stderr+detached.stdout
            unbind_result=json.loads(detached.stdout)
            assert unbind_result['binding']['enabled'] is False and unbind_result['inference_submitted'] is False
            assert history()==baseline
        if args.restart:
            (root/'stop-provider').touch()
            service.wait(timeout=30)
            assert service.returncode == 0, 'owned service did not exit normally'
            output.close()
            (root/'stop-provider').unlink()
            (root/'provider-ready.json').unlink()
            env['CODEX_PROVIDER_OPERATOR_PORT']=str(port)
            output=(root/'provider-restarted-test.log').open('wb')
            service=subprocess.Popen([str(args.service_test),'--ignored','--exact',
                'synthetic_operator_service','--nocapture'],env=env,stdout=output,
                stderr=subprocess.STDOUT,cwd=root/'w')
            wait(lambda:(root/'provider-ready.json').exists(),'restarted owned service readiness')
            assert json.loads((root/'provider-ready.json').read_text())['port']==port
            assert history()==baseline, 'service restart changed native history'
            cursor=rpc('codex_history',{'work_id':work['work_id']})['cursor']
            # This work has no assignment. Its specific rejection proves the
            # local facade used the new controller, without changing control.
            proof=subprocess.run([str(args.provider_binary),'work','evaluate','--work-id',work['work_id'],
                '--cursor',json.dumps(cursor),'--decision','accepted','--text','Owned unassigned result probe',
                '--state',str(root/'provider-state')],env=env,capture_output=True,text=True,timeout=40)
            (root/'restarted-local-controller-proof.json').write_text(proof.stdout)
            assert proof.returncode != 0 and json.loads(proof.stdout)=={'error':'assignment_unavailable'}, 'local CLI retained the preceding controller'
            assert sum(row['op']=='submit' for row in bridge_commands())==1
            print('Owned service restart: native history unchanged, local controller rotated, no model submission',flush=True)
        def publish_source():
            mapping, parent = {}, None
            for message in saved:
                mapping[message['id']] = {'parent':parent,'message':message}
                parent = message['id']
            temporary = root / 'saved-source.tmp'
            temporary.write_text(json.dumps({'conversation_id':'owned-saved-chat','current_node':parent,'mapping':mapping}))
            temporary.replace(root / 'saved-source.json')
        def admit_saved_source(selected=None):
            # Explicit recovery by this test; the product installs no idle reader.
            target = next(row['id'] for row in reversed(saved) if row['role']=='user')
            until = time.monotonic()+40
            while time.monotonic()<until:
                observed = rpc('codex_status',{'work_id':work['work_id']},observation_pending=True)
                if observed and (observed.get('source_admission') or {}).get('source',{}).get('message')==target:
                    return observed
                result = rpc('codex_sessions',{'action':'source_sync','thread_id':thread,
                    **({'source_message_id':selected} if selected else {})})
                with (root/'explicit-source-recovery.jsonl').open('a') as record:
                    record.write(json.dumps(result)+'\n')
                until_retry = time.monotonic()+(result['retry_after_seconds'] if result['status']=='waiting' else .05)
                while time.monotonic()<until_retry:
                    drain()
                    time.sleep(.05)
            raise AssertionError('Explicit source recovery did not admit '+target+'; inspect '+str(root))
        if args.sibling:
            publish_source()
            submit('CLI sibling absent from older Android branch')
            wait(lambda: sum(row['op']=='submit' for row in bridge_commands())==2,
                 'newer CLI sibling submitted')
            wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'CLI sibling settled')
            committed_before=history()
            saved += [{'id':'android-sibling-user','role':'user','content':{'content_type':'text','parts':['Original Android sibling input']}},
                {'id':'android-sibling-progress','role':'assistant','channel':'commentary','recipient':'all',
                 'status':'finished_partial_completion','end_turn':False,'content':{'content_type':'text','parts':['SIBLING_ORIGINAL_PROGRESS']}}]
            publish_source()
            rejected=rpc('codex_sessions',{'action':'source_sync','thread_id':thread},True)
            assert rejected['error']=='bound_chat_branch_changed', rejected
            blocked=wait(lambda:(status if (status:=rpc('codex_status',{'work_id':work['work_id']}))['development_binding']['source_selection_required'] else None),
                         'actual source branch conflict detected')
            assert history()==committed_before, 'implicit branch conflict imported an arbitrary sibling'
            (root/'source-read-rate-limit').touch()
            rejected=rpc('codex_sessions',{'action':'source_sync','thread_id':thread},True)
            assert rejected['error']=='source_selection_required',rejected
            assert rpc('codex_status',{'work_id':work['work_id']})['development_binding']['source_selection_required'] is True
            (root/'source-read-rate-limit').unlink()
            join={'action':'source_join','work_id':work['work_id'],'conversation_id':'owned-saved-chat',
                  'call_id':'explicit-android-sibling','source_message_id':'android-sibling-user'}
            rejected=rpc('codex_sessions',{**join,'source_message_id':'another-original'},True)
            assert rejected['error']=='source_selection_required',rejected
            def context_ready():
                value=rpc('codex_sessions',join)
                return value if value['status']=='context_required' else None
            context=wait(context_ready,'explicit sibling context admission')
            assert context['turn_token'] is None and context['tools'] is None and context['inference_submitted'] is False
            assert 'CLI sibling absent from older Android branch' in json.dumps(context['history'])
            assert context['source_admission']['source_anchor']==anchor
            sibling_submit=next(row for row in reversed(bridge_commands()) if row['op']=='submit')
            original_token=sibling_submit['instructions'][-1].split('Exact turn_token: ')[1].split('\n')[0]
            catalog=rpc('native_catalog',{'turn_token':original_token})
            token=catalog['turn_token'];tools=catalog['tools']
            name=next(name for name in tools if name.rsplit('.',1)[-1] in ['exec_command','shell_command','shell'])
            script="printf 'one-sibling-effect\\n' >> sibling-effect-count"
            if name.endswith('exec_command'):
                arguments={'cmd':script,'max_output_tokens':1000,'login':False,'yield_time_ms':1000}
            elif name.endswith('shell_command'):
                arguments={'command':script,'timeout_ms':10000,'max_output_chars':1000}
            else:
                arguments={'command':['bash','-c',script],'timeout_ms':10000}
            action={'turn_token':token,'call_id':'sibling-once','name':name,'input':json.dumps(arguments),'report':'Execute the selected original sibling once'}
            rejected=rpc('native_tool',action,True)
            assert rejected['error']=='source_context_required',rejected
            assert not (root/'w/sibling-effect-count').exists()
            first_turn=context['native_turn']
            saved += [{'id':'android-next-user','role':'user','content':{'content_type':'text','parts':['Next original after interruption']}},
                {'id':'android-next-progress','role':'assistant','channel':'commentary','recipient':'all',
                 'status':'finished_partial_completion','end_turn':False,'content':{'content_type':'text','parts':['NEXT_ORIGINAL_PROGRESS']}}]
            publish_source()
            wait(lambda: any(turn['id']==first_turn and turn['status'] in ['failed','interrupted']
                 for turn in native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']),
                 'selected original interrupted in Native')
            admit_saved_source('android-next-user')
            wait(lambda: (status if (status:=rpc('codex_status',{'work_id':work['work_id']},observation_pending=True))
                 and status['phase']=='running' and (status.get('source_admission') or {}).get('source',{}).get('message')=='android-next-user' else None),
                 'next original admitted to its own Native turn')
            join={'action':'source_join','work_id':work['work_id'],'conversation_id':'owned-saved-chat',
                  'call_id':'next-android-original','source_message_id':'android-next-user'}
            context=wait(context_ready,'next original context admission')
            assert context['native_turn'] != first_turn
            assert 'CLI sibling absent from older Android branch' in json.dumps(context['history'])
            catalog=rpc('native_catalog',{'turn_token':original_token})
            token=catalog['turn_token']
            action['turn_token']=token
            rejected=rpc('native_tool',action,True)
            assert rejected['error']=='source_context_required',rejected
            assert not (root/'w/sibling-effect-count').exists()
            joined=rpc('codex_sessions',{**join,'accept':context['receipt']})
            assert joined['status']=='ready' and joined['turn_token']==token
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as calls:
                pending=calls.submit(rpc,'native_tool',action)
                wait(pending.done,'selected sibling actual Native effect')
                result=pending.result()
            assert rpc('native_tool',action)==result
            assert (root/'w/sibling-effect-count').read_text()=='one-sibling-effect\n'
            saved += [{'id':'android-sibling-final','role':'assistant','channel':'final','recipient':'all',
                       'status':'finished_successfully','end_turn':True,'content':{'content_type':'text','parts':['SIBLING_ORIGINAL_FINAL']}}]
            publish_source()
            wait(lambda:rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed','selected sibling Native completion')
            wait(lambda:'SIBLING_ORIGINAL_FINAL' in screen,'selected sibling original final in live TUI')
            final=history()
            assert final[:len(committed_before)]==committed_before,'branch reconciliation rewrote previous Native history'
            assert len({row['turnId'] for row in final})==4
            assert sum(row['op']=='submit' for row in bridge_commands())==2,'sibling was reinferred'
            status=rpc('codex_status',{'work_id':work['work_id']})
            assert status['source_participation']['context_accepted'] is True
            assert status['source_participation']['context_required'] is True
            (root/'sibling-final-history.json').write_text(json.dumps(final,indent=2))
            args.report.write_text(json.dumps({'accepted':True,'fixture':str(root),'thread_id':thread,
                'explicit_source_sibling_admitted':True,'unknown_source_rejected':True,'native_context_required_before_effect':True,
                'interrupted_selected_source_next_original_requires_context':True,
                'branch_conflict_survives_read_rate_limit':True,'previous_native_items_unchanged':len(committed_before),
                'actual_native_effects':1,'saved_answers_reinferred':0,'primary_submissions':2,
                'actual_android_native_core_and_live_tui':True,'actual_web_or_android_chat_client':False},indent=2)+'\n')
            print('Explicit sibling: canonical ancestry/context, one actual Native effect, exact retry, original final and live TUI; earlier Native history unchanged',flush=True)
            return
        if args.source_timeout and not (args.binding or args.observer_recovery):
            (root / 'source-read-timeout-once').touch()
        for index in [1,2]:
            if args.source_timeout and (args.binding or args.observer_recovery) and index==2:
                (root/'source-read-timeout-once').touch()
            if args.observer_recovery and index==2:
                (root/'source-read-branch-conflict').touch()
            # Equal text with distinct physical message IDs is two user inputs.
            text = 'Identical external input for source identity proof'
            saved += [{'id':f'web-user-{index}','role':'user','content':{'content_type':'text','parts':[text]}},
                {'id':f'web-final-{index}','role':'assistant','recipient':'all','channel':'final',
                 'status':'finished_successfully','end_turn':True,
                 'content':{'content_type':'text','parts':[f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}']}}]
            publish_source()
            if args.binding and index==1:
                assert rpc('codex_status',{'work_id':work['work_id']})['development_binding']['enabled'] is False
                sync=rpc('codex_sessions',{'action':'source_sync','thread_id':thread})
                assert sync['status']=='ready' and sync['inference_submitted'] is False
                assert history()==baseline, 'detached original source was admitted'
                source=rpc('codex_chats',{'action':'read','conversation_id':'owned-saved-chat','limit':200})
                while not source['complete']:
                    source=rpc('codex_chats',{'action':'read','conversation_id':'owned-saved-chat','limit':200,'after':source['cursor']})
                cursor=rpc('codex_history',{'work_id':work['work_id']})
                command=[str(args.provider_binary),'work','bind','--work-id',work['work_id'],
                    '--call-id','owned-rebind','--cursor',json.dumps(cursor['cursor']),
                    '--context-cursor',json.dumps(cursor['context_cursor']),
                    '--conversation-id','owned-saved-chat','--source-cursor',json.dumps(source['cursor']),
                    '--state',str(root/'provider-state')]
                bound=subprocess.run(command,env=env,capture_output=True,text=True,timeout=40)
                (root/'rebind-result.json').write_text(bound.stdout)
                assert bound.returncode==0, bound.stderr+bound.stdout
                assert json.loads(bound.stdout)['binding']['enabled'] is True
            if args.observer_recovery and index==2:
                if args.source_timeout:
                    retry=rpc('codex_sessions',{'action':'source_sync','thread_id':thread})
                    assert retry['status']=='waiting' and retry['reason']=='cdp_operation_timeout',retry
                    assert (root/'source-read-timeout-once.observed').exists()
                    (root/'retry-wait-status.json').write_text(json.dumps(retry,indent=2))
                rejected=rpc('codex_sessions',{'action':'source_sync','thread_id':thread},True)
                assert rejected['error']=='bound_chat_branch_changed', rejected
                assert (root/'source-read-branch-conflict.observed').exists()
                blocked=rpc('codex_status',{'work_id':work['work_id']})
                assert blocked['development_binding']['last_observation']['state']=='blocked'
                assert blocked['development_binding']['last_observation']['reason']=='bound_chat_branch_changed'
                assert not any(row['item'].get('text')=='WEB_ORIGINAL_COMPLETE_ANSWER_2' for row in history())
                (root/'blocked-source-status.json').write_text(json.dumps(blocked,indent=2))
                (root/'source-read-branch-conflict').unlink()
            admit_saved_source()
            wait(lambda: any(row['item'].get('text')==f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}' for row in history()),
                 f'external input {index} consumed and original answer committed')
            wait(lambda: f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}' in screen, 'external answer displayed in live TUI')
            wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'external turn settled')
            current = history()
            users = [row for row in current if row['item']['type']=='userMessage' and text in json.dumps(row['item'])]
            assert len(users)==index, users
            assert len({row['item']['clientId'] for row in users})==index, users
            commands = bridge_commands()
            assert sum(row['op']=='submit' for row in commands)==1, 'saved input was submitted to Chat again'
            assert sum(row['op']=='follow' for row in commands)==index
            print(f'External source {index}: actual queue consumption, canonical answer and live TUI; no primary request',flush=True)
        if args.source_timeout:
            assert (root / 'source-read-timeout-once.observed').exists(), 'source read fault was not exercised'
        if args.observer_recovery:
            assert rpc('codex_status',{'work_id':work['work_id']})['native_observer_running'] is False
            print('Explicit source retry/branch recovery preserved originals; no idle remote observer',flush=True)
        if args.binding:
            replay=subprocess.run(unbind_command,env=env,capture_output=True,text=True,timeout=40)
            (root/'unbind-replay.stdout').write_text(replay.stdout)
            (root/'unbind-replay.stderr').write_text(replay.stderr)
            assert replay.returncode==0 and json.loads(replay.stdout)==unbind_result
            assert rpc('codex_status',{'work_id':work['work_id']})['development_binding']['enabled'] is True
            print('Explicit unbind survives service restart, synchronize admits no detached source, rebind consumes original once; receipt replay preserves new connection',flush=True)
        if args.join:
            (root/'source-read-rate-limit').touch()
        saved += [{'id':'web-user-3','role':'user','content':{'content_type':'text','parts':['External native tool proof']}},
            {'id':'web-answer-3','role':'assistant','recipient':'all','channel':'final',
             'status':'finished_partial_completion','end_turn':False,
             'content':{'content_type':'text','parts':['Native action in progress.']}}]
        publish_source()
        if args.join:
            pending_join={'action':'source_join','work_id':work['work_id'],'conversation_id':'owned-saved-chat','call_id':'join-after-completed-before-admission'}
            pending=rpc('codex_sessions',pending_join)
            assert pending['status']=='waiting' and pending['reason']=='app_chat_rate_limited' and pending['retry_after_seconds']==3 and pending['inference_submitted'] is False,pending
            (root/'join-before-admission-rate-limit.json').write_text(json.dumps(pending,indent=2))
            (root/'source-read-rate-limit').unlink()
            wait(lambda: rpc('codex_sessions',pending_join)['status']=='context_required','same source join recovers pending admission')
        else:
            admit_saved_source()
        wait(lambda: sum(row['op']=='follow' for row in bridge_commands())==3,
             'active external source admitted')
        if args.source_admission:
            attachment=[str(args.provider_binary),'work','attach','--work-id',work['work_id'],'--state',str(root/'provider-state'),'--diagnostic-caps','true']
            view=subprocess.run(attachment,env=env,capture_output=True,text=True,timeout=40)
            assert view.returncode==0, view.stderr+view.stdout
            context=json.loads(view.stdout)
            (root/'active-source-native-context.json').write_text(view.stdout)
            rejected=subprocess.run([*attachment,'--accept',context['receipt']],env=env,capture_output=True,text=True,timeout=40)
            assert rejected.returncode!=0 and json.loads(rejected.stdout)=={'error':'app_response_owns_execution'}, rejected.stdout
            # The owned fixture journal supplies the same private controller as
            # the actual local facade; it is never emitted in a report.
            key=hashlib.sha256(json.dumps(['operator-work-v1',work['work_id']],separators=(',',':')).encode()).hexdigest()
            controller=json.loads((root/'provider-state'/(key+'.json')).read_text())['payload']['capability']
            denied=rpc('codex_finish',{'work_id':work['work_id'],'controller':controller,
                'call_id':'wrong-source-finish','text':'Must not replace the original source final'},True)
            assert denied['error']=='app_response_owns_execution', denied
            assert rpc('codex_status',{'work_id':work['work_id']})['phase']=='running'
        joined=None
        if args.join:
            join_args={'action':'source_join','work_id':work['work_id'],'conversation_id':'owned-saved-chat',
                'source_message_id':'web-user-3','call_id':'owned-source-join'}
            rejected=rpc('codex_sessions',{**join_args,'source_message_id':'old-source'},True)
            assert rejected['error']=='source_selection_required',rejected
            (root/'source-read-rate-limit').touch()
            pending=rpc('codex_sessions',join_args)
            assert pending['status']=='waiting' and pending['reason']=='app_chat_rate_limited' and pending['retry_after_seconds']==3,pending
            (root/'join-active-rate-limit.json').write_text(json.dumps(pending,indent=2))
            (root/'source-read-rate-limit').unlink()
            original_saved=json.loads((root/'saved-source.json').read_text())
            branched=json.loads(json.dumps(original_saved))
            branched['mapping']['unselected-source']={'parent':anchor,'message':{'id':'unselected-source','role':'user',
                'content':{'content_type':'text','parts':['Unselected concurrent branch']}}}
            (root/'join-branched-source.tmp').write_text(json.dumps(branched))
            (root/'join-branched-source.tmp').replace(root/'saved-source.json')
            implicit={key:value for key,value in join_args.items() if key!='source_message_id'}
            rejected=rpc('codex_sessions',implicit,True)
            assert rejected['error']=='source_selection_required',rejected
            context=rpc('codex_sessions',join_args)
            assert context['status']=='context_required' and context['inference_submitted'] is False
            assert context['tools'] is None and context['turn_token'] is None
            assert any('External native tool proof' in json.dumps(row) for row in context['history'])
            rejected=rpc('codex_sessions',{**join_args,'accept':'wrong-context-receipt'},True)
            assert rejected['error']=='source_context_changed',rejected
            command=[str(args.provider_binary),'work','source-join','--work-id',work['work_id'],
                '--conversation-id','owned-saved-chat','--source-message-id','web-user-3',
                '--call-id','owned-source-join-accept','--accept',context['receipt'],
                '--diagnostic-caps','true','--state',str(root/'provider-state')]
            accepted=subprocess.run(command,env=env,capture_output=True,text=True,timeout=40)
            (root/'source-join-accepted-private.json').write_text(accepted.stdout)
            assert accepted.returncode==0,accepted.stderr+accepted.stdout
            joined=json.loads(accepted.stdout)
            assert joined['status']=='ready' and joined['native_turn']==context['native_turn']
            assert joined['source_admission']==context['source_admission'] and joined['inference_submitted'] is False
            assert rpc('codex_status',{'work_id':work['work_id']})['source_participation']['context_accepted'] is True
            publish_source()
        original_token = submitted['instructions'][-1].split('Exact turn_token: ')[1].split('\n')[0]
        catalog = rpc('native_catalog',{'turn_token':original_token})
        current_token = catalog['turn_token']
        tools = catalog['tools']
        if joined:
            assert joined['turn_token']==current_token and joined['tools']==tools, 'join invented another tool admission'
        assert current_token != original_token
        name = next(name for name in tools if name.rsplit('.',1)[-1] in ['exec_command','shell_command','shell'])
        script = "printf 'one-external-effect\\n' >> external-effect-count"
        if name.endswith('exec_command'):
            arguments={'cmd':script,'max_output_tokens':1000,'login':False,'yield_time_ms':1000}
        elif name.endswith('shell_command'):
            arguments={'command':script,'timeout_ms':10000,'max_output_chars':1000}
        else:
            arguments={'command':['bash','-c',script],'timeout_ms':10000}
        action={'turn_token':current_token,'call_id':'source-native-once','name':name,
            'input':json.dumps(arguments),'report':'Perform the owned external native action once'}
        def source_tool_message(message_id, args, connector='Termux Bridge'):
            return {'id':message_id,'role':'assistant','recipient':'api_tool.call_tool','channel':None,
                'status':'finished_successfully','content':{'content_type':'code','text':json.dumps({
                    'path':f'/{connector}/owned-source-link/native_tool','args':args})}}
        if args.source_admission:
            saved.insert(-1,source_tool_message('platform-native-request',action))
            publish_source()
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as calls:
            pending=calls.submit(rpc,'native_tool',action)
            wait(pending.done,'native retained tool result')
            result=pending.result()
        assert rpc('native_tool',action)==result, 'retained MCP retry changed its original receipt'
        assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
        if args.source_admission:
            changed={**action,'report':'Rejected reuse with changed arguments'}
            denied=rpc('native_tool',changed,True)
            assert denied['error']=='mcp_call_changed',denied
            saved[-1:-1]=[source_tool_message('platform-native-retry',action),
                source_tool_message('platform-rejected-change',changed),
                source_tool_message('platform-other-connector',action,'Different connector')]
            publish_source()
            def correlated_source():
                page=rpc('codex_history',{'work_id':work['work_id']})
                return next((row for row in page['source_executions']
                    if {'platform-native-request','platform-native-retry'} <= set(row['native_tool_calls'])),None)
            correlated=wait(correlated_source,'original platform/native tool correlation')
            assert set(correlated['native_tool_calls'])=={'platform-native-request','platform-native-retry'},correlated
            assert {call['native_call_id'] for call in correlated['native_tool_calls'].values()}=={result['id']}
            assert {call['logical_call_id'] for call in correlated['native_tool_calls'].values()}=={'source-native-once'}
            assert current_token not in json.dumps(correlated) and action['input'] not in json.dumps(correlated)
            (root/'original-platform-native-call-correlations.json').write_text(json.dumps(correlated,indent=2))
            assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
        if args.provider_binary and not args.direct:
            synchronized_call = subprocess.run([str(args.provider_binary), 'work', 'synchronize',
                '--thread-id', thread, '--state', str(root/'provider-state')], env=env,
                capture_output=True, text=True, timeout=40)
            (root/'pending-synchronization-cli.json').write_text(synchronized_call.stdout)
            (root/'pending-synchronization-cli.stderr').write_text(synchronized_call.stderr)
            assert synchronized_call.returncode == 0, 'authenticated source synchronization CLI failed'
            synchronized = json.loads(synchronized_call.stdout)
            assert synchronized['status']=='waiting' and synchronized['inference_submitted'] is False, synchronized
            before_overlap = history()
            submit('Next native input after both external turns')
            until = time.monotonic() + .5
            while time.monotonic() < until:
                drain()
                assert history() == before_overlap, 'native draft was admitted before the active source committed'
                assert sum(row['op']=='submit' for row in bridge_commands())==1
                time.sleep(.05)
            assert 'Synchronizing ChatGPT' in '\n'.join(display.display), 'pending full native draft not visible'
        saved[-1].update(status='finished_successfully',end_turn=True)
        saved[-1]['content']['parts']=['Native action in progress.\nWEB_ORIGINAL_COMPLETE_ANSWER_3']
        publish_source()
        wait(lambda: any('WEB_ORIGINAL_COMPLETE_ANSWER_3' in row['item'].get('text','') for row in history()),
             'active external native tool turn completed')
        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'active source settled')
        wait(lambda: 'WEB_ORIGINAL_COMPLETE_ANSWER_3' in screen, 'complete external tool answer in live TUI')
        def disclosure_proof(label):
            def frame():
                return '\n'.join(display.display)
            caption = '진행 내용 · 최종 답변에 포함'
            wait(lambda: caption in frame() and 'WEB_ORIGINAL_COMPLETE_ANSWER_3' in frame(),
                 label + ' exact source progress folded in current viewport')
            folded = frame()
            (root / (label + '-folded.txt')).write_text(folded)
            assert folded.count('Native action in progress.') == 1, folded
            retained = ['WEB_ORIGINAL_COMPLETE_ANSWER_3',
                'Perform the owned external native action once', 'external-effect-count']
            assert all(text in folded for text in retained), folded
            rows = display.display
            progress_row = next(i for i, text in enumerate(rows) if caption in text)
            candidates = [(i, wcswidth(text[:text.index('+ Show details')]))
                for i, text in enumerate(rows) if abs(i - progress_row) <= 4 and '+ Show details' in text]
            row, column = min(candidates, key=lambda position: abs(position[0] - progress_row))
            os.write(master, f'\x1b[<0;{column+2};{row+1}M\x1b[<0;{column+2};{row+1}m'.encode())
            wait(lambda: frame().count('Native action in progress.') == 2,
                 label + ' mouse disclosure reveals original progress')
            expanded = frame()
            assert all(text in expanded for text in retained), expanded
            (root / (label + '-expanded.txt')).write_text(expanded)
            os.write(master, f'\x1b[<0;{column+2};{row+1}M\x1b[<0;{column+2};{row+1}m'.encode())
            wait(lambda: frame().count('Native action in progress.') == 1 and caption in frame(),
                 label + ' mouse disclosure closes without losing final')
            # Clicking a disclosure enters Native transcript selection. Leave
            # that user browsing mode before testing automatic live tail.
            os.write(master,b'\x1b')
            wait(lambda:'enter details' not in frame(),label+' exits transcript selection')
            # Native Escape first clears selection; a separate Escape invokes
            # its existing return-to-latest action (owned_transcript_input).
            os.write(master,b'\x1b')
        if display:
            disclosure_proof('live')
            print('Actual Provider journal CLI → live TUI: source-only folding and mouse expansion preserve complete final',flush=True)
        assert sum(row['op']=='submit' for row in bridge_commands())==1
        print('Active external source: refreshed discovery, one actual native effect, retained result retry and complete final',flush=True)
        if not args.provider_binary or args.direct:
            submit('Next native input after both external turns')
        wait(lambda: sum(row['op']=='submit' for row in bridge_commands())==2,
             'native reentry after external turns')
        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'native reentry completion')
        source=json.loads((root/'saved-source.json').read_text())
        saved, seen, current=[], set(), source['current_node']
        while current is not None:
            assert current not in seen, 'invalid synthetic source ancestry'
            seen.add(current)
            node=source['mapping'][current]
            saved.append(node['message'])
            current=node['parent']
        saved.reverse()
        saved += [{'id':'web-user-4','role':'user','content':{'content_type':'text','parts':['External input after returning through Codex']}},
            {'id':'web-final-4','role':'assistant','recipient':'all','channel':'final',
             'status':'finished_successfully','end_turn':True,
             'content':{'content_type':'text','parts':['WEB_ANSWER_AFTER_NATIVE_REENTRY']}}]
        publish_source()
        admit_saved_source()
        wait(lambda: any(row['item'].get('text')=='WEB_ANSWER_AFTER_NATIVE_REENTRY' for row in history()),
             'repeated native/web alternation on the saved branch')
        wait(lambda: 'WEB_ANSWER_AFTER_NATIVE_REENTRY' in screen, 'repeated alternation live display')
        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'last external completion')
        final = history()
        assert len({row['turnId'] for row in final})==6, final
        if args.join:
            rejected=rpc('codex_sessions',{**join_args,'source_message_id':'web-user-4','call_id':'ended-source-join'},True)
            assert rejected['error']=='source_response_ended',rejected
            participation=rpc('codex_history',{'work_id':work['work_id']})['source_participation']
            observed={row['turn_id']:row['participation'] for row in participation}
            sources=rpc('codex_history',{'work_id':work['work_id']})['source_admissions']
            by_source={row['source']['message']:observed[row['turn_id']] for row in sources}
            assert by_source['web-user-1']['state']=='execution_not_joined'
            assert by_source['web-user-2']['state']=='execution_not_joined'
            assert by_source['web-user-3']['context_accepted'] is True
            assert by_source['web-user-3']['catalog_discovered'] is True
            assert by_source['web-user-3']['tool_requests']==1
            print('Active source join: actual context receipt, same native turn/catalog, original retained result, omission and ended-source detection',flush=True)

        assert len(rpc('codex_sessions',{'action':'list'})['work'])==1
        if args.interrupted:
            saved += [{'id':'web-user-6','role':'user','content':{'content_type':'text','parts':['Original interrupted input']}},
                {'id':'web-progress-6','role':'assistant','recipient':'all','channel':'commentary',
                 'status':'finished_partial_completion','end_turn':False,
                 'content':{'content_type':'text','parts':['ORIGINAL_INTERRUPTED_PUBLIC_PROGRESS']}}]
            publish_source()
            admit_saved_source()
            wait(lambda: sum(row['op']=='follow' for row in bridge_commands())==5,
                 'interrupted response originally admitted')
            wait(lambda: 'ORIGINAL_INTERRUPTED_PUBLIC_PROGRESS' in screen, 'partial original displayed live')
            saved += [{'id':'web-user-7','role':'user','content':{'content_type':'text','parts':['Next original input after interruption']}},
                {'id':'web-final-7','role':'assistant','recipient':'all','channel':'final',
                 'status':'finished_successfully','end_turn':True,
                 'content':{'content_type':'text','parts':['ORIGINAL_RESPONSE_AFTER_INTERRUPTION']}}]
            publish_source()
            admit_saved_source()
            wait(lambda: any(row['item'].get('text')=='ORIGINAL_RESPONSE_AFTER_INTERRUPTION' for row in history()),
                 'next original answer after failed source')
            wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'next source settled')
            wait(lambda: 'ORIGINAL_RESPONSE_AFTER_INTERRUPTION' in screen, 'next original answer in live TUI')
            interrupted_history=history()
            partial=[row for row in interrupted_history if row['item'].get('text')=='ORIGINAL_INTERRUPTED_PUBLIC_PROGRESS']
            assert len(partial)==1, 'interrupted public progress lost or duplicated'
            turns=native_rpc(home,'thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns']
            failed=next(turn for turn in turns if turn['id']==partial[0]['turnId'])
            assert failed['status'] in ['failed','interrupted'], 'unfinished source was falsely completed'
            assert sum(row['op']=='submit' for row in bridge_commands())==2
            assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
            (root/'interrupted-native-turn.json').write_text(json.dumps(failed,indent=2))
            print('Interrupted original: partial preserved, Native failure confirmed, next source consumed once without inference',flush=True)
        if args.direct:
            select_route('direct', 'direct-stop')
        saved += [{'id':'web-user-5','role':'user','content':{'content_type':'text','parts':['Cancel this external response']}},
            {'id':'web-progress-5','role':'assistant','recipient':'all','channel':'commentary',
             'status':'finished_partial_completion','end_turn':False,
             'content':{'content_type':'text','parts':['External response still active.']}}]
        publish_source()
        admit_saved_source()
        wait(lambda: sum(row['op']=='follow' for row in bridge_commands())==(7 if args.interrupted else 5),
             'cancellable external response admitted')
        latest=next(row for row in reversed(bridge_commands()) if row['op']=='submit')
        latest_token=latest['instructions'][-1].split('Exact turn_token: ')[1].split('\n')[0]
        active_token=rpc('native_catalog',{'turn_token':latest_token})['turn_token']
        stopped=rpc('codex_stop',{'work_id':work['work_id']})
        assert stopped['native_stop_confirmed'] and stopped['host_stop_confirmed'], stopped
        denied=rpc('native_tool',{**action,'turn_token':active_token,'call_id':'after-stop'},True)
        assert denied['error']=='mcp_turn_unavailable', denied
        assert rpc('native_tool',action)==result, 'Stop of a newer source lost an earlier settled receipt'
        assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
        final=history()
        assert len({row['turnId'] for row in final})==(9 if args.interrupted else 7)
        if args.source_admission:
            records=rpc('codex_history',{'work_id':work['work_id']})['source_admissions']
            expected=['web-user-1','web-user-2','web-user-3','web-user-4']
            if args.interrupted:
                expected += ['web-user-6','web-user-7']
            expected += ['web-user-5']
            assert {record['source']['message'] for record in records} == set(expected)
            assert len(records)==len(expected), 'source mapping lost or duplicated'
            assert len({record['account_ref'] for record in records})==1
            for record in records:
                assert record['schema']=='codex-chatgpt-source-admission-v1'
                assert record['work_id']==work['work_id'] and record['thread_id']==thread
                assert record['source']['conversation']=='owned-saved-chat'
                original=next(message for message in saved if message['id']==record['source']['message'])
                actual=next(row['item'] for row in final if row['turnId']==record['turn_id']
                    and row['item']['type']=='userMessage')
                assert actual['clientId']==record['client_user_message_id']
                logical='chat-source:'+hashlib.sha256(json.dumps([record['account_ref'],record['source']['conversation'],record['source']['message']],separators=(',',':')).encode()).hexdigest()
                client='termux-queue-'+hashlib.sha256(json.dumps(['native-queue-v1',thread,logical],separators=(',',':')).encode()).hexdigest()
                assert actual['clientId']==client, 'canonical account/source identity absent from native queue key'
                assert actual['content'][0]['text']=='\n'.join(original['content']['parts'])
                assert record['execution_home']==str(home/'.codex')
                assert record['workspace']==str(root/'w')
            latest=rpc('codex_status',{'work_id':work['work_id']})['source_admission']
            assert latest['source']['message']=='web-user-5'
            (root/'source-admissions.json').write_text(json.dumps(records,indent=2))
            print('Original source IDs, native queue consumption and per-turn mappings remain queryable after seven sources',flush=True)
        if display:
            terminal.terminate()
            terminal.wait(timeout=15)
            drain()
            os.close(master)
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 80, 160, 0, 0))
            display.reset()
            decoder.reset()
            terminal = subprocess.Popen([str(prefix / 'bin/codex'), 'resume', thread,
                '--profile','chatgpt','-c','tui.animations=false'], env=env, cwd=root / 'w',
                stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            disclosure_proof('resumed')
            assert history() == final, 'local presentation or resume changed canonical source history'
            print('Actual resumed TUI: original source groups fold and expand; canonical history unchanged',flush=True)
        print('Active source Stop: actual native interruption, retired execution lease and earlier result retained; host Stop is synthetic',flush=True)
        (root / 'native-history.json').write_text(json.dumps(final,indent=2))
        args.report.write_text(json.dumps({'accepted':True,'fixture':str(root),'thread_id':thread,
            'initial_native_items':len(baseline),'external_inputs':7 if args.interrupted else 5,'native_turns':9 if args.interrupted else 7,'completed_turns':7 if args.interrupted else 6,
            'interrupted_source_native_failure_and_next_original':bool(args.interrupted),
            'immutable_source_native_admissions':bool(args.source_admission),
            'explicit_binding_detachment_restart_and_rebind':bool(args.binding),
            'observable_source_retry_block_and_recovery':bool(args.observer_recovery),
            'active_source_join_context_and_same_native_lease':bool(args.join),
            'actual_external_native_effects':1,'retained_mcp_result_retry':True,
            'active_source_native_stop_and_lease_retirement':True,'actual_external_chat_server_stop':False,
            'saved_answers_reinferred':0,'primary_submissions':2,'actual_android_queue_history_and_live_tui':True,
            'actual_provider_cli_live_and_resumed_mouse_disclosure':bool(display),
            'actual_web_or_android_chat_client':False,'settled_provider_restart_same_saved_chat':bool(args.restart),'active_external_native_draft_serialization':bool(args.provider_binary and not args.direct),'direct_route_source_execution':bool(args.direct),'automatic_provider_reentry':bool(args.direct),'source_timeout_recovered':bool(args.source_timeout),'simultaneous_entrances_qualified':False},indent=2)+'\n')
    finally:
        (root / 'terminal.txt').write_text(screen)
        if display:
            (root / 'terminal-visible.txt').write_text('\n'.join(display.display))
        (root / 'stop-provider').touch()
        for process in [terminal,service]:
            if process is not None:
                if process.poll() is None:
                    process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill(); process.wait()
        for path in server_paths(home):
            try:
                pid=int(path.with_name('pid').read_text())
                expected=home/'.local/lib/codex/core/generations/local-permission-fixture/runtime'
                if Path(f'/proc/{pid}/exe').resolve()==expected:
                    os.kill(pid,signal.SIGTERM)
            except (OSError,ValueError):
                pass
        if master is not None:
            os.close(master)
        output.close()
        print('Owned source/native records retained at ' + str(root),flush=True)


if __name__=='__main__':
    os.umask(0o077)
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['core','frontend','service-test','report']:
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--provider-binary',type=Path,
        help='Exercise actual synchronization/presentation CLI, overlapping draft and mouse disclosure; requires pyte and wcwidth')
    parser.add_argument('--sibling',action='store_true',help='Qualify explicit stale-client sibling selection, canonical ancestry and mandatory context before effects')
    parser.add_argument('--restart',action='store_true',help='Qualify settled service restart and automatic source observation with no extra native prompt')
    parser.add_argument('--direct', action='store_true', help='Qualify retained source, actual native effect and Stop after Direct transfers, with automatic Provider reentry')
    parser.add_argument('--native-created', action='store_true', help='Create and complete Native work before ordinary TUI resume; verify same execution backend')
    parser.add_argument('--bridge-first', action='store_true', help='Qualify ordinary Bridge-native admission without saved-Chat reads; actual Native/TUI, synthetic Chat')
    parser.add_argument('--replace-native-owner', action='store_true', help='Retire only the completed owned Native backend before the second Bridge join; verify same work/history and a replacement owner')
    parser.add_argument('--fresh-auth-home', action='store_true', help='Remove only the owned inert auth fixture before Native/TUI startup; no OpenAI credentials or login bypass')
    parser.add_argument('--join', action='store_true', help='Qualify active response context/accept, same native tool lease and omission/ended-source diagnostics')
    parser.add_argument('--source-recovery', dest='observer_recovery', action='store_true', help='Verify retry/blocked diagnostics and recovery of an actual host-selected branch conflict without restart')
    parser.add_argument('--binding', action='store_true', help='Exercise actual work unbind/bind controls and source admission barrier across optional restart')
    parser.add_argument('--source-admission', action='store_true', help='Verify durable original source/native turn IDs and consumed input after source advancement')
    parser.add_argument('--interrupted', action='store_true', help='Preserve interrupted public response and consume next original without reinference')
    parser.add_argument('--source-timeout', action='store_true', help='Inject one saved-source CDP read timeout and require continued observation')
    arguments=parser.parse_args()
    if arguments.replace_native_owner and not (arguments.bridge_first and arguments.native_created):
        parser.error('--replace-native-owner requires --bridge-first --native-created')
    if (arguments.restart or arguments.binding or arguments.join) and not arguments.provider_binary:
        parser.error('--restart requires the actual --provider-binary control facade')
    probe(arguments)
