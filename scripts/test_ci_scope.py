import unittest
import subprocess
import sys
from pathlib import Path
from scripts.ci_scope import scope


class ScopeTests(unittest.TestCase):
    def test_changed_paths(self):
        for paths,want in [(['README.md','CURRENT.md'],'docs'),([], 'docs'),
                           (['scripts/check.sh','SPEC.md'],'python'),
                           (['.github/scripts/rald4_signing.py'],'python'),
                           (['scripts/test_ci_scope.py','crates/core/src/main.rs'],'full'),
                           (['Cargo.lock'],'full'),(['install.sh'],'full'),
                           (['.github/workflows/auto-release-termux.yml'],'full'),
                           (['unknown'], 'full')]:
            with self.subTest(paths=paths):self.assertEqual(scope(paths),want)

    def test_nul_paths_without_shell_splitting(self):
        script=Path(__file__).with_name('ci_scope.py')
        result=subprocess.run([sys.executable,str(script),'--stdin'],input=b'NOTES with spaces.md\0crates/odd name.rs\0',capture_output=True)
        self.assertEqual(result.returncode,0)
        self.assertEqual(result.stdout,b'full\n')


if __name__=='__main__':unittest.main()
