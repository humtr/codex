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
