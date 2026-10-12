"""Fit only normal Chromium windows owned by the private viewer helper."""
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def fit_browser(root, report=None):
    root = Path(root).resolve(strict=True)
    env = json.loads((root / 'environment.json').read_text())
    owner = json.loads((root / 'account-view-process.json').read_text())
    process = Path('/proc') / str(owner['pid'])
    assert os.readlink(process / 'exe') == owner['image']
    assert process.joinpath('stat').read_text().rsplit(')', 1)[1].split()[19] == str(owner['start'])
    assert env['HOME'].startswith(str(root) + '/') and env['XAUTHORITY'].startswith(str(root) + '/')
    # XOpenDisplay uses these to authenticate; none of their values are logged.
    previous = {key: os.environ.get(key) for key in ('DISPLAY', 'XAUTHORITY')}
    os.environ.update({key: env[key] for key in previous})
    lib = ctypes.CDLL(env['PREFIX'] + '/lib/libX11.so')
    lib.XOpenDisplay.argtypes = [ctypes.c_char_p]
    lib.XOpenDisplay.restype = ctypes.c_void_p
    lib.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
    lib.XDefaultRootWindow.restype = ctypes.c_ulong
    lib.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    lib.XInternAtom.restype = ctypes.c_ulong
    lib.XGetWindowProperty.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_long, ctypes.c_long, ctypes.c_int, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte))]
    lib.XFree.argtypes = [ctypes.c_void_p]
    lib.XCloseDisplay.argtypes = [ctypes.c_void_p]
    class SizeHints(ctypes.Structure):
        _fields_=[('flags',ctypes.c_long),('x',ctypes.c_int),('y',ctypes.c_int),
                  ('width',ctypes.c_int),('height',ctypes.c_int),('min_width',ctypes.c_int),
                  ('min_height',ctypes.c_int),('max_width',ctypes.c_int),('max_height',ctypes.c_int),
                  ('width_inc',ctypes.c_int),('height_inc',ctypes.c_int),('min_aspect_x',ctypes.c_int),
                  ('min_aspect_y',ctypes.c_int),('max_aspect_x',ctypes.c_int),('max_aspect_y',ctypes.c_int),
                  ('base_width',ctypes.c_int),('base_height',ctypes.c_int),('win_gravity',ctypes.c_int)]
    lib.XGetWMNormalHints.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(SizeHints),ctypes.POINTER(ctypes.c_long)]
    lib.XSetWMNormalHints.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(SizeHints)]
    lib.XDisplayWidth.argtypes=[ctypes.c_void_p,ctypes.c_int];lib.XDisplayWidth.restype=ctypes.c_int
    lib.XDisplayHeight.argtypes=[ctypes.c_void_p,ctypes.c_int];lib.XDisplayHeight.restype=ctypes.c_int
    lib.XGetGeometry.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(ctypes.c_ulong),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_int),ctypes.POINTER(ctypes.c_uint),ctypes.POINTER(ctypes.c_uint),ctypes.POINTER(ctypes.c_uint),ctypes.POINTER(ctypes.c_uint)]
    lib.XSync.argtypes=[ctypes.c_void_p,ctypes.c_int]
    display = lib.XOpenDisplay(env['DISPLAY'].encode())
    assert display
    def atom(name):
        return lib.XInternAtom(display, name.encode(), False)
    def values(window, property_):
        kind, count, remaining = ctypes.c_ulong(), ctypes.c_ulong(), ctypes.c_ulong()
        format_ = ctypes.c_int()
        data = ctypes.POINTER(ctypes.c_ubyte)()
        lib.XGetWindowProperty(display, window, property_, 0, 4096, False, 0,
                              ctypes.byref(kind), ctypes.byref(format_), ctypes.byref(count),
                              ctypes.byref(remaining), ctypes.byref(data))
        if not data:
            return []
        try:
            return list(ctypes.cast(data, ctypes.POINTER(ctypes.c_ulong))[:count.value]) if format_.value == 32 else []
        finally:
            lib.XFree(data)
    def owned(pid):
        proc = Path('/proc') / str(pid)
        try:
            if os.readlink(proc / 'exe') != env['PREFIX'] + '/lib/chromium/chrome':
                return False
            entries = dict(item.split(b'=', 1) for item in (proc / 'environ').read_bytes().split(b'\0') if b'=' in item)
            if entries.get(b'HOME') != env['HOME'].encode() or entries.get(b'DISPLAY') != env['DISPLAY'].encode():
                return False
            for _ in range(16):
                if pid == owner['pid']:
                    return process.joinpath('stat').read_text().rsplit(')', 1)[1].split()[19] == str(owner['start'])
                pid = int((Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()[1])
        except (OSError, ValueError):
            pass
        return False
    fitted = []
    try:
        normal = atom('_NET_WM_WINDOW_TYPE_NORMAL')
        for window in values(lib.XDefaultRootWindow(display), atom('_NET_CLIENT_LIST')):
            pids = values(window, atom('_NET_WM_PID'))
            if not pids or not owned(pids[0]) or normal not in values(window, atom('_NET_WM_WINDOW_TYPE')):
                continue
            # Chromium's desktop minimum can exceed a narrow Android framebuffer.
            # Preserve all other hints; clamp only this qualified window's minima.
            hints,supplied=SizeHints(),ctypes.c_long()
            if lib.XGetWMNormalHints(display,window,ctypes.byref(hints),ctypes.byref(supplied)) and hints.flags&(1<<4):
                hints.min_width=min(hints.min_width,lib.XDisplayWidth(display,0))
                hints.min_height=min(hints.min_height,lib.XDisplayHeight(display,0))
                lib.XSetWMNormalHints(display,window,ctypes.byref(hints));lib.XSync(display,False)
            for args in (['-i', '-r', hex(window), '-b', 'remove,fullscreen'],['-i', '-r', hex(window), '-b', 'add,fullscreen'],
                         ['-i', '-r', hex(window), '-t', '0'], ['-s', '0'], ['-i', '-a', hex(window)]):
                result = subprocess.run([env['PREFIX'] + '/bin/wmctrl', *args], env=env,
                                        capture_output=True, timeout=5)
                if result.returncode:
                    raise RuntimeError('Owned browser window adjustment failed')
            fitted.append(window)
        if not fitted:
            raise RuntimeError('No qualified normal browser window')
        if report is not None:
            time.sleep(.15);lib.XSync(display,False)
            geometry=[]
            for window in fitted:
                root_window=ctypes.c_ulong();x,y=ctypes.c_int(),ctypes.c_int()
                width,height,border,depth=(ctypes.c_uint() for _ in range(4))
                assert lib.XGetGeometry(display,window,ctypes.byref(root_window),ctypes.byref(x),ctypes.byref(y),ctypes.byref(width),ctypes.byref(height),ctypes.byref(border),ctypes.byref(depth))
                geometry.append({'width':width.value,'height':height.value})
            report.update(windowHandles=[hex(window) for window in fitted],windowGeometry=geometry,desktopWidth=lib.XDisplayWidth(display,0),desktopHeight=lib.XDisplayHeight(display,0))
        return len(fitted)
    finally:
        lib.XCloseDisplay(display)
        for key, value in previous.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


if __name__ == '__main__':
    report={}
    report['ownedBrowserWindowsFitted']=fit_browser(sys.argv[1],report)
    print(json.dumps(report))


def desktop_size(root):
    """Read owned display geometry without focusing or modifying any window."""
    env=json.loads((Path(root)/'environment.json').read_text())
    previous={key:os.environ.get(key) for key in ('DISPLAY','XAUTHORITY')}
    os.environ.update({key:env[key] for key in previous})
    lib=ctypes.CDLL(env['PREFIX']+'/lib/libX11.so')
    lib.XOpenDisplay.argtypes=[ctypes.c_char_p];lib.XOpenDisplay.restype=ctypes.c_void_p
    lib.XDisplayWidth.argtypes=[ctypes.c_void_p,ctypes.c_int];lib.XDisplayWidth.restype=ctypes.c_int
    lib.XDisplayHeight.argtypes=[ctypes.c_void_p,ctypes.c_int];lib.XDisplayHeight.restype=ctypes.c_int
    lib.XCloseDisplay.argtypes=[ctypes.c_void_p]
    display=None
    try:
        display=lib.XOpenDisplay(env['DISPLAY'].encode())
        if not display:raise RuntimeError('Owned display unavailable')
        return lib.XDisplayWidth(display,0),lib.XDisplayHeight(display,0)
    finally:
        if display:lib.XCloseDisplay(display)
        for key,value in previous.items():
            if value is None:os.environ.pop(key,None)
            else:os.environ[key]=value
