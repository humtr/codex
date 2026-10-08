import unittest

from check_command_boundary import nonzero_tests, qualify


def source(variants, convention="kebab-case"):
    return f'#[strum(serialize_all = "{convention}")]\npub enum SlashCommand {{\n{variants}\n}}\n'


class CommandBoundaryTests(unittest.TestCase):
    def test_existing_names_and_aliases_survive_the_only_new_command(self):
        original = 'Resume, Agents, Permissions,\n#[strum(to_string = "pwd", serialize = "cwd")]\nPwd,'
        self.assertEqual(qualify(source(original), source(original + '\nSwitch,')),
                         {"upstream_variants": 4, "extension": "switch",
                          "patched_inventory_verified": True})

    def test_collision_in_primary_alias_or_platform_hidden_name_refuses(self):
        for variant in ['Switch,', '#[strum(serialize = "switch")]\nOther,',
                        '#[strum(to_string = "switch", serialize = "other")]\nOther,',
                        '#[cfg(windows)]\nSwitch,']:
            with self.subTest(variant=variant), self.assertRaisesRegex(ValueError, 'upstream owns'):
                qualify(source('Resume,\n' + variant))

    def test_renamed_removed_or_added_upstream_command_refuses(self):
        for patched in ['Resume, Switch,', 'Resume, Agents, Profile, Switch,',
                        'Resume, #[strum(serialize = "other")] Agents, Switch,']:
            with self.subTest(patched=patched), self.assertRaisesRegex(ValueError, 'changed upstream'):
                qualify(source('Resume, Agents,'), source(patched))

    def test_unknown_representation_or_empty_inventory_refuses(self):
        for text in [source('Resume,', 'snake_case'), source(''),
                     source('Resume(String),'), source('#[strum(unknown = "switch")] Other,'),
                     source('#[serde(rename = "switch")] Other,')]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                qualify(text)

    def test_native_proof_rejects_zero_failed_missing_or_ambiguous_test_set(self):
        valid = 'Summary [3.050s] 25 tests run: 25 passed, 5585 skipped'
        self.assertEqual(nonzero_tests(valid), 25)
        for log in ['', 'Summary [1s] 0 tests run: 0 passed',
                    'Summary [1s] 25 tests run: 24 passed, 1 failed', valid + '\n' + valid]:
            with self.subTest(log=log), self.assertRaises(ValueError):
                nonzero_tests(log)


if __name__ == "__main__":
    unittest.main()
