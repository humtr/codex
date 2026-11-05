"""Provider-owned mobile desktop; private account home, one CDP pipe and OS handoff."""
import ctypes
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlparse, parse_qs

PANEL_HEIGHT = 42

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

def browser_windows(tree):
    # Chromium custom profiles do not always appear in _NET_CLIENT_LIST here.
    # Inspect this private display and match only the owned profile's WM_CLASS.
    pattern = r'(0x[0-9a-f]+)[^\n]*\("chromium \(/root/browser-profile\)" "Chromium"\)\s+(\d+)x(\d+)'
    return [match[0] for match in re.findall(pattern, tree)
            if int(match[1]) > 100 and int(match[2]) > 100]

def panel_config(root, prefix):
    action = shlex.join([str(prefix/'bin/python3'), str(root/'desktop.py'), '--browser-request', str(root)])
    config = root/'tint2rc'
    config.write_text(f"""rounded = 0
border_width = 0
background_color = #20252c 100
panel_items = PTC
panel_size = 100% {PANEL_HEIGHT}
panel_position = bottom center horizontal
panel_padding = 6 4 6
panel_background_id = 1
panel_layer = top
panel_dock = 0
strut_policy = follow_size
autohide = 0
taskbar_mode = single_desktop
taskbar_padding = 4 0 4
task_text = 1
task_icon = 0
task_font = sans 11
task_font_color = #ffffff 100
task_active_font_color = #ffffff 100
task_maximum_size = 250 34
mouse_left = toggle_iconify
button = new
button_text = Browser
button_tooltip = Open Chromium in this Linux desktop
button_font = sans bold 11
button_font_color = #ffffff 100
button_padding = 10 0 0
button_lclick_command = {action}
time1_format = %H:%M
time1_font = sans 11
clock_font_color = #ffffff 100
clock_padding = 8 0
""")
    config.chmod(0o600)
    return config


def fit_window(env, window):
    lib = ctypes.CDLL(str(Path(env['PREFIX'])/'lib/libX11.so'))
    lib.XOpenDisplay.argtypes = [ctypes.c_char_p]
    lib.XOpenDisplay.restype = ctypes.c_void_p
    class Hints(ctypes.Structure):
        _fields_ = [('flags',ctypes.c_long)]+[(name,ctypes.c_int) for name in
            ['x','y','width','height','min_width','min_height','max_width','max_height',
             'width_inc','height_inc','min_aspect_x','min_aspect_y','max_aspect_x',
             'max_aspect_y','base_width','base_height','win_gravity']]
    lib.XGetWMNormalHints.argtypes = [ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(Hints),ctypes.POINTER(ctypes.c_long)]
    lib.XSetWMNormalHints.argtypes = [ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(Hints)]
    lib.XSync.argtypes = [ctypes.c_void_p,ctypes.c_int]
    lib.XCloseDisplay.argtypes = [ctypes.c_void_p]
    display = lib.XOpenDisplay(env['DISPLAY'].encode())
    if not display:
        raise RuntimeError('desktop_display_unavailable')
    try:
        hints, supplied = Hints(), ctypes.c_long()
        handle = int(window, 16)
        if lib.XGetWMNormalHints(display,handle,ctypes.byref(hints),ctypes.byref(supplied)) and hints.flags & (1 << 4):
            hints.min_width = hints.min_height = 1
            lib.XSetWMNormalHints(display,handle,ctypes.byref(hints))
            lib.XSync(display,False)
    finally:
        lib.XCloseDisplay(display)
    subprocess.run([str(Path(env['PREFIX'])/'bin/xdotool'),'windowmap',window],env=env,check=True,capture_output=True,timeout=3)
    subprocess.run([str(Path(env['PREFIX'])/'bin/wmctrl'),'-i','-r',window,'-b','add,maximized_vert,maximized_horz'],env=env,check=True,capture_output=True,timeout=3)
    subprocess.run([str(Path(env['PREFIX'])/'bin/xdotool'),'windowraise',window,'windowfocus',window],env=env,check=True,capture_output=True,timeout=3)


def serve(home, root, prefix, display):
    os.umask(0o077)
    configure_browser(home)
    processes = []
    env = {'HOME':str(home),'PATH':str(prefix/'bin'),'PREFIX':str(prefix),'TMPDIR':str(root),
           'DISPLAY':':'+str(display),'XAUTHORITY':str(root/'Xauthority'),'LANG':'C.UTF-8'}
    os.environ.update({key:env[key] for key in ['DISPLAY','XAUTHORITY']})
    opener = private(root/'bin')/'xdg-open'
    opener.write_text('#!/bin/sh\nset -eu\ntest "$#" = 1\numask 077\nprintf "%s" "$1" > /ui/open-url.tmp\nmv /ui/open-url.tmp /ui/open-url\n')
    opener.chmod(0o700)
    parent = os.getpid()
    def parent_lifetime():
        libc = ctypes.CDLL(None,use_errno=True)
        if libc.prctl(1,signal.SIGKILL,0,0,0) != 0 or os.getppid() != parent:
            raise OSError('desktop_parent_unavailable')
    def start(command, environment=env, *, pipe=False):
        child = subprocess.Popen(command,env=environment,stdin=subprocess.DEVNULL,
                                 stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,
                                 pass_fds=(3,4) if pipe else (), preexec_fn=parent_lifetime)
        processes.append(child)
        return child
    def guest(program, args):
        return [str(prefix/'bin/proot'),'--kill-on-exit','--link2symlink','--sysvipc','-L',
                '--change-id=0:0','--rootfs=.','--cwd=/root','--bind=/dev','--bind=/proc','--bind=/sys',
                '--bind=/dev/urandom:/dev/random',f'--bind={home}:/root',f'--bind={root}:/ui',
                f'--bind={root/"tmp"}:/tmp',f'--bind={root/"shm"}:/dev/shm',
                f'--bind={prefix/"tmp/.X11-unix"}:/tmp/.X11-unix','/usr/bin/env','-i','HOME=/root',
                'PATH=/ui/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin',
                'LANG=C.UTF-8',f'DISPLAY=:{display}','XAUTHORITY=/ui/Xauthority','BROWSER=/ui/bin/xdg-open',program,*args]
    guest_env = {**env,'PROOT_TMP_DIR':str(root),'PROOT_L2S_DIR':str(root/'links')}
    def windows():
        return subprocess.run([str(prefix/'bin/xwininfo'),'-root','-tree'],env=env,
                              capture_output=True,text=True,check=True,timeout=3).stdout
    try:
        start([str(prefix/'bin/openbox')])
        start([str(prefix/'bin/tint2'),'-c',str(panel_config(root,prefix))])
        app = start(guest('/usr/bin/dbus-run-session',['--','/usr/bin/chatgpt','--no-sandbox','--disable-gpu',
                    '--disable-dev-shm-usage','--user-data-dir=/root/profile','--remote-debugging-pipe']),guest_env,pipe=True)
        os.close(3);os.close(4)
        browser=None
        app_window=None
        while app.poll() is None:
            match=re.search(r'(0x[0-9a-f]+)[^\n]*\("chatgpt \(/root/profile\)" "Chatgpt"\)\s+(\d+)x(\d+)',windows())
            if match and int(match[2]) > 100 and int(match[3]) > 100 and match[1] != app_window:
                fit_window(env,match[1]);app_window=match[1]
            requested=root/'open-browser.json'
            if requested.exists():
                requested.unlink()
                if browser is None or browser.poll() is not None:
                    geometry=subprocess.run([str(prefix/'bin/xwininfo'),'-root'],env=env,capture_output=True,text=True,check=True,timeout=3).stdout
                    width=int(re.search(r'Width: (\d+)',geometry)[1]);height=int(re.search(r'Height: (\d+)',geometry)[1])
                    browser=start(guest('/usr/bin/chromium',['--no-sandbox','--disable-gpu','--disable-dev-shm-usage',
                        '--no-first-run','--no-default-browser-check','--password-store=basic',
                        '--user-data-dir=/root/browser-profile','--force-device-scale-factor=0.9',
                        f'--window-size={width},{height-PANEL_HEIGHT}','about:blank']),guest_env)
                deadline=time.monotonic()+30
                while True:
                    found=browser_windows(windows())
                    if found:
                        fit_window(env,found[0]);break
                    if browser.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError('desktop_browser_window_unavailable')
                    time.sleep(.1)
            queued=root/'open-url'
            if queued.exists():
                url=queued.read_text();queued.unlink()
                opened=False
                if authorization_url(url):
                    result=subprocess.run([str(prefix/'bin/termux-am-socket'),
                        'start -a android.intent.action.VIEW -d '+url],env=env,capture_output=True,timeout=20)
                    opened=result.returncode==0 and b'Error:' not in result.stdout and not result.stderr
                write(root/'browser-handoff.json',{'opened':opened})
            time.sleep(.25)
    finally:
        for process in reversed(processes):
            if process.poll() is None: process.terminate()
        for process in reversed(processes):
            try: process.wait(timeout=2)
            except subprocess.TimeoutExpired: process.kill();process.wait(timeout=2)



if __name__=='__main__':
    if sys.argv[1:2]==['--browser-request']:
        root=private(Path(sys.argv[2]));write(root/'open-browser.json',{'show':True})
    else:
        signal.signal(signal.SIGTERM,lambda *_: sys.exit(0))
        serve(*(Path(value) for value in sys.argv[1:4]),int(sys.argv[4]))
