#!/usr/bin/env python3
"""Owned actual Core/Manager/native task qualification; no credentials or remote model."""
import argparse
import base64
import fcntl
import http.server
import json
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import socket
import struct
import sys
import subprocess
import tempfile
import termios
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / ".github/scripts"))
from rald5_publication import parse_manifest, raw_public_key


class Ws:
    def __init__(self, path):
        self.socket = socket.socket(socket.AF_UNIX)
        self.socket.settimeout(12)
        self.socket.connect(os.path.realpath(path))
        self.pending = b""
        self.next = 0
        self.closed = set()
        key = base64.b64encode(os.urandom(16)).decode()
        self.socket.sendall((f"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n").encode())
        while b"\r\n\r\n" not in self.pending:
            self.pending += self.socket.recv(65536)
        header, self.pending = self.pending.split(b"\r\n\r\n", 1)
        assert b"101 Switching Protocols" in header
        self.call("initialize", {"clientInfo": {"name": "task_qualification", "version": "1"}, "capabilities": {"experimentalApi": True}})
        self.send({"method": "initialized"})

    def take(self, count):
        while len(self.pending) < count:
            data = self.socket.recv(65536)
            assert data, "native socket closed"
            self.pending += data
        data, self.pending = self.pending[:count], self.pending[count:]
        return data

    def send(self, value):
        data = json.dumps(value).encode()
        size = len(data)
        header = bytes([0x81, 0x80 | size]) if size < 126 else bytes([0x81, 0xfe]) + struct.pack("!H", size)
        mask = os.urandom(4)
        self.socket.sendall(header + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

    def receive(self):
        opcode, length = self.take(2)
        length &= 127
        if length == 126:
            length = struct.unpack("!H", self.take(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self.take(8))[0]
        data = self.take(length)
        assert opcode & 15 == 1
        message = json.loads(data)
        if message.get("method") == "thread/closed":
            self.closed.add(message["params"]["threadId"])
        return message

    def call(self, method, params):
        if method == "thread/resume":
            self.closed.discard(params["threadId"])
        self.next += 1
        self.send({"id": self.next, "method": method, "params": params})
        while True:
            message = self.receive()
            if message.get("id") == self.next:
                assert "error" not in message, (method, message.get("error"))
                return message["result"]

    def wait_closed(self, thread, timeout=10):
        deadline = time.monotonic() + timeout
        previous = self.socket.gettimeout()
        try:
            while thread not in self.closed:
                remaining = deadline - time.monotonic()
                assert remaining > 0, "owned thread did not close"
                self.socket.settimeout(remaining)
                self.receive()
        finally:
            self.socket.settimeout(previous)

    def close(self):
        self.socket.close()


def fixture_activation_state(identity, public_key):
    with tempfile.TemporaryDirectory(prefix="tqkey-") as temporary:
        key = raw_public_key("openssl", public_key, Path(temporary))
    return f"format=codex-activation-state-v3\nupdate_key={key}\ncurrent={identity}\ncurrent_key={key}\nprevious_present=0\nprevious=\nprevious_key=\n"


def materialize_signed_generation(generation, native):
    # Download-size/index controls are published beside the generation, not
    # installed payload. Reuse the strict signed inventory parser.
    _, inventory = parse_manifest(generation / "release.manifest")
    for relative in [rel for rel, _, _ in inventory] + ["release.manifest", "release.sig"]:
        target = native / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(generation / relative, target)


def qualify(generation, parent, public_key):
    core = generation / "core"
    with tempfile.TemporaryDirectory(prefix="tq", dir=parent) as temporary:
        root = Path(temporary)
        home, prefix = root / "h", root / "p"
        identity = dict(line.split("\t", 1) for line in (generation / "generation.meta").read_text().splitlines()[1:])["generation_id"]
        native = home / ".local/lib/codex/core/generations" / identity
        materialize_signed_generation(generation, native)
        for path in [home / ".codex", home / ".local/share/codex/core/config", prefix / "bin", prefix / "etc/tls"]:
            path.mkdir(parents=True, exist_ok=True, mode=0o700)
        shutil.copy2(core, prefix / "bin/codex")
        (prefix / "etc/resolv.conf").write_text("nameserver 127.0.0.1\n")
        (prefix / "etc/tls/cert.pem").write_text("fixture\n")
        (home / ".local/share/codex/core/activation-state").write_text(fixture_activation_state(identity, public_key))
        real = Path(shutil.which("python3")).parent
        env = {"HOME": str(home), "PREFIX": str(prefix), "TMPDIR": str(parent), "PATH": f"{prefix / 'bin'}:{real}", "SSL_CERT_FILE": str(real.parent / "etc/tls/cert.pem")}
        release = threading.Event()
        model_started = threading.Event()

        class Model(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                assert self.path == "/v1/responses"
                self.rfile.read(int(self.headers["Content-Length"]))
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.end_headers()
                self.wfile.write(b'event: response.created\ndata: {"type":"response.created","response":{"id":"owned-active"}}\n\n')
                self.wfile.flush()
                model_started.set()
                release.wait(45)

        model = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Model)
        model.daemon_threads = True
        threading.Thread(target=model.serve_forever, daemon=True).start()
        config = f'''model = "fixture-model"
model_provider = "fixture"
check_for_update_on_startup = false
thread_unload_delay_secs = 0
[features]
memories = false
goals = true
[projects."{root}"]
trust_level = "trusted"
[model_providers.fixture]
name = "Fixture"
base_url = "http://127.0.0.1:{model.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
'''
        (home / ".codex/config.toml").write_text(config)
        clients, servers = [], set()

        def run(*args, account=None, timeout=25):
            selected = dict(env)
            if account:
                selected["CODEX_HOME"] = str(account)
            return subprocess.run([str(prefix / "bin/codex"), *args], env=selected, cwd=root, capture_output=True, timeout=timeout)

        def tui(args, account=None):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 100, 0, 0))
            selected = dict(env, TERM="xterm-256color")
            if account:
                selected["CODEX_HOME"] = str(account)
            process = subprocess.Popen([str(prefix / "bin/codex"), *args], env=selected, cwd=root, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            os.set_blocking(master, False)
            clients.append((process, master))
            return process, master

        def drain(master):
            try:
                while select.select([master], [], [], 0)[0]:
                    if not os.read(master, 65536):
                        break
            except OSError:
                pass

        def bind(account):
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                for path in (home / ".local/share/codex/core/servers").glob("*"):
                    if not (path / "owner").exists() or not (path / "pid").exists():
                        continue
                    if (path / "owner").read_bytes().split(b"\0")[0] == os.fsencode(account) and (path / "s").exists():
                        pid = int((path / "pid").read_text())
                        servers.add(pid)
                        return path
                for _, master in clients:
                    drain(master)
                time.sleep(.05)
            raise AssertionError("owned native server did not bind")

        def wait_owner(thread, desired):
            deadline = time.monotonic() + 15
            last = ""
            while time.monotonic() < deadline:
                output = run("termux", "task", "status", thread)
                last = output.stderr.decode() if output.returncode else output.stdout.decode()
                if output.returncode == 0 and desired in output.stdout.decode():
                    return output.stdout.decode()
                for _, master in clients:
                    drain(master)
                time.sleep(.05)
            raise AssertionError(f"expected native task owner/state not reached: {last}")

        def wait_transfer(thread, process, master):
            deadline = time.monotonic() + 15
            diagnostic = b""
            last = b""
            while time.monotonic() < deadline:
                try:
                    while select.select([master], [], [], 0)[0]:
                        diagnostic = (diagnostic + os.read(master, 65536))[-65536:]
                except OSError:
                    pass
                assert process.poll() is None, f"owned transfer exited {process.returncode}: {diagnostic[-1500:]!r}"
                output = run("termux", "task", "status", thread)
                last = output.stdout + output.stderr
                if output.returncode == 0 and b"owner=account-b" in output.stdout:
                    assert b"state=idle" in output.stdout
                    return
                time.sleep(.05)
            locks = []
            for record in (home / ".local/share/codex/core/servers").glob("*"):
                pid = int((record / "pid").read_text())
                try:
                    if Path(f"/proc/{pid}/exe").resolve() != native / "runtime":
                        continue
                    for fd in Path(f"/proc/{pid}/fdinfo").iterdir():
                        locks.extend(f"{pid}: {line}" for line in fd.read_text().splitlines() if line.startswith("lock:"))
                except OSError:
                    pass
            raise AssertionError(f"owned transfer timeout; status={last!r}, kernel-locks={locks[:32]!r}, terminal={diagnostic[-1500:]!r}")

        try:
            created = run("termux", "profile", "create", "account-a")
            assert created.returncode == 0, created.stderr.decode()
            a = home / ".local/share/codex/manager/profiles/account-a/home"
            (a / "config.toml").write_text(config)
            process, master = tui(["--no-alt-screen"], a)
            record = bind(a)
            # Exited Core bindings outlive account homes within retained generations.
            # Seed only owned metadata; real Core/Manager must discover A past it.
            exited = subprocess.Popen([sys.executable, "-c", "pass"], env=env)
            assert exited.wait(timeout=5) == 0
            stale_owner = os.fsencode(root / "retired-account") + b"\0" + os.fsencode(native / "runtime")
            digest = 0xcbf29ce484222325
            for byte in stale_owner:
                digest = ((digest ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
            stale = record.parent / f"{digest:016x}"
            stale.mkdir(mode=0o700)
            for name, data in [("owner", stale_owner), ("pid", f"{exited.pid}\n".encode())]:
                (stale / name).write_bytes(data)
                (stale / name).chmod(0o600)
            discovered = run("termux", "task", "status")
            assert discovered.returncode == 0, discovered.stderr.decode()
            assert (stale / "owner").read_bytes() == stale_owner
            assert (stale / "pid").read_bytes() == f"{exited.pid}\n".encode()
            print("PASS native public task discovery ignores exited retired metadata without mutation")
            rpc = Ws(record / "s")
            started = rpc.call("thread/start", {"cwd": str(root)})
            thread = started["thread"]["id"]
            turn = rpc.call("turn/start", {"threadId": thread, "input": [{"type": "text", "text": "Owned synthetic cancellable task.", "text_elements": []}]})
            assert model_started.wait(10), "loopback model not reached"
            rpc.call("thread/goal/set", {"threadId": thread, "objective": "Owned qualification goal.", "status": "active"})
            # Abrupt TUI departure plus client disconnect while the turn is active.
            process.terminate()
            process.wait(timeout=5)
            rpc.close()
            text = wait_owner(thread, "state=active")
            assert "owner=account-a" in text
            print("PASS native running-disconnect preserves active writer and discovers account-a")
            # Cross-account same-ID native resume really fails while A owns the writer.
            other, other_master = tui(["resume", thread])
            second_record = bind(home / ".codex")
            b = Ws(second_record / "s")
            try:
                b.call("thread/resume", {"threadId": thread, "cwd": str(root)})
                raise AssertionError("native cross-account writer lock was not enforced")
            except AssertionError as error:
                assert "active writer" in str(error), str(error)
            b.close()
            other.terminate(); other.wait(timeout=5)
            # Manager reconnect must reuse A's actual server without a new writer.
            back, back_master = tui(["termux", "task", "reconnect", thread])
            time.sleep(1)
            assert back.poll() is None
            assert "owner=account-a" in wait_owner(thread, "state=active")
            back.terminate(); back.wait(timeout=5)
            # Native cancellation is done before transfer; idle delay0 releases A's writer.
            rpc = Ws(record / "s")  # Observe closure without subscribing to the thread.
            stopped = run("termux", "task", "stop", thread)
            assert stopped.returncode == 0, stopped.stderr.decode()
            assert b"Task stopped" in stopped.stdout
            print("PASS native owner reconnect, cross-account lock rejection and confirmed cancellation")
            # Start A again; keep its subscriber connected to prove stopped-but-held distinction.
            rpc.wait_closed(thread)
            rpc.call("thread/resume", {"threadId": thread, "cwd": str(root)})
            process2, master2 = tui(["resume", thread], a)
            assert rpc.call("thread/goal/get", {"threadId": thread})["goal"]["status"] == "paused"
            held = run("termux", "task", "takeover", thread)
            assert held.returncode == 1 and b"writer is still owned" in held.stderr, held.stderr.decode()
            print("PASS native retained subscriber blocks takeover without starting another writer")
            # Explicit whole-server scope kills only A, then ordinary B resume owns the same UUID.
            text = wait_owner(thread, "owner=account-a")
            token = text.split("server=", 1)[1].strip()
            transferred, transferred_master = tui(["termux", "task", "takeover", thread, "--force-server", token])
            text = wait_owner(thread, "owner=default")
            assert transferred.poll() is None
            assert "state=idle" in text
            print("PASS native PID-stable force scope and ordinary same-ID current-account takeover")
            rpc.close()
            # The previous held-subscriber client is outside server termination scope.
            # Close it before the independent normal-transfer scenario: otherwise it
            # can reconnect when A's server is recreated and legitimately reclaim a writer.
            process2.terminate(); process2.wait(timeout=5)
            # Observe B closure before reusing the same UUID in a fresh A server.
            b = Ws(second_record / "s")
            transferred.terminate(); transferred.wait(timeout=5)
            b.wait_closed(thread)
            b.close()
            process3, master3 = tui(["resume", thread], a)
            record = bind(a)
            rpc = Ws(record / "s")
            rpc.call("thread/resume", {"threadId": thread, "cwd": str(root)})
            model_started.clear()
            rpc.call("turn/start", {"threadId": thread, "input": [{"type": "text", "text": "Owned normal-transfer task.", "text_elements": []}]})
            assert model_started.wait(10)
            process3.kill(); process3.wait(timeout=5)
            rpc.close()
            wait_owner(thread, "state=active")
            # Fresh-launch selection must also choose the omitted takeover target.
            chosen = run("termux", "profile", "create", "account-b")
            assert chosen.returncode == 0, chosen.stderr.decode()
            destination = home / ".local/share/codex/manager/profiles/account-b/home"
            (destination / "config.toml").write_text(config)
            selected = run("termux", "profile", "default", "account-b")
            assert selected.returncode == 0, selected.stderr.decode()
            normal, normal_master = tui(["termux", "task", "takeover", thread])
            wait_transfer(thread, normal, normal_master)
            print("PASS native goal pause persists and normal stop transfers the writer to the saved-default account")
        finally:
            release.set()
            model.shutdown()
            for process, master in clients:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill(); process.wait()
                os.close(master)
            # A restarted daemon may replace the PID after bind() first sees its
            # old rendezvous. Include the final owned records and wait for shutdown
            # before removing its home, so writers cannot recreate cleanup paths.
            for record in (home / ".local/share/codex/core/servers").glob("*"):
                try:
                    servers.add(int((record / "pid").read_text()))
                except (OSError, ValueError):
                    pass
            for pid in servers:
                try:
                    for sig in [signal.SIGTERM, signal.SIGKILL]:
                        if Path(f"/proc/{pid}/exe").resolve() != native / "runtime":
                            break
                        os.kill(pid, sig)
                        deadline = time.monotonic() + 3
                        while Path(f"/proc/{pid}/exe").resolve() == native / "runtime" and time.monotonic() < deadline:
                            time.sleep(.025)
                    assert Path(f"/proc/{pid}/exe").resolve() != native / "runtime", "owned server did not exit before root cleanup"
                except (OSError, ProcessLookupError):
                    pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public-key", required=True, type=Path)
    parser.add_argument("--generation", required=True, type=Path)
    parser.add_argument("--workdir", required=True, type=Path)
    options = parser.parse_args()
    qualify(options.generation.resolve(), options.workdir.resolve(), options.public_key.resolve())
