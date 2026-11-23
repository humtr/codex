//! `termux-orch` — Stage 2 Slice 7 Durable Multi-Role Orchestration (`@TermuxOrch` + `termux-orch relay`).
//!
//! Subcommands:
//! - `termux-orch mcp`                                           — stdio MCP server (`@TermuxOrch`, 6 static tools)
//! - `termux-orch relay [--bind 127.0.0.1:0] [--health-url-file <path>] [--poll-ms 3000]`
//! - `termux-orch tick`                                          — run a single Runtime (R) tick
//! - `termux-orch status`                                        — inspect state root & works summary
//! - `termux-orch --describe-tools | --version | --help`

mod http_client;
mod ledger;
mod mcp;
mod owners;
mod relay;
mod rollover;
mod shell_job;

use crate::ledger::Ledger;
use crate::mcp::{build_static_tools, OrchMcpHandler};
use crate::owners::{OwnerClient, OwnerConfig};
use crate::relay::{run_relay_daemon, RelayEngine};
use mcp_kit::serve_stdio;
use serde_json::json;
use std::env;
use std::path::PathBuf;

fn default_state_root() -> PathBuf {
    if let Some(custom) = env::var_os("ORCH_STATE_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(custom);
    }
    let home = env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/home"));
    home.join(".local/state/codex/orchestration")
}

fn default_receipt_dir() -> PathBuf {
    if let Some(custom) = env::var_os("ORCH_RECEIPT_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(custom);
    }
    let prefix = env::var_os("PREFIX")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/data/data/com.termux/files/usr"));
    prefix.join("var/tmp/termux-orch-mcp-receipts")
}

fn print_help() {
    println!(
        "termux-orch {} — Stage 2 Durable Multi-Role Orchestration (@TermuxOrch + Runtime R)\n\n\
         Usage:\n\
         \x20 termux-orch mcp\n\
         \x20 termux-orch relay [--bind 127.0.0.1:0] [--health-url-file <path>] [--poll-ms 3000]\n\
         \x20 termux-orch tick\n\
         \x20 termux-orch status\n\
         \x20 termux-orch --describe-tools | --version | --help",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => {
            print_help();
        }
        Some("-V" | "--version") => {
            println!("termux-orch {}", env!("CARGO_PKG_VERSION"));
        }
        Some("--describe-tools") => {
            let tools: Vec<_> = build_static_tools()
                .iter()
                .map(|t| t.to_mcp_json())
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "tools": tools })).unwrap()
            );
        }
        None | Some("mcp") => {
            let ledger = match Ledger::open(default_state_root()) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("termux-orch: failed to open orchestration ledger: {e}");
                    std::process::exit(1);
                }
            };
            let owners = OwnerClient::new(OwnerConfig::from_env());
            let mut handler = OrchMcpHandler::new(ledger, owners, default_receipt_dir());
            if let Err(e) = serve_stdio(&mut handler) {
                eprintln!("termux-orch mcp error: {e}");
                std::process::exit(1);
            }
        }
        Some("relay") => {
            let mut bind = "127.0.0.1:0".to_owned();
            let mut health_url_file = env::var_os("HOME").map(|home| {
                PathBuf::from(home).join(".local/state/codex/services/termux-orch-relay/health.url")
            });
            let mut poll_ms: u64 = 3000;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--bind" if i + 1 < args.len() => {
                        bind = args[i + 1].clone();
                        i += 2;
                    }
                    "--health-url-file" if i + 1 < args.len() => {
                        health_url_file = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    }
                    "--poll-ms" if i + 1 < args.len() => {
                        poll_ms = args[i + 1].parse().unwrap_or(3000);
                        i += 2;
                    }
                    other => {
                        eprintln!("termux-orch relay: unknown flag '{other}'");
                        std::process::exit(2);
                    }
                }
            }
            let ledger = match Ledger::open(default_state_root()) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("termux-orch relay: failed to open ledger: {e}");
                    std::process::exit(1);
                }
            };
            let owners = OwnerClient::new(OwnerConfig::from_env());
            if let Err(e) =
                run_relay_daemon(ledger, owners, &bind, health_url_file.as_deref(), poll_ms)
            {
                eprintln!("termux-orch relay error: {e}");
                std::process::exit(1);
            }
        }
        Some("shell-job") if args.len() == 5 => {
            let result = Ledger::open(PathBuf::from(&args[1])).and_then(|ledger| {
                shell_job::ShellJob::new(&ledger, &args[2], &args[3])?
                    .run(&ledger, &args[2], &args[4])
            });
            if result.is_err() {
                std::process::exit(1);
            }
        }
        Some("tick") => {
            let ledger = match Ledger::open(default_state_root()) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("termux-orch tick: failed to open ledger: {e}");
                    std::process::exit(1);
                }
            };
            let owners = OwnerClient::new(OwnerConfig::from_env());
            let engine = RelayEngine::new(ledger, owners);
            let tick = engine.tick_once();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "works_scanned": tick.works_scanned,
                    "participants_settled": tick.participants_settled,
                    "wait_sets_fired": tick.wait_sets_fired,
                    "deliveries_sent": tick.deliveries_sent,
                    "deliveries_deferred": tick.deliveries_deferred,
                    "budget_trips": tick.budget_trips
                }))
                .unwrap()
            );
        }
        Some("status") => {
            let root = default_state_root();
            let ledger = match Ledger::open(root.clone()) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("termux-orch status: failed to open ledger: {e}");
                    std::process::exit(1);
                }
            };
            let works = ledger.list_works().unwrap_or_default();
            let active = works.iter().filter(|w| !w.is_terminal()).count();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "state_root": root.display().to_string(),
                    "total_works": works.len(),
                    "active_works": active
                }))
                .unwrap()
            );
        }
        Some(other) => {
            eprintln!("termux-orch: unknown command '{other}'");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{build_extension_registry, STATIC_TOOL_NAMES};
    use mcp_kit::McpHandler;
    use serde_json::Value;
    use std::collections::HashSet;
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::{Arc, Mutex};
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

    #[test]
    fn exposes_exactly_six_static_tools_with_uniform_annotations_and_disjoint_extensions() {
        let tools = build_static_tools();
        assert_eq!(tools.len(), 6);
        let names: HashSet<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        for expected in STATIC_TOOL_NAMES {
            assert!(names.contains(expected), "missing static tool {expected}");
        }
        for t in &tools {
            let j = t.to_mcp_json();
            assert_eq!(j["annotations"]["readOnlyHint"], true);
            assert_eq!(j["annotations"]["destructiveHint"], false);
            assert_eq!(j["annotations"]["openWorldHint"], false);
        }
        let reg = build_extension_registry();
        let cat = reg.catalog_json();
        for spec in cat["actions"].as_array().unwrap() {
            let act = spec["action"].as_str().unwrap();
            assert!(!names.contains(act), "extension overlaps static tool {act}");
        }
    }

    #[test]
    fn ledger_and_relay_full_tier1_promote_barrier_front_busy_backoff_and_rebind() {
        let root = tmp_dir("termux-orch-e2e");
        let state_dir = root.join("orch");
        let rcpt_dir = root.join("rcpts");
        let token_path = root.join("control.token");
        {
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(&token_path)
                .unwrap();
            f.write_all(b"0123456789abcdef0123456789abcdef0123456789abcdef\n")
                .unwrap();
        }

        #[derive(Default)]
        struct MockState {
            front_streaming: bool,
            worker_settled: bool,
            renames: Vec<(String, String)>,
            sends: Vec<(Option<String>, String)>,
        }
        let mstate = Arc::new(Mutex::new(MockState {
            front_streaming: true,
            worker_settled: false,
            renames: Vec::new(),
            sends: Vec::new(),
        }));
        let mstate_srv = Arc::clone(&mstate);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let stop_srv = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_srv_clone = Arc::clone(&stop_srv);
        let srv = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            while !stop_srv_clone.load(std::sync::atomic::Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                        let mut raw = Vec::new();
                        let mut tmp = [0u8; 4096];
                        let mut content_len: Option<usize> = None;
                        let mut header_end: Option<usize> = None;
                        while let Ok(n) = stream.read(&mut tmp) {
                            if n == 0 {
                                break;
                            }
                            raw.extend_from_slice(&tmp[..n]);
                            if header_end.is_none() {
                                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                                    header_end = Some(pos + 4);
                                    let head = String::from_utf8_lossy(&raw[..pos]);
                                    for line in head.lines() {
                                        if let Some((k, v)) = line.split_once(':') {
                                            if k.trim().eq_ignore_ascii_case("content-length") {
                                                content_len = v.trim().parse::<usize>().ok();
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(he) = header_end {
                                if raw.len() >= he + content_len.unwrap_or(0) {
                                    break;
                                }
                            }
                        }
                        let req = String::from_utf8_lossy(&raw);
                        let first = req.lines().next().unwrap_or("");
                        let body_str = req.split("\r\n\r\n").nth(1).unwrap_or("");
                        let body: Value = serde_json::from_str(body_str).unwrap_or(json!({}));
                        let mut st = mstate_srv.lock().unwrap();
                        let resp_val = if first.starts_with("GET /control/status") {
                            json!({"state":"ok","account":{"account_id":"acc-1","id":"u1"}})
                        } else if first.starts_with("POST /control/projects") {
                            json!({"state":"ok","project_id":"g-p-orch1","name":body["name"]})
                        } else if first.starts_with("GET /control/chats?") {
                            json!({"state":"ok","chats":[{"id":"chat-f2","title":"New Mobile Chat"}]})
                        } else if first.starts_with("GET /control/chats/chat-f2") {
                            json!({"state":"ok","messages":[{"role":"user","text":"MARKER_F2_CONTINUE please continue"}]})
                        } else if first.contains("/rename") {
                            let cid = first
                                .split("/control/chats/")
                                .nth(1)
                                .and_then(|s| s.split("/rename").next())
                                .unwrap_or("")
                                .to_owned();
                            st.renames
                                .push((cid, body["title"].as_str().unwrap_or("").to_owned()));
                            json!({"state":"ok"})
                        } else if first.starts_with("POST /control/send") {
                            let cid = body["chat_id"].as_str().map(ToOwned::to_owned);
                            let prompt = body["prompt"].as_str().unwrap_or("").to_owned();
                            st.sends.push((cid.clone(), prompt));
                            let assigned = cid.unwrap_or_else(|| {
                                if body["prompt"].as_str().unwrap_or("").contains("Lead [L1]") {
                                    "chat-l1".to_owned()
                                } else {
                                    "chat-w1".to_owned()
                                }
                            });
                            json!({"state":"in_progress","chat_id":assigned,"generating":true})
                        } else if first.starts_with("POST /control/observe") {
                            let cid = body["chat_id"].as_str().unwrap_or("");
                            if cid == "chat-f1" || cid == "chat-f2" {
                                if st.front_streaming {
                                    json!({"state":"in_progress","chat_id":cid,"generating":true,"remote_stream_status":"IS_STREAMING"})
                                } else {
                                    json!({"state":"settled","chat_id":cid,"generating":false,"remote_stream_status":"COMPLETE"})
                                }
                            } else if cid == "chat-w1" {
                                if st.worker_settled {
                                    json!({"state":"settled","chat_id":cid,"generating":false,"assistant_text":"WORKER_1_DONE_OK"})
                                } else {
                                    json!({"state":"in_progress","chat_id":cid,"generating":true})
                                }
                            } else {
                                json!({"state":"settled","chat_id":cid,"generating":false})
                            }
                        } else {
                            json!({"state":"ok"})
                        };
                        let resp_body = resp_val.to_string();
                        let http = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                            resp_body.len()
                        );
                        let _ = stream.write_all(http.as_bytes());
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        let ledger = Ledger::open(state_dir).unwrap();
        let owners = OwnerClient::new(OwnerConfig {
            provider_url: format!("http://{addr}"),
            token_path: Some(token_path),
            home: root.clone(),
            tmpdir: root.clone(),
            codex_socket_override: None,
            notify_cmd: Some("/bin/true".to_owned()),
        });
        let mut handler = OrchMcpHandler::new(ledger.clone(), owners.clone(), rcpt_dir);

        // 1. Tier 1 direct Work creation + auto project + [F1] rename
        let (created, is_err) = handler.call_tool(
            "orch_work",
            &json!({
                "action": "create",
                "call_id": "c-create-1",
                "title": "Build Pipeline",
                "goal": "Run long build and verify",
                "tier": "direct",
                "front_chat_id": "chat-f1"
            }),
        );
        assert!(!is_err, "create failed: {created}");
        let wid = created["work_id"].as_str().unwrap().to_owned();
        assert_eq!(created["project_id"], "g-p-orch1");

        // 2. Register Worker W1 (spawns chat-w1, renames [W1-build] ...) & arm Wait-Set
        let (w1, is_err) = handler.call_tool(
            "orch_dispatch",
            &json!({
                "action": "register_worker",
                "call_id": "c-w1",
                "work_id": wid,
                "backend": "chatgpt",
                "slug": "build",
                "task_summary": "Run release build",
                "prompt": "Build it"
            }),
        );
        assert!(!is_err, "register_worker failed: {w1}");
        assert_eq!(w1["participant"]["participant_id"], "W1");
        assert_eq!(w1["session_title"], "[W1-build] Run release build");

        // 3. Promote Tier 1 -> Tier 2/3 orchestrated in place (attaches Lead L1, keeps W1!)
        let (prom, is_err) = handler.call_tool(
            "orch_work",
            &json!({
                "action": "promote",
                "call_id": "c-prom-1",
                "work_id": wid,
                "lead_kind": "chatgpt"
            }),
        );
        assert!(!is_err, "promote failed: {prom}");
        assert_eq!(prom["tier"], "orchestrated");
        assert_eq!(prom["lead"]["ref_id"], "chat-l1");
        assert_eq!(prom["preserved_participants"], 1);

        // 4. Arm Fan-in Barrier Wait-Set for W1 -> wake Lead L1
        let (ws, is_err) = handler.call_tool(
            "orch_dispatch",
            &json!({
                "action": "wait_set",
                "call_id": "c-ws-1",
                "work_id": wid,
                "wake_target": "lead",
                "all_of": ["W1"]
            }),
        );
        assert!(!is_err, "wait_set failed: {ws}");

        // 5. While W1 is running, relay tick settles nothing
        let relay = RelayEngine::new(ledger.clone(), owners.clone());
        let t0 = relay.tick_once();
        assert_eq!(t0.participants_settled, 0);
        assert_eq!(t0.wait_sets_fired, 0);

        // 6. W1 finishes -> relay tick settles W1, fires barrier, wakes Lead L1!
        mstate.lock().unwrap().worker_settled = true;
        let t1 = relay.tick_once();
        assert_eq!(t1.participants_settled, 1);
        assert_eq!(t1.wait_sets_fired, 1);
        assert_eq!(t1.deliveries_sent, 1);
        let work_after_wake = ledger.load_work(&wid).unwrap();
        assert_eq!(work_after_wake.budget.wakeups_used, 1);
        assert_eq!(work_after_wake.consumed_seq, 4);
        assert_eq!(work_after_wake.last_seq, 5);

        // 7. New mobile chat F2 rebinds via marker -> [F2] Build Pipeline!
        let (reb, is_err) = handler.call_tool(
            "orch_work",
            &json!({
                "action": "rebind_front",
                "call_id": "c-rebind-2",
                "work_id": wid,
                "front_marker": "MARKER_F2_CONTINUE"
            }),
        );
        assert!(!is_err, "rebind_front failed: {reb}");
        assert_eq!(reb["front_chat_id"], "chat-f2");
        assert_eq!(reb["front_title"], "[F2] Build Pipeline");

        // 8. Lead submits complete report while F2 is streaming on mobile -> deferred (PENDING_DELIVERY)!
        let (rep, is_err) = handler.call_tool(
            "orch_report",
            &json!({
                "action": "complete",
                "call_id": "c-rep-final",
                "work_id": wid,
                "summary": "All verified!",
                "flush_now": true
            }),
        );
        assert!(!is_err, "report failed: {rep}");
        assert_eq!(rep["delivered_immediately"], false);
        assert_eq!(rep["pending_deliveries"], 1);

        // 9. Mobile finishes streaming (stream_status=COMPLETE), force next_attempt_unix=0 -> delivered to F2!
        mstate.lock().unwrap().front_streaming = false;
        {
            let _lk = ledger.lock_work(&wid).unwrap();
            let mut w = ledger.load_work(&wid).unwrap();
            w.pending_deliveries[0].next_attempt_unix = 0;
            ledger.save_work(&w).unwrap();
        }
        let t2 = relay.tick_once();
        assert_eq!(t2.deliveries_sent, 1);
        let final_w = ledger.load_work(&wid).unwrap();
        assert!(final_w.pending_deliveries.is_empty());
        assert_eq!(final_w.state, "completed");

        let sends = mstate.lock().unwrap().sends.clone();
        assert!(sends
            .iter()
            .any(|(cid, p)| cid.as_deref() == Some("chat-l1") && p.contains("[ORCH_WAKEUP]")));
        assert!(sends
            .iter()
            .any(|(cid, p)| cid.as_deref() == Some("chat-f2")
                && p.contains("[ORCH_REPORT:complete]")));

        stop_srv.store(true, std::sync::atomic::Ordering::Release);
        let _ = srv.join();
        let _ = fs::remove_dir_all(&root);
    }
}
