#!/usr/bin/env python3
"""G packaged-entrypoint live acceptance; one bounded unsupervised batch.

Preparation makes no model request. Start is explicit and exclusive. Native work
uses the installed disposable Core/Provider package. Old work is never replayed.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import select
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import traceback

SOURCE = Path(__file__).resolve().parents[1]
if not (SOURCE / 'providers/chatgpt').is_dir():
    SOURCE = Path(json.loads((Path(__file__).parent / 'plan.json').read_text())['repository'])
sys.path.insert(0, str(SOURCE / 'scripts'))
from provider_qualification import explicit_input
from qualify_permission_lifecycle import fixture
from qualify_provider_chat_entry import control, native_work, InputClient, preflight_native, cli_final_visible
from qualify_provider_f import Batch, save, stamp, sha, creation_call_id, followup_branch, input_completed

TASK = '''User-authorized G installed product acceptance in this isolated project. Use native Codex tools for every read, patch, command and test; no Work, computer use, package installation or network. Preserve test_original.py byte-for-byte. First demonstrate its failures. Implement release_label.py parse_label(text): accept exactly chatgpt-NAME-sSEQUENCE, with NAME matching [a-z][a-z0-9]*(?:-[a-z0-9]+)*, and SEQUENCE a canonical positive decimal <= 2**63-1; return (NAME, SEQUENCE). Reject every non-string, invalid separator/case/sequence with ValueError. Add a CLI taking exactly one label argument and printing JSON {"name": NAME, "sequence": SEQUENCE}; invalid input exits 2 without traceback or successful JSON. Add meaningful regression tests, run all tests and report actual changes/results/elapsed seconds using a real clock. Do not change the original tests or loosen requirements. Publish the complete final through the native original result and this saved Chat. Never fabricate, replay an uncertain call or create another primary work.'''
ORIGINAL = '''import unittest
from release_label import parse_label
class Contract(unittest.TestCase):
 def test_valid(self):
  self.assertEqual(parse_label('chatgpt-alpha-beta2-s17'),('alpha-beta2',17))
  self.assertEqual(parse_label('chatgpt-a-s1'),('a',1))
 def test_invalid(self):
  for value in [None,True,1,'chatgpt-a-s0','chatgpt-a-s01','chatgpt-A-s1','chatgpt-a--b-s1','chatgpt-a-s9223372036854775808','chatgpt-a-s1\\n']:
   with self.subTest(value=value), self.assertRaises(ValueError):parse_label(value)
if __name__=='__main__':unittest.main()
'''


def input_client_id(root):
    return 'g-input-'+hashlib.sha256(os.fsencode(root.resolve())).hexdigest()[:24]


def cold_budget(record, deadline_unix_ms):
    assert record['selection']['reasoning']=='current'
    start=record['assignment']['started_at_unix_ms']
    assert isinstance(start,int) and start>0
    budget=dict(record['assignment']['budget'])
    budget['max_elapsed_seconds']=max(budget['max_elapsed_seconds'],(deadline_unix_ms-start+999)//1000)
    return budget


def require_connector_ready(info, proof):
    probe=proof.get('live_mcp_probe') or {}
    expected=set(info['expected_tools'])
    assert proof.get('refresh') and proof.get('cache_refresh') and probe.get('accepted'),'backend MCP readiness missing'
    assert probe.get('model_requests')==0 and probe.get('native_effects')==0
    assert any(sample.get('read_call') and sample.get('schema_agreement') is True and set(sample.get('names',[]))==expected
               for sample in probe.get('samples',[])),'actual backend MCP catalog/read probe missing'


def prepare(args):
    prior = explicit_input('PRIOR_ROOT')
    connector = explicit_input('CONNECTOR_REFERENCE')
    bun = explicit_input('BUN')
    tunnel_profile = explicit_input('TUNNEL_PROFILE')
    tunnel_alias = explicit_input('TUNNEL_ALIAS', path=False)
    os.umask(0o077)
    root = Path(tempfile.mkdtemp(prefix='gg.', dir=os.environ['TMPDIR']))
    native = Path(tempfile.mkdtemp(prefix='gn.', dir=os.environ['TMPDIR']))
    (root / 'candidate').mkdir()
    for name, artifact in [('core', args.core), ('builder', args.builder), ('provider', args.provider)]:
        shutil.copy2(artifact, root / 'candidate' / name)
    shutil.copy2(SOURCE / 'providers/chatgpt/src/app_bridge.js', root / 'candidate/app_bridge.js')
    home, prefix, env = fixture(native, args.core, args.frontend, generation=args.generation)
    env.update(XDG_CONFIG_HOME=str(root / 'config'), XDG_STATE_HOME=str(root / 'xdg-state'))
    (root / 'source-inputs.json').write_text(json.dumps(dict(base_commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=SOURCE).decode().strip(),
        files={str(p.relative_to(SOURCE)):sha(p) for base in ('providers/chatgpt','crates/runtime-binding','crates/core','crates/release-builder')
               for p in (SOURCE/base).rglob('*') if p.is_file() and not {'target','__pycache__','.git'}.intersection(p.parts)}),indent=2)+'\n')
    binary=root/'candidate/provider';builder=root/'candidate/builder';launcher=prefix/'bin/codex'
    subprocess.run([builder,'provider-package','--binary',binary,'--assets',SOURCE/'providers/chatgpt','--source-inputs',root/'source-inputs.json',
                    '--source-sha',subprocess.check_output(['git','rev-parse','HEAD'],cwd=SOURCE).decode().strip(),'--generation-id','chatgpt-g-live-1','--release-sequence','1',
                    '--private-key',native/'fixture-key.pem','--openssl',shutil.which('openssl'),'--output',root/'package'],check=True,capture_output=True)
    subprocess.run([launcher,'provider','install','chatgpt','--local',root/'package'],env=env,check=True,capture_output=True)
    package_binary=home/'.local/lib/codex/providers/chatgpt/generations/chatgpt-g-live-1/codex-chatgpt-provider'
    old_env=json.loads((prior/'environment-private.json').read_text())
    old_config=Path(old_env.get('XDG_CONFIG_HOME',str(Path(old_env['HOME'])/'.config')))/'codex/providers/chatgpt.toml'
    settings=root/'config/codex/providers/chatgpt.toml';settings.parent.mkdir(parents=True)
    shutil.copy2(old_config,settings)
    public=[str(launcher),'provider','chatgpt']
    changes=['execution.mode','operator','execution.core',str(launcher),'execution.home',str(home/'.codex'),
             'execution.workspace',str(native/'w'),'model.default','chatgpt-desktop/gpt-6','model.reasoning_effort','high',
             'transport.return_path','mcp','transport.connector_name','Termux Bridge','history.project_name','Termux Provider: G','history.create_if_missing','true']
    subprocess.run(public+['config','set',*changes],env=env,check=True,capture_output=True)
    subprocess.run(public+['config','unset','history.project_id'],env=env,check=True,capture_output=True)
    descriptor=prior/'relay/desktop-host.json'
    process=subprocess.Popen(public+['serve','--state',str(root/'state'),'--descriptor',str(descriptor),'--port','0'],env=env,
                             stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=(root/'provider.log').open('ab'),start_new_session=True)
    assert select.select([process.stdout],[],[],30)[0],'Provider startup outcome unknown; inspect, do not replay'
    ready=json.loads(process.stdout.readline());save(root/'provider-ready-private.json',ready,True)
    prepared=subprocess.run(public+['prepare','--home',str(home/'.codex'),'--state',str(root/'state')],env=env,check=True,capture_output=True)
    save(root/'profile-prepare.json',json.loads(prepared.stdout),True)
    info=dict(mode='common',binary=str(package_binary),binary_sha256=sha(package_binary),state=str(root/'state'),home=str(home/'.codex'),
              workspace=str(native/'w'),core=str(launcher),fixture_home=str(home),fixture_root=str(native),pid=process.pid,
              port=int(ready['base_url'].split(':')[-1].split('/')[0]),connector_reference=str(connector),prior_root=str(prior),tunnel_profile=str(tunnel_profile),tunnel_alias=tunnel_alias,
              desktop_session=str(prior/'relay'),expected_tools=['native_catalog','native_tool','codex_host','codex_chats','codex_sessions','codex_attach','codex_history','codex_tools','codex_execute','codex_answer','codex_report','codex_finish','codex_status','codex_stop','codex_handoff','codex_native_summary'])
    save(root/'info.json',info,True);save(root/'environment-private.json',env,True)
    plan=dict(schema='termux-codex-g-live-v1',repository=str(SOURCE),desktop_session=str(prior/'relay'),project_name='Termux Provider: G',
              bindings=dict(initial='app://-/assets/app-initial-ed5807835d97.js',shared='app://-/assets/app-shared-2d77298f0100.js'),
              provider_owner=(root/'state/owner-v1').read_text(),shared_bridge=True,phase_label='G',input_client_id=input_client_id(root),
              bun=str(bun),connector_name='Termux Bridge',
              host_model='gpt-6-thinking',thinking_effort='extended',max_source_requests=20,max_user_turns=2,max_elapsed_seconds=1800,max_no_progress_seconds=240,
              assignment_budget=dict(max_parallel=1,max_app_starts=16,max_native_calls=40,max_elapsed_seconds=900),
              max_control_observations=700,max_model_responses=20,actual_Chat_requests_so_far=0)
    save(root/'plan.json',plan,True)
    client=(SOURCE/'scripts/chat_entry_client.mjs').read_text().replace("'../providers/chatgpt/desktop_cdp.mjs'",json.dumps(str(SOURCE/'providers/chatgpt/desktop_cdp.mjs')))
    (root/'client.mjs').write_text(client)
    (root/'run.py').write_text(Path(__file__).read_text())
    wrapper=root/'mcp-stdio';wrapper.write_text('#!'+os.environ['PREFIX']+'/bin/sh\nexec '+shlex.join([os.environ['PREFIX']+'/bin/env','-i',*[k+'='+v for k,v in env.items()],*public,'mcp-stdio','--state',str(root/'state')])+'\n');wrapper.chmod(0o700)
    for name in ('managed-mcp','managed-local','direct','general'):
        workspace=root/('w-'+name);workspace.mkdir();(workspace/'release_label.py').write_text('def parse_label(text):\n    return text\n')
        (workspace/'test_original.py').write_text(ORIGINAL)
        for command in [['git','init','-q'],['git','add','.'],['git','-c','user.name=G fixture','-c','user.email=g-fixture@localhost','commit','-qm','owned original G contract']]:
            subprocess.run(command,cwd=workspace,env=env,check=True,capture_output=True)
    (root/'task.txt').write_text(TASK+'\n')
    for name, reasoning in [('direct','current'),('general','managed')]:
        create=dict(action='create',call_id=creation_call_id(root,name),reasoning=reasoning,return_path='mcp',workspace=str(root/('w-'+name)),role='g-'+name,budget=plan['assignment_budget'])
        instructions=('This Chat is the reasoning source. After create, start this genuine task once, attach and echo its exact receipt as accept before effects. Discover tools, execute with stable call IDs, publish through codex_report and finish through codex_finish.' if name=='direct' else 'Delegate reasoning to the created managed worker. Start this genuine task once with a stable call ID, wait on the original work, read its completed native history and report the actual result here. Do not attach or execute on behalf of the managed worker.')
        (root/('task-'+name+'.txt')).write_text('Use only the installed Termux Bridge MCP; no Work. Create exactly one work with these fields: '+json.dumps(create)+'. '+instructions+'\nTask:\n'+TASK+'\nPublish the full final in this saved Chat, then G_'+name.upper()+'_COMPLETE alone on the last line.\n')
    preflight_native(root,info,env)
    files=[p for p in root.rglob('*') if p.is_file() and (p.parent==root and p.suffix in ('.py','.mjs','.txt') or p.parent==root/'candidate')]
    sealed={str(p.relative_to(root)):sha(p) for p in files}
    sealed.update({str(SOURCE/name):sha(SOURCE/name) for name in ['scripts/provider_qualification.py','scripts/qualify_provider_f.py','scripts/qualify_provider_chat_entry.py','scripts/qualify_tasks.py','providers/chatgpt/desktop_cdp.mjs']})
    save(root/'sealed-inputs.json',sealed,True)
    save(root/'setup-complete.json',dict(prepared=True,actual_Chat_requests=0,service_pid=process.pid,public_package_selected=True,native_preflight=True),True)
    print(json.dumps(dict(root=str(root),native_fixture=str(native),prepared=True,actual_Chat_requests=0)))


def connect(root):
    info=json.loads((root/'info.json').read_text())
    prior=Path(info['prior_root'])
    profile=json.loads(Path(info['tunnel_profile']).read_text())['control_plane']
    alias=info['tunnel_alias']
    te=json.loads((prior/'tunnel-environment-private.json').read_text())
    def run(name,args):
        out=subprocess.run(args,env=te,capture_output=True,timeout=65)
        (root/(name+'-private.json')).write_bytes(out.stdout);(root/(name+'.stderr')).write_bytes(out.stderr)
        assert out.returncode==0,name+' failed; inspect before another action'
        return json.loads(out.stdout)
    status=run('tunnel-before',['tunnel-client','runtimes','status',alias,'--json'])
    assert status['process_running'] and status['healthy'] and status['alias']==alias
    save(root/'tunnel-switch-intent.json',dict(prior=sha(prior/'mcp-stdio'),new=sha(root/'mcp-stdio'),actual_Chat_requests=0,**stamp()),True)
    run('tunnel-disconnect',['tunnel-client','runtimes','disconnect',alias,'--json'])
    run('tunnel-connect',['tunnel-client','runtimes','connect','--alias',alias,'--tunnel-id',profile['tunnel_id'],'--runtime-api-key',profile['api_key'],
         '--profile','g-product','--profile-dir',str(root/'profiles'),'--mcp-command',str(root/'mcp-stdio'),'--json'])
    after=run('tunnel-after',['tunnel-client','runtimes','status',alias,'--json'])
    assert after['process_running'] and after['healthy'] and after['ready']
    plan=json.loads((root/'plan.json').read_text())
    refresh=subprocess.run([plan['bun'],str(SOURCE/'scripts/d3_refresh.mjs'),str(root)],capture_output=True,timeout=260)
    (root/'refresh.stdout').write_bytes(refresh.stdout);(root/'refresh.stderr').write_bytes(refresh.stderr)
    assert refresh.returncode==0,'backend MCP refresh/probe failed; no model submitted'
    require_connector_ready(json.loads((root/'info.json').read_text()),json.loads((root/'refresh-private.json').read_text()))
    save(root/'connection-ready.json',dict(ready=True,backend_mcp_ready=True,Chat_requests=0,refresh_submitted=True,**stamp()),True)
    print(json.dumps(dict(ready=True,Chat_requests=0)))


def prepare_remaining(prior):
    """Distinct settled continuation: retain accepted MCP proof, never replay work."""
    review = json.loads((prior/'review-1/review.json').read_text())
    records = native_work(prior)
    assert len(records) == 2 and {r['phase'] for r in records} == {'completed', 'stopped'}
    assert len([r for r in review['reports'] if r['accepted']]) == 1
    assert all(r['unreturned'] == 0 for r in review['reports'])
    assert json.loads((prior/'failure-stops-private.json').read_text())[0]['remaining_jobs'] == []
    os.umask(0o077)
    root = Path(tempfile.mkdtemp(prefix='gg.', dir=os.environ['TMPDIR']))
    shutil.copytree(prior/'candidate',root/'candidate')
    for name in ['info.json','environment-private.json','source-inputs.json']:
        shutil.copy2(prior/name,root/name)
    client=(SOURCE/'scripts/chat_entry_client.mjs').read_text().replace("'../providers/chatgpt/desktop_cdp.mjs'",json.dumps(str(SOURCE/'providers/chatgpt/desktop_cdp.mjs')))
    (root/'client.mjs').write_text(client)
    plan=json.loads((prior/'plan.json').read_text())
    plan.update(max_source_requests=20,max_model_responses=20,max_user_turns=2,
                deadline_unix_ms=int(time.time()*1000)+1800*1000,
                cases=['managed-local','direct','general'],input_client_id=input_client_id(root),
                completed_prior_work={r['id']:dict(passed=r['phase']=='completed') for r in records},
                prior_root=str(prior),prior_review_sha256=sha(prior/'review-1/review.json'))
    # Local functions require another model request after each tool result.
    # This bounds actual dispatches, not the number of saved conversations.
    plan['assignment_budget']=dict(plan['assignment_budget'],max_app_starts=16)
    save(root/'plan.json',plan,True)
    (root/'run.py').write_text(Path(__file__).read_text())
    shutil.copy2(SOURCE/'scripts/d3_notify.py',root/'notify.py')
    (root/'task.txt').write_text(TASK+'\n')
    for name in plan['cases']:
        workspace=root/('w-'+name);workspace.mkdir()
        (workspace/'release_label.py').write_text('def parse_label(text):\n    return text\n')
        (workspace/'test_original.py').write_text(ORIGINAL)
        for command in [['git','init','-q'],['git','add','.'],['git','-c','user.name=G fixture','-c','user.email=g-fixture@localhost','commit','-qm','owned original G continuation contract']]:
            subprocess.run(command,cwd=workspace,check=True,capture_output=True)
    for name in ['direct','general']:
        text=(prior/('task-'+name+'.txt')).read_text()
        text=text.replace(creation_call_id(prior,name),creation_call_id(root,name))
        text=text.replace(str(prior/('w-'+name)),str(root/('w-'+name)))
        (root/('task-'+name+'.txt')).write_text(text)
    files=[p for p in root.rglob('*') if p.is_file() and (p.parent==root or p.parent==root/'candidate')]
    sealed={str(p.relative_to(root)):sha(p) for p in files}
    sealed.update({str(SOURCE/name):sha(SOURCE/name) for name in ['scripts/provider_qualification.py','scripts/qualify_provider_f.py','scripts/qualify_provider_chat_entry.py','scripts/qualify_tasks.py','providers/chatgpt/desktop_cdp.mjs']})
    info=json.loads((root/'info.json').read_text())
    config=Path(json.loads((root/'environment-private.json').read_text())['XDG_CONFIG_HOME'])/'codex/providers/chatgpt.toml'
    sealed[str(config)]=sha(config)
    save(root/'sealed-inputs.json',sealed,True)
    save(root/'setup-complete.json',dict(prepared=True,actual_Chat_requests=0,reuses_installed_package=True,prior_root=str(prior)),True)
    print(json.dumps(dict(root=str(root),prepared=True,actual_Chat_requests=0,additional_dispatch_ceiling=20)))


def audit(root,batch):
    reports=[]
    for record in native_work(root):
        if record['id'] in batch.plan.get('completed_prior_work',{}):continue
        status=batch.rpc('codex_status',{'work_id':record['id']})
        assert status['completed'] and not status['pending_calls'] and not status['human_requests']
        history=batch.rpc('codex_history',{'work_id':record['id']});save(root/(record['id']+'-history-private.json'),history)
        workspace=Path(record['workspace']);assert (workspace/'test_original.py').read_text()==ORIGINAL
        suite=subprocess.run([sys.executable,'-B','-m','unittest','discover','-v'],cwd=workspace,env=batch.env,capture_output=True,timeout=45)
        (root/(record['id']+'-tests.log')).write_bytes(suite.stdout+suite.stderr);assert suite.returncode==0
        check=subprocess.run([sys.executable,'-B','-c',"from release_label import parse_label; assert parse_label('chatgpt-abc-z2-s9223372036854775807')==('abc-z2',9223372036854775807)"],cwd=workspace,env=batch.env,capture_output=True,timeout=15);assert check.returncode==0
        diff=subprocess.run(['git','diff','--check'],cwd=workspace,env=batch.env,capture_output=True);assert diff.returncode==0
        patch=subprocess.run(['git','diff','--binary','HEAD'],cwd=workspace,env=batch.env,capture_output=True,check=True);(root/(record['id']+'-diff.patch')).write_bytes(patch.stdout)
        cli=root/(record['id']+'-original-cli.txt');assert cli.is_file()
        finals=[v['item']['text'] for v in history['items'] if v['item'].get('type')=='agentMessage' and v['item'].get('phase')=='final_answer']
        assert finals and all(cli_final_visible(cli.read_text(errors='replace'),value) for value in finals),'original CLI final mismatch: '+record['id']
        assert len(list(workspace.glob('test_*.py')))>=2,'meaningful added regression suite required'
        saved=(status.get('managed_lane') or {}).get('conversation_id')
        if saved:
            pages=batch.read(saved);save(root/(record['id']+'-saved-chat-private.json'),pages)
            texts=['\n'.join(p for p in m['content'].get('parts',[]) if isinstance(p,str))
                   for page in pages for m in page.get('messages',[])
                   if m.get('role')=='assistant' and isinstance(m.get('content'),dict)]
            # Saved rich presentation may retain box wrappers omitted by the
            # Provider's presentation projection; retain every final text in order.
            assert all(any(cli_final_visible(text,final) for text in texts) for final in finals),'saved original final unavailable: '+record['id']
        reports.append(dict(work_id=record['id'],thread_id=record['thread'],workspace=str(workspace),native_final_in_original_cli=True,tests=True))
    assert len(reports)==len(batch.plan.get('cases',['managed-mcp','managed-local','direct','general']))
    return reports


def prepare_cold(root, prior_requests=0):
    """Arm one new task on the settled current-reasoning work after host restart."""
    assert not (root/'plan.json').exists()
    assert isinstance(prior_requests,int) and 0<=prior_requests<4
    request_ceiling=4-prior_requests
    prior=explicit_input('PRIOR_BATCH')
    target=next(w for w in native_work(root) if w['selection']['reasoning']=='current')
    assert target['phase']=='stopped' and not target['pending']
    shutil.copytree(prior/'candidate',root/'candidate')
    shutil.copy2(SOURCE/'scripts/d3_notify.py',root/'notify.py')
    (root/'run.py').write_text(Path(__file__).read_text())
    (root/'client.mjs').write_text((SOURCE/'scripts/chat_entry_client.mjs').read_text().replace("'../providers/chatgpt/desktop_cdp.mjs'",json.dumps(str(SOURCE/'providers/chatgpt/desktop_cdp.mjs'))))
    info=json.loads((root/'info.json').read_text())
    require_connector_ready(info,json.loads((root/'refresh-private.json').read_text()))
    plan=json.loads((prior/'plan.json').read_text())
    plan.update(cold_followup=True,desktop_session=info['desktop_session'],
                input_client_id=input_client_id(root),deadline_unix_ms=int(time.time()*1000)+1200000,
                max_elapsed_seconds=1200,max_source_requests=request_ceiling,max_model_responses=request_ceiling,max_control_observations=400,
                prior_cold_requests=prior_requests,total_cold_request_ceiling=4,
                target_work=target['id'],source_conversation=json.loads((prior/'direct-source-private.json').read_text())['conversation_id'],
                completed_prior_work={w['id']:{} for w in native_work(root) if w['id']!=target['id']})
    save(root/'plan.json',plan,True)
    workspace=Path(target['workspace'])
    baseline={p.name:sha(p) for p in workspace.glob('*.py')}
    save(root/'baseline-files.json',baseline,True)
    shutil.copy2(workspace/'release_label.py',root/'release_label-before.py')
    task='''User-authorized G Termux-process cold continuation: this is a NEW follow-up on the existing work and workspace, not a replay. Keep test_original.py and test_regression.py byte-for-byte and retain the existing parser/CLI contract. Add test_format_label.py first and demonstrate failure because format_label is missing. Implement release_label.format_label(name, sequence), producing the canonical chatgpt-NAME-sSEQUENCE accepted by parse_label. Validate NAME by the existing name contract and sequence as an int (never bool), 1..2**63-1; reject invalid types/names/sequences with ValueError. Add meaningful inverse/invalid/boundary regression tests; run all tests, check actual diff and original hashes. Use only native Codex tools, no Work/network/packages/computer use. Report real results and elapsed time. Publish the complete final through codex_finish and this same saved Chat; end both with G_COLD_FOLLOWUP_COMPLETE.'''
    (root/'task-cold.txt').write_text(task+'\n')
    files=[root/name for name in ['run.py','notify.py','client.mjs','info.json','environment-private.json','plan.json','baseline-files.json','release_label-before.py','task-cold.txt','refresh-private.json']]
    files+=list((root/'candidate').iterdir())
    files+=[SOURCE/name for name in ['scripts/provider_qualification.py','scripts/qualify_provider_f.py','scripts/qualify_provider_chat_entry.py','scripts/qualify_tasks.py','providers/chatgpt/desktop_cdp.mjs']]
    save(root/'sealed-inputs.json',{str(p):sha(p) for p in files if p.is_file()},True)
    save(root/'connection-ready.json',dict(ready=True,Chat_requests=0,recovered_same_endpoint=True),True)
    print(json.dumps(dict(root=str(root),prepared=True,model_requests=0,additional_dispatch_ceiling=request_ceiling)))


def cold(root):
    save(root/'run-started.json',dict(pid=os.getpid(),**stamp()),True)
    result=dict(automated_gates=False,scope='Termux restart: same saved Chat and native work, new follow-up',physical_reboot=False,started=stamp(),supervisor_calls_between_tasks=0)
    batch=None
    try:
        batch=Batch(root);batch.check();batch.client.call('init');batch.client.call('release_source')
        wid=batch.plan['target_work'];record=next(w for w in native_work(root) if w['id']==wid)
        history=batch.rpc('codex_history',{'work_id':wid});save(root/'before-history-private.json',history,True)
        # Current-reasoning extension preserves the original start/counters/caps.
        budget=cold_budget(record,batch.plan['deadline_unix_ms'])
        claimed=batch.session(action='claim',work_id=wid,call_id='g-cold-claim-'+input_client_id(root),cursor=history['cursor'],context_cursor=history['context_cursor'],budget=budget)
        save(root/'claim-private.json',claimed,True)
        task=(root/'task-cold.txt').read_text();batch.start('cold-followup',claimed,task)
        pages=batch.read(batch.plan['source_conversation'])
        branch=followup_branch(pages);pages=batch.read(batch.plan['source_conversation'],branch=branch)
        save(root/'saved-source-before-private.json',pages,True)
        instruction='The genuine follow-up task below has ALREADY been started once on the original native work. Do not create another work or call start again. Attach with work_id='+wid+' and controller='+claimed['controller']+'. Echo the exact returned receipt as accept before effects, then discover and use native tools. Finish the original native work and this saved Chat.\n\n'+task
        args=dict(action='chat_input',conversation_id=batch.plan['source_conversation'],source_cursor=pages[-1]['cursor'],task=instruction,call_id='g-cold-input-'+input_client_id(root),budget=dict(max_app_starts=batch.plan['max_source_requests'],max_elapsed_seconds=900))
        save(root/'saved-input-intent-private.json',args,True)
        submitted=batch.session(**args);save(root/'saved-input-private.json',submitted,True)
        while True:
            batch.check();follow=batch.session(action='chat_result',input_id=submitted['input_id'],max_wait_seconds=3);batch.drain()
            if follow.get('observation_unavailable'):
                batch.wait_for_saved_retry(follow['observation_unavailable'].get('retry_after_seconds',5),submitted['input_id'])
            error=(follow.get('observation') or {}).get('error')
            if error=='mcp_host_confirmation_required':batch.client.call('approve_saved',{'input_id':submitted['input_id']})
            elif error:raise RuntimeError('cold_source_error_preserved_no_replay:'+str(error))
            if input_completed(follow):break
            batch.observe_current()
        save(root/'saved-result-private.json',follow,True)
        status=batch.rpc('codex_status',{'work_id':wid});assert status['completed'] and not status['pending_calls'] and not status['human_requests']
        history=batch.rpc('codex_history',{'work_id':wid});save(root/'final-history-private.json',history,True)
        for _ in range(20):batch.drain();time.sleep(.05)
        finals=[v['item']['text'] for v in history['items'] if v['item'].get('type')=='agentMessage' and v['item'].get('phase')=='final_answer' and 'G_COLD_FOLLOWUP_COMPLETE' in v['item']['text']]
        assert finals and all(cli_final_visible((root/(wid+'-original-cli.txt')).read_text(errors='replace'),text) for text in finals)
        workspace=Path(record['workspace']);baseline=json.loads((root/'baseline-files.json').read_text())
        for name in ['test_original.py','test_regression.py']:assert sha(workspace/name)==baseline[name]
        assert (workspace/'test_format_label.py').is_file()
        tests=subprocess.run([sys.executable,'-B','-m','unittest','discover','-v'],cwd=workspace,env=batch.env,capture_output=True,timeout=45);(root/'final-tests.log').write_bytes(tests.stdout+tests.stderr);assert tests.returncode==0
        subprocess.run(['git','diff','--check'],cwd=workspace,env=batch.env,check=True,capture_output=True)
        (root/'final-diff.patch').write_bytes(subprocess.check_output(['git','diff','--binary','HEAD'],cwd=workspace,env=batch.env))
        batch.check(force=True);result.update(automated_gates=True,work_id=wid,thread_id=record['thread'],same_saved_conversation=batch.plan['source_conversation'],original_tests_preserved=True,native_final_in_original_cli=True)
    except Exception as error:
        (root/'failure-traceback-private.txt').write_text(traceback.format_exc());result['failure']=dict(type=type(error).__name__,message=str(error)[:400],unknown_requests_replayed=False)
    finally:
        if batch:
            result['control_observations']=batch.control_observations
            if not result['automated_gates']:
                try:save(root/'failure-stop-private.json',control(batch.info,batch.env,'codex_stop',{'work_id':batch.plan['target_work']}),True)
                except Exception:result['stop']='unconfirmed_preserved'
                try:batch.client.call('stop_inputs')
                except Exception:pass
            try:
                snapshot=batch.client.call('snapshot');save(root/'final-source-snapshot-private.json',snapshot,True);result['app_requests']=len(snapshot.get('requests',[]));result['cohort_app_requests']=result['app_requests']+batch.plan.get('prior_cold_requests',0);batch.client.call('dispose')
            except Exception:result['source_cleanup']='preserved'
            try:batch.close()
            except Exception:result['observer_cleanup']='preserved'
        result['ended']=stamp();save(root/'result.json',result,True)
        (root/'turns.md').write_text('# G Termux cold follow-up\n\n'+json.dumps(result,ensure_ascii=False,indent=2)+'\n')
        if (root/'notification-binding.json').exists():
            from d3_notify import send
            send(root)


def run(root):
    if json.loads((root/'plan.json').read_text()).get('cold_followup'):return cold(root)
    save(root/'run-started.json',dict(pid=os.getpid(),**stamp()),True)
    result=dict(automated_gates=False,scope='G installed immutable Core/Provider three entries plus explicit local alternative',started=stamp(),supervisor_calls_between_tasks=0)
    batch=None
    try:
        batch=Batch(root);batch.check();batch.client.call('init')
        cases=batch.plan.get('cases',['managed-mcp','managed-local','direct','general'])
        if 'managed-mcp' in cases:batch.managed('managed-mcp',TASK,workspace=root/'w-managed-mcp')
        if 'managed-local' in cases:batch.managed('managed-local',TASK,route='local_function',workspace=root/'w-managed-local')
        for step,name in enumerate(['direct','general'],1):
            if name not in cases:continue
            batch.source_task(name,(root/('task-'+name+'.txt')).read_text(),'G_'+name.upper()+'_COMPLETE',step)
            batch.client.call('release_source')
        for _ in range(20):batch.drain();time.sleep(.05)
        result['audit']=audit(root,batch);batch.check(force=True);result['automated_gates']=True
    except Exception as error:
        (root/'failure-traceback-private.txt').write_text(traceback.format_exc())
        result['failure']=dict(type=type(error).__name__,message=str(error)[:400],unknown_requests_replayed=False)
    finally:
        if batch:
            result['steps']=batch.steps;result['control_observations']=batch.control_observations
            if not result['automated_gates']:
                stops=[]
                for record in native_work(root):
                    if record['id'] in batch.plan.get('completed_prior_work',{}):continue
                    if record['phase'] in ('completed','stopped','stop_requested'):continue
                    try:stops.append(control(batch.info,batch.env,'codex_stop',{'work_id':record['id']}))
                    except Exception:stops.append(dict(work_id=record['id'],state='unconfirmed_preserved'))
                save(root/'failure-stops-private.json',stops)
                try:batch.client.call('stop_inputs')
                except Exception:pass
            try:
                snapshot=batch.client.call('snapshot');save(root/'final-source-snapshot-private.json',snapshot)
                result['app_requests']=len(snapshot.get('requests',[]));batch.client.call('dispose')
            except Exception:result['source_cleanup']='preserved'
            try:batch.close()
            except Exception:result['observer_cleanup']='preserved'
        result['ended']=stamp();save(root/'result.json',result)
        (root/'turns.md').write_text('# G installed product turns\n\n'+''.join('## '+v['name']+'\n\n'+str(v['elapsed_seconds'])+' seconds\n\n' for v in result.get('steps',[])))
        if (root/'notification-binding.json').exists():
            from d3_notify import send
            send(root)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('action',choices=('prepare','prepare-remaining','prepare-cold','connect','start','run'));parser.add_argument('root',type=Path,nargs='?')
    for name in ('core','builder','provider','frontend','generation'):parser.add_argument('--'+name,type=Path)
    parser.add_argument('--prior-cold-requests',type=int,default=0)
    args=parser.parse_args()
    if args.action=='prepare':prepare(args);return
    root=args.root.resolve(strict=True)
    if args.action=='prepare-cold':prepare_cold(root,args.prior_cold_requests);return
    if args.action=='prepare-remaining':prepare_remaining(root);return
    if args.action=='connect':connect(root);return
    if args.action=='start':
        assert (root/'connection-ready.json').exists() and not (root/'launch-intent.json').exists() and not (root/'run-started.json').exists()
        plan=json.loads((root/'plan.json').read_text())
        assert time.time()*1000 < plan.get('deadline_unix_ms',float('inf')),'expired batch intent'
        for name,digest in json.loads((root/'sealed-inputs.json').read_text()).items():
            assert sha(root/name)==digest,'sealed input changed before launch'
        save(root/'launch-intent.json',stamp(),True)
        child=subprocess.Popen([sys.executable,'-B',str(root/'run.py'),'run',str(root)],stdin=subprocess.DEVNULL,stdout=(root/'run.stdout').open('ab'),stderr=(root/'run.stderr').open('ab'),start_new_session=True)
        print(json.dumps(dict(started=True,pid=child.pid,root=str(root))));return
    run(root)


if __name__=='__main__':main()
