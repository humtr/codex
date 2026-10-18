"""Single-tap app switching for the retained private qualification desktop."""
import fcntl
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from linux_desktop_trial import private, write, browser_windows
from open_dual_trial import record, owner


def command(env, *args):
    return subprocess.run([str(Path(env['PREFIX'])/'bin'/args[0]), *args[1:]],
                          env=env, capture_output=True, text=True, timeout=3)


def descendant(pid, controller):
    for _ in range(48):
        if pid == controller:
            return True
        if pid <= 1:
            break
        try:
            raw = Path(f'/proc/{pid}/stat').read_text()
            pid = int(raw[raw.rfind(')')+2:].split()[1])
        except (OSError, ValueError):
            break
    return False


def windows(env, state, app):
    tree = command(env, 'xwininfo', '-root', '-tree')
    if tree.returncode:
        raise RuntimeError('private_display_unavailable')
    if app == 'browser':
        found = browser_windows(tree.stdout)
    else:
        found = [w for w, width, height in re.findall(
            r'(0x[0-9a-f]+)[^\n]*\("chatgpt \(/root/profile\)" "Chatgpt"\)\s+(\d+)x(\d+)',
            tree.stdout) if int(width) > 100 and int(height) > 100]
    owned = []
    for w in found:
        prop = command(env, 'xprop', '-id', w, '_NET_WM_PID', 'WM_STATE', 'WM_TRANSIENT_FOR')
        match = re.search(r'_NET_WM_PID\(CARDINAL\) = (\d+)', prop.stdout)
        # Electron keeps withdrawn pre-created windows with the same WM_CLASS.
        # Only the WM-managed main window is a switch target. Dialogs stay with it.
        managed = re.search(r'window state: (Normal|Iconic)', prop.stdout)
        transient = re.search(r'WM_TRANSIENT_FOR\(WINDOW\)', prop.stdout)
        if match and managed and not transient and descendant(int(match[1]), state['pid']):
            owned.append(w)
    if len(owned) > 1:
        raise RuntimeError('ambiguous_owned_window')
    return owned


def action_for(active, iconic):
    return 'minimize' if active and not iconic else 'restore'


def switch(env, window):
    prop = command(env, 'xprop', '-id', window, 'WM_STATE')
    iconic = 'Iconic' in prop.stdout
    active = command(env, 'xprop', '-root', '_NET_ACTIVE_WINDOW')
    ids = re.findall(r'0x[0-9a-f]+', active.stdout)
    focused = bool(ids and int(ids[-1], 16) == int(window, 16))
    action = action_for(focused, iconic)
    if action == 'minimize':
        result = command(env, 'xdotool', 'windowminimize', window)
    else:
        # Keep this app on the current desktop. Never change the user's desktop,
        # start a new conversation or force fullscreen over the pinned controls.
        result = command(env, 'wmctrl', '-i', '-r', window, '-t', '0')
        if result.returncode == 0:
            result = command(env, 'wmctrl', '-i', '-a', window)
    if result.returncode:
        raise RuntimeError('owned_window_switch_failed')
    return action


def toggle(marker, app='browser'):
    assert app in ('browser', 'chatgpt')
    state = record(marker); owner(state)
    root = private(Path(state['root']))
    fd = os.open(root/f'.panel-{app}.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        assert os.fstat(fd).st_uid == os.getuid() and not os.fstat(fd).st_mode & 0o077
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return 'pending'
        env = record(root/'environment.json')
        assert env['XAUTHORITY'] == str(root/'xauthority')
        assert env['DISPLAY'] == ':'+str(state['display'])
        env = {**os.environ, **env}
        found = windows(env, state, app)
        if found:
            return switch(env, found[0])
        if app == 'browser':
            write(root/'open-browser.json', {'show': True})
        else:
            # Electron's existing profile singleton opens its window. Use the
            # controller's exact command, never a new profile or debug endpoint.
            invocation = record(root/'app-command.json')
            argv = invocation['argv']
            assert argv[0] == env['PREFIX']+'/bin/python'
            assert '/usr/bin/chatgpt' in argv and '--user-data-dir=/root/profile' in argv
            assert '--isolated' in argv
            with (root/'panel-app.log').open('a') as log:
                subprocess.Popen(argv, env=invocation['environment'], stdin=subprocess.DEVNULL,
                                 stdout=log, stderr=log, start_new_session=True)
        # Wait only on cold launch. Existing windows never enter this poll loop;
        # the lock coalesces repeated taps while the controller is starting it.
        until = time.monotonic()+30
        while time.monotonic() < until:
            owner(state)
            found = windows(env, state, app)
            if found:
                command(env, 'wmctrl', '-i', '-r', found[0], '-b', 'add,maximized_vert,maximized_horz')
                command(env, 'wmctrl', '-i', '-a', found[0])
                return 'started'
            time.sleep(.1)
        raise RuntimeError('owned_window_start_pending')
    finally:
        os.close(fd)


def main():
    data = Path.home()/'.local/share/codex-web-experiment/linux-desktop'
    assert len(sys.argv) in (1, 2, 3)
    marker = Path(sys.argv[1]) if len(sys.argv) >= 2 else data/'session.json'
    app = sys.argv[2] if len(sys.argv) == 3 else 'browser'
    print(json.dumps({'app': app, 'action': toggle(marker, app)}))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
