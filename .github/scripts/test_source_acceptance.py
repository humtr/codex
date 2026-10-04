import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import textwrap
import unittest

import test_rald5_publication as fixtures

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/rald7-full-acceptance.yml'


class SourceAcceptanceTests(unittest.TestCase):
    def test_event_source_and_one_serial_corpus(self):
        workflow = WORKFLOW.read_text()
        for stale in ['EXPECTED_PARENT', 'EXPECTED_MAIN', 'ACCEPTED_SOURCE',
                      'PUBLIC_GENERATION', 'PUBLIC_SEQUENCE', '  packages:']:
            self.assertNotIn(stale, workflow)
        product = workflow.split('- name: Fetch exact event product source', 1)[1].split('- name: Cross-build', 1)[0]
        self.assertIn('origin "$GITHUB_SHA"', product)
        self.assertIn('rev-parse HEAD)" = "$GITHUB_SHA"', product)
        self.assertEqual(workflow.count('cargo test --workspace --locked'), 1)
        self.assertIn('cargo test --workspace --locked -- --test-threads=1', workflow)
        self.assertEqual(workflow.count('git -C source fetch --no-tags --depth=2'), 3)
        self.assertNotIn('git -C source fetch --no-tags --depth=1', workflow)
        self.assertIn('git diff --check HEAD^ HEAD', workflow)
        self.assertIn('needs: [contract, workspace, public-audit]', workflow)
        for path in ["'.github/**'", "'crates/**'", "'scripts/**'", "'Cargo.lock'"]:
            self.assertIn(path, workflow)
        for required in ['--require-download-size', 'parse_index', 'parse_manifest',
                         'PUBLIC_KEY_SHA256', ' = "$main_sha"']:
            self.assertIn(required, workflow)

    def test_authenticated_audit_executes_and_rejects_each_fault(self):
        block = WORKFLOW.read_text().split('- name: Verify authenticated stable snapshot and every published byte', 1)[1].split('  accepted:', 1)[0]
        script = textwrap.dedent(block.split('run: |\n', 1)[1])
        parsed = subprocess.run(['bash', '-n'], input=script, text=True, capture_output=True)
        self.assertEqual(parsed.returncode, 0, parsed.stderr)
        for fault in [None, 'key', 'index-signature', 'index-shape',
                      'manifest-signature', 'manifest-path', 'manifest-mode',
                      'payload', 'sidecar', 'missing-sidecar', 'main-moved']:
            with self.subTest(fault=fault):
                fixture = fixtures.Rald5PublicationTests('test_verify_public_accepts_exact_pages_tree')
                fixture.setUp()
                try:
                    root = fixture.root
                    source = root / 'source/.github/scripts'
                    source.mkdir(parents=True)
                    shutil.copy2(ROOT / '.github/scripts/rald5_publication.py', source)
                    release = fixture.signed / 'releases' / fixtures.GENERATION
                    files = {
                        'update-index-v1': fixture.signed / 'update-index-v1',
                        'update-index-v1.sig': fixture.signed / 'update-index-v1.sig',
                        'public.pem': fixture.public_key,
                        **{str(p.relative_to(release)): p for p in release.rglob('*') if p.is_file()},
                        'compat/download-size-v1': fixture.signed / 'download-size-v1',
                        'compat/download-size-v1.sig': fixture.signed / 'download-size-v1.sig',
                    }

                    def resign(data, signature):
                        fixtures.openssl('pkeyutl', '-sign', '-rawin', '-inkey', str(fixture.private_key),
                                         '-in', str(data), '-out', str(signature))

                    if fault in ['index-signature', 'manifest-signature']:
                        name = 'update-index-v1.sig' if fault == 'index-signature' else 'release.sig'
                        files[name].write_bytes(b'x' * 64)
                    elif fault == 'index-shape':
                        files['update-index-v1'].write_text(files['update-index-v1'].read_text() + 'extra\tx\n')
                        resign(files['update-index-v1'], files['update-index-v1.sig'])
                    elif fault in ['manifest-path', 'manifest-mode']:
                        manifest = files['release.manifest']
                        data = manifest.read_text()
                        if fault == 'manifest-path':
                            data = data.replace('file\truntime\t', 'file\t../runtime\t')
                        else:
                            data = data.replace('\t0755\n', '\t0644\n', 1)
                        manifest.write_text(data)
                        resign(manifest, files['release.sig'])
                    elif fault == 'payload':
                        files['runtime'].write_bytes(b'wrong')
                    elif fault == 'sidecar':
                        files['compat/download-size-v1'].write_bytes(b'wrong')
                    elif fault == 'missing-sidecar':
                        del files['compat/download-size-v1']
                    urls = {}
                    raw = f"https://raw.githubusercontent.com/humtr/codex/{'a' * 40}/"
                    for name, path in files.items():
                        url = 'https://fixture.invalid/public.pem' if name == 'public.pem' else (
                            raw + name if name.startswith('update-index') else fixtures.BASE + name)
                        urls[url] = str(path)
                    (root / 'urls.json').write_text(json.dumps(urls))
                    tools = root / 'bin'
                    tools.mkdir()
                    (tools / 'curl').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
                        import json, os, sys
                        from pathlib import Path
                        args = sys.argv[1:]
                        root = Path(os.environ['AUDIT_FIXTURE'])
                        source = json.loads((root / 'urls.json').read_text()).get(args[-1])
                        if source is None: sys.exit(22)
                        data = Path(source).read_bytes()
                        if len(data) > int(args[args.index('--max-filesize')+1]): sys.exit(63)
                        Path(args[args.index('--output')+1]).write_bytes(data)
                    '''))
                    (tools / 'git').write_text(f'#!{sys.executable}\n' + textwrap.dedent('''\
                        import os
                        from pathlib import Path
                        root = Path(os.environ['AUDIT_FIXTURE'])
                        counter = root / 'git-count'
                        count = int(counter.read_text()) + 1 if counter.exists() else 1
                        counter.write_text(str(count))
                        sha = 'b' * 40 if count > 1 and os.environ.get('AUDIT_MOVE') == '1' else 'a' * 40
                        print(sha + '\\trefs/heads/main')
                    '''))
                    for p in tools.iterdir():
                        p.chmod(0o755)
                    env = {**os.environ,
                           'AUDIT_FIXTURE': str(root), 'AUDIT_MOVE': '1' if fault == 'main-moved' else '0',
                           'RUNNER_TEMP': str(root / 'runner'), 'GITHUB_REPOSITORY': 'humtr/codex',
                           'PUBLIC_KEY_URL': 'https://fixture.invalid/public.pem',
                           'PUBLIC_KEY_SHA256': '0' * 64 if fault == 'key' else hashlib.sha256(fixture.public_key.read_bytes()).hexdigest(),
                           'PATH': str(tools) + os.pathsep + os.environ['PATH'],
                           'PYTHONDONTWRITEBYTECODE': '1'}
                    result = subprocess.run(['bash', '-c', script], cwd=root, env=env,
                                            text=True, capture_output=True, timeout=30)
                    if fault is None:
                        self.assertEqual(result.returncode, 0, result.stderr)
                        self.assertIn('verified_sequence=11', result.stdout)
                        self.assertIn('Authenticated public snapshot', result.stdout)
                    else:
                        self.assertNotEqual(result.returncode, 0, result.stdout)
                finally:
                    fixture.tearDown()


if __name__ == '__main__':
    unittest.main()
