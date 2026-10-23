#!/usr/bin/env python3
"""Prepare/run a bounded, unattended D3 trial. Preparing never submits inference.

An explicit start launches one detached local process. Completion/result/state
checks sequence frozen prompts; quality contracts run after all work ends.
Canonical append notifications replace repeated RPC/status polling. Ambiguous
submissions are preserved, never replayed by this harness.
"""
import argparse
import ctypes
import datetime
import fcntl
import hashlib
import json
import os
import platform
from pathlib import Path
import pty
import re
import resource
import selectors
import shlex
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import urllib.request

SOURCE = Path(__file__).resolve().parents[1]
if not (SOURCE / "scripts").is_dir():
    SOURCE = Path(json.loads((Path(__file__).parent / "plan.json").read_text())["repository"])
sys.path.insert(0, str(SOURCE / "scripts"))
from provider_qualification import explicit_input
from qualify_permission_lifecycle import fixture, rpc as native_rpc, server_paths


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def snapshot_diff_check(previous, project):
    result = subprocess.run(["git", "diff", "--no-index", "--check", str(previous), str(project)], capture_output=True)
    # --no-index implies --exit-code: 1 can mean an ordinary clean diff.
    # --check emits diagnostics for whitespace defects; errors also emit output.
    diagnostics = result.stdout + result.stderr
    assert result.returncode in (0, 1) and not diagnostics, diagnostics.decode(errors="replace")


def now():
    return {"UTC": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "monotonic_ns": time.monotonic_ns()}


def save(path, value, exclusive=False):
    if exclusive:
        with path.open("x") as stream:
            json.dump(value, stream, ensure_ascii=False, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
    else:
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2))
        temporary.replace(path)


def stopped_host_reconciled(state):
    """A released driver retains positive Stop proof across service restart."""
    return (not state["pending_calls"] and not state["human_requests"]
            and (state["host_state"] == "stop_confirmed"
                 or state["driver_phase"] == "released_after_stop"
                 and state["last_observed_host_state"] == "stop_confirmed"
                 and not state["host_observation_available"]))


def reconciled_resume(ready, thread, calls, returns):
    assert ready["thread_id"] == thread, "resume thread changed"
    assert calls.keys() == returns.keys(), "unreturned native effects; do not replay"
    assert ready["native_calls"] == ready["native_results"] == len(calls), "resume call counts changed"
    if calls:
        assert ready["original_call_ids"] == sorted(calls), "resume original calls changed"
        assert ready["original_results_sha256"] == digest_results(returns), "resume original results changed"


def digest_results(returns):
    return hashlib.sha256(json.dumps(returns, sort_keys=True).encode()).hexdigest()


def owned_fault_tunnel(case, observed):
    """A declared cut may end the process; require its admitted registry identity."""
    assert observed["profile_dir"] == str(case / "profiles"), "tunnel owner changed"
    admitted = json.loads((case / "tunnel-ready-private.json").read_text())
    assert admitted["process_running"] and admitted["ready"] and admitted["healthy"], "initial tunnel admission absent"
    for key in ("alias", "profile_dir", "tunnel_id", "config_path", "profile_path"):
        assert admitted.get(key) and observed.get(key) == admitted[key], "tunnel registry identity changed"
    assert observed["process"] == admitted["process"], "tunnel process registration changed"
    assert observed["process"]["target_kind"] == "command"
    assert observed["process"]["target_value"] == str(case / "mcp-stdio"), "tunnel command owner changed"
    return True


def tunnel_profile():
    path = explicit_input("TUNNEL_PROFILE")
    return json.loads(path.read_text())["control_plane"]


def repaired_tunnel(case, before, after):
    for key in ("alias", "profile_dir", "tunnel_id", "config_path", "profile_path"):
        assert after.get(key) == before[key], "repaired tunnel destination/owner changed"
    assert after["process"]["target_kind"] == "command"
    assert after["process"]["target_value"] == str(case / "mcp-stdio"), "repaired tunnel command changed"
    for key, value in before["process"].items():
        if key != "started_at":
            assert after["process"].get(key) == value, "repaired tunnel registration changed"
    return after["process_running"] and after["ready"] and after["healthy"]


def reconnect_fault_tunnel(case, thread, results_digest):
    alias = json.loads((case / "tunnel-ready-private.json").read_text())["alias"]
    env = tunnel_environment()
    command = ["tunnel-client", "runtimes", "status", alias, "--json"]
    before = local_json(command, env)
    assert owned_fault_tunnel(case, before)
    profile = tunnel_profile()
    assert before["tunnel_id"] == profile["tunnel_id"], "tunnel destination changed"
    save(case / "recovery-tunnel-before-private.json", before)
    save(case / "recovery-tunnel-intent.json", {"thread": thread,
         "original_results_sha256": results_digest, "effect_count": 1, **now()}, exclusive=True)
    # The injected EOF can already have ended the owner. No Stop is needed then.
    if before["process_running"]:
        try:
            stopped = local_json(["tunnel-client", "runtimes", "stop", alias, "--json"], env)
        except AssertionError:
            observed = local_json(command, env)
            assert owned_fault_tunnel(case, observed)
            assert not observed["process_running"], "Stop unconfirmed; preserve without another Stop or Connect"
            stopped = {"reply_reconciled": True, "owned_process_stopped": True}
    else:
        stopped = {"already_stopped": True, "verified_admitted_owner": True}
    save(case / "recovery-tunnel-stop-private.json", stopped)
    connect_error = None
    try:
        connected = local_json(["tunnel-client", "runtimes", "connect", "--alias", alias,
            "--tunnel-id", profile["tunnel_id"], "--runtime-api-key", profile["api_key"],
            "--profile", "d2-" + case.name, "--profile-dir", str(case / "profiles"),
            "--mcp-command", str(case / "mcp-stdio"), "--json"], env)
        save(case / "recovery-tunnel-connect-private.json", connected)
    except AssertionError:
        # A lost local response does not justify repeating Connect. Observe its
        # exact destination/registration and readiness instead.
        connect_error = "connect_reply_unconfirmed"
    for delay in (0, .25, 1, 3):
        if delay:
            time.sleep(delay)
        after = local_json(command, env)
        save(case / "recovery-tunnel-after-private.json", after)
        if repaired_tunnel(case, before, after):
            assert after["process"].get("started_at") != before["process"].get("started_at"), "Connect has no new owner receipt"
            save(case / "recovery-tunnel-receipt.json", {"accepted": True,
                 "connect_reply_reconciled": bool(connect_error), "connect_requests": 1, **now()})
            return
    raise AssertionError("owned reconnect unconfirmed; preserve intent without repeating Connect")


def terminal_for_turn(row, turn):
    return (turn is not None and row["type"] == "event_msg"
            and row["payload"].get("type") in ("task_complete", "turn_aborted")
            and row["payload"].get("turn_id") == turn)


def completion_claimed(text, marker):
    return re.search(r"(?m)^\s*(?:#{1,6}\s+)?(?:\*\*|`)?" + re.escape(marker) + r"(?:\*\*|`)?\s*$", text) is not None


def verify_immutable_inputs(root, plan, manifest):
    paths = ["run.py", "contract.py", "fault_proxy.py", "notify.py", "manage.py", "backend.mjs",
             "refresh.mjs", "app_setup.mjs", "plan.json", "candidate/provider", "candidate/app_bridge.js"]
    paths.extend("baseline/" + path for path in plan["baseline"])
    for name in plan["order"]:
        paths.append(name + "/tasks.json")
        metadata = json.loads((root / name / "tasks.json").read_text())
        paths.extend(f"{name}/request-{step}.txt" for step in range(1, metadata["count"] + 1))
        paths.append(name + "/config/codex/providers/chatgpt.toml")
    assert all(path in manifest["files"] and sha(root / path) == manifest["files"][path] for path in paths), "sealed controller/product/task inputs changed during run"
    assert all(sha(SOURCE / path) == digest for path, digest in manifest["helper_inputs"].items()), "imported helper inputs changed during run"


def expected_result_cut(case, calls, returns):
    """Reconcile the exact durable effect, canonical return and withheld wire result."""
    assert calls.keys() == returns.keys(), "unreturned native call at declared cut"
    fired = json.loads((case / "fault-fired.json").read_text())
    assert fired["effect_count"] == 1 and fired["forwarded_to_Chat"] is False
    assert (case / "effects/fault.effects").read_text() == "fault\n"
    wire = case / "withheld-mcp-result-private.json"
    assert sha(wire) == fired["result_sha256"], "withheld original result changed"
    probes = []
    for identity, call in calls.items():
        if call.get("name", "").endswith("exec_command"):
            command = json.loads(call["arguments"]).get("cmd", "")
            if shlex.split(command) == ["python3", "-B", "probe.py", "--root", "../effects", "--label", "fault"]:
                probes.append(identity)
    assert len(probes) == 1, "effect probe must execute exactly once"
    result = json.loads(wire.read_text())["result"]
    assert result["isError"] is False
    original = json.loads(result["content"][0]["text"])
    if "result" in original:  # Operator adds native history around the original result.
        original = original["result"]
    identity, = probes
    if "call_id" in original:  # Original native return inside Operator history.
        assert original["call_id"] == identity
    else:  # Adapter native_tool wire receipt uses id/name.
        call = calls[identity]
        qualified = (call.get("namespace", "") + "." + call["name"]).lstrip(".")
        assert original["id"] == identity and original["name"] == qualified
    assert original["output"] == returns[identity]["output"]
    assert "D3_EFFECT_RETURNED_FAULT" in str(original["output"])
    return True


class AppendWatch:
    """Kernel-notified tree, incremental public canonical records, no status RPC."""
    def __init__(self, sessions):
        self.sessions = sessions
        sessions.mkdir(parents=True, exist_ok=True)
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.fd = self.libc.inotify_init1(os.O_NONBLOCK | os.O_CLOEXEC)
        if self.fd < 0:
            raise OSError(ctypes.get_errno(), "inotify_init1")
        self.watched = set()
        self.offsets = {}
        self.pending = {}
        self.add_directories()

    def add_directories(self):
        for path in [self.sessions, *self.sessions.rglob("*")]:
            if path.is_dir() and path not in self.watched:
                # MODIFY, CLOSE_WRITE, MOVED_TO, CREATE: append or new dated dir.
                if self.libc.inotify_add_watch(self.fd, os.fsencode(path), 0x2 | 0x8 | 0x80 | 0x100) < 0:
                    raise OSError(ctypes.get_errno(), "inotify_add_watch")
                self.watched.add(path)

    def read(self):
        try:
            os.read(self.fd, 65536)
        except BlockingIOError:
            pass
        self.add_directories()
        rows = []
        files = list(self.sessions.rglob("*.jsonl"))
        assert len(files) <= 1, "unexpected additional primary thread"
        for path in files:
            offset = self.offsets.get(path, 0)
            assert path.stat().st_size >= offset, "canonical record truncated"
            with path.open("rb") as stream:
                stream.seek(offset)
                data = stream.read()
                self.offsets[path] = stream.tell()
            chunks = (self.pending.get(path, b"") + data).split(b"\n")
            self.pending[path] = chunks.pop()
            rows.extend(json.loads(line) for line in chunks if line)
        return rows

    def close(self):
        os.close(self.fd)


def original_tests(text):
    start = text.index("mod original_tests {")
    depth = 0
    for i in range(text.index("{", start), len(text)):
        depth += (text[i] == "{") - (text[i] == "}")
        if depth == 0:
            return text[start:i + 1]
    raise AssertionError("original test module incomplete")


COMMON = """Stay in this disposable project and SAME primary native thread. Actual
effects must use advertised native Codex tools; no desktop executor, other model,
commit/push, package install or external dependency. Preserve AGENTS.md,
REQUIREMENTS.md, Cargo.toml, Cargo.lock, probe.py, original_tests and every prior
regression module byte for byte. Put each turn's NEW regressions in a NEW
src/test_*.rs module, included from main.rs with #[cfg(test)]. Run them red
before implementation, then cargo test --offline --locked, git diff --check and
actual diff review. Use CARGO_TARGET_DIR already set outside this project. Discover
the full native tool schemas as needed. If Operator controls are advertised,
attach/accept the current receipt before effects and codex_finish after verified
results; otherwise return the normal native final. Report concise Korean progress.
Do not create a new primary thread or deliberately replace the saved Chat.
"""

TASKS = [
    """D3_B1. Extend this existing Rust CLI with `transfer FROM TO POSITIVE_I64`.
Add Command::Transfer(String,String,i64). Missing source, insufficient funds,
nonpositive amount and destination overflow are errors. Self-transfer validates
source/funds but leaves balances unchanged. ledger::apply must apply an ENTIRE
batch atomically: any error leaves the original Ledger unchanged. Keep add and
sorted TSV output, line-numbered parse errors, stderr/exit2/no stdout on error.
Update README. No persistence or dry-run yet. Final marker D3_B1_COMPLETE.
""",
    """D3_B2. Add --state PATH, loading an existing strict UTF-8 TSV ledger and
applying the complete stdin batch to it. Missing file starts empty. Duplicate
accounts, malformed records, negative/out-of-range balances are errors. On success
print the usual sorted report AND atomically replace PATH, using an owned unique
temp in its parent after validation/serialization. Errors preserve target bytes,
remove only owned temp, stderr/exit2/no stdout/panic. Default without --state stays
unchanged. Update README. No dry-run yet. Final marker D3_B2_COMPLETE.
""",
    """D3_C1. Add --dry-run, composable with --state PATH in either argument order:
validate and print the projected sorted report, but never create/modify the state
file or leave a temp. Normal commit stays unchanged. Preserve all error behavior.
Also run this owned native probe EXACTLY ONCE, through exec_command with
yield_time_ms=1000: `python3 -B probe.py --root ../effects --label long --delay 8`.
Use write_stdin to collect its ORIGINAL session's eventual output; do not rerun it.
Require D3_EFFECT_RETURNED_LONG, then verify ../effects/long.effects has one line.
Update README, test dry-run/no-write cases. Final marker D3_C1_COMPLETE.
""",
]
FAULT_TASKS = [
    """D3_A1. Run EXACTLY ONCE `python3 -B probe.py --root ../effects --label fault`.
An explicitly owned trial proxy will close the MCP stdio transport AFTER native
execution/result exists but BEFORE forwarding that original MCP result to Chat.
Never rerun the command or infer rollback from a missing response. Reconcile native
history/current results if accessible. Do not alter the probe or effects file.
If delivery fails, state what is known without inventing success. If genuinely
reconciled in this turn, final marker D3_A1_RECONCILED.
""",
    """D3_A2. Genuine follow-up after the owned result-delivery interruption. Stay
in the SAME primary native thread. Reconcile latest canonical calls/results and
current state. Read ../effects/fault.effects: it must contain exactly one `fault`
line. Do not execute the earlier command again, delete evidence or guess pending
effects. If fully reconciled, create recovery-note.txt with the known effect count
and original result marker D3_EFFECT_RETURNED_FAULT, using native apply_patch;
verify it and report D3_A2_COMPLETE. If reconciliation remains uncertain, explain
the actual uncertainty; never claim completion or silently change route.
""",
]


def task_rules(name, plan):
    rules = COMMON + "Completion markers must appear on their own line after verified success.\n"
    if name.startswith("fault") and plan.get("automatic_fault_recovery"):
        rules = rules.replace("Do not create a new primary thread or deliberately replace the saved Chat.",
            "Do not create a new primary thread. After A1, the local controller will reconcile the exact original native/wire result and one durable effect, reconnect the owned MCP tunnel once, and preserve the source Chat. Recovery may use a successor saved Chat on the SAME primary native thread. Historical commands must not be replayed.")
    return rules


def prepare(args):
    desktop_session = explicit_input("DESKTOP_SESSION")
    connector_reference = explicit_input("CONNECTOR_REFERENCE")
    frontend = explicit_input("FRONTEND")
    bun = explicit_input("BUN")
    source_environment = explicit_input("PRIOR_ENVIRONMENT")
    tunnel_alias = explicit_input("TUNNEL_ALIAS", path=False)
    tunnel_environment()
    tunnel_profile()
    stopped_work_proof = args.stopped_work_proof or explicit_input("STOPPED_WORK_PROOF")
    d2 = args.d2.resolve()
    frozen = json.loads((d2 / "corrected-candidate-inputs.json").read_text())
    assert all(sha(SOURCE / path) == digest for path, digest in frozen["source"].items()), "D2 input drift"
    root = Path(tempfile.mkdtemp(prefix="d3.", dir=os.environ["TMPDIR"]))
    root.chmod(0o700)
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "codex/acceptance" / root.name
    cache.mkdir(parents=True, mode=0o700)
    (cache / "root").write_text(str(root))
    (root / "candidate").mkdir()
    shutil.copy2(d2 / "candidate-corrected/codex-chatgpt-provider", root / "candidate/provider")
    shutil.copy2(d2 / "candidate-corrected/app_bridge.js", root / "candidate/app_bridge.js")
    assert sha(root / "candidate/provider") == frozen["binary_sha256"]
    for source, name in [(Path(__file__), "run.py"), (SOURCE / "scripts/d3_contract.py", "contract.py"),
                         (SOURCE / "scripts/d3_fault_proxy.py", "fault_proxy.py"),
                         (SOURCE / "scripts/d3_notify.py", "notify.py")]:
        shutil.copy2(source, root / name)
    shutil.copytree(SOURCE / "scripts/fixtures/d3", root / "baseline")
    subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=root / "baseline", check=True, capture_output=True)
    fault_only = getattr(args, "fault_only", False)
    order = (["fault_operator", "fault_adapter"] if fault_only else
             ["normal_adapter", "normal_operator", "fault_operator", "fault_adapter"])
    baseline = {str(p.relative_to(root / "baseline")): sha(p) for p in (root / "baseline").rglob("*") if p.is_file()}
    plan = {"version": 1, "repository": str(SOURCE), "prepared": now(), "d2": str(d2), "cache": str(cache),
            "desktop_session": str(desktop_session), "bun": str(bun),
            "candidate_sha256": frozen["binary_sha256"], "baseline": baseline, "order": order,
            "matched_comparison_order": order, "max_total_app_dispatches": 24, "negative_app_dispatches": 0,
            "expected_genuine_user_tasks": 10, "expected_app_requests": [10, 14],
            "expected_saved_chats": [4, 6], "boundary_status_query_limit": 6,
            "task_timeout_seconds": 1200, "supervisory_model_calls": 0,
            "fault": "owned MCP stdio EOF after durable native effect/result; not desktop or device loss",
            "deferred": ["real context saturation/compaction policy", "ChatGPT-first entry",
                         "wider approval/question/reviewer boundaries", "repeat speed ranking", "deployment/activation"]}
    if fault_only:
        plan.update(scope="A-only recovery qualification", qualification="unattended_from_start", automatic_fault_recovery=True,
                    expected_genuine_user_tasks=4, expected_app_requests=[4, 4],
                    expected_saved_chats=[4, 4], max_total_app_dispatches=6)
    save(root / "plan.json", plan)
    # Existing owner-aware service/connector primitives are frozen for this trial.
    manage = (d2 / "manage.py").read_text()
    old = "info['binary'],'mcp-stdio','--state',info['state']"
    new = "*([info['binary'],'mcp-stdio','--state',info['state']] if route=='prior-service' else [sys.executable,'-B',str(root/'fault_proxy.py'),'--root',str(d),'--',info['binary'],'mcp-stdio','--state',info['state']])"
    assert old in manage
    (root / "manage.py").write_text(manage.replace(old, new))
    backend = (d2 / "backend.mjs").read_text().replace(str(d2), str(root))
    backend = backend.replace("root+'/protocol.json'", "root+'/plan.json'")
    backend = backend.replace("termux.codex.d2.acceptance.v1", "termux.codex.d3.acceptance.v1")
    (root / "backend.mjs").write_text(backend)
    refresh = (SOURCE / "scripts/d3_refresh.mjs").read_text().replace("'../providers/chatgpt/desktop_cdp.mjs'", repr(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs")))
    (root / "refresh.mjs").write_text(refresh)
    app_setup = (SOURCE / "scripts/d3_app_setup.mjs").read_text().replace("'../providers/chatgpt/desktop_cdp.mjs'", repr(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs")))
    (root / "app_setup.mjs").write_text(app_setup)
    prior = root / "prior-service"
    prior.mkdir()
    prior_source = (args.prior_service or d2 / "restored-connection").resolve()
    previous = json.loads((prior_source / "info.json").read_text())
    previous["return_path"] = "mcp"
    save(prior / "info.json", previous)
    shutil.copy2(source_environment, prior / "environment-private.json")
    save(root / "prior-ownership.json", {"original": previous, "tunnel_profile_dir": str(prior_source / "profiles"), "tunnel_alias": tunnel_alias,
         "stopped_work_proof": str(stopped_work_proof)})
    core = Path(os.environ["PREFIX"]) / "bin/codex"
    for name in order:
        case = root / name
        case.mkdir()
        short = Path(tempfile.mkdtemp(prefix="r3.", dir=os.environ["TMPDIR"]))
        home, prefix, env = fixture(short, core, frontend)
        shutil.copytree(root / "baseline", case / "w")
        (case / "effects").mkdir()
        config = home / ".codex/config.toml"
        config.write_text(config.read_text() + '\n[projects.' + json.dumps(str(case / "w")) + ']\ntrust_level="trusted"\n')
        mode = "operator" if name.endswith("operator") else "model_adapter"
        info = {"route": name, "mode": mode, "return_path": "mcp", "state": str(case / "state"),
                "home": str(home / ".codex"), "workspace": str(case / "w"), "core": str(prefix / "bin/codex"),
                "runtime": str(home / ".local/lib/codex/core/generations/local-permission-fixture/runtime"),
                "fixture_root": str(short), "binary": str(root / "candidate/provider"),
                "desktop_session": str(desktop_session), "connector_reference": str(connector_reference),
                "bridge": str(root / "candidate/app_bridge.js"), "sha256": frozen["binary_sha256"]}
        for key, path in [("core_sha256", core), ("runtime_sha256", Path(info["runtime"])),
                          ("frontend_sha256", Path(info["runtime"]).parent / "helpers/2")]:
            info[key] = sha(path)
        save(case / "info.json", info)
        config = case / "config/codex/providers"
        config.mkdir(parents=True)
        (config / "chatgpt.toml").write_text('version=1\n[model]\ndefault="chatgpt-desktop/gpt-6"\nreasoning_effort="high"\n'
            '[transport]\nreturn_path="mcp"\nconnector_name="Termux Bridge"\n[execution]\nmode=' + json.dumps(mode) + '\n' +
            ''.join(key + '=' + json.dumps(info[key]) + '\n' for key in ("core", "home", "workspace")) +
            '\n[history]\nmode="project"\nproject_name="Termux Provider: D3"\ncreate_if_missing=false\n')
        env["XDG_CONFIG_HOME"] = str(case / "config")
        env["CARGO_TARGET_DIR"] = str(cache / "compiler" / name)
        env["CARGO_NET_OFFLINE"] = "true"
        save(case / "environment-private.json", env)
        for command in [["git", "init", "-q"], ["git", "add", "."],
                        ["git", "-c", "user.name=Owned trial", "-c", "user.email=owned@example.invalid", "commit", "-qm", "frozen D3 baseline"]]:
            subprocess.run(command, cwd=case / "w", env=env, check=True, capture_output=True)
        tasks = FAULT_TASKS if name.startswith("fault") else TASKS
        for index, task in enumerate(tasks, 1):
            (case / f"request-{index}.txt").write_text(task + "\n" + task_rules(name, plan))
        save(case / "tasks.json", {"kind": "fault" if name.startswith("fault") else "normal", "count": len(tasks),
                                    "prompt_hashes": [sha(case / f"request-{i}.txt") for i in range(1, len(tasks) + 1)]})
    infos = [json.loads((root / name / "info.json").read_text()) for name in order]
    for key in ("core_sha256", "runtime_sha256", "frontend_sha256"):
        assert len({v[key] for v in infos}) == 1
    save(root / "inputs.json", {"source_HEAD": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=SOURCE, text=True).strip(),
         "helper_inputs": {str(p.relative_to(SOURCE)): sha(p) for p in [
             SOURCE / "scripts/qualify_permission_lifecycle.py", SOURCE / "scripts/qualify_tasks.py", SOURCE / "scripts/provider_qualification.py",
             SOURCE / "providers/chatgpt/desktop_cdp.mjs"]},
         "qualified_D2_source": frozen, "files": {str(p.relative_to(root)): sha(p) for p in root.rglob("*")
          if p.is_file() and ".git" not in p.parts and "environment-private" not in p.name}})
    print(json.dumps({"root": str(root), "cache": str(cache), "model_requests": 0, "status": "prepared_not_started"}))


def seal(root):
    assert not (root / "launch-intent.json").exists() and not (root / "run-started.json").exists()
    project = json.loads((root / "project.json").read_text())
    assert project["name"] == "Termux Provider: D3" and project["compatible"]
    for source, name in [(SOURCE / "scripts/qualify_provider_d3.py", "run.py"),
                         (SOURCE / "scripts/d3_contract.py", "contract.py"),
                         (SOURCE / "scripts/d3_fault_proxy.py", "fault_proxy.py"),
                         (SOURCE / "scripts/d3_notify.py", "notify.py")]:
        shutil.copy2(source, root / name)
    (root / "app_setup.mjs").write_text((SOURCE / "scripts/d3_app_setup.mjs").read_text().replace(
        "'../providers/chatgpt/desktop_cdp.mjs'", repr(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs"))))
    (root / "refresh.mjs").write_text((SOURCE / "scripts/d3_refresh.mjs").read_text().replace(
        "'../providers/chatgpt/desktop_cdp.mjs'", repr(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs"))))
    plan = json.loads((root / "plan.json").read_text())
    plan["project"] = project
    for name in plan["order"]:
        case = root / name
        path = case / "config/codex/providers/chatgpt.toml"
        text = path.read_text().split('\n[history]')[0]
        path.write_text(text + '\n[history]\nmode="project"\nproject_name="Termux Provider: D3"\ncreate_if_missing=false\nproject_id=' + json.dumps(project["id"]) + '\n')
        tasks = FAULT_TASKS if name.startswith("fault") else TASKS
        for index, task in enumerate(tasks, 1):
            (case / f"request-{index}.txt").write_text(task + "\n" + task_rules(name, plan))
        metadata = json.loads((case / "tasks.json").read_text())
        metadata["prompt_hashes"] = [sha(case / f"request-{i}.txt") for i in range(1, len(tasks) + 1)]
        save(case / "tasks.json", metadata)
    plan["sealed"] = now()
    save(root / "plan.json", plan)
    manifest = json.loads((root / "inputs.json").read_text())
    manifest["files"] = {str(p.relative_to(root)): sha(p) for p in root.rglob("*")
                         if p.is_file() and ".git" not in p.parts and p.name not in ("inputs.json", "preflight-private.json")}
    save(root / "inputs.json", manifest)
    print(json.dumps({"sealed": str(root), "project": project["name"], "model_requests": 0}))


def tunnel_environment():
    value = json.loads(explicit_input("TUNNEL_ENVIRONMENT").read_text())
    environment = value.get("tunnel_environment", value)
    assert isinstance(environment, dict) and all(isinstance(k, str) and isinstance(v, str) for k, v in environment.items()), "explicit tunnel environment must contain strings"
    return environment


def local_json(command, env):
    try:
        result = subprocess.run(command, env=env, capture_output=True, timeout=45)
    except subprocess.TimeoutExpired:
        # Tunnel Connect arguments contain the runtime credential. Never copy
        # the command into result reasons, logs or the completion notification.
        raise AssertionError("owned local operation timed out; outcome unknown, no retry") from None
    assert result.returncode == 0, "owned local operation failed; no retry"
    return json.loads(result.stdout)


def control(info, env, name, args):
    key = subprocess.check_output([info["binary"], "token", "--state", info["state"]], env=env, text=True).strip()
    request = urllib.request.Request(f'http://127.0.0.1:{info["port"]}/mcp',
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + key})
    with urllib.request.urlopen(request, timeout=35) as response:
        value = json.load(response)["result"]
    assert not value["isError"], "owned control failed"
    return json.loads(value["content"][0]["text"])


def preflight(root):
    """Fresh read-only checks, never open a competing CDP connection or submit Chat."""
    ownership = json.loads((root / "prior-ownership.json").read_text())
    info = ownership["original"]
    env = json.loads((root / "prior-service/environment-private.json").read_text())
    assert sha(Path(info["binary"])) == info["sha256"]
    host = json.loads(explicit_input("DESKTOP_SESSION").read_text())
    proc = Path("/proc", str(host["pid"]))
    assert (proc / "exe").resolve() == Path(host["image"]).resolve()
    fields = (proc / "stat").read_text().rsplit(")", 1)[1].split()
    assert fields[0] != "Z" and fields[19] == str(host["start"]), "desktop ownership changed"
    status = local_json([info["binary"], "status", "--state", info["state"]], env)
    assert status["live"] and status["process"]["pid"] == info["pid"] and status["process"]["image"] == info["binary"]
    tunnel = local_json(["tunnel-client", "runtimes", "status", ownership["tunnel_alias"], "--json"], tunnel_environment())
    assert tunnel["process_running"] and tunnel["ready"] and tunnel["healthy"]
    assert tunnel["profile_dir"] == ownership["tunnel_profile_dir"], "trial tunnel ownership changed"
    works = control(info, env, "codex_sessions", {"action": "list"})["work"]
    observations = []
    stopped = json.loads(Path(ownership["stopped_work_proof"]).read_text())
    assert stopped["native_stop_confirmed"] and stopped["host_stop_confirmed"] and not stopped["remaining_jobs"]
    for work in works:
        state = control(info, env, "codex_status", {"work_id": work["work_id"]})
        assert state["native_status"]["type"] == "notLoaded" and state["driver_phase"] == "released_after_stop"
        assert not state["pending_calls"] and not state["human_requests"]
        assert all(v["settled"] for v in state["call_observations"])
        observations.append(state)
    save(root / "preflight-private.json", {"host": host, "Provider": status, "tunnel": tunnel, "work": observations, **now()})
    return {"ready": True, "owned_live_desktop": True, "previous_work_settled": True,
            "model_requests": 0, "connection_changed": False, **now()}


def acquire(root):
    evidence = preflight(root)
    save(root / "lease-intent.json", evidence, exclusive=True)
    info = json.loads((root / "prior-ownership.json").read_text())["original"]
    env = json.loads((root / "prior-service/environment-private.json").read_text())
    local_json(["tunnel-client", "runtimes", "stop", json.loads((root / "prior-ownership.json").read_text())["tunnel_alias"], "--json"], tunnel_environment())
    local_json([info["binary"], "stop", "--state", info["state"]], env)
    save(root / "lease-ready.json", {"previous_connection_released": True, "desktop_restarted": False, **now()}, exclusive=True)


def regression_proof(project, previous, output, env):
    """Check accumulated tests run and new tests reject the preceding product."""
    output.mkdir()
    modules = sorted(project.glob("src/test_*.rs"))
    added = [p for p in modules if not (previous / p.relative_to(project)).exists()]
    assert added, "each development turn needs a new src/test_*.rs regression module"
    trial = Path(tempfile.mkdtemp(prefix="d3red.", dir=os.environ["TMPDIR"]))
    green = trial / "green"
    red = trial / "red"
    shutil.copytree(project, green, ignore=shutil.ignore_patterns(".git", "target"))
    shutil.copytree(previous, red, ignore=shutil.ignore_patterns(".git", "target"))
    for path in added:
        shutil.copy2(path, red / path.relative_to(project))
        with (red / "src/main.rs").open("a") as stream:
            stream.write(f'\n#[cfg(test)] mod {path.stem};\n')
    results = {}
    for label, work in [("green", green), ("red", red)]:
        e = {**env, "CARGO_TARGET_DIR": str(Path(env["CARGO_TARGET_DIR"]) / output.name / label)}
        result = subprocess.run(["cargo", "test", "--offline", "--locked"], cwd=work, env=e, capture_output=True, timeout=120)
        log = result.stdout + result.stderr
        (output / (label + ".log")).write_bytes(log)
        passed = re.findall(r"^test (\S+) \.\.\. ok$", log.decode(errors="replace"), re.M)
        results[label] = {"exit": result.returncode, "passed_tests": passed}
    assert results["green"]["exit"] == 0
    for name in ("original_tests::add_and_sort", "original_tests::invalid_command_and_negative_balance"):
        assert name in results["green"]["passed_tests"], "original test not executed"
    for module in modules:
        assert any(name.startswith(module.stem + "::") for name in results["green"]["passed_tests"]), "regression module not executed"
    assert results["red"]["exit"] != 0, "new regression did not reject preceding implementation"
    save(output / "review.json", {"accepted": True, "previous_product_rejected": True, "tests": results,
                                  "new_modules": [p.name for p in added], "copy": str(trial)})


def stop_owned_servers(case, info):
    final = json.loads((case / "native-read-final-private.json").read_text())
    assert not final["jobs"]
    expected = Path(info["runtime"]).resolve()
    stopped = []
    for path in server_paths(Path(info["home"]).parent):
        pid = int((path.parent / "pid").read_text())
        if pid in [v["pid"] for v in stopped]:
            continue
        stopped.append(stop_owned_pid(pid, expected))
    save(case / "native-servers-stopped.json", {"servers": stopped, "user_files_removed": False, **now()})


def stop_owned_pid(pid, expected):
    # Termux Python does not expose os.pidfd_open/signal.pidfd_send_signal.
    # These Linux architectures share the asm-generic pidfd syscall numbers.
    assert platform.machine() in ("aarch64", "x86_64", "riscv64")
    libc = ctypes.CDLL(None, use_errno=True)
    fd = libc.syscall(ctypes.c_long(434), ctypes.c_int(pid), ctypes.c_uint(0))
    if fd < 0:
        raise OSError(ctypes.get_errno(), "pidfd_open; no numeric PID fallback")
    try:
        proc = Path("/proc", str(pid))
        assert (proc / "exe").resolve() == expected.resolve(), "private process identity changed"
        start = (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]
        sent = libc.syscall(ctypes.c_long(424), ctypes.c_int(fd), ctypes.c_int(signal.SIGTERM), ctypes.c_void_p(), ctypes.c_uint(0))
        if sent < 0:
            raise OSError(ctypes.get_errno(), "pidfd_send_signal")
        poller = selectors.DefaultSelector()
        try:
            poller.register(fd, selectors.EVENT_READ)
            assert poller.select(15), "private process stop unconfirmed"
        finally:
            poller.close()
        return {"pid": pid, "start_ticks": start, "kernel_stop_confirmed": True}
    finally:
        os.close(fd)


def long_result_proof(calls, returns):
    probes = []
    for identity, call in calls.items():
        if not call.get("name", "").endswith("exec_command"):
            continue
        args = json.loads(call["arguments"])
        if re.search(r"\bprobe\.py\b.*--label\s+long\b", args.get("cmd", "")):
            probes.append((identity, args))
    assert len(probes) == 1 and probes[0][1]["yield_time_ms"] == 1000, "long probe must start exactly once with short yield"
    output = str(returns[probes[0][0]]["output"])
    match = re.search(r"Process running with session ID (\d+)", output)
    assert match, "native pending process was not exercised"
    session = int(match[1])
    collected = [str(returns[identity]["output"]) for identity, call in calls.items()
                 if call.get("name", "").endswith("write_stdin") and json.loads(call["arguments"])["session_id"] == session]
    assert any("D3_EFFECT_RETURNED_LONG" in result for result in collected), "original session result not returned"


class Trial:
    def __init__(self, root, recovery="recovery-1"):
        self.root = root
        assert re.fullmatch(r"recovery-[1-9][0-9]*", recovery)
        self.recovery = root / recovery
        self.plan = json.loads((root / "plan.json").read_text())
        self.selector = selectors.DefaultSelector()
        self.cost = {"status_calls": 0, "status_wall_ns": 0, "canonical_appended_bytes": 0,
                     "supervisory_model_calls": 0, "kernel_wakeups": 0}
        self.rows = []
        self.thread = None
        self.work = None
        self.calls = {}
        self.returns = {}
        self.cli = None
        self.active_turn = None
        self.step = None
        self.step_started = None

    def event(self, name, **values):
        with (self.root / "events.jsonl").open("a") as stream:
            stream.write(json.dumps({"event": name, **now(), **values}, ensure_ascii=False) + "\n")
        save(self.root / "status.json", {"event": name, **now(), **values, "observer_cost": self.cost})

    def tool(self, name, args):
        return control(self.info, self.env, name, args)

    def service(self, script, *args):
        command = [sys.executable, "-B", str(self.root / script), *args]
        if script.endswith(".mjs"):
            command = [self.plan["bun"], str(self.root / script), *args]
            if script == "app_setup.mjs":
                if (self.root / "validator-repair.json").exists():
                    command[1] = str(self.root / "app_setup-revision2.mjs")
                command.insert(2, str(self.root))
            elif script == "refresh.mjs":
                if (self.root / "connection-readiness-repair.json").exists():
                    command[1] = str(self.root / "refresh-with-readiness.mjs")
                command = command[:2] + [str(self.case)]
        result = subprocess.run(command, capture_output=True, timeout=240)
        with (self.case / "service-operations.log").open("ab") as stream:
            stream.write(result.stdout + result.stderr)
        assert result.returncode == 0, "service operation incomplete; never repeat an uncertain mutation"

    def pump(self, timeout):
        for key, _ in self.selector.select(timeout):
            self.cost["kernel_wakeups"] += 1
            if key.data == "pty":
                try:
                    chunk = os.read(key.fd, 65536)
                except OSError:
                    chunk = b""
                if chunk:
                    self.terminal.write(chunk)
                    self.terminal.flush()
            else:
                for row in self.watch.read():
                    payload = row.get("payload", {})
                    if row["type"] == "event_msg" and payload.get("type") == "task_started":
                        self.active_turn = payload.get("turn_id")
                    if row["type"] == "session_meta":
                        self.thread = payload["id"]
                        save(self.case / "thread.json", {"thread_id": self.thread})
                    if row["type"] == "response_item":
                        kind = payload.get("type")
                        if kind in ("function_call", "custom_tool_call"):
                            assert payload["call_id"] not in self.calls, "duplicate original native call"
                            self.calls[payload["call_id"]] = payload
                        elif kind in ("function_call_output", "custom_tool_call_output"):
                            assert payload["call_id"] not in self.returns, "duplicate native return"
                            self.returns[payload["call_id"]] = payload
                        elif kind == "reasoning":
                            continue
                    self.rows.append(row)
                    encoded = json.dumps(row, ensure_ascii=False).encode() + b"\n"
                    self.cost["canonical_appended_bytes"] += len(encoded)
                    self.canonical.write(encoded)
                    self.canonical.flush()

    def settled(self, recoverable_cut=False):
        assert self.calls.keys() == self.returns.keys(), "unreturned/unknown native calls"
        native = native_rpc(Path(self.info["home"]).parent, "thread/read", {"threadId": self.thread, "includeTurns": False})
        # task_complete can be appended just before app-server publishes Idle.
        # Only reconcile this bounded publication race for this exact terminal
        # turn; never reinterpret an arbitrary active/quiet thread as finished.
        terminal_seen = self.active_turn is not None and any(
            r["type"] == "event_msg" and r["payload"].get("type") in ("task_complete", "turn_aborted")
            and r["payload"].get("turn_id") == self.active_turn for r in self.rows)
        for delay in (.05, .2, 1):
            if native["thread"]["status"]["type"] != "active" or not terminal_seen:
                break
            self.pump(delay)
            native = native_rpc(Path(self.info["home"]).parent, "thread/read", {"threadId": self.thread, "includeTurns": False})
        jobs = native_rpc(Path(self.info["home"]).parent, "thread/backgroundTerminals/list", {"threadId": self.thread, "limit": 100})
        save(self.case / "native-read-final-private.json", {"native": native, "jobs": jobs["data"], **now()})
        status = native["thread"]["status"]["type"]
        assert (status == "idle" or recoverable_cut and status == "systemError") and not jobs["data"] and not jobs.get("nextCursor"), f"native boundary unsettled: {status}, jobs={len(jobs['data'])}"
        if self.info["mode"] == "model_adapter":
            state = local_json([self.info["binary"], "status", "--state", self.info["state"], "--thread", self.thread], self.env)
            save(self.case / "boundary-status-private.json", state)
            lane = state["lane"]
            assert lane["phase"] == "idle" and not lane["pending_native_calls"] and not lane["last_error"]
            return True  # Source additionally requires positive app terminal done before Idle.
        if self.work is None:
            works = self.tool("codex_sessions", {"action": "list"})["work"]
            assert len(works) == 1
            self.work = works[0]
            save(self.case / "work-private.json", self.work)
        for delay in (0, 2, 4, 8, 16, 30):
            if delay:
                until = time.monotonic() + delay
                while time.monotonic() < until:
                    self.pump(until - time.monotonic())
            started = time.monotonic_ns()
            value = self.tool("codex_status", {"work_id": self.work["work_id"]})
            self.cost["status_calls"] += 1
            self.cost["status_wall_ns"] += time.monotonic_ns() - started
            save(self.case / "boundary-status-private.json", value)
            if value["host_state"] == "stream_settled" and value["native_status"]["type"] == "idle":
                return True
            if (recoverable_cut and (self.case / "fault-fired.json").exists()
                    and (value["host_state"] == "stream_settled"
                         or value["host_state"] == "error_observed"
                         and value["host_observation"].get("error") == "app_stream_failed")
                    and value["call_observations"] and all(v["settled"] for v in value["call_observations"])
                    and not value["human_requests"] and not value["pending_calls"]
                    and expected_result_cut(self.case, self.calls, self.returns)):
                # Both a truthful app final without codex_finish and an app
                # stream error can follow the declared result cut. Native/wire
                # results are independently settled; new-user-turn recovery
                # reconciles receipts and stops/releases the old lane itself.
                save(self.case / "expected-cut-reconciled.json", {"error_observed": True,
                     "native_effect_and_results_reconciled": True, "first_turn_claimed_successful": False,
                     "next_action": "declared genuine A2 follow-up, never replay A1", **now()})
                return True
            if value["host_state"] in ("error_observed", "stop_unconfirmed") or value["phase"] == "reconciliation_required":
                if not recoverable_cut:
                    break
        raise AssertionError("app settlement/reconciliation unconfirmed; do not queue another task")

    def boundary_review(self, step, fault, project_override=None):
        """Quality audit, invoked only after all cases have finished."""
        project = project_override or self.case / "w"
        previous = self.root / "baseline" if step == 1 else self.case / f"snapshot-{step - 1}"
        for name in ("AGENTS.md", "REQUIREMENTS.md", "Cargo.toml", "Cargo.lock", "probe.py"):
            assert (project / name).read_bytes() == (previous / name).read_bytes(), "frozen input changed"
        assert original_tests((project / "src/main.rs").read_text()) == original_tests((self.root / "baseline/src/main.rs").read_text())
        for path in previous.rglob("*.rs"):
            if path.name.startswith("test_") or "tests" in path.relative_to(previous).parts:
                assert (project / path.relative_to(previous)).read_bytes() == path.read_bytes(), "prior regression modified"
        if fault:
            assert (self.case / "effects/fault.effects").read_text() == "fault\n"
            assert (self.case / "fault-fired.json").exists(), "fault boundary not exercised"
            if step == 2:
                assert "D3_EFFECT_RETURNED_FAULT" in (project / "recovery-note.txt").read_text()
        else:
            contract = self.root / ("contract-revision2.py" if (self.root / "validator-repair.json").exists() else "contract.py")
            suffix = "-corrected" if (self.root / "validator-repair.json").exists() else ""
            result = subprocess.run([sys.executable, "-B", str(contract), str(project), str(step),
                                     str(self.case / f"review-{step}{suffix}")], capture_output=True, timeout=300)
            (self.case / f"contract-{step}.log").write_bytes(result.stdout + result.stderr)
            assert result.returncode == 0, "frozen external contract failed"
            regression_proof(project, previous, self.case / f"regression-{step}", self.env)
            if step == 3:
                assert (self.case / "effects/long.effects").read_text() == "long\n"
                long_result_proof(self.calls, self.returns)
        snapshot_diff_check(previous, project)
        save(self.case / f"review-{step}.json", {"automated_contract_accepted": True, "final_semantic_review": "pending",
             "native_calls": len(self.calls), "native_results": len(self.returns), "unreturned": [], **now()})

    def capture_boundary(self, step, fault):
        """Cheap evidence capture, with no quality decision, compiler or model."""
        project = self.case / "w"
        shutil.copytree(project, self.case / f"snapshot-{step}", ignore=shutil.ignore_patterns(".git", "target"))
        (self.case / f"diff-{step}.patch").write_bytes(subprocess.check_output(["git", "diff"], cwd=project))
        untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=project).split(b"\0")
        save(self.case / f"untracked-{step}.json", {os.fsdecode(p): sha(project / os.fsdecode(p)) for p in untracked if p})
        save(self.case / f"capture-{step}.json", {"quality_review": "deferred_until_all_work_ends",
             "native_calls": len(self.calls), "native_results": len(self.returns), **now()})

    def recover_fault_connection(self):
        """One owned reconnect after receipts settle, never retry the native effect."""
        assert expected_result_cut(self.case, self.calls, self.returns)
        before_calls = digest_results(self.returns)
        owner = local_json([self.info["binary"], "status", "--state", self.info["state"]], self.env)
        assert owner["live"] and owner["process"]["pid"] == self.info["pid"]
        assert owner["process"]["image"] == self.info["binary"]
        assert Path("/proc", str(self.info["pid"]), "exe").resolve() == Path(self.info["binary"]).resolve()
        if self.info["mode"] == "operator":
            # Fence the broken driver even if A1 managed to finish truthfully;
            # genuine A2 must get recovered receipts and a fresh saved Chat.
            stop = self.tool("codex_stop", {"work_id": self.work["work_id"]})
            save(self.case / "recovery-native-host-stop.json", stop)
            assert stop["native_stop_confirmed"] and stop["host_stop_confirmed"] and not stop["remaining_jobs"]
        reconnect_fault_tunnel(self.case, self.thread, before_calls)
        # A fresh stdio initialization/catalog checks the repaired local endpoint.
        # A2 itself is the real Chat-side transport test; this is not a claim
        # that server-side cached MCP sessions have already recovered.
        self.check_recovery_stdio()
        if self.info["mode"] == "model_adapter":
            self.inject_recovery_history()
        self.pump(.01)
        assert digest_results(self.returns) == before_calls
        assert expected_result_cut(self.case, self.calls, self.returns)
        save(self.case / "connection-recovery.json", {"thread": self.thread, "tunnel_reconnects": 1,
             "native_effect_replayed": False, "Provider_or_desktop_restarted": False,
             "adapter_readable_handoff": self.info["mode"] == "model_adapter",
             "operator_recovery": "existing native receipt/driver recovery on genuine A2",
             "local_readiness_confirmed": True, "Chat_transport_test": "genuine A2 still required", **now()})
        self.event("fault_connection_recovered_next_genuine_turn", case=self.case.name)

    def check_recovery_stdio(self):
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "D3-recovery-readiness", "version": "1"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}]
        process = subprocess.Popen([str(self.case / "mcp-stdio")], env=self.env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=(self.case / "recovery-stdio.stderr").open("wb"))
        selector = selectors.DefaultSelector()
        try:
            selector.register(process.stdout, selectors.EVENT_READ)
            replies = []
            for request in requests:
                process.stdin.write(json.dumps(request).encode() + b"\n")
                process.stdin.flush()
                if "id" not in request:
                    continue
                assert selector.select(30), "recovery stdio response missing; no retry"
                response = json.loads(process.stdout.readline())
                assert response["id"] == request["id"] and "error" not in response
                replies.append(response)
            tools = replies[-1]["result"]["tools"]
            assert len(tools) == (14 if self.info["mode"] == "operator" else 2)
            assert {"native_catalog", "native_tool"} <= {v["name"] for v in tools}
            save(self.case / "recovery-stdio-readiness.json", {"accepted": True, "tool_count": len(tools),
                 "model_requests": 0, "native_effects": 0, **now()})
        finally:
            selector.close()
            process.stdin.close()
            process.terminate()
            process.wait(timeout=10)

    def inject_recovery_history(self):
        """Use the existing public-history handoff to renew a broken adapter Chat."""
        home = Path(self.info["home"]).parent
        snapshot = native_rpc(home, "thread/read", {"threadId": self.thread, "includeTurns": True})["thread"]
        jobs = native_rpc(home, "thread/backgroundTerminals/list", {"threadId": self.thread, "limit": 100})
        assert snapshot["id"] == self.thread and snapshot["status"]["type"] == "idle"
        assert not jobs["data"] and not jobs.get("nextCursor")
        command = [self.info["binary"], "checkpoint", "--thread", self.thread,
            "--from-model", "chatgpt-desktop/gpt-6", "--model", "chatgpt-desktop/gpt-6", "--effort", "high"]
        result = subprocess.run(command, input=json.dumps(snapshot).encode(), env=self.env, capture_output=True, timeout=35)
        (self.case / "recovery-checkpoint.stderr").write_bytes(result.stderr)
        assert result.returncode == 0, "public recovery history rejected; preserve without submission"
        item = json.loads(result.stdout)
        assert "D3_EFFECT_RETURNED_FAULT" in json.dumps(item), "original native return absent from handoff"
        save(self.case / "recovery-handoff-private.json", item)
        save(self.case / "recovery-handoff-intent.json", {"thread": self.thread,
             "handoff_sha256": sha(self.case / "recovery-handoff-private.json"), **now()}, exclusive=True)
        receipt = native_rpc(home, "thread/inject_items", {"threadId": self.thread, "items": [item]})
        save(self.case / "recovery-handoff-receipt.json", receipt)

    def audit_all(self):
        failures = []
        for name in self.plan["order"]:
            self.case = self.root / name
            self.info = json.loads((self.case / "info.json").read_text())
            self.env = json.loads((self.case / "environment-private.json").read_text())
            tasks = json.loads((self.case / "tasks.json").read_text())
            self.calls, self.returns = {}, {}
            starts, terminals = set(), set()
            try:
                for path in (Path(self.info["home"]) / "sessions").rglob("*.jsonl"):
                    for line in path.open():
                        row = json.loads(line)
                        value = row.get("payload", {})
                        if row["type"] == "response_item" and value.get("type") in ("function_call", "custom_tool_call"):
                            assert value["call_id"] not in self.calls, "duplicate native call"
                            self.calls[value["call_id"]] = value
                        elif row["type"] == "response_item" and value.get("type") in ("function_call_output", "custom_tool_call_output"):
                            assert value["call_id"] not in self.returns, "duplicate native return"
                            self.returns[value["call_id"]] = value
                        elif row["type"] == "event_msg" and value.get("type") == "task_started":
                            starts.add(value["turn_id"])
                        elif row["type"] == "event_msg" and value.get("type") in ("task_complete", "turn_aborted"):
                            terminals.add(value["turn_id"])
                assert self.calls.keys() == self.returns.keys(), "unreturned native call"
                assert starts == terminals and len(starts) == tasks["count"], "unexpected/missing native user turn"
            except Exception as error:
                failures.append({"case": name, "stage": "canonical_records", "error": str(error)})
                continue
            fault = tasks["kind"] == "fault"
            for step in range(1, tasks["count"] + 1):
                try:
                    final = json.loads((self.case / f"complete-{step}.json").read_text())["native_final"]
                    if not (fault and step == 1):
                        marker = "D3_A2_COMPLETE" if fault else ("D3_B1_COMPLETE", "D3_B2_COMPLETE", "D3_C1_COMPLETE")[step - 1]
                        text = final["payload"].get("last_agent_message") or ""
                        assert completion_claimed(text, marker), "affirmative native completion marker absent"
                        assert re.search(r"소요시간:.*(?:초|분)", text.splitlines()[-1]), "native numeric timing footer absent"
                    self.boundary_review(step, fault, self.case / f"snapshot-{step}")
                except Exception as error:
                    failures.append({"case": name, "step": step, "error": str(error)})
            try:
                self.audit_chat(fault)
            except Exception as error:
                failures.append({"case": name, "stage": "saved_Chat", "error": str(error)})
        save(self.root / "batch-quality-review.json", {"accepted": not failures, "failures": failures,
             "review_started_only_after_all_cases_completed": True, **now()})
        return not failures

    def run_case(self, name, resume_after=0):
        self.thread = self.work = self.active_turn = None
        self.step = self.step_started = None
        self.case = self.root / name
        self.info = json.loads((self.case / "info.json").read_text())
        self.env = json.loads((self.case / "environment-private.json").read_text())
        self.event("case_resume" if resume_after else "case_start", case=name, after=resume_after)
        if not resume_after:
            self.service("manage.py", name, "start")
            self.info = json.loads((self.case / "info.json").read_text())
            self.service("backend.mjs", name, "init")
            self.service("refresh.mjs", name)
        self.watch = AppendWatch(Path(self.info["home"]) / "sessions")
        self.rows, self.calls, self.returns = [], {}, {}
        if resume_after:
            for step in range(1, resume_after + 1):
                assert (self.case / f"complete-{step}.json").exists(), "prior task has no accepted checkpoint"
            assert not (self.case / f"submission-{resume_after + 1}.json").exists(), "next task already submitted; do not replay"
            history = self.watch.read()  # Adopt once; subsequent reads remain incremental.
            self.rows = [row for row in history if not (row["type"] == "response_item" and row["payload"].get("type") == "reasoning")]
            for row in self.rows:
                value = row.get("payload", {})
                if row["type"] == "response_item" and value.get("type") in ("function_call", "custom_tool_call"):
                    assert value["call_id"] not in self.calls
                    self.calls[value["call_id"]] = value
                elif row["type"] == "response_item" and value.get("type") in ("function_call_output", "custom_tool_call_output"):
                    assert value["call_id"] not in self.returns
                    self.returns[value["call_id"]] = value
            self.thread = json.loads((self.case / "thread.json").read_text())["thread_id"]
            if (self.case / "resume-ready.json").exists():
                ready = json.loads((self.case / "resume-ready.json").read_text())
                reconciled_resume(ready, self.thread, self.calls, self.returns)
                stop_path = self.root / ready["stop_proof"] if "stop_proof" in ready else self.case / "failure-stop.json"
                stop = json.loads(stop_path.read_text())
                assert stop["confirmed"] and not stop["result"]["remaining_jobs"]
                native = native_rpc(Path(self.info["home"]).parent, "thread/read", {"threadId": self.thread, "includeTurns": False})
                jobs = native_rpc(Path(self.info["home"]).parent, "thread/backgroundTerminals/list", {"threadId": self.thread, "limit": 100})
                assert native["thread"]["status"]["type"] in ("idle", "systemError") and not jobs["data"] and not jobs.get("nextCursor")
                works = self.tool("codex_sessions", {"action": "list"})["work"]
                self.work, = [v for v in works if v["thread_id"] == self.thread]
                state = self.tool("codex_status", {"work_id": self.work["work_id"]})
                assert stopped_host_reconciled(state), "stopped/released host proof missing"
            else:
                self.settled()  # Positive native/host/result reconciliation before client replacement.
            expected = (Path(self.info["runtime"]).parent / "helpers/2").resolve()
            old_clients = []
            for process in Path("/proc").iterdir():
                if not process.name.isdigit():
                    continue
                try:
                    if (process / "exe").resolve() == expected and self.info["workspace"].encode() in (process / "cmdline").read_bytes().split(b"\0"):
                        old_clients.append(stop_owned_pid(int(process.name), expected))
                except FileNotFoundError:
                    pass
            save(self.case / "old-CLI-client-ended-for-resume.json", {"clients": old_clients, "thread_id": self.thread,
                 "native_idle_results_complete": True, "task_replayed": False, **now()})
        self.selector.register(self.watch.fd, selectors.EVENT_READ, "canonical")
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 48, 220, 0, 0))
        command = [self.info["core"], "--no-alt-screen", "--profile", "chatgpt", "-C", self.info["workspace"]]
        if resume_after:
            command += ["resume", self.thread]
        self.cli = subprocess.Popen(command,
                                    env=self.env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        self.selector.register(master, selectors.EVENT_READ, "pty")
        terminal_path = self.case / ("terminal-resumed-private.bin" if resume_after else "terminal-private.bin")
        self.terminal = terminal_path.open("ab")
        self.canonical = (self.case / ("canonical-resumed-public-private.jsonl" if resume_after else "canonical-public-private.jsonl")).open("ab")
        # Native readiness is local, no model call. CLI receives first real task.
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            self.pump(.25)
            if b"GPT-6" in terminal_path.read_bytes():
                break
            assert self.cli.poll() is None, "CLI startup failed"
        else:
            raise AssertionError("CLI readiness deadline")
        tasks = json.loads((self.case / "tasks.json").read_text())
        fault = tasks["kind"] == "fault"
        if fault:
            (self.case / "fault-armed").touch()
        if (fault and resume_after == 1 and self.plan.get("automatic_fault_recovery")
                and not (self.case / "connection-recovery.json").exists()):
            self.recover_fault_connection()
        for step in range(resume_after + 1, tasks["count"] + 1):
            step_started = now()
            self.step, self.step_started = step, step_started
            old_calls, old_returns = len(self.calls), len(self.returns)
            text = (self.case / f"request-{step}.txt").read_text()
            assert sha(self.case / f"request-{step}.txt") == tasks["prompt_hashes"][step - 1]
            text += (f'\nTiming: this turn starts at Termux monotonic_ns={step_started["monotonic_ns"]}, UTC={step_started["UTC"]}. '
                     'At the end of your existing final verification command, read Python time.monotonic_ns() and compute elapsed seconds from that start. '
                     'The LAST line of the final answer must be `소요시간: 약 N초 (최종 시계 확인까지)`, using this observed duration. '
                     'If using codex_finish, include the task marker and timing footer both in its native final summary and in your visible Chat final. '
                     'Do not invent elapsed time. The external controller separately records completion-observed duration.\n')
            (self.case / f"effective-request-{step}.txt").write_text(text)
            save(self.case / f"submission-{step}.json", {"state": "intent_before_IO", **now()}, exclusive=True)
            self.event("task_submit", case=name, step=step)
            offset = len(self.rows)
            if step == 1:
                os.write(master, b"\x1b[200~" + text.encode() + b"\x1b[201~")
                self.pump(.2)
                os.write(master, b"\r")
            else:
                native_rpc(Path(self.info["home"]).parent, "turn/start", {"threadId": self.thread, "input": [{"type": "text", "text": text}]})
            deadline = time.monotonic() + self.plan["task_timeout_seconds"]
            final = None
            scanned = offset
            while time.monotonic() < deadline:
                self.pump(min(30, max(.01, deadline - time.monotonic())))
                for row in self.rows[scanned:]:
                    if terminal_for_turn(row, self.active_turn):
                        final = row
                scanned = len(self.rows)
                if final:
                    break
                assert self.cli.poll() is None, "original CLI exited"
            assert final is not None, "task deadline; preserve unknown work without resubmission"
            final_seen = now()
            self.settled(recoverable_cut=fault and step == 1)
            inference_finished = now()
            self.capture_boundary(step, fault)
            save(self.case / f"complete-{step}.json", {"native_final": final, "started": step_started,
                 "native_final_seen": final_seen, "settlement_finished": inference_finished, "capture_finished": now(),
                 "native_calls": len(self.calls) - old_calls, "native_results": len(self.returns) - old_returns})
            elapsed = (final_seen["monotonic_ns"] - step_started["monotonic_ns"]) / 1e9
            with (self.root / "turns.md").open("a") as stream:
                stream.write(f'\n## {name} · {step}턴\n\n' + (final["payload"].get("last_agent_message") or "[완료 답변 없음]") +
                             f'\n\n소요시간: {elapsed:.3f}초 (제어기 완료 관측 기준)\n')
            self.event("task_finished_review_deferred", case=name, step=step)
            if fault and step == 1 and self.plan.get("automatic_fault_recovery"):
                self.recover_fault_connection()
        # Positive stops are mandatory before another case acquires the app.
        if self.info["mode"] == "operator":
            stop = self.tool("codex_stop", {"work_id": self.work["work_id"]})
            save(self.case / "native-stop.json", stop)
            assert stop["native_stop_confirmed"] and stop["host_stop_confirmed"] and not stop["remaining_jobs"]
        os.killpg(self.cli.pid, signal.SIGTERM)
        self.cli.wait(timeout=15)
        self.selector.unregister(master)
        os.close(master)
        self.selector.unregister(self.watch.fd)
        self.watch.close()
        self.terminal.close()
        self.canonical.close()
        stop_owned_servers(self.case, self.info)
        save(self.case / "native-ended.json", {"all_tasks_settled": True, **now()})
        self.service("manage.py", name, "close")
        self.service("backend.mjs", name, "read")
        try:
            self.service("app_setup.mjs", "titles", name)
        except Exception as error:
            save(self.case / "title-operation-pending-review.json", {"error": str(error), **now()})
        self.service("backend.mjs", name, "cleanup")
        app = json.loads((self.case / "read-backend-private.json").read_text())
        save(self.case / "route-complete.json", {"work_complete": True, "quality_review": "deferred_until_all_work_ends",
             "app_dispatches": len(app["requests"]), **now()})
        self.event("case_complete_review_deferred", case=name)

    def audit_chat(self, fault):
        app = json.loads((self.case / "read-backend-private.json").read_text())
        assert app["saved"] and app["requests"], "saved Chat / actual dispatch evidence absent"
        assert len(app["saved"]) <= (2 if fault or (self.case / "resume-ready.json").exists() else 1), "unexpected conversation renewal"
        assert all(not v["isTemporary"] for v in app["saved"])
        assert all(v.get("model") == "gpt-6-thinking" and v.get("effort") == "extended"
                   for chat in app["saved"] for v in chat["metadata"] if v.get("model"))
        assert all(any(v.get("model") == "gpt-6-thinking" for v in chat["metadata"]) for chat in app["saved"])
        public_finals = ["\n".join(part for part in m.get("content", {}).get("parts", []) if isinstance(part, str))
                         for chat in app["saved"] for m in chat["publicMessages"]
                         if m.get("author", {}).get("role") == "assistant" and m.get("channel") in (None, "final")]
        for marker in (["D3_A2_COMPLETE"] if fault else ["D3_B1_COMPLETE", "D3_B2_COMPLETE", "D3_C1_COMPLETE"]):
            assert any(completion_claimed(value, marker) and "소요시간:" in value.splitlines()[-1] for value in public_finals if value), "saved Chat affirmative final/timing footer absent"
        project = self.plan["project"]["id"]
        titles = json.loads((self.case / "titles.json").read_text())
        assert len(titles) == len(app["saved"]) and all(v["project"] == project and v["content_unchanged"] for v in titles)
        save(self.case / "chat-quality-review.json", {"accepted": True, **now()})

    def run(self, resume_case=None, resume_after=0):
        lock = (self.root / "run.lock").open("w")
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        save(self.recovery / "run-started.json" if resume_case else self.root / "run-started.json", {"pid": os.getpid(), **now()}, exclusive=True)
        self.event("run_resumed" if resume_case else "run_started", case=resume_case, after=resume_after)
        try:
            inputs = json.loads((self.root / "inputs.json").read_text())
            assert all(sha(SOURCE / p) == h for p, h in inputs["helper_inputs"].items()), "helper inputs changed"
            if resume_case:
                assert resume_after > 0
                repair_inputs = json.loads((self.recovery / "inputs.json").read_text())
                assert all(sha(self.root / p) == h for p, h in repair_inputs.items()), "reconciled recovery inputs changed"
                index = self.plan["order"].index(resume_case)
                assert all((self.root / name / "route-complete.json").exists() for name in self.plan["order"][:index])
                self.run_case(resume_case, resume_after=resume_after)
                remaining = self.plan["order"][index + 1:]
            else:
                assert all(sha(self.root / p) == h for p, h in inputs["files"].items()), "prepared trial inputs changed"
                acquire(self.root)
                remaining = self.plan["order"]
            for name in remaining:
                self.run_case(name)
            self.case = self.root / "prior-service"
            self.service("manage.py", "prior-service", "start")
            restored = json.loads((self.case / "info.json").read_text())
            assert control(restored, json.loads((self.case / "environment-private.json").read_text()), "codex_sessions", {"action": "list"}) is not None
            self.service("refresh.mjs", "prior-service")
            save(self.root / "connection-restored.json", {"same_original_binary_state_home_workspace": True,
                 "info": str(self.case / "info.json"), "connector_readiness_confirmed": True, "model_requests": 0, **now()})
            self.event("all_work_complete_start_batch_review")
            accepted = self.audit_all()
            verify_immutable_inputs(self.root, self.plan, inputs)
            usage = resource.getrusage(resource.RUSAGE_SELF)
            self.cost.update(controller_cpu_seconds=usage.ru_utime + usage.ru_stime, controller_peak_RSS_kib=usage.ru_maxrss)
            self.event("run_complete", final_semantic_review="pending")
            save(self.root / "result.json", {"work_complete": True, "automated_gates": accepted,
                 "unattended_control_accepted": accepted and resume_case is None,
                 "supervisor_resume_used": resume_case is not None,
                 "sealed_inputs_unchanged": True,
                 "final_semantic_review": "pending", "cost": self.cost, **now()})
        except BaseException as error:
            reason = str(error) or type(error).__name__
            if self.step_started is not None and not (self.case / f"complete-{self.step}.json").exists():
                failed = now()
                elapsed = (failed["monotonic_ns"] - self.step_started["monotonic_ns"]) / 1e9
                save(self.case / f"failed-{self.step}.json", {"started": self.step_started, "failed": failed, "elapsed_seconds": elapsed, "reason": reason})
                with (self.root / "turns.md").open("a") as stream:
                    stream.write(f'\n## {self.case.name} · {self.step}턴 실패/미확인\n\n{error}\n\n소요시간: {elapsed:.3f}초 (실패 관측까지)\n')
            stop = {"requested": False, "confirmed": False}
            if self.thread is not None:
                try:
                    save(self.case / f"failure-stop-{self.recovery.name}-intent.json", now(), exclusive=True)
                    stop["requested"] = True
                    if self.info["mode"] == "operator":
                        if self.work is None:
                            works = self.tool("codex_sessions", {"action": "list"})["work"]
                            matches = [v for v in works if v["thread_id"] == self.thread]
                            assert len(matches) == 1
                            self.work = matches[0]
                        result = self.tool("codex_stop", {"work_id": self.work["work_id"]})
                        stop.update(result=result, confirmed=result["native_stop_confirmed"] and result["host_stop_confirmed"] and not result["remaining_jobs"])
                    elif self.active_turn is not None:
                        native_rpc(Path(self.info["home"]).parent, "turn/interrupt", {"threadId": self.thread, "turnId": self.active_turn})
                        deadline = time.monotonic() + 20
                        while time.monotonic() < deadline:
                            self.pump(deadline - time.monotonic())
                            if any(r["type"] == "event_msg" and r["payload"].get("type") == "turn_aborted" for r in self.rows):
                                break
                        state = local_json([self.info["binary"], "status", "--state", self.info["state"], "--thread", self.thread], self.env)
                        jobs = native_rpc(Path(self.info["home"]).parent, "thread/backgroundTerminals/list", {"threadId": self.thread, "limit": 100})
                        stop.update(result=state, remaining_jobs=jobs["data"], confirmed=state["lane"]["phase"] == "stopped" and not jobs["data"] and not jobs.get("nextCursor"))
                except BaseException as stop_error:
                    stop["error"] = str(stop_error)
                save(self.case / "failure-stop.json", stop)
            usage = resource.getrusage(resource.RUSAGE_SELF)
            self.cost.update(controller_cpu_seconds=usage.ru_utime + usage.ru_stime, controller_peak_RSS_kib=usage.ru_maxrss)
            self.event("preserved_stop", reason=reason, case=getattr(self, "case", self.root).name)
            save(self.root / "result.json", {"automated_gates": False, "reason": reason,
                 "unattended_control_accepted": False, "supervisor_resume_used": resume_case is not None,
                 "uncertain_actions_replayed": False, "preserve_processes_and_artifacts": True,
                 "owned_stop": stop, "cost": self.cost, **now()})
            raise


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--d2", type=Path, required=True)
    p.add_argument("--prior-service", type=Path, help="current verified owned connection to restore")
    p.add_argument("--stopped-work-proof", type=Path, help="confirmed native/host Stop for that connection")
    p.add_argument("--fault-only", action="store_true", help="fresh matched A-only trial; reuse accepted B/C evidence")
    for action in ("start", "run", "status", "preflight", "seal", "resume", "run-resume"):
        p = sub.add_parser(action)
        p.add_argument("root", type=Path)
        if action in ("resume", "run-resume"):
            p.add_argument("--case", required=True)
            p.add_argument("--after", type=int, required=True)
            p.add_argument("--recovery", default="recovery-1")
    args = parser.parse_args()
    if args.action == "prepare":
        prepare(args)
    elif args.action == "preflight":
        print(json.dumps(preflight(args.root)))
    elif args.action == "seal":
        seal(args.root)
    elif args.action == "status":
        path = args.root / "result.json"
        if not path.exists():
            path = args.root / "status.json"
        state = "launch_requested_check_PID_and_log" if (args.root / "launch-intent.json").exists() else "prepared_not_started"
        print(path.read_text() if path.exists() else json.dumps({"state": state}))
    elif args.action in ("start", "resume"):
        recovery = args.action == "resume"
        if not recovery:
            assert not (args.root / "run-started.json").exists(), "already started; inspect status instead of replay"
        else:
            assert args.after > 0 and (args.root / args.case / f"complete-{args.after}.json").exists()
            assert not (args.root / args.case / f"submission-{args.after + 1}.json").exists(), "next task already submitted"
        ack = json.loads((args.root / "notification-probe-ack.json").read_text())
        assert ack["receiver_verified"] and ack["same_thread"] and ack["notifier_sha256"] == sha(args.root / "notify.py"), "same-session notification not demonstrated"
        review = json.loads((Path(json.loads((args.root / "plan.json").read_text())["cache"]) / "setup-validation/review.json").read_text())
        executable = Path(__file__).resolve() if recovery else args.root / "run.py"
        assert review["accepted"] and review["controller_sha256"] == sha(executable), "setup validation missing or stale"
        recovery_root = args.root / args.recovery if recovery else args.root
        assert not recovery or re.fullmatch(r"recovery-[1-9][0-9]*", args.recovery)
        save(recovery_root / "launch-intent.json", now(), exclusive=True)
        log = (recovery_root / "runner.log").open("ab")
        command = [sys.executable, "-B", str(executable), "run-resume" if recovery else "run", str(args.root)]
        if recovery:
            command += ["--case", args.case, "--after", str(args.after), "--recovery", args.recovery]
        child = subprocess.Popen(command,
                                 stdin=subprocess.DEVNULL, stdout=log, stderr=log, start_new_session=True, close_fds=True)
        save(recovery_root / "launched.json", {"pid": child.pid, **now()})
        print(json.dumps({"detached_pid": child.pid, "root": str(args.root)}))
    else:
        try:
            Trial(args.root, args.recovery if args.action == "run-resume" else "recovery-1").run(args.case if args.action == "run-resume" else None, args.after if args.action == "run-resume" else 0)
        finally:
            if (args.root / "result.json").exists() and (args.root / "notification-binding.json").exists():
                destination = args.root
                if args.action == "run-resume":
                    destination = args.root / args.recovery
                    shutil.copy2(args.root / "result.json", destination / "result.json")
                try:
                    notice = subprocess.run([sys.executable, "-B", str(args.root / "notify.py"), "send", str(destination)],
                                            capture_output=True, timeout=75)
                    (destination / "notification-dispatch.log").write_bytes(notice.stdout + notice.stderr)
                except subprocess.TimeoutExpired:
                    save(args.root / "notification-dispatch-unknown.json", {"delivery_unknown": True, "retry": False, **now()})


if __name__ == "__main__":
    main()
