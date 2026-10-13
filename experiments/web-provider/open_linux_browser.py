"""Reopen this candidate's validated sign-in URL in the Android browser."""
import json
import os
from pathlib import Path
import subprocess
import sys
from linux_desktop_trial import authorization_url
from open_dual_trial import record, owner


def main():
    data = Path.home()/'.local/share/codex-web-experiment/linux-desktop'
    assert len(sys.argv) in (1, 2)
    marker = Path(sys.argv[1]) if len(sys.argv) == 2 else data/'session.json'
    state = record(marker); owner(state)
    root = Path(state['root'])
    target = root/'browser-target.json'
    url = record(target)['url'] if target.exists() else 'https://chatgpt.com/'
    assert url == 'https://chatgpt.com/' or authorization_url(url)
    result = subprocess.run([os.environ['PREFIX']+'/bin/termux-am-socket',
                             'start -a android.intent.action.VIEW -d '+url],
                            capture_output=True, text=True, timeout=20)
    ok = result.returncode == 0 and 'Error:' not in result.stdout and not result.stderr
    print(json.dumps({'browserRequested': ok}))
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
