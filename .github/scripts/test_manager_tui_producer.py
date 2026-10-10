"""Run the real producer acquisition and builder dispatch with owned transports."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest

import test_manager_tui_artifact as fixtures

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/auto-release-termux.yml'


def step(name):
    block = WORKFLOW.read_text().split('      - name: ' + name + '\n', 1)[1].split('\n      - name:', 1)[0]
    return textwrap.dedent(block.split('        run: |\n', 1)[1])


class NativeProducerTests(unittest.TestCase):
    def test_actual_acquisition_refuses_bad_archive_failed_run_and_changed_source(self):
        for fault in (None, 'archive', 'failed-run', 'source'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                source, artifact, proof, sha, digest, run, run_file = fixtures.NativeArtifactTests().fixture(root)
                (source / '.github/scripts').symlink_to(ROOT / '.github/scripts', target_is_directory=True)
                archive = root / 'input.tar.gz'
                with tarfile.open(archive, 'w:gz') as bundle:
                    for p in artifact.iterdir(): bundle.add(p, arcname='artifact/' + p.name)
                    for p in proof.iterdir(): bundle.add(p, arcname='proof/_temp/' + p.name)
                archive_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                if fault == 'archive': archive.write_bytes(b'not qualified')
                if fault == 'failed-run':
                    run['conclusion'] = 'failure'; run_file.write_text(json.dumps(run))
                if fault == 'source':
                    p = source / 'frontend/manager-tui/native.patch'; p.write_text('changed\n')
                    subprocess.run(['git', '-C', str(source), 'add', '.'], check=True)
                    subprocess.run(['git', '-C', str(source), '-c', 'user.name=Owned', '-c', 'user.email=x@example.invalid', 'commit', '-m', 'changed'], check=True, stdout=subprocess.DEVNULL)
                tools = root / 'tools'; tools.mkdir()
                real_git = shutil.which('git')
                commands = {
                    'curl': 'import shutil,sys; shutil.copyfile(' + repr(str(archive)) + ',sys.argv[sys.argv.index("--output")+1])',
                    'gh': 'import sys; sys.stdout.write(' + repr(run_file.read_text()) + ')',
                    'git': 'import os,sys;\nif "fetch" not in sys.argv: os.execv(' + repr(real_git) + ',[' + repr(real_git) + ']+sys.argv[1:])',
                }
                for name, code in commands.items():
                    p = tools / name; p.write_text('#!' + sys.executable + '\n' + code + '\n'); p.chmod(0o755)
                env = {**os.environ, 'PATH': str(tools) + ':' + os.environ['PATH'], 'RUNNER_TEMP': str(root),
                       'MANAGER_TUI_INPUT_URL': 'https://example.invalid/qualified', 'MANAGER_TUI_INPUT_SHA256': archive_digest,
                       'MANAGER_TUI_RUN_ID': '123', 'MANAGER_TUI_SOURCE_SHA': sha, 'MANAGER_TUI_VERSION': '0.161.0',
                       'MANAGER_TUI_SHA256': digest, 'PYTHONDONTWRITEBYTECODE': '1'}
                result = subprocess.run(['bash', '-c', step('Acquire successful qualified Manager TUI input')], cwd=root, env=env, text=True, capture_output=True, timeout=20)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr)
                if fault == 'archive': self.assertFalse((root / 'manager-tui-input/artifact').exists())
                if fault is None: self.assertIn(digest, result.stdout)

    def test_actual_candidate_builder_receives_paired_flags_and_historical_build_stays_unpaired(self):
        for version, bridge in [('0.161.0', False), ('0.154.0', False), ('0.161.0', True)]:
            with self.subTest(version=version), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); binary = root / 'source/target/release/codex-release-builder'; binary.parent.mkdir(parents=True)
                binary.write_text('#!' + sys.executable + '\n' + textwrap.dedent('''\
                    import json,os,sys
                    from pathlib import Path
                    if sys.argv[1] == 'fetch': print('archive_sha256\\t' + 'a'*64)
                    else: Path(os.environ['CAPTURE']).write_text(json.dumps(sys.argv[1:]))
                ''')); binary.chmod(0o755)
                script = step('Fetch, adapt, and qualify unsigned candidate').split('\npython3 source/.github/scripts/rald3_preflight.py candidate', 1)[0]
                for field, value in [('upstream_version', version), ('archive_sha256', 'a'*64), ('generation_id', 'owned-candidate'), ('manager_tui_bridge_deploy', str(bridge).lower())]:
                    script = script.replace('${{ steps.decision.outputs.' + field + ' }}', value)
                env = {**os.environ, 'RUNNER_TEMP': str(root), 'GITHUB_WORKSPACE': str(root), 'MANAGER_TUI_VERSION': '0.161.0', 'CAPTURE': str(root/'args.json')}
                result = subprocess.run(['bash', '-c', script], cwd=root, env=env, capture_output=True, text=True, timeout=15)
                self.assertEqual(result.returncode, 0, result.stderr)
                args = json.loads((root/'args.json').read_text())
                self.assertIn('--manager', args)
                if version == '0.161.0' and not bridge:
                    self.assertEqual(args[args.index('--manager-tui')+1], str(root/'manager-tui-input/artifact/codex'))
                    self.assertEqual(args[args.index('--manager-tui-version')+1], version)
                else: self.assertNotIn('--manager-tui', args)

    def test_actual_manual_deploy_selects_only_authenticated_qualified_same_version(self):
        # run blocks are dedented; preserve the real complete shell branch.
        script = 'if test "$MANAGER_TUI_SAME_VERSION_DEPLOY"' + step('Resolve official upstream stable').split('if test "$MANAGER_TUI_SAME_VERSION_DEPLOY"', 1)[1].split('# Historical explicitly selected fixtures', 1)[0]
        script = script.replace('${{ steps.stable.outputs.current_version }}', '${CURRENT_VERSION}')
        # The shell branch uses a single-quoted workflow expression.
        script = script.replace("'${CURRENT_VERSION}'", '"$CURRENT_VERSION"')
        for fault in (None, 'version', 'branch', 'publication', 'other-mode'):
            with self.subTest(fault=fault):
                env = {**os.environ, 'MANAGER_TUI_SAME_VERSION_DEPLOY':'true', 'GITHUB_EVENT_NAME':'workflow_dispatch', 'GITHUB_REF':'refs/heads/main', 'RALD5_PUBLICATION_AUTHORIZED':'true', 'CURRENT_VERSION':'0.161.0', 'MANAGER_TUI_VERSION':'0.161.0', 'MANAGER_TUI_ARCHIVE_SHA256':'a'*64}
                if fault == 'version': env['CURRENT_VERSION']='0.162.1'
                if fault == 'branch': env['GITHUB_REF']='refs/heads/rewrite/rust-core'
                if fault == 'publication': env['RALD5_PUBLICATION_AUTHORIZED']='false'
                if fault == 'other-mode': env['DOWNLOAD_SIZE_SAME_VERSION_DEPLOY']='true'
                result = subprocess.run(['bash','-ec',script + '\nprintf "%s %s %s\\n" "$candidate" "$upstream_version" "$archive_sha256"'], env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode == 0, fault is None, result.stderr)
                if fault is None: self.assertEqual(result.stdout, 'true 0.161.0 ' + 'a'*64 + '\n')

    def test_actual_bridge_is_exact_old_stable_and_requires_paired_deployment_authorization(self):
        body = step('Resolve official upstream stable')
        start = body.index('test "${MANAGER_TUI_BRIDGE_DEPLOY:-false}" != true')
        script = body[start:body.index('# Historical explicitly selected fixtures',start)]
        for field in ('current_version', 'current_generation', 'current_release_sequence', 'release_sequence'):
            script = script.replace("'${{ steps.stable.outputs."+field+" }}'", '"$' + field.upper() + '"')
        for fault in (None, 'standalone', 'generation', 'sequence', 'next-sequence'):
            with self.subTest(fault=fault):
                env={**os.environ, 'MANAGER_TUI_BRIDGE_DEPLOY':'true', 'MANAGER_TUI_SAME_VERSION_DEPLOY':'true', 'GITHUB_EVENT_NAME':'workflow_dispatch', 'GITHUB_REF':'refs/heads/main', 'RALD5_PUBLICATION_AUTHORIZED':'true', 'CURRENT_VERSION':'0.161.0', 'CURRENT_GENERATION':'local-hosted-0-161-0-053e35bee0cb', 'CURRENT_RELEASE_SEQUENCE':'41', 'RELEASE_SEQUENCE':'42', 'MANAGER_TUI_VERSION':'0.161.0', 'MANAGER_TUI_ARCHIVE_SHA256':'a'*64}
                if fault=='standalone':env['MANAGER_TUI_SAME_VERSION_DEPLOY']='false'
                if fault=='generation':env['CURRENT_GENERATION']='other'
                if fault=='sequence':env['CURRENT_RELEASE_SEQUENCE']='42'
                if fault=='next-sequence':env['RELEASE_SEQUENCE']='44'
                result=subprocess.run(['bash','-ec',script+'\nprintf "%s\\n" "$acceptance_suffix"'],env=env,text=True,capture_output=True)
                self.assertEqual(result.returncode==0,fault is None,result.stderr)
                if fault is None:self.assertEqual(result.stdout,'-manager-tui-bridge\n')

    def test_actual_native_smoke_requires_same_version_success_without_stderr(self):
        script = step('Run exact Manager, Core, and runtime smoke natively').split("if test '${{ needs.producer.outputs.upstream_version }}'", 1)[1].split('\nrm "$candidate/.manager-probe-deferred"', 1)[0]
        script = ("if test '0.161.0'" + script).replace('${{ needs.producer.outputs.manager_tui_bridge_deploy }}', 'false')
        for output, stderr, rc in [('codex-cli 0.161.0', '', 0), ('codex-cli 0.160.0', '', 0), ('codex-cli 0.161.0', 'failure', 0), ('codex-cli 0.161.0', '', 1)]:
            with self.subTest(output=output, stderr=stderr, rc=rc), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); (root/'helpers').mkdir(); native = root/'helpers/2'
                native.write_text('#!/bin/sh\nprintf "%s\\n" "' + output + '"\nprintf "%s" "' + stderr + '" >&2\nexit ' + str(rc) + '\n'); native.chmod(0o755)
                result = subprocess.run(['bash','-ec',script], env={**os.environ, 'candidate':str(root), 'RUNNER_TEMP':str(root), 'MANAGER_TUI_VERSION':'0.161.0'}, text=True, capture_output=True)
                self.assertEqual(result.returncode == 0, output == 'codex-cli 0.161.0' and not stderr and rc == 0, result.stderr)

    def test_actual_decision_retains_lkg_for_unqualified_ordinary_backend(self):
        script = step('Resolve official upstream stable').split('# Historical explicitly selected fixtures', 1)[1].split('\n', 1)[1].split('suffix="', 1)[0]
        for version, suffix, expected in [('0.161.0', '', 'true'), ('0.162.0', '', 'false'), ('0.154.0', '-acceptance', 'true')]:
            with self.subTest(version=version):
                result = subprocess.run(['bash', '-ec', script + '\nprintf "candidate=%s\\n" "$candidate"'],
                                        env={**os.environ, 'candidate':'true', 'acceptance_suffix':suffix, 'upstream_version':version, 'MANAGER_TUI_VERSION':'0.161.0'}, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(result.stdout.endswith('candidate=' + expected + '\n'), result.stdout)


if __name__ == '__main__': unittest.main()
