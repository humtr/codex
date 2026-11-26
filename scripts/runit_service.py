#!/usr/bin/env python3
"""Opt-in runit 3-state generation supervision for standalone services.

Manages immutable component generations under:
  $HOME/.local/lib/codex/<component>/{generations/<version>/<bin>, current, previous}

And opt-in Termux runit services under:
  $PREFIX/var/service/<service>/{run, log/run}
  (logs under $HOME/.local/state/codex/services/<service>/log)

For tunnel-wrapped MCP services (e.g. `termux-shell-tunnel`, `termux-codex-tunnel`,
`termux-chatgpt-tunnel`), pass `--runner-component transport/tunnel-client
--runner-binary tunnel-client` alongside `--component mcp/<name> --binary <mcp-bin>`.
The `run` script executes `<runner>/current/<runner-bin>`, while `status` / `activate`
verify BOTH the running tunnel image (`/proc/<pid>/exe`) and the linked MCP component
generation (recording verified active generations under
`$HOME/.local/state/codex/services/<service>/active-generation.json`).

Invariants:
- Installing/updating a component generation NEVER auto-enrolls a new runit service.
- Service `run` scripts execute the stable `current/<bin>` symlink (never hardcoded
  versioned paths), surviving reboots and upgrades cleanly.
- `status` reports 3 separate states:
    1. `selected_generation` (from `current` symlink + binary SHA-256)
    2. `running_generation` (from live PID `/proc/<pid>/exe` + start ticks)
    3. `activation_state` (`active` | `pending_activation` | `stopped` | `not_enrolled` | `error`)
- Safe activation checks the busy guard, stops the selected services, changes
  generation links, and starts and verifies each service. A failed activation
  restores the old generation after stopping the attempted services.
"""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time

NAME_RE = re.compile(r"^[a-z0-9][a-z0-9._-]{0,63}$")
VER_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]{0,63}$")

# Match install.sh component_specs: installation directories are not release tags.
RELEASE_COMPONENTS = {
    "providers/chatgpt": "chatgpt-provider",
    "transport/tunnel-client": "tunnel-client",
    "mcp/termux-shell": "termux-shell-mcp",
    "mcp/termux-codex": "termux-codex-mcp",
    "mcp/termux-chatgpt": "termux-chatgpt-mcp",
    "mcp/termux-orch": "termux-orch",
}

KNOWN_CLI_FLAGS = frozenset(
    {
        "--component",
        "--binary",
        "--service",
        "--runner-component",
        "--runner-binary",
        "--companion-service",
        "--home",
        "--prefix",
        "--readiness-cmd",
        "--busy-check-cmd",
        "--timeout",
        "--arg",
        "--env",
        "-h",
        "--help",
    }
)


class SupervisionError(RuntimeError):
    pass


def validate_token(value: str, regex: re.Pattern, label: str) -> str:
    if not regex.fullmatch(value) or value in (".", ".."):
        raise SupervisionError(f"invalid {label}: {value!r}")
    return value


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def read_checksums(path: Path) -> dict[str, str]:
    """Parse unique SHA-256 entries in release and extracted bundle manifests."""
    if path.is_symlink() or not path.is_file():
        raise ValueError("checksum manifest is not a regular file")
    entries = {}
    for line in path.read_text(encoding="ascii").splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        fields = line.split()
        if len(fields) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", fields[0]):
            raise ValueError("malformed checksum entry")
        name = fields[1].removeprefix("*").removeprefix("./")
        if name in entries:
            raise ValueError("duplicate checksum entry")
        entries[name] = fields[0].lower()
    return entries


def fsync_dir(path: Path) -> None:
    fd = os.open(str(path), os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def ensure_private_dir(path: Path) -> None:
    if path.is_symlink():
        raise SupervisionError(f"refusing symlink directory: {path}")
    path.mkdir(parents=True, exist_ok=True)
    os.chmod(path, 0o700)


def atomic_write_private(path: Path, body: str, mode: int = 0o600) -> None:
    parent = path.parent
    ensure_private_dir(parent)
    tmp = parent / f".{path.name}.tmp.{os.getpid()}.{time.time_ns()}"
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(str(tmp), flags, mode)
    try:
        os.fchmod(fd, mode)
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            fd = -1
            f.write(body)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
    except BaseException:
        if fd >= 0:
            try:
                os.close(fd)
            except OSError:
                pass
        try:
            tmp.unlink()
        except OSError:
            pass
        raise
    fsync_dir(parent)


def atomic_symlink(link_path: Path, target: str) -> None:
    parent = link_path.parent
    ensure_private_dir(parent)
    tmp = parent / f".{link_path.name}.tmp.{os.getpid()}.{time.time_ns()}"
    os.symlink(target, tmp)
    try:
        os.replace(tmp, link_path)
    except BaseException:
        try:
            tmp.unlink()
        except OSError:
            pass
        raise
    fsync_dir(parent)


def proc_start_ticks(pid: int) -> int | None:
    try:
        raw = Path(f"/proc/{pid}/stat").read_text()
    except OSError:
        return None
    parts = raw.rsplit(") ", 1)
    if len(parts) != 2:
        return None
    fields = parts[1].split()
    if len(fields) < 20 or fields[0] in ("Z", "X"):
        return None
    try:
        return int(fields[19])
    except ValueError:
        return None


def proc_exe_path(pid: int) -> str | None:
    try:
        return os.readlink(f"/proc/{pid}/exe")
    except OSError:
        return None


def proc_children(pid: int, start_ticks: int) -> list[tuple[int, int, str]] | None:
    children: list[tuple[int, int, str]] = []
    try:
        entries = list(Path("/proc").iterdir())
    except OSError:
        return None
    for entry in entries:
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / "stat").read_text().rsplit(") ", 1)[1].split()
            if int(fields[1]) == pid and fields[0] not in ("Z", "X"):
                try:
                    exe = os.readlink(entry / "exe")
                except OSError:
                    exe = ""
                child_pid, child_ticks = int(entry.name), int(fields[19])
                if proc_start_ticks(child_pid) == child_ticks:
                    children.append((child_pid, child_ticks, exe))
        except (OSError, ValueError, IndexError):
            continue
    return children if proc_start_ticks(pid) == start_ticks else None


def proc_child_exes(pid: int, start_ticks: int) -> list[str]:
    return [exe for _, _, exe in proc_children(pid, start_ticks) or ()]


def normalize_command(cmd: list[str] | str | None) -> list[str] | None:
    if cmd is None:
        return None
    if isinstance(cmd, str):
        tokens = shlex.split(cmd)
        if not tokens:
            raise SupervisionError("command must not be empty")
        return tokens
    if len(cmd) == 1 and any(ch.isspace() for ch in cmd[0]):
        tokens = shlex.split(cmd[0])
        if not tokens:
            raise SupervisionError("command must not be empty")
        return tokens
    if not cmd:
        raise SupervisionError("command must not be empty")
    return list(cmd)


def preprocess_cli_argv(argv: list[str]) -> list[str]:
    """Normalize `--arg <val>` and `--readiness-cmd`/`--busy-check-cmd` tokens
    so hyphenated subcommand flags (e.g. `curl -fsS http://...` or `--arg --profile`)
    are never misparsed as `runit_service.py` options by argparse.
    """
    out: list[str] = []
    i = 0
    n = len(argv)
    while i < n:
        tok = argv[i]
        if tok == "--arg" and i + 1 < n:
            out.append(f"--arg={argv[i + 1]}")
            i += 2
            continue
        if tok in ("--readiness-cmd", "--busy-check-cmd"):
            j = i + 1
            collected: list[str] = []
            while j < n:
                nxt = argv[j]
                flag_name = nxt.split("=", 1)[0]
                if flag_name in KNOWN_CLI_FLAGS:
                    break
                collected.append(nxt)
                j += 1
            if not collected:
                # Let argparse produce its normal missing-argument error.
                out.append(tok)
                i += 1
                continue
            if len(collected) == 1:
                cmd_str = collected[0]
            else:
                cmd_str = shlex.join(collected)
            out.append(f"{tok}={cmd_str}")
            i = j
            continue
        out.append(tok)
        i += 1
    return out


def validate_component_path(value: str) -> str:
    parts = value.split("/")
    if not (1 <= len(parts) <= 3):
        raise SupervisionError(f"invalid component path: {value!r}")
    for seg in parts:
        validate_token(seg, NAME_RE, "component")
    return "/".join(parts)


class ServiceSupervisor:
    def __init__(
        self,
        *,
        component: str,
        binary_name: str,
        service_name: str,
        runner_component: str | None = None,
        runner_binary: str | None = None,
        companion_services: list[str] | tuple[str, ...] | None = None,
        home: Path | None = None,
        prefix: Path | None = None,
        sv_bin: str = "sv",
        proc_reader=None,
    ):
        self.component = validate_component_path(component)
        self.binary_name = validate_token(binary_name, NAME_RE, "binary_name")
        self.service_name = validate_token(service_name, NAME_RE, "service_name")
        if (runner_component is None) != (runner_binary is None):
            raise SupervisionError(
                "--runner-component and --runner-binary must either both be set or both be omitted"
            )
        self.runner_component = (
            validate_component_path(runner_component) if runner_component is not None else None
        )
        self.runner_binary = (
            validate_token(runner_binary, NAME_RE, "runner_binary")
            if runner_binary is not None
            else None
        )
        companions = (
            validate_token(name, NAME_RE, "companion_service")
            for name in companion_services or ()
        )
        self.companion_services = tuple(dict.fromkeys(
            name for name in companions if name != self.service_name
        ))
        self.home = home or Path.home()
        self.prefix = prefix or Path(os.environ.get("PREFIX", "/data/data/com.termux/files/usr"))
        self.sv_bin = sv_bin
        self._proc_reader = proc_reader

        self.comp_root = self.home / ".local/lib/codex" / self.component
        self.generations_dir = self.comp_root / "generations"
        self.current_link = self.comp_root / "current"
        self.previous_link = self.comp_root / "previous"
        self.service_dir = self.prefix / "var/service" / self.service_name
        self.state_dir = self.home / ".local/state/codex/services" / self.service_name
        self.active_marker_path = self.state_dir / "active-generation.json"

        if self.runner_component is not None and self.runner_binary is not None:
            self.runner_root: Path | None = self.home / ".local/lib/codex" / self.runner_component
            self.runner_generations_dir: Path | None = self.runner_root / "generations"
            self.runner_current_link: Path | None = self.runner_root / "current"
        else:
            self.runner_root = None
            self.runner_generations_dir = None
            self.runner_current_link = None

    @property
    def has_runner(self) -> bool:
        return self.runner_component is not None and self.runner_binary is not None

    def generation_dir(self, version: str) -> Path:
        validate_token(version, VER_RE, "version")
        return self.generations_dir / version

    def generation_binary(self, version: str) -> Path:
        return self.generation_dir(version) / self.binary_name

    def _read_link_version(self, link: Path) -> str | None:
        if not link.is_symlink():
            return None
        target = os.readlink(link)
        prefix = "generations/"
        if not target.startswith(prefix):
            return None
        ver = target[len(prefix) :].strip("/")
        if VER_RE.fullmatch(ver):
            return ver
        return None

    def _selected_from_links(
        self,
        *,
        current_link: Path,
        previous_link: Path | None,
        generations_dir: Path,
        binary_name: str,
        component: str,
    ) -> dict:
        return self._generation_info(
            version=self._read_link_version(current_link),
            previous=self._read_link_version(previous_link) if previous_link is not None else None,
            generations_dir=generations_dir, binary_name=binary_name, component=component,
        )

    def _generation_info(
        self, *, version: str | None, previous: str | None,
        generations_dir: Path, binary_name: str, component: str,
    ) -> dict:
        ver, prev = version, previous
        if ver is None:
            return {
                "version": None,
                "previous_version": prev,
                "binary_path": None,
                "binary_sha256": None,
                "valid": False,
            }
        generation_dir = generations_dir / ver
        bin_path = generation_dir / binary_name
        release_component = RELEASE_COMPONENTS.get(component, binary_name)
        if (binary_name != release_component or generations_dir.is_symlink() or
                generation_dir.is_symlink() or bin_path.is_symlink() or
                not bin_path.is_file() or not os.access(bin_path, os.X_OK)):
            return {
                "version": ver,
                "previous_version": prev,
                "binary_path": str(bin_path),
                "binary_sha256": None,
                "valid": False,
            }
        # Release signatures cover archive digests; older/direct manifests may
        # instead cover the binary itself. install.sh admits the signed archive
        # before extraction and keeps its release manifest, not the archive.
        signed_dir = generation_dir / "signed"
        sums_path = signed_dir / "SHA256SUMS"
        sig_path = signed_dir / "SHA256SUMS.sig"
        key_path = signed_dir / "release-public-key.pem"
        try:
            if (generation_dir.is_symlink() or signed_dir.is_symlink() or
                    not sums_path.is_file() or sums_path.is_symlink() or
                    not sig_path.is_file() or sig_path.is_symlink() or
                    sig_path.stat().st_size != 64):
                raise ValueError("missing or invalid release signature")
            trusted_key = self.home / ".local/lib/codex/wrapper/release-public-key.pem"
            if not trusted_key.is_file() or trusted_key.is_symlink():
                raise ValueError("missing installed trusted key")
            # Retired verifiers are retained by the installer before rotation.
            # A generation's own key is never a trust root. Legacy generations
            # without a key copy may verify against any installed verifier.
            retired_dir = trusted_key.parent / "retired-keys"
            candidates = [trusted_key]
            if retired_dir.is_dir() and not retired_dir.is_symlink():
                candidates.extend(
                    key for key in sorted(retired_dir.glob("*.pem"))
                    if re.fullmatch(r"[0-9a-f]{64}", key.stem)
                    and key.is_file() and not key.is_symlink()
                    and sha256_file(key) == key.stem
                )
            if key_path.exists() or key_path.is_symlink():
                if not key_path.is_file() or key_path.is_symlink():
                    raise ValueError("invalid generation public key")
                supplied_key = key_path.read_bytes()
                candidates = [key for key in candidates if key.read_bytes() == supplied_key]
            for key in candidates:
                verified = subprocess.run(
                    ["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(key),
                     "-rawin", "-in", str(sums_path), "-sigfile", str(sig_path)],
                    capture_output=True, timeout=10,
                )
                if verified.returncode == 0:
                    break
            else:
                raise ValueError("generation signature verification failed")

            entry = f"bin/{binary_name}"
            release_checksums = read_checksums(sums_path)
            expected = release_checksums.get(entry)
            headers = sums_path.read_text(encoding="ascii").splitlines()[:4]
            if expected is None or (headers and headers[0].startswith("# format=")):
                if (len(headers) != 4 or headers[:2] != [
                        "# format=codex-release-sums-v1", f"# component={release_component}"] or
                        not re.fullmatch(r"# sequence=[1-9][0-9]{0,9}", headers[2]) or
                        int(headers[2].removeprefix("# sequence=")) > 2147483647 or
                        not re.fullmatch(r"# commit=[0-9a-f]{40}", headers[3])):
                    raise ValueError("invalid release manifest headers")
            if expected is None:
                archive = f"{release_component}-aarch64-linux-android.tar.gz"
                archive_digest = release_checksums.get(archive)
                if archive_digest is None:
                    raise ValueError("missing signed archive checksum")
                # The archive sidecar is optional in install.sh; if retained,
                # it must agree with the authenticated release manifest.
                sidecar = signed_dir / f"{archive}.sha256"
                if sidecar.exists() or sidecar.is_symlink():
                    if read_checksums(sidecar).get(archive) != archive_digest:
                        raise ValueError("archive checksum sidecar mismatch")
                # This is a local integrity check of the installer-admitted
                # extraction, not a new cryptographic verification of its origin.
                expected = read_checksums(generation_dir / "SHA256SUMS").get(entry)
                packaged_bin = generation_dir / entry
                if (expected is None or packaged_bin.parent.is_symlink() or
                        packaged_bin.is_symlink() or not packaged_bin.is_file() or
                        sha256_file(packaged_bin) != expected):
                    raise ValueError("extracted bundle checksum mismatch")
            actual = sha256_file(bin_path)
            if actual != expected:
                raise ValueError("binary checksum mismatch")
        except (OSError, UnicodeError, ValueError, subprocess.TimeoutExpired):
            return {
                "version": ver, "previous_version": prev, "binary_path": str(bin_path),
                "binary_sha256": None, "valid": False,
            }
        return {
            "version": ver,
            "previous_version": prev,
            "binary_path": str(bin_path.resolve()),
            "binary_sha256": actual,
            "valid": True,
        }

    def selected_generation(self) -> dict:
        return self._selected_from_links(
            current_link=self.current_link,
            previous_link=self.previous_link,
            generations_dir=self.generations_dir,
            binary_name=self.binary_name,
            component=self.component,
        )

    def _component_generation(self, version: str) -> dict:
        return self._generation_info(
            version=validate_token(version, VER_RE, "version"), previous=None,
            generations_dir=self.generations_dir, binary_name=self.binary_name,
            component=self.component,
        )

    def selected_runner_generation(self) -> dict | None:
        if not self.has_runner:
            return None
        assert self.runner_current_link is not None
        assert self.runner_generations_dir is not None
        assert self.runner_binary is not None
        assert self.runner_component is not None
        return self._selected_from_links(
            current_link=self.runner_current_link,
            previous_link=self.runner_root / "previous" if self.runner_root else None,
            generations_dir=self.runner_generations_dir,
            binary_name=self.runner_binary,
            component=self.runner_component,
        )

    def _write_active_marker(
        self,
        *,
        running: dict,
        selected: dict,
        runner_selected: dict | None,
    ) -> None:
        payload = {
            "component": self.component,
            "component_version": selected.get("version"),
            "component_sha256": selected.get("binary_sha256"),
            "runner_component": self.runner_component,
            "runner_version": runner_selected.get("version") if runner_selected else None,
            "runner_exe": runner_selected.get("binary_path") if runner_selected else None,
            "pid": running.get("pid"),
            "start_ticks": running.get("start_ticks"),
            "recorded_at_ns": time.time_ns(),
        }
        atomic_write_private(
            self.active_marker_path,
            json.dumps(payload, indent=2, sort_keys=True) + "\n",
            mode=0o600,
        )

    def is_enrolled(self) -> bool:
        run_script = self.service_dir / "run"
        return run_script.is_file() and not run_script.is_symlink()

    def _inspect_running(self) -> dict:
        if not self.is_enrolled():
            return {"enrolled": False, "state": "not_enrolled", "pid": None, "exe": None, "start_ticks": None}
        if self._proc_reader is not None:
            return self._proc_reader(self.service_dir)
        try:
            res = subprocess.run(
                [self.sv_bin, "status", str(self.service_dir)],
                capture_output=True,
                text=True,
                timeout=10,
            )
        except subprocess.TimeoutExpired:
            return {
                "enrolled": True,
                "state": "error",
                "pid": None,
                "exe": None,
                "start_ticks": None,
                "raw": "sv status timed out",
            }
        line = (res.stdout or res.stderr).strip()
        # The logger has its own PID even when the service has stopped.
        service_status = line.split("; ", 1)[0]
        m = re.match(r"^(run|finish):\s+.*?\(pid\s+(\d+)\)", service_status)
        if not m:
            state = "stopped" if line.startswith("down:") else "error"
            return {"enrolled": True, "state": state, "pid": None, "exe": None, "start_ticks": None, "raw": line}
        sv_state, pid = m.group(1), int(m.group(2))
        exe = proc_exe_path(pid)
        ticks = proc_start_ticks(pid)
        return {
            "enrolled": True,
            "state": "running" if sv_state == "run" else sv_state,
            "pid": pid,
            "exe": exe,
            "start_ticks": ticks,
            "raw": line,
        }

    def _version_from_exe(
        self,
        exe: str | None,
        generations_dir: Path | None = None,
        binary_name: str | None = None,
    ) -> str | None:
        if not exe:
            return None
        gdir = generations_dir or self.generations_dir
        bname = binary_name or self.binary_name
        cleaned = exe[:-10] if exe.endswith(" (deleted)") else exe
        try:
            rel = Path(cleaned).relative_to(gdir.resolve())
            if len(rel.parts) == 2 and rel.parts[1] == bname:
                return rel.parts[0]
        except ValueError:
            pass
        return None

    def _companions(self) -> list["ServiceSupervisor"]:
        return [
            ServiceSupervisor(
                component=self.component,
                binary_name=self.binary_name,
                service_name=cname,
                home=self.home,
                prefix=self.prefix,
                sv_bin=self.sv_bin,
                proc_reader=self._proc_reader,
            )
            for cname in self.companion_services
        ]

    def _child_exes_for(self, running: dict) -> list[str]:
        pid = running.get("pid")
        ticks = running.get("start_ticks")
        if not isinstance(pid, int) or not isinstance(ticks, int):
            return []
        if self._proc_reader is not None:
            return list(running.get("child_exes", []))
        return proc_child_exes(pid, ticks)

    def _runner_matches_selected_component(
        self,
        *,
        running: dict,
        selected: dict,
        child_exes: list[str] | None = None,
    ) -> tuple[bool, str | None]:
        """Confirm that the runner's live MCP child uses the selected image."""
        exes = child_exes if child_exes is not None else self._child_exes_for(running)
        if "" in exes:
            return False, None
        components = [(exe, ver) for exe in exes if (ver := self._version_from_exe(exe)) is not None]
        versions = {ver for _, ver in components}
        version = next(iter(versions)) if len(versions) == 1 else None
        return (
            bool(selected.get("valid")) and len(components) == 1
            and components[0][0] == selected.get("binary_path")
        ), version

    def status(self, *, readiness_cmd: list[str] | str | None = None) -> dict:
        norm_readiness = normalize_command(readiness_cmd)
        selected = self.selected_generation()
        runner_selected = self.selected_runner_generation()
        running = self._inspect_running()
        running_deleted = bool(running.get("exe") and str(running["exe"]).endswith(" (deleted)"))

        if self.has_runner:
            assert self.runner_generations_dir is not None
            assert self.runner_binary is not None
            running_runner_ver = self._version_from_exe(
                running.get("exe"),
                self.runner_generations_dir,
                self.runner_binary,
            )
            child_exes = self._child_exes_for(running)
            if any(
                exe.endswith(" (deleted)") and self._version_from_exe(exe) is not None
                for exe in child_exes
            ):
                running_deleted = True
            runner_comp_matches, running_ver = self._runner_matches_selected_component(
                running=running,
                selected=selected,
                child_exes=child_exes,
            )
        else:
            running_runner_ver = None
            running_ver = self._version_from_exe(running.get("exe"))
            runner_comp_matches = True

        readiness_ok = True
        readiness_error = None
        if norm_readiness and running.get("state") == "running":
            try:
                r = subprocess.run(norm_readiness, capture_output=True, text=True, timeout=10)
                if r.returncode != 0:
                    readiness_ok = False
                    readiness_error = (r.stderr or r.stdout).strip()[:200] or f"exit {r.returncode}"
            except subprocess.TimeoutExpired:
                readiness_ok = False
                readiness_error = "readiness command timed out"
            except OSError as err:
                readiness_ok = False
                readiness_error = str(err)[:200]

        expected_exe = (
            runner_selected["binary_path"]
            if self.has_runner and runner_selected is not None
            else selected["binary_path"]
        )
        all_selected_valid = selected["valid"] and (
            not self.has_runner or (runner_selected is not None and runner_selected["valid"])
        )

        if not running.get("enrolled"):
            activation_state = "not_enrolled"
        elif not all_selected_valid:
            activation_state = "error"
        elif running.get("state") != "running":
            activation_state = "stopped" if running.get("state") == "stopped" else "error"
        elif not readiness_ok:
            activation_state = "error"
        elif running_deleted or running.get("exe") != expected_exe or not runner_comp_matches:
            activation_state = "pending_activation"
        else:
            activation_state = "active"

        out = {
            "component": self.component,
            "service": self.service_name,
            "selected_generation": selected,
            "running_generation": {
                "version": running_ver,
                "state": running.get("state"),
                "pid": running.get("pid"),
                "start_ticks": running.get("start_ticks"),
                "exe": running.get("exe"),
                "deleted_image": running_deleted,
                "readiness_ok": readiness_ok,
                "readiness_error": readiness_error,
            },
            "activation_state": activation_state,
        }
        if self.has_runner:
            out["runner_component"] = self.runner_component
            out["selected_runner_generation"] = runner_selected
            out["running_generation"]["runner_version"] = running_runner_ver
        if self.companion_services:
            comp_statuses = [c.status() for c in self._companions()]
            out["companions"] = comp_statuses
            if activation_state == "active":
                for cst in comp_statuses:
                    cstate = cst["activation_state"]
                    if cstate in ("pending_activation", "stopped", "error"):
                        activation_state = cstate
                        out["activation_state"] = cstate
                        break
        return out

    def enroll(self, *, exec_args: list[str], extra_env: dict[str, str] | None = None) -> dict:
        selected = self.selected_generation()
        if not selected["valid"]:
            raise SupervisionError(
                f"cannot enroll '{self.service_name}': no valid selected generation under {self.current_link}"
            )
        if self.has_runner:
            runner_selected = self.selected_runner_generation()
            if runner_selected is None or not runner_selected["valid"]:
                raise SupervisionError(
                    f"cannot enroll '{self.service_name}': no valid runner generation under {self.runner_current_link}"
                )
            assert self.runner_current_link is not None
            assert self.runner_binary is not None
            stable_bin = self.runner_current_link / self.runner_binary
        else:
            stable_bin = self.current_link / self.binary_name

        ensure_private_dir(self.service_dir)
        log_dir = self.service_dir / "log"
        ensure_private_dir(log_dir)
        log_out = self.state_dir / "log"
        ensure_private_dir(log_out)

        env_lines = []
        for k, v in sorted((extra_env or {}).items()):
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", k):
                raise SupervisionError(f"invalid env key: {k!r}")
            env_lines.append(f"export {k}={shlex.quote(v)}")
        env_block = ("\n".join(env_lines) + "\n") if env_lines else ""
        arg_str = " ".join(shlex.quote(a) for a in exec_args)
        exec_line = f"exec {shlex.quote(str(stable_bin))}" + (f" {arg_str}" if arg_str else "")

        run_script = (
            "#!/bin/sh\n"
            "set -eu\n"
            "exec 2>&1\n"
            f"{env_block}"
            f"{exec_line}\n"
        )
        log_script = f"#!/bin/sh\nexec svlogd -tt {shlex.quote(str(log_out))}\n"

        run_path = self.service_dir / "run"
        log_run_path = log_dir / "run"
        atomic_write_private(run_path, run_script, mode=0o700)
        atomic_write_private(log_run_path, log_script, mode=0o700)
        return {"enrolled": True, "service_dir": str(self.service_dir), "stable_bin": str(stable_bin)}

    @contextmanager
    def _activation_lock(self):
        ensure_private_dir(self.comp_root)
        path = self.comp_root / ".activation.lock"
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as err:
            os.close(fd)
            raise SupervisionError(f"another activation is running for '{self.component}'") from err
        except BaseException:
            os.close(fd)
            raise
        try:
            yield
        finally:
            os.close(fd)

    def activate(
        self,
        version: str,
        *,
        busy_check_cmd: list[str] | str | None = None,
        readiness_cmd: list[str] | str | None = None,
        restart_timeout: float = 15.0,
    ) -> dict:
        with self._activation_lock():
            return self._activate(version, normalize_command(busy_check_cmd),
                                  normalize_command(readiness_cmd), restart_timeout)

    def _activate(self, version: str, busy: list[str] | None,
                  readiness: list[str] | None, timeout: float) -> dict:
        validate_token(version, VER_RE, "version")
        candidate = self._component_generation(version)
        if not candidate["valid"]:
            raise SupervisionError(f"generation missing or invalid: {candidate['binary_path']}")
        target_exe = candidate["binary_path"]
        runner = self.selected_runner_generation()
        if self.has_runner and (runner is None or not runner["valid"]):
            raise SupervisionError(f"runner generation missing or invalid under {self.runner_current_link}")

        if self.component == "mcp/termux-orch":
            orch_relay = self.prefix / "var/service/termux-orch-relay/run"
            orch_tunnel = self.prefix / "var/service/termux-orch-tunnel/run"
            if orch_relay.is_file() and orch_tunnel.is_file():
                if self.service_name != "termux-orch-tunnel" or not self.has_runner:
                    raise SupervisionError("activate the shared Orch generation from its runner-backed tunnel")
                if "termux-orch-relay" not in self.companion_services:
                    raise SupervisionError("use --companion-service termux-orch-relay for the shared Orch generation")

        enrolled = [s for s in [self, *self._companions()] if s.is_enrolled()]
        running = []
        for service in enrolled:
            observation = service._wait_past_finish(timeout)
            if observation.get("state") == "stopped":
                continue
            if observation.get("state") != "running" or not observation.get("start_ticks"):
                raise SupervisionError(f"cannot verify state of '{service.service_name}'")
            if service.has_runner:
                observed_runner = service._version_from_exe(
                    observation.get("exe"), service.runner_generations_dir, service.runner_binary,
                )
                if observed_runner is None:
                    raise SupervisionError(f"'{service.service_name}' is not running this runner")
            elif service._version_from_exe(observation.get("exe")) is None:
                raise SupervisionError(f"'{service.service_name}' is not running this component")
            if service.has_runner:
                observation["publishes_health_url"] = service._health_file_stamp() is not None
                observation["component_children"] = service._child_exes_for(observation)
            running.append((service, observation))

        current = self._read_link_version(self.current_link)
        previous = self._read_link_version(self.previous_link)
        selection_changed = current != version
        expected_primary = runner["binary_path"] if runner else target_exe
        if not selection_changed and all(
            info["exe"] == (expected_primary if svc is self else target_exe)
            and (not svc.has_runner or svc._runner_matches_selected_component(
                running=info, selected=candidate, child_exes=info.get("component_children"),
            )[0])
            for svc, info in running
        ):
            return self.status(readiness_cmd=readiness)

        runner_changed = any(
            service.has_runner and info["exe"] != expected_primary
            for service, info in running
        )
        if running and (selection_changed or runner_changed) and busy is None:
            status = self.status(readiness_cmd=readiness)
            status["activation_state"] = "pending_activation"
            status["deferred_reason"] = "busy check required for a running service"
            return status

        if running and busy is not None:
            try:
                checked = subprocess.run(busy, capture_output=True, text=True, timeout=10)
                reason = None if checked.returncode == 0 else (checked.stderr or checked.stdout).strip()[:200] or "busy"
            except subprocess.TimeoutExpired:
                reason = "busy check timed out"
            except OSError as err:
                reason = str(err)
            if reason is not None:
                # A rejected/unknown busy check is a hard fence: never publish
                # the candidate selection while an old process may still run.
                status = self.status(readiness_cmd=readiness)
                status["activation_state"] = "pending_activation"
                status["deferred_reason"] = reason
                return status

        if not running:
            if selection_changed:
                self._select(version, current)
            return self.status(readiness_cmd=readiness)

        # A runner-only reload must not roll back an unrelated MCP generation.
        # On a pending component activation, use the observed old component,
        # not just the historical previous link, as the rollback target.
        observed_versions = set()
        for service, info in running:
            if service.has_runner:
                _, observed = service._runner_matches_selected_component(
                    running=info, selected=candidate, child_exes=info.get("component_children"),
                )
            else:
                observed = service._version_from_exe(info.get("exe"))
            if observed is not None:
                observed_versions.add(observed)
        if len(observed_versions) > 1:
            raise SupervisionError("cannot restart with mixed running component generations")
        prior = current if selection_changed else next(iter(observed_versions), current)
        prior_previous = previous if prior == current and previous != prior else None
        if prior is None or not self._component_generation(prior)["valid"]:
            raise SupervisionError("cannot restart without a verified rollback generation")

        stopping = []
        stopped = False
        try:
            for service, _ in running:
                stopping.append(service)
                service._stop_service(timeout=timeout)
            stopped = True
            if selection_changed:
                self._select(version, current)
            self._start_group(running, target_exe, runner, readiness, timeout)
        except (SupervisionError, OSError) as err:
            try:
                if stopped:
                    failures = []
                    for service, _ in running:
                        try:
                            service._stop_service(timeout=timeout, force=True)
                        except (SupervisionError, OSError) as stop:
                            failures.append(f"{service.service_name}: {stop}")
                    if failures:
                        raise SupervisionError("could not drain attempted generation: " + ", ".join(failures))
                    if not self._component_generation(prior)["valid"]:
                        raise SupervisionError("rollback generation is no longer valid")
                    self._select(prior, prior_previous)
                    self._start_group(running, str(self.generation_binary(prior).resolve()),
                                      runner, readiness, timeout)
                    if runner_changed:
                        raise SupervisionError("shared runner selection changed; original runner not restored")
                else:
                    # No selection changed. Restart only the services whose
                    # stop was requested; a failed stop may have killed one.
                    for service in reversed(stopping):
                        service._run_sv("up", timeout)
            except (SupervisionError, OSError) as recovery:
                raise SupervisionError(f"activation failed: {err}; recovery incomplete: {recovery}") from err
            if stopped:
                outcome = "rolled back" if prior != version else "original component restored"
                raise SupervisionError(f"activation of '{version}' failed ({outcome}): {err}") from err
            raise SupervisionError(f"activation stopped before changing selection: {err}") from err

        return self.status(readiness_cmd=readiness)

    def _select(self, current: str, previous: str | None) -> None:
        if previous is not None and previous != current:
            atomic_symlink(self.previous_link, f"generations/{previous}")
        elif self.previous_link.is_symlink():
            self.previous_link.unlink()
            fsync_dir(self.comp_root)
        atomic_symlink(self.current_link, f"generations/{current}")

    def _start_group(self, running, component_exe, runner, readiness, timeout) -> None:
        # Start the relay before reopening the tunnel to incoming work.
        for service, before in reversed(running):
            expected = runner["binary_path"] if service.has_runner else component_exe
            info = service._restart_and_verify(
                expected_bin=expected, old_pid=before.get("pid"), old_ticks=before.get("start_ticks"),
                readiness_cmd=readiness if service is self else None, timeout=timeout, sv_action="start",
                require_health_url=before.get("publishes_health_url", False),
            )
            if service.has_runner:
                service._write_active_marker(running=info, selected=self.selected_generation(),
                                             runner_selected=runner)

    def rollback(
        self,
        *,
        busy_check_cmd: list[str] | str | None = None,
        readiness_cmd: list[str] | str | None = None,
        restart_timeout: float = 15.0,
    ) -> dict:
        with self._activation_lock():
            previous = self._read_link_version(self.previous_link)
            if not previous:
                raise SupervisionError(f"no previous generation recorded at {self.previous_link}")
            return self._activate(previous, normalize_command(busy_check_cmd),
                                  normalize_command(readiness_cmd), restart_timeout)

    def _wait_past_finish(self, timeout: float) -> dict:
        deadline = time.monotonic() + timeout
        while True:
            observation = self._inspect_running()
            if observation.get("state") != "finish" or time.monotonic() >= deadline:
                return observation
            time.sleep(0.05)

    def _stop_service(self, *, timeout: float, force: bool = False) -> None:
        before = self._inspect_running()
        children = []
        if self.has_runner and self._proc_reader is None and before.get("state") == "running":
            children = proc_children(before["pid"], before["start_ticks"])
            if children is None:
                raise SupervisionError(f"cannot inspect children of '{self.service_name}'")
        def wait_stopped() -> bool:
            deadline = time.monotonic() + timeout
            while True:
                if self._inspect_running().get("state") == "stopped" and not any(
                    proc_start_ticks(pid) == ticks for pid, ticks, _ in children
                ):
                    return True
                if time.monotonic() >= deadline:
                    return False
                time.sleep(0.05)

        self._run_sv("down", timeout)
        if wait_stopped():
            return
        if force and self._inspect_running().get("state") == "running":
            self._run_sv("kill", timeout)
            if wait_stopped():
                return
        raise SupervisionError(f"cannot confirm '{self.service_name}' and its children stopped")

    def _health_file_stamp(self) -> tuple[int, int] | None:
        try:
            observed = (self.state_dir / "health.url").stat()
            return observed.st_ino, observed.st_mtime_ns
        except FileNotFoundError:
            return None

    def _tunnel_ready(self, before: tuple[int, int] | None, timeout: float, *, required: bool = False) -> bool:
        try:
            after = self._health_file_stamp()
            if after is None:
                return before is None and not required
            if after == before:
                return False
            url = (self.state_dir / "health.url").read_text().strip()
            match = re.fullmatch(r"http://127\.0\.0\.1:(\d{1,5})", url)
            if not match or not 0 < int(match[1]) <= 65535:
                return False
            conn = http.client.HTTPConnection("127.0.0.1", int(match[1]), timeout=timeout)
            try:
                conn.request("GET", "/readyz")
                return conn.getresponse().status == 200
            finally:
                conn.close()
        except (OSError, http.client.HTTPException, ValueError):
            return False

    def _run_sv(self, action: str, timeout: float) -> None:
        try:
            result = subprocess.run(
                [self.sv_bin, action, str(self.service_dir)],
                capture_output=True, text=True, timeout=timeout,
            )
        except (subprocess.TimeoutExpired, OSError) as err:
            raise SupervisionError(f"sv {action} failed for '{self.service_name}': {err}") from err
        if result.returncode != 0:
            reason = (result.stderr or result.stdout).strip()[:200]
            raise SupervisionError(f"sv {action} failed ({result.returncode}): {reason}")

    def _restart_and_verify(
        self,
        *,
        expected_bin: str,
        old_pid: int | None,
        old_ticks: int | None,
        readiness_cmd: list[str] | None,
        timeout: float,
        sv_action: str = "restart",
        require_health_url: bool = False,
    ) -> dict:
        old_health = self._health_file_stamp() if self.has_runner else None
        self._run_sv(sv_action, timeout)
        deadline = time.monotonic() + timeout
        selected_component = self.selected_generation() if self.has_runner else None
        while time.monotonic() < deadline:
            cur = self._inspect_running()
            changed = (
                cur.get("pid") is not None
                and (cur.get("pid") != old_pid or cur.get("start_ticks") != old_ticks)
            )
            if cur.get("state") == "running" and changed and cur.get("start_ticks") and cur.get("exe") == expected_bin:
                if selected_component is not None and not self._runner_matches_selected_component(
                    running=cur, selected=selected_component
                )[0]:
                    time.sleep(0.1)
                    continue
                remaining = deadline - time.monotonic()
                if self.has_runner and not self._tunnel_ready(
                    old_health, max(0.05, min(1.0, remaining)), required=require_health_url
                ):
                    time.sleep(0.1)
                    continue
                remaining = max(0.2, min(5.0, deadline - time.monotonic()))
                try:
                    chk = readiness_cmd is None or subprocess.run(
                        readiness_cmd,
                        capture_output=True,
                        text=True,
                        timeout=remaining,
                    ).returncode == 0
                    if chk:
                        current = self._inspect_running()
                        if all(cur[key] == current[key] for key in ("state", "pid", "start_ticks", "exe")):
                            if selected_component is None or self._runner_matches_selected_component(
                                running=current, selected=selected_component,
                            )[0]:
                                return cur
                except (subprocess.TimeoutExpired, OSError):
                    pass
            time.sleep(0.1)
        raise SupervisionError(
            f"timed out waiting for '{self.service_name}' to run verified image {expected_bin}"
        )


def main(argv: list[str] | None = None) -> int:
    raw_argv = list(sys.argv[1:] if argv is None else argv)
    norm_argv = preprocess_cli_argv(raw_argv)

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", required=True)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--service", required=True)
    parser.add_argument(
        "--runner-component",
        default=None,
        help="Optional supervisor runner component (e.g. transport/tunnel-client for MCP tunnels)",
    )
    parser.add_argument(
        "--runner-binary",
        default=None,
        help="Optional supervisor runner binary name (e.g. tunnel-client)",
    )
    parser.add_argument(
        "--companion-service",
        action="append",
        default=[],
        help="Additional service name sharing the same component generation (e.g. termux-orch-relay)",
    )
    parser.add_argument("--home", type=Path, default=None)
    parser.add_argument("--prefix", type=Path, default=None)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_status = sub.add_parser("status", help="report selected, running, and activation state")
    p_status.add_argument("--readiness-cmd", default=None)

    p_enroll = sub.add_parser("enroll", help="explicitly opt-in and create runit service scripts")
    p_enroll.add_argument("--arg", action="append", default=[])
    p_enroll.add_argument("--env", action="append", default=[], help="KEY=VALUE")

    p_act = sub.add_parser("activate", help="select generation and safely rebind enrolled service")
    p_act.add_argument("version")
    p_act.add_argument("--busy-check-cmd", default=None)
    p_act.add_argument("--readiness-cmd", default=None)
    p_act.add_argument("--timeout", type=float, default=15.0)

    p_rb = sub.add_parser("rollback", help="roll back to previous generation")
    p_rb.add_argument("--busy-check-cmd", default=None)
    p_rb.add_argument("--readiness-cmd", default=None)
    p_rb.add_argument("--timeout", type=float, default=15.0)

    args = parser.parse_args(norm_argv)
    try:
        sup = ServiceSupervisor(
            component=args.component,
            binary_name=args.binary,
            service_name=args.service,
            runner_component=args.runner_component,
            runner_binary=args.runner_binary,
            companion_services=args.companion_service,
            home=args.home,
            prefix=args.prefix,
        )
        if args.cmd == "status":
            res = sup.status(readiness_cmd=args.readiness_cmd)
        elif args.cmd == "enroll":
            env_map = {}
            for item in args.env:
                if "=" not in item:
                    raise SupervisionError(f"--env must be KEY=VALUE: {item!r}")
                k, v = item.split("=", 1)
                env_map[k] = v
            res = sup.enroll(exec_args=args.arg, extra_env=env_map)
        elif args.cmd == "activate":
            res = sup.activate(
                args.version,
                busy_check_cmd=args.busy_check_cmd,
                readiness_cmd=args.readiness_cmd,
                restart_timeout=args.timeout,
            )
        elif args.cmd == "rollback":
            res = sup.rollback(
                busy_check_cmd=args.busy_check_cmd,
                readiness_cmd=args.readiness_cmd,
                restart_timeout=args.timeout,
            )
        else:
            raise SupervisionError(f"unknown command {args.cmd}")
        print(json.dumps(res, indent=2, sort_keys=True))
        return 0
    except (SupervisionError, OSError, ValueError) as err:
        print(f"runit_service: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
