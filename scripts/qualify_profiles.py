#!/usr/bin/env python3
"""Exercise profile lifecycle through owned actual Core/Manager/native runtimes."""
import argparse
import fcntl
import hashlib
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import time

from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor


def qualify(generation, parent, public_key):
    with tempfile.TemporaryDirectory(prefix="pq", dir=parent) as temporary:
        root = Path(temporary)
        home, prefix = root / "h", root / "p"
        fields = parse_descriptor(generation / "generation.meta")
        identity = fields["generation_id"]
        expected_version = f"codex-cli {fields['upstream_package_version']}\n".encode()
        native = home / ".local/lib/codex/core/generations" / identity
        materialize_signed_generation(generation, native)
        for path in [home / ".codex", home / ".local/share/codex/core/config", prefix / "bin", prefix / "etc/tls"]:
            path.mkdir(parents=True, exist_ok=True, mode=0o700)
        core = prefix / "bin/codex"
        shutil.copy2(generation / "core", core)
        (prefix / "etc/resolv.conf").write_text("nameserver 127.0.0.1\n")
        (prefix / "etc/tls/cert.pem").write_text("owned\n")
        (home / ".local/share/codex/core/activation-state").write_text(fixture_activation_state(identity, public_key))
        real = Path(shutil.which("python3")).parent
        env = {"HOME": str(home), "PREFIX": str(prefix), "TMPDIR": str(parent), "PATH": f"{prefix / 'bin'}:{real}", "TERM": "xterm-256color", "SSL_CERT_FILE": str(real.parent / "etc/tls/cert.pem")}
        config = '''model = "owned-model"
model_provider = "owned"
check_for_update_on_startup = false
[features]
memories = false
[model_providers.owned]
name = "Owned fixture"
base_url = "http://127.0.0.1:9/v1"
wire_api = "responses"
requires_openai_auth = false
'''
        (home / ".codex/config.toml").write_text(config)
        profiles = home / ".local/share/codex/manager/profiles"
        clients, servers = [], set()

        def run(*args, account=None, success=True):
            selected = dict(env)
            if account is not None:
                selected["CODEX_HOME"] = str(account)
            result = subprocess.run([str(core), *args], env=selected, cwd=root, capture_output=True, timeout=25)
            if success:
                assert result.returncode == 0, (args, result.stderr.decode())
            return result

        def profile(*args, **kwargs):
            return run("termux", "profile", *args, **kwargs)

        def launch(account=None):
            master, slave = pty.openpty()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 32, 100, 0, 0))
            selected = dict(env)
            if account is not None:
                selected["CODEX_HOME"] = str(account)
            process = subprocess.Popen([str(core)], env=selected, cwd=root, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            os.set_blocking(master, False)
            clients.append((process, master))
            return process, master

        def bind(account, process, master):
            deadline = time.monotonic() + 15
            diagnostic = b""
            while time.monotonic() < deadline:
                try:
                    while select.select([master], [], [], 0)[0]:
                        diagnostic = (diagnostic + os.read(master, 65536))[-4096:]
                except OSError:
                    pass
                assert process.poll() is None, f"owned native client exited: {diagnostic!r}"
                for path in (home / ".local/share/codex/core/servers").glob("*"):
                    if (path / "owner").exists() and (path / "pid").exists() and (path / "owner").read_bytes().split(b"\0")[0] == os.fsencode(account):
                        pid = int((path / "pid").read_text())
                        if (path / "s").exists() and Path(f"/proc/{pid}/exe").resolve() == native / "runtime":
                            servers.add(pid)
                            return pid
                time.sleep(.05)
            raise AssertionError(f"owned selected native server did not bind: {diagnostic!r}")

        def stop_owned():
            for process, master in clients:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                os.close(master)
            clients.clear()
            for path in (home / ".local/share/codex/core/servers").glob("*"):
                if (path / "pid").exists():
                    servers.add(int((path / "pid").read_text()))
            for pid in servers:
                for sig in [signal.SIGTERM, signal.SIGKILL]:
                    if Path(f"/proc/{pid}/exe").resolve() != native / "runtime":
                        break
                    os.kill(pid, sig)
                    deadline = time.monotonic() + 3
                    while Path(f"/proc/{pid}/exe").resolve() == native / "runtime" and time.monotonic() < deadline:
                        time.sleep(.025)
                assert Path(f"/proc/{pid}/exe").resolve() != native / "runtime", "owned server remained alive"

        try:
            for name in ["work", "override", "idle"]:
                assert profile("create", name).stdout == f"created: {name}\n".encode()
                (profiles / name / "home/config.toml").write_text(config)
            account = profiles / "work/home"
            override = profiles / "override/home"
            assert profile("default", "work").stdout == b"default: work\n"
            assert profile("current").stdout == b"current: work\nsource: saved\n"
            assert run("--version").stdout == expected_version
            assert (account / "sessions").is_symlink(), "actual Core must prepare selected home"
            process, master = launch()
            pid = bind(account, process, master)
            assert Path(f"/proc/{pid}/fd/36").stat().st_ino == account.stat().st_ino
            lease = os.open(account, os.O_RDONLY)
            try:
                try:
                    fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    pass
                else:
                    raise AssertionError("actual native server lost home lease")
            finally:
                os.close(lease)
            assert profile("delete", "work", success=False).returncode == 1
            profile("default", "default")
            assert profile("rename", "work", "new", success=False).returncode == 1
            assert profile("delete", "work", success=False).returncode == 1
            # An unrelated idle profile remains mutable while work is running.
            assert profile("rename", "idle", "renamed-idle").stdout == b"renamed: idle -> renamed-idle\n"
            profile("delete", "renamed-idle")
            profile("default", "work")
            second, second_master = launch(override)
            override_pid = bind(override, second, second_master)
            assert override_pid != pid
            assert profile("current", account=override).stdout == b"current: override\nsource: inherited\n"
            print("PASS native saved default, inherited priority, FD36 server lease and active/idle profile separation")
            process.terminate()
            process.wait(timeout=5)
            profile("default", "default")
            assert profile("delete", "work", success=False).returncode == 1, "persistent server still holds lease"
            stop_owned()
            local = account / "owned-local-settings"
            local.write_bytes(b"local-owned-sentinel")
            original = local.stat()
            assert profile("rename", "work", "renamed").stdout == b"renamed: work -> renamed\n"
            after = (profiles / "renamed/home/owned-local-settings").stat()
            assert (original.st_ino, original.st_mode) == (after.st_ino, after.st_mode)
            # Prepared compatibility links must survive a rename and another launch.
            profile("default", "renamed")
            assert run("--version").stdout == expected_version
            renamed_client, renamed_master = launch()
            bind(profiles / "renamed/home", renamed_client, renamed_master)
            profile("default", "default")
            assert profile("delete", "renamed", success=False).returncode == 1
            stop_owned()
            shared = home / ".codex"
            sentinel = shared / "sessions/owned-shared-sentinel"
            sentinel.write_bytes(b"shared-owned-sentinel")
            before = {str(p.relative_to(shared)): hashlib.sha256(p.read_bytes()).hexdigest() for p in shared.rglob("*") if p.is_file() and not p.is_symlink()}
            assert profile("delete", "renamed").stdout == b"deleted: renamed\n"
            assert not (profiles / "renamed").exists()
            after = {str(p.relative_to(shared)): hashlib.sha256(p.read_bytes()).hexdigest() for p in shared.rglob("*") if p.is_file() and not p.is_symlink()}
            assert before == after
            print("PASS native prepared-home rename, inode preservation and delete without shared-history changes")
            preference = home / ".local/share/codex/manager/default-profile-v1"
            preference.write_bytes(b"invalid-owned-preference")
            assert run("--version").stdout == expected_version
            assert profile("default", success=False).returncode == 1
            assert preference.read_bytes() == b"invalid-owned-preference"
            preference.write_bytes(b"codex-manager-default-profile-v1\nprofile\tdefault\n")
            manager = native / "manager"
            manager.rename(native / "manager.unavailable")
            try:
                assert run("--version").stdout == expected_version
            finally:
                (native / "manager.unavailable").rename(manager)
            print("PASS native malformed preference and optional Manager absence preserve ordinary launch")
        finally:
            stop_owned()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public-key", required=True, type=Path)
    parser.add_argument("--generation", required=True, type=Path)
    parser.add_argument("--workdir", required=True, type=Path)
    options = parser.parse_args()
    qualify(options.generation.resolve(), options.workdir.resolve(), options.public_key.resolve())
