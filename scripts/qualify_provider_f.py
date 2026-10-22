#!/usr/bin/env python3
"""Bounded F combined-route batch; local observations never invoke a supervisor model.

All task submissions have exclusive intent, explicit finite budgets and immutable
inputs. Uncertain delivery is retained without replay. Only the final batch is
independently audited; local waits and approval replies are protocol handling.
"""
import argparse
import concurrent.futures
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import struct
import subprocess
import sys
import termios
import time
import traceback
import urllib.request

SOURCE = Path(__file__).resolve().parents[1]
if not (SOURCE / "scripts").is_dir():
    SOURCE = Path(json.loads((Path(__file__).parent / "plan.json").read_text())["repository"])
sys.path.insert(0, str(SOURCE / "scripts"))
from qualify_provider_chat_entry import InputClient, control, native_work, observer_argv, observer_env, cli_final_visible
from d3_notify import send as notify


def stamp():
    return {"UTC": datetime.datetime.now(datetime.timezone.utc).isoformat(), "monotonic_ns": time.monotonic_ns()}


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value, exclusive=False):
    with Path(path).open("x" if exclusive else "w") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.flush()
        os.fsync(stream.fileno())


def input_completed(value):
    return value.get("complete") is True and (value.get("input") or {}).get("phase") == "completed"


def creation_call_id(root, name):
    # Session creation keys are service-wide. A separate owned run must not
    # reuse an earlier task name's key with a different workspace/payload.
    namespace = hashlib.sha256(os.fsencode(root.resolve())).hexdigest()[:24]
    return "f-create-" + namespace + "-" + name


def require_host_capabilities(observation, required):
    capabilities = (observation.get("host") or {}).get("capabilities") or {}
    for name in required:
        assert capabilities.get(name) is True, "required_host_capability_missing:" + name


def followup_branch(pages):
    """Use a saved public final as an explicit branch, preserving hidden siblings."""
    for page in reversed(pages):
        for message in reversed(page["messages"]):
            if (message.get("role") == "assistant" and message.get("end_turn") is True
                    and message.get("unavailable") is None and message.get("content") is not None
                    and message.get("channel") in (None, "final")):
                return message["id"]
    raise AssertionError("settled_public_followup_branch_unavailable")


class Batch:
    def __init__(self, root):
        self.root = root
        self.info = json.loads((root / "info.json").read_text())
        self.env = json.loads((root / "environment-private.json").read_text())
        self.plan = json.loads((root / "plan.json").read_text())
        self.deadline = time.monotonic() + self.plan["max_elapsed_seconds"]
        self.client = InputClient(root)
        self.observers = {}
        self.steps = []
        self.works = {}
        self.expected_stops = set()
        self.control_observations = 0

    def event(self, kind, **fields):
        with (self.root / "events.jsonl").open("a") as stream:
            stream.write(json.dumps({"kind": kind, **stamp(), **fields}, ensure_ascii=False) + "\n")

    def check(self, force=False):
        wall_deadline = self.plan.get("deadline_unix_ms")
        if time.monotonic() >= self.deadline or (isinstance(wall_deadline, int) and time.time() * 1000 >= wall_deadline):
            raise RuntimeError("batch_deadline_exhausted_no_replay")
        manifest_path = self.root / "sealed-inputs.json"
        def identity(path):
            value = path.stat()
            return (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns)
        metadata = identity(manifest_path)
        if not hasattr(self, "_sealed_manifest"):
            raw = manifest_path.read_bytes()
            self._sealed_manifest = json.loads(raw)
            self._sealed_manifest_digest = hashlib.sha256(raw).hexdigest()
            self._sealed_metadata = {}
            self._manifest_metadata = metadata
        elif force or metadata != self._manifest_metadata:
            if hashlib.sha256(manifest_path.read_bytes()).hexdigest() != self._sealed_manifest_digest:
                raise RuntimeError("sealed_manifest_changed")
            self._manifest_metadata = metadata
        for name, expected in self._sealed_manifest.items():
            path = self.root / name
            metadata = identity(path)
            if force or self._sealed_metadata.get(name) != metadata:
                if sha(path) != expected:
                    raise RuntimeError("sealed_input_changed")
                self._sealed_metadata[name] = metadata

    def current_records(self):
        excluded = set(self.plan.get("completed_prior_work", {})) | set(self.plan.get("preflight_stopped_work", []))
        return [record for record in native_work(self.root) if record["id"] not in excluded
            and not (record.get("phase") in ("completed", "stopped") and record["id"] in getattr(self, "observers", {}))]

    def check_progress(self, snapshot):
        limit = self.plan.get("max_no_progress_seconds")
        if limit is None:
            return
        records = []
        excluded = set(self.plan.get("completed_prior_work", {})) | set(self.plan.get("preflight_stopped_work", []))
        for record in native_work(self.root):
            if record["id"] in excluded:
                continue
            paths = (Path(record["execution"]["home"]) / "sessions").rglob("*" + record["thread"] + ".jsonl")
            records.extend((str(path), path.stat().st_size, path.stat().st_mtime_ns) for path in paths)
        # Snapshot file timestamps and stream flags are deliberately excluded.
        # Activity only bounds observation; it never proves task completion.
        signature = hashlib.sha256(json.dumps([snapshot.get("messages", []), sorted(records)], sort_keys=True).encode()).hexdigest()
        now = time.monotonic()
        previous = getattr(self, "_progress", None)
        if previous is None or previous[0] != signature:
            self._progress = (signature, now)
        elif now - previous[1] >= limit:
            raise RuntimeError("source_and_native_activity_stalled_preserved_no_replay")

    def rpc(self, name, args):
        if self.plan.get("agent_fixture"):
            from qualify_provider_f_agents import validate_control_arguments
            declarations = json.loads((self.root / "fixture-tools.json").read_text())
            validate_control_arguments(declarations, name, args)
        reading = (name == "codex_chats" and args.get("action") == "read"
            or name == "codex_sessions" and args.get("action") == "chat_result")
        for attempt in range(3 if reading else 1):
            self.check()
            self.control_observations += 1
            assert self.control_observations <= self.plan["max_control_observations"], "control_observation_budget_exhausted"
            evidence = self.root / f"control-{self.control_observations:04d}-private.json"
            try:
                return control(self.info, self.env, name, args, evidence_path=evidence)
            except Exception:
                self.event("control_failed_preserved", tool=name, action=args.get("action"), observation=self.control_observations)
                cause = None
                if evidence.exists():
                    try:
                        row = json.loads(evidence.read_text())
                        cause = json.loads(row["content"][0]["text"]).get("error")
                    except (ValueError, TypeError, KeyError, IndexError):
                        pass
                if not reading or cause != "app_bridge_operation_failed" or attempt == 2:
                    raise
                self.event("fresh_read_after_failed_observation", tool=name, action=args.get("action"), failed_observation=self.control_observations)
                time.sleep(.5 * (attempt + 1))

    def session(self, **args):
        return self.rpc("codex_sessions", args)

    def observe(self, work, state=None):
        if work["work_id"] in self.plan.get("completed_prior_work", {}):
            return
        record = next(r for r in native_work(self.root) if r["id"] == work["work_id"])
        if not record.get("native_turn"):
            return
        # Remote TUI resume needs the native claim's already loaded config.
        # Status is passive; never load an old native thread just for observing.
        if state is None:
            state = self.rpc("codex_status", {"work_id":record["id"]})
        if state["native_status"]["type"] == "notLoaded":
            return
        binding = self.root / (work["work_id"] + "-cli-binding.json")
        prior = self.observers.get(work["work_id"])
        if prior:
            process, fd = prior
            previous = json.loads(binding.read_text())
            if process.poll() is None and previous["execution"] == record["execution"]:
                return
            self.drain()
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            os.close(fd)
            del self.observers[work["work_id"]]
            binding.rename(binding.with_name(binding.stem + "-prior-" + str(time.monotonic_ns()) + ".json"))
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 180, 0, 0))
        argv = observer_argv(self.info, record)
        process = subprocess.Popen(argv, cwd=record.get("workspace") or self.info["workspace"], env=observer_env(self.env, record),
            stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        self.observers[work["work_id"]] = (process, master)
        save(self.root / (work["work_id"] + "-cli-binding.json"), {"argv": argv, "execution": record["execution"], "thread_id": record["thread"]})

    def drain(self):
        for work_id, (_, fd) in self.observers.items():
            while select.select([fd], [], [], 0)[0]:
                try:
                    chunk = os.read(fd, 1024 * 1024)
                except OSError:
                    break
                if not chunk:
                    break
                with (self.root / (work_id + "-original-cli.txt")).open("ab") as stream:
                    stream.write(chunk)

    def approve(self, work, status):
        block = (status.get("managed_lane") or {}).get("host_block") or (status.get("host_observation") or {}).get("waiting_for")
        if block == "mcp_host_confirmation_required":
            token = subprocess.check_output([self.info["binary"], "token", "--state", self.info["state"]], env=self.env, text=True).strip()
            request = urllib.request.Request("http://127.0.0.1:" + str(self.info["port"]) + "/__approve", data=b"",
                headers={"Authorization": "Bearer " + token, "X-Codex-Thread-Id": work["thread_id"]})
            with urllib.request.urlopen(request, timeout=30) as response:
                approved = json.load(response)
            self.event("original_app_confirmation_replied", work_id=work["work_id"], confirmed=approved.get("app_approval_requested"))

    def created(self, name, route="mcp", workspace=None, dependencies=None, parent=None):
        args = {"action": "create", "call_id": creation_call_id(self.root, name), "reasoning": "managed", "return_path": route,
            "workspace": str(workspace or self.root / ("w-" + name)), "role": name,
            "budget": self.plan["assignment_budget"]}
        if dependencies:
            args["dependencies"] = [d["work_id"] for d in dependencies]
        if parent:
            args.update(parent_work_id=parent["work_id"], parent_controller=parent["controller"])
        work = self.session(**args)
        self.works[name] = work
        save(self.root / (name + "-binding-private.json"), work, True)
        return work

    def start(self, name, work, task):
        intent = {"name": name, "work_id": work["work_id"], "task_sha256": hashlib.sha256(task.encode()).hexdigest(), **stamp()}
        save(self.root / (name + "-intent.json"), intent, True)
        result = self.session(action="start", work_id=work["work_id"], controller=work["controller"], call_id="f-start-" + name, task=task)
        work["controller"] = result["controller"]
        self.event("task_started", name=name, work_id=work["work_id"])
        self.observe(work)
        return intent

    def wait(self, name, work, intent):
        while True:
            self.check()
            self.drain()
            state = self.session(action="wait", work_id=work["work_id"], max_wait_seconds=3)
            status = state["status"]
            self.approve(work, status)
            if state["wait_reason"] == "completed":
                assert not status["pending_calls"] and not status["human_requests"], "completed_with_pending_native_calls"
                end = stamp()
                row = {"name": name, "work_id": work["work_id"], "thread_id": work["thread_id"], "started": intent,
                    "ended": end, "elapsed_seconds": (end["monotonic_ns"] - intent["monotonic_ns"]) / 1e9}
                self.steps.append(row)
                save(self.root / (name + "-complete-private.json"), state)
                self.event("task_completed", **row)
                return state
            if state["wait_reason"] in ("attention_required", "human_request"):
                save(self.root / (name + "-attention-private.json"), state)
                raise RuntimeError("original_native_attention_required_no_replay")

    def managed(self, name, task, route="mcp", workspace=None, dependencies=None, parent=None):
        work = self.created(name, route, workspace, dependencies, parent)
        self.wait(name, work, self.start(name, work, task))
        return work

    def active_server_stop(self):
        """One declared cancellation, after an actual native marker and server stream.

        These are local protocol observations, not intermediate quality reviews.
        A missed stream is a failed qualification, never fabricated Stop proof.
        """
        work = self.created("server-stop")
        intent = self.start("server-stop", work, (self.root / "task-server-stop.txt").read_text())
        until = min(self.deadline, time.monotonic() + self.plan["stop_probe_timeout_seconds"])
        observed = None
        while time.monotonic() < until:
            self.check()
            self.drain()
            status = self.rpc("codex_status", {"work_id": work["work_id"]})
            self.approve(work, status)
            conversation = (status.get("managed_lane") or {}).get("conversation_id")
            if conversation and (self.root / "w-server-stop/F_STOP_READY").is_file():
                lane = self.client.call("owned_lane_status", {"conversation_id": conversation})
                if lane["streamActive"] and not lane["stopRequested"]:
                    server = self.client.call("server_status", {"conversation_id": conversation})
                    if server["status"] == "IS_STREAMING":
                        observed = {"lane": lane, "server": server, **stamp()}
                        break
            if status["completed"]:
                raise RuntimeError("active_server_stop_window_missed")
            time.sleep(.2)
        assert observed is not None, "active_server_stop_not_observed"
        save(self.root / "active-server-stop-before.json", observed, True)
        save(self.root / "active-server-stop-intent.json", {"work_id": work["work_id"], **stamp()}, True)
        stop = self.rpc("codex_stop", {"work_id": work["work_id"]})
        save(self.root / "active-server-stop-private.json", stop, True)
        assert stop["native_stop_confirmed"] and stop["host_stop_confirmed"] and stop["remaining_jobs"] == [], "active_server_stop_unconfirmed"
        after = self.client.call("owned_lane_status", {"conversation_id": conversation})
        server = self.client.call("server_status", {"conversation_id": conversation})
        save(self.root / "active-server-stop-after.json", {"lane": after, "server": server, **stamp()}, True)
        assert after["serverStopState"] == "terminal_observed" and not after["streamActive"] and not after["pending"], "local_cancel_is_not_server_stop"
        assert server["status"] in ("COMPLETE", "FAILURE"), "fresh_server_stop_not_terminal"
        self.expected_stops.add(work["work_id"])
        end = stamp()
        self.steps.append({"name": "active-server-stop", "work_id": work["work_id"], "started": intent, "ended": end,
            "elapsed_seconds": (end["monotonic_ns"] - intent["monotonic_ns"]) / 1e9, "expected_cancellation": True})

    def evaluate(self, name, work, findings):
        history = self.rpc("codex_history", {"work_id": work["work_id"]})
        return self.session(action="evaluate", work_id=work["work_id"], controller=work["controller"],
            call_id="f-evaluate-" + name, cursor=history["cursor"], decision="accepted", text=findings)

    def source_wait(self, name, marker):
        while True:
            self.check()
            self.drain()
            snap = self.client.call("snapshot")
            save(self.root / (name + "-source-snapshot-private.json"), snap)
            if snap.get("stopped") or (snap.get("localStreamStopped") and "aborted" in snap.get("terminals", [])):
                raise RuntimeError("source_stopped_before_final_receipt_preserved_no_replay")
            if snap.get("error") == "mcp_host_confirmation_required":
                self.client.call("approve")
            elif snap.get("error"):
                raise RuntimeError("source_chat_error_no_replay")
            self.observe_current()
            self.check_progress(snap)
            if snap.get("accepted") and not snap.get("pending") and not snap.get("streamActive"):
                finals = ["\n".join(v for v in m.get("content", {}).get("parts", []) if isinstance(v, str))
                    for m in snap.get("messages", []) if m.get("author", {}).get("role") == "assistant" and m.get("end_turn") is True]
                if any(marker in value.splitlines() for value in finals):
                    return self.client.call("saved", {"title": self.plan.get("phase_label", "F") + "-" + name})
                if "done" in snap.get("terminals", []):
                    raise RuntimeError("source_finished_without_expected_marker")
            time.sleep(1)

    def observe_current(self):
        for record in self.current_records():
            if record.get("native_turn"):
                bound = {"work_id":record["id"],"thread_id":record["thread"]}
                status = self.rpc("codex_status", {"work_id":record["id"]})
                self.observe(bound, state=status)
                self.approve(bound, status)
                if self.plan.get("agent_fixture"):
                    from qualify_provider_f_agents import observe as observe_fixture
                    observe_fixture(self, record, status)

    def wait_for_saved_retry(self, seconds, input_id):
        until = min(self.deadline, time.monotonic() + max(2, seconds))
        while time.monotonic() < until:
            self.check()
            self.drain()
            # Local owned-lane state only: no saved conversation GET here.
            snapshot = self.client.call("saved_snapshot", {"input_id":input_id})
            if snapshot.get("error") == "mcp_host_confirmation_required":
                self.client.call("approve_saved", {"input_id":input_id})
            self.observe_current()
            time.sleep(min(5, max(0, until-time.monotonic())))

    def source_task(self, name, task, marker, step):
        started = stamp()
        save(self.root / (name + "-source-intent.json"), started, True)
        outcome = "submission_or_observation_failed"
        try:
            snapshot = self.client.call("snapshot")
            if snapshot.get("source_released") is True:
                self.client.call("new_source")
            self.client.call("submit", {"step": step, "text": task})
            result = self.source_wait(name, marker)
            save(self.root / (name + "-source-private.json"), result)
            outcome = "saved_result_observed"
            return result
        finally:
            ended = stamp()
            self.steps.append({"name": name, "started": started, "ended": ended, "outcome": outcome,
                "elapsed_seconds": (ended["monotonic_ns"] - started["monotonic_ns"]) / 1e9})

    def read(self, conversation, branch=None):
        pages, cursor = [], None
        while True:
            args = {"action": "read", "conversation_id": conversation, "limit": 50}
            if branch is not None:
                args["branch"] = branch
            if cursor:
                args["after"] = cursor
            page = self.rpc("codex_chats", args)
            pages.append(page)
            cursor = page["cursor"]
            if page["complete"]:
                return pages

    def close(self):
        self.drain()
        for process, fd in self.observers.values():
            # Owned read-only terminal clients only; canonical backend stays alive.
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            os.close(fd)
        self.client.close()


def audit(root, batch):
    """All concrete output/CLI/native/saved comparisons run after the final task."""
    reports = []
    for record in native_work(root):
        if record["id"] in batch.plan.get("preflight_stopped_work", []):
            proof = json.loads((root / "managed-empty-native-preflight-private.json").read_text())
            assert proof["work_id"] == record["id"] and proof["model_requests"] == 0 and proof["native_turns"] == 0
            assert proof["stop"]["native_stop_confirmed"] and not proof["stop"]["remaining_jobs"]
            assert record.get("native_turn") is None and not record["pending"], "preflight_work_started_or_unsettled"
            workspace = Path(record["workspace"])
            assert not subprocess.check_output(["git", "status", "--porcelain"], cwd=workspace, env=batch.env)
            reports.append({"work_id": record["id"], "preflight_only": True, "model_requests": 0,
                "current_status_inferred": False, "proof": str(root / "managed-empty-native-preflight-private.json")})
            continue
        status = batch.rpc("codex_status", {"work_id": record["id"]})
        prior = batch.plan.get("completed_prior_work", {}).get(record["id"])
        if prior is not None:
            assert prior["passed"] and status["native_status"]["type"] in ("idle", "notLoaded") and not status["pending_calls"] and not status["human_requests"]
            for name, digest in prior["workspace_files"].items():
                assert sha(Path(record["workspace"]) / name) == digest, "completed_prior_workspace_changed"
            reports.append({"work_id":record["id"],"reused_completed_proof":prior})
            continue
        if record["id"] in batch.expected_stops:
            assert record["phase"] == "stopped" and not status["pending_calls"] and not status["human_requests"], "expected_stop_not_quiescent"
            history = batch.rpc("codex_history", {"work_id": record["id"]})
            save(root / (record["id"] + "-history-private.json"), history)
            assert (Path(record["workspace"]) / "F_STOP_READY").is_file()
            cli = root / (record["id"] + "-original-cli.txt")
            assert cli.is_file() and cli.stat().st_size > 0, "original_cli_missing"
            reports.append({"work_id": record["id"], "expected_cancellation": True, "server_stop_proof": str(root / "active-server-stop-after.json"), "original_cli": str(cli)})
            continue
        assert status["completed"], "native_work_not_completed"
        history = batch.rpc("codex_history", {"work_id": record["id"]})
        save(root / (record["id"] + "-history-private.json"), history)
        messages = json.dumps(history, ensure_ascii=False)
        assert messages and not status["pending_calls"] and not status["human_requests"]
        workspace = Path(record.get("workspace") or batch.info["workspace"])
        suite = subprocess.run([sys.executable, "-B", "-m", "unittest", "discover", "-v"], cwd=workspace, env=batch.env, capture_output=True, timeout=40)
        (root / (record["id"] + "-tests.log")).write_bytes(suite.stdout + suite.stderr)
        assert suite.returncode == 0, "final_project_suite_failed"
        diff = subprocess.run(["git", "diff", "--check"], cwd=workspace, env=batch.env, capture_output=True)
        assert diff.returncode == 0 and not diff.stdout and not diff.stderr
        check = subprocess.run([sys.executable, "-B", "-c", """import json,subprocess,sys
from ledger import total
assert total([10**400,-10**400,7])==7
assert total(iter([2,3]))==5
for invalid in [[True],[1.5],["2"]]:
 try: total(invalid)
 except ValueError: pass
 else: raise AssertionError("invalid total accepted")
p=subprocess.run([sys.executable,"-B","ledger.py"],input="[2,3,-1]",text=True,capture_output=True)
assert p.returncode==0 and json.loads(p.stdout)=={"total":4}
for invalid in ["[true]","{","{}"]:
 p=subprocess.run([sys.executable,"-B","ledger.py"],input=invalid,text=True,capture_output=True)
 assert p.returncode==2 and not p.stdout.strip() and "Traceback" not in p.stderr
"""], cwd=workspace, env=batch.env, capture_output=True, timeout=40)
        (root / (record["id"] + "-independent-contract.log")).write_bytes(check.stdout + check.stderr)
        assert check.returncode == 0, "independent_development_contract_failed"
        if workspace.name == "w-direct":
            check = subprocess.run([sys.executable, "-B", "-c", """from ledger import apply_batch
changes=[-2,5];assert apply_batch(4,changes)==7 and changes==[-2,5]
for balance,values in [(1,[-2,3]),(True,[]),(1,[False]),(-1,[]),(1.0,[])]:
 try: apply_batch(balance,values)
 except ValueError: pass
 else: raise AssertionError("invalid batch accepted")
"""], cwd=workspace, env=batch.env, capture_output=True, timeout=30)
            (root / "independent-followup-contract.log").write_bytes(check.stdout + check.stderr)
            assert check.returncode == 0, "independent_followup_contract_failed"
        patch = subprocess.run(["git", "diff", "--binary", "HEAD"], cwd=workspace, env=batch.env, capture_output=True, check=True)
        (root / (record["id"] + "-diff.patch")).write_bytes(patch.stdout)
        assert (workspace / "test_original.py").read_bytes() == (root / "baseline/test_original.py").read_bytes()
        cli = root / (record["id"] + "-original-cli.txt")
        assert cli.exists() and cli.stat().st_size > 0, "original_cli_missing"
        # Canonical public final text must actually occur in its original terminal.
        finals = [row["item"].get("text", "") for row in history["items"]
            if row["item"].get("type") == "agentMessage" and row["item"].get("phase") == "final_answer"]
        assert finals, "canonical_final_missing"
        assert all(cli_final_visible(cli.read_text(errors="replace"), f) for f in finals), "canonical_final_not_visible_in_original_cli"
        lane = status.get("managed_lane") or {}
        sources = []
        for conversation in lane.get("previous_chats", []) + ([lane["conversation_id"]] if lane.get("conversation_id") else []):
            pages = batch.read(conversation)
            save(root / (conversation + "-read-private.json"), pages)
            sources.append({"conversation_id": conversation, "messages": sum(len(p["messages"]) for p in pages)})
        reports.append({"work_id": record["id"], "native_completed": True, "tests_passed": True, "original_cli": str(cli), "sources": sources})
    return reports


def run(root):
    save(root / "run-started.json", {"pid": os.getpid(), **stamp()}, True)
    batch = None
    result = {"automated_gates": False, "accepted_cases": [], "started": stamp(), "supervisory_model_calls_between_tasks": 0,
        "billing_observation": "unknown", "scope": "F combined-route live batch; full F acceptance requires independent final review and remaining fault/input cases"}
    try:
        batch = Batch(root)
        batch.event("batch_started")
        batch.check()
        batch.client.call("init")
        if batch.plan.get("required_host_capabilities"):
            require_host_capabilities(batch.rpc("codex_host", {"action": "status"}), batch.plan["required_host_capabilities"])
        if batch.plan.get("agent_fixture"):
            from qualify_provider_f_agents import perform
            result["final_audit"] = perform(root, batch)
            result["control_observations"] = batch.control_observations
            result["automated_gates"] = True
            return
        general_first = bool(batch.plan.get("resume_source") and batch.plan.get("general_first"))
        if general_first:
            # This independent route has never started in the earlier failed
            # continuations. Exercise it before revisiting their follow-up.
            batch.source_task("general-mcp", (root / "task-general.txt").read_text(), "F_GENERAL_COMPLETE", 2)
        if batch.plan.get("active_server_stop") and not batch.plan.get("resume_source"):
            batch.active_server_stop()
        task = (root / "task-development.txt").read_text()
        if batch.plan.get("resume_source"):
            direct = {"conversation_id": batch.plan["resume_source"]["conversation_id"]}
            batch.observe(batch.plan["resume_source"])
        else:
            batch.managed("managed-mcp", task, workspace=root / "w-managed-mcp")
            batch.managed("managed-local", task, route="local_function", workspace=root / "w-managed-local")
            direct = batch.source_task("direct-mcp", (root / "task-direct.txt").read_text(), "F_DIRECT_COMPLETE", 1)
        batch.client.call("release_source")
        all_pages = batch.read(direct["conversation_id"])
        save(root / "direct-source-before-private.json", all_pages)
        branch = batch.plan.get("followup_source_branch") or followup_branch(all_pages)
        pages = batch.read(direct["conversation_id"], branch=branch)
        save(root / "direct-read-before-private.json", pages)
        args = {"action": "chat_input", "conversation_id": direct["conversation_id"], "source_cursor": pages[-1]["cursor"],
            "task": (root / "task-followup.txt").read_text(), "call_id": batch.plan.get("saved_input_call_id", "f-existing-direct-followup"), "budget": batch.plan["assignment_budget"]}
        save(root / "old-chat-input-intent.json", {"args_sha256": hashlib.sha256(json.dumps(args, sort_keys=True).encode()).hexdigest(), **stamp()}, True)
        follow_started = stamp()
        submitted = batch.session(**args)
        save(root / "old-chat-input-private.json", submitted)
        while True:
            batch.check()
            follow = batch.session(action="chat_result", input_id=submitted["input_id"], max_wait_seconds=3)
            batch.drain()
            if follow.get("observation_unavailable"):
                batch.event("saved_result_observation_unavailable", input_id=submitted["input_id"], cause=follow["observation_unavailable"])
                save(root / "saved-read-diagnostics-private.json", batch.client.call("read_diagnostics"))
                # Yield locally while other native/protocol observations below
                # continue. No new input, supervisor turn or budget reset.
                delay = follow["observation_unavailable"].get("retry_after_seconds", 5)
                if not isinstance(delay, (int, float)) or isinstance(delay, bool) or not 0 <= delay <= 86400:
                    raise RuntimeError("invalid_saved_read_retry_time")
                batch.wait_for_saved_retry(delay, submitted["input_id"])
            if (follow.get("observation") or {}).get("error") == "mcp_host_confirmation_required":
                batch.client.call("approve_saved", {"input_id": submitted["input_id"]})
            elif (follow.get("observation") or {}).get("error"):
                raise RuntimeError("existing_chat_input_error_preserved_no_replay")
            if input_completed(follow):
                break
            batch.observe_current()
        follow_ended = stamp()
        batch.steps.append({"name": "same-chat-native-followup", "started": follow_started, "ended": follow_ended,
            "elapsed_seconds": (follow_ended["monotonic_ns"] - follow_started["monotonic_ns"]) / 1e9})
        save(root / "old-chat-result-private.json", follow)
        if not general_first:
            batch.client.call("new_source")
            batch.source_task("general-mcp", (root / "task-general.txt").read_text(), "F_GENERAL_COMPLETE", 2)
        batch.client.call("release_source")
        # One terminal flush at batch end; no quality review occurred between tasks.
        until = time.monotonic() + 2
        while time.monotonic() < until:
            batch.drain()
            time.sleep(.05)
        result["control_observations"] = batch.control_observations
        result["final_audit"] = audit(root, batch)
        batch.check(force=True)
        result["steps"] = batch.steps
        result["automated_gates"] = True
    except (Exception, KeyboardInterrupt) as error:
        (root / "failure-traceback-private.txt").write_text(traceback.format_exc())
        result["failure"] = {"type": type(error).__name__, "message": str(error)[:300], "state": "preserved_no_uncertain_request_replayed"}
    finally:
        if batch:
            result["steps"] = batch.steps
            if isinstance(batch.control_observations, int):
                result["control_observations"] = batch.control_observations
            if not result["automated_gates"]:
                stops = []
                # A Stop already dispatched by this batch is observed, never resent.
                already_stopped = set(batch.expected_stops) | set(batch.plan.get("preflight_stopped_work", [])) | set(batch.plan.get("completed_prior_work", {}))
                if (root / "active-server-stop-intent.json").exists():
                    already_stopped.add(json.loads((root / "active-server-stop-intent.json").read_text())["work_id"])
                for record in native_work(root):
                    if record["id"] in already_stopped:
                        continue
                    if record.get("phase") == "stop_requested":
                        stops.append({"work_id":record["id"],"state":"stop_requested_preserved","native_stop_requested":False})
                        continue
                    if record.get("phase") in ("completed", "stopped") and not record.get("pending"):
                        stops.append({"work_id":record["id"],"state":record["phase"] + "_preserved","native_stop_requested":False})
                        continue
                    try:
                        observed = control(batch.info, batch.env, "codex_stop", {"work_id": record["id"]})
                        stops.append({"work_id": record["id"], "observation": observed})
                    except Exception as error:
                        stops.append({"work_id": record["id"], "state": "unknown", "type": type(error).__name__})
                save(root / "failure-owned-native-stops-private.json", stops)
                try:
                    save(root / "failure-owned-source-stops-private.json", batch.client.call("stop_inputs"))
                except Exception as error:
                    result["source_stop"] = {"state": "unknown", "type": type(error).__name__}
            try:
                snapshot = batch.client.call("snapshot")
                save(root / "final-input-snapshot-private.json", snapshot)
                if isinstance(snapshot.get("requests"), list):
                    result["app_requests"] = len(snapshot["requests"])
                    result["app_request_metadata"] = snapshot["requests"]
                batch.client.call("dispose")
            except Exception as error:
                result["cleanup"] = {"state": "preserved", "type": type(error).__name__}
            try:
                batch.close()
            except Exception as error:
                result["observer_cleanup"] = {"state": "preserved", "type": type(error).__name__}
        result["ended"] = stamp()
        result["elapsed_wall_seconds"] = (datetime.datetime.fromisoformat(result["ended"]["UTC"]) - datetime.datetime.fromisoformat(result["started"]["UTC"])).total_seconds()
        result["elapsed_monotonic_seconds"] = (result["ended"]["monotonic_ns"] - result["started"]["monotonic_ns"]) / 1e9
        save(root / "result.json", result)
        with (root / "turns.md").open("w") as stream:
            stream.write("# F batch timings\n\n")
            for row in result.get("steps", []):
                stream.write("## " + row["name"] + "\n\n" + str(row["elapsed_seconds"]) + " seconds\n\n")
        notify(root)


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["start", "run"])
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    if args.action == "start":
        assert (args.root / "notification-probe-ack.json").exists(), "same-session receipt not confirmed"
        save(args.root / "launch-intent.json", stamp(), True)
        with (args.root / "batch.log").open("ab") as output:
            p = subprocess.Popen([sys.executable, "-B", str(args.root / "run.py"), "run", str(args.root)], stdin=subprocess.DEVNULL,
                stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        save(args.root / "launched-process.json", {"pid": p.pid, **stamp()}, True)
        print(json.dumps({"started": True, "pid": p.pid, "root": str(args.root), "supervisor_monitoring": False}))
    else:
        run(args.root)


if __name__ == "__main__":
    main()
