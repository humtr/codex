#!/usr/bin/env python3
"""Run the AppHost shutdown acceptance cases with a locally built binary on Android."""
import argparse
import base64
import ctypes
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import socket
import stat
import subprocess
import tempfile
import time
import urllib.request

PREFIX = Path(os.environ.get("PREFIX", "/data/data/com.termux/files/usr"))
LIBC = ctypes.CDLL(None, use_errno=True)
SELF = os.getpid()


def eventually(check, seconds=15):
    deadline = time.monotonic() + seconds
    while True:
        value = check()
        if value:
            return value
        if time.monotonic() >= deadline:
            raise AssertionError(f"condition did not become true within {seconds}s")
        time.sleep(0.025)


def processes():
    result = {}
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            state = (path / "stat").read_text().rsplit(")", 1)[1].split()
            result[int(path.name)] = (int(state[1]), int(state[19]), state[0])
        except (OSError, ValueError):
            continue
    return result


def descendants(root=SELF):
    all_processes = processes()
    owned = {root}
    while new := {pid for pid, (parent, _, _) in all_processes.items() if parent in owned} - owned:
        owned.update(new)
    owned.remove(root)
    return {pid: all_processes[pid] for pid in owned}


def identity(pid):
    path = Path("/proc") / str(pid)
    try:
        cmd = (path / "cmdline").read_bytes().split(b"\0")
        if b"host-supervisor" in cmd:
            return "supervisor"
        if b"--shm-helper" in cmd:
            return "shm-helper"
        return (path / "comm").read_text().strip()
    except OSError:
        return ""


def signal_owned(pid, start, number):
    # Android ARM64 implements these syscalls even when Python omits the wrappers.
    fd = LIBC.syscall(434, pid, 0)
    if fd < 0:
        if ctypes.get_errno() == 3:
            return
        raise OSError(ctypes.get_errno(), "pidfd_open")
    try:
        current = descendants().get(pid)
        if current is not None and current[1] == start and current[2] != "Z":
            if LIBC.syscall(424, fd, number, ctypes.c_void_p(), 0) < 0 and ctypes.get_errno() != 3:
                raise OSError(ctypes.get_errno(), "pidfd_send_signal")
    finally:
        os.close(fd)


def proc_fds(pid):
    paths = Path("/proc") / str(pid) / "fd"
    values = set()
    for path in paths.iterdir():
        try:
            values.add(os.readlink(path))
        except FileNotFoundError:
            pass
    return values


def cleanup_test(children):
    """Catch failures without signaling any process outside this test harness."""
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        for child in children:
            child.poll()
        table = descendants()
        if not table:
            return
        for pid, (parent, started, state) in table.items():
            if state != "Z":
                signal_owned(pid, started, signal.SIGKILL)
            elif parent == SELF and all(pid != child.pid for child in children):
                try:
                    os.waitpid(pid, os.WNOHANG)
                except ChildProcessError:
                    pass
        time.sleep(0.03)
    raise AssertionError(f"test cleanup retained {descendants()}")


def start(binary, rootfs, work, profile, children, viewer, *, prefix=PREFIX):
    offset = len(children)
    config = work / f"provider-{offset}.toml"
    settings = ["[host]", f"profile_dir = {json.dumps(str(profile))}"]
    if viewer:
        settings.append(f"viewer_assets = {json.dumps(str(viewer))}")
    config.write_text("\n".join(settings) + "\n")
    state = work / f"state-{offset}"
    env = {**os.environ, "HOME": str(work), "TMPDIR": str(work), "PREFIX": str(prefix)}
    log = work / f"provider-{offset}.log"
    with log.open("w") as output:
        child = subprocess.Popen(
            [binary, "serve", "--config", config, "--bind", "127.0.0.1:0",
             "--state", state, "--rootfs", rootfs, "--eager-host"],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=output,
            stderr=output,
        )
    children.append(child)
    return child, log, state


def request(port, path, *, token=None, method="GET"):
    header = {"Authorization": f"Bearer {token}"} if token else {}
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", headers=header, method=method)
    with urllib.request.urlopen(req, timeout=12) as response:
        return json.load(response)


def wait_ready(provider, log):
    def ready():
        content = log.read_text(errors="replace")
        assert provider.poll() is None, content[-1200:]
        m = re.search(r"listening on http://127\.0\.0\.1:(\d+)", content)
        if m:
            port = int(m[1])
            assert request(port, "/healthz")["host_running"] is True
            return port
        return None
    return eventually(ready, 20)


def wait_host(provider):
    def host():
        entries = descendants(provider.pid)
        names = {identity(pid): pid for pid in entries}
        if {"supervisor", "proot", "shm-helper", "Xvnc"} <= names.keys():
            return names
        return None
    return eventually(host)


def locked(profile):
    with profile.with_suffix(".lock").open("rb") as lease:
        try:
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return True
    return False


def gone(entries):
    now = processes()
    return all(pid not in now or now[pid][1] != value[1] for pid, value in entries.items())


def stopped(entries, supervisor, profile):
    # When the Provider is gone, this harness substitutes for Android init
    # for the supervisor itself. A guest adopted here is a product failure.
    assert all(pid == supervisor for pid, (parent, _, _) in descendants().items()
               if parent == SELF and pid in entries), descendants()
    adopted = descendants().get(supervisor)
    if adopted is not None and adopted[0] == SELF and adopted[2] == "Z":
        os.waitpid(supervisor, os.WNOHANG)
    if not locked(profile):
        assert gone({pid: value for pid, value in entries.items() if pid != supervisor}), (
            "Superseded descendants still present after the profile unlocked"
        )
    return gone(entries)


def display_locks(xvnc):
    result = []
    for path in (PREFIX / "tmp").glob(".X*-lock"):
        try:
            if path.read_text().strip() == str(xvnc):
                socket_path = path.parent / ".X11-unix" / path.name[1:].removesuffix("-lock")
                result.append((path, socket_path))
        except OSError:
            pass
    return result


def open_viewer(port, state):
    token = (state / "control.token").read_text().strip()
    url = request(port, "/control/viewer", token=token)["url"]
    capability = url.split("/viewer/", 1)[1].split("/", 1)[0]
    sock = socket.create_connection(("127.0.0.1", port), timeout=5)
    key = base64.b64encode(os.urandom(16)).decode()
    sock.sendall(
        (f"GET /viewer/{capability}/websock HTTP/1.1\r\nHost: 127.0.0.1\r\n"
         "Connection: Upgrade\r\nUpgrade: websocket\r\n"
         f"Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n").encode()
    )
    reply = sock.recv(4096)
    if b" 101 " not in reply.split(b"\r\n", 1)[0]:
        sock.close()
        raise AssertionError("viewer did not upgrade")
    return sock


def open_sse(port):
    body = b'{"input":"Local shutdown test","stream":true}'
    req = urllib.request.Request(f"http://127.0.0.1:{port}/v1/responses", data=body,
                                 headers={"Content-Type": "application/json"}, method="POST")
    response = urllib.request.urlopen(req, timeout=5)
    try:
        assert response.headers.get_content_type() == "text/event-stream"
        for _ in range(5):
            if b"response.in_progress" in response.readline():
                return response
        raise AssertionError("no live SSE frame")
    except BaseException:
        response.close()
        raise


def run(args, case):
    children = []
    with tempfile.TemporaryDirectory(prefix="ash-", dir=PREFIX / "tmp") as path:
        work = Path(path)
        profile = work / "profile"
        viewer_socket = None
        sse = None
        locks = []
        try:
            if case == "boot-cancel":
                # A stopped launcher supplies a repeatable pre-PRoot-exec boundary.
                prefix = work / "prefix"
                (prefix / "bin").mkdir(parents=True)
                (prefix / "tmp").symlink_to(PREFIX / "tmp")
                for tool in ("Xvnc", "openbox", "wmctrl"):
                    (prefix / "bin" / tool).symlink_to(PREFIX / "bin" / tool)
                shim = prefix / "bin/proot"
                shim.write_text(f"#!{PREFIX}/bin/sh\nkill -STOP $$\nexec '{PREFIX}/bin/proot' \"$@\"\n")
                shim.chmod(0o700)
                child, log, _ = start(args.binary, args.rootfs, work, profile, children, None, prefix=prefix)
                def stopped_launcher():
                    tree = descendants(child.pid)
                    return tree if any(item[2] == "T" for item in tree.values()) else None
                previous = eventually(stopped_launcher)
                supervisor = next(pid for pid in previous if identity(pid) == "supervisor")
                assert locked(profile)
                signal_owned(child.pid, processes()[child.pid][1], signal.SIGKILL)
                child.wait(timeout=5)
                try:
                    eventually(lambda: stopped(previous, supervisor, profile), 5)
                except AssertionError as error:
                    raise AssertionError((descendants(), {
                        pid: (identity(pid), Path(f"/proc/{pid}/wchan").read_text().strip())
                        for pid in descendants()
                    }, log.read_text(errors="replace")[-1200:])) from error
                assert not locked(profile) and not list(work.glob("cgp-*"))
                assert not descendants(), descendants()
                print(case, "PASS", flush=True)
                return

            if case in ("boot-dbus", "boot-dbus-kill"):
                child, log, _ = start(args.binary, args.rootfs, work, profile, children, None)
                def booting():
                    assert "listening on http://" not in log.read_text(errors="replace"), "CDP already ready"
                    tree = descendants(child.pid)
                    return tree if any(identity(pid) == "dbus-daemon" for pid in tree) else None
                previous = eventually(booting)
                supervisor = next(pid for pid in previous if identity(pid) == "supervisor")
                fault = signal.SIGTERM if case == "boot-dbus" else signal.SIGKILL
                signal_owned(child.pid, processes()[child.pid][1], fault)
                assert child.wait(timeout=5) == (0 if fault == signal.SIGTERM else -signal.SIGKILL)
                eventually(lambda: stopped(previous, supervisor, profile), 5)
                assert not locked(profile) and not list(work.glob("cgp-*"))
                assert not descendants(), descendants()
                print(case, f"PASS ({fault.name} before CDP readiness)", flush=True)
                return

            child, log, state = start(args.binary, args.rootfs, work, profile, children, args.viewer)
            port = wait_ready(child, log)
            names = wait_host(child)
            previous = descendants(child.pid)
            locks = display_locks(names["Xvnc"])
            assert locks and locked(profile)
            policy_fds = proc_fds(names["supervisor"])
            control = {fd for fd in policy_fds if fd.startswith("socket:[")}
            forbidden = {str(profile.with_suffix(".lock")), *control}
            assert str(profile.with_suffix(".lock")) in policy_fds and control
            for key in ("proot", "shm-helper", "Xvnc"):
                assert not (proc_fds(names[key]) & forbidden), key

            started = time.monotonic()
            if case in ("term", "term-sse"):
                if case == "term-sse":
                    # Suspend the tracer before opening the SSE. The real driver
                    # cannot complete its initial guest CDP operations.
                    proot = names["proot"]
                    signal_owned(proot, previous[proot][1], signal.SIGSTOP)
                    eventually(lambda: processes().get(proot, (None, None, None))[2] == "T", 2)
                    sse = open_sse(port)
                    started = time.monotonic()
                if args.viewer and case == "term":
                    viewer_socket = open_viewer(port, state)
                    started = time.monotonic()
                signal_owned(child.pid, processes()[child.pid][1], signal.SIGTERM)
                assert child.wait(timeout=5) == 0
            elif case == "restart":
                token = (state / "control.token").read_text().strip()
                assert request(port, "/control/host/restart", token=token, method="POST")["restarted"]
                assert gone(previous) and locked(profile)
                assert request(port, "/healthz")["host_running"] is True
                previous = descendants(child.pid)
                signal_owned(child.pid, processes()[child.pid][1], signal.SIGTERM)
                assert child.wait(timeout=5) == 0
            elif case == "supervisor-kill":
                target = names["supervisor"]
                singleton = profile / "chromium/SingletonLock"
                lock_target = os.readlink(singleton)
                signal_owned(target, previous[target][1], signal.SIGKILL)
                eventually(lambda: not request(port, "/healthz")["host_running"], 2)
                candidate, err, _ = start(args.binary, args.rootfs, work, profile, children, None)
                assert candidate.wait(timeout=5) != 0
                assert "host_profile_busy_stale_electron" in err.read_text(errors="replace")
                assert os.readlink(singleton) == lock_target
                print(case, "PASS (live singleton preserved)", flush=True)
                return
            elif case == "proot-kill":
                target = names["proot"]
                signal_owned(target, previous[target][1], signal.SIGKILL)
            elif case == "provider-kill":
                helper = names["shm-helper"]
                signal_owned(helper, previous[helper][1], signal.SIGSTOP)
                eventually(lambda: processes().get(helper, (None, None, None))[2] == "T", 2)
                signal_owned(child.pid, processes()[child.pid][1], signal.SIGKILL)
                child.wait(timeout=5)
            else:
                raise AssertionError(case)

            if case in ("provider-kill", "proot-kill"):
                candidate, output, _ = start(args.binary, args.rootfs, work, profile, children, None)
                if case == "proot-kill":
                    eventually(lambda: not request(port, "/healthz")["host_running"], 2)
                eventually(lambda: stopped(previous, names["supervisor"], profile), 2 if case == "proot-kill" else 5)
                wait_ready(candidate, output)
                assert locked(profile)
                still = descendants(candidate.pid)
                signal_owned(candidate.pid, processes()[candidate.pid][1], signal.SIGTERM)
                assert candidate.wait(timeout=5) == 0
                eventually(lambda: gone(still), 5)
                if child.poll() is None:
                    signal_owned(child.pid, processes()[child.pid][1], signal.SIGTERM)
                    assert child.wait(timeout=5) == 0
            eventually(lambda: gone(previous), 5)
            remains = list(work.glob("cgp-*"))
            assert not locked(profile) and not remains, (
                locked(profile),
                [(str(p.relative_to(work)), oct(p.lstat().st_mode))
                 for root in remains for p in (root, *list(root.rglob("*"))[:50])],
                log.read_text(errors="replace")[-1200:],
            )
            assert all(not lock.exists() and not sock.exists() for lock, sock in locks)
            assert not descendants(), descendants()
            print(case, f"PASS ({time.monotonic() - started:.3f}s including requested restart)", flush=True)
        finally:
            if viewer_socket:
                viewer_socket.close()
            if sse:
                sse.close()
            cleanup_test(children)
            if case == "supervisor-kill":
                # Direct supervisor death is outside the cleanup contract. Remove
                # only artifacts of the test's confirmed, now-dead Xvnc.
                for lock, sock in locks:
                    if lock.is_file() and lock.read_text().strip() == str(names["Xvnc"]):
                        if sock.is_socket() and sock.lstat().st_uid == os.getuid():
                            sock.unlink()
                        if lock.lstat().st_uid == os.getuid():
                            lock.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("rootfs", type=Path)
    parser.add_argument("--viewer", type=Path)
    parser.add_argument("--case", action="append")
    args = parser.parse_args()
    args.binary = args.binary.resolve()
    args.rootfs = args.rootfs.resolve()
    if args.viewer:
        args.viewer = args.viewer.resolve()
    assert LIBC.prctl(36, 1, 0, 0, 0) == 0
    service_roots = {pid for pid in processes() if identity(pid) in ("runsv", "runsvdir")}
    baseline = {pid: entry[1] for pid, entry in processes().items() if entry[0] in service_roots or pid in service_roots}
    for case in args.case or ("term", "term-sse", "boot-cancel", "boot-dbus", "boot-dbus-kill", "proot-kill", "provider-kill", "restart", "supervisor-kill"):
        run(args, case)
    after = processes()
    assert all(pid in after and after[pid][1] == start for pid, start in baseline.items())
    print(f"preserved runit identities: {len(baseline)}")


if __name__ == "__main__":
    main()
