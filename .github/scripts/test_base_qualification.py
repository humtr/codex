"""Base-only inventory and real workflow guards, with no publication authority."""
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import rald3_preflight as preflight
from test_manager_tui_producer import step


class BaseQualificationTests(unittest.TestCase):
    def fixture(self, root):
        (root / 'helpers').mkdir()
        assets = {'core': b'core', 'runtime': b'runtime',
                  'codex-code-mode-host': b'host', 'helpers/0': b'open', 'helpers/1': b'manual'}
        hashes = {}
        for name, data in assets.items():
            path = root / name
            path.write_bytes(data);path.chmod(0o755)
            hashes[name] = hashlib.sha256(data).hexdigest()
        fields = {
            'generation_id': 'base-qualification-owned',
            'upstream_package_identity': preflight.PACKAGE_IDENTITY,
            'upstream_package_version': '0.161.0',
            'source_artifact_digest': '3c02e2ae34be0d06e62557e98fc5c0a783bec5a2fed406fe00e565803bf84ee8',
            'expected_platform': 'android', 'expected_architecture': 'aarch64',
            'patch_policy_id': 'termux-fd-remap-v4',
            'patch_report': 'termux-fd-remap-v4;code_mode_host_sha256=' + hashes['codex-code-mode-host'],
            'runtime_digest': hashes['runtime'], 'core_artifact_digest': hashes['core'],
            'manager_artifact_digest': '-', 'core_api_identity': 'core-api-v1',
            'persistent_schema_identity': 'schema-v1', 'qualification': 'qualified',
            'creation_metadata': preflight.R10_METADATA, 'upstream_doctor': 'unsupported',
            'helper_count': '2',
        }
        text = preflight.GENERATION_FORMAT + '\n' + ''.join(k + '\t' + v + '\n' for k, v in fields.items())
        text += 'helper\ttermux-browser-open-v1\t' + hashes['helpers/0'] + '\n'
        text += 'helper\ttermux-browser-manual-v1\t' + hashes['helpers/1'] + '\n'
        (root / 'generation.meta').write_text(text);(root / 'generation.meta').chmod(0o644)

    def test_complete_base_passes_but_is_not_an_integrated_candidate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp);self.fixture(root)
            self.assertEqual(preflight.verify_base_candidate(root, '0.161.0')['manager_artifact_digest'], '-')
            with self.assertRaises(preflight.PreflightError):
                preflight.verify_qualified_candidate(root, '0.161.0')

    def test_base_refuses_changed_missing_extended_and_symlinked_assets(self):
        for fault in ('core', 'runtime', 'codex-code-mode-host', 'helpers/0', 'helpers/1',
                      'missing', 'symlink', 'extra', 'manager', 'deferred', 'mode', 'declaration'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp);self.fixture(root)
                if fault in ('core','runtime','codex-code-mode-host','helpers/0','helpers/1'):
                    (root / fault).write_bytes(b'changed')
                elif fault == 'missing':(root / 'helpers/1').unlink()
                elif fault == 'symlink':
                    (root / 'helpers/0').unlink();(root / 'helpers/0').symlink_to(root / 'helpers/1')
                elif fault == 'extra':(root / 'helpers/2').write_text('undeclared frontend')
                elif fault == 'manager':(root / 'manager').write_text('undeclared Manager')
                elif fault == 'deferred':(root / preflight.DEFERRED_MARKER).write_bytes(preflight.DEFERRED_MARKER_BYTES)
                elif fault == 'mode':(root / 'helpers/0').chmod(0o644)
                else:
                    p = root / 'generation.meta';p.write_text(p.read_text().replace('helper_count\t2','helper_count\t3'))
                with self.assertRaises((preflight.PreflightError,OSError)):
                    preflight.verify_base_candidate(root, '0.161.0')

    def test_source_qualification_and_rebuild_use_proposed_recipe_not_old_public_version(self):
        resolve = step('Resolve official upstream stable').split('          qualification_args=()', 1)[0]
        # step() dedents its literal shell, so stop before decision dispatch.
        resolve = resolve.split('qualification_args=()', 1)[0]
        resolve += '\nprintf "%s %s %s\\n" "$upstream_version" "$archive_sha256" "$REBUILD_CURRENT"\n'
        for base, current, expected_rebuild in [('true','0.163.0','false'),
                                                 ('false','0.163.0','false'),
                                                 ('false','0.164.0','true')]:
            with self.subTest(base=base,current=current), tempfile.TemporaryDirectory() as tmp:
                script = resolve.replace('${{ inputs.base_only }}', base)
                env = {**os.environ, 'RUNNER_TEMP':tmp,'REBUILD_CURRENT':'true',
                       'UPSTREAM_VERSION':'0.164.0','UPSTREAM_ARCHIVE_SHA256':'a'*64,
                       'CURRENT_VERSION':current,'CURRENT_ARCHIVE_SHA256':'b'*64}
                result = subprocess.run(['bash','-c',script],env=env,capture_output=True,text=True)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertEqual(result.stdout.strip(), '0.164.0 '+'a'*64+' '+expected_rebuild)

    def test_real_source_guard_refuses_publish_before_acquisition(self):
        script = step('Fetch exact accepted source').split('\nrm -rf source', 1)[0]
        for publish,ref,event,same,accepted in (
                ('false','refs/heads/rewrite/rust-core','push',True,True),
                ('false','refs/heads/rewrite/rust-core','workflow_dispatch',True,True),
                ('true','refs/heads/rewrite/rust-core','push',True,False),
                ('false','refs/heads/main','push',True,False),
                ('false','refs/heads/rewrite/rust-core','schedule',True,False),
                ('false','refs/heads/rewrite/rust-core','push',False,False)):
            with self.subTest(publish=publish,ref=ref,event=event,same=same):
                actual = script.replace('${{ inputs.publish }}', publish).replace('${{ inputs.base_only }}', 'true')
                env = {**os.environ,'BASE_ONLY':'true','CODEX_SOURCE_SHA':'a'*40,
                       'GITHUB_SHA':('a' if same else 'b')*40,'GITHUB_REF':ref,'GITHUB_EVENT_NAME':event}
                result = subprocess.run(['bash','-c',actual],env=env,capture_output=True,text=True)
                self.assertEqual(result.returncode == 0,accepted,result.stderr)

    def test_real_base_smoke_never_requires_manager_or_frontend(self):
        script = step('Run exact Manager, Core, and runtime smoke natively')
        script = script.replace('${{ needs.producer.outputs.upstream_version }}','0.161.0')
        usage = 'usage: codex update [--help] | --force | --build-local | --local <DIRECTORY> | --remote <HTTPS_BASE_URL> | --rollback'
        warning = ('linker: Warning: failed to find generated linker configuration from "/linkerconfig/ld.config.txt"\n'
                   'WARNING: linker: Warning: failed to find generated linker configuration from "/linkerconfig/ld.config.txt"\n')
        for fault in (None,'runtime-version','runtime-stderr','core-failure'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp);candidate = root/'rald3-smoke/candidate';candidate.mkdir(parents=True)
                tools = root/'tools';tools.mkdir()
                linker = root/'rald4-bionic/bin/linker64';linker.parent.mkdir(parents=True)
                linker.write_text('#!/bin/sh\nprintf \'%s\\n\' \''+usage+'\'\nprintf \'%s\' \''+warning+'\' >&2\nexit '+('1' if fault=='core-failure' else '0')+'\n')
                linker.chmod(0o755)
                runtime = candidate/'runtime'
                runtime.write_text('#!/bin/sh\necho codex-cli '+('0.162.0' if fault=='runtime-version' else '0.161.0')+'\n'+('echo failure >&2\n' if fault=='runtime-stderr' else ''))
                runtime.chmod(0o755)
                (tools/'uname').write_text('#!/bin/sh\necho aarch64\n');(tools/'uname').chmod(0o755)
                env={**os.environ,'BASE_ONLY':'true','RUNNER_TEMP':str(root),'PATH':str(tools)+':'+os.environ['PATH']}
                result=subprocess.run(['bash','-c',script],env=env,capture_output=True,text=True)
                self.assertEqual(result.returncode==0,fault is None,result.stderr)
                self.assertFalse((root/'frontend.out').exists())
                self.assertFalse((root/'manager.out').exists())


if __name__ == '__main__':
    unittest.main()
