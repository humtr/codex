import argparse,json,os,tempfile,unittest
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


if __name__=='__main__':unittest.main()
