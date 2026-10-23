import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('backend_boundary', Path(__file__).with_name('check_backend_boundary.py'))
boundary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boundary)


class BoundaryTests(unittest.TestCase):
    def test_exact_guards_pass_but_extra_backend_edits_and_files_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                subprocess.run(['git', '-C', str(root), *args], check=True, capture_output=True)
            git('init')
            git('config', 'user.name', 'Owned fixture')
            git('config', 'user.email', 'owned@example.invalid')
            for name, (variable, indent) in boundary.GUARDS.items():
                path = root / name
                path.parent.mkdir(parents=True)
                path.write_text(' ' * indent + f'if {variable}.approval_policy == Some(AskForApproval::UnlessTrusted) {{\n  reject();\n}}\n')
            for name, replacements in boundary.TRANSFORMS.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('\n'.join(before for before, _ in replacements))
            git('add', '.')
            git('commit', '-m', 'official fixture')
            for name, (variable, indent) in boundary.GUARDS.items():
                path = root / name
                path.write_text(boundary.compatible(path.read_text(), variable, indent))
            for name, replacements in boundary.TRANSFORMS.items():
                path = root / name
                value = path.read_text()
                for before, after in replacements:
                    value = value.replace(before, after)
                path.write_text(value)
            boundary.verify(root)
            path = root / next(iter(boundary.GUARDS))
            original = path.read_text()
            path.write_text(original + 'unrelated_change();\n')
            with self.assertRaises(ValueError):
                boundary.verify(root)
            path.write_text(original)
            extra = root / 'codex-rs/core/src/extra.rs'
            extra.write_text('unowned source')
            with self.assertRaises(ValueError):
                boundary.verify(root)


if __name__ == '__main__':
    unittest.main()
