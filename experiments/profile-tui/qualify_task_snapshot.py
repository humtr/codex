"""Observe actual owned Core/Manager kernel writers across native account transfer."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from qualify_tasks import qualify


def qualify_snapshot(generation, parent, public_key):
    observed = []

    def observe(core, environment, root, thread, expected):
        home = Path(environment["HOME"])
        # Keep the caller on the first writer's profile, even after the real writer
        # moves to default/B. A selected or original account must not become owner.
        caller = home / ".local/share/codex/manager/profiles/account-a/home"
        env = dict(environment, CODEX_HOME=str(caller))
        def query(command):
            result = subprocess.run([str(core), "termux", command], env=env,
                                    cwd=root, capture_output=True, timeout=15)
            assert result.returncode == 0, result.stderr.decode()
            assert result.stderr == b"" and len(result.stdout) <= 256 * 1024
            return json.loads(result.stdout)
        profiles = query("__profile-snapshot-v1")
        assert profiles["current"] == "account-a"
        before = {p: p.stat() for p in [caller, caller / "config.toml"]}
        snapshot = query("__task-snapshot-v1")
        assert set(snapshot) == {"schema", "tasks"}
        assert snapshot["schema"] == "codex-manager-tasks-v1"
        tasks = snapshot["tasks"]
        assert len(tasks) <= 1024
        assert [t["id"] for t in tasks] == sorted({t["id"] for t in tasks})
        for task in tasks:
            assert set(task) == {"id", "owner_profile", "state", "server_token"}
            assert task["state"] in {"active", "idle", "systemError", "notLoaded"}
            pid, start = task["server_token"].split(":")
            assert int(pid) > 1 and start.isdecimal()
            assert task["owner_profile"] in profiles["profiles"] or task["owner_profile"] is None
        owner = next(t for t in tasks if t["id"] == thread)
        assert owner["owner_profile"] == expected, owner
        status = subprocess.run([str(core), "termux", "task", "status", thread],
                                env=env, cwd=root, capture_output=True, timeout=15)
        assert status.returncode == 0, status.stderr.decode()
        text = status.stdout.decode()
        assert "\towner=" + expected + "\t" in text
        assert "\tstate=" + owner["state"] + "\t" in text
        assert "\tserver=" + owner["server_token"] + "\n" in text
        assert str(root) not in json.dumps(snapshot), "snapshot exposes owned paths"
        assert all((p.stat().st_ino, p.stat().st_mode, p.stat().st_size, p.stat().st_mtime_ns) ==
                   (s.st_ino, s.st_mode, s.st_size, s.st_mtime_ns) for p, s in before.items())
        observed.append(expected)
        print("PASS private native writer snapshot: caller=account-a, actual=" + expected)

    qualify(generation, parent, public_key, observe_owner=observe)
    assert observed == ["account-a", "default", "account-b"], observed
    print("PASS native snapshot follows current writer rather than creator/caller/default preference")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generation", required=True, type=Path)
    parser.add_argument("--public-key", required=True, type=Path)
    parser.add_argument("--workdir", required=True, type=Path)
    args = parser.parse_args()
    qualify_snapshot(args.generation.resolve(), args.workdir.resolve(), args.public_key.resolve())
