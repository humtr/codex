#!/usr/bin/env python3
"""Nonzero proof of the actual one-shot coordinated Core and Manager delivery workflow."""
from pathlib import Path
import hashlib,os,shutil,subprocess,sys,tempfile,textwrap,unittest

WORKFLOW=Path(__file__).parents[1]/'workflows/auto-release-termux.yml'
TEXT=WORKFLOW.read_text()
SOURCE='59308430d39dc3c57159e260e5f7a873c1b16b8d'
PARENT='471590750468f6d99b520c3a8d47b03a474aacbb'
CURRENT='local-hosted-0-160-0-b7c1e61019b1-retired-server-discovery'
NEW='local-hosted-0-160-0-59308430d39d-profile-lifecycle'
MESSAGE='release-profile-lifecycle-seq40-source-59308430'

def body(name):
 block=TEXT.split('      - name: '+name+'\n',1)[1].split('      - name:',1)[0]
 return textwrap.dedent(block.split('        run: |\n',1)[1])

class DeliveryTests(unittest.TestCase):
 def test_authorization_source_and_nonforce_cas(self):
  predicate=f"github.event_name == 'push' && github.ref == 'refs/heads/main' && github.event.before == '{PARENT}' && github.event.head_commit.message == '{MESSAGE}'"
  self.assertEqual(TEXT.count(predicate),3)
  expression=next(line.split('${{',1)[1].split('}}',1)[0] for line in TEXT.splitlines() if 'PROFILE_RELEASE_PUSH_BRIDGE: ${{' in line)
  names=['github.event_name','github.ref','github.event.before','github.event.head_commit.message']
  valid=['push','refs/heads/main',PARENT,MESSAGE]
  def allowed(values):
   code=expression
   for k,v in zip(names,values,strict=True):code=code.replace(k,repr(v))
   return eval(code.replace('&&','and'),{'__builtins__':{}},{})
  self.assertTrue(allowed(valid))
  for i in range(4):
   values=list(valid);values[i]='wrong';self.assertFalse(allowed(values))
  for key in ['CODEX_SOURCE_SHA','RALD5_SOURCE_SHA','PROFILE_RELEASE_ACCEPTED_SOURCE_SHA']:self.assertIn(f"{key}: '{SOURCE}'",TEXT)
  self.assertIn('force:false',TEXT);self.assertNotIn('force: true',TEXT)
  self.assertNotIn('PERMISSION_MEMORY_RELEASE_PUSH_BRIDGE',TEXT)

 def test_actual_admission_block(self):
  start=TEXT.index('          test "$PROFILE_RELEASE_PUSH_BRIDGE" = true -o')
  end=TEXT.index('          if test "$RALD4_POSITIVE_GATE" = true; then',start)
  code=textwrap.dedent(TEXT[start:end])
  stable={'current_version':'0.160.0','current_generation':CURRENT,'current_release_sequence':'39','release_sequence':'40'}
  for k in stable:code=code.replace("'${{ steps.stable.outputs."+k+" }}'",'"$STABLE_'+k+'"')
  script='set -euo pipefail\n'+code+'\ntest "$candidate" = true\ntest "$acceptance_suffix" = -profile-lifecycle\n'
  flags=['RALD4_POSITIVE_GATE','RALD5_SAME_VERSION_ACCEPTANCE','RALD45_TRANSITION_STAGE','RALD45_TRANSITION_PROMOTE','RALD5_NEGATIVE_GATE','LEGACY_LAG_JUMP_REMEDIATION','UX1_SAME_VERSION_DEPLOY','UPDATE_PROGRESS_SAME_VERSION_DEPLOY','EXACT_CURRENT_FASTPATH_SAME_VERSION_DEPLOY','NO_EMOJI_SAME_VERSION_DEPLOY','DOWNLOAD_SIZE_SAME_VERSION_DEPLOY']
  env={**os.environ,**{k:'false' for k in flags},**{'STABLE_'+k:v for k,v in stable.items()},'PROFILE_RELEASE_PUSH_BRIDGE':'true','GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/main','RALD5_PUBLICATION_AUTHORIZED':'true','CODEX_SOURCE_SHA':SOURCE,'PROFILE_RELEASE_ACCEPTED_SOURCE_SHA':SOURCE,'PROFILE_RELEASE_TARGET_VERSION':'0.160.0','PROFILE_RELEASE_CURRENT_GENERATION':CURRENT,'PROFILE_RELEASE_CURRENT_SEQUENCE':'39','PROFILE_RELEASE_TARGET_SEQUENCE':'40','PROFILE_RELEASE_TARGET_ARCHIVE_SHA256':'7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c','upstream_version':'0.160.0','archive_sha256':'7f0fe42ff22ecfa3a47bc4a34f5b22c4218b431a4ec0aba51c7d98299f07900c','candidate':'false'}
  self.assertEqual(subprocess.run(['bash','-c',script],env=env,capture_output=True).returncode,0)
  faults={'PROFILE_RELEASE_PUSH_BRIDGE':'false','CODEX_SOURCE_SHA':'0'*40,'GITHUB_EVENT_NAME':'schedule','GITHUB_REF':'refs/heads/other','RALD5_PUBLICATION_AUTHORIZED':'false','candidate':'true','upstream_version':'0.161.0','archive_sha256':'0'*64,**{'STABLE_'+k:'wrong' for k in stable},**{k:'true' for k in flags}}
  for k,v in faults.items():
   with self.subTest(field=k):self.assertNotEqual(subprocess.run(['bash','-c',script],env={**env,k:v},capture_output=True).returncode,0)

 def component(self,fault=None):
  with tempfile.TemporaryDirectory(prefix='active-task-release-') as temp:
   root=Path(temp);stable=root/'rald3-stable';new=root/'rald3-candidate/candidate';stable.mkdir();new.mkdir(parents=True)
   helper=root/'source/.github/scripts/rald3_preflight.py';helper.parent.mkdir(parents=True)
   helper.write_bytes(subprocess.check_output(['git','show',SOURCE+':.github/scripts/rald3_preflight.py'],cwd=WORKFLOW.parent))
   inventory=[]
   for rel in ['core','runtime','codex-code-mode-host','helpers/0','helpers/1','manager']:
    p=new/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(rel.encode());p.chmod(0o755)
    inventory.append((rel,hashlib.sha256(p.read_bytes()).hexdigest(),'0755'))
   digest=hashlib.sha256((new/'manager').read_bytes()).hexdigest()
   core_digest=hashlib.sha256((new/'core').read_bytes()).hexdigest()
   old=[('generation_id',CURRENT),('manager_artifact_digest','0'*64),('core_artifact_digest','a'*64),('patch_policy_id','termux-fd-remap-v3'),('runtime_digest','b'*64),('helper','one'),('helper','two')]
   current=[(k,NEW if k=='generation_id' else digest if k=='manager_artifact_digest' else core_digest if k=='core_artifact_digest' else v) for k,v in old]
   if fault:
    kind,key=fault
    if kind=='field':current=[(k,'wrong' if k==key else v) for k,v in current]
    elif kind=='old':old=[(k,'wrong' if k==key else v) for k,v in old]
    elif kind=='duplicate':old.append(('manager_artifact_digest','0'*64));current.append(('manager_artifact_digest',digest))
    elif kind=='duplicatecore':old.append(('core_artifact_digest','a'*64));current.append(('core_artifact_digest',core_digest))
    elif kind=='samecore':old=[(k,core_digest if k=='core_artifact_digest' else v) for k,v in old]
    elif kind=='order':current.reverse()
    elif kind=='same':current=[(k,'0'*64 if k=='manager_artifact_digest' else v) for k,v in current]
    elif kind=='bytes':(new/key).write_bytes(b'changed')
    elif kind=='mode':(new/key).chmod(0o644)
   for path,rows in [(stable/'generation.meta',old),(new/'generation.meta',current)]:path.write_text('codex-local-generation-v2\n'+''.join(k+'\t'+v+'\n' for k,v in rows))
   if fault and fault[0]=='framing':
    p=new/'generation.meta';p.write_bytes(p.read_bytes().rstrip(b'\n'))
   headers=[('generation_id',CURRENT),('release_sequence','36'),('channel','stable'),('expected_platform','android'),('expected_architecture','aarch64'),('core_api_identity','core-api-v1'),('persistent_schema_identity','schema-v1'),('release_public_key','11'*32),('file_count',str(len(inventory)))]
   (stable/'release.manifest').write_text('codex-release-v4\n'+''.join(k+'\t'+v+'\n' for k,v in headers)+''.join('file\t'+'\t'.join(row)+'\n' for row in sorted(inventory)))
   code=body('Bind profile Core and Manager release to authenticated stable payloads').replace('${{ steps.decision.outputs.generation_id }}',NEW)
   return subprocess.run(['bash','-c',code],cwd=root,env={**os.environ,'RUNNER_TEMP':str(root),'PYTHONDONTWRITEBYTECODE':'1'},capture_output=True).returncode

 def test_actual_core_and_manager_delta(self):self.assertEqual(self.component(),0)
 def test_changed_payload_bytes_and_modes_rejected(self):
  for rel in ['core','runtime','codex-code-mode-host','helpers/0','helpers/1','manager']:
   for kind in ['bytes','mode']:
    with self.subTest(path=rel,fault=kind):self.assertNotEqual(self.component((kind,rel)),0)
 def test_descriptor_mismatch_duplicate_order_and_framing_rejected(self):
  faults=[('field',k) for k in ['generation_id','manager_artifact_digest','core_artifact_digest','patch_policy_id','runtime_digest','helper']]+[('old','generation_id'),('duplicate',''),('duplicatecore',''),('samecore',''),('order',''),('same',''),('framing','')]
  for fault in faults:
   with self.subTest(fault=fault):self.assertNotEqual(self.component(fault),0)
 def test_no_stable_core_substitution_and_noop_routes(self):
  self.assertNotIn('cp "$stable/core" "$candidate/core"',TEXT)
  self.assertNotIn('rald3-protected',TEXT)
  self.assertIn('needs.producer.outputs.profile_release_push_bridge',TEXT)
  self.assertIn('snapshot > "$RUNNER_TEMP/rald5-after-noop"',TEXT)

if __name__=='__main__':unittest.main()
