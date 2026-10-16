"""Actual signed Core surface trial; owned files, bounded model port, durable proof."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import traceback

from siwc_core_probe import probe as core_probe
from desktop_lifecycle import RUNTIME, desktop_ready, ready
from desktop_development_scenario import Development
from desktop_generation_scenario import Generation
from desktop_repository_scenario import RepositoryDevelopment

SOURCE = Path(__file__).resolve().parent
PROBE_DIGEST = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def restart_idle_app(runtime, root, output):
    """Explicit idle app fault, allowing only the settled trial's own lane."""
    from desktop_lifecycle import live, ensure
    from desktop_lifecycle_probe import stop_owned
    from linux_desktop_trial import write
    binding=Path(runtime['binding']);identity=live(binding);assert identity
    observed=json.loads(Path(str(binding)+'.proof').read_text())
    assert observed['requests']==observed['arrivals']==0,'operating_adapter_has_activity'
    # The app relay has one CDP peer. Inspect through the existing trial lease,
    # preserving that lease so this really exercises generation-change recovery.
    import secrets
    nonce=secrets.token_hex(16)
    write(root/'inspect-request.json',{'nonce':nonce})
    deadline=time.monotonic()+20
    while not (root/'inspect-result.json').exists():
        assert time.monotonic()<deadline,'app_idle_inspection_timeout'
        time.sleep(.05)
    reply=json.loads((root/'inspect-result.json').read_text())
    assert reply['nonce']==nonce and 'error' not in reply,'app_idle_inspection_failed'
    state=reply['state'];write(output/'app-idle-inspection.json',state)
    assert not state['foreignLanes'] and not state['composerHasDraft'] and not state['visibleGeneration'],'app_is_not_idle'
    assert live(binding)==identity
    observed=json.loads(Path(str(binding)+'.proof').read_text())
    assert observed['requests']==observed['arrivals']==0,'operating_adapter_started_work'
    previous=stop_owned(Path(runtime['session']))
    ensure(desktop_only=True)
    current=live(Path(runtime['session']));assert current and current['pid']!=previous['pid'] and current['root']!=previous['root']
    assert live(binding)==identity,'operating_adapter_restarted_unexpectedly'
    return {'signedAppRestartedAfterSettledTurn':True,'noForeignLaneOrDraftInterrupted':True,'operatingAdapterRetained':True}


class Surface:
    def __init__(self, output):
        self.output = output
        self.questions = []
        self.resolved = []

    def configure(self, config, root, home, workspace, marker):
        (workspace/'nonce.txt').write_text(marker)
        (workspace/'worker.py').write_text('import sys,time\nfrom pathlib import Path\nx=sys.argv[1]\nassert x in ["A","B"]\nPath(x+".start").write_text(str(time.monotonic_ns()))\nprint(x+"_START",flush=True)\ntime.sleep(5)\nPath(x+".end").write_text(str(time.monotonic_ns()))\nprint(x+"_DONE",flush=True)\n')
        config = config.replace('memories = false\n', 'memories = false\ndefault_mode_request_user_input = true\n')
        return config+'\n[mcp_servers.owned_catalog]\ncommand = '+json.dumps(os.environ['PREFIX']+'/bin/python')+'\nargs = '+json.dumps(['-B',str(SOURCE/'owned_catalog_mcp.py'),str(workspace/'nonce.txt'),str(workspace/'mcp-evidence.json')])+'\n'

    def task(self, marker):
        return (f'The marker is {marker}. This is an explicitly authorized development-surface trial only in the current disposable directory. '
            'Say LOCAL_INITIAL_REPORT once before tools. Do these steps in order: '
            '1. Use tool_search to discover owned_catalog _read_marker. '
            '2. Call that exact discovered MCP tool ONCE; use the actual returned marker. '
            '3. Use the advertised custom/freeform apply_patch tool to create proof.txt containing exactly the marker and a newline. '
            '4. In ONE local gateway call use its calls parameter: a JSON-encoded array of TWO independent name/input pairs for the JSON exec_command tool: '
            'python worker.py A and python worker.py B, each login=false, yield_time_ms=1000, max_output_tokens=200. '
            'Do not load login profiles or start Termux services in this isolated test home. '
            'Issue both calls together without waiting for the first result; do not combine them in a shell command. '
            '5. Again use one gateway calls array to poll the TWO returned background session IDs with write_stdin, chars="", yield_time_ms=10000 until both A_DONE and B_DONE arrive. Never restart either command. '
            '6. Say LOCAL_INTERMEDIATE_REPORT once. Call request_user_input with exactly one question: id fixture_choice, header Surface, question "Choose KEEP for this disposable test?", two options KEEP and OTHER with descriptions. '
            'The test driver will answer KEEP through the native question-response API. Wait for that actual answer. '
            '7. Final answer exactly the marker. Do not fake tool outputs, skip a required tool, run additional commands, or read credentials. If a tool is missing, explain and stop.')

    def observe(self, row):
        if row.get('method') == 'serverRequest/resolved':
            self.resolved.append(row['params'].get('requestId'))

    def respond(self, row, send):
        if row.get('method') != 'item/tool/requestUserInput':
            return False
        questions = row.get('params', {}).get('questions', [])
        assert len(questions)==1 and questions[0]['id']=='fixture_choice'
        assert questions[0]['header']=='Surface' and any(o['label']=='KEEP' for o in questions[0]['options'])
        assert not self.questions, 'duplicate fixture question'
        self.questions.append(row['id'])
        send({'id':row['id'],'result':{'answers':{'fixture_choice':{'answers':['KEEP']}}}})
        return True

    def verify(self, root, home, workspace, items, report):
        payloads = [row.get('payload', {}) for path in (home/'.codex/sessions').rglob('*.jsonl')
            for line in path.read_text().splitlines() if line.strip()
            for row in [json.loads(line)] if row.get('type')=='response_item']
        (self.output/'native-tools.json').write_text(json.dumps([p for p in payloads if p.get('type') in ['function_call','function_call_output','custom_tool_call','custom_tool_call_output','tool_search_call','tool_search_output']],indent=2)+'\n')
        (self.output/'native-items.json').write_text(json.dumps([i for i in items if i.get('type') not in ['agentMessage','reasoning','userMessage']],indent=2)+'\n')
        custom = [p for p in payloads if p.get('type')=='custom_tool_call' and p.get('name')=='apply_patch']
        calls = [p for p in payloads if p.get('type')=='function_call']
        poll = [p for p in calls if p.get('name')=='write_stdin']
        evidence = json.loads((workspace/'mcp-evidence.json').read_text()) if (workspace/'mcp-evidence.json').exists() else {}
        stamps = {name:int((workspace/name).read_text()) for name in ['A.start','A.end','B.start','B.end'] if (workspace/name).exists()}
        overlap = len(stamps)==4 and max(stamps['A.start'],stamps['B.start'])<min(stamps['A.end'],stamps['B.end'])
        exact_answer = any(p.get('type')=='function_call_output' and 'KEEP' in str(p.get('output')) for p in payloads)
        result = {'mcpCalls':evidence.get('actualCalls',0),'freeformPatches':len(custom),
            'successfulCommands':report['successfulShellCalls'],'backgroundPolls':len(poll),
            'backgroundOverlap':overlap,'startGapSeconds':abs(stamps['A.start']-stamps['B.start'])/1e9 if len(stamps)==4 else None,
            'questionRequests':len(self.questions),'questionResolved':all(q in self.resolved for q in self.questions),
            'answerReturnedToModel':exact_answer,'answerSource':'explicit deterministic test driver, not physical user interaction',
            'persistedFileChanges':sum(i.get('type')=='fileChange' for i in items)}
        result['accepted'] = (result['mcpCalls']==1 and result['freeformPatches']==1 and result['successfulCommands']==2
            and result['backgroundPolls']>=2 and overlap and result['questionRequests']==1 and result['questionResolved'] and exact_answer)
        return result


def run(args):
    runtime = json.loads(RUNTIME.read_text())
    installed = Path(runtime['installation'])
    if args.live:
        assert ready(runtime) and desktop_ready(Path(runtime['session'])), 'operating_app_must_be_ready'
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='surface-', dir=os.environ['TMPDIR']) as temporary:
        root = Path(temporary)
        if args.live:
            candidate = SOURCE if args.source_candidate else installed/'source'
            wrapper = root/'bounded.mjs'
            wrapper.write_text('import {DesktopLocalTransport} from '+json.dumps((candidate/'desktop_local_transport.mjs').as_uri())+';\nconst root=process.argv[4];\nconst p=DesktopLocalTransport.prototype.prepare;DesktopLocalTransport.prototype.prepare=function(...args){this.maxDispatches='+str(48 if args.repository else 24 if args.development else 4 if args.generation else 12)+';return p.apply(this,args);};\nconst e=DesktopLocalTransport.prototype.exchange,b=[];\nDesktopLocalTransport.prototype.exchange=async function(...args){const r=await e.apply(this,args);b.push({type:r.type,calls:(r.calls||[]).map(c=>({id:c.id,name:c.name}))});(await import("node:fs")).writeFileSync(root+"/surface-batches.json",JSON.stringify(b),{mode:0o600});return r;};\nawait import('+json.dumps((candidate/'desktop_local_core_trace.mjs').as_uri())+');\n')
            command = [runtime['bun'],str(wrapper),runtime['bridge'],runtime['session'],str(root)]
            if args.restart_app:
                hook=(SOURCE/'desktop_idle_inspection.mjs').as_uri()
                wrapper.write_text(wrapper.read_text().replace('return r;};',
                    'await (await import('+json.dumps(hook)+')).watchIdleInspection(this,root);return r;};'))
        else:
            command = [runtime['bun'],str(SOURCE/('desktop_development_fixture.mjs' if args.development else 'desktop_surface_fixture.mjs')),runtime['bridge'],str(root)]
        product_source = SOURCE if args.source_candidate or not args.live else installed/'source'
        from install_desktop_provider import RUNTIME_FILES
        product_digests = {n:hashlib.sha256((product_source/n).read_bytes()).hexdigest()
            for n in RUNTIME_FILES if n!='README.md'}
        scenario_source = SOURCE/('desktop_repository_scenario.py' if args.repository else 'desktop_development_scenario.py')
        with (args.output/'adapter.log').open('w') as log:
            child = subprocess.Popen(command,stdout=log,stderr=log,env={**os.environ,'CODEX_CHAT_DEVELOPMENT_COMPACTION':args.compaction})
            result = {}
            try:
                deadline=time.monotonic()+15
                while not (root/'server.json').exists():
                    assert child.poll() is None,'owned_adapter_failed'
                    assert time.monotonic()<deadline,'owned_adapter_timeout'
                    time.sleep(.1)
                options=argparse.Namespace(desktop_server=root/'server.json',model='chatgpt-desktop/gpt-6',account_free_wire=False,
                    compact=False,cancel_resume=False,app_name='owned-native-surface',effort='none',transcript_check=True,
                    mid_turn_compact=False,interrupt_tool=False,reject_tool=False,mcp_discovery=False,
                    scenario=RepositoryDevelopment(args.output,args.repository) if args.repository else Development(args.output,args.compaction) if args.development else Generation() if args.generation else Surface(args.output),tui_trace_path=args.output/'tui.txt')
                if args.restart_app:options.scenario.restart_app=lambda:restart_idle_app(runtime,root,args.output)
                started=time.monotonic()
                try:
                    result=core_probe(options)
                except Exception as error:
                    result=getattr(error,'probe_report',{})
                    result.update(realCoreToolLoopAccepted=False,failureClass=type(error).__name__,
                        failureFrames=[{'file':Path(f.filename).name,'line':f.lineno} for f in traceback.extract_tb(error.__traceback__)][-5:])
                proof=json.loads((root/'server.json.proof').read_text())
                for name in ['surface-host.json','surface-batches.json','public-trace.json','fixture-context.json']:
                    if (root/name).exists():
                        (args.output/name).write_bytes((root/name).read_bytes())
                (args.output/'adapter-proof.json').write_text(json.dumps(proof,indent=2)+'\n')
                batches=json.loads((args.output/'surface-batches.json').read_text()) if (args.output/'surface-batches.json').exists() else json.loads((args.output/'surface-host.json').read_text()) if (args.output/'surface-host.json').exists() else []
                batch_sizes=[len(b.get('calls',b.get('callIds',[]))) for b in batches]
                result.update(realService=args.live,actualSignedApp=args.live,installedPayload=installed.name,sourceCandidate=args.source_candidate,
                    productDigests=product_digests,productInputsUnchanged=all(hashlib.sha256((product_source/n).read_bytes()).hexdigest()==d for n,d in product_digests.items()),actualChatUserSubmissions=proof['observedChatSubmissions'] if args.live else 0,
                    observedUserSubmissions=proof['observedChatSubmissions'],toolReturns=proof['toolResultSubmissions'],
                    modelStarts=proof.get('observedNativeDispatches',0),guardianRequests=proof['guardianRequests'],compactionSummaries=proof['compactionSummaries'],appliedCheckpoints=proof['appliedCheckpoints'],
                    automaticRetries=proof['retryAttempts'],proStarts=proof['submittedProRequests'],adapterError=proof.get('errorCode'),
                    batchSizes=batch_sizes,parallelBatchObserved=2 in batch_sizes,elapsedSeconds=round(time.monotonic()-started,3),
                    probeSha256=PROBE_DIGEST,
                    fixtureSha256=hashlib.sha256((SOURCE/('desktop_development_fixture.mjs' if args.development else 'desktop_surface_fixture.mjs')).read_bytes()).hexdigest(),
                    repositoryScenario=str(args.repository) if args.repository else None,developmentScenario=args.development,generationScenario=args.generation,compactionMode=args.compaction,appRestartRequested=args.restart_app,scenarioSha256=hashlib.sha256((SOURCE/'desktop_generation_scenario.py' if args.generation else scenario_source).read_bytes()).hexdigest() if args.development or args.repository or args.generation else None)
                result['accepted']=bool(result.get('realCoreToolLoopAccepted') and result.get('persistedReportCheck') and result.get('coldTranscriptRestored')
                    and result.get('resumeWithoutModelRequest') and result.get('renderWithoutModelRequest') and (args.repository or args.development or args.generation or result['parallelBatchObserved'])
                    and all(result.get('renderedTranscript',{}).get(k) for k in ['initial','intermediate','final'])
                    and (not args.development or result['compactionSummaries']>=1 and result['appliedCheckpoints']>=1)
                    and (args.repository or args.development or args.generation or result['guardianRequests']>=1) and not result['adapterError'] and result['automaticRetries']==0 and result['proStarts']==0)
            finally:
                if 'options' in locals() and hasattr(options.scenario,'cleanup'):
                    options.scenario.cleanup()
                child.terminate()
                try:
                    child.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    child.kill();child.wait(timeout=5)
            result['ownedAdapterExited']=child.poll() is not None
    result['temporaryRootRemoved']=not root.exists()
    (args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
    return result.get('accepted',False)


if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--live',action='store_true',help='Explicit actual GPT-6 Instant trial, at most12 model starts including guardian; no Pro or automatic retry')
    p.add_argument('--repository',type=Path,help='Actual tracked repository copy evaluation; requires --live; cap48 starts/20minutes')
    p.add_argument('--development',action='store_true',help='Iterative development, manual compaction and resume; live cap24 model starts')
    p.add_argument('--generation',action='store_true',help='Short two-request Core/app restart proof; requires live source and --restart-app')
    p.add_argument('--compaction',choices=['manual','automatic','mid-turn','failed','interrupted'],default='manual',help='Development checkpoint trigger or owned fault injection')
    p.add_argument('--source-candidate',action='store_true',help='Use this source worktree with existing installed immutable dependencies; do not change operating runtime')
    p.add_argument('--restart-app',action='store_true',help='Explicit idle signed-app restart between settled development turns; no foreign lanes or drafts allowed')
    p.add_argument('--output',required=True,type=Path)
    args=p.parse_args()
    if args.repository and (not args.live or args.development):p.error('--repository requires --live and cannot combine with --development')
    if args.compaction!='manual' and not args.development:p.error('--compaction requires --development')
    if args.generation and (args.development or args.repository or not args.restart_app):p.error('--generation requires --restart-app and its own isolated scenario')
    if args.restart_app and not(args.live and (args.development or args.generation) and args.source_candidate):p.error('--restart-app requires --live --development/--generation --source-candidate')
    raise SystemExit(0 if run(args) else 1)
