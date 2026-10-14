"""Two real native clients, a shared profile and isolated Chat tool/result lanes."""
import argparse
import concurrent.futures
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

SOURCE=Path(__file__).resolve().parent


def run(args):
    # Production credentials, activation, conversations and provider processes
    # are not imported or replaced. Only the signed app login is reused.
    sys.path.insert(0,str(args.fixture_source))
    from qualify_permission_lifecycle import fixture
    from desktop_provider_profile import install
    with tempfile.TemporaryDirectory(prefix='chat-native-dual-',dir=os.environ['TMPDIR']) as temporary:
        root=Path(temporary);root.chmod(0o700)
        binding=root/'server.json'
        with (root/'server.log').open('w') as log:
            server=subprocess.Popen([str(args.bun),str(SOURCE/'desktop_responses_server.mjs'),str(args.bridge),str(args.session),str(binding)],stdout=log,stderr=log,start_new_session=True)
        try:
            for _ in range(100):
                if binding.exists():break
                assert server.poll() is None,'owned_adapter_start_failed'
                time.sleep(.1)
            assert binding.exists(),'owned_adapter_start_timeout'
            home,prefix,env=fixture(root,args.core,args.frontend)
            registry=home/'.local/share/codex/core/config/profiles';registry.mkdir(mode=0o700)
            with patch.dict(os.environ,{'HOME':str(home)}),contextlib.redirect_stdout(io.StringIO()):
                install(binding,registry,args.catalog)
            # No account-specific Chat profile is created by this fixture.
            roots=[]
            for i in range(2):
                account=root/f'account{i}';account.mkdir(mode=0o700)
                (account/'config.toml').write_text('check_for_update_on_startup=false\napproval_policy="never"\nsandbox_mode="danger-full-access"\n[features]\nplugins=false\nmemories=false\n')
                (account/'config.toml').chmod(0o600);roots.append(account)
            def invoke(i):
                account=roots[i];work=root/f'work{i}';work.mkdir(mode=0o700);nonce=os.urandom(12).hex();before=hashlib.sha256((account/'config.toml').read_bytes()).hexdigest()
                child_env={**env,'CODEX_HOME':str(account)}
                child_env.pop('OPENAI_API_KEY',None);child_env.pop('OPENAI_BASE_URL',None)
                task=f'Use the local shell tool to write exactly {nonce} to proof.txt. Use a separate second shell tool call to read proof.txt. Reply with exactly {nonce}. Do no other work.'
                if args.autoreview and i==0:
                    (account/'rules').mkdir(mode=0o700)
                    executable=work/'fixture-write';executable.write_text('#!'+os.environ['PREFIX']+'/bin/sh\nprintf %s '+nonce+' > proof.txt\n');executable.chmod(0o700)
                    (account/'rules/qualification.rules').write_text('prefix_rule(pattern = ["./fixture-write"], decision = "prompt")\n'+'prefix_rule(pattern = ['+json.dumps(str(executable))+'], decision = "prompt")\n');(account/'rules/qualification.rules').chmod(0o600)
                    with (account/'config.toml').open('a') as f:f.write('guardian_approval=true\n')
                    task='Use the local shell tool to execute ./fixture-write exactly once. This is an approved fixture in this disposable workspace. Do not rewrite it, use another interpreter, or bypass its approval rule. Use a separate shell call to read proof.txt. Reply with exactly '+nonce+'.'
                command=[str(prefix/'bin/codex'),'--profile','chatgpt','exec','--skip-git-repo-check','--json','--model','chatgpt-desktop/gpt-6','-c','model_reasoning_effort="none"','-c','web_search="disabled"',task]
                if args.autoreview and i==0:
                    before=hashlib.sha256((account/'config.toml').read_bytes()).hexdigest()
                    command[command.index('exec')+1:command.index('exec')+1]=['-c','approval_policy="on-request"','-c','approvals_reviewer="auto_review"']
                with (work/'out').open('w') as output,(work/'err').open('w') as error:
                    child=subprocess.Popen(command,cwd=work,env=child_env,stdin=subprocess.DEVNULL,stdout=output,stderr=error,start_new_session=True)
                    try:child.wait(timeout=600)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid,signal.SIGTERM)
                        try:child.wait(timeout=5)
                        except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=5)
                rows=[]
                for line in (work/'out').read_text().splitlines():
                    try:rows.append(json.loads(line))
                    except ValueError:pass
                items=[r['item'] for r in rows if r.get('type')=='item.completed']
                commands=[item for item in items if item.get('type')=='command_execution']
                reply=any(item.get('type')=='agent_message' and item.get('text','').strip()==nonce for item in items)
                alias=account/'chatgpt.config.toml'
                return {'exit':child.returncode,'successfulNativeCommands':sum(c.get('exit_code')==0 for c in commands),'exactReply':reply,'exactMarker':(work/'proof.txt').is_file() and (work/'proof.txt').read_text().strip()==nonce,'sharedProfileProjected':alias.is_symlink() and alias.resolve()==(registry/'chatgpt.config.toml').resolve(),'baseConfigUnchanged':hashlib.sha256((account/'config.toml').read_bytes()).hexdigest()==before,'completedItemTypes':sorted({item.get('type','') for item in items}),'turnCompleted':any(r.get('type')=='turn.completed' for r in rows)}
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:clients=list(pool.map(invoke,range(2)))
            proof=json.loads(Path(str(binding)+'.proof').read_text())
            result={'actualAndroidNativeClients':2,'clients':clients,'maxConcurrentRequests':proof['maxConcurrentRequests'],'transport':proof.get('selectedControl',{}).get('transport'),'ownerConflicts':proof['ownerConflicts'],'guardianRequests':proof['guardianRequests'],'guardianIndependentConversations':proof['guardianIndependentConversations'],'adapterErrorCode':proof.get('errorCode'),'proInference':False,'productionChanged':False}
            result['accepted']=all(c['exit']==0 and c['successfulNativeCommands']>=2 and c['exactReply'] and c['exactMarker'] and c['sharedProfileProjected'] and c['baseConfigUnchanged'] for c in clients) and result['maxConcurrentRequests']>=2 and not result['ownerConflicts'] and result['transport']=='signed-app-internal'
            if args.autoreview:result['accepted']=result['accepted'] and result['guardianRequests']>0 and result['guardianIndependentConversations']>0
            args.report.write_text(json.dumps(result));args.report.chmod(0o600);print(json.dumps(result));return result['accepted']
        finally:
            # The exact child was created here; never stop the installed adapter.
            server.terminate()
            try:server.wait(timeout=5)
            except subprocess.TimeoutExpired:server.kill();server.wait(timeout=5)


if __name__=='__main__':
    p=argparse.ArgumentParser()
    for name in ['core','frontend','fixture-source','bun','bridge','session','catalog','report']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--autoreview',action='store_true');args=p.parse_args();raise SystemExit(0 if run(args) else 1)
