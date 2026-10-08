#!/usr/bin/env python3
"""Native ordinary completion and memory exclusion using only synthetic roots.

Runs actual Core, Manager and adapted upstream with a loopback Responses fixture.
No credentials, installed state, remote model or real Android provider is used.
"""
import argparse
import http.server
import json
import os
from pathlib import Path
import queue
import shutil
import signal
import sqlite3
import subprocess
import tempfile
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / ".github/scripts"))
from rald3_preflight import parse_descriptor


class Rpc:
    def __init__(self, process):
        self.process = process
        self.messages = queue.Queue()
        self.events = []
        self.sequence = 0
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        for line in self.process.stdout:
            try:
                self.messages.put(json.loads(line))
            except ValueError:
                self.messages.put({"invalid": True})

    def pump(self, deadline):
        assert self.process.poll() is None, "native server exited"
        try:
            message = self.messages.get(timeout=max(0.01, min(0.2, deadline - time.monotonic())))
        except queue.Empty:
            return None
        assert "invalid" not in message, "invalid native protocol"
        if "method" in message:
            self.events.append(message)
        return message

    def request(self, method, params):
        self.sequence += 1
        identity = self.sequence
        self.process.stdin.write(json.dumps({"id": identity, "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            message = self.pump(deadline)
            if message and message.get("id") == identity:
                assert "error" not in message, f"native request failed: {method}"
                return message["result"]
        raise AssertionError(f"native request timeout: {method}")

    def wait(self, predicate, description):
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if predicate():
                return
            self.pump(deadline)
        raise AssertionError(description)


def qualify(core, generation, parent):
    with tempfile.TemporaryDirectory(prefix="nq", dir=parent) as temporary:
        root = Path(temporary)
        home, prefix = root / "h", root / "p"
        fields = parse_descriptor(generation / "generation.meta")
        identity = fields["generation_id"]
        assert ";permission_policy=" in fields["patch_report"]
        installed = home / ".local/lib/codex/core/generations" / identity
        shutil.copytree(generation, installed)
        assert (installed / "manager").is_file()
        for directory in [prefix / "bin", prefix / "etc/tls", home / ".codex", home / ".local/share/codex/core/config"]:
            directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        shutil.copy2(core, prefix / "bin/codex")
        (prefix / "etc/resolv.conf").write_text("nameserver 127.0.0.1\n")
        (prefix / "etc/tls/cert.pem").write_text("fixture\n")
        (home / ".local/share/codex/core/activation-state").write_text(f"format=codex-activation-state-v3\nupdate_key={'11' * 32}\ncurrent={identity}\ncurrent_key={'11' * 32}\nprevious_present=0\nprevious=\nprevious_key=\n")
        real_bin = Path(shutil.which("sh")).parent
        calls = root / "calls.jsonl"
        provider = prefix / "bin/termux-notification"
        provider.write_text(f"#!{shutil.which('python3')}\nimport json,sys\nwith open({str(calls)!r},'a') as f: f.write(json.dumps(sys.argv[1:])+'\\n')\n")
        provider.chmod(0o700)
        env = {"HOME": str(home), "PREFIX": str(prefix), "TMPDIR": str(parent), "CODEX_HOME": str(home / ".codex"), "PATH": f"{prefix / 'bin'}:{real_bin}", "SSL_CERT_FILE": str(real_bin.parent / "etc/tls/cert.pem")}
        model_calls = []

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                assert self.path == "/v1/responses"
                data = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                serialized = json.dumps(data)
                memory = "memory_consolidation" in serialized or any("memory_consolidation" in value for value in self.headers.values())
                model_calls.append("memory" if memory else "ordinary")
                text = "synthetic memory completion" if memory else "synthetic ordinary completion"
                item = {"id": "m1", "type": "message", "role": "assistant", "status": "completed", "phase": "final_answer", "content": [{"type": "output_text", "text": text, "annotations": []}]}
                events = [
                    {"type": "response.created", "response": {"id": "r1"}},
                    {"type": "response.output_item.added", "output_index": 0, "item": item},
                    {"type": "response.output_text.delta", "output_index": 0, "content_index": 0, "delta": text},
                    {"type": "response.output_item.done", "output_index": 0, "item": item},
                    {"type": "response.completed", "response": {"id": "r1", "status": "completed", "output": [item], "usage": {"input_tokens": 1, "output_tokens": 1, "total_tokens": 2}}},
                ]
                body = "".join(f"event: {event['type']}\ndata: {json.dumps(event)}\n\n" for event in events).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        config = f'''model = "fixture-model"
model_provider = "fixture"
check_for_update_on_startup = false
[features]
memories = false
[memories]
generate_memories = true
use_memories = true
consolidation_model = "fixture-model"
[projects."{root}"]
trust_level = "trusted"
[model_providers.fixture]
name = "Fixture"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
'''
        (home / ".codex/config.toml").write_text(config)
        configured = subprocess.run([str(prefix / "bin/codex"), "termux", "notify", "set", "--hooks", "Stop,UserInputRequest"], env=env, cwd=root, capture_output=True, timeout=10)
        assert configured.returncode == 0 and configured.stdout == b"saved\n"
        record = home / ".local/share/codex/manager/notifications/config-v1"
        record_before = record.read_bytes()
        stderr = open(root / "native.stderr", "w")
        process = subprocess.Popen([str(prefix / "bin/codex"), "app-server", "--listen", "stdio://"], env=env, cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr, text=True, start_new_session=True)
        rpc = Rpc(process)
        try:
            rpc.request("initialize", {"clientInfo": {"name": "synthetic-notify-proof", "version": "1"}, "capabilities": {"experimentalApi": True}})
            started = rpc.request("thread/start", {"cwd": str(root)})
            thread = started["thread"]["id"]
            assert started["sandbox"]["type"] == "dangerFullAccess"
            turn = rpc.request("turn/start", {"threadId": thread, "input": [{"type": "text", "text": "Synthetic ordinary completion proof.", "text_elements": []}]})["turn"]["id"]
            rpc.wait(lambda: any(event["method"] == "turn/completed" and event["params"]["turn"]["id"] == turn and event["params"]["turn"]["status"] == "completed" for event in rpc.events), "ordinary completion failed")
            rpc.wait(lambda: calls.exists() and len(calls.read_text().splitlines()) == 1, "ordinary notification missing")
            first_calls = calls.read_bytes()
            args = json.loads(first_calls)
            assert args[args.index("--content") + 1] == "synthetic ordinary completion"
            assert model_calls == ["ordinary"]
            memory_db = home / ".codex/memories_1.sqlite"
            state_db = home / ".codex/state_5.sqlite"
            assert memory_db.exists() and state_db.exists()
            with sqlite3.connect(state_db) as db:
                db.execute("UPDATE threads SET memory_mode='enabled' WHERE id=?", (thread,))
                assert db.execute("SELECT memory_mode FROM threads WHERE id=?", (thread,)).fetchone() == ("enabled",)
            with sqlite3.connect(memory_db) as db:
                now = int(time.time())
                db.execute("INSERT INTO stage1_outputs (thread_id,source_updated_at,raw_memory,rollout_summary,generated_at) VALUES (?,?,?,?,?)", (thread, now, "synthetic memory fact", "synthetic rollout summary", now))
            memory_root = home / ".codex/memories"
            memory_root.mkdir(exist_ok=True)
            (memory_root / "MEMORY.md").write_text("Synthetic memory.\n")
            (memory_root / "memory_summary.md").write_text("v1\nSynthetic memory summary.\n")
            second = rpc.request("thread/start", {"cwd": str(root), "config": {"features.memories": True}})
            assert second["thread"]["id"] != thread

            second_turn = rpc.request("turn/start", {"threadId": second["thread"]["id"], "input": [{"type": "text", "text": "Synthetic root turn triggers enabled memory startup.", "text_elements": []}]})["turn"]["id"]
            rpc.wait(lambda: any(event["method"] == "turn/completed" and event["params"]["turn"]["id"] == second_turn and event["params"]["turn"]["status"] == "completed" for event in rpc.events), "second ordinary completion failed")

            def succeeded():
                with sqlite3.connect(memory_db) as db:
                    row = db.execute("SELECT status,last_error FROM jobs WHERE kind='memory_consolidate_global' AND job_key='global'").fetchone()
                if row and row[0] == "failed":
                    raise AssertionError(f"native memory job failed: {row[1]}")
                return row == ("done", None)

            rpc.wait(succeeded, "native memory job did not succeed")
            with sqlite3.connect(memory_db) as db:
                assert db.execute("SELECT selected_for_phase2 FROM stage1_outputs WHERE thread_id=?", (thread,)).fetchone() == (1,)
                assert db.execute("SELECT last_success_watermark FROM jobs WHERE kind='memory_consolidate_global'").fetchone()[0] >= now
            assert sorted(model_calls) == ["memory", "ordinary", "ordinary"], "memory worker did not use actual native model path"
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                rpc.pump(deadline)
            deliveries = [json.loads(line) for line in calls.read_text().splitlines()]
            assert len(deliveries) == 2, "memory work emitted a completion notification"
            assert all(args[args.index("--content") + 1] == "synthetic ordinary completion" for args in deliveries)
            assert record.read_bytes() == record_before
            rendered = (home / ".local/share/codex/core/config/config.toml").read_text()
            assert 'notify = ["codex", "termux", "notify", "emit", "Stop"]' in rendered
            assert "hooks.Stop" not in rendered and "emit UserInputRequest" in rendered
            assert not (home / ".codex/auth.json").exists()
            print("PASS native ordinary completion: two root turns, one single-line Manager delivery each")
            print("PASS enabled native memory consolidation: successful job and real model fixture, zero extra delivery")
            print("PASS user-input matcher, Manager preferences, auth-free fixture and isolated public Core path")
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
            stderr.close()
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--generation", type=Path, required=True)
    parser.add_argument("--temp-parent", type=Path, required=True)
    args = parser.parse_args()
    assert all(path.is_absolute() for path in [args.core, args.generation, args.temp_parent])
    qualify(args.core, args.generation, args.temp_parent)
