#!/usr/bin/env python3
"""Nonzero proof of the actual one-shot Manager delivery workflow."""
from pathlib import Path
import hashlib,os,shutil,subprocess,sys,tempfile,textwrap,unittest

WORKFLOW=Path(__file__).parents[1]/'workflows/auto-release-termux.yml'
TEXT=WORKFLOW.read_text()
SOURCE='af1a3df6d99a01b5c649f0f47beef6703128feab'
PARENT='3da4b294e1a6d62b1b5ba775acf88aae375c3189'
CURRENT='local-hosted-0-160-0-705afb98098f-active-task-handoff'
PROTECTED='local-hosted-0-160-0-6523921a1d33-permission-memory'
NEW='local-hosted-0-160-0-af1a3df6d99a-active-task-handoff-r2'
MESSAGE='release-owner-handoff-seq38-source-af1a3df6'

def body(name):
 block=TEXT.split('      - name: '+name+'\n',1)[1].split('      - name:',1)[0]
 return textwrap.dedent(block.split('        run: |\n',1)[1])

class DeliveryTests(unittest.TestCase):
 def test_authorization_source_and_nonforce_cas(self):
  predicate=f"github.event_name == 'push' && github.ref == 'refs/heads/main' && github.event.before == '{PARENT}' && github.event.head_commit.message == '{MESSAGE}'"
  self.assertEqual(TEXT.count(predicate),3)
  expression=next(line.split('${{',1)[1].split('}}',1)[0] for line in TEXT.splitlines() if 'ACTIVE_TASK_RELEASE_PUSH_BRIDGE: ${{' in line)
  names=['github.event_name','github.ref','github.event.before','github.event.head_commit.message']
  valid=['push','refs/heads/main',PARENT,MESSAGE]
  def allowed(values):
   code=expression
   for k,v in zip(names,values,strict=True):code=code.replace(k,repr(v))
   return eval(code.replace('&&','and'),{'__builtins__':{}},{})
  self.assertTrue(allowed(valid))
  for i in range(4):
   values=list(valid);values[i]='wrong';self.assertFalse(allowed(values))
  for key in ['CODEX_SOURCE_SHA','RALD5_SOURCE_SHA','ACTIVE_TASK_RELEASE_ACCEPTED_SOURCE_SHA']:self.assertIn(f"{key}: '{SOURCE}'",TEXT)
  self.assertIn('force:false',TEXT);self.assertNotIn('force: true',TEXT)
  self.assertNotIn('PERMISSION_MEMORY_RELEASE_PUSH_BRIDGE',TEXT)

 def test_actual_admission_block(self):
  start=TEXT.index('          test "$ACTIVE_TASK_RELEASE_PUSH_BRIDGE" = true -o')
  end=TEXT.index('          if test "$RALD4_POSITIVE_GATE" = true; then',start)
  code=textwrap.dedent(TEXT[start:end])
  stable={'current_version':'0.160.0','current_generation':CURRENT,'current_release_sequence':'37','release_sequence':'38'}
  for k in stable:code=code.replace("'${{ steps.stable.outputs."+k+" }}'",'"$STABLE_'+k+'"')
  script='set -euo pipefail\n'+code+'\ntest "$candidate" = true\ntest "$acceptance_suffix" = -active-task-handoff-r2\n'
  flags=['RALD4_POSITIVE_GATE','RALD5_SAME_VERSION_ACCEPTANCE','RALD45_TRANSITION_STAGE','RALD45_TRANSITION_PROMOTE','RALD5_NEGATIVE_GATE','LEGACY_LAG_JUMP_REMEDIATION','UX1_SAME_VERSION_DEPLOY','UPDATE_PROGRESS_SAME_VERSION_DEPLOY','EXACT_CURRENT_FASTPATH_SAME_VERSION_DEPLOY','NO_EMOJI_SAME_VERSION_DEPLOY','DOWNLOAD_SIZE_SAME_VERSION_DEPLOY']
  env={**os.environ,**{k:'false' for k in flags},**{'STABLE_'+k:v for k,v in stable.items()},'ACTIVE_TASK_RELEASE_PUSH_BRIDGE':'true','GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/main','RALD5_PUBLICATION_AUTHORIZED':'true','CODEX_SOURCE_SHA':SOURCE,'ACTIVE_TASK_RELEASE_ACCEPTED_SOURCE_SHA':SOURCE,'ACTIVE_TASK_RELEASE_TARGET_VERSION':'0.160.0','ACTIVE_TASK_RELEASE_CURRENT_GENERATION':CURRENT,'ACTIVE_TASK_RELEASE_CURRENT_SEQUENCE':'37','ACTIVE_TASK_RELEASE_TARGET_SEQUENCE':'38','ACTIVE_TASK_RELEASE_TARGET_ARCHIVE_SHA256':'7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c','upstream_version':'0.160.0','archive_sha256':'7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c','candidate':'false'}
  self.assertEqual(subprocess.run(['bash','-c',script],env=env,capture_output=True).returncode,0)
  faults={'ACTIVE_TASK_RELEASE_PUSH_BRIDGE':'false','CODEX_SOURCE_SHA':'0'*40,'GITHUB_EVENT_NAME':'schedule','GITHUB_REF':'refs/heads/other','RALD5_PUBLICATION_AUTHORIZED':'false','candidate':'true','upstream_version':'0.161.0','archive_sha256':'0'*64,**{'STABLE_'+k:'wrong' for k in stable},**{k:'true' for k in flags}}
  for k,v in faults.items():
   with self.subTest(field=k):self.assertNotEqual(subprocess.run(['bash','-c',script],env={**env,k:v},capture_output=True).returncode,0)

 def test_protected_generation_authentication_rejects_substitution(self):
  code=body('Authenticate exact protected sequence36 generation')
  for fault in [None,'signature','manifest','descriptor','other-signed-generation','sequence','identity','key','mode']:
   with self.subTest(fault=fault),tempfile.TemporaryDirectory(prefix='protected-release-') as temp:
    root=Path(temp);stable=root/'rald3-stable';stable.mkdir();download=root/'downloads';download.mkdir()
    helper=root/'source/.github/scripts/rald3_preflight.py';helper.parent.mkdir(parents=True)
    helper.write_bytes(subprocess.check_output(['git','show',SOURCE+':.github/scripts/rald3_preflight.py'],cwd=WORKFLOW.parent))
    private=root/'fixture-private.pem';public=stable/'update-public-key.pem'
    subprocess.run(['openssl','genpkey','-algorithm','Ed25519','-out',str(private)],check=True,capture_output=True)
    subprocess.run(['openssl','pkey','-in',str(private),'-pubout','-out',str(public)],check=True,capture_output=True)
    identity=CURRENT if fault=='other-signed-generation' else PROTECTED
    meta=download/'generation.meta';meta.write_text('codex-local-generation-v2\n'+'generation_id\t'+identity+'\n')
    headers=[('generation_id',identity),('release_sequence','36'),('channel','stable'),('expected_platform','android'),('expected_architecture','aarch64'),('core_api_identity','core-api-v1'),('persistent_schema_identity','schema-v1'),('release_public_key','11'*32),('file_count','1')]
    manifest=download/'release.manifest';manifest.write_text('codex-release-v4\n'+''.join(k+'\t'+v+'\n' for k,v in headers)+'file\tgeneration.meta\t'+hashlib.sha256(meta.read_bytes()).hexdigest()+'\t'+('0755' if fault=='mode' else '0644')+'\n')
    subprocess.run(['openssl','pkeyutl','-sign','-rawin','-inkey',str(private),'-in',str(manifest),'-out',str(download/'release.sig')],check=True,capture_output=True)
    if fault=='signature':(download/'release.sig').write_bytes(b'x'*64)
    if fault=='manifest':manifest.write_bytes(b'changed')
    if fault=='descriptor':meta.write_bytes(b'changed')
    if fault=='key':public.write_bytes(b'wrong')
    tools=root/'bin';tools.mkdir();curl=tools/'curl'
    curl.write_text('#!'+sys.executable+'\nimport os,sys,shutil\nfrom pathlib import Path\na=sys.argv[1:]\nshutil.copyfile(Path(os.environ["FIXTURE_DOWNLOADS"])/a[-1].rsplit("/",1)[1],a[a.index("--output")+1])\n');curl.chmod(0o755)
    env={**os.environ,'RUNNER_TEMP':str(root),'PATH':str(tools)+os.pathsep+os.environ['PATH'],'FIXTURE_DOWNLOADS':str(download),'ACTIVE_TASK_RELEASE_PROTECTED_GENERATION':'wrong' if fault=='identity' else PROTECTED,'ACTIVE_TASK_RELEASE_PROTECTED_SEQUENCE':'37' if fault=='sequence' else '36','PYTHONDONTWRITEBYTECODE':'1'}
    result=subprocess.run(['bash','-c',code],cwd=root,env=env,capture_output=True,timeout=20)
    self.assertEqual(result.returncode==0,fault is None)

 def component(self,fault=None):
  with tempfile.TemporaryDirectory(prefix='active-task-release-') as temp:
   root=Path(temp);stable=root/'rald3-protected';new=root/'rald3-candidate/candidate';stable.mkdir();new.mkdir(parents=True)
   helper=root/'source/.github/scripts/rald3_preflight.py';helper.parent.mkdir(parents=True)
   helper.write_bytes(subprocess.check_output(['git','show',SOURCE+':.github/scripts/rald3_preflight.py'],cwd=WORKFLOW.parent))
   inventory=[]
   for rel in ['core','runtime','codex-code-mode-host','helpers/0','helpers/1','manager']:
    p=new/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(rel.encode());p.chmod(0o755)
    inventory.append((rel,hashlib.sha256(p.read_bytes()).hexdigest(),'0755'))
   digest=hashlib.sha256((new/'manager').read_bytes()).hexdigest()
   old=[('generation_id',PROTECTED),('manager_artifact_digest','0'*64),('core_artifact_digest','a'*64),('patch_policy_id','termux-fd-remap-v3'),('runtime_digest','b'*64),('helper','one'),('helper','two')]
   current=[(k,NEW if k=='generation_id' else digest if k=='manager_artifact_digest' else v) for k,v in old]
   if fault:
    kind,key=fault
    if kind=='field':current=[(k,'wrong' if k==key else v) for k,v in current]
    elif kind=='old':old=[(k,'wrong' if k==key else v) for k,v in old]
    elif kind=='duplicate':old.append(('manager_artifact_digest','0'*64));current.append(('manager_artifact_digest',digest))
    elif kind=='order':current.reverse()
    elif kind=='same':current=[(k,'0'*64 if k=='manager_artifact_digest' else v) for k,v in current]
    elif kind=='bytes':(new/key).write_bytes(b'changed')
    elif kind=='mode':(new/key).chmod(0o644)
   for path,rows in [(stable/'generation.meta',old),(new/'generation.meta',current)]:path.write_text('codex-local-generation-v2\n'+''.join(k+'\t'+v+'\n' for k,v in rows))
   if fault and fault[0]=='framing':
    p=new/'generation.meta';p.write_bytes(p.read_bytes().rstrip(b'\n'))
   headers=[('generation_id',PROTECTED),('release_sequence','36'),('channel','stable'),('expected_platform','android'),('expected_architecture','aarch64'),('core_api_identity','core-api-v1'),('persistent_schema_identity','schema-v1'),('release_public_key','11'*32),('file_count',str(len(inventory)))]
   (stable/'release.manifest').write_text('codex-release-v4\n'+''.join(k+'\t'+v+'\n' for k,v in headers)+''.join('file\t'+'\t'.join(row)+'\n' for row in sorted(inventory)))
   code=body('Bind active-task Manager-only release to exact protected payloads').replace('${{ steps.decision.outputs.generation_id }}',NEW)
   return subprocess.run(['bash','-c',code],cwd=root,env={**os.environ,'RUNNER_TEMP':str(root),'PYTHONDONTWRITEBYTECODE':'1'},capture_output=True).returncode

 def test_actual_manager_only_delta(self):self.assertEqual(self.component(),0)
 def test_changed_payload_bytes_and_modes_rejected(self):
  for rel in ['core','runtime','codex-code-mode-host','helpers/0','helpers/1','manager']:
   for kind in ['bytes','mode']:
    with self.subTest(path=rel,fault=kind):self.assertNotEqual(self.component((kind,rel)),0)
 def test_descriptor_mismatch_duplicate_order_and_framing_rejected(self):
  faults=[('field',k) for k in ['generation_id','manager_artifact_digest','core_artifact_digest','patch_policy_id','runtime_digest','helper']]+[('old','generation_id'),('duplicate',''),('order',''),('same',''),('framing','')]
  for fault in faults:
   with self.subTest(fault=fault):self.assertNotEqual(self.component(fault),0)
 def test_core_descriptor_preservation_requires_unique_binding(self):
  code=body('Preserve authenticated stable Core for Manager-only delivery').split("<<'PYCORE'\n",1)[1].split('PYCORE\n',1)[0]
  with tempfile.TemporaryDirectory(prefix='core-binding-') as temp:
   root=Path(temp);core=root/'core';core.write_bytes(b'authenticated stable Core');meta=root/'generation.meta'
   for count in [1,0,2]:
    meta.write_text('codex-local-generation-v2\n'+'core_artifact_digest\t'+'0'*64+'\n' if count==1 else 'codex-local-generation-v2\n'+('core_artifact_digest\t'+'0'*64+'\n')*count)
    result=subprocess.run([sys.executable,'-',str(meta),str(core)],input=code,text=True,capture_output=True)
    self.assertEqual(result.returncode==0,count==1)
    if count==1:self.assertIn(hashlib.sha256(core.read_bytes()).hexdigest(),meta.read_text())

 def test_preservation_and_noop_routes_remain_bound(self):
  code=body('Preserve authenticated stable Core for Manager-only delivery')
  self.assertIn('manifest-entry "$stable/release.manifest" core',code)
  self.assertIn('sha256sum "$stable/core"',code)
  self.assertIn('cp "$stable/core" "$candidate/core"',code)
  self.assertIn('needs.producer.outputs.active_task_release_push_bridge',TEXT)
  self.assertIn('snapshot > "$RUNNER_TEMP/rald5-after-noop"',TEXT)

if __name__=='__main__':unittest.main()
