//! Real standalone MCP boundary: no Provider binding, Core or Native claim owner.
#![cfg(unix)]
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Worker {
    child: Child,
    replies: mpsc::Receiver<Value>,
    next: u64,
}
impl Worker {
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let row = json!({"jsonrpc":"2.0","id":self.next,"method":method,"params":params});
        writeln!(self.child.stdin.as_mut().unwrap(), "{row}").unwrap();
        self.child.stdin.as_mut().unwrap().flush().unwrap();
        let response = self
            .replies
            .recv_timeout(Duration::from_secs(10))
            .expect("original MCP response");
        assert_eq!(response["id"], self.next);
        response
    }
    fn shell(&mut self, args: &Value) -> Value {
        let response = self.call(
            "tools/call",
            json!({"name":"termux_shell","arguments":args}),
        );
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn standalone_stdio_survives_absent_provider_and_preserves_original_effect() {
    let root = std::env::temp_dir().join(format!("termux-worker-boundary-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let prefix = root.join("prefix");
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::create_dir_all(prefix.join("var/tmp/codex-native-effects/shared-native-v1")).unwrap();
    let actual_shell = std::env::var_os("PREFIX")
        .map(PathBuf::from)
        .map(|p| p.join("bin/sh"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("/bin/sh"));
    assert!(actual_shell.is_file(), "test requires a real POSIX shell");
    std::os::unix::fs::symlink(actual_shell, prefix.join("bin/sh")).unwrap();
    let claims = prefix.join("var/tmp/codex-native-effects/shared-native-v1/claims.json");
    fs::write(&claims, b"isolated inaccessible Native admission sentinel").unwrap();
    fs::set_permissions(&claims, fs::Permissions::from_mode(0o000)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codex-termux-mcp-worker"))
        .args(["--state", root.join("absent-provider").to_str().unwrap()])
        .env("PREFIX", &prefix)
        .env("HOME", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (send, replies) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let Ok(value) = serde_json::from_str(&line) else {
                break;
            };
            if send.send(value).is_err() {
                break;
            }
        }
    });
    let mut worker = Worker {
        child,
        replies,
        next: 0,
    };
    assert!(worker.call("initialize",json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"worker-test","version":"1"}}))["result"].is_object());
    writeln!(
        worker.child.stdin.as_mut().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let tools = worker.call("tools/list", json!({}));
    let tools = tools["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "termux_shell");
    let args = json!({"call_id":"original","command":"printf x >> counter; printf WORKER_BOUNDARY_OK","cwd":root,"timeout_ms":1000});
    let first = worker.shell(&args);
    assert_eq!(first["exit_code"], 0);
    assert_eq!(first["stdout"], "WORKER_BOUNDARY_OK");
    let replay = worker.shell(&args);
    assert_eq!(replay["replayed_from_receipt"], true);
    assert_eq!(fs::read(root.join("counter")).unwrap(), b"x");
    let mut changed = args;
    changed["command"] = json!("printf duplicate >> counter");
    let conflict = worker.shell(&changed);
    assert_eq!(conflict["state"], "rejected");
    assert_eq!(conflict["error"], "termux_call_id_payload_conflict");
    assert_eq!(fs::read(root.join("counter")).unwrap(), b"x");
    drop(worker);
    fs::set_permissions(&claims, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        fs::read(&claims).unwrap(),
        b"isolated inaccessible Native admission sentinel"
    );
    fs::remove_dir_all(root).unwrap();
}
