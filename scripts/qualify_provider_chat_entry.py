#!/usr/bin/env python3
"""Bounded real ChatGPT-origin/native qualification; preparation submits no model.

The input client is explicitly separate from the app-less execution Provider.
No shell executor, alternate transcript or per-turn supervisory model is added.
"""
import argparse
import datetime
import difflib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import resource
import select
import shlex
import shutil
import subprocess
import struct
import sys
import tempfile
import termios
import time
import tomllib
import urllib.request

SOURCE = Path(__file__).resolve().parents[1]
if not (SOURCE / "scripts").is_dir():
    SOURCE = Path(json.loads((Path(__file__).parent / "plan.json").read_text())["repository"])
sys.path.insert(0, str(SOURCE / "scripts"))
from provider_qualification import explicit_input
from qualify_permission_lifecycle import fixture, rpc as native_rpc, server_paths
from qualify_provider_d3 import stop_owned_pid


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


def invoke(root, name, argv, env, timeout=40):
    p = subprocess.run(argv, env=env, capture_output=True, text=True, timeout=timeout)
    (root / (name + "-private.json")).write_text(p.stdout)
    (root / (name + ".stderr")).write_text(p.stderr)
    assert p.returncode == 0, name + " failed; private evidence retained"
    return json.loads(p.stdout)


def control(info, env, name, args, *, evidence_path=None):
    token = subprocess.check_output([info["binary"], "token", "--state", info["state"]], env=env, text=True).strip()
    request = urllib.request.Request("http://127.0.0.1:" + str(info["port"]) + "/mcp",
        data=json.dumps({"jsonrpc": "2.0", "id": name, "method": "tools/call", "params": {"name": name, "arguments": args}}).encode(),
        headers={"Authorization": "Bearer " + token, "Content-Type": "application/json"})
    row = json.load(urllib.request.urlopen(request, timeout=30))["result"]
    if evidence_path is not None:
        save(evidence_path, row, exclusive=True)
    assert not row.get("isError"), name + " rejected"
    return json.loads(row["content"][0]["text"])


def check_native_socket_budget(home):
    # Core uses a 16-hex namespace and checks the absolute path before spawn.
    socket = Path(home) / ".local/share/codex/core/catalogs" / ("0" * 16) / "s"
    size = len(os.fsencode(socket))
    if size > 107:
        raise ValueError(f"native fixture socket path is {size} bytes; Core permits 107")
    return size


def preflight_native(root, info, env):
    """Start only the isolated backend; no thread, turn or model request."""
    socket_bytes = check_native_socket_budget(info["fixture_home"])
    if (root / "native-preflight.json").exists():
        previous = json.loads((root / "native-preflight.json").read_text())
        socket = Path((root / "native-preflight.stdout").read_text().strip())
        catalog = subprocess.check_output([info["binary"], "catalog"], env=env, timeout=15)
        paths = server_paths(Path(info["fixture_home"]), info["runtime"]) if info.get("runtime") else server_paths(Path(info["fixture_home"]))
        assert previous["accepted"] and socket in paths
        assert catalog == (root / "native-preflight-catalog.json").read_bytes(), "preflight catalog changed"
        return
    save(root / "native-preflight-intent.json", stamp(), True)
    catalog = subprocess.run([info["binary"], "catalog"], env=env,
        capture_output=True, cwd=info["workspace"], timeout=15)
    (root / "native-preflight-catalog.stderr").write_bytes(catalog.stderr)
    assert catalog.returncode == 0, "native preflight catalog failed; live connection untouched"
    (root / "native-preflight-catalog.json").write_bytes(catalog.stdout)
    # Pool::prepare sends serde_json's compact catalog bytes. The Core namespace
    # binds those exact bytes: CLI pretty output would start a second backend.
    wire_catalog = json.dumps(json.loads(catalog.stdout), ensure_ascii=False,
        separators=(",", ":")).encode()
    (root / "native-preflight-input.json").write_bytes(wire_catalog)
    prepared = subprocess.run([info["core"], "__app-server-catalog-v1"], env=env,
        input=wire_catalog, capture_output=True, cwd=info["workspace"], timeout=40)
    (root / "native-preflight.stdout").write_bytes(prepared.stdout)
    (root / "native-preflight.stderr").write_bytes(prepared.stderr)
    assert prepared.returncode == 0, "native preparation failed; live connection untouched; inspect retained stderr"
    socket = Path(prepared.stdout.decode().strip())
    assert socket.parent.parent == Path(info["fixture_home"]) / ".local/share/codex/core/catalogs"
    paths = server_paths(Path(info["fixture_home"]), info["runtime"]) if info.get("runtime") else server_paths(Path(info["fixture_home"]))
    assert socket in paths, "native backend ownership not verified"
    save(root / "native-preflight.json", {"accepted": True, "socket_bytes": socket_bytes,
        "model_requests": 0, "native_threads_created": 0, **stamp()})


def prepare(root):
    assert not (root / "plan.json").exists(), "already prepared"
    os.umask(0o077)
    d2 = explicit_input("D2_ROOT")
    desktop_session = explicit_input("DESKTOP_SESSION")
    connector_reference = explicit_input("CONNECTOR_REFERENCE")
    tunnel_profile = explicit_input("TUNNEL_PROFILE")
    tunnel_alias = explicit_input("TUNNEL_ALIAS", path=False)
    frontend = explicit_input("FRONTEND")
    frozen = json.loads((d2 / "corrected-candidate-inputs.json").read_text())
    assert all(sha(SOURCE / name) == digest for name, digest in frozen["source"].items()), "qualified product input drift"
    (root / "candidate").mkdir()
    shutil.copy2(d2 / "candidate-corrected/codex-chatgpt-provider", root / "candidate/provider")
    shutil.copy2(d2 / "candidate-corrected/app_bridge.js", root / "candidate/app_bridge.js")
    assert sha(root / "candidate/provider") == frozen["binary_sha256"]
    shutil.copy2(__file__, root / "run.py")
    client = (SOURCE / "scripts/chat_entry_client.mjs").read_text().replace(
        "'../providers/chatgpt/desktop_cdp.mjs'", json.dumps(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs")))
    (root / "client.mjs").write_text(client)
    refresh = (SOURCE / "scripts/d3_refresh.mjs").read_text().replace(
        "'../providers/chatgpt/desktop_cdp.mjs'", json.dumps(str(SOURCE / "providers/chatgpt/desktop_cdp.mjs")))
    (root / "refresh.mjs").write_text(refresh)
    shutil.copy2(d2 / "manage.py", root / "manage.py")
    prior = root / "prior-service"
    prior.mkdir()
    shutil.copy2(root / "prior-info-private.json", prior / "info.json")
    shutil.copy2(root / "prior-environment-private.json", prior / "environment-private.json")
    # The longer evidence root overflowed AF_UNIX's path budget on Android.
    # Keep its separately owned fixture under a short TMPDIR path; never alias
    # HOME through a symlink (Core verifies canonical installation ownership).
    native = Path(tempfile.mkdtemp(prefix="ce.", dir=os.environ["TMPDIR"]))
    save(root / "native-fixture.json", {"root": str(native), "owner": str(root), **stamp()})
    check_native_socket_budget(native / "h")
    core = Path(os.environ["PREFIX"]) / "bin/codex"
    home, prefix, env = fixture(native, core, frontend)
    workspace = root / "w"
    workspace.mkdir()
    (workspace / "ledger.py").write_text("def total(entries):\n    return -sum(entries)\n")
    original = """import unittest
from ledger import total
class OriginalTests(unittest.TestCase):
    def test_sum(self): self.assertEqual(total([2, 3, -1]), 4)
    def test_empty(self): self.assertEqual(total([]), 0)
"""
    (workspace / "test_original.py").write_text(original)
    shutil.copytree(workspace, root / "baseline")
    config = home / ".codex/config.toml"
    config.write_text(config.read_text() + '\n[projects.' + json.dumps(str(workspace)) + ']\ntrust_level="trusted"\n')
    env["XDG_CONFIG_HOME"] = str(root / "config")
    env["CARGO_NET_OFFLINE"] = "true"
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "codex/acceptance" / root.name
    cache.mkdir(parents=True, mode=0o700)
    (cache / "root").write_text(str(root))
    env["CARGO_TARGET_DIR"] = str(cache / "compiler")
    cfg = root / "config/codex/providers/chatgpt.toml"
    cfg.parent.mkdir(parents=True)
    cfg.write_text('version=1\n[model]\ndefault="chatgpt-desktop/gpt-6"\nreasoning_effort="high"\n'
        '[transport]\nreturn_path="mcp"\nconnector_name="Termux Bridge"\n'
        '[execution]\nmode="operator"\n' + ''.join(k + '=' + json.dumps(str(v)) + '\n' for k, v in
        [("core", prefix / "bin/codex"), ("home", home / ".codex"), ("workspace", workspace)]))
    info = {"mode": "operator", "return_path": "mcp", "binary": str(root / "candidate/provider"),
        "sha256": frozen["binary_sha256"], "state": str(root / "state"), "home": str(home / ".codex"),
        "workspace": str(workspace), "core": str(prefix / "bin/codex"), "fixture_home": str(home), "fixture_root": str(native), "desktop_session": str(desktop_session),
        "connector_reference": str(connector_reference), "tunnel_profile": str(tunnel_profile), "tunnel_alias": tunnel_alias}
    save(root / "info.json", info)
    save(root / "environment-private.json", env)
    for argv in [["git", "init", "-q"], ["git", "add", "."], ["git", "-c", "user.name=Owned trial", "-c", "user.email=owned@example.invalid", "commit", "-qm", "frozen Chat-entry baseline"]]:
        subprocess.run(argv, cwd=workspace, env=env, check=True, capture_output=True)
    host = json.loads(desktop_session.read_text())
    descriptor = json.loads((Path(host["root"]) / "desktop-host.json").read_text())
    plan = {"repository": str(SOURCE), "prepared": stamp(), "cache": str(cache), "source_HEAD": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=SOURCE, text=True).strip(),
        "qualified_product": frozen, "desktop_session": str(desktop_session),
        "bun": descriptor["host"]["image"], "project_name": "Termux Provider: Chat entry", "host_model": "gpt-6", "thinking_effort": "extended",
        "bindings": {"initial": "app://-/assets/app-initial-ed5807835d97.js", "shared": "app://-/assets/app-shared-2d77298f0100.js"},
        "max_user_turns": 2, "max_source_requests": 4, "expected_saved_chats": 1, "task_timeout_seconds": 1200,
        "provider_app_host": "absent: no rootfs and no descriptor", "input_client": "existing authenticated Linux ChatGPT client via backend API; not an execution Provider",
        "scope": "ordinary ChatGPT-origin primary native control without Provider app acquisition; not removal of the qualification input client or quota attribution", "supervisory_model_calls_between_tasks": 0}
    save(root / "plan.json", plan)
    common = """Use ONLY the installed Termux Bridge MCP for native development. Do not use Work, handoff-to-Work, another model, computer use, or your own shell. The native Provider has NO Linux app host: do not request Chat renewal or a native summary. Native Codex owns all execution, approvals and canonical records. Do not expose credentials or private reasoning. Use exact logical call IDs and never replay uncertain effects. Discover the full native tools, use real commands and patches, preserve test_original.py byte-for-byte, run the complete project tests, and only claim completion after actual results. Publish a concise report and the final answer through codex_report/codex_finish AND in this Chat. Keep this saved Chat and the same primary native thread. Measure elapsed seconds with a real clock and print them on a separate line. Put the requested completion marker alone on the final line, without Markdown formatting.\n"""
    first = """This is a user-authorized real ChatGPT-entry development experiment in the Provider's disposable workspace. First use codex_sessions create (call_id chat-entry-create), then codex_sessions start with this genuine development task. Use codex_attach, wait for its native round if needed, then echo the returned receipt as accept before effects. Fix ledger.py total(entries): exact sum of integers, reject bool/non-integer values with ValueError. Implement a CLI reading one JSON array from stdin and printing {\"total\": integer}; invalid input exits 2 without traceback or successful JSON. Keep the original tests unchanged. Demonstrate the baseline test failure, implement the fix/features with native tools, add regressions, and run all tests. Complete with marker CHAT_ENTRY_TURN_1_COMPLETE.\n"""
    second = """Genuine follow-up on the previous implementation. Use codex_sessions list to locate THIS work, read codex_history and claim the settled work using its exact cursor/context_cursor. Keep the SAME native thread. Start this new genuine task through codex_sessions start, attach and accept the current receipt. Add apply_batch(balance, changes): exact integer balance and changes (reject bool/non-integer), return the final balance, and reject any intermediate negative balance with ValueError. Do not mutate changes. Add optional --balance INTEGER to the CLI; default array-sum behavior remains compatible. Invalid inputs/overdraft exit 2 without traceback or successful JSON. Preserve all existing tests, add tests covering successful/failed batches and CLI behavior, and run the full suite. Complete with marker CHAT_ENTRY_TURN_2_COMPLETE.\n"""
    for step, task in enumerate([first, second], 1):
        (root / (f"request-{step}.txt")).write_text(task + common)
    binding = {"thread_id": os.environ["CODEX_THREAD_ID"], "core": os.environ["CODEX_PROFILE_CORE"],
        "environment": {k: os.environ[k] for k in ["HOME", "CODEX_HOME", "CODEX_SQLITE_HOME", "CODEX_PROFILE_CORE", "PREFIX", "PATH", "TMPDIR", "LANG", "TERM"] if k in os.environ}}
    binding["core_sha256"] = sha(binding["core"])
    save(root / "notification-binding.json", binding)
    print(json.dumps({"root": str(root), "prepared": True, "model_requests": 0, "provider_app_host": False}))


def ready_prior(root):
    info = json.loads((root / "prior-info-private.json").read_text())
    env = json.loads((root / "prior-environment-private.json").read_text())
    status = invoke(root, "prior-current-status", [info["binary"], "status", "--state", info["state"]], env)
    assert status["live"] and status["process"]["pid"] == info["pid"] and sha(info["binary"]) == info["sha256"]
    states = [control(info, env, "codex_status", {"work_id": w["work_id"]}) for w in control(info, env, "codex_sessions", {"action": "list"})["work"]]
    assert all(not s["pending_calls"] and not s["human_requests"] and s["native_status"]["type"] == "notLoaded" and s["driver_phase"] == "released_after_stop" for s in states)
    te = json.loads((root / "tunnel-environment-private.json").read_text())
    tunnel = invoke(root, "prior-current-tunnel", ["tunnel-client", "runtimes", "status", json.loads((root / "info.json").read_text())["tunnel_alias"], "--json"], te)
    assert tunnel["process_running"] and tunnel["ready"] and tunnel["healthy"]
    previous = json.loads((root / "prior-tunnel-private.json").read_text())
    assert tunnel["profile_dir"] == previous["profile_dir"], "prior tunnel owner changed"
    return info, env, te


def setup(root):
    assert not (root / "setup-intent.json").exists(), "setup already requested; reconcile, do not retry"
    info = json.loads((root / "info.json").read_text())
    env = json.loads((root / "environment-private.json").read_text())
    profile = json.loads(Path(info["tunnel_profile"]).read_text())["control_plane"]
    # The MCP list probe cannot prove native preparation. Exercise the exact
    # private catalog preparation before releasing the user's idle connection.
    preflight_native(root, info, env)
    prior, pe, te = ready_prior(root)
    save(root / "setup-intent.json", stamp(), True)
    invoke(root, "prior-tunnel-stop", ["tunnel-client", "runtimes", "stop", json.loads((root / "info.json").read_text())["tunnel_alias"], "--json"], te)
    invoke(root, "prior-provider-stop", [prior["binary"], "stop", "--state", prior["state"]], pe)
    p = subprocess.Popen([info["binary"], "serve", "--state", info["state"], "--port", "0"], cwd=info["workspace"], env=env,
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=(root / "provider.log").open("ab"), text=True, start_new_session=True)
    assert select.select([p.stdout], [], [], 25)[0], "Provider readiness unknown; no repeat"
    line = p.stdout.readline()
    assert line, "Provider exited; inspect provider.log"
    ready = json.loads(line)
    save(root / "provider-ready-private.json", ready)
    info.update(pid=p.pid, port=int(ready["base_url"].split(":")[-1].split("/")[0]))
    save(root / "info.json", info)
    invoke(root, "profile-prepare", [info["binary"], "prepare", "--home", info["home"], "--state", info["state"]], env)
    control(info, env, "codex_sessions", {"action": "list"})
    wrapper = root / "mcp-stdio"
    wrapper.write_text("#!" + os.environ["PREFIX"] + "/bin/sh\nexec " + shlex.join([os.environ["PREFIX"] + "/bin/env", "-i", *[k + "=" + v for k, v in env.items()], info["binary"], "mcp-stdio", "--state", info["state"]]) + "\n")
    wrapper.chmod(0o700)
    invoke(root, "trial-tunnel-connect", ["tunnel-client", "runtimes", "connect", "--alias", json.loads((root / "info.json").read_text())["tunnel_alias"], "--tunnel-id", profile["tunnel_id"],
        "--runtime-api-key", profile["api_key"], "--profile", "chat-entry-direct", "--profile-dir", str(root / "profiles"), "--mcp-command", str(wrapper), "--json"], te, 60)
    tunnel = invoke(root, "trial-tunnel-ready", ["tunnel-client", "runtimes", "status", json.loads((root / "info.json").read_text())["tunnel_alias"], "--json"], te)
    assert tunnel["process_running"] and tunnel["ready"] and tunnel["healthy"]
    plan = json.loads((root / "plan.json").read_text())
    refresh = subprocess.run([plan["bun"], str(root / "refresh.mjs"), str(root)], capture_output=True, text=True, timeout=150)
    (root / "refresh.log").write_text(refresh.stdout + refresh.stderr)
    assert refresh.returncode == 0, "connector readiness failed; no model submitted"
    client = InputClient(root)
    try:
        initialized = client.call("init")
        save(root / "input-client-ready.json", initialized)
    finally:
        client.close()
    files = ["run.py", "client.mjs", "refresh.mjs", "manage.py", "plan.json", "request-1.txt", "request-2.txt", "candidate/provider", "candidate/app_bridge.js", "config/codex/providers/chatgpt.toml", "baseline/ledger.py", "baseline/test_original.py"]
    helpers = [SOURCE / "scripts/provider_qualification.py", SOURCE / "scripts/qualify_permission_lifecycle.py", SOURCE / "scripts/qualify_provider_d3.py", SOURCE / "scripts/qualify_tasks.py", SOURCE / "providers/chatgpt/desktop_cdp.mjs"]
    save(root / "sealed-inputs.json", {**{name: sha(root / name) for name in files}, **{str(path): sha(path) for path in helpers}})
    save(root / "setup-complete.json", {"model_requests": 0, "provider_has_app_host": False, **stamp()})
    print(json.dumps({"setup_ready": True, "root": str(root), "model_requests": 0}))


class InputClient:
    def __init__(self, root):
        self.root = root
        self.sequence = 0
        plan = json.loads((root / "plan.json").read_text())
        client_root = Path(plan.get("input_client_root", str(root)))
        self.process = subprocess.Popen([plan["bun"], str(client_root / "client.mjs"), str(client_root)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=(root / "input-client.stderr").open("ab"), text=True, bufsize=1)
        assert select.select([self.process.stdout], [], [], 25)[0], "input client connection unknown"
        ready = json.loads(self.process.stdout.readline())
        assert ready.get("ready"), "input client not ready"

    def call(self, action, args=None):
        self.sequence += 1
        self.process.stdin.write(json.dumps({"id": self.sequence, "action": action, "args": args or {}}) + "\n")
        self.process.stdin.flush()
        assert select.select([self.process.stdout], [], [], 25)[0], "input operation unknown; never replay"
        line = self.process.stdout.readline()
        assert line, "input client exited"
        row = json.loads(line)
        assert row.get("id") == self.sequence and "error" not in row, "input operation failed; inspect private evidence"
        return row["value"]

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=10)


def sealed(root):
    return all(sha(root / name) == digest for name, digest in json.loads((root / "sealed-inputs.json").read_text()).items())


def native_work(root):
    records = []
    state = Path(json.loads((root / "info.json").read_text())["state"])
    for path in state.glob("*.json"):
        try:
            envelope = json.loads(path.read_text())
        except (ValueError, OSError):
            continue
        row = envelope.get("payload") if isinstance(envelope, dict) else None
        if isinstance(row, dict) and row.get("schema") == "termux-codex-operator-work-v2":
            # Archived controllers carry the same schema; only the current
            # journal key is an authoritative work binding.
            key = hashlib.sha256(json.dumps(["operator-work-v1", row["id"]], separators=(",", ":")).encode()).hexdigest()
            if path.stem == key:
                records.append(row)
    return records


def observer_argv(info, record):
    """Attach the original TUI to the execution owner's exact Core backend."""
    directory = Path(record["execution"]["directory"])
    assert directory.parent == Path(info["fixture_home"]) / ".local/share/codex/core/catalogs"
    socket = directory / "s"
    paths = server_paths(Path(info["fixture_home"]), info["runtime"]) if info.get("runtime") else server_paths(Path(info["fixture_home"]))
    assert socket in paths, "observer backend unavailable"
    owner = record["execution"]["process"]
    if info.get("runtime"):
        assert Path(owner["image"]) == Path(info["runtime"]), "observer installed generation changed"
    proc = Path("/proc", str(owner["pid"]))
    assert (proc / "exe").resolve() == Path(owner["image"])
    assert (proc / "stat").read_text().rsplit(")", 1)[1].split()[19] == str(owner["start"])
    # Process survival does not prove that the TUI can get past onboarding.
    # Inspect only the account presence/requirement; never retain account data.
    from qualify_tasks import Ws
    ws = Ws(str(socket), uri_path="/rpc")
    try:
        account = ws.call("account/read", {"refreshToken":False})
        require_observer_account(account)
    finally:
        ws.close()
    return [info["core"], "--remote", "unix://" + str(socket), "--no-alt-screen", "--profile", "chatgpt", "resume", record["thread"]]


def require_observer_account(account):
    assert account.get("requiresOpenaiAuth") is False or account.get("account") is not None, "observer_native_login_required_before_cli_capture"


def prepare_fixture_observer_profile(home, fixture_root, routing):
    """Explicit setup of a new disposable home, never an existing user config."""
    home, fixture_root = Path(home), Path(fixture_root)
    assert fixture_root.resolve() == fixture_root and home.resolve() == home
    assert fixture_root.is_relative_to(Path(os.environ['TMPDIR']).resolve())
    assert home.is_dir() and home.is_relative_to(fixture_root) and home != fixture_root
    assert routing.startswith(b'# codex-chatgpt-provider generated v1\n')
    config = tomllib.loads(routing.decode())
    assert config['model_provider'] == 'chatgpt_desktop'
    assert config['model_providers']['chatgpt_desktop']['requires_openai_auth'] is False
    target = home/'config.toml'
    if target.exists() or target.is_symlink():
        assert not target.is_symlink() and target.read_bytes() == routing, 'existing_fixture_config_preserved'
        return {'reused':True,'config_sha256':sha(target),'model_requests':0}
    with target.open('xb') as stream:
        os.fchmod(stream.fileno(),0o600)
        stream.write(routing)
        stream.flush()
        os.fsync(stream.fileno())
    return {'reused':False,'config_sha256':sha(target),'model_requests':0}


def observer_env(env, record):
    """The socket and client configuration must belong to the same native work."""
    return {**env, "CODEX_HOME":record["execution"]["home"]}


def inline_emphasis_visible(text):
    """Render paired emphasis; preserve unpaired arithmetic and code contents."""
    code = []
    def protect(match):
        code.append(match[0])
        return '\ue000'+str(len(code)-1)+'\ue001'
    rendered = re.sub(r'`+[^`]*`+', protect, text)
    rendered = re.sub(r'(?<![\\*])\*\*(?![\s*])(.+?)(?<![\s*])\*\*(?!\*)', r'\1', rendered)
    rendered = re.sub(r'(?<![\\*])\*(?![\s*])([^*\n]+?)(?<!\s)\*(?!\*)', r'\1', rendered)
    return re.sub('\ue000([0-9]+)\ue001', lambda match: code[int(match[1])], rendered)


def cli_final_visible(screen, text):
    """Match all final-answer text in order after its last terminal heading.

    The original TUI renders Markdown, including table borders. Do not require
    its display bytes to contain Markdown delimiters; preserve every cell value.
    """
    plain = re.sub(r'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)', '', screen)
    plain = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', plain)
    plain = re.sub(r'\x1b[@-_]', '', plain)
    visible = re.sub(r"\s+", "", plain)
    segments = []
    code_fence = None
    for line in text.splitlines():
        fence = re.match(r'^\s*(`{3,}|~{3,})', line)
        if fence:
            if code_fence is None:
                code_fence = fence[1][0]
            elif code_fence == fence[1][0]:
                code_fence = None
            continue
        if code_fence is None:
            line = re.sub(r'^#{1,6}\s+', '', line)
            line = re.sub(r'^[-*+]\s+', '', line)
        cells = line.strip().strip('|').split('|') if code_fence is None and line.lstrip().startswith('|') else [line]
        for cell in cells:
            variants = {cell} if code_fence is not None else {
                variant
                for rendered in (cell, inline_emphasis_visible(cell))
                for variant in (rendered, rendered.replace('`', ''))}
            values = {re.sub(r"\s+", "", value) for value in variants}
            values = {value for value in values if value and not re.fullmatch(r'[-:]+', value)}
            if values:
                segments.append(values)
    if not segments:
        return False
    # A final may repeat its first line internally (for example a rich layout
    # tag). The last such line need not be the beginning of the whole answer.
    starts = sorted({position for value in segments[0] for position in [visible.find(value)] if position >= 0})
    while starts:
        start = starts.pop(0)
        position = start
        for variants in segments:
            matches = [(found, len(value)) for value in variants if (found := visible.find(value, position)) >= 0]
            if not matches:
                break
            found, length = min(matches)
            position = found + length
        else:
            return True
        starts = sorted({found for value in segments[0] if (found := visible.find(value, start + 1)) >= 0})
    return False


def cli_completion_proof(screen, history):
    # A PTY is a terminal stream, not a newline-delimited transcript. Cursor
    # movement may place the prompt after the marker in the captured raw line.
    # Match each *entire* canonical final answer including its marker, ignoring
    # terminal controls and wrapping whitespace, rather than a raw-line anchor.
    plain = re.sub(r'\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)', '', screen.decode(errors="replace"))
    plain = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', plain)
    plain = re.sub(r'\x1b[@-_]', '', plain)
    flattened = re.sub(r"\s+", "", plain)
    for notice in ["This conversation is open in another app", "This conversation is read-only or unavailable"]:
        assert re.sub(r"\s+", "", notice) not in flattened, "native CLI observer lost its execution binding"
    finals = [item["text"] for turn in history["thread"]["turns"] for item in turn["items"]
        if item["type"] == "agentMessage" and item.get("phase") == "final_answer"]
    for step in [1, 2]:
        marker = f"CHAT_ENTRY_TURN_{step}_COMPLETE"
        answer = [text for text in finals if marker in text]
        assert len(answer) == 1, "canonical final answer boundary missing"
        assert answer[0].splitlines()[-1].strip() == marker, "canonical final marker line missing"
        assert re.sub(r"\s+", "", answer[0]) in flattened, "native CLI complete final answer missing"
    return {"both_complete_canonical_final_answers_visible": True, "both_completion_markers_visible": True,
        "canonical_standalone_marker_lines": True, "terminal_controls_and_wrapping_normalized": True,
        "ownership_error": False}


def quality(root, info, env):
    workspace = Path(info["workspace"])
    assert (workspace / "test_original.py").read_bytes() == (root / "baseline/test_original.py").read_bytes()
    suite = subprocess.run([sys.executable, "-B", "-m", "unittest", "discover", "-v"], cwd=workspace, env=env, capture_output=True, text=True, timeout=60)
    (root / "tests.log").write_text(suite.stdout + suite.stderr)
    assert suite.returncode == 0, "project tests failed"
    check = subprocess.run([sys.executable, "-B", "-c", '''import importlib.util,json,subprocess,sys
from ledger import total,apply_batch
assert total([10**400,-10**400,7])==7
assert total(iter([2,3]))==5
for invalid in [[True],[1.5],["2"]]:
 try: total(invalid)
 except ValueError: pass
 else: raise AssertionError("invalid total accepted")
changes=[-2,5];assert apply_batch(4,changes)==7 and changes==[-2,5]
for b,c in [(1,[-2,3]),(True,[]),(1,[False]),(1.0,[])]:
 try: apply_batch(b,c)
 except ValueError: pass
 else: raise AssertionError("invalid batch accepted")
for argv,data,value in [([],"[2,3,-1]",4),(["--balance","4"],"[-2,5]",7)]:
 p=subprocess.run([sys.executable,"-B","ledger.py",*argv],input=data,text=True,capture_output=True)
 assert p.returncode==0 and json.loads(p.stdout)=={"total":value},(p.returncode,p.stdout,p.stderr)
for argv,data in [([],"[true]"),([],"{"),([],"{}"),(["--balance","1"],"[-2,3]")]:
 p=subprocess.run([sys.executable,"-B","ledger.py",*argv],input=data,text=True,capture_output=True)
 assert p.returncode==2 and "Traceback" not in p.stderr and not p.stdout.strip(),(p.returncode,p.stdout,p.stderr)
print("independent API/CLI cases passed")
'''], cwd=workspace, env=env, capture_output=True, text=True, timeout=60)
    (root / "independent-checks.log").write_text(check.stdout + check.stderr)
    assert check.returncode == 0, "independent API/CLI checks failed"
    diff = subprocess.run(["git", "diff", "--check"], cwd=workspace, env=env, capture_output=True)
    assert diff.returncode == 0
    (root / "actual.diff").write_bytes(subprocess.check_output(["git", "diff"], cwd=workspace, env=env))
    full = []
    for name in sorted({p.name for p in workspace.glob("*.py")} | {p.name for p in (root / "baseline").glob("*.py")}):
        before, after = root / "baseline" / name, workspace / name
        full.extend(difflib.unified_diff(before.read_text().splitlines(True) if before.exists() else [],
            after.read_text().splitlines(True) if after.exists() else [],
            fromfile="a/" + name if before.exists() else "/dev/null", tofile="b/" + name if after.exists() else "/dev/null"))
    (root / "actual-full.diff").write_text("".join(full))
    snapshot = root / "final-workspace"
    shutil.copytree(workspace, snapshot, ignore=shutil.ignore_patterns(".git", "__pycache__"))


def restore(root, info, env):
    te = json.loads((root / "tunnel-environment-private.json").read_text())
    tunnel = invoke(root, "trial-tunnel-before-close", ["tunnel-client", "runtimes", "status", json.loads((root / "info.json").read_text())["tunnel_alias"], "--json"], te)
    assert tunnel["profile_dir"] == str(root / "profiles"), "tunnel owner changed"
    invoke(root, "trial-tunnel-stop", ["tunnel-client", "runtimes", "stop", json.loads((root / "info.json").read_text())["tunnel_alias"], "--json"], te)
    invoke(root, "trial-provider-stop", [info["binary"], "stop", "--state", info["state"]], env)
    p = subprocess.run([sys.executable, "-B", str(root / "manage.py"), "prior-service", "start"], capture_output=True, text=True, timeout=150)
    (root / "restore.log").write_text(p.stdout + p.stderr)
    assert p.returncode == 0, "prior restoration needs reconciliation"
    prior = json.loads((root / "prior-service/info.json").read_text())
    previous = json.loads((root / "prior-info-private.json").read_text())
    assert all(prior[k] == previous[k] for k in ["binary", "sha256", "state", "home", "workspace", "port"])
    plan = json.loads((root / "plan.json").read_text())
    p = subprocess.run([plan["bun"], str(root / "refresh.mjs"), str(root / "prior-service")], capture_output=True, text=True, timeout=150)
    (root / "restore-refresh.log").write_text(p.stdout + p.stderr)
    assert p.returncode == 0, "restored connector readiness unconfirmed"
    save(root / "connection-restored.json", {"same_original_binary_state_home_workspace_port": True, "info": str(root / "prior-service/info.json"), "model_requests": 0, **stamp()})


def notify(root, result):
    b = json.loads((root / "notification-binding.json").read_text())
    assert sha(b["core"]) == b["core_sha256"]
    message = "[CHAT_ENTRY_RESULT] " + ("자동 검사 완료·최종 검수 대기" if result.get("automated_gates") else "실패·보존 상태 검수 필요") + "\n결과: " + str(root / "result.json") + "\n승인된 ChatGPT 진입점 실증의 실제 diff·검사·원래 CLI·native 기록·저장 Chat을 대조해 보고하세요. 불확실한 요청을 재전송하거나 시험을 새로 시작하지 마세요."
    save(root / "notification-intent.json", {"message": message, "thread_id": b["thread_id"], **stamp()}, True)
    p = subprocess.run([b["core"], "queue", "--thread", b["thread_id"], "--message", message], env=b["environment"], capture_output=True, text=True, timeout=60)
    (root / "notification.stdout").write_text(p.stdout)
    (root / "notification.stderr").write_text(p.stderr)
    save(root / "notification-receipt.json", {"exit": p.returncode, "queue_accepted": p.returncode == 0, "receiver_verified": False, **stamp()})


def run(root):
    assert sealed(root), "sealed inputs changed"
    save(root / "run-started.json", {"pid": os.getpid(), **stamp()}, True)
    info = json.loads((root / "info.json").read_text())
    env = json.loads((root / "environment-private.json").read_text())
    plan = json.loads((root / "plan.json").read_text())
    client = None
    terminal = None
    master = None
    screen = bytearray()
    observations = []
    initial_usage = resource.getrusage(resource.RUSAGE_SELF)
    result = {"automated_gates": False, "root": str(root), "started": stamp(), "scope": plan["scope"], "supervisory_model_calls_between_tasks": 0}
    try:
        client = InputClient(root)
        for step in [1, 2]:
            assert sealed(root)
            continuing = step == 1 and bool(plan.get("input_client_root"))
            if continuing:
                snap = client.call("snapshot")
                assert [v["step"] for v in snap["submissions"]] == [1], "resume source boundary changed"
                assert len(snap["requests"]) <= plan["max_source_requests"]
                started = json.loads((Path(plan["input_client_root"]) / "turn-1-intent.json").read_text())
                assert len(native_work(root)) == 1, "resume native work boundary changed"
                save(root / "source-resumed-private.json", snap)
            else:
                started = stamp()
            save(root / f"turn-{step}-intent.json", started, True)
            if not continuing:
                client.call("submit", {"step": step, "text": (root / f"request-{step}.txt").read_text()})
            until = time.monotonic() + plan["task_timeout_seconds"]
            last_snapshot = None
            while time.monotonic() < until:
                if master is not None and select.select([master], [], [], 0)[0]:
                    try:
                        screen.extend(os.read(master, 1024 * 1024))
                    except OSError:
                        pass
                    (root / "original-cli.txt").write_bytes(screen)
                works = native_work(root) if not observations else []
                # thread/start registration precedes its first durable turn.
                # An empty thread has no rollout for an upstream TUI to resume.
                if works and works[0].get("native_turn") and not observations:
                    assert len(works) == 1
                    work = {"work_id": works[0]["id"], "thread_id": works[0]["thread"]}
                    observations.append(work)
                    save(root / "native-work.json", work)
                    master, slave = pty.openpty()
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 42, 180, 0, 0))
                    argv = observer_argv(info, works[0])
                    save(root / "original-cli-binding.json", {"argv": argv, "execution": works[0]["execution"], "thread_id": work["thread_id"]})
                    terminal = subprocess.Popen(argv, cwd=info["workspace"], env=observer_env(env, works[0]),
                        stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
                    os.close(slave)
                snap = client.call("snapshot")
                last_snapshot = snap
                save(root / "input-snapshot-private.json", snap)
                error = snap.get("error")
                if error == "mcp_host_confirmation_required":
                    client.call("approve")
                elif error:
                    raise AssertionError("source Chat failed: " + str(error))
                if not snap.get("pending") and not snap.get("streamActive") and snap.get("accepted"):
                    finals = ["\n".join(v for v in m.get("content", {}).get("parts", []) if isinstance(v, str))
                        for m in snap.get("messages", []) if m.get("author", {}).get("role") == "assistant"
                        and m.get("channel") in [None, "final"] and m.get("end_turn") is True and m.get("status") == "finished_successfully"]
                    if any(f"CHAT_ENTRY_TURN_{step}_COMPLETE" in text.splitlines() for text in finals):
                        assert observations, "no native work"
                        state = control(info, env, "codex_status", {"work_id": observations[0]["work_id"]})
                        if state.get("completed"):
                            assert not state["pending_calls"] and not state["human_requests"] and state["driver_phase"] == "none"
                            save(root / f"native-complete-{step}-private.json", state)
                            break
                    elif snap.get("terminals") == ["done"]:
                        raise AssertionError("source Chat ended without native completion marker")
                time.sleep(2)
            else:
                raise AssertionError("task deadline; preserve and reconcile, never resend")
            ended = stamp()
            save(root / f"turn-{step}-complete.json", {"started": started, "ended": ended, "elapsed_seconds": (ended["monotonic_ns"]-started["monotonic_ns"])/1e9})
            save(root / f"source-complete-{step}-private.json", last_snapshot)
            with (root / "turns.md").open("a") as stream:
                stream.write(f'\n## 직결mcp · {step}턴\n\n' + '\n\n'.join(finals) + f'\n\n소요시간: {(ended["monotonic_ns"]-started["monotonic_ns"])/1e9:.3f}초 (완료 관측까지)\n')
        saved = client.call("saved", {"title": "진입점-직결mcp:개발1~2·Provider앱호스트없음"})
        save(root / "saved-chat-private.json", saved)
        assert len(saved["submissions"]) == 2
        assert len(saved["requests"]) <= plan["max_source_requests"]
        metadata = [m.get("metadata", {}) for m in saved["public_messages"] if m.get("author", {}).get("role") == "assistant"]
        models = [m.get("model_slug") for m in metadata]
        assert plan["host_model"] in models and all(m in [None, plan["host_model"]] for m in models), "unexpected source model"
        assert all(m.get("thinking_effort") in [None, plan["thinking_effort"]] for m in metadata), "unexpected source effort"
        quality(root, info, env)
        history = native_rpc(Path(info["fixture_home"]), "thread/read", {"threadId": observations[0]["thread_id"], "includeTurns": True})
        save(root / "canonical-thread-private.json", history)
        if master is not None:
            until = time.monotonic() + 3
            while time.monotonic() < until and select.select([master], [], [], .2)[0]:
                try:
                    screen.extend(os.read(master, 1024 * 1024))
                except OSError:
                    break
            (root / "original-cli.txt").write_bytes(screen)
            save(root / "original-cli-completion-proof.json", cli_completion_proof(screen, history))
        state = control(info, env, "codex_status", {"work_id": observations[0]["work_id"]})
        assert state["driver_phase"] == "none" and not state["pending_calls"]
        stopped = control(info, env, "codex_stop", {"work_id": observations[0]["work_id"]})
        save(root / "native-stop-private.json", stopped)
        assert stopped["native_stop_confirmed"] and not stopped["remaining_jobs"]
        assert not list(Path(info["state"]).glob("host-*")), "unexpected app host artifacts"
        assert sealed(root)
        client.call("dispose")
        client.close()
        client = None
        if terminal is not None and terminal.poll() is None:
            stop_owned_pid(terminal.pid, Path("/proc", str(terminal.pid), "exe").resolve())
            terminal.wait(timeout=15)
        servers = []
        expected = Path(info["fixture_home"]) / ".local/lib/codex/core/generations/local-permission-fixture/runtime"
        for path in server_paths(Path(info["fixture_home"])):
            pid = int((path.parent / "pid").read_text())
            if pid not in [s["pid"] for s in servers]:
                servers.append(stop_owned_pid(pid, expected))
        save(root / "native-servers-stopped.json", {"servers": servers, "user_files_removed": False})
        service_root = Path(plan.get("input_client_root", str(root)))
        restore(service_root, info, env)
        if service_root != root:
            shutil.copy2(service_root / "connection-restored.json", root / "connection-restored.json")
        result.update(automated_gates=True, native_thread=observations[0]["thread_id"], saved_chats=1, user_turns=2,
            source_requests=len(saved["requests"]), provider_app_host=False, native_driver_phase=state["driver_phase"], sealed_inputs_unchanged=True)
    except Exception as error:
        result["failure"] = str(error)
        try:
            if client is not None:
                save(root / "failure-source-private.json", client.call("snapshot"))
        except Exception as observation_error:
            result["source_observation_error"] = str(observation_error)
    finally:
        if terminal is not None and terminal.poll() is None:
            result["cli_preserved_for_reconciliation"] = True
        try:
            if client is not None:
                client.close()
        except Exception:
            pass
        result["finished"] = stamp()
        usage = resource.getrusage(resource.RUSAGE_SELF)
        result["controller_cpu_seconds"] = usage.ru_utime + usage.ru_stime - initial_usage.ru_utime - initial_usage.ru_stime
        result["controller_peak_rss_kib"] = usage.ru_maxrss
        save(root / "result.json", result)
        save(Path(plan["cache"]) / "result.json", result)
        notify(root, result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "setup", "start", "run"])
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    os.umask(0o077)
    if args.action == "prepare":
        prepare(args.root)
    elif args.action == "setup":
        setup(args.root)
    elif args.action == "start":
        assert sealed(args.root) and (args.root / "setup-complete.json").exists()
        save(args.root / "launch-intent.json", stamp(), True)
        p = subprocess.Popen([sys.executable, "-B", str(args.root / "run.py"), "run", str(args.root)],
            stdin=subprocess.DEVNULL, stdout=(args.root / "controller.log").open("ab"), stderr=subprocess.STDOUT, start_new_session=True)
        print(json.dumps({"started": True, "pid": p.pid, "root": str(args.root), "models_submitted_by_start_command": 0}))
    else:
        run(args.root)


if __name__ == "__main__":
    main()
