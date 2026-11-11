import unittest
from tui_regression_results import REQUIRED, verify


class TuiRegressionResultsTests(unittest.TestCase):
    def output(self, names=REQUIRED):
        return '\n'.join(f'  \x1b[32mPASS\x1b[0m [ 0.001s] codex-tui module::{name}' for name in names)

    def test_all_required_actual_passes_are_accepted(self):
        self.assertEqual(verify(self.output()), len(REQUIRED))

    def test_missing_compiled_skipped_failed_duplicate_and_partial_names_are_rejected(self):
        name = REQUIRED[-1]
        for replacement in ('', f'compiled {name}', f'  SKIP codex-tui module::{name}',
                            f'  FAIL codex-tui module::{name}', self.output([name, name]),
                            self.output([name + '_not_the_required_test'])):
            with self.subTest(replacement=replacement):
                with self.assertRaises(ValueError):
                    verify(self.output(REQUIRED[:-1]) + '\n' + replacement)
        with self.assertRaises(ValueError):
            verify(self.output() + '\nFAIL codex-tui module::' + name)


if __name__ == '__main__':
    unittest.main()
