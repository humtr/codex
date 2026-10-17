"""Actual signed Android Core + real adapter; deterministic app port, zero Chat."""
import argparse
import hashlib
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from siwc_core_probe import probe as core_probe


@contextmanager
def bounded_app(runtime_path):
    from desktop_lifecycle import desktop_ready, live
    stat=runtime_path.lstat()
    assert not runtime_path.is_symlink() and stat.st_uid==os.getuid() and stat.st_mode&0o777==0o600
    runtime=json.loads(runtime_path.read_text());session=Path(runtime['session'])
    assert not live(session), 'existing_desktop_must_be_preserved'
    with tempfile.TemporaryDirectory(prefix='local-app-',dir=os.environ['TMPDIR']) as temporary:
        with (Path(temporary)/'controller.log').open('w') as log:
            child=subprocess.Popen(['python','-B',str(Path(runtime['source'])/'linux_desktop_trial.py'),'serve',
                '--proot-source',runtime['proot'],'--bun',runtime['bun'],'--novnc',runtime['novnc'],
                '--seconds','300','--inspect'],stdout=log,stderr=log)
            try:
                deadline=time.monotonic()+60
                while True:
                    owner=live(session)
                    if owner and owner['pid']==child.pid and desktop_ready(session):break
                    assert child.poll() is None,'bounded_app_start_failed'
                    assert time.monotonic()<deadline,'bounded_app_start_timeout'
                    time.sleep(.2)
                yield runtime
            finally:
                if child.poll() is None:child.terminate()
                try:child.wait(timeout=20)
                except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)


def probe(bun, bridge, session=None, trace_path=None,interrupt_tool=False,return_transport='local',reject_tool=False,mcp_discovery=False,invalid_tool=False,invalid_input='malformed'):
    assert return_transport in ['local','mcp']
    assert not reject_tool or not(session or interrupt_tool or return_transport!='local'),'rejection_fixture_is_offline_only'
    assert not mcp_discovery or not(session or interrupt_tool or reject_tool or return_transport!='local'),'discovery_fixture_is_offline_only'
    assert return_transport!='mcp' or not(session or interrupt_tool),'mcp_live_control_not_qualified'
    assert not invalid_tool or not(session or interrupt_tool or reject_tool or mcp_discovery or return_transport!='local'),'invalid_input_fixture_is_offline_only'
    invalid_arguments={'malformed':'{"cmd":','array':'[1,2]','whitespace':' \n'}[invalid_input]
    if session:
        from desktop_lifecycle import desktop_ready
        assert desktop_ready(session),'live_desktop_must_be_ready_before_trial'
    from install_desktop_provider import RUNTIME_FILES
    inputs={name:hashlib.sha256((Path(__file__).parent/name).read_bytes()).hexdigest() for name in RUNTIME_FILES if name!='README.md'}
    with tempfile.TemporaryDirectory(prefix='local-core-', dir=os.environ['TMPDIR']) as temporary:
        root = Path(temporary)
        with (root/'adapter.log').open('w') as log:
            if session:
                command=[str(bun),str(Path(__file__).with_name('desktop_local_core_trace.mjs')),str(bridge),str(session),str(root)]
            else:
                command=[str(bun),str(Path(__file__).with_name('desktop_discovery_core_fixture.mjs' if mcp_discovery else 'desktop_mcp_core_fixture.mjs' if return_transport=='mcp' else 'desktop_local_core_fixture.mjs')),str(bridge),str(root)]
            child = subprocess.Popen(command, stdout=log, stderr=log, env={**os.environ,'CODEX_CHAT_RETURN_TRANSPORT':'local','CODEX_CHAT_LOCAL_MAX_DISPATCHES':'3','CODEX_LOCAL_PROBE_INTERRUPT_TOOL':'1' if interrupt_tool else '0','CODEX_LOCAL_PROBE_REJECT_TOOL':'1' if reject_tool else '0','CODEX_LOCAL_PROBE_INVALID_TOOL':'1' if invalid_tool else '0','CODEX_LOCAL_PROBE_INVALID_INPUT':invalid_input})
            try:
                binding = root/'server.json'
                deadline = time.monotonic()+10
                while not binding.exists():
                    assert child.poll() is None, 'fixture_start_failed'
                    assert time.monotonic()<deadline, 'fixture_start_timeout'
                    time.sleep(.05)
                args = argparse.Namespace(desktop_server=binding, model='chatgpt-desktop/gpt-6',
                    account_free_wire=False, compact=False, cancel_resume=False,
                    app_name='owned-local-return-fixture', effort='none', transcript_check=True,
                    mid_turn_compact=False,tui_trace_path=root/'native-tui.txt',interrupt_tool=interrupt_tool,reject_tool=reject_tool,mcp_discovery=mcp_discovery)
                try:result = core_probe(args)
                except Exception as error:
                    result=getattr(error,'probe_report',{})
                    result.setdefault('realCoreToolLoopAccepted',False)
                    result['probeFailureClass']=type(error).__name__
                state = json.loads(Path(str(binding)+'.proof').read_text())
                result['latencySamples']=state.get('latencySamples',[])
                result['productInputs']=inputs
                result['productInputsUnchanged']=all(hashlib.sha256((Path(__file__).parent/name).read_bytes()).hexdigest()==digest for name,digest in inputs.items())
                assert result['productInputsUnchanged'],'product_inputs_changed_during_trial'
                if not any([session,interrupt_tool,reject_tool,invalid_tool,mcp_discovery]) and return_transport=='local':
                    samples=result['latencySamples']
                    assert len(samples)==3 and all(x['outcome']=='completed' and 0<=x['admitted']<=x['desktopReady']<=x['exchangeSettled'] and x.get('firstDispatchObserved',-1)>=0 and x.get('firstPublicOutputObserved',-1)>=0 for x in samples),'request_timing_not_observed'

                if invalid_tool:
                    error=json.loads((root/'fixture-input-error.json').read_text()) if (root/'fixture-input-error.json').exists() else {}
                    result['nativeArgumentErrorReturned']=error.get('callId')=='fixture-call-1' and error.get('rawHistory')==invalid_arguments
                    result['invalidInputCase']=invalid_input
                    result['invalidInputResults']=state.get('returnedNativeResults')
                if trace_path and (root/'public-trace.json').exists():
                    trace_path.write_bytes((root/'public-trace.json').read_bytes());trace_path.chmod(0o600)
                if trace_path and args.tui_trace_path.exists():
                    tui_trace=trace_path.with_name(trace_path.stem+'-tui.txt')
                    tui_trace.write_bytes(args.tui_trace_path.read_bytes());tui_trace.chmod(0o600)
                if state.get('errorCode') and (root/'fixture-result.json').exists():
                    # Owned deterministic results only; no actual account/model data.
                    result['fixtureResultDiagnostic'] = json.loads((root/'fixture-result.json').read_text())
                result.update(realService=bool(session), actualSignedApp=bool(session), deterministicAppPort=not bool(session),
                    chatSubmissions=state['observedChatSubmissions'] if session else 0, proInference=False, productionChanged=False,
                    fixtureUserSubmissions=state['observedChatSubmissions'],
                    fixtureToolSubmissions=state['toolResultSubmissions'],
                    fixtureDispatches=state.get('selectedControl', {}).get('dispatches'),
                    fixtureTotalDispatches=state.get('observedNativeDispatches',0),
                    adapterError=state.get('errorCode'), failureSite=state.get('failureSite'), nativeFailure=state.get('nativeFailure'), returnTransport=state['returnTransport'])
                if return_transport=='mcp':
                    proof=root/'mcp-fixture-proof.json'
                    result['mcpWire']=json.loads(proof.read_text()) if proof.exists() else {}
                    result['returnedNativeResults']=state['returnedNativeResults']
                result['accepted'] = result.pop('realCoreToolLoopAccepted') and (1<=result['fixtureUserSubmissions']<=2 if session else result['fixtureUserSubmissions']==(2 if interrupt_tool or reject_tool or mcp_discovery else 1)) and result['fixtureToolSubmissions']==(0 if return_transport=='mcp' else 3 if invalid_tool else 2) and result['fixtureDispatches']==(1 if return_transport=='mcp' else 2 if reject_tool else 4 if invalid_tool else 3) and not result['adapterError'] and all(result.get(k,False) for k in ['persistedReportCheck','coldTranscriptRestored','resumeWithoutModelRequest','renderWithoutModelRequest']) and all(result.get('renderedTranscript',{}).get(k) for k in ['initial','intermediate','final'])
                if reject_tool:result['accepted'] &= all(result.get(k) for k in ['rejectedSampleObserved','noAutomaticResubmit','recoveryPreservedExistingFile']) and result['successfulShellCalls']==2 and result['persistedNativeCommands']==2 and result['fixtureTotalDispatches']==4
                if invalid_tool:result['accepted'] &= result['nativeArgumentErrorReturned'] and result['successfulShellCalls']==2 and result['persistedNativeCommands']==2 and state['returnedNativeResults']==3 and state['requests']==4 and state['observedChatSubmissions']==1 and state['toolResultSubmissions']==3
                if mcp_discovery:result['accepted'] &= result.get('ownedMcpCalls')==1 and result.get('persistedMcpCalls')==1 and state['modelRequestedSearches']==1 and state['nativeSearchLoadedTools']==1
                if return_transport=='mcp':result['accepted'] &= result['mcpWire'].get('exactCoreReturns')==2 and result['returnedNativeResults']==2
                return result
            finally:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill(); child.wait(timeout=5)
                if return_transport=='mcp' and binding.exists():
                    socket=Path(json.loads(binding.read_text())['mcpSocket'])
                    if isinstance(locals().get('result'),dict):
                        result['ownedMcpSocketRemoved']=not socket.exists() and not socket.parent.exists()
                        result['accepted'] = result.get('accepted',False) and result['ownedMcpSocketRemoved']


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    for name in ['bun', 'bridge', 'report']:
        parser.add_argument('--'+name, type=Path, required=True)
    parser.add_argument('--live-session',type=Path,help='Explicit bounded live proof, at most three native generation starts; never Pro')
    parser.add_argument('--bounded-runtime',type=Path,help='Start an otherwise stopped trial app for at most 300 seconds and close only that owned controller')
    parser.add_argument('--interrupt-tool',action='store_true',help='Interrupt an owned real native command, then resume without replaying its mutation')
    parser.add_argument('--reject-tool',action='store_true',help='Offline only: reject a completed tool sample, then recover with new input without repeating a completed command')
    parser.add_argument('--mcp-discovery',action='store_true',help='Offline only: actual Core search/discovered namespace/MCP result path')
    parser.add_argument('--invalid-tool',action='store_true',help='Offline only: preserve invalid JSON, return actual Core error, then correct without a new user message')
    parser.add_argument('--invalid-input',choices=['malformed','array','whitespace'],default='malformed')
    parser.add_argument('--return-transport',choices=['local','mcp'],default='local',help='mcp is an offline real-Core/stdio qualification; no live Chat acceptance implied')
    args = parser.parse_args()
    assert not(args.live_session and args.bounded_runtime)
    assert not args.reject_tool or not(args.live_session or args.bounded_runtime or args.interrupt_tool or args.return_transport!='local')
    assert not args.mcp_discovery or not(args.live_session or args.bounded_runtime or args.interrupt_tool or args.reject_tool or args.return_transport!='local')
    assert args.return_transport!='mcp' or not(args.live_session or args.bounded_runtime or args.interrupt_tool),'mcp_live_control_not_qualified'
    assert not args.invalid_tool or not(args.live_session or args.bounded_runtime or args.interrupt_tool or args.reject_tool or args.mcp_discovery or args.return_transport!='local')
    trace=args.report.with_name(args.report.stem+'-public-trace.json')
    if args.bounded_runtime:
        with bounded_app(args.bounded_runtime) as runtime:
            assert Path(runtime['bun'])==args.bun and Path(runtime['bridge'])==args.bridge
            result=probe(args.bun,args.bridge,Path(runtime['session']),trace,args.interrupt_tool,args.return_transport)
    else:
        result = probe(args.bun, args.bridge,args.live_session,trace,args.interrupt_tool,args.return_transport,args.reject_tool,args.mcp_discovery,args.invalid_tool,args.invalid_input)
    args.report.write_text(json.dumps(result, indent=2)+'\n')
    args.report.chmod(0o600)
    print(json.dumps(result))
    raise SystemExit(0 if result['accepted'] else 1)
