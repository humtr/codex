"""Execute real hosted Pages/readback transport with owned signed fixtures."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import textwrap
import unittest
from unittest.mock import patch

import test_rald5_publication as fixtures

ROOT = Path(__file__).resolve().parents[2]
PAGES = ROOT / '.github/workflows/publish-termux-update-pages.yml'
AUTO = ROOT / '.github/workflows/auto-release-termux.yml'
CANDIDATE = 'local-owned-frontend-candidate'
MAIN = 'a' * 40


class PublicTransportTests(unittest.TestCase):
    def fixture(self, current_frontend, candidate_frontend):
        fixture = fixtures.Rald5PublicationTests('test_verify_public_accepts_exact_pages_tree')
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        root = fixture.root
        fixture.build_signed(frontend=current_frontend)
        current = root / 'current'
        shutil.copytree(fixture.signed, current)
        with patch.object(fixtures, 'GENERATION', CANDIDATE), patch.object(
                fixtures, 'BASE', f'https://humtr.github.io/codex/{CANDIDATE}/'):
            fixture.build_signed(frontend=candidate_frontend)
            assets = root / 'release-assets'
            result = fixture.prepare(assets)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
        source = root / 'publication-source/.github/scripts'
        source.mkdir(parents=True)
        shutil.copy2(ROOT / '.github/scripts/rald5_publication.py', source)
        urls = {}
        for signed in (current, fixture.signed):
            generation = fixtures.GENERATION if signed == current else CANDIDATE
            base = f'https://humtr.github.io/codex/{generation}/'
            release = signed / 'releases' / generation
            for path in release.rglob('*'):
                if path.is_file():
                    urls[base + str(path.relative_to(release))] = str(path)
            for name in ('download-size-v1', 'download-size-v1.sig'):
                urls[base + 'compat/' + name] = str(signed / name)
            for name in ('update-index-v1', 'update-index-v1.sig'):
                urls[base + name] = str(signed / name)
                if signed == current:
                    urls[f'https://raw.githubusercontent.com/humtr/codex/{MAIN}/{name}'] = str(signed / name)
        (root / 'urls.json').write_text(json.dumps(urls))
        tools = root / 'bin'
        tools.mkdir()
        (tools / 'curl').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
            import json, os, sys
            from pathlib import Path
            args = sys.argv[1:]
            root = Path(os.environ['TRANSPORT_FIXTURE'])
            source = json.loads((root / 'urls.json').read_text()).get(args[-1])
            code = 200 if source else 404
            if not source:
                if '--write-out' in args: print(code, end='')
                sys.exit(22 if '--fail' in args else 0)
            data = Path(source).read_bytes()
            if '--max-filesize' in args and len(data) > int(args[args.index('--max-filesize')+1]):
                sys.exit(63)
            flag = '--output' if '--output' in args else '-o'
            Path(args[args.index(flag)+1]).write_bytes(data)
            if '--write-out' in args: print(code, end='')
        '''))
        (tools / 'git').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
            import os
            from pathlib import Path
            root = Path(os.environ['TRANSPORT_FIXTURE'])
            marker = root / 'git-calls'
            count = int(marker.read_text())+1 if marker.exists() else 1
            marker.write_text(str(count))
            sha = 'b'*40 if count > 1 and os.environ.get('MOVE_MAIN') == '1' else 'a'*40
            print(sha+'\\trefs/heads/main')
        '''))
        (tools / 'gh').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
            import os, shutil, sys
            from pathlib import Path
            args = sys.argv[1:]
            assert args[:2] == ['release', 'download']
            destination = Path(args[args.index('--dir')+1])
            assets = 'bridge-assets' if destination.name == 'migration-stage' else 'release-assets'
            for path in (Path(os.environ['TRANSPORT_FIXTURE'])/assets).iterdir():
                shutil.copy2(path, destination/path.name)
        '''))
        for tool in tools.iterdir():
            tool.chmod(0o755)
        env = {**os.environ, 'TRANSPORT_FIXTURE': str(root),
               'PATH': str(tools) + os.pathsep + os.environ['PATH'],
               'GENERATION_ID': CANDIDATE, 'RELEASE_SEQUENCE': fixtures.SEQUENCE,
               'CURRENT_GENERATION': fixtures.GENERATION, 'EXPECTED_MAIN_SHA': MAIN, 'MIGRATION_BRIDGE_GENERATION': '',
               'GITHUB_REPOSITORY': 'humtr/codex', 'PYTHONDONTWRITEBYTECODE': '1'}
        return fixture, current, assets, env

    def test_real_pages_reconstructs_legacy_and_extended_lkg_and_candidate(self):
        block = PAGES.read_text().split('- name: Reconstruct and verify signed generation', 1)[1].split('      - name:', 1)[0]
        script = textwrap.dedent(block.split('        run: |\n', 1)[1])
        syntax = subprocess.run(['bash', '-n'], input=script, text=True, capture_output=True)
        self.assertEqual(syntax.returncode, 0, syntax.stderr)
        cases = [(False, False, None), (False, True, None), (True, False, None), (True, True, None),
                 (False, True, 'candidate-helper'), (True, True, 'current-helper'),
                 (True, True, 'main-moved'), (False, True, 'legacy-no-size'),
                 (True, True, 'missing-current-helper')]
        for current_frontend, candidate_frontend, fault in cases:
            with self.subTest(current=current_frontend, candidate=candidate_frontend, fault=fault):
                fixture, current, assets, env = self.fixture(current_frontend, candidate_frontend)
                root = fixture.root
                key_hash = hashlib.sha256(fixture.public_key.read_bytes()).hexdigest()
                actual = script.replace('62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c', key_hash)
                if fault == 'candidate-helper':
                    (assets / 'helper-2').write_bytes(b'changed')
                elif fault == 'current-helper':
                    (current / 'releases' / fixtures.GENERATION / 'helpers/2').write_bytes(b'changed')
                elif fault in ('legacy-no-size', 'missing-current-helper'):
                    urls = json.loads((root / 'urls.json').read_text())
                    names = ('compat/download-size-v1', 'compat/download-size-v1.sig') if fault == 'legacy-no-size' else ('helpers/2',)
                    for name in names:
                        del urls[fixtures.BASE + name]
                    (root / 'urls.json').write_text(json.dumps(urls))
                elif fault == 'main-moved':
                    env['MOVE_MAIN'] = '1'
                result = subprocess.run(['bash', '-c', actual], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=30)
                success = fault in (None, 'legacy-no-size')
                self.assertEqual(result.returncode == 0, success, result.stderr[-1200:])
                if success:
                    for generation, frontend in ((fixtures.GENERATION, current_frontend), (CANDIDATE, candidate_frontend)):
                        release = root / 'site' / generation
                        self.assertTrue((release / 'helpers/0').is_file())
                        self.assertEqual((release / 'helpers/2').is_file(), frontend)
                        self.assertEqual((release / 'core').stat().st_mode & 0o7777, 0o755)
                    self.assertEqual({path.name for path in (root / 'site').iterdir()}, {fixtures.GENERATION, CANDIDATE})
                    self.assertIn('current_generation=' + fixtures.GENERATION, result.stdout)

    def test_real_pages_retains_one_signed_migration_bridge_and_refuses_asset_faults(self):
        bridge = 'local-hosted-0-161-0-ab644771ac89-manager-tui-bridge'
        block = PAGES.read_text().split('- name: Reconstruct and verify signed generation', 1)[1].split('      - name:', 1)[0]
        script = textwrap.dedent(block.split('        run: |\n', 1)[1])
        for fault in (None, 'missing', 'changed', 'extra', 'key', 'sequence', 'identifier', 'dedup-current', 'dedup-candidate'):
            with self.subTest(fault=fault):
                selected_current = bridge if fault == 'dedup-current' else fixtures.GENERATION
                selected_candidate = bridge if fault == 'dedup-candidate' else CANDIDATE
                with patch.object(fixtures, 'GENERATION', selected_current), patch.object(fixtures, 'BASE', 'https://humtr.github.io/codex/' + selected_current + '/'), patch.object(sys.modules[__name__], 'CANDIDATE', selected_candidate):
                    fixture, current, assets, env = self.fixture(False, fault != 'dedup-candidate')
                root = fixture.root
                env['MIGRATION_BRIDGE_GENERATION'] = 'unexpected' if fault == 'identifier' else bridge
                migration = fixtures.Rald5PublicationTests('test_verify_public_accepts_exact_pages_tree')
                migration.setUp();self.addCleanup(migration.tearDown)
                if fault != 'key':
                    migration.private_key = fixture.private_key
                    migration.public_key = fixture.public_key
                with patch.object(fixtures, 'GENERATION', bridge), patch.object(fixtures, 'SEQUENCE', '43' if fault == 'sequence' else '42'), patch.object(fixtures, 'BASE', 'https://humtr.github.io/codex/' + bridge + '/'):
                    migration.build_signed(frontend=False)
                    bridge_assets = root / 'bridge-assets'
                    result = migration.prepare(bridge_assets)
                    self.assertEqual(result.returncode, 0, result.stderr.decode())
                if fault == 'missing': (bridge_assets/'helper-1').unlink()
                if fault == 'changed': (bridge_assets/'runtime').write_text('changed')
                if fault == 'extra': (bridge_assets/'unexpected').write_text('extra')
                key_digest = hashlib.sha256(fixture.public_key.read_bytes()).hexdigest()
                actual = script.replace('62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c',key_digest)
                result = subprocess.run(['bash','-c',actual],cwd=root,env=env,capture_output=True,text=True,timeout=25)
                success = fault in (None, 'dedup-current', 'dedup-candidate')
                self.assertEqual(result.returncode==0,success,result.stderr)
                if success:
                    self.assertTrue((root/'site'/bridge/'core').is_file())
                    self.assertEqual(len(list((root/'site').iterdir())),2 if fault else 3)
                    self.assertEqual((root/'migration-stage').exists(),fault is None)
                else:
                    self.assertFalse((root/'site'/bridge).exists())

    def test_real_https_fetch_follows_authenticated_optional_helper_inventory(self):
        workflow = AUTO.read_text()
        block = workflow.split('          fetch_generation() {', 1)[1].split('          current_index=', 1)[0]
        script = textwrap.dedent('          fetch_generation() {' + block)
        for frontend, fault in ((False, None), (True, None), (True, 'payload'), (True, 'missing'), (True, 'signature')):
            with self.subTest(frontend=frontend, fault=fault):
                fixture, current, assets, env = self.fixture(frontend, frontend)
                root = fixture.root
                release = fixture.signed / 'releases' / CANDIDATE
                if fault == 'payload':
                    (release / 'helpers/2').write_bytes(b'changed')
                elif fault == 'missing':
                    (release / 'helpers/2').unlink()
                elif fault == 'signature':
                    (release / 'release.sig').write_bytes(b'x' * 64)
                destination = root / 'readback'
                destination.mkdir()
                env.update(root=str(destination), authority=str(fixture.public_key), candidate=CANDIDATE)
                actual = script + '\nfetch_generation "$candidate" "https://humtr.github.io/codex/$candidate/update-index-v1" true\n'
                actual += 'python3 publication-source/.github/scripts/rald5_publication.py verify-public --root "$root" --public-key "$authority" --generation "$candidate" --release-sequence 11 --release-base "https://humtr.github.io/codex/$candidate/" --require-download-size\n'
                result = subprocess.run(['bash', '-ec', actual], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=30)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1200:])
                if fault is None:
                    self.assertEqual((destination / CANDIDATE / 'helpers/2').exists(), frontend)
                if fault == 'signature':
                    self.assertFalse((destination / CANDIDATE / 'core').exists())


if __name__ == '__main__':
    unittest.main()
