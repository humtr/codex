"""A completed official-plan Core turn with real shell tools in disposable state."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import http.server
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import threading
import fcntl
import pty
import re
import struct
import termios
from protocol import Transcript, events as response_events

SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE.parents[1]/'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest


def sha(path):
    with path.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()


def render_transcript(launcher, workspace, env, thread_id, marker, trace_path=None):
    """Render only an owned persisted fixture; never send a user/model turn."""
    master, slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',60,120,0,0))
    def session():
        os.setsid();fcntl.ioctl(slave,termios.TIOCSCTTY,0)
    child=subprocess.Popen([str(launcher),'--no-daemon','--no-alt-screen','resume',thread_id],cwd=workspace,
        env=env,stdin=slave,stdout=slave,stderr=slave,preexec_fn=session,close_fds=True)
    os.close(slave);output=b'';trusted=False;rendered={}
    try:
        deadline=time.monotonic()+20
        while time.monotonic()<deadline and child.poll() is None:
            if not select.select([master],[],[],.1)[0]:continue
            try:chunk=os.read(master,65536)
            except OSError:break
            for query,response in [(b'\x1b[6n',b'\x1b[1;1R'),(b'\x1b[c',b'\x1b[?1;2c'),
                (b'\x1b]10;?',b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\'),(b'\x1b]11;?',b'\x1b]11;rgb:0000/0000/0000\x1b\\')]:
                if query in chunk:os.write(master,response)
            output+=chunk
            text=re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]','',output.decode(errors='replace'))
            if not trusted and re.search(r'do you trust|trust this (?:directory|folder|workspace)',text,re.I):
                os.write(master,b'\r');trusted=True
            # User task and command echo contain the same markers: require the
            # native assistant bullet, not a substring anywhere in the screen.
            rendered={key:bool(re.search(r'(?:^|\n)\s*•\s*'+re.escape(value),text)) for key,value in
                [('initial','LOCAL_INITIAL_REPORT'),('intermediate','LOCAL_INTERMEDIATE_REPORT'),('final',marker)]}
            if all(rendered.values()):return {**rendered,'nativeTuiRunning':True}
        return rendered
    finally:
        if trace_path:
            trace_path.write_bytes(output);trace_path.chmod(0o600)
        if child.poll() is None:
            os.killpg(child.pid,signal.SIGINT)
            try:child.wait(timeout=3)
            except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGTERM);child.wait(timeout=5)
        os.close(master)


def probe(args):
    real_home=Path.home();host_prefix=Path(os.environ['PREFIX'])
    desktop=None
    if args.desktop_server:
        assert args.model in ['chatgpt-desktop/instant','chatgpt-desktop/gpt-6','chatgpt-desktop/gpt-5.6-sol'], 'automated Pro inference prohibited'
        record=args.desktop_server.lstat()
        assert not args.desktop_server.is_symlink() and record.st_uid==os.getuid() and not record.st_mode&0o077
        desktop=json.loads(args.desktop_server.read_text())
        from urllib.parse import urlparse
        endpoint=urlparse(desktop['baseUrl']);assert endpoint.scheme=='http' and endpoint.hostname=='127.0.0.1' and endpoint.port and endpoint.path=='/v1'
        proc=Path('/proc')/str(desktop['pid']);stat=(proc/'stat').read_text();assert stat[stat.rfind(')')+2:].split()[19]==desktop['start'] and os.readlink(proc/'exe')==desktop['image']
        # The private binding identifies the service, not the requested family.
        # Per-request routing is validated by the adapter before Chat submission.
        assert desktop['model']=='chatgpt-desktop/instant';token=desktop['token']
    elif args.account_free_wire:token='owned-synthetic-wire-token'
    else:
        record=args.connection.lstat()
        assert not args.connection.is_symlink() and record.st_uid==os.getuid() and not record.st_mode&0o077
        connection=json.loads(args.connection.read_text())
        assert connection['status']=='connected' and 'chatgpt.tokens.use.direct' in connection['scopes'] and not connection.get('pendingRefresh')
        token=connection['credentials']['accessToken']
        assert connection['credentials']['expiresAt']>time.time()*1000+180000
    activation=dict(x.split('=',1) for x in (real_home/'.local/share/codex/core/activation-state').read_text().splitlines() if '=' in x)
    generation=real_home/'.local/lib/codex/core/generations'/activation['current']
    fields=parse_descriptor(generation/'generation.meta');_,inventory=parse_manifest(generation/'release.manifest')
    report={'realService':not args.account_free_wire,'nativeCoreGeneration':fields['generation_id'],'model':args.model,'realCoreToolLoopAccepted':False,'turnCompleted':False,'shellToolCompleted':False,'workspaceMarkerVerified':False,'successfulShellCalls':0,'nativeReplyMatched':False}
    with tempfile.TemporaryDirectory(prefix='plan-core-',dir=os.environ['TMPDIR']) as temporary:
        root=Path(temporary);root.chmod(0o700)
        home,prefix,workspace=root/'h',root/'p',root/'w'
        for p in (home/'.codex',prefix/'bin',prefix/'etc/tls',workspace,home/'.local/share/codex/core/config',root/'t'):p.mkdir(parents=True,mode=0o700)
        der=root/'key.der';der.write_bytes(bytes.fromhex('302a300506032b6570032100'+activation['current_key']))
        public=root/'public.pem'
        subprocess.run(['openssl','pkey','-pubin','-inform','DER','-in',str(der),'-out',str(public)],check=True,capture_output=True)
        subprocess.run(['openssl','pkeyutl','-verify','-pubin','-inkey',str(public),'-rawin','-in',str(generation/'release.manifest'),'-sigfile',str(generation/'release.sig')],check=True,capture_output=True)
        for name,digest,mode in inventory:assert sha(generation/name)==digest and (generation/name).stat().st_mode&0o777==int(mode,8)
        installed=home/'.local/lib/codex/core/generations'/fields['generation_id'];materialize_signed_generation(generation,installed)
        launcher=prefix/'bin/codex';shutil.copy2(generation/'core',launcher)
        (prefix/'bin/openssl').symlink_to(shutil.which('openssl'))
        shutil.copy2(host_prefix/'etc/tls/cert.pem',prefix/'etc/tls/cert.pem')
        shutil.copy2(host_prefix/'etc/resolv.conf',prefix/'etc/resolv.conf')
        (home/'.local/share/codex/core/activation-state').write_text(fixture_activation_state(fields['generation_id'],public))
        marker=os.urandom(16).hex();backend=None;wire_failures=[];transcript=Transcript(marker)
        base_url=desktop['baseUrl'] if desktop else 'https://api.openai.com/v1'
        if args.account_free_wire:
            (workspace/'nonce.txt').write_text(marker+'\n');(workspace/'proof.txt').write_text(marker+'\n')
            class Handler(http.server.BaseHTTPRequestHandler):
                def log_message(self,*_):pass
                def do_POST(self):
                    try:
                        assert self.path=='/v1/responses' and self.headers.get('Authorization')=='Bearer owned-synthetic-wire-token'
                        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                        unsupported={'background','conversation','max_output_tokens','max_tool_calls','metadata','moderation','multi_agent','prompt','prompt_cache_retention','safety_identifier','temperature','top_logprobs','top_p','truncation','user','previous_response_id'}&body.keys()
                        if unsupported:wire_failures.append({'unsupportedFields':sorted(unsupported)});raise ValueError('unsupported fields')
                        assert body.get('store') is False and body.get('stream') is True
                        transcript.receive(body)
                        if any(kind in ('tool_search','image_generation','file_search','code_interpreter','computer_use') for kind in (v['type'] for v in transcript.routes.values())):raise ValueError('unsupported tools')
                        if len(transcript.calls)<2:item=transcript.next_command(workspace,{k:env[k] for k in ('HOME','CODEX_HOME','PREFIX')})
                        else:item={'id':'msg_owned','type':'message','role':'assistant','status':'completed','content':[{'type':'output_text','text':transcript.final_reply(),'annotations':[]}]}
                        payload=response_events(item,len(transcript.calls));self.send_response(200);self.send_header('Content-Type','text/event-stream');self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
                    except Exception as e:
                        wire_failures.append({'failureClass':type(e).__name__,'fixtureDiagnostic':str(e)[:180]});self.send_response(400);self.end_headers()
            backend=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
            threading.Thread(target=backend.serve_forever,daemon=True).start();base_url=f'http://127.0.0.1:{backend.server_port}/v1'
        config=f'''model = {json.dumps(args.model)}
model_provider = "openai_chatgpt_plan"
model_reasoning_effort = {json.dumps((getattr(args,'effort',None) or 'none') if desktop else 'low')}
approval_policy = "on-request"
approvals_reviewer = "auto_review"
sandbox_mode = "danger-full-access"
web_search = "disabled"
check_for_update_on_startup = false
[analytics]
enabled = false
[feedback]
enabled = false
[features]
memories = false
[shell_environment_policy]
inherit = "none"
set = {{ PATH = {json.dumps(str(host_prefix/'bin'))}, HOME = {json.dumps(str(home))}, PREFIX = {json.dumps(str(prefix))}, CODEX_HOME = {json.dumps(str(home/'.codex'))}, TMPDIR = {json.dumps(str(root/'t'))} }}
[model_providers.openai_chatgpt_plan]
name = "ChatGPT plan"
base_url = {json.dumps(base_url)}
env_key = "ACCESS_TOKEN"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
'''
        if desktop:
            # Match the installed model's native catalog/search capabilities.
            # An unknown model fallback eagerly expands unrelated schemas and
            # does not exercise the actual installed provider configuration.
            catalog = real_home/'.local/share/codex-web-experiment/native-provider/models.json'
            assert catalog.is_file() and not catalog.is_symlink()
            assert any(m['slug']==args.model for m in json.loads(catalog.read_text())['models'])
            config='model_catalog_json = '+json.dumps(str(catalog))+'\n'+config
        (home/'.codex/config.toml').write_text(config)
        env={'HOME':str(home),'PREFIX':str(prefix),'PATH':str(prefix/'bin')+':'+str(host_prefix/'bin'),'TMPDIR':str(root/'t'),'LANG':'C.UTF-8','CODEX_HOME':str(home/'.codex'),'ACCESS_TOKEN':token,'SSL_CERT_FILE':str(prefix/'etc/tls/cert.pem'),'TERM':'xterm-256color','SHELL':str(host_prefix/'bin/bash')}
        process=subprocess.Popen([str(launcher),'app-server','--listen','stdio://'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
        def send(value):process.stdin.write(json.dumps(value)+'\n');process.stdin.flush()
        buffer='';pending={};events=[]
        def receive(deadline):
            nonlocal buffer
            while time.monotonic()<deadline:
                if '\n' in buffer:
                    line,buffer=buffer.split('\n',1)
                    try:return json.loads(line)
                    except ValueError:continue
                ready,_,_=select.select([process.stdout,process.stderr],[],[],min(1,max(0,deadline-time.monotonic())))
                for stream in ready:
                    raw=os.read(stream.fileno(),65536)
                    if not raw and process.poll() is not None:return None
                    if stream is process.stdout:buffer+=raw.decode(errors='replace')
                    # stderr is drained without storing credentials or body text.
            return None
        def rpc(identifier,method,params,deadline):
            send({'id':identifier,'method':method,'params':params})
            while time.monotonic()<deadline:
                row=receive(deadline)
                if row is None:raise RuntimeError('rpc_timeout')
                if row.get('id')==identifier:
                    if 'error' in row:raise RuntimeError('rpc_rejected_'+method.replace('/','_'))
                    return row['result']
                if 'method' in row:events.append(row)
        try:
            deadline=time.monotonic()+(1200 if args.compact else 300 if desktop else 180)
            rpc(1,'initialize',{'clientInfo':{'name':args.app_name,'title':'Codex Termux Trial','version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline)
            send({'method':'initialized','params':{}})
            thread=rpc(2,'thread/start',{'model':args.model,'cwd':str(workspace),'approvalPolicy':'on-request','sandbox':'danger-full-access','ephemeral':not(args.cancel_resume or args.compact or getattr(args,'transcript_check',False)),'config':{'model_auto_compact_token_limit':16300,'model_auto_compact_token_limit_scope':'total'} if args.mid_turn_compact else {}},deadline)
            thread_id=thread['thread']['id']
            task=f'Use the local shell tool to write exactly {marker} to proof.txt in the current directory. Use a separate second shell tool call to read it back. Reply with exactly {marker}. No other work.'
            if getattr(args,'transcript_check',False):
                task=f'First say LOCAL_INITIAL_REPORT in commentary. Use the local shell tool to write exactly {marker} to proof.txt in the current directory. After its result say LOCAL_INTERMEDIATE_REPORT in commentary. Use a separate second shell tool call to read it back. Your final answer must be exactly {marker}. Do no other work.'
            if getattr(args,'interrupt_tool',False):
                task=f'First say LOCAL_INITIAL_REPORT. Run exactly one shell command with yield_time_ms 30000: printf %s {marker} > proof.txt; echo $$ > command.pid; sleep 45; printf MUTATION_REPLAYED >> proof.txt. Wait for its result. Final answer exactly {marker}. Do no other work.'
            if args.compact:
                seed_text=marker+'\n'+('Reference-only context row; no action.\n'*220 if args.auto_compact or args.mid_turn_compact else '')
                (workspace/'seed.txt').write_text(seed_text)
                task='Read all of seed.txt in one native shell call. Its first line is PROOF_SECRET; the remaining lines are reference only. Remember only the first line as PROOF_SECRET without rereading the seed. In a separate second native shell call write that exact value to proof.txt. Final reply must be exactly that value. No other work.'
            initial_submitted=0;initial_cancelled=0
            proxy_proof=Path(str(args.desktop_server)+'.proof') if desktop else None
            if desktop and not proxy_proof.exists():
                session=json.loads((real_home/'.local/share/codex-web-experiment/linux-desktop/session.json').read_text())
                installed=real_home/'.local/share/codex-web-experiment/native-provider/runtime.json'
                actual=real_home/'.local/share/codex-web-experiment/native-provider/server.json' if installed.exists() else Path(session['root'])/'desktop-server.json';assert json.loads(actual.read_text())==desktop,'desktop_proof_owner_mismatch'
                proxy_proof=Path(str(actual)+'.proof')

            if args.cancel_resume and proxy_proof.exists():
                before=json.loads(proxy_proof.read_text());initial_submitted=before.get('submittedRequests',0);initial_cancelled=before.get('cancelledRequests',0)
            initial_compaction_summaries=json.loads(proxy_proof.read_text()).get('compactionSummaries',0) if proxy_proof else 0
            started=rpc(3,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':task}]},deadline)
            if args.cancel_resume:
                submitted=False
                while time.monotonic()<deadline:
                    if proxy_proof.exists() and json.loads(proxy_proof.read_text()).get('submittedRequests',0)>initial_submitted:
                        submitted=True;break
                    row=receive(min(deadline,time.monotonic()+.1))
                    if row:
                        if row.get('method')=='turn/completed':raise RuntimeError('desktop_ended_before_cancel_submit')
                        events.append(row)
                report['cancelAfterActualSubmit']=submitted
                if not submitted:raise RuntimeError('desktop_submit_not_observed')
                rpc(4,'turn/interrupt',{'threadId':thread_id,'turnId':started['turn']['id']},deadline)
                before_cancel_commands=0
                while time.monotonic()<deadline:
                    row=events.pop(0) if events else receive(deadline)
                    if row is None:break
                    if row.get('method')=='item/completed' and row.get('params',{}).get('item',{}).get('type')=='commandExecution':before_cancel_commands+=1
                    if row.get('method')=='turn/completed':
                        report['nativeCancelAccepted']=row.get('params',{}).get('turn',{}).get('status')=='interrupted';break
                report['noToolBeforeCancel']=before_cancel_commands==0
                for _ in range(100):
                    if proxy_proof.exists() and json.loads(proxy_proof.read_text()).get('cancelledRequests',0)>initial_cancelled:break
                    time.sleep(.1)
                report['desktopCancelSettled']=proxy_proof.exists() and json.loads(proxy_proof.read_text()).get('cancelledRequests',0)>initial_cancelled
                os.killpg(process.pid,signal.SIGINT)
                try:process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=5)
                report['firstCoreStoppedBeforeResume']=process.poll() is not None
                process.stdin.close();process.stdout.close();process.stderr.close()
                process=subprocess.Popen([str(launcher),'app-server','--listen','stdio://'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
                buffer='';events.clear()
                rpc(5,'initialize',{'clientInfo':{'name':args.app_name,'title':'Codex Termux Trial','version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline)
                send({'method':'initialized','params':{}})
                resumed=rpc(6,'thread/resume',{'threadId':thread_id,'approvalPolicy':'on-request','sandbox':'danger-full-access'},deadline)
                report['nativeResumeAccepted']=resumed['thread']['id']==thread_id
                events.clear()
                rpc(7,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':'Continue after the cancelled turn. '+task}]},deadline)
            def collect_turn():
                report['turnCompleted']=False;report['nativeReplyMatched']=False
                while time.monotonic()<deadline:
                    row=events.pop(0) if events else receive(deadline)
                    if row is None:break
                    item=row.get('params',{}).get('item',{})
                    if item.get('type')=='contextCompaction':
                        key='warmCompactionStarted' if row.get('method')=='item/started' else 'warmCompactionCompleted' if row.get('method')=='item/completed' else None
                        if key:report[key]=item.get('id')
                    if row.get('method')=='thread/tokenUsage/updated':
                        report['lastTokenUsage']=row.get('params',{}).get('tokenUsage',{}).get('last',{})
                        report.setdefault('observedContextEstimates',[]).append(report['lastTokenUsage'].get('totalTokens',0))
                    if row.get('method')=='item/completed' and row.get('params',{}).get('item',{}).get('type')=='commandExecution':
                        report['shellToolCompleted']|=row['params']['item'].get('status')=='completed' and row['params']['item'].get('exitCode')==0
                        report['successfulShellCalls']+=int(row['params']['item'].get('status')=='completed' and row['params']['item'].get('exitCode')==0)
                    if row.get('method')=='item/completed' and row.get('params',{}).get('item',{}).get('type')=='agentMessage':
                        report['nativeReplyMatched']|=row['params']['item'].get('text','').strip()==marker
                    if row.get('method')=='turn/completed':
                        turn=row.get('params',{}).get('turn',{});report['turnCompleted']=turn.get('status')=='completed'
                        report['lastTurnStatus']=turn.get('status')
                        error=turn.get('error')
                        if error:report['turnErrorCode']=error.get('codexErrorInfo','unclassified')
                        break
                    if 'id' in row and 'method' in row:
                        # Unexpected permission/tool requests are not auto-approved by this driver.
                        send({'id':row['id'],'error':{'code':-32601,'message':'Unexpected request in owned proof'}})
                        report['unexpectedServerRequest']=True
            if getattr(args,'interrupt_tool',False):
                pid_file=workspace/'command.pid'
                while time.monotonic()<deadline and not pid_file.exists():
                    row=receive(min(deadline,time.monotonic()+.1))
                    if row:
                        if row.get('method')=='turn/completed':raise RuntimeError('command_ended_before_interrupt')
                        events.append(row)
                assert pid_file.is_file(),'native_command_not_observed'
                pid=int(pid_file.read_text().strip());command_proc=Path('/proc')/str(pid)
                initial_marker_stat=(workspace/'proof.txt').stat()
                assert command_proc.resolve().parent==Path('/proc') and (command_proc/'cwd').resolve()==workspace
                assert marker.encode() in (command_proc/'cmdline').read_bytes(),'native_command_owner_mismatch'
                def command_identity():
                    try:
                        stat=(command_proc/'stat').read_text();fields=stat[stat.rfind(')')+2:].split()
                        report['nativeCommandState']=fields[0]
                        return None if fields[0] in ['Z','X'] else fields[19]
                    except FileNotFoundError:return None
                identity=command_identity();assert identity
                requests_before_stop=json.loads(proxy_proof.read_text())['requests']
                rpc(41,'turn/interrupt',{'threadId':thread_id,'turnId':started['turn']['id']},deadline)
                collect_turn()
                report['nativeToolStopAccepted']=report.get('lastTurnStatus')=='interrupted'
                for _ in range(50):
                    if command_identity()!=identity:break
                    time.sleep(.1)
                report['turnInterruptLeftCommandRunning']=command_identity()==identity
                if report['turnInterruptLeftCommandRunning']:
                    # Interrupting model work need not terminate upstream-owned
                    # background terminals. Use its explicit scoped control,
                    # never guess an OS process or kill unrelated jobs.
                    terminals=rpc(43,'thread/backgroundTerminals/list',{'threadId':thread_id,'limit':100},deadline)
                    owned=[t for t in terminals['data'] if t['cwd']==str(workspace) and marker in t['command']]
                    assert len(owned)==1,'native_terminal_owner_not_unique'
                    terminated=rpc(44,'thread/backgroundTerminals/terminate',{'threadId':thread_id,'processId':owned[0]['processId']},deadline)
                    report['explicitTerminalTermination']=terminated.get('terminated') is True
                    remaining=rpc(45,'thread/backgroundTerminals/list',{'threadId':thread_id,'limit':100},deadline)
                    report['ownedTerminalRemovedFromCore']=all(t['processId']!=owned[0]['processId'] for t in remaining['data'])
                    assert report['ownedTerminalRemovedFromCore'],'native_terminal_registry_not_settled'
                    for _ in range(50):
                        if command_identity()!=identity:break
                        time.sleep(.1)
                report['nativeCommandExitedAfterStop']=command_identity()!=identity
                report['stopWithoutProviderRequest']=json.loads(proxy_proof.read_text())['requests']==requests_before_stop
                assert all(report.get(k) for k in ['nativeToolStopAccepted','nativeCommandExitedAfterStop','stopWithoutProviderRequest']),'native_stop_not_qualified'
                events.clear()
                rpc(42,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':f'Say LOCAL_INTERMEDIATE_REPORT. Read the current proof.txt with one shell call. Preserve the file; do not rerun the interrupted command. Final answer exactly {marker}. No other work.'}]},deadline)
            collect_turn()
            marker_file=workspace/'proof.txt'
            if getattr(args,'reject_tool',False):
                rejected=json.loads(proxy_proof.read_text())
                report['rejectedSampleObserved']=report.get('lastTurnStatus')=='failed' and rejected.get('errorCode')=='unadvertised_tool'
                assert report['rejectedSampleObserved'] and report['successfulShellCalls']==1,'expected_rejected_sample_missing'
                before=marker_file.stat();requests=rejected['requests']
                time.sleep(.3)
                report['noAutomaticResubmit']=json.loads(proxy_proof.read_text())['requests']==requests
                assert report['noAutomaticResubmit'],'unexpected_automatic_resubmit'
                events.clear()
                rpc(61,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':f'Recover from the rejected tool. Say LOCAL_INTERMEDIATE_REPORT. Read the current proof.txt with one shell call, preserving the completed write. Final answer exactly {marker}. No other work.'}]},deadline)
                collect_turn();after=marker_file.stat()
                report['recoveryPreservedExistingFile']=all(getattr(before,k)==getattr(after,k) for k in ['st_ino','st_mtime_ns','st_size'])
            report['workspaceMarkerVerified']=marker_file.is_file() and marker_file.read_text().strip()==marker
            report['realCoreToolLoopAccepted']=not args.account_free_wire and report['turnCompleted'] and report['shellToolCompleted'] and report['workspaceMarkerVerified'] and report['successfulShellCalls']>=(1 if getattr(args,'interrupt_tool',False) else 2) and report['nativeReplyMatched']
            if getattr(args,'interrupt_tool',False):
                current=marker_file.stat()
                report['resumePreservedExistingFile']=all(getattr(initial_marker_stat,k)==getattr(current,k) for k in ['st_ino','st_mtime_ns','st_size'])
                report['realCoreToolLoopAccepted'] &= report['resumePreservedExistingFile']
            if getattr(args,'transcript_check',False):
                stored=rpc(81,'thread/read',{'threadId':thread_id,'includeTurns':True},deadline)
                items=[item for turn in stored['thread']['turns'] for item in turn['items']]
                def report_evidence(values):
                    return [{'phase':item.get('phase'),'initial':'LOCAL_INITIAL_REPORT' in item.get('text',''),
                        'intermediate':'LOCAL_INTERMEDIATE_REPORT' in item.get('text',''),
                        'final':item.get('text','').strip()==marker,'characters':len(item.get('text',''))}
                        for item in values if item.get('type')=='agentMessage']
                evidence=report_evidence(items)
                report['nativeReports']=evidence
                report['persistedReportCheck']=all(sum(item[k] for item in evidence)==1 for k in ['initial','intermediate','final'])
                report['persistedNativeCommands']=sum(item.get('type')=='commandExecution' and item.get('status')=='completed' for item in items)
                before_requests=json.loads(Path(str(args.desktop_server)+'.proof').read_text())['requests']
                os.killpg(process.pid,signal.SIGINT)
                try:process.wait(timeout=8)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=5)
                process.stdin.close();process.stdout.close();process.stderr.close()
                process=subprocess.Popen([str(launcher),'app-server','--listen','stdio://'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
                buffer='';events.clear()
                rpc(82,'initialize',{'clientInfo':{'name':args.app_name,'version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline)
                send({'method':'initialized','params':{}})
                resumed=rpc(83,'thread/resume',{'threadId':thread_id,'approvalPolicy':'on-request','sandbox':'danger-full-access'},deadline)
                cold=rpc(84,'thread/read',{'threadId':thread_id,'includeTurns':True},deadline)
                cold_items=[item for turn in cold['thread']['turns'] for item in turn['items']]
                report['coldTranscriptRestored']=resumed['thread']['id']==thread_id and report_evidence(cold_items)==evidence
                report['resumeWithoutModelRequest']=json.loads(Path(str(args.desktop_server)+'.proof').read_text())['requests']==before_requests
                os.killpg(process.pid,signal.SIGINT)
                try:process.wait(timeout=8)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=5)
                # The fixture has no operating shared-server installation. The
                # qualified launcher still provides the signed runtime's FDs.
                report['renderedTranscript']=render_transcript(launcher,workspace,env,thread_id,marker,getattr(args,'tui_trace_path',None))
                report['renderWithoutModelRequest']=json.loads(Path(str(args.desktop_server)+'.proof').read_text())['requests']==before_requests
            if args.cancel_resume:
                report['realCoreToolLoopAccepted'] &= all(report.get(k) for k in ['cancelAfterActualSubmit','nativeCancelAccepted','noToolBeforeCancel','desktopCancelSettled','nativeResumeAccepted','firstCoreStoppedBeforeResume'])
            if args.compact and report['realCoreToolLoopAccepted']:
                report['preCompactToolLoopAccepted']=True
                print(json.dumps({'nativeWarmupAccepted':True,'lastTokenUsage':report.get('lastTokenUsage',{})}),flush=True)
                for name in ['seed.txt','proof.txt']:
                    own=workspace/name;assert own.is_file() and not own.is_symlink()
                    assert own.read_text()==seed_text if name=='seed.txt' else own.read_text().strip()==marker
                    own.unlink()  # Only the two generated nonce fixtures, never user files.
                def persisted_compactions():
                    return sum(json.loads(line).get('type')=='compacted' for rollout in (home/'.codex/sessions').rglob('*.jsonl') for line in rollout.read_text().splitlines() if line.strip())
                def restart_for_probe(overrides=None):
                    nonlocal process,buffer
                    os.killpg(process.pid,signal.SIGINT)
                    try:process.wait(timeout=8)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=5)
                    process.stdin.close();process.stdout.close();process.stderr.close()
                    process=subprocess.Popen([str(launcher),'app-server','--listen','stdio://'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
                    buffer='';events.clear();rpc(20,'initialize',{'clientInfo':{'name':args.app_name,'version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline);send({'method':'initialized','params':{}})
                    resumed=rpc(21,'thread/resume',{'threadId':thread_id,'config':overrides or {},'approvalPolicy':'on-request','sandbox':'danger-full-access'},deadline)
                    assert resumed['thread']['id']==thread_id;events.clear()
                if args.interrupt_compact or args.failed_compact:
                    committed_before=persisted_compactions()
                    before_submit=json.loads(proxy_proof.read_text()).get('submittedRequests',0)
                    before_cancel=json.loads(proxy_proof.read_text()).get('cancelledRequests',0)
                    if args.failed_compact:
                        env['ACCESS_TOKEN']='owned-rejected-local-proof-token';restart_for_probe()
                    events.clear();rpc(22,'thread/compact/start',{'threadId':thread_id},deadline)
                    compact_turn_id=None;settled=None
                    if args.interrupt_compact:
                        while time.monotonic()<deadline:
                            row=receive(min(deadline,time.monotonic()+.1))
                            if row:
                                events.append(row)
                                if row.get('method')=='turn/started':compact_turn_id=row.get('params',{}).get('turn',{}).get('id')
                                if row.get('method')=='turn/completed':raise RuntimeError('compaction_finished_before_interrupt')
                            submitted=json.loads(proxy_proof.read_text()).get('submittedRequests',0)>before_submit
                            if submitted and compact_turn_id:break
                        assert submitted and compact_turn_id,'compact_submit_not_observed'
                        rpc(23,'turn/interrupt',{'threadId':thread_id,'turnId':compact_turn_id},deadline)
                        report['compactInterruptedAfterActualSubmit']=True
                    while time.monotonic()<deadline:
                        row=events.pop(0) if events else receive(deadline)
                        if not row:break
                        if row.get('method')=='turn/completed':settled=row.get('params',{}).get('turn',{}).get('status');break
                    expected='interrupted' if args.interrupt_compact else 'failed'
                    assert settled==expected,'compaction_fault_not_observed'
                    report['faultTurnStatus']=settled;report['faultCommittedNoSummary']=persisted_compactions()==committed_before
                    assert report['faultCommittedNoSummary'],'fault_committed_history'
                    if args.interrupt_compact:
                        for _ in range(150):
                            if json.loads(proxy_proof.read_text()).get('cancelledRequests',0)>before_cancel:break
                            time.sleep(.1)
                        report['interruptedDesktopSettled']=json.loads(proxy_proof.read_text()).get('cancelledRequests',0)>before_cancel
                        assert report['interruptedDesktopSettled'],'desktop_interrupt_not_settled'
                    else:env['ACCESS_TOKEN']=token
                    restart_for_probe();events.clear()
                    rpc(24,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':'Recall the remembered PROOF_SECRET from the completed tool outputs. Reply with exactly that value. Do not call tools.'}]},deadline);collect_turn()
                    report['faultHistorySurvivedRestart']=report['turnCompleted'] and report['nativeReplyMatched']
                    assert report['faultHistorySurvivedRestart'],'fault_history_lost'
                before_compact=initial_compaction_summaries if args.mid_turn_compact else json.loads(proxy_proof.read_text()).get('compactionSummaries',0)
                events.clear()
                if args.mid_turn_compact:
                    assert report.get('warmCompactionStarted') and report.get('warmCompactionCompleted')==report['warmCompactionStarted'],'mid_turn_compaction_not_observed'
                    report['midTurnAutomaticCompaction']=True;report['automaticCompactionRequestedManually']=False
                elif args.auto_compact:
                    warm_tokens=report.get('lastTokenUsage',{}).get('totalTokens',0);assert warm_tokens>0,'missing_context_estimate'
                    threshold=max(1000,warm_tokens-300);report['automaticThreshold']=threshold
                    restart_for_probe({'model_auto_compact_token_limit':threshold,'model_auto_compact_token_limit_scope':'total'})
                    events.clear();rpc(15,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':'Reply with exactly remembered PROOF_SECRET. Do not call tools or do other work.'}]},deadline)
                    report['automaticCompactionRequestedManually']=False
                else:rpc(8,'thread/compact/start',{'threadId':thread_id},deadline)
                start_id=report.get('warmCompactionStarted') if args.mid_turn_compact else None
                completed_id=report.get('warmCompactionCompleted') if args.mid_turn_compact else None
                while not args.mid_turn_compact and time.monotonic()<deadline:
                    row=events.pop(0) if events else receive(deadline)
                    if row is None:break
                    item=row.get('params',{}).get('item',{})
                    if item.get('type')=='contextCompaction':
                        if row.get('method')=='item/started':start_id=item.get('id')
                        if row.get('method')=='item/completed':completed_id=item.get('id')
                    if row.get('method')=='error':raise RuntimeError('native_compaction_error')
                    if row.get('method')=='turn/completed':
                        if args.auto_compact:report['automaticTurnCompleted']=row.get('params',{}).get('turn',{}).get('status')=='completed'
                        break
                report['nativeCompactionCompleted']=bool(start_id and completed_id==start_id)
                print(json.dumps({'nativeCompactionCompleted':report['nativeCompactionCompleted'],'automatic':args.auto_compact or args.mid_turn_compact}),flush=True)
                report['actualChatgptCompactionSummary']=json.loads(proxy_proof.read_text()).get('compactionSummaries',0)>before_compact
                summaries=[]
                for rollout in (home/'.codex/sessions').rglob('*.jsonl'):
                    for line in rollout.read_text().splitlines():
                        try:value=json.loads(line)
                        except ValueError:continue
                        if value.get('type')=='compacted':summaries.append(value.get('payload',{}).get('message',''))
                report['persistedSummaryContainsToolOnlySecret']=any(marker in text for text in summaries)
                if not all(report.get(k) for k in ['nativeCompactionCompleted','actualChatgptCompactionSummary','persistedSummaryContainsToolOnlySecret']):raise RuntimeError('native_checkpoint_not_accepted')
                calls_before=report['successfulShellCalls'];events.clear()
                task_after='Recall PROOF_SECRET from the previous tool result/checkpoint; its original files are gone. Make one native shell call to write that value to continuity.txt, then a separate second native shell call to read continuity.txt. Reply with exactly PROOF_SECRET. Do not reread old files or perform other work.'
                rpc(9,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':task_after}]},deadline);collect_turn()
                continuity=workspace/'continuity.txt'
                report['postCompactNativeToolLoopAccepted']=report['turnCompleted'] and report['nativeReplyMatched'] and report['successfulShellCalls']-calls_before>=2 and continuity.is_file() and continuity.read_text().strip()==marker
                report['bridgeAppliedCheckpoint']=json.loads(proxy_proof.read_text()).get('appliedCheckpoints',0)>0
                if not report['postCompactNativeToolLoopAccepted']:raise RuntimeError('post_compact_tool_loop_failed')
                os.killpg(process.pid,signal.SIGINT)
                try:process.wait(timeout=8)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=5)
                process.stdin.close();process.stdout.close();process.stderr.close()
                process=subprocess.Popen([str(launcher),'app-server','--listen','stdio://'],env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
                buffer='';events.clear();rpc(10,'initialize',{'clientInfo':{'name':args.app_name,'title':'Codex Termux Trial','version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline);send({'method':'initialized','params':{}})
                resumed=rpc(11,'thread/resume',{'threadId':thread_id,'approvalPolicy':'on-request','sandbox':'danger-full-access'},deadline)
                report['compactedThreadResumed']=resumed['thread']['id']==thread_id
                if args.restart_desktop:
                    from desktop_lifecycle_probe import stop_owned
                    desktop_session=real_home/'.local/share/codex-web-experiment/linux-desktop/session.json'
                    old_desktop=stop_owned(desktop_session)
                    report['desktopStoppedBeforeNativeResumeTurn']=True
                calls_before=report['successfulShellCalls'];events.clear()
                rpc(12,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':'Reply with exactly the remembered PROOF_SECRET. Do not call tools or do other work.'}]},deadline);collect_turn()
                report['checkpointSurvivedCoreRestart']=report['turnCompleted'] and report['nativeReplyMatched'] and report['successfulShellCalls']==calls_before
                if args.restart_desktop:
                    new_desktop=json.loads(desktop_session.read_text())
                    report['checkpointSurvivedDesktopRestart']=report['checkpointSurvivedCoreRestart'] and new_desktop['pid']!=old_desktop['pid'] and new_desktop['root']!=old_desktop['root']
                report['realCoreToolLoopAccepted']=all(report.get(k) for k in ['preCompactToolLoopAccepted','nativeCompactionCompleted','actualChatgptCompactionSummary','persistedSummaryContainsToolOnlySecret','postCompactNativeToolLoopAccepted','bridgeAppliedCheckpoint','compactedThreadResumed','checkpointSurvivedCoreRestart'])
                if args.restart_desktop:report['realCoreToolLoopAccepted'] &= report.get('checkpointSurvivedDesktopRestart',False)
                if args.auto_compact:report['realCoreToolLoopAccepted'] &= report.get('automaticTurnCompleted',False)
                if args.mid_turn_compact:report['realCoreToolLoopAccepted'] &= report.get('midTurnAutomaticCompaction',False)
                if args.interrupt_compact or args.failed_compact:report['realCoreToolLoopAccepted'] &= all(report.get(k) for k in ['faultCommittedNoSummary','faultHistorySurvivedRestart'])
            if args.account_free_wire:
                report.update({'wireFailures':wire_failures,'correlatedToolOutputs':len(transcript.outputs),'accountFreeWireAccepted':report['turnCompleted'] and report['shellToolCompleted'] and len(transcript.outputs)==2 and not wire_failures})
        except Exception as error:
            error.probe_report = report.copy()
            raise
        finally:
            if process.poll() is None:
                process.stdin.close()
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid,signal.SIGTERM)
                    try:process.wait(timeout=5)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=5)
            report['ownedCoreExited']=process.poll() is not None
            if backend:backend.shutdown();backend.server_close()
    return report


if __name__=='__main__':
    os.umask(0o077)
    parser=argparse.ArgumentParser();parser.add_argument('--connection',type=Path);parser.add_argument('--model',required=True);parser.add_argument('--report',type=Path,required=True);parser.add_argument('--app-name',required=True);parser.add_argument('--account-free-wire',action='store_true');parser.add_argument('--desktop-server',type=Path);parser.add_argument('--cancel-resume',action='store_true');parser.add_argument('--compact',action='store_true');parser.add_argument('--auto-compact',action='store_true');parser.add_argument('--interrupt-compact',action='store_true');parser.add_argument('--failed-compact',action='store_true');parser.add_argument('--mid-turn-compact',action='store_true');parser.add_argument('--restart-desktop',action='store_true')
    parser.add_argument('--effort',choices=['none','medium','high','xhigh'])
    args=parser.parse_args();args.compact=args.compact or args.auto_compact or args.interrupt_compact or args.failed_compact or args.mid_turn_compact
    assert sum(bool(x) for x in (args.account_free_wire,args.connection,args.desktop_server))==1
    assert not(args.cancel_resume or args.compact) or args.desktop_server
    assert not args.restart_desktop or (args.desktop_server and args.compact)
    assert not(args.cancel_resume and args.compact)
    assert sum(bool(x) for x in [args.auto_compact,args.interrupt_compact,args.failed_compact,args.mid_turn_compact])<=1
    signal.signal(signal.SIGTERM,lambda *_:(_ for _ in ()).throw(KeyboardInterrupt()))
    try:report=probe(args)
    except Exception as error:
        import re
        report=getattr(error,'probe_report',{})
        report.update({'realCoreToolLoopAccepted':False,'failureClass':type(error).__name__,'failureCode':str(error) if re.fullmatch('[a-z_]+',str(error)) else 'unclassified'})
        if isinstance(error,OSError):report.update({'failureErrno':error.errno,'failureFileName':Path(error.filename).name if error.filename else None})
        import traceback
        report['failureLocation']=[{'file':Path(f.filename).name,'line':f.lineno,'function':f.name} for f in traceback.extract_tb(error.__traceback__)][-5:]
    if args.report.exists() or args.report.is_symlink():
        state=args.report.lstat()
        assert args.report.is_file() and not args.report.is_symlink() and state.st_uid==os.getuid() and not state.st_mode&0o077
    fd, temporary=tempfile.mkstemp(dir=args.report.parent,prefix='.core-proof-')
    try:
        with os.fdopen(fd,'w') as f:f.write(json.dumps(report,indent=2)+'\n');f.flush();os.fsync(f.fileno())
        os.replace(temporary,args.report)
    finally:
        if Path(temporary).exists():Path(temporary).unlink()
    print(json.dumps(report));raise SystemExit(0 if report['realCoreToolLoopAccepted'] or report.get('accountFreeWireAccepted') else 1)
