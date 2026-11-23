//! Compiled-binary stdio JSON-RPC E2E tests for `termux-chatgpt-mcp` (`@TermuxChatGPT`).

use serde_json::{json, Value};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn temp_dir(prefix: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    DirBuilder::new().mode(0o700).create(&d).unwrap();
    d
}

fn write_private_token(dir: &Path, token: &str) -> PathBuf {
    let p = dir.join("control.token");
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&p)
        .unwrap();
    writeln!(f, "{token}").unwrap();
    p
}

fn run_mcp_session(
    provider_url: &str,
    token_path: &Path,
    receipt_dir: &Path,
    requests: &[Value],
) -> Vec<Value> {
    let bin = env!("CARGO_BIN_EXE_termux-chatgpt-mcp");
    let mut child = Command::new(bin)
        .env("CHATGPT_PROVIDER_URL", provider_url)
        .env("CHATGPT_PROVIDER_TOKEN_FILE", token_path)
        .env("TERMUX_CHATGPT_RECEIPT_DIR", receipt_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let mut stdin = child.stdin.take().unwrap();
        for req in requests {
            writeln!(stdin, "{req}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "termux-chatgpt-mcp failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

fn tool_payload(resp: &Value) -> (Value, bool) {
    let is_err = resp["result"]["isError"].as_bool().unwrap();
    let txt = resp["result"]["content"][0]["text"].as_str().unwrap();
    (serde_json::from_str(txt).unwrap(), is_err)
}

fn start_mock_provider(
    expected_token: String,
    send_counter: Arc<AtomicUsize>,
    drop_send: bool,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}");

    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut req_line = String::new();
            if reader.read_line(&mut req_line).unwrap_or(0) == 0 {
                continue;
            }
            let parts: Vec<&str> = req_line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let method = parts[0].to_owned();
            let path = parts[1].to_owned();

            let mut content_len = 0usize;
            let mut auth_header = String::new();
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                    break;
                }
                if let Some((k, v)) = h.split_once(':') {
                    if k.eq_ignore_ascii_case("content-length") {
                        content_len = v.trim().parse().unwrap_or(0);
                    } else if k.eq_ignore_ascii_case("authorization") {
                        auth_header = v.trim().to_owned();
                    }
                }
            }
            let mut body_buf = vec![0u8; content_len];
            if content_len > 0 {
                let _ = reader.read_exact(&mut body_buf);
            }
            let body_json: Value = serde_json::from_slice(&body_buf).unwrap_or(Value::Null);

            if path == "/__shutdown" {
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
                break;
            }
            if path == "/healthz" {
                let resp = json!({"ok": true, "service": "chatgpt-provider"}).to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                );
                continue;
            }

            if auth_header != format!("Bearer {expected_token}") {
                let resp = json!({"state":"error","error":"control_auth_rejected"}).to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp}",
                    resp.len()
                );
                continue;
            }

            if method == "POST" && path == "/control/send" && drop_send {
                // Simulate mid-flight transport disconnect right after accepting request.
                send_counter.fetch_add(1, Ordering::SeqCst);
                drop(stream);
                continue;
            }

            let resp_val = match (method.as_str(), path.as_str()) {
                ("GET", "/control/status") => json!({
                    "state": "ok",
                    "host_running": true,
                    "auth_state": "authenticated",
                    "completion_ready": true,
                    "models": [{"slug": "gpt-6-sol-wm"}]
                }),
                ("GET", p) if p.starts_with("/control/chats?") => json!({
                    "state": "ok",
                    "chats": [{"id": "c1", "title": "Main"}],
                    "next_cursor": null
                }),
                ("GET", p) if p.starts_with("/control/chats/c1") => json!({
                    "state": "ok",
                    "chat_id": "c1",
                    "messages": [{"role": "assistant", "text": "hello"}]
                }),
                ("POST", "/control/chats/c1/rename") => json!({
                    "state": "ok",
                    "chat_id": "c1",
                    "title": body_json["title"]
                }),
                ("GET", p) if p.starts_with("/control/projects?") => json!({
                    "state": "ok",
                    "projects": [{"id": "p1", "name": "Work"}],
                    "next_cursor": null
                }),
                ("GET", p) if p.starts_with("/control/projects/p1") => json!({
                    "state": "ok",
                    "project_id": "p1",
                    "chats": [{"id": "c1", "title": "Main"}]
                }),
                ("POST", "/control/projects") => json!({
                    "state": "ok",
                    "project_id": "p-new",
                    "name": body_json["name"]
                }),
                ("POST", "/control/send") => {
                    let count = send_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    json!({
                        "state": "settled",
                        "chat_id": body_json["chat_id"].as_str().unwrap_or("c-new"),
                        "assistant_text": format!("REPLY_{count}"),
                        "generating": false
                    })
                }
                ("POST", "/control/observe") => json!({
                    "state": "settled",
                    "chat_id": body_json["chat_id"],
                    "generating": false
                }),
                ("POST", "/control/stop") => json!({
                    "state": "stopped",
                    "stopped": true
                }),
                ("GET", "/control/viewer") => json!({
                    "state": "ok",
                    "available": true,
                    "url": "http://127.0.0.1:4319/viewer/cap/vnc.html"
                }),
                ("POST", "/control/viewer/open") => json!({
                    "state": "ok",
                    "viewer_open_requested": true
                }),
                ("POST", "/control/host/restart") => json!({
                    "state": "ok",
                    "restarted": true
                }),
                _ => json!({"state": "error", "error": "not_found"}),
            };
            let body = resp_val.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (url, handle)
}

fn stop_mock_provider(url: &str, handle: thread::JoinHandle<()>) {
    let addr = url.strip_prefix("http://").unwrap();
    if let Ok(mut s) = std::net::TcpStream::connect(addr) {
        let _ = write!(
            s,
            "GET /__shutdown HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
        );
    }
    let _ = handle.join();
}

#[test]
fn cli_flags_and_provider_down_non_poisoning_retry_and_transport_lost() {
    let bin = env!("CARGO_BIN_EXE_termux-chatgpt-mcp");
    let ver = Command::new(bin).arg("--version").output().unwrap();
    assert!(ver.status.success());
    assert!(String::from_utf8_lossy(&ver.stdout).contains("termux-chatgpt-mcp"));

    let desc = Command::new(bin).arg("--describe-tools").output().unwrap();
    assert!(desc.status.success());
    let desc_json: Value = serde_json::from_slice(&desc.stdout).unwrap();
    assert_eq!(desc_json["tools"].as_array().unwrap().len(), 6);

    let root = temp_dir("cg-mcp-down");
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let tok_path = write_private_token(&root, token);
    let rcpt = root.join("receipts");

    // Pick an unbound ephemeral loopback port
    let unused_port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let dead_url = format!("http://127.0.0.1:{unused_port}");

    // 1. Provider down: chatgpt_status is graceful; chatgpt_send fails WITHOUT poisoning call_id!
    let resps = run_mcp_session(
        &dead_url,
        &tok_path,
        &rcpt,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"chatgpt_status","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"retry-ok-1","prompt":"hello"}}}),
        ],
    );
    let (st, st_err) = tool_payload(&resps[0]);
    assert!(!st_err);
    assert_eq!(st["provider_reachable"], false);
    assert_eq!(st["auth_state"], "provider_stopped");

    let (send_down, send_err) = tool_payload(&resps[1]);
    assert!(send_err);
    assert_eq!(send_down["executed"], false);
    assert_eq!(send_down["error"], "chatgpt_provider_unavailable");

    // 2. Now start a live mock provider -> the EXACT SAME call_id must succeed cleanly!
    let sends = Arc::new(AtomicUsize::new(0));
    let (live_url, h) = start_mock_provider(token.to_owned(), Arc::clone(&sends), false);
    let resps2 = run_mcp_session(
        &live_url,
        &tok_path,
        &rcpt,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"retry-ok-1","prompt":"hello"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"retry-ok-1","prompt":"hello"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"retry-ok-1","prompt":"different"}}}),
        ],
    );
    stop_mock_provider(&live_url, h);

    let (first, err1) = tool_payload(&resps2[0]);
    assert!(!err1);
    assert_eq!(first["assistant_text"], "REPLY_1");
    assert_eq!(first["replay_safe"], true);

    let (replay, err2) = tool_payload(&resps2[1]);
    assert!(!err2);
    assert_eq!(replay["replayed_from_receipt"], true);
    assert_eq!(replay["assistant_text"], "REPLY_1");
    assert_eq!(sends.load(Ordering::SeqCst), 1);

    let (conflict, err3) = tool_payload(&resps2[2]);
    assert!(err3);
    assert_eq!(conflict["error"], "chatgpt_call_id_payload_conflict");

    // 3. Mid-flight transport drop -> delivery_unknown, and exact retry NEVER re-executes!
    let drop_sends = Arc::new(AtomicUsize::new(0));
    let (drop_url, h2) = start_mock_provider(token.to_owned(), Arc::clone(&drop_sends), true);
    let resps3 = run_mcp_session(
        &drop_url,
        &tok_path,
        &rcpt,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"drop-1","prompt":"uncertain"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"send","call_id":"drop-1","prompt":"uncertain"}}}),
        ],
    );
    stop_mock_provider(&drop_url, h2);

    let (d1, de1) = tool_payload(&resps3[0]);
    assert!(de1);
    assert_eq!(d1["state"], "delivery_unknown");
    let (d2, de2) = tool_payload(&resps3[1]);
    assert!(de2);
    assert_eq!(d2["state"], "delivery_unknown");
    assert_eq!(drop_sends.load(Ordering::SeqCst), 1);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn end_to_end_stdio_bridge_covers_all_six_tools_and_all_five_extensions() {
    let root = temp_dir("cg-mcp-e2e");
    let token = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    let tok_path = write_private_token(&root, token);
    let rcpt = root.join("receipts");
    let sends = Arc::new(AtomicUsize::new(0));
    let (url, h) = start_mock_provider(token.to_owned(), Arc::clone(&sends), false);

    let resps = run_mcp_session(
        &url,
        &tok_path,
        &rcpt,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"chatgpt_status","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"chatgpt_chats","arguments":{"action":"list","limit":10}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"chatgpt_chats","arguments":{"action":"read","chat_id":"c1"}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"chatgpt_projects","arguments":{"action":"list"}}}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"chatgpt_projects","arguments":{"action":"read","project_id":"p1"}}}),
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"observe","chat_id":"c1"}}}),
            json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"chatgpt_send","arguments":{"action":"stop"}}}),
            json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"chatgpt_catalog","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"chatgpt_call","arguments":{"action":"project_create","arguments":{"name":"ProjX","instructions":"Be sharp"}}}}),
            json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"chatgpt_call","arguments":{"action":"chat_rename","arguments":{"chat_id":"c1","title":"Renamed"}}}}),
            json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"chatgpt_call","arguments":{"action":"viewer_url","arguments":{}}}}),
            json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"chatgpt_call","arguments":{"action":"viewer_open","arguments":{}}}}),
            json!({"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"chatgpt_call","arguments":{"action":"host_restart","arguments":{}}}}),
        ],
    );
    stop_mock_provider(&url, h);

    assert_eq!(resps[0]["result"]["serverInfo"]["name"], "TermuxChatGPT");
    assert_eq!(tool_payload(&resps[1]).0["auth_state"], "authenticated");
    assert_eq!(tool_payload(&resps[2]).0["chats"][0]["id"], "c1");
    assert_eq!(tool_payload(&resps[3]).0["messages"][0]["text"], "hello");
    assert_eq!(tool_payload(&resps[4]).0["projects"][0]["id"], "p1");
    assert_eq!(tool_payload(&resps[5]).0["chats"][0]["id"], "c1");
    assert_eq!(tool_payload(&resps[6]).0["state"], "settled");
    assert_eq!(tool_payload(&resps[7]).0["stopped"], true);
    assert_eq!(tool_payload(&resps[8]).0["count"], 5);
    assert_eq!(tool_payload(&resps[9]).0["project_id"], "p-new");
    assert_eq!(tool_payload(&resps[10]).0["title"], "Renamed");
    assert_eq!(tool_payload(&resps[11]).0["available"], true);
    assert_eq!(tool_payload(&resps[12]).0["viewer_open_requested"], true);
    assert_eq!(tool_payload(&resps[13]).0["restarted"], true);

    let _ = fs::remove_dir_all(&root);
}
