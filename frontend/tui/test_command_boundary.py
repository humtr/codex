import unittest

from check_command_boundary import nonzero_tests, qualify


def source(variants, convention="kebab-case"):
    return f'#[strum(serialize_all = "{convention}")]\npub enum SlashCommand {{\n{variants}\n}}\n'


class CommandBoundaryTests(unittest.TestCase):
    def test_existing_names_and_aliases_survive_the_declared_extensions(self):
        original = 'Resume, Agents, Permissions,\n#[strum(to_string = "pwd", serialize = "cwd")]\nPwd,'
        self.assertEqual(qualify(source(original), source(original + '\nProvider, Switch,')),
                         {"upstream_variants": 4, "extensions": ["provider", "switch"],
                          "patched_inventory_verified": True})

    def test_collision_in_primary_alias_or_platform_hidden_name_refuses(self):
        for extension in ['Provider', 'Switch']:
            alias = extension.lower()
            for variant in [extension + ',', '#[strum(serialize = "' + alias + '")]\nOther,',
                            '#[strum(to_string = "' + alias + '", serialize = "other")]\nOther,',
                            '#[cfg(windows)]\n' + extension + ',']:
                with self.subTest(variant=variant), self.assertRaisesRegex(ValueError, 'upstream owns'):
                    qualify(source('Resume,\n' + variant))

    def test_renamed_removed_or_added_upstream_command_refuses(self):
        for patched in ['Resume, Provider, Switch,', 'Resume, Agents, Profile, Provider, Switch,',
                        'Resume, #[strum(serialize = "other")] Agents, Provider, Switch,',
                        'Resume, Agents, Switch,', 'Resume, Agents, Provider,',
                        'Resume, Agents, #[strum(serialize = "other")] Provider, Switch,']:
            with self.subTest(patched=patched), self.assertRaisesRegex(ValueError, 'changed upstream'):
                qualify(source('Resume, Agents,'), source(patched))

    def test_unknown_representation_or_empty_inventory_refuses(self):
        for text in [source('Resume,', 'snake_case'), source(''),
                     source('Resume(String),'), source('Resume, Resume,'), source('#[strum(unknown = "switch")] Other,'),
                     source('#[serde(rename = "switch")] Other,')]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                qualify(text)

    def test_native_proof_rejects_zero_failed_missing_or_ambiguous_test_set(self):
        valid = 'Summary [3.050s] 25 tests run: 25 passed, 5585 skipped'
        self.assertEqual(nonzero_tests(valid), 25)
        self.assertEqual(nonzero_tests('Summary [1s] 1 test run: 1 passed'), 1)
        self.assertEqual(nonzero_tests('Summary [1s] 113 tests run: 113 passed (1 slow), 2745 skipped'), 113)
        for log in ['', 'Summary [1s] 0 tests run: 0 passed',
                    'Summary [1s] 25 tests run: 24 passed, 1 failed', valid + '\n' + valid]:
            with self.subTest(log=log), self.assertRaises(ValueError):
                nonzero_tests(log)


if __name__ == "__main__":
    unittest.main()
