#!/usr/bin/env python3
"""Qualify a builder generation through the real Termux Core and native TUI.

Uses only disposable homes, a non-authenticated loopback model provider and
slash commands. No model turn, installed state or Android notification is used.
The shared-server cases additionally inspect native settings notifications.
"""

import argparse
import ast
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import sys
import termios
import time
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / ".github/scripts"))
from rald3_preflight import parse_descriptor


ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\].*?(?:\x07|\x1b\\)", re.S)
ASK = "No Sandbox (Ask for approval)"
AUTO = "No Sandbox (Approve for me)"
EXPECTED = [("on-request", "auto_review"), ("never", "user"),
            ("on-request", "auto_review"), ("on-request", "user"),
            ("on-request", "auto_review")]


def notifications(trace, method):
    result = []
    for line in trace.read_text(errors="replace").splitlines():
        if method not in line:
            continue
        for quoted in re.findall(r'"(?:\\.|[^"\\])*"', line):
            try:
                data = ast.literal_eval(quoted)
            except (ValueError, SyntaxError):
                continue
            for brace in re.finditer(r"\{", data):
                try:
                    message = json.JSONDecoder().raw_decode(data[brace.start():])[0]
                except ValueError:
                    continue
                if isinstance(message, dict) and message.get("method") == method:
                    result.append(message["params"])
                    break
    return result


def qualify(core, generation, parent, mode):
    with tempfile.TemporaryDirectory(prefix="pq", dir=parent) as temporary:
        root = Path(temporary)
        home, prefix = root / "h", root / "p"
        fields = parse_descriptor(generation / "generation.meta")
        identity = fields["generation_id"]
        assert ";permission_policy=" in fields["patch_report"]
        runtime = home / ".local/lib/codex/core/generations" / identity
        shutil.copytree(generation, runtime)
        for directory in [prefix / "bin", prefix / "etc/tls", home / ".codex",
                          home / ".local/share/codex/core/config"]:
            directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        shutil.copy2(core, prefix / "bin/codex")
        (prefix / "etc/resolv.conf").write_text("nameserver 127.0.0.1\n")
        (prefix / "etc/tls/cert.pem").write_text("fixture\n")
        (home / ".local/share/codex/core/activation-state").write_text(
            f"format=codex-activation-state-v3\nupdate_key={'11' * 32}\n"
            f"current={identity}\ncurrent_key={'11' * 32}\n"
            "previous_present=0\nprevious=\nprevious_key=\n")
        named = 'default_permissions = ":danger-full-access"\n' if mode == "named" else ""
        config = named + f'''model = "fixture-model"
model_provider = "fixture"
check_for_update_on_startup = false
[projects."{root}"]
trust_level = "trusted"
[features]
guardian_approval = true
[tui.keymap.chat]
next_permission_mode = "f7"
previous_permission_mode = "f6"
[model_providers.fixture]
name = "Fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
'''
        assert tomllib.loads(config)["model_provider"] == "fixture"
        (home / ".codex/config.toml").write_text(config)
        env = {"HOME": str(home), "PREFIX": str(prefix), "TMPDIR": str(parent),
               "CODEX_HOME": str(home / ".codex"), "TERM": "xterm-256color",
               "PATH": str(Path(shutil.which("sh")).parent),
               "SSL_CERT_FILE": str(Path(shutil.which("sh")).parent.parent / "etc/tls/cert.pem")}
        trace = root / "native.trace"
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 180, 0, 0))
        command = [shutil.which("strace"), "-f", "-e", "trace=write,writev,sendto,sendmsg",
                   "-s", "262144", "-o", str(trace), str(prefix / "bin/codex"),
                   "--no-alt-screen"]
        if mode == "embedded":
            command.append("--no-daemon")
        process = subprocess.Popen(command, env=env, cwd=root, stdin=slave,
                                   stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        server = None

        def drain(seconds):
            output = bytearray()
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], .05)[0]:
                    try:
                        data = os.read(master, 65536)
                    except OSError:
                        break
                    if b"\x1b[6n" in data:
                        os.write(master, b"\x1b[1;1R")
                    if b"\x1b[c" in data:
                        os.write(master, b"\x1b[?1;2c")
                    output.extend(data)
            assert process.poll() is None, "native TUI exited"
            assert not (home / ".codex/auth.json").exists(), "unexpected auth onboarding"
            return ANSI.sub("", output.decode(errors="replace"))

        def key(value):
            if len(value) > 1 and value.endswith(b"\r"):
                os.write(master, value[:-1])
                drain(.3)
                os.write(master, b"\r")
            else:
                os.write(master, value)
            return drain(2)

        def status(expected, initial=False):
            output = key(b"/status\r")
            deadline = time.monotonic() + 10
            while "Permissions:" not in output:
                assert process.poll() is None, "owned native client exited during status"
                assert time.monotonic() < deadline, "status did not finish: " + output[-2500:]
                output += drain(.5)
            pattern = re.escape(expected)
            if initial and mode != "named":
                pattern += r"|Custom \(danger-full-access, Ask for approval\)"
            assert re.search(r"Permissions:\s+(?:" + pattern + ")", output), expected + ": " + output[-1200:]
            assert re.search(r"Model provider:\s+fixture", output), "wrong model provider"
            assert re.search(r"Token usage:\s+0 total", output), "unexpected model turn"
            match = re.search(r"Session:\s+([0-9a-f-]{36})", output)
            assert match, "missing native session"
            return match[1]

        def menu(current=None):
            output = key(b"/permissions\r")
            deadline = time.monotonic() + 15
            while "3. Full Access" not in output:
                assert process.poll() is None, "owned native client exited during permission discovery"
                assert time.monotonic() < deadline, "permission discovery did not finish: " + output[-3000:]
                output += drain(.5)
            assert "Read Only" not in output, "unsupported choice exposed"
            assert "without a sandbox" in output, "misleading sandbox description"
            assert "sandbox.afe" not in output, "stale inline description tail"
            if current:
                assert current + " (current)" in output, "wrong current selection"

        try:
            boot = drain(1)
            deadline = time.monotonic() + 60
            while "fixture-model" not in boot:
                assert process.poll() is None, "owned native client exited during initialization"
                assert "Sign in with ChatGPT" not in boot, "unexpected auth onboarding"
                assert time.monotonic() < deadline, "fixture TUI did not initialize"
                boot += drain(.5)
            assert "Sign in with ChatGPT" not in boot, "unexpected auth onboarding"
            if mode != "embedded":
                servers = home / ".local/share/codex/core/servers"
                records = list(servers.glob("*/pid"))
                assert len(records) == 1, "missing or duplicate Core server"
                pid = int(records[0].read_text())
                executable = Path(f"/proc/{pid}/exe").resolve()
                assert executable == runtime / "runtime", "wrong server executable"
                server = (pid, Path(f"/proc/{pid}/stat").read_text().split()[21], records[0])
                deadline = time.monotonic() + 60
                while not notifications(trace, "thread/started"):
                    assert time.monotonic() < deadline, "native thread did not start"
                    drain(.5)
                started = notifications(trace, "thread/started")
                assert all(row["thread"]["modelProvider"] == "fixture" for row in started)
            # Server thread/start precedes the client's initial screen transition.
            # Let that transition and paste detection settle before sending keys.
            drain(3)
            session = status(ASK, initial=True)
            for label, number, expected in [("Ask for approval", b"1", ASK),
                                             ("Approve for me", b"2", AUTO),
                                             ("Full Access", b"3", "Full Access"),
                                             ("Approve for me", b"2", AUTO)]:
                menu()
                selected = key(number)
                if label == "Full Access":
                    assert "Enable full access?" in selected, "confirmation absent"
                    key(b"\r")
                assert status(expected) == session, "thread restarted"
            menu("Approve for me")
            key(b"\x1b")
            key(b"\x1b[18~")  # Explicit private keymap: F7, next.
            assert status(ASK) == session
            key(b"\x1b[17~")  # F6, previous.
            assert status(AUTO) == session
            assert not notifications(trace, "turn/started"), "unexpected model turn"
            if server:
                pid, start, record = server
                assert int(record.read_text()) == pid
                assert Path(f"/proc/{pid}/stat").read_text().split()[21] == start
                events = notifications(trace, "thread/settings/updated")
                assert len(events) in (5, 6), "missing or duplicate settings proof"
                settings = [event["threadSettings"] for event in events]
                assert all(event["threadId"] == session for event in events)
                assert all(row["sandboxPolicy"] == {"type": "dangerFullAccess"}
                           and row["activePermissionProfile"]["id"] == ":danger-full-access"
                           for row in settings), "native sandbox was reintroduced"
                actual = [(row["approvalPolicy"], row["approvalsReviewer"]) for row in settings]
                assert actual[-5:] == EXPECTED, "native reviewer or approval routing changed"
            print(f"PASS {mode}: four menu selections, current marker, two shortcuts, "
                  f"same thread, no model turn; native settings events={len(events) if server else 'embedded acknowledgements'}",
                  flush=True)
        finally:
            # Discover every own server even if initialization failed before binding.
            for record in (home / ".local/share/codex/core/servers").glob("*/pid"):
                pid = int(record.read_text())
                try:
                    if Path(f"/proc/{pid}/exe").resolve() != runtime / "runtime":
                        continue
                    os.kill(pid, signal.SIGTERM)
                    deadline = time.monotonic() + 5
                    while Path(f"/proc/{pid}/exe").exists() and time.monotonic() < deadline:
                        time.sleep(.05)
                    if Path(f"/proc/{pid}/exe").resolve() == runtime / "runtime":
                        os.kill(pid, signal.SIGKILL)
                except (FileNotFoundError, ProcessLookupError):
                    pass
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            os.close(master)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--generation", type=Path, required=True)
    parser.add_argument("--temp-parent", type=Path, required=True)
    args = parser.parse_args()
    assert shutil.which("strace"), "strace is required for native settings proof"
    for path in [args.core, args.generation, args.temp_parent]:
        assert path.is_absolute(), "qualification paths must be absolute"
    runtime = args.generation / "runtime"
    print("Runtime SHA256", hashlib.file_digest(runtime.open("rb"), "sha256").hexdigest(), flush=True)
    for mode in ["legacy", "named", "embedded"]:
        qualify(args.core, args.generation, args.temp_parent, mode)


if __name__ == "__main__":
    main()
