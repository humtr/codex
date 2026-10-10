"""Execute the public check script using disposable command witnesses."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECK = Path(__file__).with_name('check.sh').resolve()


class CheckTests(unittest.TestCase):
    def test_scope_cache_and_failures(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            tools = root / 'bin'; tools.mkdir()
            log = root / 'calls'
            for name in ('cargo', 'python3', 'git'):
                p = tools / name
                p.write_text('#!' + sys.executable + '\n' + '''\
import os,sys
from pathlib import Path
name=Path(sys.argv[0]).name
with open(os.environ['CALLS'],'a') as f: f.write(name+' '+' '.join(sys.argv[1:])+'\\n')
if name=='cargo':
    target=Path(os.environ['CARGO_TARGET_DIR']); target.mkdir(parents=True,exist_ok=True)
    (target/'retained').write_text('compiler cache')
if os.environ.get('FAIL_COMMAND')==name: sys.exit(19)
''')
                p.chmod(0o755)
            env = {**os.environ, 'PATH':str(tools)+os.pathsep+os.environ['PATH'],
                   'XDG_CACHE_HOME':str(root/'cache'), 'CALLS':str(log)}
            env.pop('CARGO_TARGET_DIR', None)
            for scope, cargo, python in [('docs',False,False),('python',False,True),('rust',True,False),('full',True,True)]:
                log.unlink(missing_ok=True)
                r=subprocess.run(['sh',str(CHECK),scope],env=env,capture_output=True,text=True)
                self.assertEqual(r.returncode,0,r.stderr)
                calls=log.read_text().splitlines()
                self.assertEqual(any(x.startswith('cargo ') for x in calls),cargo)
                self.assertEqual(any(x.startswith('python3 ') for x in calls),python)
                self.assertEqual(calls[-1],'git diff --check')
                if cargo:self.assertTrue((root/'cache/codex/check-target/retained').exists())
            for scope, command in [('rust','cargo'),('python','python3'),('docs','git')]:
                log.unlink()
                r=subprocess.run(['sh',str(CHECK),scope],env={**env,'FAIL_COMMAND':command})
                self.assertEqual(r.returncode,19)
                self.assertEqual(len(log.read_text().splitlines()),1)
            log.unlink()
            self.assertEqual(subprocess.run(['sh',str(CHECK),'bogus'],env=env,capture_output=True).returncode,2)
            self.assertFalse(log.exists())
            supplied=root/'explicit-target'
            self.assertEqual(subprocess.run(['sh',str(CHECK),'rust'],env={**env,'CARGO_TARGET_DIR':str(supplied)}).returncode,0)
            self.assertTrue((supplied/'retained').exists())


if __name__=='__main__': unittest.main()
