import copy
import json
from pathlib import Path
import tempfile
import unittest

from upstream_release import build_recipe, environment, load

AUTHORITY = Path(__file__).resolve().parents[2] / 'release/upstream.json'


class ReleaseInputsTests(unittest.TestCase):
    def test_frontend_and_core_share_exact_upstream_inputs(self):
        value = load(AUTHORITY)
        core, frontend = [environment(value, component) for component in ['core', 'frontend']]
        for key in ['UPSTREAM_VERSION', 'UPSTREAM_SHA', 'UPSTREAM_ARCHIVE_SHA256']:
            self.assertEqual(core[key], frontend[key])
        if value['frontend'] is None:
            self.assertNotIn('MANAGER_TUI_VERSION', core)
        else:
            self.assertEqual(core['MANAGER_TUI_VERSION'], core['UPSTREAM_VERSION'])
        self.assertEqual(frontend['RUSTUP_TOOLCHAIN'], frontend['RUST_VERSION'])
        self.assertTrue(all('\n' not in item and '\r' not in item for item in core.values()))

    def test_missing_frontend_does_not_invent_qualified_inputs(self):
        value = load(AUTHORITY)
        value['frontend'] = None
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'input.json';path.write_text(json.dumps(value))
            result = environment(load(path), 'core')
        self.assertNotIn('MANAGER_TUI_VERSION', result)
        self.assertIn('UPSTREAM_VERSION', result)

    def test_changed_or_injectable_authority_refuses_before_output(self):
        baseline = load(AUTHORITY)
        if baseline['frontend'] is None:
            baseline['frontend'] = {'source':'a'*40,'run_id':'123','sha256':'b'*64,
                'input_url':'https://github.com/humtr/codex/releases/download/owned/inputs.tar.gz',
                'input_sha256':'c'*64}
        mutations = [('schema', None, True), ('upstream', 'version', '0.0163.0'),
                     ('upstream', 'source', 'main'), ('upstream', 'archive_sha256', 'A'*64),
                     ('toolchains', 'core_rust', '1.98.0\nEVIL=yes'),
                     ('frontend', 'run_id', '0'), ('frontend', 'input_url', 'https://example.com/a.tar.gz')]
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'input.json'
            for section, key, bad in mutations:
                with self.subTest(section=section, key=key):
                    value = copy.deepcopy(baseline)
                    if key is None:value[section] = bad
                    else:value[section][key] = bad
                    path.write_text(json.dumps(value))
                    with self.assertRaises(ValueError):load(path)
            path.write_text('{"schema":1,"schema":1}')
            with self.assertRaisesRegex(ValueError, 'duplicate'):load(path)

    def test_frontend_references_are_outputs_but_recipe_changes_invalidate_proof(self):
        baseline = load(AUTHORITY)
        changed = copy.deepcopy(baseline)
        changed['frontend'] = None
        self.assertEqual(build_recipe(baseline), build_recipe(changed))
        for section, field, value in [('upstream', 'source', 'b'*40),
                                      ('toolchains', 'frontend_rust', '1.96.0'),
                                      ('qualification', 'focused_tests', 44)]:
            changed = copy.deepcopy(baseline)
            changed[section][field] = value
            self.assertNotEqual(build_recipe(baseline), build_recipe(changed))

    def test_workflows_load_authority_before_builds(self):
        root = AUTHORITY.parents[1]
        for name, checkout in [('manager-tui-native.yml', 'experiment'), ('qualify-termux-runtime.yml', 'source')]:
            text = (root/'.github/workflows'/name).read_text()
            self.assertIn(f'{checkout}/.github/scripts/upstream_release.py {checkout}/release/upstream.json', text)
            self.assertNotRegex(text, r'MANAGER_TUI_VERSION: [\"\']?[0-9]')
            self.assertNotIn('UPSTREAM_SHA:', text)


if __name__ == '__main__':unittest.main()
