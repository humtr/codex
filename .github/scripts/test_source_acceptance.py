import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest

import test_rald5_publication as fixtures

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/rald7-full-acceptance.yml'
RELEASE_WORKFLOW = ROOT / '.github/workflows/auto-release-termux.yml'


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
        self.assertEqual(workflow.count('sudo unshare --mount --pid --fork --mount-proc --propagation private'), 1)
        self.assertIn('setpriv --reuid="$(id -u)" --regid="$(id -g)" --init-groups', workflow)
        self.assertEqual(workflow.count('test "$(id -u)" -ne 0'), 2)
        isolated = workflow.split('sudo unshare --mount', 1)[1].split('- name: Clippy warnings', 1)[0]
        self.assertIn('cargo test --workspace --locked', isolated)
        self.assertEqual(isolated.count('CODEX_B10_RELEASE_CORE="$RALD7_ANDROID_CORE" cargo test'), 3)
        android_artifact_tests = {
            'test_r6_builder_publish_output_enters_existing_signed_release_admission',
            'test_rald1_local_derived_fallback_and_explicit_build_preserve_public_authority',
            'test_manager_tui_public_local_build_preserves_pair_and_refuses_unpaired_update',
        }
        self.assertEqual(set(re.findall(r'--skip ([a-z0-9_]+)', isolated)), android_artifact_tests)
        self.assertEqual(set(re.findall(
            r'CODEX_B10_RELEASE_CORE="\$RALD7_ANDROID_CORE" cargo test --locked -p codex tests::([a-z0-9_]+) -- --exact --nocapture --test-threads=1',
            isolated)), android_artifact_tests)
        self.assertNotIn('sudo cargo', isolated)
        self.assertIn('--bounding-set=-all --inh-caps=-all --ambient-caps=-all', isolated)
        self.assertEqual(workflow.count('git -C source fetch --no-tags --depth=2'), 2)
        self.assertNotIn('git -C source fetch --no-tags --depth=1', workflow)
        self.assertIn('git diff --check HEAD^ HEAD', workflow)
        self.assertIn('needs: [contract, workspace]', workflow)
        self.assertIn("needs.contract.outputs.scope == 'full'",workflow)
        self.assertIn('scripts/check.sh python',workflow)
        self.assertNotIn('public-audit:',workflow)
        self.assertIn('Restore compiler cache',workflow)

    def test_pack_follows_all_candidate_bindings_and_immediately_precedes_upload(self):
        workflow = RELEASE_WORKFLOW.read_text()
        producer = workflow.split('  producer:', 1)[1].split('  smoke:', 1)[0]
        self.assertEqual(producer.count('-cf "$work/candidate.tar"'), 1)
        build = producer.split('- name: Fetch, adapt, and qualify unsigned candidate', 1)[1].split('      - name:', 1)[0]
        self.assertNotIn('candidate.tar', build)
        pack = producer.index('      - name: Package admitted unsigned candidate')
        upload = producer.index('      - name: Upload unsigned candidate only')
        self.assertLess(pack, upload)
        self.assertNotIn('      - name:', producer[pack + 1:upload])
        for name in ['      - name: Require exact', '      - name: Bind ',
                     '      - name: Preserve authenticated stable Core']:
            for match in re.finditer(re.escape(name), producer):
                self.assertLess(match.start(), pack)

    def test_actual_pack_replaces_stale_archive_with_final_bytes_modes_and_inventory(self):
        block = RELEASE_WORKFLOW.read_text().split('      - name: Package admitted unsigned candidate', 1)[1].split('      - name:', 1)[0]
        script = textwrap.dedent(block.split('        run: |\n', 1)[1])
        with tempfile.TemporaryDirectory(prefix='release-archive-') as temp:
            root = Path(temp)
            work = root / 'rald3-candidate'
            candidate = work / 'candidate'
            candidate.mkdir(parents=True)
            for rel in ['core', 'manager', 'runtime', 'codex-code-mode-host',
                        'helpers/0', 'helpers/1', 'generation.meta', '.manager-probe-deferred']:
                path = candidate / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(('initial ' + rel).encode())
                path.chmod(0o644 if rel in ['generation.meta', '.manager-probe-deferred'] else 0o755)
            archive = work / 'candidate.tar'
            subprocess.run(['tar', '-cf', str(archive), '-C', str(work), 'candidate'], check=True)
            (candidate / 'core').write_bytes(b'final protected Core')
            (candidate / 'manager').write_bytes(b'final accepted Manager')
            (candidate / 'generation.meta').write_text('final admitted descriptor\n')
            expected = {str(p.relative_to(work)): (p.read_bytes(), p.stat().st_mode & 0o7777)
                        for p in candidate.rglob('*') if p.is_file()}
            with tarfile.open(archive) as old:
                self.assertNotEqual(old.extractfile('candidate/core').read(), expected['candidate/core'][0])
            result = subprocess.run(['bash', '-c', script], env={**os.environ, 'RUNNER_TEMP': str(root)},
                                    capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)
            with tarfile.open(archive) as packed:
                observed = {m.name: (packed.extractfile(m).read(), m.mode)
                            for m in packed.getmembers() if m.isfile()}
            self.assertEqual(observed, expected)

if __name__ == '__main__':
    unittest.main()
