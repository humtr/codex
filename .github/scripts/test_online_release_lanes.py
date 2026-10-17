"""Execute online acquisition with real signatures and owned HTTPS transports.

The final installer is a recording fixture. Actual Android Core installation and
lifecycle are qualified separately; these tests exercise transport and inventory.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import unittest

import test_release_heads as fixtures

ROOT = Path(__file__).resolve().parents[2]


class OnlineReleaseLanes(unittest.TestCase):
    def acquire(self, lane, frontend=False, fault=None):
        fixture = fixtures.ReleaseHeadsTests();fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        root = fixture.root
        generation = 'local-base-proof-12' if lane == 'base' else 'local-full-proof-12'
        fixture.head(lane, generation, 12, frontend)
        objects = dict(fixture.objects)
        original = (ROOT/'install-online.sh').read_text()
        key_url = re.search(r"BOOTSTRAP_KEY_URL='([^']+)'", original)[1]
        source = re.search(r"LOCAL_INSTALL_SOURCE_SHA='([^']+)'", original)[1]
        objects[key_url] = fixture.public.read_bytes()
        for selected, path in [('full','update-index-v1'),('base','update-index-base-v1')]:
            url = f'https://raw.githubusercontent.com/humtr/codex/{fixture.main}/{path}'
            if url in objects:
                for suffix in ['', '.sig']:
                    objects[f'https://raw.githubusercontent.com/humtr/codex/main/{path}{suffix}'] = objects[url+suffix]
        base = 'https://humtr.github.io/codex/'+generation+'/'
        if fault == 'content':objects[base+'runtime'] += b'tampered'
        if fault == 'signature':
            objects['https://raw.githubusercontent.com/humtr/codex/main/update-index-v1.sig'] = b'0'*64
        local = f'https://raw.githubusercontent.com/humtr/codex/{source}/'
        shell = shutil.which('sh')
        objects[local+'install.sh'] = ('#!'+shell+'\nprintf "%s\\n" "$@" > "$INSTALL_RESULT"\n').encode()
        objects[local+'bootstrap/codex-bootstrap'] = ('#!'+shell+'\nexit 99\n').encode()
        mapping = {}
        for index, (url, data) in enumerate(objects.items()):
            path = root/('object-'+str(index));path.write_bytes(data);mapping[url] = str(path)
        transport = root/'transport.json';transport.write_text(json.dumps(mapping))
        prefix = root/'prefix';(prefix/'bin').mkdir(parents=True)
        curl = prefix/'bin/curl'
        curl.write_text('#!'+sys.executable+'\n'+
            'import json,os,shutil,sys\n'+
            'mapping=json.load(open(os.environ["OWNED_TRANSPORT"]))\n'+
            'shutil.copyfile(mapping[sys.argv[-1]],sys.argv[sys.argv.index("--output")+1])\n')
        curl.chmod(0o755)
        (prefix/'bin/openssl').symlink_to(shutil.which('openssl'))
        script = root/'online.sh'
        script.write_text(original.replace(
            '62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c',
            hashlib.sha256(fixture.public.read_bytes()).hexdigest()))
        marker = root/'installed'
        env = {**os.environ, 'HOME':str(root/'home'), 'PREFIX':str(prefix),
               'TMPDIR':str(root),'OWNED_TRANSPORT':str(transport),'INSTALL_RESULT':str(marker)}
        result = subprocess.run([shell,str(script),'--variant',lane],env=env,text=True,capture_output=True,timeout=30)
        self.assertEqual(result.returncode == 0, fault is None, result.stderr)
        self.assertEqual(marker.exists(), fault is None)
        if marker.exists():
            self.assertEqual(len(marker.read_text().splitlines()),3)
        self.assertFalse(list(root.glob('.codex-online-install.*')))

    def test_base_legacy_full_and_enhanced_full_reach_the_existing_installer(self):
        for lane, frontend in [('base',False),('full',False),('full',True)]:
            with self.subTest(lane=lane,frontend=frontend):self.acquire(lane,frontend)

    def test_corrupt_signature_or_asset_cannot_reach_installation(self):
        for fault in ['signature','content']:
            with self.subTest(fault=fault):self.acquire('full',True,fault)


if __name__ == '__main__':unittest.main()
