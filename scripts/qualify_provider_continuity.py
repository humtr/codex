"""Actual Android queue/history/TUI proof using saved public Chat fixtures; no Chat inference."""
import argparse
import codecs
import concurrent.futures
import fcntl
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
    config=home/'.codex/config.toml'
    config.write_text(config.read_text()+'\n[projects.'+json.dumps(str(root/'w'))+']\ntrust_level = "trusted"\n')
    for name in ['owned-fixture', 'model-adapter-fixture', 'bound-source-fixture']:
        (root / name).write_text('Owned synthetic saved-source continuity proof\n')
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
            raise AssertionError(description + '; inspect ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists(), 'Provider readiness')
        port = json.loads((root / 'provider-ready.json').read_text())['port']
        seq = 0

        def rpc(name, arguments, expect_error=False):
            nonlocal seq
            seq += 1
            request = urllib.request.Request(f'http://127.0.0.1:{port}/mcp',
                data=json.dumps({'jsonrpc':'2.0','id':seq,'method':'tools/call',
                    'params':{'name':name,'arguments':arguments}}).encode(),
                headers={'Content-Type':'application/json','Authorization':'Bearer ' + TOKEN})
            with urllib.request.urlopen(request, timeout=50) as response:
                result = json.load(response)['result']
            value = json.loads(result['content'][0]['text'])
            assert result['isError']==expect_error, value
            return value

        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 80 if display else 44, 160, 0, 0))
        screen_args = [] if display else ['--no-alt-screen']
        terminal = subprocess.Popen([str(prefix / 'bin/codex'), *screen_args,
            '--profile','chatgpt','-c','tui.animations=false'], env=env, cwd=root / 'w',
            stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        wait(lambda: 'GPT-6' in screen, 'initial TUI')
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
        submit('Initial native continuity input')
        work = wait(lambda: next(iter(rpc('codex_sessions',{'action':'list'})['work']),None), 'registered native work')
        thread = work['thread_id']

        def history():
            rows, cursor = [], None
            while True:
                page = native_rpc(home,'thread/items/list',{'threadId':thread,'limit':100,'sortDirection':'asc','cursor':cursor})
                rows.extend(page['data'])
                cursor = page.get('nextCursor')
                if cursor is None:
                    return rows

        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'initial native completion')
        initial = json.loads((root / 'bridge-commands.json').read_text())
        submitted = next(row for row in reversed(initial) if row['op']=='submit')
        anchor = 'owned-adapter-' + submitted['round']
        saved = [{'id':anchor,'role':'assistant','channel':'final','status':'finished_successfully',
            'end_turn':True,'content':{'content_type':'text','parts':['OWNED_MANAGED_ROUND_COMPLETE']}}]
        baseline = history()
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
            assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==1
            print('Owned service restart: native history unchanged, local controller rotated, no model submission',flush=True)
        def publish_source():
            mapping, parent = {}, None
            for message in saved:
                mapping[message['id']] = {'parent':parent,'message':message}
                parent = message['id']
            temporary = root / 'saved-source.tmp'
            temporary.write_text(json.dumps({'conversation_id':'owned-saved-chat','current_node':parent,'mapping':mapping}))
            temporary.replace(root / 'saved-source.json')
        for index in [1,2]:
            # Equal text with distinct physical message IDs is two user inputs.
            text = 'Identical external input for source identity proof'
            saved += [{'id':f'web-user-{index}','role':'user','content':{'content_type':'text','parts':[text]}},
                {'id':f'web-final-{index}','role':'assistant','recipient':'all','channel':'final',
                 'status':'finished_successfully','end_turn':True,
                 'content':{'content_type':'text','parts':[f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}']}}]
            publish_source()
            wait(lambda: any(row['item'].get('text')==f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}' for row in history()),
                 f'external input {index} consumed and original answer committed')
            wait(lambda: f'WEB_ORIGINAL_COMPLETE_ANSWER_{index}' in screen, 'external answer displayed in live TUI')
            wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'external turn settled')
            current = history()
            users = [row for row in current if row['item']['type']=='userMessage' and text in json.dumps(row['item'])]
            assert len(users)==index, users
            assert len({row['item']['clientId'] for row in users})==index, users
            commands = json.loads((root / 'bridge-commands.json').read_text())
            assert sum(row['op']=='submit' for row in commands)==1, 'saved input was submitted to Chat again'
            assert sum(row['op']=='follow' for row in commands)==index
            print(f'External source {index}: actual queue consumption, canonical answer and live TUI; no primary request',flush=True)
        saved += [{'id':'web-user-3','role':'user','content':{'content_type':'text','parts':['External native tool proof']}},
            {'id':'web-answer-3','role':'assistant','recipient':'all','channel':'final',
             'status':'finished_partial_completion','end_turn':False,
             'content':{'content_type':'text','parts':['Native action in progress.']}}]
        publish_source()
        wait(lambda: sum(row['op']=='follow' for row in json.loads((root/'bridge-commands.json').read_text()))==3,
             'active external source admitted')
        original_token = submitted['instructions'][-1].split('Exact turn_token: ')[1].split('\n')[0]
        catalog = rpc('native_catalog',{'turn_token':original_token})
        current_token = catalog['turn_token']
        tools = catalog['tools']
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
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as calls:
            pending=calls.submit(rpc,'native_tool',action)
            wait(pending.done,'native retained tool result')
            result=pending.result()
        assert rpc('native_tool',action)==result, 'retained MCP retry changed its original receipt'
        assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
        if args.provider_binary:
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
                assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==1
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
        if display:
            disclosure_proof('live')
            print('Actual Provider journal CLI → live TUI: source-only folding and mouse expansion preserve complete final',flush=True)
        assert sum(row['op']=='submit' for row in json.loads((root/'bridge-commands.json').read_text()))==1
        print('Active external source: refreshed discovery, one actual native effect, retained result retry and complete final',flush=True)
        if not args.provider_binary:
            submit('Next native input after both external turns')
        wait(lambda: sum(row['op']=='submit' for row in json.loads((root / 'bridge-commands.json').read_text()))==2,
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
        wait(lambda: any(row['item'].get('text')=='WEB_ANSWER_AFTER_NATIVE_REENTRY' for row in history()),
             'repeated native/web alternation on the saved branch')
        wait(lambda: 'WEB_ANSWER_AFTER_NATIVE_REENTRY' in screen, 'repeated alternation live display')
        wait(lambda: rpc('codex_status',{'work_id':work['work_id']})['phase']=='completed', 'last external completion')
        final = history()
        assert len({row['turnId'] for row in final})==6, final
        assert len(rpc('codex_sessions',{'action':'list'})['work'])==1
        saved += [{'id':'web-user-5','role':'user','content':{'content_type':'text','parts':['Cancel this external response']}},
            {'id':'web-progress-5','role':'assistant','recipient':'all','channel':'commentary',
             'status':'finished_partial_completion','end_turn':False,
             'content':{'content_type':'text','parts':['External response still active.']}}]
        publish_source()
        wait(lambda: sum(row['op']=='follow' for row in json.loads((root/'bridge-commands.json').read_text()))==5,
             'cancellable external response admitted')
        latest=next(row for row in reversed(json.loads((root/'bridge-commands.json').read_text())) if row['op']=='submit')
        latest_token=latest['instructions'][-1].split('Exact turn_token: ')[1].split('\n')[0]
        active_token=rpc('native_catalog',{'turn_token':latest_token})['turn_token']
        stopped=rpc('codex_stop',{'work_id':work['work_id']})
        assert stopped['native_stop_confirmed'] and stopped['host_stop_confirmed'], stopped
        denied=rpc('native_tool',{**action,'turn_token':active_token,'call_id':'after-stop'},True)
        assert denied['error']=='mcp_turn_unavailable', denied
        assert rpc('native_tool',action)==result, 'Stop of a newer source lost an earlier settled receipt'
        assert (root/'w/external-effect-count').read_text()=='one-external-effect\n'
        final=history()
        assert len({row['turnId'] for row in final})==7
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
            'initial_native_items':len(baseline),'external_inputs':5,'native_turns':7,'completed_turns':6,
            'actual_external_native_effects':1,'retained_mcp_result_retry':True,
            'active_source_native_stop_and_lease_retirement':True,'actual_external_chat_server_stop':False,
            'saved_answers_reinferred':0,'primary_submissions':2,'actual_android_queue_history_and_live_tui':True,
            'actual_provider_cli_live_and_resumed_mouse_disclosure':bool(display),
            'actual_web_or_android_chat_client':False,'settled_provider_restart_same_saved_chat':bool(args.restart),'active_external_native_draft_serialization':bool(args.provider_binary),'simultaneous_entrances_qualified':False},indent=2)+'\n')
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
    parser.add_argument('--restart',action='store_true',help='Qualify settled service restart and automatic source observation with no extra native prompt')
    arguments=parser.parse_args()
    if arguments.restart and not arguments.provider_binary:
        parser.error('--restart requires the actual --provider-binary control facade')
    probe(arguments)
