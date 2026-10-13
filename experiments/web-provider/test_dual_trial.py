"""Real filesystem and notification-tap routing with no live Android activity."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('linux_trial', SOURCE/'linux_desktop_trial.py')
linux = importlib.util.module_from_spec(spec); spec.loader.exec_module(linux)


class TrialTests(unittest.TestCase):
    def test_candidate_browser_override_preserves_codex_and_other_associations(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            home=Path(temporary);(home/'.config').mkdir(mode=0o755)
            m=home/'.config/mimeapps.list';m.write_text('[Default Applications]\nx-scheme-handler/codex=chatgpt.desktop\ntext/plain=editor.desktop\n[Added Associations]\ntext/plain=editor.desktop;\n')
            linux.configure_browser(home);before=m.read_text();linux.configure_browser(home)
            self.assertEqual((home/'.config').stat().st_mode & 0o777,0o700);self.assertEqual(before,m.read_text());self.assertIn('x-scheme-handler/codex=chatgpt.desktop',before);self.assertIn('text/plain=editor.desktop;',before)
            self.assertEqual(before.count('x-scheme-handler/https=codex-termux-auth-browser.desktop'),1)
            self.assertEqual(before.count('x-scheme-handler/http=codex-termux-auth-browser.desktop'),1)

    def test_official_desktop_authorize_path_is_forwarded_without_accepting_other_targets(self):
        self.assertTrue(linux.authorization_url('https://auth.openai.com/oauth/authorize?response_type=code&code_challenge=fixture'))
        self.assertTrue(linux.authorization_url('https://auth.openai.com/api/accounts/authorize?state=fixture'))
        for url in ['http://auth.openai.com/oauth/authorize',
                    'https://auth.openai.com.example.invalid/oauth/authorize',
                    'https://auth.openai.com:8443/oauth/authorize',
                    'https://user@auth.openai.com/oauth/authorize',
                    'https://auth.openai.com/oauth/token',
                    'https://auth.openai.com/oauth/authorize#secret',
                    'https://auth.openai.com/oauth/authorize?%61ccess_token=fixture',
                    'https://auth.openai.com/oauth/authorize?access_token=',
                    'https://auth.openai.com/oauth/authorize?state=fixture -n other.app']:
            with self.subTest(url=url):
                self.assertFalse(linux.authorization_url(url))

    def test_tap_resolves_current_endpoint_and_refuses_reused_process(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            root=Path(temporary);home=root/'h';prefix=root/'p';data=home/'.local/share/codex-web-experiment/siwc';data.mkdir(parents=True,mode=0o700);(prefix/'bin').mkdir(parents=True)
            fake=prefix/'bin/termux-am-socket';fake.write_text('#!'+os.environ['PREFIX']+'/bin/python\nimport os,sys,json\nfrom pathlib import Path\nPath(os.environ["TAP_PROOF"]).write_text(json.dumps({"argv":sys.argv[1:]}))\nprint("Starting: owned activity")\n');fake.chmod(0o700)
            text=(Path('/proc')/str(os.getpid())/'stat').read_text();start=int(text[text.rfind(')')+2:].split()[19]);image=os.readlink(Path('/proc')/str(os.getpid())/'exe')
            v={'pid':os.getpid(),'start':start,'image':image,'port':12001,'capability':'1'*64};p=data/'viewer.json';p.write_text(json.dumps(v));p.chmod(0o600)
            proof=root/'proof.json';env={**os.environ,'HOME':str(home),'PREFIX':str(prefix),'TAP_PROOF':str(proof)}
            command=[os.environ['PREFIX']+'/bin/python','-B',str(SOURCE/'open_dual_trial.py'),'plan']
            first=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertEqual(first.returncode,0);self.assertIn(':12001/',json.loads(proof.read_text())['argv'][0])
            v['port']=12002;p.write_text(json.dumps(v));second=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertEqual(second.returncode,0);self.assertIn(':12002/',json.loads(proof.read_text())['argv'][0])
            v['start']+=1;p.write_text(json.dumps(v));proof.unlink();stale=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertNotEqual(stale.returncode,0);self.assertFalse(proof.exists())


if __name__=='__main__':unittest.main()
