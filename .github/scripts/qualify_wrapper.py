#!/usr/bin/env python3
"""Inspect official inputs or qualify Wrapper in a fresh, explicitly owned root.

Uses the public updater as the only release resolver/downloader/adaptor. No
credentials, account homes, services, or installed generation pointers are used.
The observed OS/page size is evidence, not an allowlist or a deployment gate.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import shlex
import shutil
import signal
import stat
import struct
import subprocess
import sys
import termios
import time
import uuid


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def filesystem_snapshot(roots):
    entries = {}

    def visit(path):
        info = path.lstat()
        entries[path] = (info.st_mode, info.st_size, info.st_mtime_ns, info.st_ctime_ns, info.st_ino)
        if stat.S_ISDIR(info.st_mode):
            for child in path.iterdir():
                visit(child)

    for root in roots:
        visit(root)
    return entries


def require_unchanged(label, before, roots):
    after = filesystem_snapshot(roots)
    changed = sorted(path for path in before.keys() | after.keys()
                     if before.get(path) != after.get(path))
    if changed:
        raise RuntimeError(f"{label}: changed {len(changed)} snapshot entries: {changed[:6]}")


def elf_16k(path):
    with Path(path).open("rb") as stream:
        header = stream.read(64)
        if header[:6] != b"\x7fELF\x02\x01":
            raise ValueError(f"not ELF64 LE: {path}")
        offset = struct.unpack_from("<Q", header, 32)[0]
        size, count = struct.unpack_from("<HH", header, 54)
        if size != 56 or not 0 < count < 256:
            raise ValueError("invalid ELF program headers")
        stream.seek(offset)
        headers = stream.read(size * count)
    loads = []
    for index in range(count):
        kind, _, offset, address, _, _, _, alignment = struct.unpack_from("<IIQQQQQQ", headers, index * size)
        if kind == 1:
            loads.append(alignment >= 16384 and (offset - address) % 16384 == 0)
    return bool(loads) and all(loads)


def verify_generation(generation, release):
    rows = (generation / "generation.meta").read_text().splitlines()[1:]
    fields = dict(line.split("\t", 1) for line in rows
                  if "\t" in line and not line.startswith(("helper\t", "companion\t")))
    if (fields["upstream_package_version"] != release["upstream_version"]
            or fields["source_artifact_digest"] != release["archive_sha256"]):
        raise RuntimeError("admitted generation differs from immutable discovery inputs")

    declared = [tuple(line.split("\t")[1:]) for line in rows if line.startswith("companion\t")]
    expected = []
    for path in sorted((generation / "bin").iterdir()):
        if path.name == "codex":
            continue
        mode = path.lstat().st_mode
        if not stat.S_ISREG(mode) or not mode & 0o111:
            raise RuntimeError(f"generation companion is not an executable regular file: {path.name}")
        expected.append((f"bin/{path.name}", digest(path)))
    if sorted(declared) != expected:
        raise RuntimeError("generation.meta companion rows differ from installed bin/* digests")
    return [name for name, _ in expected]


class Qualification:
    def __init__(self, args):
        self.args = args
        self.root = Path(args.root).resolve()
        self.root.mkdir(mode=0o700)  # No adoption/replay of a previous root.
        self.prefix = self.root / "prefix"
        self.home = self.root / "home"
        self.tmp = self.root / ("temporary-" + "t" * 60)
        self.codex_home = self.root / ("codex-home-" + "c" * 60)
        self.workspace = self.root / "workspace"
        for path in (self.prefix / "bin", self.prefix / "etc/tls/certs", self.home,
                     self.tmp, self.codex_home, self.workspace):
            path.mkdir(mode=0o700, parents=True, exist_ok=True)
        original_prefix = Path(os.environ.get("PREFIX", "/usr"))
        for name in ("sh", "curl", "gzip", "tar", "ps", "sha256sum"):
            source = original_prefix / "bin" / name
            if not source.is_file():
                found = shutil.which(name)
                if not found:
                    raise RuntimeError(f"required host tool missing: {name}")
                source = Path(found)
            target = self.prefix / "bin" / name
            shutil.copyfile(source, target)
            target.chmod(0o755)
        certificates = [original_prefix / "etc/tls/cert.pem", Path("/etc/ssl/certs/ca-certificates.crt")]
        resolver = [original_prefix / "etc/resolv.conf", Path("/etc/resolv.conf")]
        shutil.copyfile(next(p for p in certificates if p.is_file()), self.prefix / "etc/tls/cert.pem")
        shutil.copyfile(next(p for p in resolver if p.is_file()), self.prefix / "etc/resolv.conf")
        self.env = {k: v for k, v in os.environ.items()
                    if not k.startswith(("CODEX_", "OPENAI_", "LD_", "DYLD_", "XDG_"))
                    and k not in ("ENV", "BASH_ENV")}
        self.env.update(HOME=str(self.home), CODEX_HOME=str(self.codex_home), PREFIX=str(self.prefix),
                        TMPDIR=str(self.tmp), PATH=f"{self.prefix}/bin:/system/bin:/bin:/usr/bin", TERM="xterm-256color")
        self.binary = self.prefix / "bin/codex"
        self.daemon_possible = False
        self.runtime_present = False
        self.report = {"schema_version": 1, "status": "running", "steps": {},
                       "environment": {"machine": os.uname().machine, "kernel": os.uname().release,
                                       "page_size": os.sysconf("SC_PAGE_SIZE"), "android_api": None},
                       "authenticated_inference": "not_requested", "external_pairing": "not_requested"}
        getprop = Path("/system/bin/getprop")
        if getprop.is_file():
            prop = subprocess.run([str(getprop), "ro.build.version.sdk"], capture_output=True, text=True, timeout=5)
            if prop.returncode == 0 and prop.stdout.strip().isdigit():
                self.report["environment"]["android_api"] = int(prop.stdout.strip())
        self.save()

    def save(self):
        temporary = self.root / ".report.tmp"
        temporary.write_text(json.dumps(self.report, indent=2) + "\n")
        temporary.replace(self.root / "report.json")

    def run(self, label, args, expected=(0,), timeout=30, env=None):
        out, err = self.root / f"{label}.stdout", self.root / f"{label}.stderr"
        with out.open("xb") as stdout, err.open("xb") as stderr:
            child = subprocess.Popen([str(a) for a in args], env=self.env if env is None else env, cwd=self.workspace,
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, start_new_session=True)
            try:
                code = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                self.report["steps"][label] = {"status": "timed_out", "timeout_seconds": timeout}
                self.save()
                raise RuntimeError(f"{label}: timed out; original receipt retained")
        self.report["steps"][label] = {"exit_code": code, "stdout_bytes": out.stat().st_size,
                                       "stderr_bytes": err.stat().st_size}
        self.save()
        print(f"{label}: exit {code}", flush=True)
        if code not in expected:
            raise RuntimeError(f"{label}: unexpected exit {code}; see {err}")
        if out.stat().st_size > 1024 * 1024 or err.stat().st_size > 1024 * 1024:
            raise RuntimeError(f"{label}: oversized diagnostic output")
        return out.read_bytes()

    def cli(self, label, *args, **kwargs):
        return self.run(label, [self.binary, *args], **kwargs)

    def status(self, label):
        return json.loads(self.cli(label, "wrapper", "status", "--json"))

    def read_only_doctor(self, label, *homes, env=None):
        roots = (self.home, self.prefix, *homes)
        before = filesystem_snapshot(roots)
        try:
            output = self.cli(label, "doctor", "--json", expected=(0, 1), timeout=120, env=env)
        finally:
            require_unchanged(label, before, roots)
        doctor = json.loads(output)
        if doctor["termux_wrapper"]["status"] != "healthy":
            raise RuntimeError(f"{label}: Wrapper diagnosis is unhealthy")
        if doctor["upstream"]["status"] not in ("healthy", "unhealthy"):
            raise RuntimeError(f"{label}: upstream doctor did not complete")
        native = json.loads(doctor["upstream"]["output"])
        self.report["steps"][label]["read_only"] = True
        return doctor, native["checks"]

    def verify_shared_profiles(self, version):
        shared = self.root / "shared-home"
        shared.mkdir(mode=0o700)
        index = shared / "session_index.jsonl"
        extra = shared / "custom.config.toml"
        extra.write_text("# Shared qualification fixture\n")
        profiles = [self.root / name for name in ("profile-a", "profile-b")]
        auth = {}
        for profile in profiles:
            profile.mkdir(mode=0o700)
            auth[profile / "auth.json"] = json.dumps({"qualification_profile": profile.name}).encode() + b"\n"
            (profile / "auth.json").write_bytes(auth[profile / "auth.json"])

        thread_id = str(uuid.uuid4())
        rows = [json.dumps({"id": thread_id, "thread_name": name, "updated_at": timestamp}).encode() + b"\n"
                for name, timestamp in (("Initial", "2026-01-01T00:00:00Z"), ("Renamed", "2026-01-01T00:01:00Z"))]
        index.write_bytes(rows[0])
        # A detached log still contains the original row as well as the rename.
        (profiles[0] / index.name).write_bytes(b"".join(rows))

        shared_env = {**self.env, "CODEX_SHARED_HOME": str(shared), "CODEX_SHARED_EXTRA_FILES": extra.name}
        profile_envs = [{**shared_env, "CODEX_HOME": str(profile)} for profile in profiles]
        roots = (self.home, *profiles, shared)
        before = filesystem_snapshot(roots)
        for profile, env in zip(profiles, profile_envs):
            output = self.cli(f"{profile.name}-version", "--version", env=env)
            if output != f"codex-cli {version}\n".encode():
                raise RuntimeError(f"{profile.name}: fast version output differs from qualified release")
        require_unchanged("shared profile fast path", before, roots)
        self.read_only_doctor("shared-doctor", *profiles, shared, env=profile_envs[0])

        for profile, env in zip(profiles, profile_envs):
            self.cli(f"{profile.name}-features", "features", "list", env=env)
            for target in (extra, index):
                link = profile / target.name
                if not link.is_symlink() or link.readlink() != target:
                    raise RuntimeError(f"{profile.name}: wrong shared target for {target.name}")
        if index.read_bytes() != b"".join(rows):
            raise RuntimeError("shared session index lost or duplicated a same-thread rename row")
        if os.path.lexists(shared / "auth.json"):
            raise RuntimeError("auth.json was written into the shared home")
        for path, expected in auth.items():
            if path.is_symlink() or not path.is_file() or path.read_bytes() != expected:
                raise RuntimeError(f"{path.parent.name}: auth.json was shared or changed")
        self.report["shared_home"] = {"profiles": len(profiles), "session_index_rows": len(rows),
                                      "extra_file_symlinked": True, "auth_isolated": True}

    def verify_wrapper_upgrade(self, wrapper, installer, active, previous, replacement):
        state = Path(active["state_root"]) / "activation-state"
        current = Path(active["generation_root"]).parent / "current"
        slots = {"generation_id": active["generation_id"],
                 "previous_generation_id": previous["generation_id"]}
        if active["previous_generation_id"] != slots["previous_generation_id"]:
            raise RuntimeError("upstream upgrade did not retain the expected previous generation")
        state_before, link_before = state.read_bytes(), current.readlink()
        if replacement is None:
            replacement = self.root / "upgrade-wrapper"
            shutil.copyfile(wrapper, replacement)
            replacement.chmod(0o755)
            # An ELF overlay changes the artifact hash without changing loaded segments.
            with replacement.open("ab") as stream:
                stream.write(b"\nWrapper-only upgrade qualification\n")
        replacement_sha = digest(replacement)
        if replacement_sha == digest(self.binary):
            raise RuntimeError("replacement Wrapper must have a different SHA-256")

        curl = self.prefix / "bin/curl"
        original_curl = self.root / "original-curl"
        curl_called = self.root / "upgrade-curl-called"
        curl.rename(original_curl)
        try:
            curl.write_text(f"#!{self.prefix / 'bin/sh'}\n: > {shlex.quote(str(curl_called))}\nexit 57\n")
            curl.chmod(0o755)
            output = self.run("wrapper-upgrade", [self.prefix / "bin/sh", installer,
                                                  "--binary", replacement, "--prefix", self.prefix])
            status = json.loads(self.cli("wrapper-upgrade-status", "wrapper", "status", "--verify", "--json"))
        finally:
            original_curl.replace(curl)
        if curl_called.exists() or b"retaining the qualified active upstream generation\n" not in output:
            raise RuntimeError("Wrapper upgrade did not retain the generation offline without the updater")
        if digest(self.binary) != replacement_sha:
            raise RuntimeError("installed Wrapper differs from the replacement artifact")
        if (any(status[key] != value for key, value in slots.items())
                or state.read_bytes() != state_before or current.readlink() != link_before):
            raise RuntimeError("Wrapper upgrade changed current or previous upstream generation")
        self.report["wrapper_upgrade"] = {**slots, "replacement_sha256": replacement_sha, "offline": True}

    def native_rpc(self):
        nonce = uuid.uuid4().hex
        proof = self.workspace / "native-proof.txt"
        stderr = (self.root / "native-rpc.stderr").open("xb")
        server = subprocess.Popen([str(self.binary), "app-server", "--listen", "stdio://"],
                                  env=self.env, cwd=self.workspace, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=stderr, bufsize=0, start_new_session=True)
        pending = bytearray()

        def rpc(identifier, method, params):
            server.stdin.write((json.dumps({"id": identifier, "method": method, "params": params}) + "\n").encode())
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if b"\n" in pending:
                    line, _, rest = pending.partition(b"\n")
                    pending[:] = rest
                    reply = json.loads(line)
                    if reply.get("id") == identifier:
                        if "error" in reply:
                            raise RuntimeError(f"{method}: RPC rejected")
                        return reply["result"]
                    continue
                ready, _, _ = select.select([server.stdout], [], [], max(0, deadline - time.monotonic()))
                if not ready:
                    break
                block = os.read(server.stdout.fileno(), 65536)
                if not block or len(pending) + len(block) > 65536:
                    raise RuntimeError(f"{method}: missing or oversized response")
                pending.extend(block)
            raise RuntimeError(f"{method}: RPC timeout")

        try:
            rpc(1, "initialize", {"clientInfo": {"name": "wrapper-qualification", "version": "1"},
                                   "capabilities": {"experimentalApi": True}})
            server.stdin.write(b'{"method":"initialized"}\n')
            rpc(2, "config/read", {"includeLayers": False})
            command = f"printf '%s\\n' {shlex.quote(nonce)} > {shlex.quote(str(proof))}; printf '%s' {shlex.quote(nonce)}"
            result = rpc(3, "command/exec", {"command": [str(self.prefix / "bin/sh"), "-c", command],
                                           "cwd": str(self.workspace), "timeoutMs": 5000})
            if result["exitCode"] != 0 or result["stdout"] != nonce or proof.read_bytes() != (nonce + "\n").encode():
                raise RuntimeError("Native command did not produce the requested independent file effect")
            self.report["native_rpc"] = {"initialize": True, "config_read": True, "command_exec": True,
                                          "file_sha256": digest(proof)}
        finally:
            server.stdin.close()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(server.pid, signal.SIGTERM)
                server.wait(timeout=5)
            stderr.close()

    def pty_startup(self):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        self.daemon_possible = True
        child = subprocess.Popen([str(self.binary)], env=self.env, cwd=self.workspace,
                                 stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        output = bytearray()
        deadline = time.monotonic() + 8
        replies = [(b"\x1b[6n", b"\x1b[1;1R"), (b"\x1b[c", b"\x1b[?1;2c"),
                   (b"\x1b[?u", b"\x1b[?0u"), (b"\x1b]10;?", b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\"),
                   (b"\x1b]11;?", b"\x1b]11;rgb:0000/0000/0000\x1b\\")]
        try:
            while time.monotonic() < deadline and child.poll() is None:
                ready, _, _ = select.select([master], [], [], 0.1)
                if ready:
                    try:
                        block = os.read(master, 65536)
                    except OSError:
                        break
                    output.extend(block)
                    if len(output) > 1024 * 1024:
                        raise RuntimeError("PTY output exceeded bound")
                    for query, response in replies:
                        if query in block:
                            os.write(master, response)
            if child.poll() is not None or b"Codex" not in output or b"SUN_LEN" in output:
                raise RuntimeError("default PTY did not render Codex and remain alive")
            self.report["pty"] = {"rendered": True, "alive_seconds": 8, "output_bytes": len(output)}
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
            os.close(master)
            (self.root / "pty.raw").write_bytes(output)

    def execute(self):
        wrapper = Path(self.args.wrapper).resolve()
        installer = Path(self.args.installer).resolve()
        self.run("install", [self.prefix / "bin/sh", installer, "--binary", wrapper,
                             "--prefix", self.prefix, "--skip-upstream"])
        if digest(self.binary) != digest(wrapper):
            raise RuntimeError("installed wrapper differs from selected artifact")
        check_args = ["update", "--check", "--json"]
        if self.args.version:
            check_args += ["--version", self.args.version]
        selected = json.loads(self.cli("release-check", *check_args))
        if selected["schema_version"] != 1 or selected["status"] == "older":
            raise RuntimeError("discovery schema or downgrade rejected")
        previous_release = None
        if self.args.previous_version:
            previous_release = json.loads(self.cli("previous-release-check", "update", "--check", "--json",
                                                   "--version", self.args.previous_version))
            if (previous_release["schema_version"] != 1
                    or previous_release["upstream_version"] != self.args.previous_version
                    or previous_release["upstream_version"] == selected["upstream_version"]):
                raise RuntimeError("cross-version rollback requires a distinct, exact previous release")
        if (self.home / ".local").exists():
            raise RuntimeError("read-only discovery wrote installed state")
        replacement = Path(self.args.upgrade_wrapper).resolve() if self.args.upgrade_wrapper else None
        inputs = {"release": selected, "previous_release": previous_release, "source_sha": self.args.source_sha,
                  "wrapper_sha256": digest(wrapper), "upgrade_wrapper_sha256": digest(replacement) if replacement else None,
                  "execution_environment": {**self.report["environment"], "runner_image": os.environ.get("ImageVersion")},
                  "toolchain": {key: os.environ.get(key) for key in
                                ("RUST_VERSION", "ANDROID_NDK_VERSION", "ANDROID_API", "ANDROID_RUNTIME_APEX_SHA256")}}
        self.report["input_id"] = hashlib.sha256(json.dumps(inputs, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        self.report["inputs"] = inputs
        if self.args.inspect_only:
            self.report["status"] = "discovered"
            return
        if previous_release:
            self.cli("previous-install", "update", "--version", previous_release["upstream_version"], timeout=420)
            previous = self.status("previous-status")
            verify_generation(Path(previous["runtime_path"]).parents[1], previous_release)
        else:
            previous = None
        if self.args.archive:
            archive = Path(self.args.archive).resolve()
            if digest(archive) != selected["archive_sha256"]:
                raise RuntimeError("archive differs from discovered official digest")
            self.cli("candidate-update", "update", "--path", archive, timeout=180)
        else:
            self.cli("candidate-update", "update", "--version", selected["upstream_version"], timeout=420)
        self.runtime_present = True
        active = self.status("candidate-status")
        companions = verify_generation(Path(active["runtime_path"]).parents[1], selected)
        if previous is None:
            # Exercise exact-generation rollback even when no older archive was
            # requested. Reuse the public updater's verified cached input; this
            # tool neither downloads nor adapts a second copy itself.
            archive = Path(self.args.archive).resolve() if self.args.archive else next(
                (path for path in (Path(active["state_root"]) / "downloads").iterdir()
                 if path.is_file() and not path.is_symlink() and digest(path) == selected["archive_sha256"]), None)
            if archive is None:
                raise RuntimeError("verified archive unavailable for rollback qualification")
            previous = active
            self.cli("rollback-candidate-update", "update", "--path", archive, timeout=180)
            active = self.status("rollback-candidate-status")
            companions = verify_generation(Path(active["runtime_path"]).parents[1], selected)
        generation = Path(active["runtime_path"]).parents[1]
        if active["upstream_version"] != selected["upstream_version"]:
            raise RuntimeError("active runtime version differs from immutable discovery inputs")
        self.report["verified_companions"] = companions
        self.cli("version", "--version")
        self.cli("help", "--help")
        self.cli("features", "features", "list")
        layouts = {"wrapper": elf_16k(wrapper)}
        for executable in sorted((generation / "bin").iterdir()):
            if executable.is_file():
                layouts[executable.name] = elf_16k(executable)
                if executable.name == "codex-exec-server":
                    self.run("exec-server", [executable, "--version"])
                elif executable.name == "codex-code-mode-host":
                    self.run("code-mode-host", [executable, "--help"])
        self.report["elf_16k_alignment"] = layouts
        if not all(layouts.values()):
            raise RuntimeError("ELF alignment is incompatible with 16 KiB pages")
        self.daemon_possible = True
        started = json.loads(self.cli("daemon-start", "app-server", "daemon", "start", timeout=120))
        pid = started["pid"]
        daemon_package = self.codex_home / "packages/app-server-daemon"
        package_link = daemon_package / "current"
        standalone = self.home / ".codex/packages/standalone/current"
        managed_path = Path(started["managedCodexPath"])
        if started["status"] != "started" or managed_path != package_link / "bin/codex":
            raise RuntimeError("daemon start or ownership rejected")
        if (not package_link.is_symlink() or package_link.readlink() != standalone
                or (daemon_package / "releases").exists()):
            raise RuntimeError("fresh daemon home did not select the standalone package link")
        managed = managed_path.resolve(strict=True)
        expected = generation / "bin/codex"
        if managed != expected or Path(f"/proc/{pid}/exe").readlink() != expected:
            raise RuntimeError("daemon executable identity differs from qualified artifact")
        running = json.loads(self.cli("daemon-version", "app-server", "daemon", "version"))
        if running["status"] != "running" or running["appServerVersion"] != selected["upstream_version"]:
            raise RuntimeError("daemon control connection failed")
        doctor, checks = self.read_only_doctor("doctor", self.codex_home)
        if checks["app_server.status"]["status"] != "ok":
            raise RuntimeError("actual app-server diagnosis is unhealthy")
        self.report["doctor"] = {"wrapper": "healthy", "app_server": "ok", "overall": doctor["summary"]["status"],
                                  "upstream_failed_checks": [name for name, value in checks.items() if value["status"] == "fail"],
                                  "upstream_warnings": [name for name, value in checks.items() if value["status"] == "warning"]}
        self.cli("daemon-stop", "app-server", "daemon", "stop")
        self.native_rpc()
        self.pty_startup()
        self.cli("pty-daemon-stop", "app-server", "daemon", "stop")
        self.daemon_possible = False
        self.verify_shared_profiles(selected["upstream_version"])
        self.verify_wrapper_upgrade(wrapper, installer, active, previous, replacement)
        self.cli("rollback", "update", "--rollback")
        restored = self.status("rollback-status")
        if (restored["generation_id"] != previous["generation_id"]
                or restored["upstream_version"] != previous["upstream_version"]
                or restored["previous_generation_id"] != active["generation_id"]):
            raise RuntimeError("rollback did not swap the expected current and previous generations")
        if self.cli("rollback-version", "--version").strip() != f"codex-cli {previous['upstream_version']}".encode():
            raise RuntimeError("rollback did not launch the previous release")
        self.cli("restore-candidate", "update", "--rollback")
        restored = self.status("restored-status")
        if (restored["generation_id"] != active["generation_id"]
                or restored["previous_generation_id"] != previous["generation_id"]):
            raise RuntimeError("candidate restoration did not restore the original generation slots")
        self.report["rollback"] = {"previous_version": previous["upstream_version"],
                                   "cross_version": previous_release is not None,
                                   "exact_generation_restored": True}
        self.report["status"] = "passed"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wrapper", required=True)
    parser.add_argument("--root", required=True, help="fresh, absent output directory; never a real HOME/PREFIX")
    parser.add_argument("--installer", default=str(Path(__file__).resolve().parents[2] / "install.sh"))
    parser.add_argument("--version")
    parser.add_argument("--archive")
    parser.add_argument("--previous-version")
    parser.add_argument("--upgrade-wrapper", metavar="PATH",
                        help="compatible replacement binary with a distinct digest; defaults to an ELF copy with an overlay")
    parser.add_argument("--source-sha", default=os.environ.get("GITHUB_SHA", "local-working-tree"))
    parser.add_argument("--inspect-only", action="store_true")
    args = parser.parse_args()
    job = Qualification(args)
    try:
        job.execute()
    except Exception as error:
        job.report["status"] = "failed"
        job.report["failure"] = str(error)
        raise
    finally:
        if job.daemon_possible and job.runtime_present:
            try:
                job.cli("failure-daemon-stop", "app-server", "daemon", "stop")
            except Exception:
                job.report["cleanup"] = "unconfirmed; retain owned root and reconcile"
        job.save()
    print(json.dumps({"status": job.report["status"], "input_id": job.report["input_id"],
                      "report": str(job.root / "report.json")}))


if __name__ == "__main__":
    main()
