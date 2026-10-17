"""Frozen output owner, continued native tools and a visible reconnect; zero Chat."""
import argparse
import http.server
import json
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import tempfile
import threading
import time

from siwc_core_probe import CORE_PROOF_SOURCE,render_transcript
sys.path.insert(0,str(CORE_PROOF_SOURCE/'scripts'))
from qualify_permission_lifecycle import fixture,server_paths
from qualify_tasks import Ws
from protocol import inventory,declared_tools


def run(output,frontend,disconnect=False):
    output.mkdir(mode=0o700,parents=True,exist_ok=False)
    state=dict(x.split('=',1) for x in (Path.home()/'.local/share/codex/core/activation-state').read_text().splitlines() if '=' in x)
    generation=Path.home()/'.local/lib/codex/core/generations'/state['current']
    assert frontend.is_file(),'qualified_unadapted_frontend_required'
    stages=[];second=threading.Event();final=threading.Event();release=threading.Event();finish=threading.Event()
    errors=[]
    class Model(http.server.BaseHTTPRequestHandler):
        def log_message(self,*_):pass
        def do_POST(self):
            try:
                body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                index=len(stages);stages.append(index)
                if index==1:
                    second.set();assert release.wait(20),'owner_not_frozen'
                if index==2:
                    final.set();assert finish.wait(25),'progress_not_rendered_before_completion'
                assert index<3,'unexpected_inference_after_reconnect'
                phase='commentary' if index<2 else 'final_answer'
                text=['LOCAL_INITIAL_REPORT','LOCAL_INTERMEDIATE_REPORT',marker][index]
                items=[{'id':f'msg_owned_{index}','type':'message','role':'assistant','phase':phase,
                    'content':[{'type':'output_text','text':text,'annotations':[]}]}]
                if index<2:
                    routes=inventory(declared_tools(body));choices=[key for key in routes if key[1]=='exec_command'];assert len(choices)==1
                    namespace,name=choices[0]
                    script='from pathlib import Path;Path('+repr('first.txt' if index==0 else 'proof.txt')+').write_text('+repr(marker+'\n')+')'
                    command=shlex.join([sys.executable,'-c',script])
                    items.append({'id':f'fc_owned_{index}','type':'function_call','call_id':f'call_owned_{index}',
                        'name':name,**({'namespace':namespace} if namespace else {}),'arguments':json.dumps({'cmd':command,'login':False,'yield_time_ms':1000,'max_output_tokens':100})})
                response={'id':f'resp_owned_{index}','object':'response','model':body['model'],'status':'completed','output':items,
                    'usage':{'input_tokens':100,'output_tokens':30,'total_tokens':130}}
                events=[('response.created',{'response':{**response,'status':'in_progress','output':[]}})]
                for position,item in enumerate(items):
                    events.append(('response.output_item.added',{'output_index':position,'item':item}))
                    if item['type']=='message':events.append(('response.output_text.delta',{'item_id':item['id'],'output_index':position,'content_index':0,'delta':text}))
                    events.append(('response.output_item.done',{'output_index':position,'item':item}))
                events.append(('response.completed',{'response':response}))
                self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
                for name,value in events:self.wfile.write(('event: '+name+'\ndata: '+json.dumps({'type':name,**value})+'\n\n').encode())
                self.wfile.flush()
            except Exception as error:errors.append(type(error).__name__)
    model=http.server.ThreadingHTTPServer(('127.0.0.1',0),Model)
    threading.Thread(target=model.serve_forever,daemon=True).start()
    owner=observer=None;result={}
    try:
        with tempfile.TemporaryDirectory(prefix='observer-',dir=os.environ['TMPDIR']) as temporary:
            root=Path(temporary);marker=os.urandom(16).hex()
            home,prefix,env=fixture(root,generation/'core',frontend)
            work=root/'w'
            catalog=json.loads((Path.home()/'.local/share/codex-web-experiment/native-provider/models.json').read_text())
            chat=next(m for m in catalog['models'] if m['slug']=='chatgpt-desktop/gpt-6')
            env['OPENAI_BASE_URL']=f'http://127.0.0.1:{model.server_port}/v1'
            try:
                started=subprocess.run([str(prefix/'bin/codex'),'__app-server-catalog-v1'],input=json.dumps({'models':[chat]}),
                    env=env,cwd=work,capture_output=True,text=True,timeout=30)
                assert started.returncode==0,'owned_server_start_failed'
                socket=Path(started.stdout.strip());owner=Ws(str(socket),uri_path='/rpc')
                thread=owner.call('thread/start',{'model':chat['slug'],'modelProvider':'owned','cwd':str(work),'ephemeral':False,
                    'approvalPolicy':'never','sandbox':'danger-full-access','config':{'model_providers.owned':{
                        'name':'Owned deterministic observer','base_url':env['OPENAI_BASE_URL'],'wire_api':'responses','requires_openai_auth':False}}})['thread']['id']
                turn=owner.call('turn/start',{'threadId':thread,'input':[{'type':'text','text':'Owned progress observation trial.'}]})['turn']['id']
                assert second.wait(15) and (work/'first.txt').read_text()==marker+'\n','first_native_step_missing'
                # Leave the output owner connected but deliberately stop reading.
                # This models a stalled display, distinct from cancellation/exit.
                if disconnect:owner.close();owner=None
                release.set();assert final.wait(15),'native_work_stopped_with_frozen_owner'
                assert (work/'proof.txt').read_text()==marker+'\n','later_native_tool_not_executed'
                before=(work/'proof.txt').stat();observer=Ws(str(socket),uri_path='/rpc')
                live=observer.call('thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns'][-1]
                assert live['id']==turn and live['status']=='inProgress'
                visible=[];settled=[]
                def on_visible(state):
                    if state.get('intermediate') and not state.get('final'):
                        visible.append(True);finish.set()
                    if state.get('final') and not settled:
                        deadline=time.monotonic()+5
                        while True:
                            current=observer.call('thread/read',{'threadId':thread,'includeTurns':True})['thread']['turns'][-1]
                            if current['status']=='completed':settled.append(current);break
                            assert time.monotonic()<deadline,'turn_completion_not_observed';time.sleep(.05)
                rendered=render_transcript(prefix/'bin/codex',work,env,thread,marker,output/'tui.txt',on_visible,
                    remote_url='unix://'+str(socket))
                assert all(rendered.get(k) for k in ['initial','intermediate','final']) and visible,'native_progress_not_visible'
                assert settled,'turn_completion_not_observed'
                completed=settled[0]
                after=(work/'proof.txt').stat()
                result={'accepted':True,'actualChatSubmissions':0,'syntheticModelStarts':len(stages),'realNativeCommands':2,
                    'frozenOutputOwnerStillConnected':not disconnect,'outputOwnerDisconnected':disconnect,'laterToolExecutedWithoutOwnerReads':True,
                    'sameActiveTurnObserved':completed['id']==turn,'visibleProgressBeforeFinalReleased':bool(visible),
                    'renderedTranscript':rendered,'resumeDidNotStartAnotherModelRequest':len(stages)==3,
                    'completedFileIdentityPreserved':all(getattr(before,k)==getattr(after,k) for k in ['st_ino','st_mtime_ns','st_size']),
                    'scope':'Real Android Core/tools/remote TUI with deterministic local model; not physical UI or real Chat failure injection'}
                assert all(result[k] for k in ['sameActiveTurnObserved','resumeDidNotStartAnotherModelRequest','completedFileIdentityPreserved']) and not errors
            finally:
                release.set();finish.set()
                for client in [owner,observer]:
                    if client:client.close()
                for path in server_paths(home):
                    pid=int(path.with_name('pid').read_text());os.kill(pid,signal.SIGTERM)
                    deadline=time.monotonic()+5
                    while path in server_paths(home) and time.monotonic()<deadline:time.sleep(.05)
                    assert path not in server_paths(home),'owned_server_not_stopped'
    finally:model.shutdown();model.server_close()
    result['temporaryRootRemoved']=not root.exists()
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result));return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--frontend',type=Path,required=True)
    parser.add_argument('--disconnect-output-owner',action='store_true')
    args=parser.parse_args();run(args.output,args.frontend,args.disconnect_output_owner)
