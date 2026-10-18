"""Real filesystem and notification-tap routing with no live Android activity."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from urllib.parse import quote

SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('linux_trial', SOURCE/'linux_desktop_trial.py')
linux = importlib.util.module_from_spec(spec); spec.loader.exec_module(linux)
import open_linux_browser as panel


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
        wrapped = 'https://chatgpt.com/codex/desktop-auth?authorize_url='
        self.assertTrue(linux.authorization_url(wrapped+quote('https://auth.openai.com/oauth/authorize?state=fixture', safe='')))
        self.assertFalse(linux.authorization_url(wrapped+quote('https://example.invalid/oauth/authorize', safe='')))
        self.assertFalse(linux.authorization_url(wrapped+quote('https://auth.openai.com/oauth/token', safe='')))
        self.assertFalse(linux.authorization_url(wrapped+quote(wrapped, safe='')))
        self.assertFalse(linux.authorization_url(wrapped+quote('https://auth.openai.com/oauth/authorize', safe='')+'&authorize_url=other'))
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
            fake=prefix/'bin/termux-am-socket';fake.write_text('#!'+sys.executable+'\nimport os,sys,json\nfrom pathlib import Path\nPath(os.environ["TAP_PROOF"]).write_text(json.dumps({"argv":sys.argv[1:]}))\nprint("Starting: owned activity")\n');fake.chmod(0o700)
            text=(Path('/proc')/str(os.getpid())/'stat').read_text();start=int(text[text.rfind(')')+2:].split()[19]);image=os.readlink(Path('/proc')/str(os.getpid())/'exe')
            v={'pid':os.getpid(),'start':start,'image':image,'port':12001,'capability':'1'*64};p=data/'viewer.json';p.write_text(json.dumps(v));p.chmod(0o600)
            proof=root/'proof.json';env={**os.environ,'HOME':str(home),'PREFIX':str(prefix),'TAP_PROOF':str(proof)}
            command=[sys.executable,'-B',str(SOURCE/'open_dual_trial.py'),'plan']
            first=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertEqual(first.returncode,0);self.assertIn(':12001/',json.loads(proof.read_text())['argv'][0])
            v['port']=12002;p.write_text(json.dumps(v));second=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertEqual(second.returncode,0);self.assertIn(':12002/',json.loads(proof.read_text())['argv'][0])
            v['start']+=1;p.write_text(json.dumps(v));proof.unlink();stale=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertNotEqual(stale.returncode,0);self.assertFalse(proof.exists())


    def test_browser_window_selection_excludes_other_profiles_and_hidden_helpers(self):
        tree = '\n'.join([
            '0xe00002 "about:blank": ("chromium (/root/browser-profile)" "Chromium") 458x669+0+0',
            '0xe00003 "other": ("chromium (/root/unrelated-profile)" "Chromium") 480x743+0+0',
            '0xe00004 "helper": ("chromium (/root/browser-profile)" "Chromium") 10x10+0+0',
            '0xe00005 "app": ("chatgpt (/root/profile)" "Chatgpt") 480x743+0+0'])
        self.assertEqual(linux.browser_windows(tree), ['0xe00002'])

    def test_panel_queues_linux_browser_with_explicit_marker_and_refuses_stale_owner(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            root=Path(temporary);root.chmod(0o700)
            stat=(Path('/proc')/str(os.getpid())/'stat').read_text();start=int(stat[stat.rfind(')')+2:].split()[19])
            state={'pid':os.getpid(),'start':start,'image':os.readlink(Path('/proc')/str(os.getpid())/'exe'),'root':str(root),'display':139}
            marker=root/'session.json';marker.write_text(json.dumps(state));marker.chmod(0o600)
            guest=root/'guest-home';guest.mkdir();env={**os.environ,'HOME':str(guest),'FIXTURE_ROOT':str(root),'FIXTURE_OWNER':str(os.getpid())}
            prefix=root/'prefix';(prefix/'bin').mkdir(parents=True)
            fake='''#!'''+sys.executable+'''
import os,sys,json
from pathlib import Path
r=Path(os.environ['FIXTURE_ROOT']);n=Path(sys.argv[0]).name
if n=='xwininfo':
 if (r/'open-browser.json').exists():print('0xe00002 "fixture": ("chromium (/root/browser-profile)" "Chromium") 480x743+0+0')
elif n=='xprop':
 if '-root' in sys.argv:print('_NET_ACTIVE_WINDOW(WINDOW): window id # 0xe00002')
 else:print('_NET_WM_PID(CARDINAL) = '+os.environ['FIXTURE_OWNER']+'\\nWM_STATE(WM_STATE):\\n    window state: Normal')
elif n=='xdotool':
 with (r/'actions.jsonl').open('a') as f:f.write(json.dumps(sys.argv[1:])+'\\n')
'''
            for name in ['xwininfo','xprop','xdotool','wmctrl']:
                executable=prefix/'bin'/name;executable.write_text(fake);executable.chmod(0o700)
            linux.write(root/'environment.json',{'PREFIX':str(prefix),'DISPLAY':':139','XAUTHORITY':str(root/'xauthority')})
            command=[sys.executable,'-B',str(SOURCE/'open_linux_browser.py'),str(marker)]
            for _ in range(2):
                valid=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertEqual(valid.returncode,0,valid.stderr.decode())
            self.assertIn('minimize',(root/'actions.jsonl').read_text())
            request=root/'open-browser.json';self.assertEqual(json.loads(request.read_text()),{'show':True});self.assertEqual(request.stat().st_mode & 0o777,0o600)
            request.unlink();state['start']+=1;marker.write_text(json.dumps(state))
            stale=subprocess.run(command,env=env,capture_output=True,timeout=5);self.assertNotEqual(stale.returncode,0);self.assertFalse(request.exists())

    def test_panel_excludes_withdrawn_dialog_and_foreign_windows_with_identical_class(self):
        tree='\n'.join(f'0x{w} "private title": ("chatgpt (/root/profile)" "Chatgpt") 480x743+0+0'
                       for w in ['10','20','30','40'])
        def command(env, *args):
            if args[0]=='xwininfo':
                text=tree
            else:
                w=args[2];text='_NET_WM_PID(CARDINAL) = '+('999' if w=='0x40' else str(os.getpid()))
                if w!='0x20':text+='\nwindow state: Normal'
                if w=='0x30':text+='\nWM_TRANSIENT_FOR(WINDOW): window id # 0x10'
            return SimpleNamespace(returncode=0,stdout=text)
        with patch.object(panel,'command',side_effect=command):
            self.assertEqual(panel.windows({}, {'pid':os.getpid()}, 'chatgpt'), ['0x10'])

    def test_duplicate_launch_tap_does_not_spawn_or_touch_the_display(self):
        import fcntl
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temporary:
            root=Path(temporary);root.chmod(0o700)
            state={'pid':os.getpid(),'root':str(root)}
            with (root/'.panel-browser.lock').open('w') as lock:
                lock_path=Path(lock.name);lock_path.chmod(0o600)
                fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
                with patch.object(panel,'record',return_value=state),patch.object(panel,'owner'),patch.object(panel,'command') as command,patch.object(panel.subprocess,'Popen') as spawn:
                    self.assertEqual(panel.toggle(root/'session.json'),'pending')
                    command.assert_not_called();spawn.assert_not_called()


if __name__=='__main__':unittest.main()
