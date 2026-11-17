#!/usr/bin/env python3
"""Owned-PREFIX fault injection for the standalone POSIX installer.

All SIGKILL targets are Popen children of this harness. This is an actual
filesystem/process-boundary test using deterministic shims, not a power cut or
a test of official upstream archive acquisition.
"""
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time
import unittest

INSTALL = Path(__file__).resolve().parents[2] / "install.sh"
IS_ANDROID_LINKER = Path("/system/bin/linker64").is_file()


class InstallerRecovery(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="wrapper-install-p0-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.prefix = self.root / "prefix"
        self.home = self.root / "home"
        self.temp = self.root / "tmp"
        self.shims = self.root / "shims"
        for path in (self.prefix / "bin", self.home, self.temp, self.shims):
            path.mkdir(parents=True)
        self.old = self.root / "old"
        self.new = self.root / "new"
        self.old.write_text("#!/bin/sh\necho ORIGINAL_WRAPPER\n")
        self.new.write_text("""#!/bin/sh
if [ "${1:-}" = update ] && [ "${2:-}" = --help ]; then
  if [ -f "$HOME/pause-validation" ]; then
    : > "$HOME/hit"
    x=0
    while [ ! -f "$HOME/release-shim" ] && [ "$x" -lt 160 ]; do
      sleep 0.05; x=$((x+1))
    done
  fi
  exit 0
fi
if [ "${1:-}" = wrapper ]; then
  [ -f "$HOME/.local/share/codex/wrapper/activation-state" ] || exit 1
  printf '{"status":"active"}\n'
  exit 0
fi
if [ "${1:-}" = update ] && [ "${2:-}" = --upstream ]; then
  echo "$$" > "$HOME/updater-pid"
  : > "$HOME/update-started"
  mode=$(cat "$HOME/mode")
  if [ "$mode" = fail ]; then exit 57; fi
  if [ "$mode" = block ]; then
    x=0
    while [ ! -f "$HOME/release-updater" ] && [ "$x" -lt 200 ]; do
      sleep 0.05; x=$((x+1))
    done
    exit 0
  fi
  if [ "$mode" = commit ] || [ "$mode" = commit-block ]; then
    root="$HOME/.local/share/codex/wrapper"
    generation="$HOME/.local/lib/codex/wrapper/generations/gen-stub"
    mkdir -p "$root" "$generation"
    digest=$(sha256sum "$PREFIX/bin/codex" | awk '{print $1}')
    printf 'format=codex-activation-state\nupdate_key=%064d\ncurrent=gen-stub\ncurrent_key=%064d\nprevious_present=0\nprevious=\nprevious_key=\n' 0 0 > "$root/activation-state"
    printf 'wrapper_artifact_digest\t%s\nupstream_package_version\t0.169.0\n' "$digest" > "$generation/generation.meta"
    if [ "$mode" = commit-block ]; then
      : > "$HOME/activation-committed"
      x=0
      while [ ! -f "$HOME/release-updater" ] && [ "$x" -lt 200 ]; do
        sleep 0.05; x=$((x+1))
      done
    fi
    exit 0
  fi
fi
exit 0
""")
        for p in (self.old, self.new):
            p.chmod(0o755)
        shutil.copyfile(self.old, self.prefix / "bin/codex")
        (self.home / "mode").write_text("fail")
        self.shim = """#!/bin/sh
op=${0##*/}
real="$TEST_REAL_TOOLS/$op"
case "$TEST_HOOK:$op" in
  stage-partial:cp|backup-partial:cp)
    match='.codex.tmp.'
    dest="${2:-}"
    if [ "$TEST_HOOK" = backup-partial ]; then
      match='.codex.previous.'
      dest="${3:-}"
    fi
    case "$dest" in
      *"$match"*)
        if [ "$TEST_HOOK" = backup-partial ]; then from="$2"; else from="$1"; fi
        dd if="$from" of="$dest" bs=1 count=6 2>/dev/null
        : > "$HOME/hit"
        x=0
        while [ ! -f "$HOME/release-shim" ] && [ "$x" -lt 160 ]; do sleep 0.05; x=$((x+1)); done
        exit 77 ;;
    esac ;;
  swap-before:mv|swap-after:mv)
    case "${2:-}" in
      *.codex.tmp.*)
        if [ "$TEST_HOOK" = swap-after ]; then "$real" "$@"; fi
        : > "$HOME/hit"
        x=0
        while [ ! -f "$HOME/release-shim" ] && [ "$x" -lt 160 ]; do sleep 0.05; x=$((x+1)); done
        if [ "$TEST_HOOK" = swap-before ]; then exec "$real" "$@"; fi
        exit 0 ;;
    esac ;;
  updated-after:mv)
    case "${2:-}" in
      */transaction.next)
        if grep -q '^phase=updated$' "$2"; then
          "$real" "$@"
          : > "$HOME/hit"
          x=0
          while [ ! -f "$HOME/release-shim" ] && [ "$x" -lt 160 ]; do sleep 0.05; x=$((x+1)); done
          exit 0
        fi ;;
    esac ;;
esac
exec "$real" "$@"
"""
        tools_dir = self.root / "realtools"
        tools_dir.mkdir()
        for tool in ("cp", "mv", "sync"):
            resolved = shutil.which(tool)
            assert resolved, tool
            (tools_dir / tool).symlink_to(resolved)
            shim = self.shims / tool
            shim.write_text(self.shim)
            shim.chmod(0o755)
        self.env = dict(os.environ, HOME=str(self.home), PREFIX=str(self.prefix),
                        TMPDIR=str(self.temp),
                        PATH=f"{self.shims}:{os.environ['PATH']}",
                        TEST_REAL_TOOLS=str(tools_dir), TEST_HOOK="")
        self.log = self.root / "installer.log"

    def run_install(self, *flags, hook="", timeout=20):
        env = dict(self.env, TEST_HOOK=hook)
        return subprocess.run(["sh", str(INSTALL), "--prefix", str(self.prefix),
                               "--binary", str(self.new), *flags],
                              env=env, capture_output=True, text=True, timeout=timeout)

    def recover(self):
        return subprocess.run(["sh", str(INSTALL), "--prefix", str(self.prefix),
                               "--recover"], env=self.env, capture_output=True,
                              text=True, timeout=20)

    def launch(self, *flags, hook=""):
        env = dict(self.env, TEST_HOOK=hook)
        self.log.write_text("")
        out = self.log.open("w")
        self.addCleanup(out.close)
        proc = subprocess.Popen(["sh", str(INSTALL), "--prefix", str(self.prefix),
                                 "--binary", str(self.new), *flags],
                                env=env, stdout=out, stderr=subprocess.STDOUT)
        self.addCleanup(self.cleanup_process, proc)
        return proc

    def cleanup_process(self, proc):
        (self.home / "release-shim").touch()
        (self.home / "release-updater").touch()
        if proc.poll() is None:
            proc.kill()  # Only the Popen-owned installer test process.
            proc.wait(timeout=4)

    def await_marker(self, marker, timeout=12):
        marker = self.home / marker
        limit = time.monotonic() + timeout
        while not marker.exists():
            if time.monotonic() > limit:
                self.fail(f"timeout waiting for owned fixture marker {marker}: {self.log.read_text()}")
            time.sleep(0.04)

    def interrupt(self, proc):
        self.assertIsNone(proc.poll(), self.log.read_text())
        proc.send_signal(signal.SIGKILL)
        self.assertEqual(proc.wait(timeout=4), -signal.SIGKILL)
        (self.home / "release-shim").touch()

    def old_bytes(self):
        return (self.prefix / "bin/codex").read_bytes() == self.old.read_bytes()

    def has_lock(self):
        return (self.prefix / "bin/.codex-install.lock").exists()

    def test_i01_copy_staging_partial_kill(self):
        p = self.launch("--skip-upstream", hook="stage-partial")
        self.await_marker("hit")
        self.interrupt(p)
        self.assertTrue(self.old_bytes())
        r = self.recover()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse(self.has_lock())

    def test_i02_staged_validation_kill(self):
        (self.home / "pause-validation").touch()
        p = self.launch("--skip-upstream")
        self.await_marker("hit")
        self.interrupt(p)
        self.assertTrue(self.old_bytes())
        self.assertEqual(self.recover().returncode, 0)

    def test_i03_backup_partial_kill(self):
        p = self.launch("--skip-upstream", hook="backup-partial")
        self.await_marker("hit")
        self.interrupt(p)
        self.assertTrue(self.old_bytes())
        self.assertEqual(self.recover().returncode, 0)

    def test_i04_before_swap_kill(self):
        p = self.launch("--skip-upstream", hook="swap-before")
        self.await_marker("hit")
        self.interrupt(p)
        self.assertTrue(self.old_bytes())
        self.assertEqual(self.recover().returncode, 0)

    def test_i05_after_swap_kill_restores_old(self):
        p = self.launch("--version", "0.169.0", hook="swap-after")
        self.await_marker("hit")
        self.interrupt(p)
        self.assertFalse(self.old_bytes())
        self.assertTrue(self.has_lock())
        r = self.recover()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.old_bytes())
        self.assertFalse(self.has_lock())

    @unittest.skipUnless(IS_ANDROID_LINKER, "requires real Android linker branch")
    def test_i06_i09_child_survives_parent_kill_and_recovery_refuses(self):
        (self.home / "mode").write_text("block")
        p = self.launch("--version", "0.169.0")
        self.await_marker("update-started")
        self.interrupt(p)
        child = int((self.home / "updater-pid").read_text())
        os.kill(child, 0)
        r = self.recover()
        self.assertNotEqual(r.returncode, 0)
        self.assertTrue(self.has_lock())
        (self.home / "release-updater").touch()

    @unittest.skipUnless(IS_ANDROID_LINKER, "requires real Android linker branch")
    def test_i07_activation_then_parent_killed_fails_closed_while_child_alive(self):
        (self.home / "mode").write_text("commit-block")
        p = self.launch("--version", "0.169.0")
        self.await_marker("activation-committed")
        self.interrupt(p)
        r = self.recover()
        self.assertNotEqual(r.returncode, 0)
        self.assertTrue(self.has_lock())
        (self.home / "release-updater").touch()
        pid = int((self.home / "updater-pid").read_text())
        for _ in range(80):
            procstat = Path(f"/proc/{pid}/stat")
            if not procstat.exists() or procstat.read_text().split(") ")[-1][0] in "ZX":
                break
            time.sleep(0.05)
        recovered = self.recover()
        self.assertEqual(recovered.returncode, 0, recovered.stderr)
        self.assertFalse(self.has_lock())

    @unittest.skipUnless(IS_ANDROID_LINKER, "requires real Android linker branch")
    def test_i08_after_activation_and_wait_kill_recovers_without_rerun(self):
        (self.home / "mode").write_text("commit")
        p = self.launch("--version", "0.169.0", hook="updated-after")
        self.await_marker("hit")
        self.interrupt(p)
        r = self.recover()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse(self.has_lock())
        self.assertFalse(self.old_bytes())

    def test_i10_pid_reuse_must_not_reclaim(self):
        p = self.launch("--skip-upstream", hook="swap-before")
        self.await_marker("hit")
        self.interrupt(p)
        owner = self.prefix / "bin/.codex-install.lock/owner"
        value = owner.read_text()
        lines = value.splitlines()
        owner.write_text("\n".join(f"pid={os.getpid()}" if v.startswith("pid=") else
                                   "start=9" if v.startswith("start=") else v
                                   for v in lines) + "\n")
        r = self.recover()
        self.assertNotEqual(r.returncode, 0)
        self.assertTrue(self.has_lock())

    def test_i11_competing_writers(self):
        p = self.launch("--skip-upstream", hook="swap-before")
        self.await_marker("hit")
        other = self.run_install("--skip-upstream")
        self.assertNotEqual(other.returncode, 0)
        self.assertIn("install", other.stderr)
        (self.home / "release-shim").touch()
        self.assertEqual(p.wait(timeout=10), 0, self.log.read_text())
        self.assertFalse(self.has_lock())

    @unittest.skipUnless(IS_ANDROID_LINKER, "requires real Android linker branch")
    def test_i12_i13_failed_bootstrap_restores_or_clears(self):
        before = (self.prefix / "bin/codex").read_bytes()
        r = self.run_install("--version", "0.169.0")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual((self.prefix / "bin/codex").read_bytes(), before)
        self.assertFalse(self.has_lock())
        (self.prefix / "bin/codex").unlink()
        r = self.run_install("--version", "0.169.0")
        self.assertNotEqual(r.returncode, 0)
        self.assertFalse((self.prefix / "bin/codex").exists())
        self.assertFalse(self.has_lock())

    def test_i14_skip_upstream(self):
        r = self.run_install("--skip-upstream")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse(self.has_lock())
        self.assertFalse(self.old_bytes())

    def test_i15_safe_reentry_replaces_one_writer(self):
        p = self.launch("--skip-upstream", hook="swap-after")
        self.await_marker("hit")
        self.interrupt(p)
        r = self.run_install("--skip-upstream")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse(self.has_lock())
        self.assertFalse(self.old_bytes())


if __name__ == "__main__":
    unittest.main(verbosity=2)
