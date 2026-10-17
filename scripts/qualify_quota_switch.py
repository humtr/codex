"""Actual Android quota handoff with synthetic accounts and a loopback model only."""
import argparse
import base64
import datetime
import fcntl
import http.server
import json
import os
from pathlib import Path
import pty
import re
import shlex
import signal
import struct
import subprocess
import tempfile
import termios
import threading
import time

from qualify_permission_lifecycle import fixture, server_paths, rpc, schema_value


def token(account):
    claims={'exp':int(time.time())+86400,'email':'fixture@example.invalid',
            'https://api.openai.com/auth':{'chatgpt_plan_type':'plus','chatgpt_account_id':account,'chatgpt_user_id':'owned-fixture'}}
    encode=lambda value:base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b'=').decode()
    return encode({'alg':'none'})+'.'+encode(claims)+'.owned-fixture'


def probe(args):
    counts={'sourceSamples':0,'destinationSamples':0,'usageReads':0,'guardian':0}
    failures=[];requests=[];state={'exhausted':False}
    with tempfile.TemporaryDirectory(prefix='qn-',dir=os.environ['TMPDIR']) as temporary:
        root=Path(temporary)
        home,prefix,env=fixture(root,args.core,args.frontend,args.manager,None)
        env.pop('OPENAI_API_KEY',None)
        work=root/'w';command=work/'owned-once';counter=work/'effects'
        command.write_text('#!'+os.environ['PREFIX']+'/bin/sh\n'
            'printf "effect\\n" >> '+shlex.quote(str(counter))+'\nprintf "QUOTA_ONCE:1\\n"\n')
        command.chmod(0o700)

        class Backend(http.server.BaseHTTPRequestHandler):
            def log_message(self,*_):pass
            def json(self,status,value):
                data=json.dumps(value).encode();self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
            def do_GET(self):
                account=self.headers.get('chatgpt-account-id','')
                requests.append({'method':'GET','path':self.path,'account':account})
                if self.path.endswith('/accounts/check'):
                    self.json(200,{'accounts':[{'id':account,'plan_type':'plus','workspace_backend_origin':'https://chatgpt.com','account_routing_override':'NO_CONSTRAINT'}]})
                elif self.path.endswith('/usage'):
                    counts['usageReads']+=1;blocked=account=='fixture-source' and state['exhausted']
                    def window(seconds,used):return {'used_percent':used,'limit_window_seconds':seconds,'reset_after_seconds':seconds,'reset_at':int(time.time())+seconds}
                    self.json(200,{'plan_type':'plus','account_id':account,'user_id':'owned-fixture','rate_limit':{'allowed':not blocked,'limit_reached':blocked,'primary_window':window(18000,100 if blocked else 20),'secondary_window':window(604800,10)}})
                elif '/models' in self.path:
                    self.json(200,json.loads((home/'.codex/native-models.json').read_text()))
                else:self.json(200,{})
            def do_POST(self):
                try:
                    payload=self.rfile.read(int(self.headers['Content-Length']))
                    if self.headers.get('Content-Encoding')=='zstd':
                        payload=subprocess.run(['zstd','-d','-q','-c'],input=payload,capture_output=True,check=True).stdout
                    body=json.loads(payload)
                    account=self.headers.get('chatgpt-account-id','')
                    requests.append({'method':'POST','path':self.path,'account':account})
                    if not self.path.endswith('/responses'):
                        self.json(200,{});return
                    fmt=(body.get('text') or {}).get('format') or {}
                    if fmt.get('type')=='json_schema':
                        if 'outcome' in fmt.get('schema',{}).get('properties',{}):
                            counts['guardian']+=1;text=json.dumps({'risk_level':'low','user_authorization':'high','outcome':'allow','rationale':'Only the owned disposable fixture command is authorized.'})
                        else:text=json.dumps(schema_value(fmt['schema']))
                        item={'id':'aux','type':'message','role':'assistant','content':[{'type':'output_text','text':text,'annotations':[]}]}
                    else:
                        assert account in ('fixture-source','fixture-destination'), 'unexpected synthetic auth route'
                        outputs=[entry for entry in body.get('input',[]) if entry.get('type')=='function_call_output']
                        has_result=any('QUOTA_ONCE:1' in json.dumps(entry) for entry in outputs)
                        if account=='fixture-source':
                            counts['sourceSamples']+=1
                            if has_result:
                                state['exhausted']=True
                                self.json(429,{'error':{'type':'usage_limit_reached','resets_at':int(time.time())+18000,'limit_window_minutes':300}});return
                            tools=list(body.get('tools',[]))
                            for entry in body.get('input',[]):
                                if entry.get('type')=='additional_tools':tools.extend(entry.get('tools',[]))
                            tool=next(entry for entry in tools if entry.get('name') in ('exec_command','functions'))
                            arguments={'cmd':str(command),'login':False,'yield_time_ms':1000,'max_output_tokens':100}
                            item={'id':'once-tool','type':'function_call','call_id':'owned-once','name':'exec_command','arguments':json.dumps(arguments)}
                            if tool.get('type')=='namespace':item['namespace']=tool['name']
                        else:
                            counts['destinationSamples']+=1
                            assert has_result,'saved completed tool result missing after account handoff'
                            users=[entry for entry in body.get('input',[]) if entry.get('role')=='user']
                            assert 'checkpoint' in json.dumps(users[-1:]), 'checkpoint continuation missing'
                            assert 'QUOTA_REQUEST' not in json.dumps(users[-1:]), 'original prompt resubmitted'
                            item={'id':'finish','type':'message','role':'assistant','content':[{'type':'output_text','text':'QUOTA_SWITCH_COMPLETE','annotations':[]}]}
                    response={'id':'owned-response','object':'response','model':body['model'],'status':'completed','output':[item],'usage':{'input_tokens':10,'output_tokens':10,'total_tokens':20}}
                    self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
                    events=[('response.created',{'type':'response.created','response':{**response,'status':'in_progress','output':[]}}),('response.output_item.added',{'type':'response.output_item.added','output_index':0,'item':item}),('response.output_item.done',{'type':'response.output_item.done','output_index':0,'item':item}),('response.completed',{'type':'response.completed','response':response})]
                    if item['type']=='message':events.insert(2,('response.output_text.delta',{'type':'response.output_text.delta','item_id':item['id'],'output_index':0,'content_index':0,'delta':item['content'][0]['text']}))
                    for name,value in events:self.wfile.write(('event: '+name+'\ndata: '+json.dumps(value)+'\n\n').encode())
                    self.wfile.flush()
                except Exception as error:
                    failures.append(type(error).__name__+': '+str(error));self.close_connection=True

        server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Backend)
        threading.Thread(target=server.serve_forever,daemon=True).start()
        address=f'http://127.0.0.1:{server.server_port}'
        env['OPENAI_BASE_URL']=address+'/v1'
        catalog=home/'.codex/native-models.json'
        value=json.loads(catalog.read_text())
        for model in value['models']:model.update(multi_agent_version='v1',context_window=262144,max_context_window=262144,auto_compact_token_limit=200000)
        catalog.write_text(json.dumps(value))
        config=('model = "gpt-6.1-sol"\nmodel_reasoning_effort = "high"\napproval_policy = "never"\napprovals_reviewer = "user"\nsandbox_mode = "danger-full-access"\ncheck_for_update_on_startup = false\n'
            +'openai_base_url = '+json.dumps(address+'/v1')+'\nchatgpt_base_url = '+json.dumps(address) +'\nmodel_catalog_json = '+json.dumps(str(catalog))+'\n'
            +'[tui]\nscreen_reader_detection_done = true\n[features]\nmemories = false\nplugins = false\n'
            +'[projects.'+json.dumps(str(work))+']\ntrust_level = "trusted"\n')
        core=prefix/'bin/codex'
        def run(*arguments):
            out=subprocess.run([str(core),*arguments],env=env,cwd=work,capture_output=True,text=True,timeout=35)
            assert out.returncode==0,(arguments,out.stderr);return out.stdout
        run('termux','profile','create','next')
        destination=home/'.local/share/codex/manager/profiles/next/home'
        for directory,account in [(home/'.codex','fixture-source'),(destination,'fixture-destination')]:
            (directory/'config.toml').write_text(config);(directory/'config.toml').chmod(0o600)
            jwt=token(account)
            (directory/'auth.json').write_text(json.dumps({'auth_mode':'chatgpt','OPENAI_API_KEY':None,'tokens':{'id_token':jwt,'access_token':jwt,'refresh_token':'owned-fixture','account_id':account},'last_refresh':datetime.datetime.now(datetime.timezone.utc).isoformat()}));(directory/'auth.json').chmod(0o600)
        process=None;master=None;screen=''
        try:
            view=json.loads(run('termux','usage','--refresh','--json','--model','gpt-6.1-sol'))
            assert len(view['rows'])==2,view
            if not all(row['state']=='ready' for row in view['rows']):
                diagnostic={}
                import select
                for name,directory in [('target',destination)]:
                    selected=dict(env,CODEX_HOME=str(directory))
                    log=root/'native-stdio.log'
                    with log.open('wb') as stderr:
                        child=subprocess.Popen([str(core),'app-server'],env=selected,cwd=directory,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=stderr,bufsize=0)
                        buffer=b'';messages=[]
                        try:
                            for index,(method,params) in enumerate([('initialize',{'clientInfo':{'name':'fixture','version':'1'},'capabilities':{'experimentalApi':True}}),('account/read',{'refreshToken':False}),('account/rateLimits/read',{'supportsLunaReserve':False,'excludeResetCreditDetails':True}),('model/list',{'limit':100})],1):
                                child.stdin.write((json.dumps({'id':index,'method':method,'params':params})+'\n').encode())
                                deadline=time.monotonic()+8
                                while time.monotonic()<deadline:
                                    if b'\n' not in buffer:
                                        if not select.select([child.stdout],[],[],max(.01,deadline-time.monotonic()))[0]:break
                                        chunk=os.read(child.stdout.fileno(),262144)
                                        if not chunk:break
                                        buffer+=chunk;continue
                                    line,buffer=buffer.split(b'\n',1);value=json.loads(line)
                                    if value.get('id')==index:messages.append({'method':method,'response':value});break
                                if index==1:child.stdin.write(b'{"method":"initialized"}\n')
                        finally:child.kill();child.wait()
                    diagnostic[name]={'responses':messages,'stderr':log.read_text()[-5000:]}
                raise AssertionError({'view':view,'requests':requests,'diagnostic':diagnostic})
            run('termux','rotation','set',json.dumps({'enabled':True,'accounts':['default','next'],'cooldown':30}))
            if args.prepare_only:
                return {'accepted':True,'accountReads':counts['usageReads'],'rowsReady':2,'realModelRequests':0,'productionChanged':False}
            master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,130,0,0))
            process=subprocess.Popen([str(core),'--no-alt-screen'],env=env,cwd=work,stdin=slave,stdout=slave,stderr=slave,start_new_session=True);os.close(slave);os.set_blocking(master,False)
            def drain():
                nonlocal screen
                import select
                if select.select([master],[],[],.05)[0]:
                    try:screen+=os.read(master,262144).decode(errors='replace')
                    except (BlockingIOError,OSError):pass
            def wait(predicate,seconds=50):
                deadline=time.monotonic()+seconds
                while time.monotonic()<deadline:
                    drain()
                    assert not failures,failures
                    if predicate():return
                    if process.poll() is not None:break
                clean=re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]','',screen)
                raise AssertionError('native quota trial timeout '+json.dumps(counts)+' requests='+json.dumps(requests[-12:])+': '+clean[-2600:])
            def loaded():
                result=[]
                from qualify_tasks import Ws
                for path in server_paths(home):
                    ws=Ws(str(path.resolve()),uri_path='/rpc')
                    try:result.extend(ws.call('thread/loaded/list',{'limit':100})['data'])
                    finally:ws.socket.close()
                return result
            wait(lambda:bool(server_paths(home)))
            wait(lambda:len(loaded())==1)
            thread=loaded()[0]
            if args.ui_proof:
                os.write(master,b'/switch');time.sleep(.3);os.write(master,b'\r')
                wait(lambda:'Profiles' in screen and 'Automatic switching' in screen)
                wait(lambda:'weekly' in screen and '5h' in screen)
                end=time.monotonic()+1
                while time.monotonic()<end:drain()
                os.write(master,b'\x1b[B'*3+b'\r')
                wait(lambda:'Switch settings' in screen)
                os.write(master,b'\r')
                wait(lambda:json.loads(run('termux','rotation','show'))['enabled']==False)
                os.write(master,b'\r')
                wait(lambda:json.loads(run('termux','rotation','show'))['enabled']==True)
                os.write(master,b'\x1b')
                end=time.monotonic()+.5
                while time.monotonic()<end:drain()
            rpc(home,'turn/start',{'threadId':thread,'input':[{'type':'text','text':'QUOTA_REQUEST: execute the owned command once, then finish.','text_elements':[]}]})
            if args.source_only:
                wait(lambda:state['exhausted'],60)
                assert counter.read_text().splitlines()==['effect']
                return {'accepted':True,'nativeCommandEffects':1,'terminalQuotaResponse':True,'counts':counts,'realModelRequests':0,'productionChanged':False}
            wait(lambda:'QUOTA_SWITCH_COMPLETE' in screen,90)
            if args.ui_proof:
                wait(lambda:'Account switched: default -> next' in screen)
            assert counter.read_text().splitlines()==['effect'],'completed command ran more than once'
            assert counts['sourceSamples']==2 and counts['destinationSamples']==1,counts
            assert thread in loaded(),'conversation UUID changed'
            attempts=list((home/'.cache/codex/manager/usage/handoffs'/thread).glob('attempts.json'))
            assert len(attempts)==1 and len(json.loads(attempts[0].read_text()))==1
            native=rpc(home,'thread/read',{'threadId':thread,'includeTurns':False})['thread']
            rollout=Path(native['path']);events=[json.loads(line) for line in rollout.read_text().splitlines()]
            prompts=[json.dumps(entry['payload'].get('content',[])) for entry in events if entry.get('type')=='response_item' and entry.get('payload',{}).get('role')=='user']
            assert sum('QUOTA_REQUEST' in p for p in prompts)==1,{'prompts':prompts,'eventKinds':sorted({str(e.get('type')) for e in events})}
            assert sum('checkpoint' in p for p in prompts)==1,prompts
            settings=rpc(home,'thread/resume',{'threadId':thread})
            assert settings['sandbox']['type']=='dangerFullAccess' and settings['approvalPolicy']=='never' and settings['approvalsReviewer']=='user',settings
            assert not failures,failures
            return {'accepted':True,'nativeCommandEffects':1,'automaticHandoffs':1,'sameThread':True,'checkpointPrompts':1,'originalPrompts':1,'visibleFinal':True,'visibleSwitchNotice':args.ui_proof,'nativeMenuAndToggle':args.ui_proof,'settingsPreserved':True,'counts':counts,'realModelRequests':0,'productionChanged':False}
        finally:
            if process and process.poll() is None:
                os.killpg(process.pid,signal.SIGTERM)
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
            if master is not None:os.close(master)
            for path in server_paths(home):
                try:
                    pid=int(path.with_name('pid').read_text())
                    if Path(f'/proc/{pid}/exe').resolve().is_relative_to(home):os.kill(pid,signal.SIGTERM)
                except (OSError,ValueError):pass
            server.shutdown();server.server_close()


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['core','manager','frontend','report']:parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--prepare-only',action='store_true')
    parser.add_argument('--source-only',action='store_true')
    parser.add_argument('--ui-proof',action='store_true')
    args=parser.parse_args()
    try:result=probe(args)
    except Exception as error:
        import traceback
        result={'accepted':False,'error':type(error).__name__+': '+str(error),'trace':traceback.format_exc(),'realModelRequests':0,'productionChanged':False}
    args.report.write_text(json.dumps(result,indent=2)+'\n');args.report.chmod(0o600)
    print(json.dumps(result));raise SystemExit(0 if result['accepted'] else 1)
