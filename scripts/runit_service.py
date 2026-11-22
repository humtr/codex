#!/usr/bin/env python3
"""Opt-in runit 3-state generation supervision for Stage 2 standalone services.

Manages immutable component generations under:
  $HOME/.local/lib/<component>/{generations/<version>/<bin>, current, previous}

And opt-in Termux runit services under:
  $PREFIX/var/service/<service>/{run, log/run}

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
  + optional readiness command, and rolls back to `previous` on failure.
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


class ServiceSupervisor:
    def __init__(
        self,
        *,
        component: str,
        binary_name: str,
        service_name: str,
        home: Path | None = None,
        prefix: Path | None = None,
        sv_bin: str = "sv",
        proc_reader=None,
    ):
        self.component = validate_token(component, NAME_RE, "component")
        self.binary_name = validate_token(binary_name, NAME_RE, "binary_name")
        self.service_name = validate_token(service_name, NAME_RE, "service_name")
        self.home = home or Path.home()
        self.prefix = prefix or Path(os.environ.get("PREFIX", "/data/data/com.termux/files/usr"))
        self.sv_bin = sv_bin
        self._proc_reader = proc_reader

        self.comp_root = self.home / ".local/lib" / self.component
        self.generations_dir = self.comp_root / "generations"
        self.current_link = self.comp_root / "current"
        self.previous_link = self.comp_root / "previous"
        self.service_dir = self.prefix / "var/service" / self.service_name

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

    def selected_generation(self) -> dict:
        ver = self._read_link_version(self.current_link)
        prev = self._read_link_version(self.previous_link)
        if ver is None:
            return {
                "version": None,
                "previous_version": prev,
                "binary_path": None,
                "binary_sha256": None,
                "valid": False,
            }
        bin_path = self.generation_binary(ver)
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

    def is_enrolled(self) -> bool:
        run_script = self.service_dir / "run"
        return run_script.is_file() and not run_script.is_symlink()

    def _inspect_running(self) -> dict:
        if not self.is_enrolled():
            return {"enrolled": False, "state": "not_enrolled", "pid": None, "exe": None, "start_ticks": None}
        if self._proc_reader is not None:
            return self._proc_reader(self.service_dir)
        res = subprocess.run(
            [self.sv_bin, "status", str(self.service_dir)],
            capture_output=True,
            text=True,
        )
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

    def _version_from_exe(self, exe: str | None) -> str | None:
        if not exe:
            return None
        cleaned = exe[:-10] if exe.endswith(" (deleted)") else exe
        try:
            rel = Path(cleaned).relative_to(self.generations_dir.resolve())
            if len(rel.parts) == 2 and rel.parts[1] == self.binary_name:
                return rel.parts[0]
        except ValueError:
            pass
        return None

    def status(self, *, readiness_cmd: list[str] | None = None) -> dict:
        selected = self.selected_generation()
        running = self._inspect_running()
        running_ver = self._version_from_exe(running.get("exe"))
        running_deleted = bool(running.get("exe") and str(running["exe"]).endswith(" (deleted)"))

        readiness_ok = True
        readiness_error = None
        if readiness_cmd and running.get("state") == "running":
            r = subprocess.run(readiness_cmd, capture_output=True, text=True, timeout=10)
            if r.returncode != 0:
                readiness_ok = False
                readiness_error = (r.stderr or r.stdout).strip()[:200] or f"exit {r.returncode}"

        if not running.get("enrolled"):
            activation_state = "not_enrolled"
        elif not selected["valid"]:
            activation_state = "error"
        elif running.get("state") != "running":
            activation_state = "stopped" if running.get("state") == "stopped" else "error"
        elif not readiness_ok:
            activation_state = "error"
        elif running_deleted or running.get("exe") != selected["binary_path"]:
            activation_state = "pending_activation"
        else:
            activation_state = "active"

        return {
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

    def enroll(self, *, exec_args: list[str], extra_env: dict[str, str] | None = None) -> dict:
        selected = self.selected_generation()
        if not selected["valid"]:
            raise SupervisionError(
                f"cannot enroll '{self.service_name}': no valid selected generation under {self.current_link}"
            )
        ensure_private_dir(self.service_dir)
        log_dir = self.service_dir / "log"
        ensure_private_dir(log_dir)
        log_out = self.home / ".local/state" / self.service_name / "log"
        ensure_private_dir(log_out)

        stable_bin = self.current_link / self.binary_name
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
        for p, body in ((run_path, run_script), (log_run_path, log_script)):
            tmp = p.with_name(f".{p.name}.tmp.{os.getpid()}")
            tmp.write_text(body, encoding="utf-8")
            os.chmod(tmp, 0o700)
            os.replace(tmp, p)
        fsync_dir(self.service_dir)
        fsync_dir(log_dir)
        return {"enrolled": True, "service_dir": str(self.service_dir), "stable_bin": str(stable_bin)}

    def activate(
        self,
        version: str,
        *,
        busy_check_cmd: list[str] | None = None,
        readiness_cmd: list[str] | None = None,
        restart_timeout: float = 15.0,
    ) -> dict:
        validate_token(version, VER_RE, "version")
        target_bin = self.generation_binary(version)
        if target_bin.is_symlink() or not target_bin.is_file() or not os.access(target_bin, os.X_OK):
            raise SupervisionError(f"generation binary missing or non-executable: {target_bin}")

        prev_ver = self._read_link_version(self.current_link)
        old_previous = self._read_link_version(self.previous_link)
        resolved_target_bin = str(target_bin.resolve())

        if prev_ver == version:
            # If enrolled and running a stale image (pending_activation), proceed to restart;
            # otherwise it is already active/settled.
            if not self.is_enrolled():
                return self.status(readiness_cmd=readiness_cmd)
            before = self._inspect_running()
            if before.get("state") != "running" or before.get("exe") == resolved_target_bin:
                return self.status(readiness_cmd=readiness_cmd)
            switched_links = False
        else:
            if prev_ver is not None:
                atomic_symlink(self.previous_link, f"generations/{prev_ver}")
            atomic_symlink(self.current_link, f"generations/{version}")
            switched_links = True
            # Installing/activating a generation NEVER auto-enrolls a new runit service.
            if not self.is_enrolled():
                return self.status(readiness_cmd=readiness_cmd)
            before = self._inspect_running()
            if before.get("state") != "running":
                return self.status(readiness_cmd=readiness_cmd)

        # If active work is in progress, defer restart safely and report pending_activation!
        if busy_check_cmd is not None:
            busy_res = subprocess.run(busy_check_cmd, capture_output=True, text=True, timeout=10)
            if busy_res.returncode != 0:
                st = self.status(readiness_cmd=readiness_cmd)
                st["deferred_reason"] = (busy_res.stderr or busy_res.stdout).strip()[:200] or "busy"
                return st

        try:
            self._restart_and_verify(
                expected_bin=resolved_target_bin,
                old_pid=before.get("pid"),
                old_ticks=before.get("start_ticks"),
                readiness_cmd=readiness_cmd,
                timeout=restart_timeout,
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
                subprocess.run([self.sv_bin, "restart", str(self.service_dir)], capture_output=True)
            raise SupervisionError(f"activation of '{version}' failed (rolled back): {err}") from err

        return self.status(readiness_cmd=readiness_cmd)

    def rollback(
        self,
        *,
        busy_check_cmd: list[str] | None = None,
        readiness_cmd: list[str] | None = None,
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
    ) -> None:
        r = subprocess.run(
            [self.sv_bin, "restart", str(self.service_dir)],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
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
                    return
                chk = subprocess.run(readiness_cmd, capture_output=True, text=True, timeout=5)
                if chk.returncode == 0:
                    return
            time.sleep(0.1)
        raise SupervisionError(
            f"timed out waiting for '{self.service_name}' to run verified image {expected_bin}"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--component", required=True)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--service", required=True)
    parser.add_argument("--home", type=Path, default=None)
    parser.add_argument("--prefix", type=Path, default=None)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_status = sub.add_parser("status", help="report selected, running, and activation state")
    p_status.add_argument("--readiness-cmd", nargs="+", default=None)

    p_enroll = sub.add_parser("enroll", help="explicitly opt-in and create runit service scripts")
    p_enroll.add_argument("--arg", action="append", default=[])
    p_enroll.add_argument("--env", action="append", default=[], help="KEY=VALUE")

    p_act = sub.add_parser("activate", help="select generation and safely rebind enrolled service")
    p_act.add_argument("version")
    p_act.add_argument("--busy-check-cmd", nargs="+", default=None)
    p_act.add_argument("--readiness-cmd", nargs="+", default=None)
    p_act.add_argument("--timeout", type=float, default=15.0)

    p_rb = sub.add_parser("rollback", help="roll back to previous generation")
    p_rb.add_argument("--busy-check-cmd", nargs="+", default=None)
    p_rb.add_argument("--readiness-cmd", nargs="+", default=None)
    p_rb.add_argument("--timeout", type=float, default=15.0)

    args = parser.parse_args(argv)
    sup = ServiceSupervisor(
        component=args.component,
        binary_name=args.binary,
        service_name=args.service,
        home=args.home,
        prefix=args.prefix,
    )
    try:
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
    except (SupervisionError, OSError) as err:
        print(f"runit_service: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
