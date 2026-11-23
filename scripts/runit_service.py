#!/usr/bin/env python3
"""Opt-in runit 3-state generation supervision for Stage 2 standalone services.

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
- Safe activation checks optional busy guard first (deferring to `pending_activation`
  rather than force-killing active work), atomically updates `previous`/`current`,
  restarts enrolled services via `sv restart`, verifies new PID + `/proc/<pid>/exe`
  + optional readiness command, and rolls back to `previous` on failure or timeout.
"""
import argparse
import hashlib
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

KNOWN_CLI_FLAGS = frozenset(
    {
        "--component",
        "--binary",
        "--service",
        "--runner-component",
        "--runner-binary",
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


def proc_ctime_ns(pid: int) -> int | None:
    try:
        return os.stat(f"/proc/{pid}").st_ctime_ns
    except OSError:
        return None


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
    ) -> dict:
        ver = self._read_link_version(current_link)
        prev = self._read_link_version(previous_link) if previous_link is not None else None
        if ver is None:
            return {
                "version": None,
                "previous_version": prev,
                "binary_path": None,
                "binary_sha256": None,
                "valid": False,
            }
        bin_path = generations_dir / ver / binary_name
        if bin_path.is_symlink() or not bin_path.is_file() or not os.access(bin_path, os.X_OK):
            return {
                "version": ver,
                "previous_version": prev,
                "binary_path": str(bin_path),
                "binary_sha256": None,
                "valid": False,
            }
        return {
            "version": ver,
            "previous_version": prev,
            "binary_path": str(bin_path.resolve()),
            "binary_sha256": sha256_file(bin_path),
            "valid": True,
        }

    def selected_generation(self) -> dict:
        return self._selected_from_links(
            current_link=self.current_link,
            previous_link=self.previous_link,
            generations_dir=self.generations_dir,
            binary_name=self.binary_name,
        )

    def selected_runner_generation(self) -> dict | None:
        if not self.has_runner:
            return None
        assert self.runner_current_link is not None
        assert self.runner_generations_dir is not None
        assert self.runner_binary is not None
        return self._selected_from_links(
            current_link=self.runner_current_link,
            previous_link=self.runner_root / "previous" if self.runner_root else None,
            generations_dir=self.runner_generations_dir,
            binary_name=self.runner_binary,
        )

    def _read_active_marker(self) -> dict | None:
        try:
            if self.active_marker_path.is_symlink() or not self.active_marker_path.is_file():
                return None
            data = json.loads(self.active_marker_path.read_text(encoding="utf-8"))
            return data if isinstance(data, dict) else None
        except (OSError, ValueError):
            return None

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
        m = re.match(r"^(run|down|finish):\s+.*?\(pid\s+(\d+)\)", line)
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

    def _runner_matches_selected_component(
        self,
        *,
        running: dict,
        selected: dict,
    ) -> tuple[bool, str | None]:
        """For runner-wrapped services (e.g. tunnel-client hosting an MCP binary),
        determine whether the currently running runner PID was started under the
        currently selected component generation.
        """
        pid = running.get("pid")
        ticks = running.get("start_ticks")
        marker = self._read_active_marker()
        if marker is not None and marker.get("pid") == pid and marker.get("start_ticks") == ticks:
            recorded_ver = marker.get("component_version")
            matches = (
                recorded_ver == selected.get("version")
                and marker.get("component_sha256") == selected.get("binary_sha256")
            )
            return matches, recorded_ver

        # If the service was restarted externally (e.g. after reboot or crash), compare the
        # live process start time against the `current` symlink mtime when `/proc/<pid>` is readable.
        if isinstance(pid, int) and self._proc_reader is None:
            p_ctime = proc_ctime_ns(pid)
            try:
                link_mtime = self.current_link.lstat().st_mtime_ns
            except OSError:
                link_mtime = None
            if p_ctime is not None and link_mtime is not None and p_ctime >= link_mtime:
                return True, selected.get("version")
        return False, marker.get("component_version") if marker else None

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
            runner_comp_matches, running_ver = self._runner_matches_selected_component(
                running=running,
                selected=selected,
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

    def activate(
        self,
        version: str,
        *,
        busy_check_cmd: list[str] | str | None = None,
        readiness_cmd: list[str] | str | None = None,
        restart_timeout: float = 15.0,
    ) -> dict:
        validate_token(version, VER_RE, "version")
        norm_busy = normalize_command(busy_check_cmd)
        norm_readiness = normalize_command(readiness_cmd)
        target_bin = self.generation_binary(version)
        if target_bin.is_symlink() or not target_bin.is_file() or not os.access(target_bin, os.X_OK):
            raise SupervisionError(f"generation binary missing or non-executable: {target_bin}")

        runner_selected = self.selected_runner_generation()
        if self.has_runner and (runner_selected is None or not runner_selected["valid"]):
            raise SupervisionError(
                f"runner generation missing or invalid under {self.runner_current_link}"
            )

        prev_ver = self._read_link_version(self.current_link)
        old_previous = self._read_link_version(self.previous_link)
        resolved_target_bin = str(target_bin.resolve())
        expected_exe = (
            runner_selected["binary_path"]
            if self.has_runner and runner_selected is not None
            else resolved_target_bin
        )

        if prev_ver == version:
            # If enrolled and running a stale image or unactivated companion generation
            # (`pending_activation`), proceed to restart; otherwise it is already active/settled.
            if not self.is_enrolled():
                return self.status(readiness_cmd=norm_readiness)
            before = self._inspect_running()
            if before.get("state") != "running":
                return self.status(readiness_cmd=norm_readiness)
            sel_now = self.selected_generation()
            comp_ok = (
                self._runner_matches_selected_component(running=before, selected=sel_now)[0]
                if self.has_runner
                else True
            )
            if before.get("exe") == expected_exe and comp_ok:
                return self.status(readiness_cmd=norm_readiness)
            switched_links = False
        else:
            if prev_ver is not None:
                atomic_symlink(self.previous_link, f"generations/{prev_ver}")
            atomic_symlink(self.current_link, f"generations/{version}")
            switched_links = True
            # Installing/activating a generation NEVER auto-enrolls a new runit service.
            if not self.is_enrolled():
                return self.status(readiness_cmd=norm_readiness)
            before = self._inspect_running()
            if before.get("state") != "running":
                return self.status(readiness_cmd=norm_readiness)

        # If active work is in progress, defer restart safely and report pending_activation!
        if norm_busy is not None:
            try:
                busy_res = subprocess.run(norm_busy, capture_output=True, text=True, timeout=10)
            except subprocess.TimeoutExpired:
                st = self.status(readiness_cmd=norm_readiness)
                st["deferred_reason"] = "busy check timed out"
                return st
            if busy_res.returncode != 0:
                st = self.status(readiness_cmd=norm_readiness)
                st["deferred_reason"] = (busy_res.stderr or busy_res.stdout).strip()[:200] or "busy"
                return st

        try:
            running_after = self._restart_and_verify(
                expected_bin=expected_exe,
                old_pid=before.get("pid"),
                old_ticks=before.get("start_ticks"),
                readiness_cmd=norm_readiness,
                timeout=restart_timeout,
            )
            if self.has_runner:
                self._write_active_marker(
                    running=running_after,
                    selected=self.selected_generation(),
                    runner_selected=runner_selected,
                )
        except SupervisionError as err:
            # Roll back `current` and restore `previous` if a previous generation existed!
            rollback_ver = prev_ver if switched_links else old_previous
            if rollback_ver is not None:
                atomic_symlink(self.current_link, f"generations/{rollback_ver}")
                if switched_links:
                    if old_previous is not None:
                        atomic_symlink(self.previous_link, f"generations/{old_previous}")
                    elif self.previous_link.is_symlink():
                        self.previous_link.unlink()
                try:
                    subprocess.run(
                        [self.sv_bin, "restart", str(self.service_dir)],
                        capture_output=True,
                        timeout=max(5.0, restart_timeout),
                    )
                except (subprocess.TimeoutExpired, OSError):
                    pass
                if self.has_runner:
                    rb_running = self._inspect_running()
                    if rb_running.get("state") == "running":
                        self._write_active_marker(
                            running=rb_running,
                            selected=self.selected_generation(),
                            runner_selected=self.selected_runner_generation(),
                        )
            raise SupervisionError(f"activation of '{version}' failed (rolled back): {err}") from err

        return self.status(readiness_cmd=norm_readiness)

    def rollback(
        self,
        *,
        busy_check_cmd: list[str] | str | None = None,
        readiness_cmd: list[str] | str | None = None,
        restart_timeout: float = 15.0,
    ) -> dict:
        prev_ver = self._read_link_version(self.previous_link)
        if not prev_ver:
            raise SupervisionError(f"no previous generation recorded at {self.previous_link}")
        return self.activate(
            prev_ver,
            busy_check_cmd=busy_check_cmd,
            readiness_cmd=readiness_cmd,
            restart_timeout=restart_timeout,
        )

    def _restart_and_verify(
        self,
        *,
        expected_bin: str,
        old_pid: int | None,
        old_ticks: int | None,
        readiness_cmd: list[str] | None,
        timeout: float,
    ) -> dict:
        try:
            r = subprocess.run(
                [self.sv_bin, "restart", str(self.service_dir)],
                capture_output=True,
                text=True,
                timeout=timeout,
            )
        except subprocess.TimeoutExpired as err:
            raise SupervisionError(f"sv restart timed out after {timeout}s") from err
        except OSError as err:
            raise SupervisionError(f"sv restart failed to execute: {err}") from err

        if r.returncode != 0:
            raise SupervisionError(
                f"sv restart failed ({r.returncode}): {(r.stderr or r.stdout).strip()[:200]}"
            )
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            cur = self._inspect_running()
            changed = (
                cur.get("pid") is not None
                and (cur.get("pid") != old_pid or cur.get("start_ticks") != old_ticks)
            )
            if cur.get("state") == "running" and changed and cur.get("exe") == expected_bin:
                if readiness_cmd is None:
                    return cur
                remaining = max(0.2, min(5.0, deadline - time.monotonic()))
                try:
                    chk = subprocess.run(
                        readiness_cmd,
                        capture_output=True,
                        text=True,
                        timeout=remaining,
                    )
                    if chk.returncode == 0:
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
