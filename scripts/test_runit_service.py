#!/usr/bin/env python3
import os
from pathlib import Path
import tempfile
import unittest

from scripts.runit_service import ServiceSupervisor, SupervisionError, main as cli_main, preprocess_cli_argv


class RunitSupervisionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="codex-runit-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.home = self.root / "home"
        self.prefix = self.root / "usr"
        self.home.mkdir(mode=0o700)
        (self.prefix / "bin").mkdir(parents=True, mode=0o700)
        self.state = {"pid": 1001, "ticks": 500, "exe": None, "running": False, "fail_restart": False}

        # Fake `sv` executable updates our simulated process state file
        self.sv_log = self.root / "sv.log"
        self.sv_bin = self.prefix / "bin/sv"
        self.sv_bin.write_text(
            "#!/bin/sh\n"
            f"echo \"$*\" >> '{self.sv_log}'\n"
            f"if [ -f '{self.root}/sv-fail' ]; then exit 1; fi\n"
            f"if [ \"$1\" = 'restart' ]; then\n"
            f"  target=$(readlink -f '{self.home}/.local/lib/codex/transport/tunnel-client/current/tunnel-client')\n"
            f"  printf '%s' \"$target\" > '{self.root}/running-exe'\n"
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
        return b

    def _proc_reader(self, _service_dir: Path) -> dict:
        exe_file = self.root / "running-exe"
        if not exe_file.exists():
            return {"enrolled": True, "state": "stopped", "pid": None, "exe": None, "start_ticks": None}
        pid = int((self.root / "pid-seq").read_text()) if (self.root / "pid-seq").exists() else 1001
        return {
            "enrolled": True,
            "state": "running",
            "pid": pid,
            "exe": exe_file.read_text().strip(),
            "start_ticks": pid * 10,
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

        # Busy guard defers restart cleanly -> pending_activation (selected=v0.0.21, running=v0.0.20)
        st_busy = sup.activate("v0.0.21", busy_check_cmd=["sh", "-c", "echo active_lease >&2; exit 1"])
        self.assertEqual(st_busy["activation_state"], "pending_activation")
        self.assertEqual(st_busy["selected_generation"]["version"], "v0.0.21")
        self.assertEqual(st_busy["running_generation"]["version"], "v0.0.20")
        self.assertEqual(st_busy["deferred_reason"], "active_lease")

        # Re-running activate("v0.0.21") once idle MUST complete deferred pending_activation!
        st_up = sup.activate("v0.0.21", readiness_cmd=["true"])
        self.assertEqual(st_up["activation_state"], "active")
        self.assertEqual(st_up["selected_generation"]["version"], "v0.0.21")
        self.assertEqual(st_up["running_generation"]["version"], "v0.0.21")
        self.assertEqual(st_up["running_generation"]["exe"], str(b21.resolve()))

        # Rollback restores v0.0.20 cleanly
        st_rb = sup.rollback(readiness_cmd=["true"])
        self.assertEqual(st_rb["activation_state"], "active")
        self.assertEqual(st_rb["selected_generation"]["version"], "v0.0.20")
        self.assertEqual(st_rb["running_generation"]["version"], "v0.0.20")

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

        with self.assertRaisesRegex(SupervisionError, "rolled back"):
            sup.activate("v0.0.21", readiness_cmd=["false"], restart_timeout=0.4)

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

        # Simulate tunnel running v0.0.21 before activation marker is recorded -> activate 0.1.0 settles it
        (self.root / "running-exe").write_text(str(b_tunnel.resolve()))
        (self.root / "pid-seq").write_text("3000")
        st_init = mcp_sup.activate("0.1.0", readiness_cmd="sh -c 'exit 0'")
        self.assertEqual(st_init["activation_state"], "active")
        self.assertEqual(st_init["running_generation"]["version"], "0.1.0")
        self.assertEqual(st_init["running_generation"]["runner_version"], "v0.0.21")

        # Upgrade MCP to 0.2.0 while busy -> pending_activation
        st_busy = mcp_sup.activate("0.2.0", busy_check_cmd="sh -c 'exit 1'")
        self.assertEqual(st_busy["activation_state"], "pending_activation")
        self.assertEqual(st_busy["selected_generation"]["version"], "0.2.0")
        self.assertEqual(st_busy["running_generation"]["version"], "0.1.0")

        # Complete upgrade when idle -> active on 0.2.0
        st_up = mcp_sup.activate("0.2.0", readiness_cmd=["true"])
        self.assertEqual(st_up["activation_state"], "active")
        self.assertEqual(st_up["running_generation"]["version"], "0.2.0")

        # Readiness timeout on 0.3.0 MUST roll back cleanly (never crash with TimeoutExpired!)
        with self.assertRaisesRegex(SupervisionError, "rolled back"):
            mcp_sup.activate("0.3.0", readiness_cmd="sleep 2", restart_timeout=0.4)
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


if __name__ == "__main__":
    unittest.main()
