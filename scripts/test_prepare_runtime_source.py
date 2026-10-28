"""Source relocation and ambiguity checks, independent of release line numbers."""
import unittest

from scripts.prepare_runtime_source import function, replacements


class RuntimeSourceTests(unittest.TestCase):
    def test_relocation_changes_only_selected_function(self):
        original = '''// unrelated upstream additions
fn earlier() {
    let value = old_expression();
}

pub(crate) fn selected(
    input: bool,
) -> bool {
    if input {
        old_expression()
    } else {
        false
    }
}

fn later() {
    old_expression();
}
'''
        actual = function(original, 'selected', replacements([
            ('old_expression()', 'new_expression()'),
        ]), 'owned.rs')
        self.assertEqual(actual, original.replace('        old_expression()', '        new_expression()'))

    def test_missing_duplicate_or_changed_expression_names_responsible_function(self):
        for source in ['fn other() {\n}\n', 'fn selected() {\n}\n' * 2,
                       'fn selected() {\n    changed_expression();\n}\n']:
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, 'owned.rs.*selected'):
                function(source, 'selected', replacements([
                    ('old_expression()', 'new_expression()'),
                ]), 'owned.rs')


if __name__ == '__main__':
    unittest.main()
