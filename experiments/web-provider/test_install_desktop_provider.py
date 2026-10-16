import argparse,ast,hashlib,json,os,re,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import install_desktop_provider as installer
from linux_desktop_trial import write


class InstalledActivationTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']);self.root=Path(self.temp.name)
        self.path=self.root/'runtime.json';self.target=self.root/'immutable'
        self.old={'baseUrl':'http://127.0.0.1:12345/v1','model':'chatgpt-desktop/instant','token':'a'*64,
                  'binding':str(self.root/'server.json'),'source':'previous/source','returnTransport':'local'}
        write(self.path,self.old)
        self.args=argparse.Namespace(return_transport=None,catalog=self.root/'models.json',home=[])
    def tearDown(self):self.temp.cleanup()
    def activate(self,ensure):
        with patch.object(installer,'RUNTIME',self.path),patch.object(installer,'DATA',self.root),patch.object(installer,'live',return_value=None),patch.object(installer,'build',return_value=(self.target,{})),patch.object(installer,'ensure',ensure),patch.object(installer,'install') as profile,patch.object(installer,'private',return_value=self.root):
            self.profile=profile
            installer.activate(self.args)
    def test_reinstall_preserves_explicit_selection_and_endpoint(self):
        observed=[]
        self.activate(lambda path:observed.append(json.loads(path.read_text())))
        current=json.loads(self.path.read_text());self.assertEqual(current,observed[0])
        self.assertEqual(current['returnTransport'],'local')
        for key in ['baseUrl','model','token']:self.assertEqual(current[key],self.old[key])
        self.assertEqual(current['source'],str(self.target/'source'))
    def test_failed_startup_restores_previous_descriptor_before_profile_write(self):
        with self.assertRaisesRegex(RuntimeError,'startup failed'):
            self.activate(lambda _:(_ for _ in ()).throw(RuntimeError('startup failed')))
        self.profile.assert_not_called()
        self.assertEqual(json.loads(self.path.read_text()),self.old)

    def test_live_work_is_refused_before_build_or_descriptor_change(self):
        with patch.object(installer,'RUNTIME',self.path),patch.object(installer,'live',return_value={'pid':123}),patch.object(installer,'build') as build:
            with self.assertRaisesRegex(RuntimeError,'requires_stopped_adapter'):installer.activate(self.args)
            build.assert_not_called()
        self.assertEqual(json.loads(self.path.read_text()),self.old)
    def test_newly_started_work_is_preserved_after_build(self):
        with patch.object(installer,'RUNTIME',self.path),patch.object(installer,'DATA',self.root),patch.object(installer,'live',side_effect=[None,{'pid':123}]),patch.object(installer,'build',return_value=(self.target,{})),patch.object(installer,'ensure') as boot:
            with self.assertRaisesRegex(RuntimeError,'requires_stopped_adapter'):installer.activate(self.args)
            boot.assert_not_called()
        self.assertEqual(json.loads(self.path.read_text()),self.old)


class RuntimePackageTests(unittest.TestCase):
    def test_packaged_bootstrap_imports_without_development_files_or_user_state(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            root=Path(tmp);target=root/'source'
            installer.copy_runtime(installer.SOURCE,target)
            self.assertEqual({p.name for p in target.iterdir()},set(installer.RUNTIME_FILES))
            self.assertFalse(any('test' in p.name or 'probe' in p.name or 'qualification' in p.name for p in target.iterdir()))
            for p in target.iterdir():
                self.assertEqual(hashlib.sha256(p.read_bytes()).digest(),hashlib.sha256((installer.SOURCE/p.name).read_bytes()).digest())
                if p.suffix=='.py':
                    for node in ast.walk(ast.parse(p.read_text())):
                        names=[node.module] if isinstance(node,ast.ImportFrom) else [a.name for a in node.names] if isinstance(node,ast.Import) else []
                        for module in names:
                            name=(module or '').split('.')[0]+'.py'
                            if (installer.SOURCE/name).exists():self.assertTrue((target/name).is_file(),name)
                for name in re.findall(r'[\x27\x22]\./([^\x27\x22]+\.(?:mjs|js|css))[\x27\x22]',p.read_text()):
                    if (installer.SOURCE/name).exists():self.assertTrue((target/name).is_file(),name)
            home=root/'home';home.mkdir()
            r=subprocess.run([sys.executable,'-B','-c',
                'import desktop_lifecycle,desktop_provider_profile,install_desktop_provider,linux_desktop_trial'],
                cwd=target,env={**os.environ,'HOME':str(home),'PYTHONPATH':str(target),'PYTHONDONTWRITEBYTECODE':'1'},capture_output=True)
            self.assertEqual(r.returncode,0,r.stderr.decode())
            self.assertEqual(list(home.iterdir()),[])


if __name__=='__main__':unittest.main()
