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
from protocol import Transcript, events as response_events

SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE.parents[1]/'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest


def sha(path):
    with path.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()


def probe(args):
    real_home=Path.home();host_prefix=Path(os.environ['PREFIX'])
    if args.account_free_wire:token='owned-synthetic-wire-token'
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
    report={'realService':not args.account_free_wire,'nativeCoreGeneration':fields['generation_id'],'model':args.model,'realCoreToolLoopAccepted':False,'turnCompleted':False,'shellToolCompleted':False,'workspaceMarkerVerified':False}
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
        base_url='https://api.openai.com/v1'
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
model_reasoning_effort = "low"
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
            deadline=time.monotonic()+180
            rpc(1,'initialize',{'clientInfo':{'name':args.app_name,'title':'Codex Termux Trial','version':'0.1.0'},'capabilities':{'experimentalApi':True}},deadline)
            send({'method':'initialized','params':{}})
            thread=rpc(2,'thread/start',{'model':args.model,'cwd':str(workspace),'approvalPolicy':'on-request','sandbox':'danger-full-access','ephemeral':True},deadline)
            thread_id=thread['thread']['id']
            task=f'Use the local shell tool to write exactly {marker} to proof.txt in the current directory. Use the shell tool to read it back. Reply with exactly {marker}. No other work.'
            rpc(3,'turn/start',{'threadId':thread_id,'input':[{'type':'text','text':task}]},deadline)
            while time.monotonic()<deadline:
                row=events.pop(0) if events else receive(deadline)
                if row is None:break
                if row.get('method')=='item/completed' and row.get('params',{}).get('item',{}).get('type')=='commandExecution':
                    report['shellToolCompleted']|=row['params']['item'].get('status')=='completed' and row['params']['item'].get('exitCode')==0
                if row.get('method')=='turn/completed':
                    turn=row.get('params',{}).get('turn',{});report['turnCompleted']=turn.get('status')=='completed'
                    error=turn.get('error')
                    if error:report['turnErrorCode']=error.get('codexErrorInfo','unclassified')
                    break
                if 'id' in row and 'method' in row:
                    # Unexpected permission/tool requests are not auto-approved by this driver.
                    send({'id':row['id'],'error':{'code':-32601,'message':'Unexpected request in owned proof'}})
                    report['unexpectedServerRequest']=True
            marker_file=workspace/'proof.txt'
            report['workspaceMarkerVerified']=marker_file.is_file() and marker_file.read_text().strip()==marker
            report['realCoreToolLoopAccepted']=not args.account_free_wire and report['turnCompleted'] and report['shellToolCompleted'] and report['workspaceMarkerVerified']
            if args.account_free_wire:
                report.update({'wireFailures':wire_failures,'correlatedToolOutputs':len(transcript.outputs),'accountFreeWireAccepted':report['turnCompleted'] and report['shellToolCompleted'] and len(transcript.outputs)==2 and not wire_failures})
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
    parser=argparse.ArgumentParser();parser.add_argument('--connection',type=Path);parser.add_argument('--model',required=True);parser.add_argument('--report',type=Path,required=True);parser.add_argument('--app-name',required=True);parser.add_argument('--account-free-wire',action='store_true')
    args=parser.parse_args()
    assert args.account_free_wire or args.connection
    signal.signal(signal.SIGTERM,lambda *_:(_ for _ in ()).throw(KeyboardInterrupt()))
    try:report=probe(args)
    except Exception as error:report={'realCoreToolLoopAccepted':False,'failureClass':type(error).__name__}
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
