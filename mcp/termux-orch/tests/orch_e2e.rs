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
    drop(mcp);
    drop(relay);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compiled_rollover_preserves_pinned_project_and_replays_every_event_once() {
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
        lanes: HashMap<String, String>,
        titles: HashMap<String, String>,
        messages: Vec<String>,
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
    let mock = std::thread::spawn(move || {
        for conn in listener.incoming() {
            let mut conn = conn.unwrap();
            let mut reader = BufReader::new(&conn);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
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
                    assert_eq!(body["project_id"], "pinned-project");
                    let prompt = body["prompt"].as_str().unwrap().to_owned();
                    let lane = body["lane_id"].as_str().unwrap().to_owned();
                    if body["chat_id"].is_null() {
                        p.create_calls += 1;
                        assert!(prompt.contains("CHECKPOINT_7B"));
                        if p.create_calls == 2 {
                            p.messages.clear();
                        }
                    }
                    let next = format!("lead-{}", p.create_calls + 1);
                    p.lanes.insert(lane, next.clone());
                    p.messages.push(prompt);
                    if body["chat_id"].is_null() && p.reject_create_receipt {
                        p.reject_create_receipt = false;
                        continue;
                    }
                    json!({"state":"settled","chat_id":next})
                }
                "/control/chats/lead-1" => {
                    json!({"state":"ok","gizmo_id":"pinned-project","user_turn_count":p.turns})
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
                    json!({"state":"ok","gizmo_id":"pinned-project","user_turn_count":turns.len(),"messages":turns})
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
            "project_id":"pinned-project","lead_ref_id":"lead-1","lead_rollover_turns":2,
            "delivery_mode":"notify_only",
        }),
    );
    let wid = created["work_id"].as_str().unwrap();
    assert_eq!(created["lead"]["generation"], 1);
    for seq in 0..18 {
        let reported = mcp.call("orch_report", json!({
            "action":"milestone","call_id":format!("event-{seq}"),"work_id":wid,
            "summary":format!("REPLAY_EVENT_{seq:02}"),"checkpoint_md":"CHECKPOINT_7B","flush_now":false
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
    }
    // A lost HTTP receipt must recover the original lane from a separate Relay process.
    tick();
    provider.lock().unwrap().host_stopped = true;
    tick();
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
            "checkpoint_md":"CHECKPOINT_7B updated","expected_lead_generation":2}
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
