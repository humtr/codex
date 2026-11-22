use serde_json::{json, Value};
use std::fs::{self, DirBuilder};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_termux-codex-mcp"))
}

fn temp_dir(prefix: &str) -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{ns}", std::process::id()));
    DirBuilder::new().mode(0o700).create(&dir).unwrap();
    dir
}

#[test]
fn end_to_end_stdio_bridge_one_shot_turn_replay_steer_interrupt_and_extensions() {
    let root = temp_dir("termux-codex-e2e");
    let home = root.join("home");
    let tmp = root.join("tmp");
    let rcpt = root.join("rcpt");
    let codex_home = home.join(".codex");
    fs::create_dir_all(codex_home.join("app-server-control")).unwrap();
    fs::create_dir_all(&tmp).unwrap();
    DirBuilder::new().mode(0o700).create(&rcpt).unwrap();

    let sock_path = tmp.join("mock-codex.sock");
    let listener = UnixListener::bind(&sock_path).unwrap();
    std::os::unix::fs::symlink(
        "/proc/self/fd/35/mock-codex.sock",
        codex_home.join("app-server-control/app-server-control.sock"),
    )
    .unwrap();

    let turn_start_count = Arc::new(AtomicUsize::new(0));
    let turn_count_srv = Arc::clone(&turn_start_count);

    // Mock official app-server daemon handling multiple sequential connections!
    let server = thread::spawn(move || {
        let mut active_steer_phase = false;
        for conn in listener.incoming().take(6) {
            let stream = conn.unwrap();
            let mut writer = stream.try_clone().unwrap();
            let mut reader = BufReader::new(stream);
            let mut key = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        key = v.trim().to_owned();
                    }
                }
            }
            let accept = ws_accept(&key);
            write!(
                writer,
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
            )
            .unwrap();
            writer.flush().unwrap();

            while let Some(msg) = read_ws_json(&mut reader) {
                let method = msg["method"].as_str().unwrap_or("");
                let id = msg.get("id").and_then(Value::as_u64);
                let resp = match method {
                    "initialize" => {
                        Some(json!({"id": id, "result": {"userAgent": "mock/0.171.0"}}))
                    }
                    "initialized" => None,
                    "thread/loaded/list" => {
                        Some(json!({"id": id, "result": {"data": ["tid-1"], "nextCursor": null}}))
                    }
                    "modelProvider/list" => {
                        Some(json!({"id": id, "result": {"data": [{"id": "openai"}]}}))
                    }
                    "config/read" => Some(
                        json!({"id": id, "result": {"config": {"model": "gpt-5.4", "model_provider": "openai", "secret_token": "MUST_NOT_LEAK"}}}),
                    ),
                    "thread/start" => {
                        Some(json!({"id": id, "result": {"thread": {"id": "tid-new-1"}}}))
                    }
                    "turn/start" => {
                        turn_count_srv.fetch_add(1, Ordering::SeqCst);
                        Some(json!({"id": id, "result": {"turn": {"id": "turn-1"}}}))
                    }
                    "thread/read" => {
                        let tid = msg["params"]["threadId"].as_str().unwrap_or("");
                        if tid == "tid-steer" {
                            let status = if active_steer_phase {
                                "completed"
                            } else {
                                "inProgress"
                            };
                            Some(json!({
                                "id": id,
                                "result": {
                                    "thread": {
                                        "id": "tid-steer",
                                        "status": {"type": if active_steer_phase { "idle" } else { "active" }, "activeFlags": []},
                                        "turns": [{"id": "turn-live-9", "status": status, "items": [{"type": "agentMessage", "text": "steered ok"}]}]
                                    }
                                }
                            }))
                        } else {
                            Some(json!({
                                "id": id,
                                "result": {
                                    "thread": {
                                        "id": tid,
                                        "status": {"type": "idle", "activeFlags": []},
                                        "cwd": "/tmp",
                                        "name": "Demo",
                                        "modelProvider": "openai",
                                        "turns": [{"id": "turn-1", "status": "completed", "items": [{"type": "agentMessage", "text": "Hello from Codex 0.171.0"}]}]
                                    }
                                }
                            }))
                        }
                    }
                    "turn/steer" => {
                        assert_eq!(msg["params"]["expectedTurnId"], "turn-live-9");
                        active_steer_phase = true;
                        Some(json!({"id": id, "result": {}}))
                    }
                    "turn/interrupt" => {
                        assert_eq!(msg["params"]["turnId"], "turn-live-9");
                        Some(json!({"id": id, "result": {}}))
                    }
                    "thread/name/set" => Some(json!({"id": id, "result": {}})),
                    "account/rateLimits/read" => Some(
                        json!({"id": id, "result": {"rateLimits": {"primary": {"usedPercent": 12}}}}),
                    ),
                    _ => Some(json!({"id": id, "result": {}})),
                };
                if let Some(r) = resp {
                    send_ws_unmasked(&mut writer, r.to_string().as_bytes());
                }
            }
        }
    });

    let mut child = Command::new(bin())
        .env("HOME", &home)
        .env("TMPDIR", &tmp)
        .env("TERMUX_CODEX_RECEIPT_DIR", &rcpt)
        .env_remove("CODEX_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let reqs = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"codex_status","arguments":{}}}),
        // 1-shot new thread + turn start
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"codex_turn","arguments":{"action":"start","call_id":"c-start-1","prompt":"hello","wait_ms":3000}}}),
        // Duplicate retry with same call_id -> replayed from receipt without calling turn/start again!
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"codex_turn","arguments":{"action":"start","call_id":"c-start-1","prompt":"hello","wait_ms":3000}}}),
        // Steer with auto-resolved expected_turn_id
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"codex_turn","arguments":{"action":"steer","call_id":"c-steer-1","thread_id":"tid-steer","prompt":"change course","wait_ms":1000}}}),
        // Interrupt with explicit turn_id
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"codex_turn","arguments":{"action":"interrupt","call_id":"c-int-1","thread_id":"tid-steer","turn_id":"turn-live-9"}}}),
        // Extension catalog + call (thread_rename + account_usage)
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"codex_call","arguments":{"action":"thread_rename","arguments":{"thread_id":"tid-new-1","name":"Renamed"}}}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"codex_call","arguments":{"action":"account_usage","arguments":{}}}}),
    ];
    {
        let stdin = child.stdin.as_mut().unwrap();
        for r in &reqs {
            writeln!(stdin, "{r}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    server.join().unwrap();

    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 9);

    // 5 static tools
    assert_eq!(lines[1]["result"]["tools"].as_array().unwrap().len(), 5);

    // codex_status redacted secrets
    let status_text = lines[2]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!status_text.contains("MUST_NOT_LEAK"));
    let status_val: Value = serde_json::from_str(status_text).unwrap();
    assert_eq!(status_val["daemon_running"], true);
    assert_eq!(status_val["config"]["model"], "gpt-5.4");

    // 1-shot start + idempotent replay
    let t1: Value =
        serde_json::from_str(lines[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(t1["thread_id"], "tid-new-1");
    assert_eq!(t1["assistant_text"], "Hello from Codex 0.171.0");
    assert_eq!(t1["replay_safe"], true);

    let t1_replay: Value =
        serde_json::from_str(lines[4]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(t1_replay["replayed_from_receipt"], true);
    assert_eq!(turn_start_count.load(Ordering::SeqCst), 1);

    // steer + interrupt + extensions
    let steer: Value =
        serde_json::from_str(lines[5]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(steer["state"], "settled");
    let intr: Value =
        serde_json::from_str(lines[6]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(intr["state"], "interrupted");
    assert_eq!(lines[7]["result"]["isError"], false);
    assert_eq!(lines[8]["result"]["isError"], false);

    let _ = fs::remove_dir_all(&root);
}

fn read_ws_json(reader: &mut BufReader<UnixStream>) -> Option<Value> {
    let mut hdr = [0u8; 2];
    reader.read_exact(&mut hdr).ok()?;
    let op = hdr[0] & 0x0f;
    if op == 0x8 {
        return None;
    }
    let masked = (hdr[1] & 0x80) != 0;
    let mut len = (hdr[1] & 0x7f) as usize;
    if len == 126 {
        let mut ext = [0u8; 2];
        reader.read_exact(&mut ext).ok()?;
        len = u16::from_be_bytes(ext) as usize;
    }
    let mut mask = [0u8; 4];
    if masked {
        reader.read_exact(&mut mask).ok()?;
    }
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).ok()?;
    if masked {
        for (i, b) in data.iter_mut().enumerate() {
            *b ^= mask[i & 3];
        }
    }
    serde_json::from_slice(&data).ok()
}

fn send_ws_unmasked(writer: &mut UnixStream, payload: &[u8]) {
    let mut hdr = vec![0x81u8];
    if payload.len() < 126 {
        hdr.push(payload.len() as u8);
    } else {
        hdr.push(126);
        hdr.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    writer.write_all(&hdr).unwrap();
    writer.write_all(payload).unwrap();
    writer.flush().unwrap();
}

fn ws_accept(key: &str) -> String {
    let mut input = key.as_bytes().to_vec();
    input.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    b64(&sha1(&input))
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let (mut h0, mut h1, mut h2, mut h3, mut h4) = (
        0x6745_2301u32,
        0xEFCD_AB89u32,
        0x98BA_DCFEu32,
        0x1032_5476u32,
        0xC3D2_E1F0u32,
    );
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 80];
    for chunk in msg.as_chunks::<64>().0 {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDCu32),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, h) in [h0, h1, h2, h3, h4].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&h.to_be_bytes());
    }
    out
}

fn b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        s.push(T[((n >> 18) & 63) as usize] as char);
        s.push(T[((n >> 12) & 63) as usize] as char);
        s.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        s.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    s
}
