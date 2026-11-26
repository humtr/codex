use serde_json::{json, Value};
use std::fs::{self, DirBuilder};
use std::io::Write;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_termux-shell-mcp"))
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
fn compiled_binary_describe_tools_stdio_replay_and_cwd_validation() {
    let root = temp_dir("termux-shell-e2e");
    let rcpt = root.join("receipts");
    DirBuilder::new().mode(0o700).create(&rcpt).unwrap();

    let desc = Command::new(bin())
        .arg("--describe-tools")
        .output()
        .unwrap();
    assert!(desc.status.success());
    let desc_json: Value = serde_json::from_slice(&desc.stdout).unwrap();
    assert_eq!(desc_json["tools"].as_array().unwrap().len(), 1);
    assert_eq!(desc_json["tools"][0]["name"], "termux_shell");
    assert_eq!(desc_json["tools"][0]["annotations"]["readOnlyHint"], false);
    assert_eq!(
        desc_json["tools"][0]["annotations"]["destructiveHint"],
        true
    );
    assert_eq!(desc_json["tools"][0]["annotations"]["openWorldHint"], true);

    let mut child = Command::new(bin())
        .env("TERMUX_SHELL_RECEIPT_DIR", &rcpt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let marker = root.join("once");
    let cmd = format!("printf 'E2E_OK'; printf 1 >> '{}'", marker.display());
    let reqs = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"termux_shell","arguments":{"call_id":"e2e-1","command":cmd,"cwd":root}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"termux_shell","arguments":{"call_id":"e2e-1","command":cmd,"cwd":root}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"termux_shell","arguments":{"call_id":"e2e-badcwd","command":"echo hi","cwd":"/nonexistent-dir-test"}}}),
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

    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 4);

    let r1: Value =
        serde_json::from_str(lines[1]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(r1["stdout"], "E2E_OK");
    assert_eq!(r1["replay_safe"], true);

    let r2: Value =
        serde_json::from_str(lines[2]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(r2["replayed_from_receipt"], true);
    assert_eq!(r2["replay_safe"], true);
    assert_eq!(fs::read(&marker).unwrap(), b"1");

    let r3: Value =
        serde_json::from_str(lines[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(r3["error"], "termux_cwd_not_found");
    assert_eq!(r3["executed"], false);
    assert_eq!(lines[3]["result"]["isError"], true);

    let _ = fs::remove_dir_all(&root);
}
