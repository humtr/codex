use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn tmp_dir(prefix: &str) -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = env::temp_dir().join(format!("{prefix}-{}-{ns}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    d
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_termux-orch"))
}

#[test]
fn cli_flags_describe_tools_and_compiled_stdio_circuit_breaker_and_replay() {
    let root = tmp_dir("termux-orch-bin");
    let state_dir = root.join("orch");
    let rcpt_dir = root.join("rcpts");

    // 1. --version and --describe-tools
    let ver = Command::new(bin_path()).arg("--version").output().unwrap();
    assert!(ver.status.success());
    assert!(String::from_utf8_lossy(&ver.stdout).starts_with("termux-orch "));

    let desc = Command::new(bin_path())
        .arg("--describe-tools")
        .output()
        .unwrap();
    assert!(desc.status.success());
    let desc_json: Value = serde_json::from_slice(&desc.stdout).unwrap();
    assert_eq!(desc_json["tools"].as_array().unwrap().len(), 6);

    // 2. Compiled stdio MCP server: initialize -> tools/list -> create -> replay -> conflict -> cancel
    let mut child = Command::new(bin_path())
        .arg("mcp")
        .env("ORCH_STATE_DIR", &state_dir)
        .env("ORCH_RECEIPT_DIR", &rcpt_dir)
        .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1")
        .env("ORCH_NOTIFY_CMD", "/bin/true")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let send = |stdin: &mut std::process::ChildStdin, reader: &mut BufReader<_>, req: Value| {
        writeln!(stdin, "{req}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };

    let init = send(
        &mut stdin,
        &mut reader,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "TermuxOrch");

    let list = send(
        &mut stdin,
        &mut reader,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    );
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 6);

    // Create Tier 1 direct Work (provider down -> graceful warning, still succeeds!)
    let create_req = json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{
            "name":"orch_work",
            "arguments":{
                "action":"create",
                "call_id":"stdio-create-1",
                "title":"Offline Direct Work",
                "goal":"Test durable ledger without provider",
                "tier":"direct",
                "project_id":"g-p-offline",
                "max_wakeups":1
            }
        }
    });
    let r3 = send(&mut stdin, &mut reader, create_req.clone());
    assert_eq!(r3["result"]["isError"], false);
    let payload3: Value =
        serde_json::from_str(r3["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let wid = payload3["work_id"].as_str().unwrap().to_owned();

    // Identical call_id replay -> replayed_from_receipt=true with exact same work_id!
    let r4 = send(
        &mut stdin,
        &mut reader,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":create_req["params"]}),
    );
    let payload4: Value =
        serde_json::from_str(r4["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload4["replayed_from_receipt"], true);
    assert_eq!(payload4["work_id"], wid);

    // Same call_id with different payload -> orch_call_id_payload_conflict
    let r5 = send(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{
                "name":"orch_work",
                "arguments":{
                    "action":"create",
                    "call_id":"stdio-create-1",
                    "title":"Different Title",
                    "goal":"Conflict test",
                    "tier":"direct"
                }
            }
        }),
    );
    let payload5: Value =
        serde_json::from_str(r5["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload5["error"], "orch_call_id_payload_conflict");

    // Extension catalog + unknown extension self-correction
    let r6 = send(
        &mut stdin,
        &mut reader,
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"orch_call","arguments":{"action":"does_not_exist"}}}),
    );
    let payload6: Value =
        serde_json::from_str(r6["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(payload6["available_actions"].as_array().unwrap().len() >= 5);

    // Register Verifier V1 + arm deadline Wait-Set + verify restart-durable tick
    let r7 = send(
        &mut stdin,
        &mut reader,
        json!({
            "jsonrpc":"2.0","id":7,"method":"tools/call",
            "params":{
                "name":"orch_dispatch",
                "arguments":{
                    "action":"register_verifier",
                    "call_id":"stdio-v1",
                    "work_id":wid,
                    "backend":"codex",
                    "ref_id":"tid-v1",
                    "slug":"audit",
                    "task_summary":"Verify diff"
                }
            }
        }),
    );
    let payload7: Value =
        serde_json::from_str(r7["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload7["participant"]["participant_id"], "V1");
    assert_eq!(payload7["session_title"], "[V1-audit] Verify diff");

    // Read works list
    let r8 = send(
        &mut stdin,
        &mut reader,
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"orch_works","arguments":{"action":"list"}}}),
    );
    let payload8: Value =
        serde_json::from_str(r8["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload8["count"], 1);

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());

    // 3. Separate `termux-orch status` process reads the persisted durable ledger after MCP exited!
    let st = Command::new(bin_path())
        .arg("status")
        .env("ORCH_STATE_DIR", &state_dir)
        .output()
        .unwrap();
    assert!(st.status.success());
    let st_json: Value = serde_json::from_slice(&st.stdout).unwrap();
    assert_eq!(st_json["total_works"], 1);
    assert_eq!(st_json["active_works"], 1);

    let _ = fs::remove_dir_all(&root);
}

struct Mcp {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Mcp {
    fn start(command: &mut Command) -> Self {
        let mut child = command
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let reader = BufReader::new(child.stdout.take().unwrap());
        Self { child, reader }
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{req}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        let rpc: Value = serde_json::from_str(&line).unwrap();
        serde_json::from_str(rpc["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

fn wait_for<T>(mut f: impl FnMut() -> Option<T>) -> T {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        if let Some(value) = f() {
            return value;
        }
        assert!(
            std::time::Instant::now() < until,
            "timed out awaiting receipt"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

fn json_file(path: &std::path::Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn http(addr: &str, method: &str, path: &str, token: Option<&str>) -> (String, String) {
    use std::io::Read;
    use std::net::TcpStream;
    use std::time::Duration;

    let mut conn = TcpStream::connect(addr).unwrap();
    conn.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    write!(conn, "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n").unwrap();
    if let Some(token) = token {
        write!(conn, "Authorization: Bearer {token}\r\n").unwrap();
    }
    write!(conn, "Content-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    conn.read_to_string(&mut response).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    (headers.lines().next().unwrap().to_owned(), body.to_owned())
}

struct Relay(std::process::Child);

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn codex_dispatch_without_a_thread_releases_the_worker_on_tick() {
    let root = tmp_dir("orch-unstarted");
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", root.join("state"))
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("ORCH_CODEX_SOCKET", root.join("missing.sock"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1");
        cmd
    };
    let mut mcp = Mcp::start(&mut command());
    let created = mcp.call(
        "orch_work",
        json!({"action":"create", "call_id":"create", "title":"Unstarted",
        "goal":"Verify a rejected launch releases the workspace", "tier":"orchestrated",
        "lead_kind":"codex", "lead_cwd":root}),
    );
    assert_eq!(created["state"], "pending", "{created}");
    let id = created["work_id"].as_str().unwrap();
    let dispatch = |mcp: &mut Mcp, call_id: &str| {
        mcp.call(
            "orch_dispatch",
            json!({"action":"register_worker", "call_id":call_id, "work_id":id,
            "backend":"codex", "prompt":"Run the check", "cwd":root}),
        )
    };
    let first = dispatch(&mut mcp, "first");
    assert_eq!(first["state"], "pending", "{first}");
    assert_eq!(first["participant"]["state"], "dispatching");
    assert_eq!(first["participant"]["ref_id"], "");

    assert!(command().arg("tick").output().unwrap().status.success());
    let read = mcp.call("orch_works", json!({"action":"read", "work_id":id}));
    assert_eq!(read["work"]["participants"][0]["state"], "failed", "{read}");
    assert!(read["work"]["pending_lead"].is_null());
    let inbox = mcp.call("orch_works", json!({"action":"inbox", "work_id":id}));
    for kind in ["lead.dispatch_failed", "worker.failed"] {
        assert!(inbox["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == kind));
    }
    let second = dispatch(&mut mcp, "second");
    assert_eq!(second["state"], "pending", "{second}");
    assert_eq!(second["participant"]["participant_id"], "W2");
    drop(mcp);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn completion_requires_verification_of_the_current_work() {
    let root = tmp_dir("termux-orch-review");
    let state = root.join("state");
    let command = || {
        let mut command = Command::new(bin_path());
        command
            .env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1");
        command
    };
    let mut mcp = Mcp::start(&mut command());
    let work = mcp.call(
        "orch_work",
        json!({"action":"create", "call_id":"create-review-work",
        "title":"Review", "goal":"Initial goal", "tier":"direct"}),
    );
    let id = work["work_id"].as_str().unwrap();
    let complete = |mcp: &mut Mcp| {
        mcp.call(
            "orch_report",
            json!({"action":"complete", "call_id":"complete-review-work",
            "work_id":id, "summary":"Verified", "flush_now":false}),
        )
    };
    for (call, verifier, script) in [
        ("verify-first", "V1", "test 2 -eq 2"),
        ("verify-again", "V2", "test 3 -eq 3"),
    ] {
        let registered = mcp.call(
            "orch_dispatch",
            json!({"action":"register_verifier", "call_id":call,
            "work_id":id, "backend":"shell", "slug":"tests", "command":script}),
        );
        assert_eq!(registered["state"], "ok", "{registered}");
        assert_eq!(complete(&mut mcp)["error"], "participants_active");
        let exit = state
            .join("jobs")
            .join(id)
            .join("shell")
            .join(format!("{verifier}.exit.json"));
        assert_eq!(wait_for(|| json_file(&exit))["exit_code"], 0);
        assert!(command().arg("tick").output().unwrap().status.success());
        if verifier == "V1" {
            let changed = mcp.call(
                "orch_work",
                json!({"action":"update_scope", "call_id":"new-scope",
                "work_id":id, "goal":"Changed goal"}),
            );
            assert_eq!(changed["state"], "ok", "{changed}");
            assert_eq!(complete(&mut mcp)["error"], "verification_required");
        }
    }
    assert_eq!(complete(&mut mcp)["work_state"], "completed");
    drop(mcp);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn fixed_shell_continuation_survives_relay_reentry_without_double_execution() {
    let root = tmp_dir("termux-orch-build-check-report");
    let state = root.join("state");
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1")
            .env("ORCH_NOTIFY_CMD", "/bin/true");
        cmd
    };
    for (case, script, expected) in [
        ("pass", "printf 'build-ok' > artifact", "completed"),
        ("fail", "exit 7", "failed"),
    ] {
        let cwd = root.join(case);
        fs::create_dir(&cwd).unwrap();
        let mut mcp = Mcp::start(&mut command());
        let work = mcp.call("orch_work", json!({"action":"create","call_id":format!("work-{case}"),
            "tier":"direct","title":"Build and check","goal":"Verify the build","delivery_mode":"notify_only"}));
        let id = work["work_id"].as_str().unwrap();
        let build = mcp.call(
            "orch_dispatch",
            json!({"action":"register_worker","call_id":format!("build-{case}"),
            "work_id":id,"backend":"shell","cwd":cwd,
            "command":format!("while [ ! -f release ]; do sleep 0.05; done; {script}")}),
        );
        assert_eq!(build["state"], "ok", "{build}");
        let wait = mcp.call(
            "orch_dispatch",
            json!({"action":"wait_set","call_id":format!("wait-{case}"),
            "work_id":id,"all_of":["W1"],"then_verify_shell":{"cwd":cwd,
            "command":"test \"$(cat artifact)\" = build-ok && printf 'check\\n' >> runs"}}),
        );
        assert_eq!(wait["state"], "ok", "{wait}");
        drop(mcp);
        assert!(command().arg("tick").output().unwrap().status.success());
        assert!(!cwd.join("runs").exists());
        fs::write(cwd.join("release"), "").unwrap();
        let path = state.join("jobs").join(id);
        let final_work = wait_for(|| {
            assert!(command().arg("tick").output().unwrap().status.success());
            json_file(&path.join("work.json")).filter(|w| w["state"] == expected)
        });
        assert!(final_work["lead"].is_null());
        assert_eq!(final_work["budget"]["wakeups_used"], 0);
        assert!(command().arg("tick").output().unwrap().status.success());
        let events = fs::read_to_string(path.join("events.jsonl")).unwrap();
        assert_eq!(
            events.matches("\"kind\":\"continuation.finished\"").count(),
            1
        );
        if case == "pass" {
            assert_eq!(fs::read_to_string(cwd.join("runs")).unwrap(), "check\n");
            assert_eq!(
                final_work["participants"][1]["verification"]["verdict"],
                "pass"
            );
        } else {
            assert!(!cwd.join("runs").exists());
            assert_eq!(final_work["participants"].as_array().unwrap().len(), 1);
        }
    }
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn detached_writer_can_start_when_the_running_mcp_path_was_removed() {
    let root = tmp_dir("termux-orch-executable-replacement");
    let state = root.join("state");
    let launcher = root.join("orch-fixture");
    fs::copy(bin_path(), &launcher).unwrap();
    let mut cmd = Command::new(&launcher);
    cmd.env("ORCH_STATE_DIR", &state)
        .env("ORCH_RECEIPT_DIR", root.join("receipts"))
        .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1");
    let mut mcp = Mcp::start(&mut cmd);
    let work = mcp.call(
        "orch_work",
        json!({"action":"create","call_id":"detached-fixture",
        "title":"Detached writer","goal":"Verify running image"}),
    );
    let id = work["work_id"].as_str().unwrap();
    fs::remove_file(&launcher).unwrap();
    let result = mcp.call(
        "orch_dispatch",
        json!({"action":"register_worker","call_id":"exec-running-image",
        "work_id":id,"backend":"shell","command":"exit 0"}),
    );
    assert_eq!(result["state"], "ok", "{result}");
    let receipt = state.join("jobs").join(id).join("shell/W1.exit.json");
    assert_eq!(wait_for(|| json_file(&receipt))["exit_code"], 0);
    drop(mcp);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detached_shell_restart_barrier_kill_and_authenticated_immediate_wake() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    let root = tmp_dir("termux-orch-resilience");
    let state = root.join("state");
    let url_path = root.join("relay/health.url");
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1")
            .env("ORCH_NOTIFY_CMD", "/bin/true");
        cmd
    };
    let start_relay = || {
        Relay(
            command()
                .args(["relay", "--poll-ms", "60000", "--health-url-file"])
                .arg(&url_path)
                .spawn()
                .unwrap(),
        )
    };
    let ledger = |id: &str| state.join("jobs").join(id);
    let mut mcp = Mcp::start(&mut command());
    let work = mcp.call(
        "orch_work",
        json!({
            "action":"create","call_id":"new-shell","title":"Shell","goal":"test","tier":"direct",
            "project_id":"offline","delivery_mode":"notify_only",
        }),
    );
    let id = work["work_id"].as_str().unwrap();
    let r = mcp.call("orch_dispatch", json!({
        "action":"register_worker","call_id":"launch","work_id":id,"backend":"shell",
        "command":"while [ ! -e release ]; do sleep 0.05; done; printf 'TAIL_OK'; printf ' STDERR_OK' >&2",
        "cwd":root,
    }));
    assert_eq!(r["state"], "ok", "{r}");
    let marker_dir = ledger(id).join("shell");
    let overlapping = mcp.call(
        "orch_dispatch",
        json!({
            "action":"register_worker","call_id":"overlap","work_id":id,"backend":"shell",
            "command":"printf 'must not overlap' > overlap", "cwd":root.join("."),
        }),
    );
    assert_eq!(overlapping["error"], "worker_workspace_busy");
    assert!(!root.join("overlap").exists());
    let meta_path = marker_dir.join("W1.meta.json");
    let exit_path = marker_dir.join("W1.exit.json");
    let meta = wait_for(|| json_file(&meta_path));
    let pid = meta["pid"].as_i64().unwrap() as i32;
    let proc_stat = || fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let start_ticks = |stat: String| {
        stat.rsplit_once(')')
            .unwrap()
            .1
            .split_whitespace()
            .nth(19)
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };
    assert_eq!(meta["start_ticks"], start_ticks(proc_stat()));
    assert_eq!(
        fs::metadata(&marker_dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&meta_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let barrier = mcp.call("orch_dispatch", json!({
        "action":"wait_set","call_id":"barrier","work_id":id,"all_of":["W1"],"on_user_event":false,
    }));
    assert_eq!(barrier["state"], "ok", "{barrier}");
    drop(mcp);

    let relay = start_relay();
    wait_for(|| fs::read_to_string(&url_path).ok());
    drop(relay);
    fs::remove_file(&url_path).unwrap();
    let relay = start_relay();
    let url = wait_for(|| fs::read_to_string(&url_path).ok());
    let addr = url.trim().strip_prefix("http://").unwrap();
    let token_path = url_path.with_file_name("wake.token");
    let token = fs::read_to_string(&token_path).unwrap();
    assert_eq!(
        fs::metadata(token_path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(meta["start_ticks"], start_ticks(proc_stat()));
    fs::write(root.join("release"), "").unwrap();
    let exit = wait_for(|| json_file(&exit_path));
    assert_eq!(exit["pid"], pid);
    assert_eq!(exit["start_ticks"], meta["start_ticks"]);
    assert_eq!(exit["exit_code"], 0);

    let health = || serde_json::from_str::<Value>(&http(addr, "GET", "/healthz", None).1).unwrap();
    let before = health()["ticks"].as_u64().unwrap();
    assert!(http(addr, "POST", "/wake", None).0.contains("401"));
    assert_eq!(health()["wake_pushes"], 0);
    let start = Instant::now();
    assert!(http(addr, "POST", "/wake", Some(&token)).0.contains("202"));
    let h = wait_for(|| {
        let h = health();
        (h["ticks"].as_u64().unwrap() > before).then_some(h)
    });
    assert!(start.elapsed() < Duration::from_millis(1000));
    assert_eq!(h["wake_pushes"], 1);
    let mut mcp = Mcp::start(&mut command());
    let w = mcp.call("orch_works", json!({"action":"read","work_id":id}));
    assert_eq!(w["work"]["participants"][0]["state"], "completed");
    assert!(w["work"]["participants"][0]["result_summary"]
        .as_str()
        .unwrap()
        .contains("TAIL_OK STDERR_OK"));
    assert_eq!(w["work"]["wait_set"]["armed"], false);
    assert_eq!(w["work"]["budget"]["wakeups_used"], 1);

    for (index, shell, expected) in [
        (2, "sleep 30", "without an exit receipt"),
        (3, "printf BAD >&2; exit 9", "exit_code=9"),
    ] {
        let alias = format!("W{index}");
        let r = mcp.call(
            "orch_dispatch",
            json!({
                "action":"register_worker","call_id":format!("launch-{index}"),"work_id":id,
                "backend":"shell","prompt":shell,"cwd":root,
            }),
        );
        assert_eq!(r["participant"]["participant_id"], alias);
        let meta = wait_for(|| json_file(&marker_dir.join(format!("{alias}.meta.json"))));
        let pid = meta["pid"].as_i64().unwrap() as i32;
        if index == 2 {
            assert_eq!(unsafe { libc::kill(-pid, libc::SIGKILL) }, 0);
        } else {
            wait_for(|| json_file(&marker_dir.join(format!("{alias}.exit.json"))));
        }
        assert!(http(addr, "POST", "/wake", Some(&token)).0.contains("202"));
        let finished = wait_for(|| {
            let w = json_file(&ledger(id).join("work.json"))?;
            let p = &w["participants"][index - 1];
            (p["state"] == "failed").then_some(p.clone())
        });
        assert!(
            finished["result_summary"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{finished}"
        );
    }
    let events = fs::read_to_string(ledger(id).join("events.jsonl")).unwrap();
    assert_eq!(events.matches("\"kind\":\"worker.completed\"").count(), 1);
    assert_eq!(events.matches("\"kind\":\"worker.failed\"").count(), 2);
    let live = mcp.call(
        "orch_dispatch",
        json!({"action":"register_worker","call_id":"cancellable-worker",
        "work_id":id,"backend":"shell","command":"echo $$ > child.pid; exec sleep 30","cwd":root}),
    );
    assert_eq!(live["state"], "ok", "{live}");
    let child_pid: i32 = wait_for(|| {
        fs::read_to_string(root.join("child.pid"))
            .ok()
            .and_then(|text| text.trim().parse().ok())
    });
    let requested = mcp.call(
        "orch_work",
        json!({"action":"cancel","call_id":"cancel-live","work_id":id}),
    );
    assert_eq!(requested["work_state"], "cancelling");
    let exit = wait_for(|| json_file(&marker_dir.join("W4.exit.json")));
    assert_eq!(exit["exit_code"], 128 + libc::SIGTERM);
    assert!(!std::path::Path::new(&format!("/proc/{child_pid}")).exists());
    assert!(http(addr, "POST", "/wake", Some(&token)).0.contains("202"));
    wait_for(|| json_file(&ledger(id).join("work.json")).filter(|w| w["state"] == "cancelled"));
    drop(mcp);
    drop(relay);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compiled_rollover_preserves_pinned_project_and_replays_every_event_once() {
    exercise_compiled_rollover(false, false, None);
}

#[test]
fn compiled_context_limit_rolls_over_with_turn_threshold_disabled() {
    exercise_compiled_rollover(true, false, None);
}

#[test]
fn compiled_unscoped_rollover_recovers_the_original_chat_from_a_later_page() {
    exercise_compiled_rollover(true, true, None);
}

#[test]
fn input_size_recovery_reuses_bounded_checkpoint_in_one_fresh_chat() {
    exercise_compiled_rollover(true, true, Some(false));
}

#[test]
fn input_size_recovery_stops_after_fresh_chat_rejects() {
    exercise_compiled_rollover(true, true, Some(true));
}

fn exercise_compiled_rollover(
    context_limit: bool,
    unscoped: bool,
    reject_fresh_size: Option<bool>,
) {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Provider {
        wrong_account: bool,
        reject_create_receipt: bool,
        host_stopped: bool,
        create_calls: usize,
        turns: u64,
        local_turn_error: Option<&'static str>,
        lanes: HashMap<String, String>,
        titles: HashMap<String, String>,
        messages: Vec<String>,
        listing_pages: usize,
    }
    let root = tmp_dir("termux-orch-rollover");
    let state = root.join("state");
    let token_path = root.join("control.token");
    fs::write(&token_path, "a".repeat(64)).unwrap();
    fs::set_permissions(&token_path, fs::Permissions::from_mode(0o600)).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let provider = Arc::new(Mutex::new(Provider::default()));
    let server = provider.clone();
    let project = (!unscoped).then_some("pinned-project");
    let mock = std::thread::spawn(move || {
        for conn in listener.incoming() {
            let mut conn = conn.unwrap();
            let mut reader = BufReader::new(&conn);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let second_page = line.contains("cursor=second-page");
            let path = line
                .split_ascii_whitespace()
                .nth(1)
                .unwrap()
                .split('?')
                .next()
                .unwrap()
                .to_owned();
            if path == "/shutdown" {
                break;
            }
            let mut size = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(s) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    size = s.trim().parse().unwrap();
                }
            }
            let mut bytes = vec![0; size];
            reader.read_exact(&mut bytes).unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            let mut p = server.lock().unwrap();
            let result = match path.as_str() {
                "/control/status" => {
                    json!({"auth_state":"authenticated","pinned_send":true,"account":{"account_id":if p.wrong_account {"other-account"} else {"pinned-account"}}})
                }
                "/control/observe" => {
                    let lane = body["lane_id"].as_str().unwrap_or_default();
                    json!({"state":"settled","generating":false,"lane_id":lane,"chat_id":p.lanes.get(lane)})
                }
                "/control/send" => {
                    assert_eq!(body["expected_account_id"], "pinned-account");
                    assert_eq!(body["project_id"].as_str(), project);
                    let prompt = body["prompt"].as_str().unwrap().to_owned();
                    let lane = body["lane_id"].as_str().unwrap().to_owned();
                    if body["chat_id"].is_null() {
                        p.create_calls += 1;
                        assert!(prompt.contains("CHECKPOINT_LEAD"));
                        if let Some(reject) = reject_fresh_size {
                            assert!(prompt.len() <= 32 * 1024);
                            if reject {
                                let result = json!({"state":"error","error":"input_too_large","lane_id":lane});
                                let raw = result.to_string();
                                write!(conn, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{raw}", raw.len()).unwrap();
                                continue;
                            }
                        }
                        if p.create_calls == 2 {
                            p.messages.clear();
                        }
                    }
                    let next = format!("lead-{}", p.create_calls + 1);
                    p.lanes.insert(lane, next.clone());
                    p.messages.push(prompt);
                    if body["chat_id"].is_null() && p.reject_create_receipt {
                        p.reject_create_receipt = false;
                        if unscoped {
                            p.lanes.remove(body["lane_id"].as_str().unwrap());
                        }
                        continue;
                    }
                    json!({"state":"settled","chat_id":next})
                }
                "/control/chats" if unscoped => {
                    p.listing_pages += 1;
                    if second_page {
                        json!({"state":"ok","chats":[{"id":"lead-2"}],"next_cursor":null})
                    } else {
                        json!({"state":"ok","chats":[{"id":"lead-1"}],"next_cursor":"second-page"})
                    }
                }
                "/control/chats/lead-1" => {
                    json!({
                        "state":"ok","gizmo_id":project,"user_turn_count":p.turns,
                        "local_turn_error":p.local_turn_error,
                    })
                }
                "/control/chats/lead-2/rename"
                | "/control/chats/lead-3/rename"
                | "/control/chats/lead-1/rename" => {
                    p.host_stopped = false;
                    p.titles
                        .insert(path, body["title"].as_str().unwrap().to_owned());
                    json!({"state":"ok"})
                }
                "/control/chats/lead-2" | "/control/chats/lead-3" => {
                    let turns = p
                        .messages
                        .iter()
                        .map(|text| json!({"role":"user","text":text}))
                        .collect::<Vec<_>>();
                    json!({"state":"ok","gizmo_id":project,"user_turn_count":turns.len(),"messages":turns})
                }
                other => panic!("unexpected provider route {other}"),
            };
            let response = result.to_string();
            write!(
                conn,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
                response.len()
            )
            .unwrap();
        }
    });
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", format!("http://{addr}"))
            .env("CHATGPT_PROVIDER_TOKEN_FILE", &token_path)
            .env("ORCH_NOTIFY_CMD", "/bin/true");
        cmd
    };
    let tick = || {
        assert!(command()
            .arg("tick")
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success())
    };
    let mut mcp = Mcp::start(&mut command());
    provider.lock().unwrap().host_stopped = true;
    let created = mcp.call(
        "orch_work",
        json!({
            "action":"create","call_id":"lead1","title":"Pinned Lead","goal":"test",
            "project_id":project,"lead_ref_id":"lead-1",
            "lead_rollover_turns":if context_limit {0} else {2},
            "delivery_mode":"notify_only",
        }),
    );
    let wid = created["work_id"].as_str().unwrap();
    assert_eq!(created["lead"]["generation"], 1);
    if context_limit {
        provider.lock().unwrap().turns = 2;
        tick();
        assert_eq!(provider.lock().unwrap().create_calls, 0);
    }
    for seq in 0..18 {
        let reported = mcp.call("orch_report", json!({
            "action":"milestone","call_id":format!("event-{seq}"),"work_id":wid,
            "summary":format!("REPLAY_EVENT_{seq:02}"),"checkpoint_md":"CHECKPOINT_LEAD","flush_now":false
        }));
        assert_eq!(reported["state"], "ok", "{reported}");
    }
    provider.lock().unwrap().wrong_account = true;
    let rejected = mcp.call(
        "orch_call",
        json!({"action":"lead_rollover","arguments":{"work_id":wid}}),
    );
    assert_eq!(rejected["error"], "pinned_account_mismatch", "{rejected}");
    assert_eq!(provider.lock().unwrap().create_calls, 0);

    {
        let mut p = provider.lock().unwrap();
        p.wrong_account = false;
        p.reject_create_receipt = true;
        p.turns = 2;
        if context_limit {
            p.local_turn_error = Some(if reject_fresh_size.is_some() {
                "input_too_large"
            } else {
                "context_length_exceeded"
            });
        }
    }
    if reject_fresh_size == Some(true) {
        for _ in 0..4 {
            tick();
        }
        let w = mcp.call("orch_works", json!({"action":"read","work_id":wid}));
        assert_eq!(w["work"]["state"], "waiting_user", "{w}");
        assert_eq!(w["work"]["lead"]["generation"], 1);
        assert_eq!(provider.lock().unwrap().create_calls, 1);
        let inbox = mcp.call(
            "orch_works",
            json!({"action":"inbox","work_id":wid,"limit":100}),
        );
        assert!(inbox["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["data"]["reason"] == "payload_too_large"));
        use std::net::TcpStream;
        let mut stop = TcpStream::connect(addr).unwrap();
        write!(stop, "GET /shutdown HTTP/1.1\r\n\r\n").unwrap();
        mock.join().unwrap();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    // A lost HTTP receipt must recover the original lane from a separate Relay process.
    tick();
    provider.lock().unwrap().host_stopped = true;
    tick();
    if unscoped {
        tick();
        assert_eq!(provider.lock().unwrap().listing_pages, 2);
    }
    let w = mcp.call("orch_works", json!({"action":"read","work_id":wid}));
    assert_eq!(w["work"]["lead"]["generation"], 2, "{w}");
    assert_eq!(w["work"]["lead"]["history"][0]["ref_id"], "lead-1");
    assert_eq!(w["work"]["lead"]["ref_id"], "lead-2");
    let rejected = mcp.call(
        "orch_dispatch",
        json!({
            "action":"wait_clear","work_id":wid,"call_id":"stale1","expected_lead_generation":1,
        }),
    );
    assert_eq!(rejected["error"], "stale_lead_generation");
    let rejected = mcp.call("orch_report", json!({
        "action":"complete","work_id":wid,"call_id":"stale2","summary":"stale","lead_ref_id":"lead-1",
    }));
    assert_eq!(rejected["error"], "stale_lead_generation");

    for _ in 0..10 {
        tick();
    }
    let w = mcp.call("orch_works", json!({"action":"read","work_id":wid}));
    assert_eq!(w["work"]["lead"]["generation"], 2);
    assert!(w["work"]["lead_replay"].is_null());
    let events = mcp.call(
        "orch_works",
        json!({"action":"inbox","work_id":wid,"after_seq":0,"limit":50}),
    );
    let rollover = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["kind"] == "lead.rolled_over")
        .unwrap();
    assert_eq!(
        rollover["data"]["reason"],
        if context_limit {
            if reject_fresh_size.is_some() {
                "context limit (input_too_large)"
            } else {
                "context limit (context_length_exceeded)"
            }
        } else {
            "turn threshold (2/2)"
        }
    );
    {
        let p = provider.lock().unwrap();
        assert_eq!(p.create_calls, 1);
        assert_eq!(p.titles["/control/chats/lead-2/rename"], "[L2] Pinned Lead");
        assert_eq!(
            p.messages
                .iter()
                .filter(|text| text.starts_with("[ORCH_INBOX_REPLAY"))
                .count(),
            3
        );
        let messages = p.messages.join("\n");
        for seq in 0..18 {
            assert_eq!(
                messages.matches(&format!("REPLAY_EVENT_{seq:02}")).count(),
                1
            );
        }
        assert_eq!(messages.matches("lead.rolled_over").count(), 1);
    }
    let manual = mcp.call(
        "orch_call",
        json!({
            "action":"lead_rollover","arguments":{"work_id":wid,"reason":"manual_test",
            "checkpoint_md":"CHECKPOINT_LEAD updated","expected_lead_generation":2}
        }),
    );
    assert_eq!(manual["state"], "ok", "{manual}");
    assert_eq!(manual["lead"]["generation"], 3);
    for _ in 0..8 {
        tick();
    }
    assert_eq!(provider.lock().unwrap().create_calls, 2);
    drop(mcp);
    let mut stop = std::net::TcpStream::connect(addr).unwrap();
    stop.write_all(b"GET /shutdown HTTP/1.1\r\n\r\n").unwrap();
    mock.join().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn durable_step_plan_orders_shared_writers_and_requires_exit_receipt() {
    let root = tmp_dir("termux-orch-steps");
    let state = root.join("state");
    let workspace = root.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("ORCH_NOTIFY_CMD", "/bin/true")
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1");
        cmd
    };
    let tick = || assert!(command().arg("tick").output().unwrap().status.success());
    let mut mcp = Mcp::start(&mut command());
    let created = mcp.call("orch_work", json!({
        "action":"create","call_id":"create-step-plan","title":"Fixed plan","goal":"Build and test",
        "delivery_mode":"notify_only",
        "steps":[
            {"step_id":"build","cwd":workspace,"command":"printf first > order; while [ ! -f release ]; do sleep .05; done","write_paths":["order"]},
            {"step_id":"append","cwd":workspace,"command":"printf second >> order","write_paths":["order"]},
            {"step_id":"verify","cwd":workspace,"command":"test \"$(cat order)\" = firstsecond","depends_on":["build","append"],"auto_verify":true,"isolation":"readonly"}
        ]
    }));
    assert_eq!(created["state"], "ok", "{created}");
    let wid = created["work_id"].as_str().unwrap();
    let get = |mcp: &mut Mcp| {
        mcp.call("orch_works", json!({"action":"read","work_id":wid}))["work"].clone()
    };
    tick();
    wait_for(|| fs::read_to_string(workspace.join("order")).ok());
    tick();
    assert_eq!(get(&mut mcp)["participants"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(workspace.join("order")).unwrap(),
        "first"
    );
    // Re-enter through a different compiled process while W1's writer still owns the job.
    fs::write(workspace.join("release"), "").unwrap();
    let result = wait_for(|| {
        tick();
        let work = get(&mut mcp);
        (work["state"] == "completed").then_some(work)
    });
    assert!(result["lead"].is_null());
    assert_eq!(result["budget"]["wakeups_used"], 0);
    assert_eq!(result["participants"][2]["verification"]["verdict"], "pass");
    assert_eq!(
        fs::read_to_string(workspace.join("order")).unwrap(),
        "firstsecond"
    );
    assert_eq!(result["participants"].as_array().unwrap().len(), 3);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn worktree_plan_detects_undeclared_edit_without_touching_source() {
    let root = tmp_dir("termux-orch-worktrees");
    let state = root.join("state");
    let repo = root.join("repo");
    fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .current_dir(&repo)
            .args(args)
            .output()
            .unwrap()
            .status
            .success())
    };
    git(&["init", "-q"]);
    fs::write(repo.join("owned"), "base").unwrap();
    fs::write(repo.join("outside"), "base").unwrap();
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "fixture",
    ]);
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1")
            .env("ORCH_NOTIFY_CMD", "/bin/true");
        cmd
    };
    let mut mcp = Mcp::start(&mut command());
    let created = mcp.call("orch_work", json!({
        "action":"create","call_id":"create-worktree-plan","title":"Worktree plan","goal":"Test scope",
        "delivery_mode":"notify_only","steps":[
            {"step_id":"isolated","cwd":repo,"isolation":"worktree","write_paths":["owned"],
             "on_failure":"wake_lead","command":"printf changed > outside"}
        ]
    }));
    assert_eq!(created["state"], "ok", "{created}");
    let wid = created["work_id"].as_str().unwrap();
    let result = wait_for(|| {
        assert!(command().arg("tick").output().unwrap().status.success());
        let work = mcp.call("orch_works", json!({"action":"read","work_id":wid}))["work"].clone();
        (work["state"] == "waiting_user").then_some(work)
    });
    let p = &result["participants"][0];
    assert_eq!(p["state"], "failed");
    assert!(p["result_summary"]
        .as_str()
        .unwrap()
        .contains("Undeclared changes"));
    assert_eq!(fs::read_to_string(repo.join("outside")).unwrap(), "base");
    let tree = PathBuf::from(p["worktree"]["path"].as_str().unwrap());
    assert_eq!(fs::read_to_string(tree.join("outside")).unwrap(), "changed");

    let created = mcp.call(
        "orch_work",
        json!({"action":"create", "call_id":"create-interactive-worktree", "title":"Interactive",
            "goal":"Do not offer completion while an undeclared change remains"}),
    );
    let wid = created["work_id"].as_str().unwrap();
    let registered = mcp.call(
        "orch_dispatch",
        json!({"action":"register_worker", "call_id":"interactive-worktree", "work_id":wid,
            "backend":"shell", "cwd":repo, "isolation":"worktree", "write_paths":["owned"],
            "command":"printf changed > outside"}),
    );
    assert_eq!(registered["state"], "ok", "{registered}");
    let failed = wait_for(|| {
        assert!(command().arg("tick").output().unwrap().status.success());
        let read = mcp.call("orch_works", json!({"action":"read","work_id":wid}));
        let worker = &read["work"]["participants"][0];
        (worker["state"] == "failed").then(|| worker.clone())
    });
    let complete = |mcp: &mut Mcp| {
        mcp.call(
            "orch_report",
            json!({"action":"complete", "work_id":wid,"call_id":"finish-interactive",
                "summary":"Completion check", "review_cwd":repo, "flush_now":false}),
        )
    };
    assert_eq!(
        complete(&mut mcp)["error"],
        "worker_workspace_scope_unconfirmed"
    );
    let tree = PathBuf::from(failed["worktree"]["path"].as_str().unwrap());
    fs::write(tree.join("outside"), "base").unwrap();
    assert_eq!(complete(&mut mcp)["error"], "verification_required");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancelling_shell_plan_escalates_when_worker_ignores_term() {
    let root = tmp_dir("termux-orch-kill");
    let state = root.join("state");
    let command = || {
        let mut cmd = Command::new(bin_path());
        cmd.env("ORCH_STATE_DIR", &state)
            .env("ORCH_RECEIPT_DIR", root.join("receipts"))
            .env("CHATGPT_PROVIDER_URL", "http://127.0.0.1:1")
            .env("ORCH_NOTIFY_CMD", "/bin/true");
        cmd
    };
    let mut mcp = Mcp::start(&mut command());
    let created = mcp.call("orch_work", json!({
        "action":"create","call_id":"create-stubborn","title":"Shell job","goal":"Cancel stubborn shell",
        "steps":[{"step_id":"stubborn","cwd":root,"command":"trap '' TERM; echo ready > marker; while :; do sleep .2; done"}]
    }));
    let wid = created["work_id"].as_str().unwrap();
    assert!(command().arg("tick").output().unwrap().status.success());
    wait_for(|| fs::read_to_string(root.join("marker")).ok());
    let cancelled = mcp.call(
        "orch_work",
        json!({"action":"cancel","call_id":"cancel-stubborn","work_id":wid}),
    );
    assert_eq!(cancelled["state"], "ok", "{cancelled}");
    wait_for(|| {
        assert!(command().arg("tick").output().unwrap().status.success());
        let work = mcp.call("orch_works", json!({"action":"read","work_id":wid}))["work"].clone();
        (work["state"] == "cancelled").then_some(work)
    });
    assert_eq!(
        json_file(&state.join("jobs").join(wid).join("shell/W1.exit.json")).unwrap()["exit_code"],
        137
    );
    fs::remove_dir_all(root).unwrap();
}
