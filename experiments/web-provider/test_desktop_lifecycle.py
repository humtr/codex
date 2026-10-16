import json,os,socket,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
import desktop_lifecycle as life
from linux_desktop_trial import write

class LifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']);self.root=Path(self.temp.name);self.path=self.root/'runtime.json'
        self.runtime={'baseUrl':'http://127.0.0.1:12345/v1','token':'a'*64,'model':'chatgpt-desktop/instant','binding':str(self.root/'server.json'),'session':str(self.root/'session.json')}
        write(self.path,self.runtime)
    def tearDown(self):self.temp.cleanup()
    def test_foreign_listener_refuses_before_desktop_or_capability_send(self):
        with socket.socket() as foreign:
            foreign.bind(('127.0.0.1',0));foreign.listen();self.runtime['baseUrl']=f'http://127.0.0.1:{foreign.getsockname()[1]}/v1';write(self.path,self.runtime)
            with patch.object(life,'desktop') as boot:
                with self.assertRaisesRegex(RuntimeError,'owned_by_another'):life.ensure(self.path)
                boot.assert_not_called()
            self.assertEqual(life.record(self.path),self.runtime)
    def test_stale_pid_identity_refuses(self):
        stat=(Path('/proc')/str(os.getpid())/'stat').read_text();start=int(stat[stat.rfind(')')+2:].split()[19])
        marker=self.root/'owner.json';write(marker,{'pid':os.getpid(),'start':start+1,'image':os.readlink(f'/proc/{os.getpid()}/exe')})
        self.assertIsNone(life.live(marker))
    def test_descriptor_must_be_private_regular_file(self):
        self.path.chmod(0o644)
        with self.assertRaises(AssertionError):life.ensure(self.path)
        self.path.chmod(0o600);alias=self.root/'alias';alias.symlink_to(self.path)
        with self.assertRaises(AssertionError):life.ensure(alias)
    def test_live_ready_adapter_is_reused_and_no_spawn(self):
        with patch.object(life,'ready',return_value=True),patch.object(life,'desktop') as desktop,patch.object(life.subprocess,'Popen') as spawn:
            self.assertEqual(life.ensure(self.path),self.runtime);desktop.assert_called_once();spawn.assert_not_called()
    def test_failed_owned_adapter_is_preserved(self):
        with patch.object(life,'ready',return_value=False),patch.object(life,'live',return_value={'pid':os.getpid()}),patch.object(life,'desktop') as desktop:
            with self.assertRaisesRegex(RuntimeError,'live_adapter_not_ready'):life.ensure(self.path)
            desktop.assert_not_called()
    def test_retiring_pending_controller_exit_allows_successor_instead_of_readiness_timeout(self):
        import time
        from types import SimpleNamespace
        self.runtime.update(source=str(self.root),bun='owned-bun',proot='owned-proot',novnc='owned-novnc')
        pending=self.root/'bootstrap-controller.json';observations=[];started=[]
        def live(path):
            if path!=pending:return None
            observations.append(path)
            return {'pid':1} if len(observations)==1 else None
        def spawn(*args,**kwargs):
            started.append(True)
            return SimpleNamespace(pid=os.getpid(),poll=lambda:None)
        with patch.object(life,'live',side_effect=live),patch.object(life,'desktop_ready',side_effect=lambda _:bool(started)),patch.object(life.subprocess,'Popen',side_effect=spawn) as process:
            life.desktop(self.runtime,time.monotonic()+.1)
            process.assert_called_once()
        self.assertGreaterEqual(len(observations),2)
    def test_installed_transport_must_match_authenticated_binding(self):
        value={**self.runtime,'returnTransport':'text'}
        self.assertTrue(life.server_matches(self.runtime,value))
        configured={**self.runtime,'returnTransport':'local'}
        self.assertFalse(life.server_matches(configured,value))
        self.assertTrue(life.server_matches(configured,{**value,'returnTransport':'local'}))
    def test_failed_startup_settles_only_new_adapter(self):
        self.runtime.update(source=str(self.root),bun='owned-bun',bridge='owned-bridge');write(self.path,self.runtime)
        log=self.root/'adapter.log'
        with patch.object(life,'ready',return_value=False),patch.object(life,'live',return_value=None),patch.object(life,'port_free'),patch.object(life,'desktop'),patch.object(life,'wait_for',side_effect=RuntimeError('trial_startup_timeout')),patch.object(life.subprocess,'Popen') as spawn:
            spawn.return_value.poll.return_value=None
            with self.assertRaisesRegex(RuntimeError,'trial_startup_timeout'):life.ensure(self.path)
            spawn.return_value.terminate.assert_called_once();spawn.return_value.wait.assert_called_once_with(timeout=5)
        self.assertTrue(log.exists())
    def test_readiness_never_sends_bearer_to_unproven_listener(self):
        import http.server,threading
        seen=[]
        class Foreign(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                seen.append(dict(self.headers));self.send_response(200);self.end_headers();self.wfile.write(b'{"model":"chatgpt-desktop/instant","proof":"wrong"}')
            def log_message(self,*args):pass
        with http.server.HTTPServer(('127.0.0.1',0),Foreign) as server:
            self.runtime['baseUrl']=f'http://127.0.0.1:{server.server_port}/v1'
            thread=threading.Thread(target=server.handle_request);thread.start()
            with patch.object(life,'live',return_value=self.runtime):self.assertFalse(life.ready(self.runtime))
            thread.join(timeout=5)
        self.assertNotIn('Authorization',seen[0]);self.assertIn('X-Termux-Trial-Challenge',seen[0])
        self.assertNotIn(self.runtime['token'],json.dumps(seen))

    def test_auth_profile_timeout_and_permanent_helper(self):
        import desktop_provider_profile as profile
        home=self.root/'home';home.mkdir(mode=0o700);template=self.root/'models.json';template.write_text('{"models":[{}]}')
        self.runtime['source']=str(self.root/'installed/source');write(self.path,self.runtime)
        value=dict(self.runtime);value.update({'pid':os.getpid(),'start':0,'image':'image'})
        with patch.object(profile,'server_record',return_value=value),patch.dict(os.environ,{'HOME':str(self.root)}):profile.install(self.root/'server.json',home,template,self.path)
        import tomllib
        d=tomllib.loads((home/'chatgpt.config.toml').read_text())['model_providers']['chatgpt_desktop_trial']
        self.assertEqual(d['base_url'],self.runtime['baseUrl']);self.assertEqual(d['auth']['timeout_ms'],120000);self.assertEqual(d['auth']['args'][1],str(self.root/'installed/source/desktop_lifecycle.py'))

if __name__=='__main__':unittest.main()
