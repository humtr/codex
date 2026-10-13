"""Resolve the current owned viewer at notification-tap time, not alert creation."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from open_viewer import viewer_url


def record(path):
    state = path.lstat()
    assert path.is_file() and not path.is_symlink() and state.st_uid == os.getuid() and not state.st_mode & 0o077
    return json.loads(path.read_text())


def owner(state):
    pid = state['pid']; assert type(pid) is int and pid > 0
    root = Path('/proc')/str(pid); text = (root/'stat').read_text()
    fields=text[text.rfind(')')+2:].split()
    assert fields[0]!='Z' and int(fields[19]) == state['start']
    assert os.readlink(root/'exe') == state['image']


def main():
    assert sys.argv[1:] in (['linux'], ['plan'])
    route = sys.argv[1]; data = Path.home()/'.local/share/codex-web-experiment'
    if route == 'linux':
        state = record(data/'linux-desktop/session.json'); owner(state)
        viewer = record(Path(state['root'])/'viewer.json'); url = viewer_url(viewer)
    else:
        viewer = record(data/'siwc/viewer.json'); owner(viewer)
        assert type(viewer['port']) is int and 0 < viewer['port'] < 65536
        assert re.fullmatch('[0-9a-f]{64}', viewer['capability'])
        url = f'http://127.0.0.1:{viewer["port"]}/{viewer["capability"]}/'
    prefix = Path(os.environ['PREFIX'])
    result = subprocess.run([str(prefix/'bin/termux-am-socket'), 'start -a android.intent.action.VIEW -d '+url], capture_output=True, text=True, timeout=20)
    ok = result.returncode == 0 and 'Error:' not in result.stdout and not result.stderr
    log = data/('linux-desktop' if route == 'linux' else 'siwc')/'notification-open.jsonl'
    with log.open('a') as output: output.write(json.dumps({'route': route, 'exit': result.returncode, 'activityStarted': 'Starting:' in result.stdout, 'ok': ok})+'\n')
    log.chmod(0o600)
    print(json.dumps({'route': route, 'activityRequested': ok}))
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
