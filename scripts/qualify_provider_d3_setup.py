#!/usr/bin/env python3
"""D3 setup qualification with local fixtures only; never contact a model."""
import argparse
import json
import os
from pathlib import Path
import selectors
import shutil
import subprocess
import sys
import tempfile
from unittest.mock import patch

import qualify_provider_d3 as controller
from qualify_provider_d3 import AppendWatch, Trial, long_result_proof, regression_proof, save, sha, stop_owned_pid
from d3_contract import check


def reference(baseline, work):
    """An independent positive control; never copied into any model workspace."""
    shutil.copytree(baseline, work)
    parse = work / "src/parse.rs"
    text = parse.read_text().replace('Add(String, i64),', 'Add(String, i64),\n    Transfer(String, String, i64),')
    text = text.replace('match fields.as_slice() {', '''match fields.as_slice() {
            ["transfer", from, to, n] => commands.push(Command::Transfer(from.to_string(), to.to_string(),
                n.parse().map_err(|_| format!("line {}: amount", index + 1))?)),''')
    parse.write_text(text)
    (work / "src/ledger.rs").write_text('''use std::collections::BTreeMap;
use crate::parse::Command;
pub type Ledger = BTreeMap<String, i64>;
pub fn apply(state: &mut Ledger, commands: &[Command]) -> Result<(), String> {
 let mut next = state.clone();
 for command in commands { match command {
  Command::Add(account,n) => {
   let value=next.get(account).copied().unwrap_or(0).checked_add(*n).ok_or("overflow")?;
   if value<0 {return Err("negative".into())} next.insert(account.clone(),value);
  }
  Command::Transfer(from,to,n) => {
   let balance=*next.get(from).ok_or("unknown source")?;
   if *n<=0 || *n>balance {return Err("funds".into())}
   if from!=to {
    let target=next.get(to).copied().unwrap_or(0).checked_add(*n).ok_or("overflow")?;
    next.insert(from.clone(),balance-n);next.insert(to.clone(),target);
   }
  }
 }} *state=next;Ok(())
}
''')
    main = work / "src/main.rs"
    original = main.read_text().split('#[cfg(test)]')[1]
    main.write_text('''mod parse; mod ledger; mod report;
use std::io::{self,Read,Write};
use std::path::PathBuf;
fn run()->Result<String,String> {
 let mut state_path=None;let mut dry=false;
 let mut args=std::env::args().skip(1);
 while let Some(arg)=args.next() {match arg.as_str() {
  "--state" if state_path.is_none()=>state_path=Some(PathBuf::from(args.next().ok_or("path")?)),
  "--dry-run" if !dry=>dry=true,_=>return Err("arguments".into())
 }}
 let mut state=ledger::Ledger::new();
 if let Some(path)=state_path.as_ref() {
  let text=match std::fs::read_to_string(path) {Ok(t)=>t,Err(e) if e.kind()==io::ErrorKind::NotFound=>String::new(),Err(e)=>return Err(e.to_string())};
  for line in text.lines() {
   let f:Vec<_>=line.split('\\t').collect();
   if f.len()!=2 || f[0].is_empty() || f[0].split_whitespace().count()!=1 {return Err("record".into())}
   let n:i64=f[1].parse().map_err(|_|"balance")?;
   if n<0 || state.insert(f[0].to_string(),n).is_some(){return Err("record".into())}
  }
 }
 let mut text=String::new();io::stdin().read_to_string(&mut text).map_err(|e|e.to_string())?;
 ledger::apply(&mut state,&parse::parse(&text)?)?;
 let output=report::render(&state);
 if let Some(path)=state_path.filter(|_|!dry) {
  let temp=path.with_file_name(format!(".d3-owned-{}",std::process::id()));
  let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|e|e.to_string())?;
  let result=(||{f.write_all(output.as_bytes())?;f.sync_all()?;std::fs::rename(&temp,&path)})();
  if result.is_err() {let _=std::fs::remove_file(&temp);}
  result.map_err(|e:io::Error|e.to_string())?;
 }
 Ok(output)
}
fn main(){match run(){Ok(text)=>print!("{text}"),Err(e)=>{eprintln!("error: {e}");std::process::exit(2)}}}
#[cfg(test)]''' + original)


def automatic_three_turn_control(work, baseline, fault=False, candidate_case=None, mode="model_adapter"):
    """Exercise the real PTY/inotify/turn loop with a local deterministic peer."""
    root = work / ("automatic-fault-control-" + mode if fault else "automatic-control")
    case = root / "normal_adapter"
    case.mkdir(parents=True)
    shutil.copytree(baseline, case / "w")
    save(root / "plan.json", {"task_timeout_seconds": 15, "automatic_fault_recovery": fault,
                             "order": [case.name], "project": {"id": "local-control"}})
    save(root / "inputs.json", {"helper_inputs": {}, "files": {}})
    (root / "prior-service").mkdir()
    save(root / "prior-service/info.json", {})
    save(root / "prior-service/environment-private.json", {})
    count = 2 if fault else 3
    finals = ["D3_A1_INDETERMINATE", "D3_A2_COMPLETE"] if fault else ["D3_B1_COMPLETE", "D3_B2_COMPLETE", "D3_C1_COMPLETE"]
    if fault:
        stub = case / "mcp-stdio"
        stub.write_text(f'''#!{sys.executable}
import json,sys
for line in sys.stdin:
 r=json.loads(line)
 if 'id' not in r:continue
 if r['method']=='initialize':v={{'protocolVersion':'2025-06-18','capabilities':{{}},'serverInfo':{{'name':'local-peer','version':'1'}}}}
 elif r['method']=='tools/list':v={{'tools':[{{'name':x}} for x in ['native_catalog','native_tool']+{[f'control_{i}' for i in range(12)] if mode == 'operator' else []!r}]}}
 else:v={{'isError':False,'content':[{{'text':json.dumps({{'id':'local-probe','name':'functions.exec_command','output':'D3_EFFECT_RETURNED_FAULT\\n'}})}}]}}
 print(json.dumps({{'jsonrpc':'2.0','id':r['id'],'result':v}}),flush=True)
''')
        stub.chmod(0o700)
    script = root / "fake_cli.py"
    script.write_text(f'''#!{sys.executable}
import json,os,pathlib,time,tty,subprocess,sys
tty.setraw(0)
path=pathlib.Path(os.environ['D3_CONTROL_RECORD'])
path.parent.mkdir(parents=True,exist_ok=True)
with path.open('a') as f:f.write(json.dumps({{'type':'session_meta','payload':{{'id':'local-control'}}}})+'\\n')
os.write(1,b'GPT-6 ready')
text=b''
while b'\\r' not in text:text+=os.read(0,65536)
def emit(kind,payload):
 with path.open('a') as f:f.write(json.dumps({{'type':kind,'payload':payload}})+'\\n')
emit('event_msg',{{'type':'task_started','turn_id':'local-1'}})
if {fault!r}:
 case=pathlib.Path({str(case)!r})
 result=subprocess.run([sys.executable,'-B','probe.py','--root','../effects','--label','fault'],cwd=case/'w',capture_output=True,text=True,check=True)
 args=json.dumps({{'cmd':'python3 -B probe.py --root ../effects --label fault'}})
 emit('response_item',{{'type':'function_call','name':'exec_command','namespace':'functions','arguments':args,'call_id':'local-probe'}})
 emit('response_item',{{'type':'function_call_output','output':result.stdout,'call_id':'local-probe'}})
 request={{'jsonrpc':'2.0','id':1,'method':'tools/call','params':{{'name':'native_tool','arguments':{{'name':'functions.exec_command','input':args}}}}}}
 cut=subprocess.run([sys.executable,'-B',{str(Path(controller.__file__).with_name('d3_fault_proxy.py'))!r},'--root',str(case),'--',str(case/'mcp-stdio')],input=json.dumps(request).encode()+b'\\n',capture_output=True)
 assert cut.returncode==42 and not cut.stdout
emit('event_msg',{{'type':'error','turn_id':'local-1','message':'intermediate transport error; completion follows'}})
time.sleep(.1)
emit('event_msg',{{'type':'task_complete','turn_id':'local-1','last_agent_message':{finals[0]!r}+'\\n소요시간: 약 0초'}})
while True:time.sleep(60)
''')
    script.chmod(0o700)
    home = root / "h/.codex"
    record = home / "sessions/rollout.jsonl"
    save(case / "info.json", {"core": str(script), "workspace": str(case / "w"), "home": str(home), "mode": mode,
                             "binary": str(Path(sys.executable).resolve()), "pid": os.getpid(), "state": "local-owned-state"})
    save(case / "environment-private.json", {**os.environ, "D3_CONTROL_RECORD": str(record)})
    for step in range(1, count + 1):
        (case / f"request-{step}.txt").write_text(f"local synthetic task {step}")
    save(case / "tasks.json", {"kind": "fault" if fault else "normal", "count": count,
                              "prompt_hashes": [sha(case / f"request-{i}.txt") for i in range(1, count + 1)]})
    subprocess.run(["git", "init", "-q"], cwd=case / "w", check=True)
    captures = []
    reviews = []
    submits = []
    recoveries = []
    admission = {"alias": "local-fixture-tunnel", "profile_dir": str(case / "profiles"),
                 "tunnel_id": "local-owned-tunnel", "config_path": "owned-config", "profile_path": "owned-profile",
                 "process_running": True, "ready": True, "healthy": True,
                 "process": {"target_kind": "command", "target_value": str(case / "mcp-stdio"), "started_at": "initial"}}
    save(case / "tunnel-ready-private.json", admission)
    connected = []
    injections = []
    restored = []

    def local_operation(command, env):
        if command[0] == str(Path(sys.executable).resolve()):
            return {"live": True, "process": {"pid": os.getpid(), "image": command[0]}}
        assert command[:2] == ["tunnel-client", "runtimes"]
        if command[2] == "connect":
            assert not connected, "Connect repeated"
            connected.append(1)
            return {"connected": True}
        assert command[2] == "status", "already-dead tunnel should not need Stop"
        return {**admission, "process_running": bool(connected), "ready": bool(connected), "healthy": bool(connected),
                "process": {**admission["process"], "started_at": "reconnected" if connected else "initial"}}

    def local_rpc(home, method, args):
        assert args["threadId"] == "local-control"
        if method == "thread/read":
            return {"thread": {"id": "local-control", "status": {"type": "idle"}, "turns": [{"id": "local-1", "status": "completed", "items": [
                {"id": "local-probe", "type": "commandExecution", "command": "python3 -B probe.py --root ../effects --label fault", "cwd": str(case / "w"),
                 "status": "completed", "aggregatedOutput": "D3_EFFECT_RETURNED_FAULT", "exitCode": 0}]}]}}
        if method == "thread/backgroundTerminals/list":
            return {"data": [], "nextCursor": None}
        if method == "thread/inject_items":
            assert not injections and "D3_EFFECT_RETURNED_FAULT" in json.dumps(args["items"])
            injections.append(1)
            return {}
        assert method == "turn/start"
        step = len(submits) + 2
        assert captures == list(range(1, step)), "preceding evidence capture missing"
        assert not reviews, "quality review ran between tasks"
        if fault:
            assert recoveries == [1], "A2 submitted before owned connection recovery"
            assert connected == [1] and injections == ([1] if mode == "model_adapter" else [])
            assert (case / "effects/fault.effects").read_text() == "fault\n"
            (case / "w/recovery-note.txt").write_text("effect_count: 1\nD3_EFFECT_RETURNED_FAULT\n")
        submits.append(step)
        with record.open("a") as stream:
            for event in [{"type": "task_started", "turn_id": f"local-{step}"},
                          {"type": "task_complete", "turn_id": f"local-{step}", "last_agent_message": finals[step - 1] + "\n소요시간: 약 0초"}]:
                stream.write(json.dumps({"type": "event_msg", "payload": event}, ensure_ascii=False) + "\n")
        return {"turn": {"id": f"local-{step}"}}

    class LocalTrial(Trial):
        def service(self, script, *args):
            if script == "manage.py" and args == ("prior-service", "start"):
                assert captures == list(range(1, count + 1))
                restored.append(1)
            if script == "backend.mjs" and args[-1] == "read":
                save(case / "read-backend-private.json", {"requests": list(range(count)), "saved": [{"isTemporary": False,
                     "metadata": [{"model": "gpt-6-thinking", "effort": "extended"}],
                     "publicMessages": [{"author": {"role": "assistant"}, "channel": "final", "content": {"parts": [value + "\n소요시간: 약 0초"]}} for value in finals]}]})
            if script == "app_setup.mjs":
                save(case / "titles.json", [{"project": "local-control", "content_unchanged": True}])

        def settled(self, recoverable_cut=False):
            assert self.calls.keys() == self.returns.keys()
            if mode == "operator":
                self.work = {"work_id": "local-work"}

        def tool(self, name, args):
            assert name == "codex_stop"
            return {"native_stop_confirmed": True, "host_stop_confirmed": True, "remaining_jobs": []}

        def boundary_review(self, step, fault):
            raise AssertionError("quality review must wait until all cases end")

        def capture_boundary(self, step, fault):
            assert fault == self.plan["automatic_fault_recovery"]
            captures.append(step)

        def recover_fault_connection(self):
            assert captures == [1] and not submits and not reviews
            super().recover_fault_connection()
            recoveries.append(1)

        def inject_recovery_history(self):
            original_info, original_env = self.info, self.env
            self.info = {**self.info, "binary": json.loads((candidate_case / "info.json").read_text())["binary"]}
            self.env = json.loads((candidate_case / "environment-private.json").read_text())
            try:
                super().inject_recovery_history()
            finally:
                self.info, self.env = original_info, original_env

        def audit_all(self):
            assert restored == [1] and captures == list(range(1, count + 1))
            for step in range(1, count + 1):
                final = json.loads((case / f"complete-{step}.json").read_text())["native_final"]
                assert final["payload"]["type"] == "task_complete"
            if fault:
                assert (case / "w/recovery-note.txt").read_text().count("D3_EFFECT_RETURNED_FAULT") == 1
                assert controller.expected_result_cut(case, self.calls, self.returns)
            reviews.append("batch")
            return True

    trial = LocalTrial(root)
    with patch.object(controller, "native_rpc", local_rpc), patch.object(controller, "stop_owned_servers"), \
            patch.object(controller, "acquire"), patch.object(controller, "control", return_value={"work": []}), \
            patch.object(controller, "local_json", local_operation), patch.object(controller, "tunnel_environment", return_value={}), \
            patch.object(controller, "tunnel_profile", return_value={"tunnel_id": "local-owned-tunnel", "api_key": "owned-fixture"}), \
            patch.object(controller, "verify_immutable_inputs") as immutable:
        trial.run()
        immutable.assert_called_once()
    assert captures == list(range(1, count + 1)) and submits == list(range(2, count + 1)) and reviews == ["batch"]
    result = json.loads((root / "result.json").read_text())
    assert result["automated_gates"] and result["unattended_control_accepted"] and not result["supervisor_resume_used"]
    assert (root / "turns.md").read_text().count("제어기 완료 관측 기준") == count
    completed = json.loads((case / "route-complete.json").read_text())
    assert completed["work_complete"] and completed["quality_review"] == "deferred_until_all_work_ends"
    assert not list(root.rglob("go-*")) and not list(root.rglob("finish"))
    return {"name": "real-controller-cut-reconnect-handoff-A2-restore-batch-" + mode if fault else "real-controller-three-turn-local-peer", "accepted": True,
            "automatic_tasks": count, "boundary_captures": captures, "intermediate_quality_reviews": 0, "model_requests": 0,
            "limits": "Real proxy/effect/stdio/checkpoint/controller paths; native RPC, tunnel service and Chat observations are local doubles. Actual Chat proof remains separate."}


def qualify(root):
    source_inputs = {str(p): sha(p) for p in [Path(controller.__file__), Path(__file__),
                     Path(__file__).with_name("d3_contract.py"), Path(__file__).with_name("d3_fault_proxy.py")]}
    work = Path(tempfile.mkdtemp(prefix="d3setup.", dir=os.environ["TMPDIR"]))
    archive = Path(json.loads((root / "plan.json").read_text())["cache"]) / "setup-validation"
    archive.mkdir(exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="attempt-", dir=archive))
    checks = []
    baseline = root / "baseline"
    checks.append({"name": "baseline", **check(baseline, 0, output / "baseline")})
    try:
        check(baseline, 1, output / "missing-feature-negative")
    except AssertionError:
        checks.append({"name": "missing-feature-negative", "accepted": True})
    else:
        raise AssertionError("baseline incorrectly accepted as transfer implementation")
    reference(baseline, work / "reference")
    for stage in (1, 2, 3):
        checks.append({"name": "reference-stage-" + str(stage), **check(work / "reference", stage, output / f"reference-{stage}")})
    shutil.copytree(work / "reference", work / "strict-parser")
    parser = work / "strict-parser/src/parse.rs"
    parser.write_text(parser.read_text().replace('match fields.as_slice() {', '''
        if fields.first()==Some(&"transfer") && fields.last().is_some_and(|n| n.parse::<i64>().is_ok_and(|v|v<=0)) {
            return Err(format!("line {}: positive amount required", index + 1));
        }
        match fields.as_slice() {'''))
    checks.append({"name": "valid-early-parse-rejection-positive", **check(work / "strict-parser", 1, output / "strict-parser")})
    (work / "reference/src/test_transfer.rs").write_text('''#[test] fn rejects_missing_transfer(){
 let mut state=crate::ledger::Ledger::new();
 crate::ledger::apply(&mut state,&crate::parse::parse("add a 2\\ntransfer a b 1").unwrap()).unwrap();
 assert_eq!(state.get("b"),Some(&1));}
''')
    with (work / "reference/src/main.rs").open("a") as stream:
        stream.write('\n#[cfg(test)] mod test_transfer;\n')
    first_case = json.loads((root / "plan.json").read_text())["order"][0]
    env = json.loads((root / first_case / "environment-private.json").read_text())
    regression_proof(work / "reference", baseline, output / "regression", env)
    checks.append({"name": "new-regression-red-and-green", "accepted": True})
    sessions = work / "sessions"
    watch = AppendWatch(sessions)
    try:
        assert watch.read() == []
        path = sessions / "2026/10/20/rollout.jsonl"
        path.parent.mkdir(parents=True)
        path.write_bytes(b'{"type":"session_')
        assert watch.read() == []
        with path.open("ab") as stream:
            stream.write(b'meta","payload":{"id":"local-only"}}\n')
        assert watch.read() == [{"type": "session_meta", "payload": {"id": "local-only"}}]
        assert watch.read() == []
        # A detached local child appends another event, with no supervision RPC.
        child = subprocess.Popen([sys.executable, "-B", "-c",
            "import sys,time;time.sleep(.1);open(sys.argv[1],'ab').write(b'{\\\"type\\\":\\\"event_msg\\\",\\\"payload\\\":{}}\\n')", str(path)],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        selector = selectors.DefaultSelector()
        selector.register(watch.fd, selectors.EVENT_READ)
        assert selector.select(5), "kernel notification absent"
        child.wait(timeout=5)
        assert watch.read() == [{"type": "event_msg", "payload": {}}]
        selector.close()
        path.write_bytes(b"")
        try:
            watch.read()
        except AssertionError:
            pass
        else:
            raise AssertionError("truncated canonical record was accepted")
    finally:
        watch.close()
    checks.append({"name": "append-partial-nested-detached-truncation", "accepted": True})
    proxy = Path(__file__).with_name("d3_fault_proxy.py")
    child = work / "fake_mcp.py"
    child.write_text('''import json,sys
for line in sys.stdin:
 r=json.loads(line)
 with open(sys.argv[1],'a') as f:f.write(str(r['id'])+'\\n')
 print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':{'content':[{'type':'text','text':'D3_EFFECT_RETURNED_FAULT'}]}}),flush=True)
''')
    case = work / "proxy"
    (case / "effects").mkdir(parents=True)
    (case / "effects/fault.effects").write_text("fault\n")
    command = [sys.executable, "-B", str(proxy), "--root", str(case), "--", sys.executable, "-B", str(child), str(case / "requests")]
    request = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "native_tool", "arguments": {
        "name": "functions.exec_command", "input": json.dumps({"cmd": "python3 -B probe.py --root ../effects --label fault"})}}}).encode() + b"\n"
    ordinary = subprocess.run(command, input=request, capture_output=True, timeout=15)
    assert ordinary.returncode == 0 and json.loads(ordinary.stdout)["id"] == 1
    (case / "fault-armed").touch()
    cut = subprocess.run(command, input=request, capture_output=True, timeout=15)
    assert cut.returncode == 42 and not cut.stdout
    fired = json.loads((case / "fault-fired.json").read_text())
    assert not fired["forwarded_to_Chat"] and fired["effect_count"] == 1
    assert (case / "withheld-mcp-result-private.json").is_file()
    assert (case / "requests").read_text() == "1\n1\n", "proxy replayed an original request"
    assert (case / "effects/fault.effects").read_text() == "fault\n"
    checks.append({"name": "MCP-pass-through-exact-boundary-EOF-no-replay", "accepted": True})
    calls = {"start": {"name": "functions.exec_command", "arguments": json.dumps({"cmd": "python3 -B probe.py --root ../effects --label long --delay 8", "yield_time_ms": 1000})},
             "collect": {"name": "functions.write_stdin", "arguments": json.dumps({"session_id": 123})}}
    returns = {"start": {"output": "Process running with session ID 123"}, "collect": {"output": "D3_EFFECT_RETURNED_LONG"}}
    long_result_proof(calls, returns)
    calls["collect"]["arguments"] = json.dumps({"session_id": 999})
    try:
        long_result_proof(calls, returns)
    except AssertionError:
        pass
    else:
        raise AssertionError("wrong background session was accepted")
    checks.append({"name": "long-result-original-session-positive-and-negative", "accepted": True})
    checks.append(automatic_three_turn_control(work, baseline))
    candidate_case = root / next(name for name in json.loads((root / "plan.json").read_text())["order"] if name.endswith("adapter"))
    for mode in ("model_adapter", "operator"):
        checks.append(automatic_three_turn_control(work, baseline, fault=True, candidate_case=candidate_case, mode=mode))
    child = subprocess.Popen([sys.executable, "-B", "-c", "import time;time.sleep(30)"],
                             stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    proof = stop_owned_pid(child.pid, Path(sys.executable))
    child.wait(timeout=5)
    assert proof["kernel_stop_confirmed"]
    checks.append({"name": "Termux-pidfd-owned-stop-kernel-readback", "accepted": True})
    # Boundary rules must distinguish a declared, independently reconciled cut
    # from ordinary app failure and from a still unreturned native call.
    boundary = Trial(root)
    boundary.case = work / "boundary"
    boundary.case.mkdir()
    (boundary.case / "effects").mkdir()
    (boundary.case / "effects/fault.effects").write_text("fault\n")
    boundary.info = {"home": str(work / "h/.codex"), "mode": "operator"}
    boundary.thread = "local-boundary"
    boundary.work = {"work_id": "local-boundary"}
    boundary.calls = {"one": {"name": "exec_command", "arguments": json.dumps({"cmd": "python3 -B probe.py --root ../effects --label fault"})}}
    good_return = {"call_id": "one", "output": "D3_EFFECT_RETURNED_FAULT"}
    boundary.returns = {"one": good_return}
    save(boundary.case / "withheld-mcp-result-private.json", {"result": {"isError": False, "content": [{"text": json.dumps(good_return)}]}})
    save(boundary.case / "fault-fired.json", {"effect_count": 1, "forwarded_to_Chat": False, "result_sha256": sha(boundary.case / "withheld-mcp-result-private.json")})
    def read_idle(home, method, args):
        return {"data": []} if method.endswith("/list") else {"thread": {"status": {"type": "idle"}}}
    observed = {"host_state": "error_observed", "phase": "reconciliation_required", "native_status": {"type": "idle"},
                "host_observation": {"error": "app_stream_failed"}, "call_observations": [{"settled": True}], "human_requests": [], "pending_calls": []}
    boundary.tool = lambda *args: observed
    with patch.object(controller, "native_rpc", read_idle):
        assert boundary.settled(recoverable_cut=True)
        try:
            boundary.settled()
        except AssertionError:
            pass
        else:
            raise AssertionError("ordinary app failure incorrectly qualified as settled")
        boundary.returns = {}
        try:
            boundary.settled(recoverable_cut=True)
        except AssertionError:
            pass
        else:
            raise AssertionError("unreturned original call incorrectly admitted recovery")
    checks.append({"name": "boundary-positive-cut-normal-error-unreturned-call", "accepted": True})
    boundary.returns = {"one": good_return}
    def read_terminal_error(home, method, args):
        return {"data": []} if method.endswith("/list") else {"thread": {"status": {"type": "systemError"}}}
    with patch.object(controller, "native_rpc", read_terminal_error):
        assert boundary.settled(recoverable_cut=True)
        try:
            boundary.settled()
        except AssertionError as error:
            assert "native boundary unsettled: systemError" in str(error)
        else:
            raise AssertionError("ordinary systemError admitted as successful boundary")
    checks.append({"name": "declared-cut-terminal-systemError-only", "accepted": True})
    observed.update(host_state="stream_settled", host_observation={"error": None}, native_status={"type": "systemError"})
    with patch.object(controller, "native_rpc", read_terminal_error):
        assert boundary.settled(recoverable_cut=True)
        (boundary.case / "effects/fault.effects").write_text("fault\nfault\n")
        try:
            boundary.settled(recoverable_cut=True)
        except AssertionError:
            pass
        else:
            raise AssertionError("duplicated effect admitted as reconciled")
        (boundary.case / "effects/fault.effects").write_text("fault\n")
    ready = {"thread_id": boundary.thread, "native_calls": 1, "native_results": 1,
             "original_call_ids": ["one"], "original_results_sha256": controller.digest_results(boundary.returns)}
    controller.reconciled_resume(ready, boundary.thread, boundary.calls, boundary.returns)
    try:
        controller.reconciled_resume(ready, boundary.thread, boundary.calls, {"one": {**good_return, "output": "changed"}})
    except AssertionError:
        pass
    else:
        raise AssertionError("changed original result admitted on resume")
    checks.append({"name": "declared-cut-positive-app-final-original-result-resume-and-duplicate-rejection", "accepted": True})
    wire = boundary.case / "withheld-mcp-result-private.json"
    boundary.calls["one"]["namespace"] = "functions"
    for receipt, accepted in [
            ({"id": "one", "name": "functions.exec_command", "output": good_return["output"]}, True),
            ({"id": "other", "name": "functions.exec_command", "output": good_return["output"]}, False),
            ({"id": "one", "name": "other.exec_command", "output": good_return["output"]}, False),
            ({"id": "one", "name": "functions.exec_command", "output": "changed"}, False)]:
        save(wire, {"result": {"isError": False, "content": [{"text": json.dumps(receipt)}]}})
        save(boundary.case / "fault-fired.json", {"effect_count": 1, "forwarded_to_Chat": False, "result_sha256": sha(wire)})
        try:
            controller.expected_result_cut(boundary.case, boundary.calls, boundary.returns)
        except AssertionError:
            assert not accepted
        else:
            assert accepted
    checks.append({"name": "adapter-qualified-wire-original-id-name-output", "accepted": True})
    boundary.active_turn = "terminal-turn"
    boundary.rows = [{"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "terminal-turn"}}]
    observed.update(native_status={"type": "idle"})
    reads = []
    def read_publication_race(home, method, args):
        if method.endswith("/list"):
            return {"data": []}
        reads.append(method)
        return {"thread": {"status": {"type": "active" if len(reads) == 1 else "idle"}}}
    with patch.object(controller, "native_rpc", read_publication_race), patch.object(boundary, "pump") as pump:
        assert boundary.settled()
        assert len(reads) == 2 and pump.call_count == 1
    for rows in ([], [{"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "another-turn"}}]):
        boundary.rows = rows
        reads.clear()
        with patch.object(controller, "native_rpc", read_publication_race), patch.object(boundary, "pump") as pump:
            try:
                boundary.settled()
            except AssertionError as error:
                assert "native boundary unsettled: active" in str(error)
            else:
                raise AssertionError("active thread without exact terminal record admitted")
            assert len(reads) == 1 and pump.call_count == 0
    checks.append({"name": "exact-terminal-turn-bounded-idle-publication-race", "accepted": True})
    before, after = work / "diff-before", work / "diff-after"
    before.mkdir()
    after.mkdir()
    (before / "file").write_text("before\n")
    (after / "file").write_text("after\n")
    controller.snapshot_diff_check(before, after)
    (after / "file").write_text("after \n")
    try:
        controller.snapshot_diff_check(before, after)
    except AssertionError as error:
        assert "trailing whitespace" in str(error)
    else:
        raise AssertionError("snapshot whitespace defect admitted")
    checks.append({"name": "no-index-clean-diff-and-whitespace-rejection", "accepted": True})
    # Exercise the actual candidate's public-history serializer without a Chat.
    handoff = Trial(root)
    handoff.case = work / "recovery-history"
    handoff.case.mkdir()
    adapter_case = next(name for name in json.loads((root / "plan.json").read_text())["order"] if name.endswith("adapter"))
    handoff.info = json.loads((root / adapter_case / "info.json").read_text())
    handoff.env = json.loads((root / adapter_case / "environment-private.json").read_text())
    handoff.thread = "local-history-recovery"
    history = {"id": handoff.thread, "status": {"type": "idle"}, "turns": [
        {"id": "old-turn", "status": "completed", "items": [
            {"id": "original-command", "type": "commandExecution", "command": "python3 -B probe.py --root ../effects --label fault",
             "cwd": "/owned/disposable", "status": "completed", "aggregatedOutput": "D3_EFFECT_RETURNED_FAULT", "exitCode": 0}]}]}
    injected = []
    def handoff_rpc(home, method, args):
        assert args["threadId"] == handoff.thread
        if method == "thread/read":
            return {"thread": history}
        if method == "thread/backgroundTerminals/list":
            return {"data": [], "nextCursor": None}
        assert method == "thread/inject_items"
        injected.append(args["items"])
        return {}
    with patch.object(controller, "native_rpc", handoff_rpc):
        handoff.inject_recovery_history()
        assert len(injected) == 1 and "<external_termux_chatgpt_handoff>" in json.dumps(injected)
        try:
            handoff.inject_recovery_history()
        except FileExistsError:
            pass
        else:
            raise AssertionError("history injection repeated after durable intent")
        assert len(injected) == 1
        history["status"]["type"] = "active"
        try:
            handoff.inject_recovery_history()
        except AssertionError:
            pass
        else:
            raise AssertionError("active history admitted as recovery boundary")
    checks.append({"name": "actual-candidate-readable-history-original-return-and-no-repeat", "accepted": True})

    # Wrong tunnel ownership must fail before Stop, Connect or context injection.
    guard = Trial(root)
    guard.case = boundary.case
    guard.thread = "local-owned-reconnect"
    guard.calls, guard.returns = boundary.calls, boundary.returns
    guard.env = {}
    guard.info = {"binary": str(Path(sys.executable).resolve()), "pid": os.getpid(), "state": "local-owned-state", "mode": "model_adapter"}
    save(wire, {"result": {"isError": False, "content": [{"text": json.dumps(good_return)}]}})
    save(guard.case / "fault-fired.json", {"effect_count": 1, "forwarded_to_Chat": False, "result_sha256": sha(wire)})
    operations = []
    def wrong_tunnel(command, env):
        operations.append(command)
        if command[0] == guard.info["binary"]:
            return {"live": True, "process": {"pid": os.getpid(), "image": guard.info["binary"]}}
        assert command[:3] == ["tunnel-client", "runtimes", "status"]
        return {"process_running": True, "profile_dir": "/foreign/owner"}
    with patch.object(controller, "local_json", wrong_tunnel), patch.object(controller, "tunnel_environment", return_value={}):
        try:
            guard.recover_fault_connection()
        except AssertionError as error:
            assert "tunnel owner changed" in str(error)
        else:
            raise AssertionError("foreign tunnel admitted recovery")
    assert len(operations) == 2 and not (guard.case / "recovery-tunnel-intent.json").exists()
    checks.append({"name": "foreign-tunnel-owner-blocks-reconnect-before-mutation", "accepted": True})
    admission = {"alias": "local-fixture-tunnel", "profile_dir": str(guard.case / "profiles"),
                 "tunnel_id": "owned-test-tunnel", "config_path": "owned-config", "profile_path": "owned-profile",
                 "process_running": True, "ready": True, "healthy": True,
                 "process": {"target_kind": "command", "target_value": str(guard.case / "mcp-stdio")}}
    save(guard.case / "tunnel-ready-private.json", admission)
    assert controller.owned_fault_tunnel(guard.case, admission)
    stopped = {**admission, "process_running": False, "ready": False, "healthy": False}
    assert controller.owned_fault_tunnel(guard.case, stopped)
    for changed in [
            {**stopped, "tunnel_id": "foreign"},
            {**stopped, "profile_dir": "/foreign/owner"},
            {**stopped, "process": {**stopped["process"], "target_value": "/foreign/stdio"}},
            {**stopped, "process": {**stopped["process"], "started_at": "different-registration"}}]:
        try:
            controller.owned_fault_tunnel(guard.case, changed)
        except AssertionError:
            pass
        else:
            raise AssertionError("changed tunnel registry admitted reconnect")
    checks.append({"name": "admitted-fault-tunnel-alive-or-stopped-same-registry-only", "accepted": True})
    for scenario in ("alive", "dead", "lost-reply-reconciled", "lost-reply-unknown", "changed-after-connect", "readiness-lag", "stop-reply-reconciled", "stop-reply-unknown"):
        case = work / ("connect-" + scenario)
        case.mkdir()
        admitted = {**admission, "profile_dir": str(case / "profiles"),
                    "process": {"target_kind": "command", "target_value": str(case / "mcp-stdio"), "started_at": "old"}}
        save(case / "tunnel-ready-private.json", admitted)
        operations, samples = [], []
        def flow(command, env):
            action = command[2]
            if action == "stop":
                operations.append(action)
                if scenario.startswith("stop-reply"):
                    raise AssertionError("owned local operation timed out; outcome unknown, no retry")
                return {"stopped": True}
            if action == "connect":
                operations.append(action)
                assert operations.count("connect") == 1
                if scenario.startswith("lost-reply"):
                    raise AssertionError("owned local operation timed out; outcome unknown, no retry")
                return {"connected": True}
            assert action == "status"
            if "connect" not in operations:
                running = scenario == "alive" or scenario.startswith("stop-reply") and ("stop" not in operations or scenario == "stop-reply-unknown")
                return {**admitted, "process_running": running, "ready": False, "healthy": False}
            samples.append(1)
            ready = scenario != "lost-reply-unknown" and (scenario != "readiness-lag" or len(samples) > 1)
            return {**admitted, "process_running": ready, "ready": ready, "healthy": ready,
                    "tunnel_id": "foreign" if scenario == "changed-after-connect" else admitted["tunnel_id"],
                    "process": {**admitted["process"], "started_at": "new" if ready else "old"}}
        with patch.object(controller, "local_json", flow), patch.object(controller, "tunnel_environment", return_value={}), \
                patch.object(controller, "tunnel_profile", return_value={"tunnel_id": admitted["tunnel_id"], "api_key": "owned-fixture"}), \
                patch.object(controller.time, "sleep"):
            try:
                controller.reconnect_fault_tunnel(case, "owned-thread", "original-results")
            except AssertionError:
                assert scenario in ("lost-reply-unknown", "changed-after-connect", "stop-reply-unknown")
            else:
                assert scenario not in ("lost-reply-unknown", "changed-after-connect", "stop-reply-unknown")
                assert json.loads((case / "recovery-tunnel-receipt.json").read_text())["connect_reply_reconciled"] == (scenario == "lost-reply-reconciled")
        assert operations.count("connect") == (scenario != "stop-reply-unknown")
        assert operations.count("stop") == (scenario == "alive" or scenario.startswith("stop-reply"))
    checks.append({"name": "one-connect-alive-dead-lost-reply-owner-change-and-readiness-lag", "accepted": True})
    for event in ("error", "task_started", "agent_message"):
        assert not controller.terminal_for_turn({"type": "event_msg", "payload": {"type": event, "turn_id": "same"}}, "same")
    assert not controller.terminal_for_turn({"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "old"}}, "same")
    assert controller.terminal_for_turn({"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "same"}}, "same")
    assert controller.completion_claimed("**D3_A2_COMPLETE**\n소요시간: 약 1초", "D3_A2_COMPLETE")
    assert not controller.completion_claimed("D3_A2_COMPLETE를 선언하지 않습니다.\n소요시간: 약 1초", "D3_A2_COMPLETE")
    checks.append({"name": "exact-turn-terminal-and-affirmative-completion-no-denial-false-positive", "accepted": True})
    immutable_root = work / "immutable-inputs"
    immutable_root.mkdir()
    immutable_plan = {"baseline": {"src/main.rs": "fixture"}, "order": ["fault_adapter"]}
    immutable_paths = ["run.py", "contract.py", "fault_proxy.py", "notify.py", "manage.py", "backend.mjs", "refresh.mjs",
                       "app_setup.mjs", "plan.json", "candidate/provider", "candidate/app_bridge.js", "baseline/src/main.rs",
                       "fault_adapter/tasks.json", "fault_adapter/request-1.txt", "fault_adapter/request-2.txt",
                       "fault_adapter/config/codex/providers/chatgpt.toml"]
    for relative in immutable_paths:
        path = immutable_root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"count": 2}) if path.name == "tasks.json" else "frozen\n")
    immutable_manifest = {"files": {relative: sha(immutable_root / relative) for relative in immutable_paths}, "helper_inputs": {}}
    controller.verify_immutable_inputs(immutable_root, immutable_plan, immutable_manifest)
    for relative in ("run.py", "candidate/provider", "fault_adapter/request-2.txt", "fault_adapter/config/codex/providers/chatgpt.toml"):
        path = immutable_root / relative
        path.write_text("changed\n")
        try:
            controller.verify_immutable_inputs(immutable_root, immutable_plan, immutable_manifest)
        except AssertionError:
            pass
        else:
            raise AssertionError("mid-run input repair qualified as unattended")
        path.write_text("frozen\n")
    checks.append({"name": "sealed-inputs-reject-mid-run-controller-product-prompt-config-repair", "accepted": True})
    with patch.object(controller.subprocess, "run", side_effect=subprocess.TimeoutExpired(["connect", "private-fixture-credential"], 45)):
        try:
            controller.local_json(["connect", "private-fixture-credential"], {})
        except AssertionError as error:
            assert "private-fixture-credential" not in str(error) and "outcome unknown" in str(error)
        else:
            raise AssertionError("unknown connection timeout admitted retry")
    checks.append({"name": "connection-timeout-unknown-without-credential-disclosure", "accepted": True})
    released = {"host_state": "unknown", "pending_calls": [], "human_requests": [],
                "driver_phase": "released_after_stop", "last_observed_host_state": "stop_confirmed", "host_observation_available": False}
    assert controller.stopped_host_reconciled(released)
    assert not controller.stopped_host_reconciled({**released, "last_observed_host_state": "unknown"})
    assert not controller.stopped_host_reconciled({**released, "pending_calls": ["unknown"]})
    assert not controller.stopped_host_reconciled({**released, "driver_phase": "started_observation_unknown"})
    checks.append({"name": "released-driver-restart-requires-persisted-positive-stop", "accepted": True})
    assert all(sha(Path(p)) == value for p, value in source_inputs.items()), "setup sources changed during qualification"
    result = {"accepted": True, "checks": checks, "groups": len(checks), "model_requests": 0,
              "live_chat_trial": False, "work": str(work), "root": str(root),
              "limits": "Local controller/contracts/proxy proof only; live Provider/Chat behavior remains untested in D3."}
    save(output / "review.json", result)
    save(archive / "review.json", {**result, "attempt": str(output),
         "controller_sha256": source_inputs[str(Path(controller.__file__))], "validator_sha256": source_inputs[str(Path(__file__).with_name("d3_contract.py"))],
         "proxy_sha256": source_inputs[str(Path(__file__).with_name("d3_fault_proxy.py"))], "source_inputs": source_inputs})
    print(json.dumps({k: result[k] for k in ("accepted", "groups", "model_requests", "live_chat_trial")}))


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    qualify(parser.parse_args().root)
