import importlib.util,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest.mock import patch
import sys
SOURCE=Path(__file__).resolve().parent
sys.path.insert(0,str(SOURCE))
import desktop_provider_profile as profile

class ProfileTests(unittest.TestCase):
    def setUp(self):
        self.temporary=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']);self.root=Path(self.temporary.name)
        self.environment=patch.dict(os.environ,{'HOME':str(self.root)});self.environment.start()
        stat=(Path('/proc')/str(os.getpid())/'stat').read_text()
        self.value={'pid':os.getpid(),'start':stat[stat.rfind(')')+2:].split()[19],'image':os.readlink(Path('/proc')/str(os.getpid())/'exe'),'model':'chatgpt-desktop/instant','baseUrl':'http://127.0.0.1:12345/v1','token':'a'*64}
        self.marker=self.root/'server.json';self.save()
        self.home=self.root/'codex';self.home.mkdir(mode=0o700)
        self.config=self.home/'config.toml';self.original='model = "existing"\n# user settings\n[features]\nmemories = false\n';self.config.write_text(self.original);self.config.chmod(0o600)
        self.template=self.root/'catalog.json';self.template.write_text(json.dumps({'models':[{'slug':'example','display_name':'Example'}]}))
    def save(self):self.marker.write_text(json.dumps(self.value));self.marker.chmod(0o600)
    def tearDown(self):self.environment.stop();self.temporary.cleanup()
    def install(self):
        from contextlib import redirect_stdout
        import io
        with redirect_stdout(io.StringIO()):profile.install(self.marker,self.home,self.template)
    def test_standalone_profile_preserves_base_and_refreshes_only_owned_binding(self):
        self.install();self.assertEqual(self.config.read_text(),self.original)
        path=self.home/'chatgpt.config.toml';d=tomllib.loads(path.read_text());self.assertEqual(d['model_provider'],'chatgpt_desktop_trial');self.assertNotIn('profiles',d);self.assertEqual(path.stat().st_mode&0o777,0o600)
        self.value['baseUrl']='http://127.0.0.1:12346/v1';self.save();self.install();self.assertEqual(self.config.read_text(),self.original)
        self.assertEqual(tomllib.loads(path.read_text())['model_providers']['chatgpt_desktop_trial']['base_url'],self.value['baseUrl'])
    def test_existing_user_named_profile_refuses_without_mutation(self):
        target=self.home/'chatgpt.config.toml';target.write_text('model="user-choice"\n');target.chmod(0o600)
        with self.assertRaises(AssertionError):self.install()
        self.assertEqual(target.read_text(),'model="user-choice"\n');self.assertEqual(self.config.read_text(),self.original)
    def test_stale_owner_and_non_loopback_endpoint_refuse_before_config_writes(self):
        for changed in [{'start':str(int(self.value['start'])+1)},{'baseUrl':'https://example.invalid/v1'}]:
            original=dict(self.value);self.value.update(changed);self.save()
            with self.assertRaises(AssertionError):self.install()
            self.assertFalse((self.home/'chatgpt.config.toml').exists());self.assertEqual(self.config.read_text(),self.original)
            self.value=original;self.save()
    def test_symlink_and_nonprivate_local_capability_refuse(self):
        self.marker.chmod(0o644)
        with self.assertRaises(AssertionError):profile.server_record(self.marker)
        self.marker.chmod(0o600);alias=self.root/'alias';alias.symlink_to(self.marker)
        with self.assertRaises(AssertionError):profile.server_record(alias)
    def test_migration_removes_only_own_legacy_block(self):
        self.config.write_text(self.original+'\n'+profile.BEGIN+'\n[profiles.chatgpt]\nmodel="trial"\n'+profile.END+'\n')
        self.install();self.assertEqual(tomllib.loads(self.config.read_text()),tomllib.loads(self.original))

if __name__=='__main__':unittest.main()
