#!/usr/bin/env python3
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

MODULE_PATH = Path(__file__).with_name("shared_state_migrate.py")
SPEC = importlib.util.spec_from_file_location("shared_state_migrate", MODULE_PATH)
assert SPEC and SPEC.loader
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def make_state(root: Path, rows):
    root.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(root / "state_5.sqlite")
    con.execute(
        """
        CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            rollout_path TEXT,
            recency_at_ms INTEGER,
            history_mode TEXT,
            name TEXT,
            daybreak_enabled INTEGER,
            project_id TEXT,
            section TEXT
        )
        """
    )
    con.executemany(
        "INSERT INTO threads VALUES (?,?,?,?,?,?,?,?)",
        rows,
    )
    con.commit()
    con.close()


def rollout(root: Path, rel: str, thread_id: str, history_mode="legacy", marker="same"):
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "timestamp": "2026-01-01T00:00:00Z",
        "type": "session_meta",
        "payload": {
            "id": thread_id,
            "session_id": thread_id,
            "timestamp": "2026-01-01T00:00:00Z",
            "history_mode": history_mode,
            "cwd": "/tmp",
            "originator": "test",
            "source": "cli",
            "model_provider": "openai",
            "cli_version": "test",
            "marker": marker,
        },
    }
    path.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")
    return path


class MigrationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.legacy = self.root / "legacy"
        self.target = self.root / "target"
        self.legacy.mkdir()
        self.target.mkdir()

    def tearDown(self):
        self.tmp.cleanup()

    def test_selects_five_recoverable_and_dedupes_by_thread(self):
        p1 = self.legacy / "a"
        p2 = self.legacy / "b"
        rows1 = []
        # Newest row is stale, then five recoverable rows.
        rows1.append(("stale", "sessions/stale.jsonl", 200, "legacy", None, 0, None, None))
        for i, activity in enumerate([190, 180, 170, 160, 150], start=1):
            tid = f"t{i}"
            rel = f"sessions/2026/01/0{i}/rollout-{tid}.jsonl"
            rollout(p1, rel, tid, marker=tid)
            rows1.append((tid, rel, activity, "legacy", f"name-{tid}", 0, None, None))
        make_state(p1, rows1)

        # Duplicate t1 byte-identically, but with a more recent explicit name.
        src = p1 / rows1[1][1]
        rel = rows1[1][1]
        (p2 / rel).parent.mkdir(parents=True, exist_ok=True)
        (p2 / rel).symlink_to(src)
        rollout(p2, "sessions/2026/02/01/rollout-u1.jsonl", "u1", marker="u1")
        make_state(
            p2,
            [
                ("t1", rel, 300, "legacy", "newer-name", 0, None, None),
                ("u1", "sessions/2026/02/01/rollout-u1.jsonl", 250, "legacy", None, 0, None, None),
            ],
        )

        plan = m.build_plan(self.legacy, self.target)
        pa = next(p for p in plan["profiles"] if p["profile"] == "a")
        self.assertEqual(len(pa["selected"]), 5)
        self.assertEqual(pa["stale_skipped"], 1)
        self.assertEqual(len(plan["unique"]), 6)
        t1 = next(u for u in plan["unique"] if u["thread_id"] == "t1")
        self.assertEqual(t1["name"], "newer-name")
        self.assertEqual(t1["target_status"], "copy")

    def test_divergent_duplicate_fails_closed(self):
        for profile, marker in [("a", "one"), ("b", "two")]:
            root = self.legacy / profile
            rel = "sessions/2026/01/01/rollout-same.jsonl"
            rollout(root, rel, "same", marker=marker)
            make_state(root, [("same", rel, 10, "legacy", None, 0, None, None)])
        with self.assertRaises(m.MigrationError):
            m.build_plan(self.legacy, self.target)

    def test_dedicated_log_db_is_not_a_conversation_dependency(self):
        root = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        rollout(root, rel, "t1")
        make_state(root, [("t1", rel, 10, "legacy", None, 0, None, None)])
        con = sqlite3.connect(root / "logs_2.sqlite")
        con.execute("CREATE TABLE logs (id INTEGER, thread_id TEXT, body TEXT)")
        con.execute("INSERT INTO logs VALUES (1,'t1','diagnostic')")
        con.commit()
        con.close()
        plan = m.build_plan(self.legacy, self.target)
        self.assertEqual(len(plan["unique"]), 1)

    def test_hidden_thread_dependency_fails_closed(self):
        root = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        rollout(root, rel, "t1")
        make_state(root, [("t1", rel, 10, "legacy", None, 0, None, None)])
        con = sqlite3.connect(root / "queue_1.sqlite")
        con.execute("CREATE TABLE queued_messages (thread_id TEXT, body TEXT)")
        con.execute("INSERT INTO queued_messages VALUES ('t1','pending')")
        con.commit()
        con.close()
        with self.assertRaises(m.MigrationError):
            m.build_plan(self.legacy, self.target)

    def test_existing_target_wins_without_copy_or_name_override(self):
        source = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        src = rollout(source, rel, "t1")
        make_state(source, [("t1", rel, 10, "legacy", "source-name", 0, None, None)])

        dst = self.target / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(src.read_bytes())
        make_state(self.target, [("t1", rel, 20, "legacy", "target-name", 0, None, None)])

        plan = m.build_plan(self.legacy, self.target)
        entry = plan["unique"][0]
        self.assertEqual(entry["target_status"], "identical")
        self.assertEqual(entry["name"], "target-name")

    def test_apply_and_rollback_normalizes_canonical_legacy_alias(self):
        source = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        src = rollout(source, rel, "t1")
        make_state(source, [("t1", rel, 10, "legacy", "source-name", 0, None, None)])

        alias = self.target / rel
        alias.parent.mkdir(parents=True, exist_ok=True)
        alias.symlink_to(src)
        make_state(self.target, [("t1", rel, 20, "legacy", "target-name", 0, None, None)])

        plan = m.build_plan(self.legacy, self.target)
        entry = plan["unique"][0]
        self.assertEqual(entry["target_status"], "replace_alias")
        self.assertEqual(entry["name"], "target-name")

        manifest = self.root / "alias-plan.json"
        journal = self.root / "alias-journal.json"
        m.write_plan(manifest, plan)
        result = m.apply_plan(m.load_plan(manifest), journal)
        self.assertEqual(result["status"], "applied")
        self.assertTrue(alias.is_file())
        self.assertFalse(alias.is_symlink())
        self.assertEqual(alias.read_bytes(), src.read_bytes())

        with journal.open() as handle:
            saved = json.load(handle)
        m.rollback(saved, journal)
        self.assertTrue(alias.is_symlink())
        self.assertEqual(alias.resolve(strict=True), src.resolve(strict=True))

    def test_finalize_uses_upstream_api_and_blocks_payload_rollback(self):
        source = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        rollout(source, rel, "t1")
        make_state(source, [("t1", rel, 10, "legacy", "restored-name", 0, None, None)])

        plan = m.build_plan(self.legacy, self.target)
        manifest = self.root / "finalize-plan.json"
        journal = self.root / "finalize-journal.json"
        m.write_plan(manifest, plan)
        applied = m.apply_plan(m.load_plan(manifest), journal)
        self.assertEqual(applied["status"], "applied")

        fake = self.root / "fake-codex"
        fake.write_text(
            f"#!{sys.executable}\n"
            + """import json
import os
from pathlib import Path
import sys

root=Path(os.environ["CODEX_HOME"])
threads={}
for path in root.rglob("*.jsonl"):
    if path.is_symlink() or not path.is_file():
        continue
    with path.open("r",encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            obj=json.loads(line)
            payload=obj.get("payload") or {}
            tid=payload.get("id")
            if isinstance(tid,str):
                threads[tid]={"id":tid,"name":None}
            break

for line in sys.stdin:
    if not line.strip():
        continue
    req=json.loads(line)
    rid=req.get("id")
    if rid is None:
        continue
    method=req.get("method")
    params=req.get("params") or {}
    if method=="initialize":
        result={}
    elif method=="thread/list":
        result={"data":list(threads.values())}
    elif method=="thread/resume":
        result={}
    elif method=="thread/turns/list":
        result={"data":[],"nextCursor":None}
    elif method=="thread/name/set":
        threads[params["threadId"]]["name"]=params["name"]
        result={}
    else:
        print(json.dumps({"id":rid,"error":{"message":"unsupported"}}),flush=True)
        continue
    print(json.dumps({"id":rid,"result":result}),flush=True)
""",
            encoding="utf-8",
        )
        fake.chmod(0o755)

        with journal.open() as handle:
            saved = json.load(handle)
        activated = m.finalize_plan(
            m.load_plan(manifest),
            saved,
            journal,
            executable=str(fake),
            runtime_home=self.root,
        )
        self.assertEqual(activated["status"], "activated")
        self.assertEqual(activated["activation"]["threads"], 1)
        self.assertEqual(activated["activation"]["resumed"], 1)
        self.assertEqual(activated["activation"]["named"], 1)

        with journal.open() as handle:
            saved = json.load(handle)
        with self.assertRaises(m.MigrationError):
            m.rollback(saved, journal)
        self.assertTrue((self.target / rel).is_file())

    def test_distinct_threads_with_same_target_path_fail_closed(self):
        for profile, tid, marker_value in [("a", "t1", "one"), ("b", "t2", "two")]:
            root = self.legacy / profile
            rel = "sessions/2026/01/01/rollout-collision.jsonl"
            rollout(root, rel, tid, marker=marker_value)
            make_state(root, [(tid, rel, 10, "legacy", None, 0, None, None)])
        with self.assertRaises(m.MigrationError):
            m.build_plan(self.legacy, self.target)

    def test_apply_and_rollback_copy_only_rollout(self):
        source = self.legacy / "a"
        rel = "sessions/2026/01/01/rollout-t1.jsonl"
        rollout(source, rel, "t1")
        make_state(source, [("t1", rel, 10, "legacy", None, 0, None, None)])
        plan = m.build_plan(self.legacy, self.target)
        manifest = self.root / "plan.json"
        journal = self.root / "journal.json"
        m.write_plan(manifest, plan)
        loaded = m.load_plan(manifest)
        result = m.apply_plan(loaded, journal)
        self.assertEqual(result["status"], "applied")
        self.assertTrue((self.target / rel).is_file())
        self.assertFalse((self.target / "state_5.sqlite").exists())

        with journal.open() as handle:
            j = json.load(handle)
        m.rollback(j, journal)
        self.assertFalse((self.target / rel).exists())
        self.assertTrue((source / rel).is_file())


if __name__ == "__main__":
    unittest.main()
