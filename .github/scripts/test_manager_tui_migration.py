"""Exercise the documented bridge procedure; Core owns actual update admission."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import test_rald5_publication as fixtures

ROOT=Path(__file__).resolve().parents[2]
BRIDGE='local-hosted-0-161-0-ab644771ac89-manager-tui-bridge'


class MigrationProcedureTests(unittest.TestCase):
    def test_actual_readme_materializes_exact_signed_bridge_modes_and_always_cleans(self):
        text=(ROOT/'README.md').read_text().split('Published browser-bridge releases before the Manager TUI',1)[1]
        script=text.split('```sh\n',1)[1].split('```',1)[0]
        for fault in (None,'download','signature','runtime','local','channel'):
            with self.subTest(fault=fault):
                fixture=fixtures.Rald5PublicationTests('test_verify_public_accepts_exact_pages_tree');fixture.setUp();self.addCleanup(fixture.tearDown)
                with patch.object(fixtures,'GENERATION',BRIDGE),patch.object(fixtures,'BASE','https://humtr.github.io/codex/'+BRIDGE+'/'),patch.object(fixtures,'SEQUENCE','42'):
                    fixture.build_signed(frontend=False);assets=fixture.root/'assets';result=fixture.prepare(assets);self.assertEqual(result.returncode,0,result.stderr)
                if fault=='signature':(assets/'release.sig').write_bytes(b'bad-signature')
                if fault=='runtime':(assets/'runtime').write_bytes(b'bad-runtime')
                tools=fixture.root/'tools';tools.mkdir();tmp=fixture.root/'tmp';tmp.mkdir();calls=fixture.root/'calls.jsonl'
                (tools/'gh').write_text('#!'+sys.executable+'\n'+'''import json,os,shutil,sys
from pathlib import Path
a=sys.argv[1:];assert a[:3]==['release','download',os.environ['BRIDGE']]
assert a[a.index('--repo')+1]=='humtr/codex'
patterns=[a[i+1] for i,v in enumerate(a) if v=='--pattern']
assert sorted(patterns)==sorted(['core','generation.meta','manager','runtime','codex-code-mode-host','helper-0','helper-1','release.manifest','release.sig'])
dst=Path(a[a.index('--dir')+1]);assert dst.parent==Path(os.environ['TMPDIR'])
for name in patterns:
 shutil.copyfile(Path(os.environ['ASSETS'])/name,dst/name)
 if os.environ['FAULT']=='download':sys.exit(1)
''')
                (tools/'codex').write_text('#!'+sys.executable+'\n'+'''import hashlib,json,os,subprocess,sys
from pathlib import Path
sys.path.insert(0,os.environ['SCRIPTS'])
from rald5_publication import parse_manifest
a=sys.argv[1:]
with open(os.environ['CALLS'],'a') as f:f.write(json.dumps(a)+'\\n')
if a[1:2]==['--local']:
 root=Path(a[2]);assert root.parent==Path(os.environ['TMPDIR'])
 values,inventory=parse_manifest(root/'release.manifest');assert values['generation_id']==os.environ['BRIDGE']
 result=subprocess.run(['openssl','pkeyutl','-verify','-pubin','-inkey',os.environ['PUBLIC_KEY'],'-rawin','-in',str(root/'release.manifest'),'-sigfile',str(root/'release.sig')],capture_output=True)
 if result.returncode:sys.exit(1)
 paths=set()
 for rel,digest,mode in inventory:
  p=root/rel;paths.add(rel)
  if hashlib.sha256(p.read_bytes()).hexdigest()!=digest:sys.exit(1)
  assert p.stat().st_mode&0o777==int(mode,8)
 assert {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}==paths|{'release.manifest','release.sig'}
 if os.environ['FAULT']=='local':sys.exit(1)
else:
 assert a==['update']
 if os.environ['FAULT']=='channel':sys.exit(1)
''')
                for p in tools.iterdir():p.chmod(0o755)
                env={**os.environ,'PATH':str(tools)+os.pathsep+os.environ['PATH'],'TMPDIR':str(tmp),'ASSETS':str(assets),'BRIDGE':BRIDGE,'FAULT':fault or '', 'CALLS':str(calls),'PUBLIC_KEY':str(fixture.public_key),'SCRIPTS':str(ROOT/'.github/scripts'),'PYTHONDONTWRITEBYTECODE':'1'}
                result=subprocess.run(['bash','-c',script],env=env,cwd=fixture.root,capture_output=True,text=True,timeout=15)
                self.assertEqual(result.returncode==0,fault is None,result.stderr)
                self.assertEqual(list(tmp.iterdir()),[],'owned download must always be removed')
                recorded=[json.loads(l) for l in calls.read_text().splitlines()] if calls.exists() else []
                self.assertEqual(len(recorded),2 if fault in (None,'channel') else 0 if fault=='download' else 1)
                if recorded:self.assertEqual(recorded[0][:2],['update','--local'])


if __name__=='__main__':unittest.main()
