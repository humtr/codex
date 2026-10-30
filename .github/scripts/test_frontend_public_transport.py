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
from rald5_publication import canonical_release_base
from release_components import CONTROL_ASSETS, FLAT_ASSETS

ROOT = Path(__file__).resolve().parents[2]
PAGES = ROOT / '.github/workflows/publish-termux-update-pages.yml'
AUTO = ROOT / '.github/workflows/auto-release-termux.yml'
CANDIDATE = 'local-owned-frontend-candidate'
MAIN = 'a' * 40


class PublicTransportTests(unittest.TestCase):
    def fixture(self, current_frontend, candidate_frontend):
        fixture = fixtures.Rald5PublicationTests('test_verify_public_accepts_exact_pages_tree')
        fixture.setUp()
        fixture.native_descriptor = True
        self.addCleanup(fixture.tearDown)
        root = fixture.root
        fixture.build_signed(frontend=current_frontend)
        current = root / 'current'
        shutil.copytree(fixture.signed, current)
        with patch.object(fixtures, 'GENERATION', CANDIDATE), patch.object(
                fixtures, 'BASE', canonical_release_base(CANDIDATE)), patch.object(fixtures, 'SEQUENCE', '12'):
            fixture.build_signed(frontend=candidate_frontend)
            if CANDIDATE.startswith(('local-relay-hosted-', 'local-base-relay-hosted-')):
                component = fixture.signed/'component-input-v1'
                metadata = {'schema': 'codex-component-input-v1', 'generation_id': CANDIDATE,
                            'source_sha': 'a'*40, 'base_inputs_sha256': 'b'*64,
                            'manifest_sha256': fixtures.sha256(fixture.signed/'releases'/CANDIDATE/'release.manifest')}
                component.write_text(json.dumps(metadata, sort_keys=True, separators=(',', ':'))+'\n')
                fixtures.openssl('pkeyutl', '-sign', '-rawin', '-inkey', str(fixture.private_key),
                                 '-in', str(component), '-out', str(component)+'.sig')
            assets = root / 'release-assets'
            result = fixture.prepare(assets)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
        source = root / 'publication-source/.github/scripts'
        source.mkdir(parents=True)
        shutil.copy2(ROOT / '.github/scripts/rald5_publication.py', source)
        for name in ['release_heads.py', 'rald3_preflight.py', 'release_components.py', 'runtime_inputs.py']:
            shutil.copy2(ROOT/'.github/scripts'/name, source)
        urls = {}
        for signed in (current, fixture.signed):
            generation = fixtures.GENERATION if signed == current else CANDIDATE
            base = canonical_release_base(generation)
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
        hooks = root/'transport-hook';hooks.mkdir()
        (hooks/'sitecustomize.py').write_text(textwrap.dedent('''\
            import io,json,os
            from pathlib import Path
            import urllib.request
            from urllib.error import HTTPError
            def owned_open(url,timeout=60):
                if isinstance(url,urllib.request.Request):url=url.full_url
                paths=json.loads((Path(os.environ['TRANSPORT_FIXTURE'])/'urls.json').read_text())
                if url not in paths:raise HTTPError(url,404,'owned missing',None,None)
                value=io.BytesIO(Path(paths[url]).read_bytes());value.geturl=lambda:url
                return value
            urllib.request.urlopen=owned_open
        '''))
        tools = root / 'bin'
        tools.mkdir()
        (tools / 'curl').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
            import json, os, sys
            from pathlib import Path
            args = sys.argv[1:]
            root = Path(os.environ['TRANSPORT_FIXTURE'])
            with (root / 'curl-requests').open('a') as log: log.write(args[-1]+'\\n')
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
            for path in (Path(os.environ['TRANSPORT_FIXTURE'])/'release-assets').iterdir():
                shutil.copy2(path, destination/path.name)
        '''))
        for tool in tools.iterdir():
            tool.chmod(0o755)
        env = {**os.environ, 'TRANSPORT_FIXTURE': str(root),
               'PATH': str(tools) + os.pathsep + os.environ['PATH'],
               'GENERATION_ID': CANDIDATE, 'RELEASE_SEQUENCE': '12',
               'RUNNER_TEMP': str(root), 'PYTHONPATH': str(hooks),
               'CURRENT_GENERATION': fixtures.GENERATION, 'EXPECTED_MAIN_SHA': MAIN,
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
                actual = actual.replace('${{ inputs.release_variant }}', 'full')
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
                actual += 'python3 publication-source/.github/scripts/rald5_publication.py verify-public --root "$root" --public-key "$authority" --generation "$candidate" --release-sequence 12 --release-base "https://humtr.github.io/codex/$candidate/" --require-download-size\n'
                result = subprocess.run(['bash', '-ec', actual], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=30)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1200:])
                if fault is None:
                    self.assertEqual((destination / CANDIDATE / 'helpers/2').exists(), frontend)
                if fault == 'signature':
                    self.assertFalse((destination / CANDIDATE / 'core').exists())

    def test_mapped_readback_uses_github_and_checks_signed_inventory(self):
        block = AUTO.read_text().split('          fetch_generation() {', 1)[1].split('          current_index=', 1)[0]
        script = textwrap.dedent('          fetch_generation() {' + block)
        generation = 'local-relay-hosted-owned-s12'
        previous = 'local-base-relay-hosted-owned-s11'
        github = f'https://github.com/humtr/codex/releases/download/{generation}/'
        prior = f'https://github.com/humtr/codex/releases/download/{previous}/'
        for fault in (None, 'map-digest', 'traversal', 'payload', 'missing', 'signature'):
            with self.subTest(fault=fault), patch.object(sys.modules[__name__], 'CANDIDATE', generation):
                fixture, _, assets, env = self.fixture(True, True)
                root = fixture.root
                release = fixture.signed / 'releases' / generation
                files = {name: {'generation': generation, 'asset': FLAT_ASSETS[name]}
                         for name, _ in fixtures.FILES + [('helpers/2', 0o755)]}
                files['helpers/0']['generation'] = previous
                files.update({name: {'generation': generation, 'asset': asset}
                              for name, asset in CONTROL_ASSETS.items()})
                mapping = {'schema': 'codex-asset-map-v1', 'generation_id': generation,
                           'base_generation': previous, 'manifest_sha256': fixtures.sha256(release/'release.manifest'),
                           'files': files}
                if fault == 'map-digest': mapping['manifest_sha256'] = '0'*64
                if fault == 'traversal': mapping['files']['helpers/0']['asset'] = '../helper-0'
                map_path = root/'asset-map-v1.json'
                map_path.write_text(json.dumps(mapping))
                urls = {github + path.name: str(path) for path in assets.iterdir() if path.is_file()}
                urls[github + 'asset-map-v1.json'] = str(map_path)
                urls[prior + 'helper-0'] = str(assets/'helper-0')
                if fault == 'payload': (assets/'helper-2').write_bytes(b'changed')
                if fault == 'missing': del urls[prior + 'helper-0']
                if fault == 'signature': (assets/'release.sig').write_bytes(b'x'*64)
                (root/'urls.json').write_text(json.dumps(urls))
                destination = root/'readback'; destination.mkdir()
                env.update(root=str(destination), authority=str(fixture.public_key), candidate=generation)
                index = github+'candidate-update-index-v1'
                actual = script + '\nfetch_generation "$candidate" "'+index+'" true\n'
                actual += 'python3 publication-source/.github/scripts/rald5_publication.py verify-public --root "$root" --public-key "$authority" --generation "$candidate" --release-sequence 12 --release-base "'+canonical_release_base(generation)+'" --require-download-size\n'
                result = subprocess.run(['bash', '-ec', actual], cwd=root, env=env,
                                        text=True, capture_output=True, timeout=30)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr[-1200:])
                requests = (root/'curl-requests').read_text().splitlines()
                self.assertTrue(all(url.startswith('https://github.com/humtr/codex/releases/download/') for url in requests))
                if fault is None:
                    self.assertIn(prior+'helper-0', requests)
                    self.assertEqual((destination/generation/'helpers/0').read_bytes(), (assets/'helper-0').read_bytes())
                if fault in ('map-digest', 'traversal', 'signature'):
                    self.assertFalse((destination/generation/'core').exists())


if __name__ == '__main__':
    unittest.main()
