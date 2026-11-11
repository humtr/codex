"""Execute the real C fixture's failure paths; diagnostics never turn them into success."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
UUID='11a0fe94-2d11-79f3-8f5b-9c1958465dc1'

class StockFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp=tempfile.TemporaryDirectory(prefix='stock-c-')
        cls.root=Path(cls.temp.name)
        cls.base=cls.root/'proof'
        cls.base.mkdir(mode=0o700)
        cls.native=cls.root/'native'
        cls.binary=cls.root/'core'
        compiler=shutil.which('cc') or shutil.which('clang')
        if compiler is None: raise RuntimeError('C compiler is required for the stock fixture gate')
        subprocess.run([compiler,'-O2','-Wall','-Wextra','-Werror',
                        '-DBASE="'+str(cls.base)+'"','-DNATIVE="'+str(cls.native)+'"',
                        str(ROOT/'.github/stock-window/workload.c'),'-o',str(cls.binary)],check=True)
    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()
    def setUp(self):
        (self.base/'stages.log').unlink(missing_ok=True)
    def run_fixture(self,args,native=False):
        env=dict(os.environ,TMPDIR=str(self.root),SENTINEL_SECRET='never-log-environment')
        return subprocess.run([str(self.native if native else self.binary),*args],
                              executable=str(self.binary),env=env,capture_output=True,text=True,timeout=5)
    def stages(self):
        return (self.base/'stages.log').read_text()
    def test_dispatch_exec_failure_preserves_exit_and_logs_no_environment(self):
        result=self.run_fixture(['termux','test-command'])
        self.assertEqual(result.returncode,11)
        self.assertIn('stage=dispatch-exec detail=11',self.stages())
        self.assertNotIn('never-log-environment',self.stages())
        self.assertEqual((self.base/'stages.log').stat().st_mode&0o777,0o600)
    def test_launcher_retains_child_failure_without_ready_or_workload(self):
        result=self.run_fixture(['launcher',UUID])
        self.assertEqual(result.returncode,14)
        self.assertIn('stage=launcher-manager-exec detail=13',self.stages())
        self.assertIn('stage=launcher-manager-status detail=3328',self.stages())
        self.assertFalse((self.base/(UUID+'.ready')).exists())
        self.assertFalse((self.base/'workloads').exists())
    def test_native_bind_failure_preserves_unlinked_slot_and_no_false_ready(self):
        result=self.run_fixture([UUID],native=True)
        self.assertEqual(result.returncode,21)
        self.assertIn('stage=slot-ready',self.stages())
        self.assertIn('stage=bind-manager-exec detail=20',self.stages())
        self.assertIn('stage=bind-manager-status detail=5120',self.stages())
        self.assertEqual(list(self.root.glob('.codex-terminal-thread-v1-*')),[])
        self.assertFalse((self.base/(UUID+'.ready')).exists())
    def test_full_diagnostic_log_cannot_grow_or_change_failure_exit(self):
        content=b'x'*65536
        (self.base/'stages.log').write_bytes(content)
        self.assertEqual(self.run_fixture(['termux','test-command']).returncode,11)
        self.assertEqual((self.base/'stages.log').read_bytes(),content)

if __name__=='__main__':unittest.main()
