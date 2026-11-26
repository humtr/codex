#!/usr/bin/env python3
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from unittest import mock

from scripts.runit_service import (
    ServiceSupervisor,
    SupervisionError,
    main as cli_main,
    preprocess_cli_argv,
    proc_child_exes,
)


class RunitSupervisionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="codex-runit-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.home = self.root / "home"
        self.prefix = self.root / "usr"
        self.home.mkdir(mode=0o700)
        (self.prefix / "bin").mkdir(parents=True, mode=0o700)
        trusted_dir = self.home / ".local/lib/codex/wrapper"
        trusted_dir.mkdir(parents=True)
        self.test_private_key = self.root / "test-ed25519-private.pem"
        self.test_public_key = trusted_dir / "release-public-key.pem"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(self.test_private_key)],
                       check=True, capture_output=True)
        subprocess.run(["openssl", "pkey", "-in", str(self.test_private_key), "-pubout", "-out", str(self.test_public_key)],
                       check=True, capture_output=True)
        self.state = {"pid": 1001, "ticks": 500, "exe": None, "running": False, "fail_restart": False}

        # Fake `sv` executable updates our simulated process state file
        self.sv_log = self.root / "sv.log"
        self.sv_bin = self.prefix / "bin/sv"
        self.sv_bin.write_text(
            "#!/bin/sh\n"
            f"echo \"$*\" >> '{self.sv_log}'\n"
            f"if [ -f '{self.root}/sv-fail' ]; then exit 1; fi\n"
            f"if [ \"$1\" = 'down' ]; then rm -f '{self.root}/running-exe' '{self.root}/running-mcp'; fi\n"
            f"if [ \"$1\" = 'start' ]; then\n"
            f"  target=$(readlink -f '{self.home}/.local/lib/codex/transport/tunnel-client/current/tunnel-client')\n"
            f"  printf '%s' \"$target\" > '{self.root}/running-exe'\n"
            f"  mcp='{self.home}/.local/lib/codex/mcp/termux-shell/current/termux-shell-mcp'\n"
            f"  if [ -f \"$mcp\" ]; then readlink -f \"$mcp\" > '{self.root}/running-mcp'; fi\n"
            f"  n=$(cat '{self.root}/pid-seq' 2>/dev/null || echo 1001)\n"
            f"  n=$((n + 1))\n"
            f"  printf '%s' \"$n\" > '{self.root}/pid-seq'\n"
            f"fi\n"
            "exit 0\n"
        )
        os.chmod(self.sv_bin, 0o755)

    def _install_gen(self, version: str, content: bytes = b"#!/bin/sh\nexit 0\n"):
        gdir = self.home / ".local/lib/codex/transport/tunnel-client/generations" / version
        gdir.mkdir(parents=True, exist_ok=True)
        b = gdir / "tunnel-client"
        b.write_bytes(content)
        os.chmod(b, 0o755)
        self._sign_test_generation(gdir, b)
        return b

    def _sign_test_generation(self, gdir: Path, binary: Path):
        signed = gdir / "signed"
        signed.mkdir(exist_ok=True)
        public = signed / "release-public-key.pem"
        public.write_bytes(self.test_public_key.read_bytes())
        sums = signed / "SHA256SUMS"
        import hashlib
        sums.write_text(f"{hashlib.sha256(binary.read_bytes()).hexdigest()}  bin/{binary.name}\n")
        subprocess.run(["openssl", "pkeyutl", "-sign", "-inkey", str(self.test_private_key), "-rawin",
                        "-in", str(sums), "-out", str(signed / "SHA256SUMS.sig")],
                       check=True, capture_output=True)

    def _proc_reader(self, _service_dir: Path) -> dict:
        exe_file = self.root / "running-exe"
        if not exe_file.exists():
            return {"enrolled": True, "state": "stopped", "pid": None, "exe": None, "start_ticks": None}
        pid = int((self.root / "pid-seq").read_text()) if (self.root / "pid-seq").exists() else 1001
        child = self.root / "running-mcp"
        return {
            "enrolled": True,
            "state": "running",
            "pid": pid,
            "exe": exe_file.read_text().strip(),
            "start_ticks": pid * 10,
            "child_exes": [child.read_text().strip()] if child.exists() else [],
        }

    def _sup(self) -> ServiceSupervisor:
        return ServiceSupervisor(
            component="transport/tunnel-client",
            binary_name="tunnel-client",
            service_name="termux-shell-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=self._proc_reader,
        )

    def test_unenrolled_activate_never_auto_enrolls_service(self):
        self._install_gen("v0.0.20")
        sup = self._sup()
        res = sup.activate("v0.0.20")
        self.assertEqual(res["activation_state"], "not_enrolled")
        self.assertEqual(res["selected_generation"]["version"], "v0.0.20")
        self.assertFalse(sup.is_enrolled())

    def test_stopped_service_does_not_use_logger_pid(self):
        sup = self._sup()
        sup._proc_reader = None
        self._install_gen("v0.0.20")
        sup.activate("v0.0.20")
        sup.enroll(exec_args=[])
        output = f"down: {sup.service_dir}: 0s, normally up; run: log: (pid {os.getpid()}) 4s\n"
        stopped = subprocess.CompletedProcess([], 0, stdout=output, stderr="")
        with mock.patch("scripts.runit_service.subprocess.run", return_value=stopped):
            status = sup.status()["running_generation"]
        self.assertEqual(status["state"], "stopped")
        self.assertIsNone(status["pid"])
        self.assertIsNone(status["exe"])

    def test_health_probe_requires_a_fresh_endpoint_even_if_old_port_answers(self):
        sup = self._sup()
        sup.state_dir.mkdir(parents=True)
        health = sup.state_dir / "health.url"
        health.write_text("http://127.0.0.1:41001\n")
        old = sup._health_file_stamp()
        with mock.patch("scripts.runit_service.http.client.HTTPConnection") as connection:
            connection.return_value.getresponse.return_value.status = 200
            self.assertFalse(sup._tunnel_ready(old, 1))
            connection.assert_not_called()
            next_health = health.with_suffix(".new")
            next_health.write_text("http://127.0.0.1:41002\n")
            next_health.replace(health)
            self.assertTrue(sup._tunnel_ready(old, 1))
            connection.assert_called_once_with("127.0.0.1", 41002, timeout=1)
            connection.return_value.request.assert_called_once_with("GET", "/readyz")
            health.unlink()
            self.assertFalse(sup._tunnel_ready(None, 1, required=True))

    def test_enroll_uses_stable_current_symlink_and_tracks_three_states(self):
        b20 = self._install_gen("v0.0.20", b"bin-20")
        b21 = self._install_gen("v0.0.21", b"bin-21")
        sup = self._sup()
        sup.activate("v0.0.20")

        enr = sup.enroll(exec_args=["run", "--profile", "shell"], extra_env={"FOO": "bar"})
        self.assertTrue(enr["enrolled"])
        run_txt = (self.prefix / "var/service/termux-shell-tunnel/run").read_text()
        # Stable `current` symlink (NEVER hardcoded version path!)
        self.assertIn(".local/lib/codex/transport/tunnel-client/current/tunnel-client", run_txt)
        self.assertNotIn("v0.0.20", run_txt)

        # Simulate service running v0.0.20
        (self.root / "running-exe").write_text(str(b20.resolve()))
        (self.root / "pid-seq").write_text("1001")
        st_active = sup.status()
        self.assertEqual(st_active["activation_state"], "active")
        self.assertEqual(st_active["running_generation"]["version"], "v0.0.20")

        # Busy guard fences selection; the old generation remains selected.
        st_busy = sup.activate("v0.0.21", busy_check_cmd=["sh", "-c", "echo active_lease >&2; exit 1"])
        self.assertEqual(st_busy["activation_state"], "pending_activation")
        self.assertEqual(st_busy["selected_generation"]["version"], "v0.0.20")
        self.assertEqual(st_busy["running_generation"]["version"], "v0.0.20")
        self.assertEqual(st_busy["deferred_reason"], "active_lease")

        # Re-running activate("v0.0.21") once idle performs the deferred selection.
        st_up = sup.activate("v0.0.21", busy_check_cmd=["true"], readiness_cmd=["true"])
        self.assertEqual(st_up["activation_state"], "active")
        self.assertEqual(st_up["selected_generation"]["version"], "v0.0.21")
        self.assertEqual(st_up["running_generation"]["version"], "v0.0.21")
        self.assertEqual(st_up["running_generation"]["exe"], str(b21.resolve()))

        # Rollback restores v0.0.20 cleanly
        st_rb = sup.rollback(busy_check_cmd=["true"], readiness_cmd=["true"])
        self.assertEqual(st_rb["activation_state"], "active")
        self.assertEqual(st_rb["selected_generation"]["version"], "v0.0.20")
        self.assertEqual(st_rb["running_generation"]["version"], "v0.0.20")

    def test_busy_guard_rejection_never_changes_current_generation(self):
        b20 = self._install_gen("v0.0.20", b"bin-20")
        self._install_gen("v0.0.21", b"bin-21")
        sup = self._sup()
        sup.activate("v0.0.20")
        sup.enroll(exec_args=["run"])
        (self.root / "running-exe").write_text(str(b20.resolve()))
        (self.root / "pid-seq").write_text("1001")

        rejected = sup.activate("v0.0.21", busy_check_cmd=["sh", "-c", "exit 1"])
        self.assertEqual(rejected["activation_state"], "pending_activation")
        self.assertEqual(sup._read_link_version(sup.current_link), "v0.0.20")

        missing = sup.activate("v0.0.21")
        self.assertEqual(missing["activation_state"], "pending_activation")
        self.assertEqual(sup._read_link_version(sup.current_link), "v0.0.20")

        with mock.patch("scripts.runit_service.subprocess.run", side_effect=OSError("guard unavailable")):
            unknown = sup.activate("v0.0.21", busy_check_cmd=["missing-busy-check"])
        self.assertEqual(unknown["activation_state"], "pending_activation")
        self.assertEqual(sup._read_link_version(sup.current_link), "v0.0.20")

    def test_failed_readiness_automatically_rolls_back_and_preserves_prior_previous(self):
        self._install_gen("v0.0.19", b"bin-19")
        b20 = self._install_gen("v0.0.20", b"bin-20")
        self._install_gen("v0.0.21", b"bin-21")
        sup = self._sup()
        sup.activate("v0.0.19")
        sup.activate("v0.0.20")
        sup.enroll(exec_args=["run"])
        (self.root / "running-exe").write_text(str(b20.resolve()))
        (self.root / "pid-seq").write_text("2000")

        with self.assertRaisesRegex(SupervisionError, "recovery incomplete"):
            sup.activate("v0.0.21", busy_check_cmd=["true"], readiness_cmd=["false"], restart_timeout=0.4)

        # Must have reverted `current` to v0.0.20 AND preserved `previous` as v0.0.19!
        sel = sup.selected_generation()
        self.assertEqual(sel["version"], "v0.0.20")
        self.assertEqual(sel["previous_version"], "v0.0.19")

    def test_cli_parses_hyphenated_readiness_cmd_and_enroll_args(self):
        norm = preprocess_cli_argv(
            [
                "--component",
                "providers/chatgpt",
                "--binary",
                "chatgpt-provider",
                "--service",
                "chatgpt-provider",
                "status",
                "--readiness-cmd",
                "curl",
                "-fsS",
                "http://127.0.0.1:4319/healthz",
            ]
        )
        self.assertIn("--readiness-cmd=curl -fsS http://127.0.0.1:4319/healthz", norm)

        norm_enroll = preprocess_cli_argv(
            [
                "--component",
                "mcp/termux-shell",
                "--binary",
                "termux-shell-mcp",
                "--runner-component",
                "transport/tunnel-client",
                "--runner-binary",
                "tunnel-client",
                "--service",
                "termux-shell-tunnel",
                "enroll",
                "--arg",
                "run",
                "--arg",
                "--profile",
                "--arg",
                "termux-shell",
            ]
        )
        self.assertIn("--arg=--profile", norm_enroll)

        # End-to-end CLI invocation with hyphenated readiness command (unenrolled)
        self._install_gen("v0.0.20")
        rc = cli_main(
            [
                "--component",
                "transport/tunnel-client",
                "--binary",
                "tunnel-client",
                "--service",
                "termux-shell-tunnel",
                "--home",
                str(self.home),
                "--prefix",
                str(self.prefix),
                "status",
                "--readiness-cmd",
                "curl",
                "-fsS",
                "http://127.0.0.1:4319/healthz",
            ]
        )
        self.assertEqual(rc, 0)

    def test_runner_wrapped_mcp_service_activation_pending_and_rollback(self):
        # Install tunnel-client runner + two MCP binary generations
        b_tunnel = self._install_gen("v0.0.21", b"tunnel-21")
        tunnel_sup = ServiceSupervisor(
            component="transport/tunnel-client",
            binary_name="tunnel-client",
            service_name="termux-shell-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=self._proc_reader,
        )
        tunnel_sup.activate("v0.0.21")

        def install_mcp(ver: str, data: bytes):
            gdir = self.home / ".local/lib/codex/mcp/termux-shell/generations" / ver
            gdir.mkdir(parents=True, exist_ok=True)
            p = gdir / "termux-shell-mcp"
            p.write_bytes(data)
            os.chmod(p, 0o755)
            self._sign_test_generation(gdir, p)
            return p

        install_mcp("0.1.0", b"mcp-0.1.0")
        install_mcp("0.2.0", b"mcp-0.2.0")
        install_mcp("0.3.0", b"mcp-0.3.0")

        mcp_sup = ServiceSupervisor(
            component="mcp/termux-shell",
            binary_name="termux-shell-mcp",
            runner_component="transport/tunnel-client",
            runner_binary="tunnel-client",
            service_name="termux-shell-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=self._proc_reader,
        )
        mcp_sup.activate("0.1.0")
        enr = mcp_sup.enroll(exec_args=["run", "--profile", "termux-shell"])
        self.assertIn("transport/tunnel-client/current/tunnel-client", enr["stable_bin"])

        # A live runner without its MCP child can't identify an active component.
        (self.root / "running-exe").write_text(str(b_tunnel.resolve()))
        (self.root / "pid-seq").write_text("3000")
        self.assertEqual(mcp_sup.status()["activation_state"], "pending_activation")
        st_init = mcp_sup.activate("0.1.0", readiness_cmd="sh -c 'exit 0'")
        self.assertEqual(st_init["activation_state"], "active")
        self.assertEqual(st_init["running_generation"]["version"], "0.1.0")
        self.assertEqual(st_init["running_generation"]["runner_version"], "v0.0.21")
        (self.root / "running-mcp").unlink()
        self.assertEqual(mcp_sup.status()["activation_state"], "pending_activation")
        mcp_sup.activate("0.1.0")

        # Upgrade MCP to 0.2.0 while busy -> pending_activation
        st_busy = mcp_sup.activate("0.2.0", busy_check_cmd="sh -c 'exit 1'")
        self.assertEqual(st_busy["activation_state"], "pending_activation")
        self.assertEqual(st_busy["selected_generation"]["version"], "0.1.0")
        self.assertEqual(st_busy["running_generation"]["version"], "0.1.0")

        # Complete upgrade when idle -> active on 0.2.0
        st_up = mcp_sup.activate("0.2.0", busy_check_cmd=["true"], readiness_cmd=["true"])
        self.assertEqual(st_up["activation_state"], "active")
        self.assertEqual(st_up["running_generation"]["version"], "0.2.0")

        # Readiness timeout on 0.3.0 MUST roll back cleanly (never crash with TimeoutExpired!)
        with self.assertRaisesRegex(SupervisionError, "recovery incomplete"):
            mcp_sup.activate("0.3.0", busy_check_cmd="true", readiness_cmd="sleep 2", restart_timeout=0.4)
        self.assertEqual(mcp_sup.selected_generation()["version"], "0.2.0")
        self.assertEqual(mcp_sup.selected_generation()["previous_version"], "0.1.0")

    def test_status_readiness_timeout_reports_error_instead_of_crashing(self):
        b20 = self._install_gen("v0.0.20", b"bin-20")
        sup = self._sup()
        sup.activate("v0.0.20")
        sup.enroll(exec_args=["run"])
        (self.root / "running-exe").write_text(str(b20.resolve()))
        (self.root / "pid-seq").write_text("4000")
        # Force an immediate timeout by using a missing executable -> OSError caught cleanly
        st = sup.status(readiness_cmd=["/nonexistent-readiness-bin-xyz"])
        self.assertEqual(st["activation_state"], "error")
        self.assertFalse(st["running_generation"]["readiness_ok"])
        self.assertIsNotNone(st["running_generation"]["readiness_error"])

    def test_child_verification_handles_deleted_overlapping_late_and_proc_failures(self):
        b_tunnel = self._install_gen("v0.0.21", b"tunnel-21")
        tunnel_sup = ServiceSupervisor(
            component="transport/tunnel-client",
            binary_name="tunnel-client",
            service_name="termux-shell-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=self._proc_reader,
        )
        tunnel_sup.activate("v0.0.21")

        def install_mcp(ver: str, data: bytes):
            gdir = self.home / ".local/lib/codex/mcp/termux-shell/generations" / ver
            gdir.mkdir(parents=True, exist_ok=True)
            p = gdir / "termux-shell-mcp"
            p.write_bytes(data)
            os.chmod(p, 0o755)
            self._sign_test_generation(gdir, p)
            return p.resolve()

        p1 = install_mcp("0.1.0", b"mcp-1")
        p2 = install_mcp("0.2.0", b"mcp-2")

        state = {
            "pid": 5000,
            "ticks": 50000,
            "child_exes": [f"{p1} (deleted)"],
            "polls": 0,
        }

        def dynamic_reader(_svc: Path) -> dict:
            state["polls"] += 1
            return {
                "enrolled": True,
                "state": "running",
                "pid": state["pid"],
                "start_ticks": state["ticks"],
                "exe": str(b_tunnel.resolve()),
                "child_exes": list(state["child_exes"]),
            }

        mcp_sup = ServiceSupervisor(
            component="mcp/termux-shell",
            binary_name="termux-shell-mcp",
            runner_component="transport/tunnel-client",
            runner_binary="tunnel-client",
            service_name="termux-shell-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=dynamic_reader,
        )
        mcp_sup.activate("0.1.0")
        mcp_sup.enroll(exec_args=["run"])

        # Deleted MCP child image must mark deleted_image=True and pending_activation!
        st_del = mcp_sup.status()
        self.assertEqual(st_del["activation_state"], "pending_activation")
        self.assertTrue(st_del["running_generation"]["deleted_image"])

        # Overlapping old + new MCP children -> pending_activation
        state["child_exes"] = [str(p1), str(p2)]
        mcp_sup.activate("0.2.0", busy_check_cmd=["sh", "-c", "exit 1"])
        self.assertEqual(mcp_sup.status()["activation_state"], "pending_activation")

        # Unreadable live child under runner -> fail-closed pending_activation
        state["child_exes"] = [str(p2), ""]
        self.assertEqual(mcp_sup.status()["activation_state"], "pending_activation")

        # Late-spawning MCP child on restart (empty initially, appears on poll 3) -> succeeds!
        state["child_exes"] = []
        sv_delay = self.prefix / "bin/sv-delay"
        sv_delay.write_text("#!/bin/sh\nexit 0\n")
        os.chmod(sv_delay, 0o755)
        mcp_sup.sv_bin = str(sv_delay)

        fail_new = False
        read_after_start = 0
        running = True
        target_exe = str(p1)

        def operate(action, _timeout):
            nonlocal running, read_after_start, target_exe
            running = action == "start"
            if not running:
                return
            target_exe = str((mcp_sup.current_link / "termux-shell-mcp").resolve())
            state["pid"] += 1
            state["ticks"] += 10
            read_after_start = 0

        def inspect():
            nonlocal read_after_start
            if not running:
                return {"state": "stopped"}
            read_after_start += 1
            state["child_exes"] = (
                [target_exe]
                if read_after_start >= 3 and not (fail_new and target_exe == str(p2))
                else []
            )
            return dynamic_reader(mcp_sup.service_dir)

        with mock.patch.object(mcp_sup, "_inspect_running", side_effect=inspect), mock.patch.object(
            mcp_sup, "_run_sv", side_effect=operate,
        ):
            self.assertEqual(mcp_sup.activate("0.2.0", busy_check_cmd=["true"], restart_timeout=2.0)["activation_state"], "active")
            read_after_start = 0
            self.assertEqual(mcp_sup.status()["activation_state"], "pending_activation")

            # A failed retry of an already selected generation restores the prior
            # running generation without duplicating the current and previous links.
            fail_new = True
            target_exe = str(p1)
            with self.assertRaisesRegex(SupervisionError, "rolled back"):
                mcp_sup.activate("0.2.0", busy_check_cmd=["true"], restart_timeout=0.5)
            sel = mcp_sup.selected_generation()
            self.assertEqual(sel["version"], "0.1.0")
            self.assertIsNone(sel["previous_version"])
            self.assertEqual(mcp_sup.status()["activation_state"], "active")

        # Real /proc helper fail-closed tests: /proc OSError & PID start_ticks change
        with mock.patch("scripts.runit_service.Path.iterdir", side_effect=OSError("proc unavailable")):
            self.assertEqual(proc_child_exes(os.getpid(), 1), [])
        real_pid = os.getpid()
        with mock.patch("scripts.runit_service.proc_start_ticks", return_value=999999999):
            self.assertEqual(proc_child_exes(real_pid, 1), [])

    def test_coordinated_companion_activation_and_rollback_prevent_mixed_orch_ledger(self):
        b_tunnel = self._install_gen("v0.0.21", b"tunnel-21")
        tunnel_sup = ServiceSupervisor(
            component="transport/tunnel-client",
            binary_name="tunnel-client",
            service_name="termux-orch-tunnel",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(self.sv_bin),
            proc_reader=self._proc_reader,
        )
        tunnel_sup.activate("v0.0.21")

        def install_orch(ver: str, data: bytes):
            gdir = self.home / ".local/lib/codex/mcp/termux-orch/generations" / ver
            gdir.mkdir(parents=True, exist_ok=True)
            p = gdir / "termux-orch"
            p.write_bytes(data)
            os.chmod(p, 0o755)
            self._sign_test_generation(gdir, p)
            return p.resolve()

        o1 = install_orch("0.1.0", b"orch-1")
        o2 = install_orch("0.1.0-new", b"orch-2")
        o3 = install_orch("0.1.0-bad", b"orch-3")

        live = {
            "termux-orch-tunnel": {"state": "running", "pid": 7001, "ticks": 70010, "exe": str(b_tunnel.resolve()), "child_exes": [str(o1)]},
            "termux-orch-relay": {"state": "running", "pid": 7002, "ticks": 70020, "exe": str(o1), "child_exes": []},
        }
        fail_service = {"name": None}
        events: list[tuple] = []

        sv_orch = self.prefix / "bin/sv-orch"
        sv_orch.write_text("#!/bin/sh\nexit 0\n")
        os.chmod(sv_orch, 0o755)

        def reader(svc_dir: Path) -> dict:
            rec = live[svc_dir.name]
            return {
                "enrolled": True,
                "state": rec["state"],
                "pid": rec["pid"],
                "start_ticks": rec["ticks"],
                "exe": rec["exe"],
                "child_exes": list(rec["child_exes"]),
            }

        # Intercept subprocess.run(['sv-orch', 'stop|start|restart', svc]) and record event order.
        import subprocess as sp
        real_run = sp.run

        def fake_run(cmd, *args, **kwargs):
            if isinstance(cmd, list) and len(cmd) >= 3 and cmd[0] == str(sv_orch):
                act = cmd[1]
                svc_name = Path(cmd[2]).name
                cur_target = str((self.home / ".local/lib/codex/mcp/termux-orch/current/termux-orch").resolve())
                events.append((svc_name, act, os.readlink(self.home / ".local/lib/codex/mcp/termux-orch/current")))
                if act == "down":
                    live[svc_name]["state"] = "stopped"
                    live[svc_name]["pid"] = None
                elif not (fail_service["name"] == svc_name and cur_target == str(o3)):
                    live[svc_name]["state"] = "running"
                    live[svc_name]["pid"] = (live[svc_name]["pid"] or 7000) + 10
                    live[svc_name]["ticks"] += 100
                    if svc_name == "termux-orch-tunnel":
                        live[svc_name]["child_exes"] = [cur_target]
                    else:
                        live[svc_name]["exe"] = cur_target
                return sp.CompletedProcess(cmd, 0, "", "")
            return real_run(cmd, *args, **kwargs)

        orch_sup = ServiceSupervisor(
            component="mcp/termux-orch",
            binary_name="termux-orch",
            service_name="termux-orch-tunnel",
            runner_component="transport/tunnel-client",
            runner_binary="tunnel-client",
            companion_services=["termux-orch-relay"],
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(sv_orch),
            proc_reader=reader,
        )
        relay_sup = ServiceSupervisor(
            component="mcp/termux-orch",
            binary_name="termux-orch",
            service_name="termux-orch-relay",
            home=self.home,
            prefix=self.prefix,
            sv_bin=str(sv_orch),
            proc_reader=reader,
        )
        orch_sup.activate("0.1.0")
        orch_sup.enroll(exec_args=["mcp"])
        relay_sup.enroll(exec_args=["relay"])
        self.assertEqual(orch_sup.status()["activation_state"], "active")

        with mock.patch("scripts.runit_service.subprocess.run", side_effect=fake_run):
            st_ok = orch_sup.activate("0.1.0-new", busy_check_cmd=["true"], restart_timeout=1.0)
            self.assertEqual(st_ok["activation_state"], "active")
            self.assertEqual(st_ok["running_generation"]["version"], "0.1.0-new")
            self.assertEqual(st_ok["companions"][0]["running_generation"]["version"], "0.1.0-new")
            # Companion relay must have stopped BEFORE current flipped to 0.1.0-new!
            self.assertEqual(events[:4], [
                ("termux-orch-tunnel", "down", "generations/0.1.0"),
                ("termux-orch-relay", "down", "generations/0.1.0"),
                ("termux-orch-relay", "start", "generations/0.1.0-new"),
                ("termux-orch-tunnel", "start", "generations/0.1.0-new"),
            ])

            # Inject failure on companion relay start -> BOTH tunnel & relay MUST roll back to 0.1.0-new!
            fail_service["name"] = "termux-orch-tunnel"
            events.clear()
            with self.assertRaisesRegex(SupervisionError, "rolled back"):
                orch_sup.activate("0.1.0-bad", busy_check_cmd=["true"], restart_timeout=0.4)
            # The new relay has already started. Rollback must stop it and
            # the attempted tunnel before exposing the old selection.
            rollback_stop = [event for event in events if event[1] == "down"
                             and event[2] == "generations/0.1.0-bad"]
            self.assertEqual([event[0] for event in rollback_stop],
                             ["termux-orch-tunnel", "termux-orch-relay"])
            st_rb = orch_sup.status()
            self.assertEqual(st_rb["activation_state"], "active")
            self.assertEqual(st_rb["selected_generation"]["version"], "0.1.0-new")
            self.assertEqual(live["termux-orch-tunnel"]["child_exes"], [str(o2)])
            self.assertEqual(live["termux-orch-relay"]["exe"], str(o2))
            self.assertEqual(o3.exists(), True)


if __name__ == "__main__":
    unittest.main()
