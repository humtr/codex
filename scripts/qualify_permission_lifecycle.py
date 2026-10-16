"""Owned Android execution proof; deterministic local model, no real account access."""
import argparse
import ast
import fcntl
import http.server
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE / '.github/scripts'))
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest
from qualify_tasks import Ws, fixture_activation_state



def sha(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def fixture(root, core, frontend, manager=None, generation=None):
    if generation is None:
        real_home = Path.home()
        activation = dict(line.split('=', 1) for line in
            (real_home / '.local/share/codex/core/activation-state').read_text().splitlines() if '=' in line)
        installed = real_home / '.local/lib/codex/core/generations' / activation['current']
    else:
        installed = generation.resolve(strict=True)
    fields = parse_descriptor(installed / 'generation.meta')
    assert fields['upstream_package_version'] in ['0.161.0', '0.163.0']
    # Explicit base fixtures are private signer inputs, not public release admission.
    # The actual Core verifies the resulting signature/inventory before execution.
    if frontend is None:
        assert generation is not None and fields['helper_count'] == '2'
        records = [line.split('\t') for line in (installed / 'release.manifest').read_text().splitlines() if line.startswith('file\t')]
        expected = {'core', 'runtime', 'codex-code-mode-host', 'generation.meta',
                    'browser/open/curl', 'browser/manual/curl'}
        assert len(records) == len(expected) and {row[1] for row in records} == expected
        inventory = [(row[1], row[2], row[3]) for row in records]
    else:
        _, inventory = parse_manifest(installed / 'release.manifest')
    assert all((installed / name).is_file() and not (installed / name).is_symlink()
               and sha(installed / name) == digest for name, digest, _ in inventory)
    generation = root / 'local-permission-fixture'
    for name in [row[0] for row in inventory] + ['release.manifest', 'release.sig']:
        target = generation / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(installed / name, target)
    core_mode = (generation / 'core').stat().st_mode & 0o777
    shutil.copy2(core, generation / 'core')
    (generation / 'core').chmod(core_mode)
    if manager is not None:
        manager_mode = (generation / 'manager').stat().st_mode & 0o777
        shutil.copy2(manager, generation / 'manager')
        (generation / 'manager').chmod(manager_mode)
    if frontend is not None:
        raw = frontend.read_bytes()
        changed = 0
        for old, new, count in [
            (b'/etc/resolv.conf', b'/proc/self/fd/33', 2),
            (b'/etc/codex/config.toml', b'/dev/fd/34/config.toml', 1),
            (b'/etc/codex/requirements.toml', b'/dev/fd/34/requirements.toml', 1),
            (b'/etc/codex/managed_config.toml', b'/dev/fd/34/managed_config.toml', 1),
        ]:
            assert len(old) == len(new) and raw.count(old) == count and not raw.count(new)
            changed += sum(a != b for a, b in zip(old, new)) * count
            raw = raw.replace(old, new)
        assert changed == 54
        frontend_mode = (generation / 'helpers/2').stat().st_mode & 0o777
        (generation / 'helpers/2').write_bytes(raw)
        (generation / 'helpers/2').chmod(frontend_mode)
    text = (generation / 'generation.meta').read_text()
    text = text.replace(fields['generation_id'], generation.name)
    lines = []
    for line in text.splitlines():
        if line.startswith('core_artifact_digest\t'):
            line = 'core_artifact_digest\t' + sha(generation / 'core')
        elif manager is not None and line.startswith('manager_artifact_digest\t'):
            line = 'manager_artifact_digest\t' + sha(generation / 'manager')
        elif line.startswith('helper\ttermux-manager-tui-v1:'):
            line = 'helper\ttermux-manager-tui-v1:' + fields['upstream_package_version'] + '\t' + sha(generation / 'helpers/2')
        lines.append(line)
    (generation / 'generation.meta').write_text('\n'.join(lines) + '\n')
    key, public = root / 'fixture-key.pem', root / 'fixture-public.pem'
    subprocess.run(['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', str(key)], check=True, capture_output=True)
    key.chmod(0o600)
    subprocess.run(['openssl', 'pkey', '-in', str(key), '-pubout', '-out', str(public)], check=True, capture_output=True)
    der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(public), '-outform', 'DER'])
    manifest = (generation / 'release.manifest').read_text().replace(fields['generation_id'], generation.name)
    manifest = '\n'.join(
        'release_public_key\t' + der[-32:].hex() if line.startswith('release_public_key\t') else
        '\t'.join(['file', line.split('\t')[1], sha(generation / line.split('\t')[1]), line.split('\t')[3]])
        if line.startswith('file\t') else line for line in manifest.splitlines()) + '\n'
    (generation / 'release.manifest').write_text(manifest)
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-inkey', str(key), '-rawin', '-in', str(generation / 'release.manifest'), '-out', str(generation / 'release.sig')], check=True, capture_output=True)
    home, prefix = root / 'h', root / 'p'
    for path in [home / '.codex', home / '.local/share/codex/core/config', prefix / 'bin', prefix / 'etc/tls', root / 't', root / 'w']:
        path.mkdir(parents=True, mode=0o700)
    shutil.copytree(generation, home / '.local/lib/codex/core/generations' / generation.name)
    shutil.copy2(core, prefix / 'bin/codex')
    (home / '.local/share/codex/core/activation-state').write_text(fixture_activation_state(generation.name, public))
    host_prefix = Path(os.environ['PREFIX'])
    shutil.copy2(host_prefix / 'etc/resolv.conf', prefix / 'etc/resolv.conf')
    shutil.copy2(host_prefix / 'etc/tls/cert.pem', prefix / 'etc/tls/cert.pem')
    (prefix / 'bin/openssl').symlink_to(shutil.which('openssl'))
    catalog_path = Path(__file__).parent / 'fixtures/permission-model.json'
    native = json.loads(catalog_path.read_text())
    catalog = {'models': [native]}
    for name in ['native-models.json', 'models_cache.json']:
        (home / '.codex' / name).write_text(json.dumps(catalog))
        (home / '.codex' / name).chmod(0o600)
    (home / '.codex/config.toml').write_text(
        'model = "gpt-6.1-sol"\nmodel_reasoning_effort = "none"\n'
        'model_catalog_json = ' + json.dumps(str(home / '.codex/native-models.json')) + '\n'
        'approval_policy = "never"\napprovals_reviewer = "user"\nsandbox_mode = "danger-full-access"\ncheck_for_update_on_startup = false\n'
        '[tui]\nscreen_reader_detection_done = true\n[features]\nmemories = false\n')
    (home / '.codex/config.toml').chmod(0o600)
    (home / '.codex/auth.json').write_text(json.dumps({'OPENAI_API_KEY': 'owned-inert-fixture', 'auth_mode': 'apikey'}))
    (home / '.codex/auth.json').chmod(0o600)
    env = dict(HOME=str(home), CODEX_HOME=str(home / '.codex'), PREFIX=str(prefix),
        TMPDIR=os.environ['TMPDIR'], PATH=str(prefix / 'bin') + ':' + str(host_prefix / 'bin'),
        TERM='xterm-256color', LANG='C.UTF-8', SHELL=str(host_prefix / 'bin/bash'),
        SSL_CERT_FILE=str(prefix / 'etc/tls/cert.pem'))
    for name in ['OPENAI_API_KEY', 'CODEX_ACCESS_TOKEN', 'ACCESS_TOKEN', 'CODEX_SQLITE_HOME',
        'CODEX_PROFILE_CORE', 'CODEX_CHAT_RETURN_PROFILE', 'CODEX_EXEC_SERVER_URL', 'CODEX_TERMUX_BACKEND_STARTUP',
        'TERMUX_SESSION_ID', 'CODEX_TERMUX_WINDOW_TOKEN', 'CODEX_TERMUX_WINDOW_READY', 'CODEX_TERMUX_SESSION_ORIGIN']:
        env.pop(name, None)
    # Built-in providers cannot be redefined; upstream supports these env overrides.
    env['OPENAI_BASE_URL'] = 'http://127.0.0.1:9/v1'
    env['OPENAI_API_KEY'] = 'owned-inert-fixture'
    return home, prefix, env


def server_paths(home):
    expected=home/'.local/lib/codex/core/generations/local-permission-fixture/runtime'
    paths=[]
    for family in ['servers','catalogs']:
        for path in (home/'.local/share/codex/core'/family).glob('*/s'):
            try:
                pid=int(path.with_name('pid').read_text())
                if Path(f'/proc/{pid}/exe').resolve()==expected:paths.append(path)
            except (OSError,ValueError):pass
    return paths


def rpc(home, method, params):
    matches=[];loaded=[]
    for path in server_paths(home):
        ws=Ws(str(path.resolve()),uri_path='/rpc')
        try:
            data=ws.call('thread/loaded/list',{'limit':100})
            assert not data.get('nextCursor'),'owned loaded fixture exceeded page'
            loaded.extend(data['data'])
            if params.get('threadId') in data['data']:matches.append(path)
        finally:ws.close()
    if method=='thread/loaded/list':return {'data':loaded}
    assert len(matches)==1,'one owned thread server required'
    ws=Ws(str(matches[0].resolve()), uri_path='/rpc')
    try:return ws.call(method,params)
    finally:ws.close()



def schema_value(schema):
    if 'enum' in schema:
        for preferred in ['allow','low','high']:
            if preferred in schema['enum']:return preferred
        return schema['enum'][0]
    if schema.get('type')=='object':
        return {k:schema_value(v) for k,v in schema.get('properties',{}).items()}
    if schema.get('type')=='boolean':return True
    if schema.get('type') in ['integer','number']:return 0
    if schema.get('type')=='array':return []
    return 'Owned disposable fixture explicitly authorized by its test operator.'


def probe(args):
    with tempfile.TemporaryDirectory(prefix='pl-',dir=os.environ['TMPDIR']) as tmp:
        root=Path(tmp)
        home,prefix,env=fixture(root,args.core,args.frontend,args.manager,args.generation)
        # Deterministic Responses fixture uses the public v1 tool wire format.
        # Production model metadata is never changed.
        for name in ['native-models.json','models_cache.json']:
            p=home/'.codex'/name;catalog=json.loads(p.read_text())
            for entry in catalog['models']:
                entry.update(multi_agent_version='v1', context_window=262144,
                    max_context_window=262144, auto_compact_token_limit=200000)
            p.write_text(json.dumps(catalog))
        work=root/'w'; command=work/'owned-proof'
        command.write_text('#!'+os.environ['PREFIX']+'/bin/sh\n'
            'printf "%s" "$1" > '+shlex.quote(str(work))+'/"$1"\n'
            'printf "OWNED_PERMISSION_PROOF:%s\\n" "$1"\n')
        command.chmod(0o700)
        rules=home/'.codex/rules';rules.mkdir(mode=0o700)
        (rules/'owned.rules').write_text('prefix_rule(pattern = ['+json.dumps(str(command))+'], decision = "prompt")\n')
        counts={'primary':0,'guardian':0,'nativeResults':0}; failures=[]

        class Model(http.server.BaseHTTPRequestHandler):
            def log_message(self,*_):pass
            def do_POST(self):
                try:
                    assert self.path=='/v1/responses'
                    body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                    fmt=(body.get('text') or {}).get('format') or {}
                    guardian=fmt.get('type')=='json_schema' and 'outcome' in fmt.get('schema',{}).get('properties',{})
                    if guardian:
                        counts['guardian']+=1
                        text=json.dumps({'risk_level':'low','user_authorization':'high','outcome':'allow','rationale':'Execute only the owned disposable fixture program requested by the test operator.'})
                        item={'id':'msg-owned','type':'message','role':'assistant','content':[{'type':'output_text','text':text,'annotations':[]}]}
                    elif fmt.get('type')=='json_schema':
                        text=json.dumps(schema_value(fmt['schema']))
                        item={'id':'msg-owned','type':'message','role':'assistant','content':[{'type':'output_text','text':text,'annotations':[]}]}
                    else:
                        counts['primary']+=1
                        outputs=[d.get('output') for d in body.get('input',[]) if d.get('type')=='function_call_output']
                        if outputs:counts['lastOwnedOutput']=str(outputs[-1])[-600:]
                        users=[d for d in body.get('input',[]) if d.get('role')=='user']
                        labels=re.findall(r'PL_(?:MAIN|RESUME|SIDE)',json.dumps(users[-1:]));assert labels,'owned user label missing'
                        label=labels[-1]
                        done=any(d.get('type')=='function_call_output' and 'OWNED_PERMISSION_PROOF:'+label in json.dumps(d) for d in body.get('input',[]))
                        if done:
                            counts['nativeResults']+=1
                            item={'id':'msg-owned','type':'message','role':'assistant','content':[{'type':'output_text','text':label+'_OK','annotations':[]}]}
                        else:
                            tools=list(body.get('tools',[]))
                            for entry in body.get('input',[]):
                                if entry.get('type')=='additional_tools':tools.extend(entry.get('tools',[]))
                            tool=next(t for t in tools if t.get('name') in ['exec_command','functions'])
                            namespace=tool.get('name') if tool.get('type')=='namespace' else None
                            session=re.search(r'Process running with session ID (\d+)',str(outputs[-1])) if outputs else None
                            name='write_stdin' if session else 'exec_command'
                            arguments={'session_id':int(session[1]),'chars':'','yield_time_ms':1000,'max_output_tokens':100} if session else {'cmd':shlex.quote(str(command))+' '+label,'login':False,'yield_time_ms':1000,'max_output_tokens':100}
                            item={'id':'fc-owned','type':'function_call','call_id':'call_'+label+'_'+str(counts['primary']),'name':name,'arguments':json.dumps(arguments)}
                            if namespace:item['namespace']=namespace
                    response={'id':'resp-owned','object':'response','model':body['model'],'status':'completed','output':[item],'usage':{'input_tokens':10,'output_tokens':10,'total_tokens':20}}
                    self.send_response(200);self.send_header('Content-Type','text/event-stream');self.end_headers()
                    events=[('response.created',{'type':'response.created','response':{**response,'status':'in_progress','output':[]}}),
                        ('response.output_item.added',{'type':'response.output_item.added','output_index':0,'item':item}),
                        ('response.output_item.done',{'type':'response.output_item.done','output_index':0,'item':item}),
                        ('response.completed',{'type':'response.completed','response':response})]
                    if item['type']=='message':
                        events.insert(2,('response.output_text.delta',{'type':'response.output_text.delta','item_id':item['id'],'output_index':0,'content_index':0,'delta':item['content'][0]['text']}))
                    for name,data in events:self.wfile.write(('event: '+name+'\ndata: '+json.dumps(data)+'\n\n').encode())
                    self.wfile.flush()
                except Exception as e:
                    failures.append(type(e).__name__+':'+str(e));self.close_connection=True

        model=http.server.ThreadingHTTPServer(('127.0.0.1',0),Model);model.daemon_threads=True
        threading.Thread(target=model.serve_forever,daemon=True).start()
        env['OPENAI_BASE_URL']=f'http://127.0.0.1:{model.server_port}/v1'
        config=home/'.codex/config.toml'
        old=config.read_text()
        catalog_override='model_catalog_json = '+json.dumps(str(home/'.codex/native-models.json'))+'\n'
        if args.native_catalog or args.catalog_layer!='user':
            old=re.sub(r'^model_catalog_json = .*\n','',old,flags=re.M)
        if not args.native_catalog and args.catalog_layer in ['project','profile']:
            layer=work/'.codex/config.toml' if args.catalog_layer=='project' else home/'.codex/startup-proof.config.toml'
            layer.parent.mkdir(exist_ok=True,mode=0o700)
            layer.write_text(catalog_override);layer.chmod(0o600)
        old=re.sub(r'^sandbox_mode = .*\n','',old,flags=re.M)
        old=old.replace('approval_policy = "never"','approval_policy = "on-request"').replace('approvals_reviewer = "user"','approvals_reviewer = "auto_review"')
        if getattr(args,'manager_default_reviewer',False):
            old=re.sub(r'^(approval_policy|approvals_reviewer) = .*\n','',old,flags=re.M)
        old=old.replace('[features]\n','[features]\nguardian_approval = true\nplugins = false\n')
        old += "\n[projects."+json.dumps(str(work))+"]\ntrust_level = \"trusted\"\n"
        old = 'model_provider = "owned"\n' + old
        if args.named_permission:
            old = 'default_permissions = "owned"\n' + old
            old += '\n[permissions.owned.filesystem]\n":root" = "write"\n[permissions.owned.network]\nenabled = true\n'
        old += '\n[model_providers.owned]\nname = "Owned fixture"\nbase_url = ' + json.dumps(env['OPENAI_BASE_URL']) + '\nwire_api = "responses"\nrequires_openai_auth = false\n'
        config.write_text(old)
        processes=[];masters=[];observers=[];screen=''

        def start(thread=None):
            master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,160,0,0))
            argv=[str(prefix/'bin/codex'),'--no-alt-screen']
            if not args.native_catalog:
                if args.catalog_layer=='cli':argv+=['-c',catalog_override.strip()]
                elif args.catalog_layer=='profile':argv+=['--profile','startup-proof']
            if thread:argv+=['resume',thread]
            process=subprocess.Popen(argv,env=env,cwd=work,stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
            os.close(slave);processes.append(process);masters.append(master)
            return process,master

        def drain(master,seconds=.2):
            nonlocal screen
            end=time.monotonic()+seconds
            while time.monotonic()<end:
                if select.select([master],[],[],.05)[0]:
                    try:screen+=os.read(master,65536).decode(errors='replace')
                    except OSError:break

        def wait(master,predicate,seconds=40):
            end=time.monotonic()+seconds
            while time.monotonic()<end:
                drain(master)
                if predicate():return
                assert not failures,failures
            raise AssertionError('owned lifecycle timeout '+str(counts)+' failures='+str(failures)+': '+re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]','',screen[-1800:]))

        def stop(process,master):
            os.killpg(process.pid,signal.SIGTERM)
            # Cold resume proof: retire only this fixture's own official server.
            for path in server_paths(home):
                pid=int(path.with_name('pid').read_text())
                try:
                    if Path(f'/proc/{pid}/exe').resolve().is_relative_to(home):os.kill(pid,signal.SIGTERM)
                except OSError:pass
            try:process.wait(timeout=8)
            except subprocess.TimeoutExpired:os.kill(process.pid,signal.SIGKILL);process.wait()
            drain(master)

        def loaded():
            if not server_paths(home):return []
            try:return rpc(home,'thread/loaded/list',{})['data']
            except (OSError,TimeoutError):return []

        def single_backend():
            if not args.expect_single_backend:return
            paths=server_paths(home)
            assert len(paths)==1, 'one resolved backend required: '+str(paths)
            expected='servers' if args.native_catalog else 'catalogs'
            assert paths[0].parent.parent.name==expected,paths
            other='catalogs' if args.native_catalog else 'servers'
            assert not list((home/'.local/share/codex/core'/other).glob('*/pid')), 'unexpected backend started'

        def snapshot(thread):
            d=rpc(home,'thread/resume',{'threadId':thread})
            return {k:d.get(k) for k in ['sandbox','approvalPolicy','approvalsReviewer','activePermissionProfile']}

        def idle(thread):
            d=rpc(home,'thread/read',{'threadId':thread,'includeTurns':False})['thread']
            return d.get('status',{}).get('type')=='idle'

        try:
            process,master=start()
            wait(master,lambda:bool(server_paths(home)))
            wait(master,lambda:len(loaded())==1)
            single_backend()
            ids=loaded();assert len(ids)==1,ids;thread=ids[0]
            rpc(home,'turn/start',{'threadId':thread,'input':[{'type':'text','text':'PL_MAIN','text_elements':[]}]})
            wait(master,lambda:(work/'PL_MAIN').is_file() and counts['nativeResults']>=1)
            wait(master,lambda:idle(thread))
            first=snapshot(thread)
            rpc(home,'thread/settings/update',{'threadId':thread,'permissions':':workspace','approvalPolicy':'on-request','approvalsReviewer':'auto_review'})
            stale=snapshot(thread);assert stale['sandbox']['type']=='workspaceWrite',stale
            stop(process,master)
            time.sleep(1)
            process,master=start(thread)
            wait(master,lambda:process.poll() is None and thread in loaded())
            time.sleep(3);drain(master)
            restored=snapshot(thread)
            if args.expect_stale_resume:
                assert restored['sandbox']['type']=='workspaceWrite',restored
                return {'accepted':True,'expectedStaleResumeObserved':True,'actualAndroidCommands':1,
                    'guardianRequests':counts['guardian'],'resumed':restored,'realModelInference':False,'productionChanged':False}
            assert restored['sandbox']['type']=='dangerFullAccess',restored
            assert restored['approvalPolicy']=='on-request' and restored['approvalsReviewer']=='auto_review',restored
            rpc(home,'turn/start',{'threadId':thread,'input':[{'type':'text','text':'PL_RESUME','text_elements':[]}]})
            wait(master,lambda:(work/'PL_RESUME').is_file() and counts['nativeResults']>=2)
            wait(master,lambda:idle(thread))
            before_side=set(loaded())
            single_backend()
            # Observe the actual fork response without tracing startup bundle
            # extraction/signature checks, which can cause artificial timeouts.
            owners=[]
            for path in server_paths(home):
                ws=Ws(str(path.resolve()),uri_path='/rpc')
                try:
                    if thread in ws.call('thread/loaded/list',{})['data']:owners.append(path)
                finally:ws.close()
            assert len(owners)==1, 'one owned parent server required'
            pid=int(owners[0].with_name('pid').read_text())
            observer=subprocess.Popen([shutil.which('strace'),'-f','-p',str(pid),'-e','trace=write,writev,sendto,sendmsg','-s','262144','-o',str(root/'trace-1')],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            observers.append(observer)
            wait(master,lambda:('TracerPid:\t'+str(observer.pid)) in Path(f'/proc/{pid}/status').read_text())
            for character in b'/btw':os.write(master,bytes([character]));drain(master,.1)
            drain(master,.4);os.write(master,b'\r')
            wait(master,lambda:bool(set(loaded())-before_side))
            child=next(iter(set(loaded())-before_side))
            def side_response():
                # Backend visibility can precede delivery to the frontend.
                # Wait for the actual fork response rather than sampling its
                # trace once immediately after the backend lists the child.
                for line in (root/'trace-1').read_text(errors='replace').splitlines():
                    if child not in line or 'sandbox' not in line:continue
                    for quoted in re.findall(r'"(?:\\.|[^"\\])*"',line):
                        try:data=ast.literal_eval(quoted)
                        except (ValueError,SyntaxError):continue
                        for brace in re.finditer(r'\{',data):
                            try:d=json.JSONDecoder().raw_decode(data[brace.start():])[0]
                            except ValueError:continue
                            result=d.get('result',{}) if isinstance(d,dict) else {}
                            if result.get('thread',{}).get('id')==child and 'sandbox' in result:
                                return {k:result.get(k) for k in ['sandbox','approvalPolicy','approvalsReviewer','activePermissionProfile']}
                return None
            wait(master,side_response)
            side=side_response()
            assert side and side['sandbox']['type']=='dangerFullAccess',side
            rpc(home,'turn/start',{'threadId':child,'input':[{'type':'text','text':'PL_SIDE','text_elements':[]}]})
            wait(master,lambda:(work/'PL_SIDE').is_file() and counts['nativeResults']>=3)
            assert counts['guardian']>=3,counts
            single_backend()
            assert 'error building bubblewrap command' not in screen
            return {'accepted':True,'namedPermission':args.named_permission,'nativeCatalog':args.native_catalog,'catalogLayer':args.catalog_layer,'singleBackendAccepted':args.expect_single_backend,'backendFamilies':sorted({p.parent.parent.name for p in server_paths(home)}),'actualAndroidCommands':3,'guardianRequests':counts['guardian'],'nativeResults':counts['nativeResults'],'initial':first,'seeded':stale,'resumed':restored,'side':side,'realModelInference':False,'productionChanged':False}
        finally:
            for observer in observers:
                if observer.poll() is None:
                    observer.terminate()
                    try:observer.wait(timeout=5)
                    except subprocess.TimeoutExpired:observer.kill();observer.wait()
            for process in processes:
                if process.poll() is None:
                    os.killpg(process.pid,signal.SIGTERM)
                    try:process.wait(timeout=8)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
            for path in server_paths(home):
                try:
                    pid=int(path.with_name('pid').read_text())
                    if Path(f'/proc/{pid}/exe').resolve().is_relative_to(home):os.kill(pid,signal.SIGTERM)
                except (OSError,ValueError):pass
            # Only this test's own runtime/plugin workers have cwd below its root.
            retired=[]
            for entry in Path('/proc').iterdir():
                if not entry.name.isdigit():continue
                try:
                    if Path(os.readlink(entry/'cwd')).is_relative_to(root):
                        pid=int(entry.name);os.kill(pid,signal.SIGTERM);retired.append(pid)
                except OSError:pass
            end=time.monotonic()+5
            while time.monotonic()<end and any(Path('/proc',str(pid)).exists() for pid in retired):time.sleep(.05)
            for master in masters:os.close(master)
            model.shutdown()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--core',type=Path,required=True);p.add_argument('--frontend',type=Path);p.add_argument('--generation',type=Path);p.add_argument('--manager',type=Path);p.add_argument('--manager-default-reviewer',action='store_true');p.add_argument('--report',type=Path,required=True);p.add_argument('--named-permission',action='store_true');p.add_argument('--native-catalog',action='store_true');p.add_argument('--catalog-layer',choices=['user','cli','project','profile'],default='user');p.add_argument('--expect-single-backend',action='store_true');p.add_argument('--expect-stale-resume',action='store_true');a=p.parse_args()
    if a.frontend is None and a.generation is None:p.error('--frontend or explicit Core-only --generation is required')
    result=probe(a);a.report.write_text(json.dumps(result)+'\n');a.report.chmod(0o600);print(json.dumps(result))
