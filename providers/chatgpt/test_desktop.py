import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest

SOURCE = Path(__file__).parent/'src/desktop.py'
spec = importlib.util.spec_from_file_location('provider_desktop', SOURCE)
desktop = importlib.util.module_from_spec(spec)
spec.loader.exec_module(desktop)


class DesktopTests(unittest.TestCase):
    def test_auth_handoff_preserves_account_files_and_other_mime_handlers(self):
        with tempfile.TemporaryDirectory() as temporary:
            home=Path(temporary)
            (home/'.config').mkdir()
            (home/'.config/mimeapps.list').write_text('[Default Applications]\nx-scheme-handler/http=old.desktop\nx-scheme-handler/custom=keep.desktop\n')
            (home/'.codex').mkdir()
            auth=home/'.codex/auth.json'
            auth.write_bytes(b'owned-test-account-state')
            desktop.configure_browser(home)
            config=(home/'.config/mimeapps.list').read_text()
            self.assertIn('x-scheme-handler/http=codex-termux-auth-browser.desktop',config)
            self.assertIn('x-scheme-handler/https=codex-termux-auth-browser.desktop',config)
            self.assertIn('x-scheme-handler/custom=keep.desktop',config)
            self.assertEqual(auth.read_bytes(),b'owned-test-account-state')
            entry=(home/'.local/share/applications/codex-termux-auth-browser.desktop').read_text()
            self.assertIn('Exec=/ui/bin/xdg-open %u',entry)

    def test_oauth_handoff_accepts_the_existing_app_protocol(self):
        for url in ['https://auth.openai.com/oauth/authorize?client_id=fixture&redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback',
                    'https://auth.openai.com/api/accounts/authorize',
                    'https://chatgpt.com/codex/desktop-auth?authorize_url=https%3A%2F%2Fauth.openai.com%2Foauth%2Fauthorize']:
            self.assertTrue(desktop.authorization_url(url),url)
        for url in ['http://auth.openai.com/oauth/authorize','https://auth.openai.com.attacker.test/oauth/authorize',
                    'https://auth.openai.com/oauth/authorize?access_token=fixture',
                    'https://user@auth.openai.com/oauth/authorize','https://auth.openai.com/oauth/authorize#fragment',
                    'https://chatgpt.com/codex/desktop-auth?authorize_url=https%3A%2F%2Fexample.test%2F']:
            self.assertFalse(desktop.authorization_url(url),url)

    def test_taskbar_browser_action_reaches_its_current_desktop(self):
        with tempfile.TemporaryDirectory(prefix='desktop action ') as temporary:
            root=Path(temporary)
            shutil.copyfile(SOURCE,root/'desktop.py')
            prefix=Path(os.environ.get('PREFIX',str(Path(sys.executable).parent.parent)))
            panel=desktop.panel_config(root,prefix).read_text()
            action=next(line.split('=',1)[1].strip() for line in panel.splitlines() if line.startswith('button_lclick_command ='))
            subprocess.run(shlex.split(action),check=True,capture_output=True)
            self.assertEqual(json.loads((root/'open-browser.json').read_text()),{'show':True})
            self.assertFalse((root/'profile').exists())


if __name__=='__main__':
    unittest.main()
