"""Owned Linux desktop candidate; private account data survives disposable OS tests."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shlex
import signal
import struct
import subprocess
import tempfile
import time
from urllib.parse import urlparse, parse_qs

VERSION = '26.1007.21434'
DIGEST = '55fae1e3fb1d6069a90620e9e70737dcf8bf2255604d157e282cc6748a1baf49'
PROOT_SOURCE = '5a427a1f924550c176ab34197e9fcf5d010fdbd9'
SOURCE = Path(__file__).resolve().parent


def private(path, *, tighten=False):
    path.mkdir(parents=True, exist_ok=True, mode=0o700)
    st = path.lstat()
    assert path.is_dir() and not path.is_symlink() and st.st_uid == os.getuid()
    if tighten:
        path.chmod(0o700)
    else:
        assert not st.st_mode & 0o077
    return path


def write(path, value):
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix='.state-')
    try:
        with os.fdopen(fd, 'w') as f:
            json.dump(value, f); f.write('\n'); f.flush(); os.fsync(f.fileno())
        os.replace(tmp, path)
    finally:
        if Path(tmp).exists(): Path(tmp).unlink()


def configure_browser(home):
    applications = private(home/'.local/share/applications', tighten=True)
    desktop = applications/'codex-termux-auth-browser.desktop'
    desktop.write_text('[Desktop Entry]\nType=Application\nName=Termux authentication browser\nExec=/ui/bin/xdg-open %u\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\n')
    desktop.chmod(0o600)
    configuration = private(home/'.config', tighten=True)/'mimeapps.list'
    lines = configuration.read_text().splitlines() if configuration.exists() else []
    result=[]; in_defaults=False; added=False
    for line in lines:
        if line.startswith('[') and in_defaults and not added:
            result+=['x-scheme-handler/http=codex-termux-auth-browser.desktop','x-scheme-handler/https=codex-termux-auth-browser.desktop']; added=True
        if line=='[Default Applications]': in_defaults=True
        elif line.startswith('['): in_defaults=False
        if not (in_defaults and line.startswith(('x-scheme-handler/http=','x-scheme-handler/https='))): result.append(line)
    if not added:
        if not in_defaults: result.append('[Default Applications]')
        result+=['x-scheme-handler/http=codex-termux-auth-browser.desktop','x-scheme-handler/https=codex-termux-auth-browser.desktop']
    fd, temporary=tempfile.mkstemp(dir=configuration.parent,prefix='.mime-')
    try:
        with os.fdopen(fd,'w') as f:f.write('\n'.join(result)+'\n');f.flush();os.fsync(f.fileno())
        os.replace(temporary,configuration)
    finally:
        if Path(temporary).exists():Path(temporary).unlink()



def authorization_url(url, *, nested=False):
    try:
        parsed = urlparse(url)
        query = parse_qs(parsed.query, keep_blank_values=True)
        if (len(url) >= 16384 or any(c.isspace() for c in url) or parsed.scheme != 'https'
                or parsed.port not in (None, 443) or parsed.username or parsed.password or parsed.fragment
                or {'id_token', 'access_token', 'refresh_token'}.intersection(query)):
            return False
        if parsed.hostname == 'auth.openai.com':
            return parsed.path == '/oauth/authorize' or parsed.path.startswith('/api/accounts/')
        if not nested and parsed.hostname == 'chatgpt.com' and parsed.path == '/codex/desktop-auth':
            targets = query.get('authorize_url', [])
            return len(targets) == 1 and authorization_url(targets[0], nested=True)
        return False
    except ValueError:
        return False


def panel_config(root, prefix, rootfs):
    marker = Path.home()/'.local/share/codex-web-experiment/linux-desktop/session.json'
    buttons = []
    # Read official package assets already installed on this device. Do not
    # redraw the brands or redistribute the proprietary app's artwork.
    icons = [('chatgpt', 'ChatGPT', rootfs/'usr/share/pixmaps/chatgpt.png'),
             ('browser', 'Chromium', rootfs/'usr/share/icons/hicolor/48x48/apps/chromium.png')]
    for app, label, image in icons:
        assert image.is_file(), 'installed_app_icon_required'
        action = shlex.join([str(prefix/'bin/python'), '-B', str(SOURCE/'open_linux_browser.py'), str(marker), app])
        buttons.append(f'''button = new
button_icon = {image}
button_max_icon_size = 28
button_tooltip = {label}: open / minimize / restore
button_padding = 12 4
button_lclick_command = {action}
''')
    config = root/'tint2rc'
    config.write_text(f"""rounded = 0
border_width = 0
background_color = #20252c 100
panel_items = PP
panel_size = 100% 42
panel_position = bottom center horizontal
panel_padding = 6 0 4
panel_background_id = 1
panel_layer = top
panel_dock = 0
strut_policy = follow_size
autohide = 0
mouse_effects = 1
{''.join(buttons)}
""")
    config.chmod(0o600)
    return config



def browser_windows(tree):
    # Chromium custom profiles do not always appear in _NET_CLIENT_LIST here.
    # Inspect this private display and match only the owned profile's WM_CLASS.
    pattern = r'(0x[0-9a-f]+)[^\n]*\("chromium \(/root/browser-profile\)" "Chromium"\)\s+(\d+)x(\d+)'
    return [match[0] for match in re.findall(pattern, tree)
            if int(match[1]) > 100 and int(match[2]) > 100]


def paths(args):
    prefix = Path(os.environ['PREFIX'])
    cache = private(Path.home()/'.cache/codex-web-experiment/linux-desktop'/VERSION)
    data = private(Path.home()/'.local/share/codex-web-experiment/linux-desktop')
    tool = args.proot_source.resolve(strict=True)
    exported=tool/'.codex-provider-source.json'
    if exported.exists():
        from open_dual_trial import record
        manifest=record(exported);assert manifest['commit']==PROOT_SOURCE
        actual={str(p.relative_to(tool)):hashlib.sha256(p.read_bytes()).hexdigest() for p in tool.rglob('*') if p.is_file() and p!=exported and '__pycache__' not in p.parts}
        assert actual==manifest['files'], 'installed proot source changed'
    else:
        assert subprocess.check_output(['git', '-C', str(tool), 'rev-parse', 'HEAD'], text=True).strip() == PROOT_SOURCE
        assert not subprocess.check_output(['git', '-C', str(tool), 'status', '--porcelain'], text=True).strip()
    env = {'HOME': str(private(cache/'setup-home')), 'TMPDIR': str(prefix/'tmp'), 'PATH': str(prefix/'bin'),
           'TERMUX__PREFIX': str(private(cache/'trial-prefix')), 'TERMUX__HOME': str(cache/'setup-home'),
           'PD_PROOT_BIN': str(prefix/'bin/proot'), 'PYTHONPATH': str(tool), 'LANG': 'C.UTF-8', 'PD_FORCE_NO_COLORS': '1'}
    base = [str(prefix/'bin/python'), '-c', 'from proot_distro.cli import main; main()']
    return prefix, cache, data, env, base


def prepare(args):
    prefix, cache, data, env, base = paths(args)
    archive = cache/'chatgpt_arm64.deb'
    with archive.open('rb') as f: digest = hashlib.file_digest(f, 'sha256').hexdigest()
    assert digest == DIGEST, 'accepted public package changed'
    rootfs = cache/'trial-prefix/var/lib/proot-distro/containers/chatgpt-trial/rootfs'
    with (cache/'prepare.log').open('w') as out:
        if not rootfs.exists(): subprocess.run(base+['install', '--name', 'chatgpt-trial', 'debian:13-slim'], env=env, stdout=out, stderr=out, check=True, timeout=180)
        command = '''printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d
chmod 755 /usr/sbin/policy-rc.d
apt-get update -o Acquire::Retries=2 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends /candidate/chatgpt_arm64.deb ca-certificates chromium
'''
        subprocess.run(base+['login', '--isolated', '--bind', str(cache)+':/candidate', 'chatgpt-trial', '--', '/bin/sh', '-c', command], env=env, stdout=out, stderr=out, check=True, timeout=900)
    private(data/'home')
    write(data/'candidate.json', {'version': VERSION, 'sha256': DIGEST, 'prootSource': PROOT_SOURCE, 'rootfs': str(rootfs), 'production': False})
    print(json.dumps({'candidatePrepared': True, 'version': VERSION, 'liveProfilesUsed': False}))



def start_candidate(args):
    _, _, data, _, _ = paths(args)
    command = [os.environ['PREFIX']+'/bin/python', '-B', str(SOURCE/'linux_desktop_trial.py'), 'serve',
               '--proot-source', str(args.proot_source), '--bun', str(args.bun),
               '--novnc', str(args.novnc), '--seconds', str(args.seconds), *(['--inspect'] if args.inspect else [])]
    with (data/'controller.log').open('a') as output:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output,
                                   stderr=output, start_new_session=True)
    print(json.dumps({'candidateControllerStarted': True, 'pid': process.pid,
                      'automaticExpiry': bool(args.seconds), 'production': False}))


def serve(args):
    prefix, cache, data, env, base = paths(args)
    assert (data/'candidate.json').is_file(), 'prepare the candidate first'
    from desktop_lifecycle import live
    if live(data/'session.json'):
        raise RuntimeError('another recorded candidate controller is alive')
    lock_fd = os.open(data/'.session-lock', os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW, 0o600)
    fcntl.flock(lock_fd, fcntl.LOCK_EX|fcntl.LOCK_NB)
    processes = []
    with tempfile.TemporaryDirectory(prefix='ld-', dir=os.environ['TMPDIR']) as temp:
        root = private(Path(temp)); home = private(data/'home')
        configure_browser(home)
        display = next(n for n in range(128, 256) if not (prefix/'tmp'/f'.X{n}-lock').exists() and not (prefix/'tmp/.X11-unix'/f'X{n}').exists())
        def field(v): return struct.pack('!H', len(v))+v
        auth = root/'xauthority'
        auth.write_bytes(struct.pack('!H',65535)+field(b'localhost')+field(str(display).encode())+field(b'MIT-MAGIC-COOKIE-1')+field(secrets.token_bytes(16))); auth.chmod(0o600)
        native = {'HOME': str(home), 'TMPDIR': str(prefix/'tmp'), 'PATH': str(prefix/'bin'), 'PREFIX': str(prefix), 'DISPLAY': f':{display}', 'XAUTHORITY': str(auth), 'LANG': 'C.UTF-8'}
        write(root/'environment.json', native)
        opener = private(root/'bin')/'xdg-open'
        opener.write_text('#!/bin/sh\nset -eu\ntest "$#" = 1\numask 077\nprintf "%s" "$1" > /ui/open-url.tmp\nmv /ui/open-url.tmp /ui/open-url\n')
        opener.chmod(0o700)
        def start(command, environment, log):
            with (root/log).open('w') as out: p = subprocess.Popen(command, env=environment, stdout=out, stderr=out, start_new_session=True)
            processes.append(p); return p
        try:
            x = start([str(prefix/'bin/Xvnc'), f':{display}', '-geometry', '480x785', '-depth', '24', '-rfbunixpath', str(root/'rfb.sock'), '-rfbunixmode', '0600', '-rfbport', '-1', '-SecurityTypes', 'None', '-auth', str(auth), '-nolisten', 'tcp'], native, 'xvnc.log')
            for _ in range(80):
                if (root/'rfb.sock').exists(): break
                assert x.poll() is None; time.sleep(.1)
            else: raise RuntimeError('private display unavailable')
            start([str(prefix/'bin/openbox')], native, 'openbox.log')
            rootfs = cache/'trial-prefix/var/lib/proot-distro/containers/chatgpt-trial/rootfs'
            start([str(prefix/'bin/tint2'), '-c', str(panel_config(root, prefix, rootfs))], native, 'panel.log')
            viewer = start([str(args.bun), str(SOURCE/'termux_viewer.ts'), str(args.novnc), str(root/'rfb.sock'), str(root/'viewer.json')], native, 'viewer.log')
            cmd = base+['login', '--isolated', '--bind', str(prefix/'tmp/.X11-unix')+':/tmp/.X11-unix', '--bind', str(root)+':/ui', '--bind', str(home)+':/root', '--env', f'DISPLAY=:{display}', '--env', 'XAUTHORITY=/ui/xauthority', '--env', 'BROWSER=/ui/bin/xdg-open', '--env', 'PATH=/ui/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin', 'chatgpt-trial', '--', '/usr/bin/dbus-run-session', '--', '/usr/bin/chatgpt', '--no-sandbox', '--disable-gpu', '--disable-dev-shm-usage', '--user-data-dir=/root/profile']
            if args.inspect:
                command = root/'app-command.json'
                write(command, {'argv': cmd, 'environment': env})
                private(home/'profile', tighten=True)
                app = start([str(args.bun), str(SOURCE/'native_chromium_host.mjs'), str(home/'profile'),
                             str(root/'desktop-host.json'), '--guest-desktop', str(command)], native, 'app.log')
            else:
                app = start(cmd, env, 'app.log')
            for _ in range(160):
                assert app.poll() is None and viewer.poll() is None, 'candidate exited before rendering'
                tree = subprocess.run([str(prefix/'bin/xwininfo'), '-root', '-tree'], env=native, capture_output=True, text=True, timeout=3).stdout
                match = re.search(r'(0x[0-9a-f]+) "ChatGPT".*?\b(?:[2-9]\d{2,}|1\d{3,})x(?:[2-9]\d{2,}|1\d{3,})\+', tree)
                if match and (root/'viewer.json').exists(): break
                time.sleep(.5)
            else: raise RuntimeError('candidate window did not appear')
            subprocess.run([str(prefix/'bin/wmctrl'), '-i', '-r', match[1], '-b', 'add,maximized_vert,maximized_horz'], env=native, check=True, capture_output=True, timeout=5)
            stat = (Path('/proc')/str(os.getpid())/'stat').read_text()
            write(data/'session.json', {'pid': os.getpid(), 'start': int(stat[stat.rfind(')')+2:].split()[19]), 'image': os.readlink(Path('/proc')/str(os.getpid())/'exe'), 'root': str(root), 'display': display, 'seconds': args.seconds, 'started': time.time(), 'production': False})
            action = shlex.join([str(prefix/'bin/python'), '-B', str(SOURCE/'open_dual_trial.py'), 'linux'])
            subprocess.run([str(prefix/'bin/termux-notification'), '--id', 'codex-linux-login', '--group', 'codex-web-experiment', '--title', 'Linux 앱 화면 복귀', '--content', '한 번 터치해 Linux 화면을 여세요. 하단 아이콘으로 앱을 열거나 최소화·복귀합니다.', '--action', action, '--priority', 'high'], capture_output=True, timeout=20, check=True)
            print(json.dumps({'linuxViewerReady': True, 'notificationSent': True, 'production': False}), flush=True)
            browser = None
            def show_browser():
                nonlocal browser
                if browser is None or browser.poll() is not None:
                    browser_cmd = base+['login', '--isolated', '--bind', str(prefix/'tmp/.X11-unix')+':/tmp/.X11-unix',
                                        '--bind', str(root)+':/ui', '--bind', str(home)+':/root',
                                        '--env', f'DISPLAY=:{display}', '--env', 'XAUTHORITY=/ui/xauthority',
                                        'chatgpt-trial', '--', '/usr/bin/chromium', '--no-sandbox', '--disable-gpu',
                                        '--disable-dev-shm-usage', '--no-first-run', '--no-default-browser-check',
                                        '--password-store=basic', '--user-data-dir=/root/browser-profile',
                                        '--force-device-scale-factor=0.9', '--window-size=480,743', 'about:blank']
                    browser = start(browser_cmd, env, 'browser.log')
                for _ in range(150):
                    listing = subprocess.run([str(prefix/'bin/xwininfo'), '-root', '-tree'], env=native, capture_output=True, text=True, timeout=3).stdout
                    windows = browser_windows(listing)
                    if windows:
                        subprocess.run([str(prefix/'bin/wmctrl'), '-i', '-r', windows[0], '-b', 'add,maximized_vert,maximized_horz'], env=native, capture_output=True, timeout=3)
                        subprocess.run([str(prefix/'bin/xdotool'), 'windowraise', windows[0], 'windowfocus', windows[0]], env=native, capture_output=True, timeout=3)
                        write(data/'browser-proof.json', {'running': browser.poll() is None, 'windows': len(windows), 'androidActivityRequested': False})
                        return
                    if browser.poll() is not None: break
                    time.sleep(.2)
                write(data/'browser-proof.json', {'running': browser.poll() is None, 'windows': 0, 'androidActivityRequested': False})
            end = time.monotonic()+args.seconds if args.seconds else None
            while (end is None or time.monotonic() < end) and app.poll() is None:
                request = root/'open-browser.json'
                if request.exists():
                    request.unlink()
                    show_browser()
                queued = root/'open-url'
                if queued.exists():
                    url = queued.read_text(); queued.unlink()
                    if authorization_url(url):
                        # Only the temporary private desktop retains the pending URL.
                        write(root/'browser-target.json', {'url': url})
                        # OAuth state/code challenges stay out of chat and logs.
                        result = subprocess.run([str(prefix/'bin/termux-am-socket'), 'start -a android.intent.action.VIEW -d '+url], env=native, capture_output=True, text=True, timeout=20)
                        ok = result.returncode == 0 and 'Error:' not in result.stdout and not result.stderr
                        write(data/'browser-handoff.json', {'time': time.time(), 'opened': ok, 'exit': result.returncode, 'activityStarted': 'Starting:' in result.stdout})
                    else: write(data/'browser-handoff.json', {'time': time.time(), 'opened': False, 'unrecognizedTarget': True})
                time.sleep(.5)
        finally:
            for p in reversed(processes):
                if p.poll() is None:
                    os.killpg(p.pid, signal.SIGTERM)
                    try: p.wait(timeout=8)
                    except subprocess.TimeoutExpired: os.killpg(p.pid, signal.SIGKILL); p.wait(timeout=5)
            subprocess.run([str(prefix/'bin/termux-notification-remove'), 'codex-linux-login'], capture_output=True, timeout=15)
            write(data/'session.json', {'running': False, 'production': False, 'stopped': time.time()})
            os.close(lock_fd)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['prepare', 'serve', 'start'])
    parser.add_argument('--proot-source', type=Path, required=True)
    parser.add_argument('--bun', type=Path)
    parser.add_argument('--novnc', type=Path)
    parser.add_argument('--inspect', action='store_true', help='Expose only private CDP pipes through the authenticated owned relay')
    parser.add_argument('--seconds', type=int, default=0, help='0 keeps the foreground candidate open until closed; positive values bound a trial')
    args = parser.parse_args()
    assert 0 <= args.seconds <= 3600
    if args.command in ('serve', 'start'): assert args.bun and args.novnc
    if args.command == 'serve':
        def stop(signum, frame):
            raise SystemExit(0)
        signal.signal(signal.SIGTERM, stop)
    {'prepare': prepare, 'serve': serve, 'start': start_candidate}[args.command](args)
