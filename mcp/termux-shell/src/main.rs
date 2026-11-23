//! `@TermuxShell` — standalone independent Termux app-UID MCP host shell worker.
//!
//! Zero dependency on Codex Core, `chatgpt-provider`, or Linux AppHost.
//! Exposes exactly ONE static tool (`termux_shell`) with uniform immutable annotations
//! (`readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false`,
//! `idempotentHint: true`) and atomic `call_id` transport deduplication receipts.

use mcp_kit::{
    base64_encode, serve_stdio, sha256_hex, validate_call_id, McpHandler, ReceiptClaim,
    ReceiptStore, ToolDef,
};
use serde_json::{json, Value};
use std::env;
use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RECEIPT_SCHEMA: &str = "termux-mcp-shell-receipt";
const MAX_COMMAND_BYTES: usize = 32 * 1024;
const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_TIMEOUT_MS: u64 = 600_000;

fn tool_def() -> ToolDef {
    ToolDef::new(
        "termux_shell",
        "Independent Termux app-UID host shell (zero dependency on Codex, Provider, or Linux AppHost). \
         One MCP tool only: supply a stable call_id and resend the exact same arguments after uncertain \
         delivery to retrieve the original receipt without re-executing.",
        json!({
            "type": "object",
            "properties": {
                "call_id": { "type": "string", "minLength": 1, "maxLength": 160 },
                "command": { "type": "string", "minLength": 1, "maxLength": 32768 },
                "cwd": { "type": "string", "description": "Absolute working directory; defaults to Termux HOME" },
                "timeout_ms": { "type": "integer", "minimum": 1000, "maximum": 600000 }
            },
            "required": ["call_id", "command"],
            "additionalProperties": false
        }),
        true,
    )
}

fn fail(reason: &'static str) -> Value {
    json!({
        "state": "rejected",
        "executed": false,
        "error": reason
    })
}

const DEFAULT_TERMUX_PREFIX: &str = "/data/data/com.termux/files/usr";

fn default_receipt_root() -> Result<PathBuf, &'static str> {
    if let Some(custom) = env::var_os("TERMUX_SHELL_RECEIPT_DIR").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(custom));
    }
    let prefix = env::var_os("PREFIX")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TERMUX_PREFIX));
    Ok(prefix.join("var/tmp/termux-mcp-shell-receipts"))
}

fn default_shell_path() -> Result<PathBuf, &'static str> {
    if let Some(prefix) = env::var_os("PREFIX").filter(|v| !v.is_empty()) {
        let candidate = PathBuf::from(prefix).join("bin/sh");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    let fallback = PathBuf::from("/bin/sh");
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err("termux_shell_binary_unavailable")
}

struct ProcessGroupGuard(Option<i32>);

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            if pid > 0 {
                // Terminate ONLY the child's private process group.
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
    }
}

struct StreamCapture {
    handle: Option<thread::JoinHandle<()>>,
    data: Arc<Mutex<(Vec<u8>, bool)>>,
}

impl StreamCapture {
    fn spawn<R: Read + Send + 'static>(mut stream: R) -> Self {
        let data = Arc::new(Mutex::new((Vec::new(), false)));
        let shared = Arc::clone(&data);
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
                        let room = MAX_OUTPUT_BYTES.saturating_sub(guard.0.len());
                        guard.0.extend_from_slice(&buf[..room.min(n)]);
                        if n > room {
                            guard.1 = true;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            handle: Some(handle),
            data,
        }
    }

    fn finish(mut self, timeout: Duration) -> (Vec<u8>, bool) {
        let deadline = Instant::now() + timeout;
        let mut incomplete = false;
        if let Some(handle) = self.handle.take() {
            while !handle.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                incomplete = true;
            }
        }
        let mut guard = self.data.lock().unwrap_or_else(|e| e.into_inner());
        (std::mem::take(&mut guard.0), guard.1 || incomplete)
    }
}

fn run_shell_command(script: &str, cwd: &Path, timeout_ms: u64, shell: &Path) -> Value {
    let mut cmd = Command::new(shell);
    cmd.arg("-c")
        .arg(script)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => {
            return json!({
                "state": "spawn_failed",
                "executed": false,
                "error": "termux_shell_spawn_failed"
            });
        }
    };

    let pid = child.id() as i32;
    let group = ProcessGroupGuard((pid > 0).then_some(pid));
    let stdout = StreamCapture::spawn(child.stdout.take().expect("piped stdout"));
    let stderr = StreamCapture::spawn(child.stderr.take().expect("piped stderr"));

    let started = Instant::now();
    let deadline = started + Duration::from_millis(timeout_ms);
    let (exit_code, signal, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status.code(), status.signal(), false),
            Ok(None) => {
                if Instant::now() >= deadline {
                    if pid > 0 {
                        unsafe {
                            libc::kill(-pid, libc::SIGKILL);
                        }
                    }
                    let _ = child.wait();
                    break (None, Some(libc::SIGKILL), true);
                }
                thread::sleep(Duration::from_millis(15));
            }
            Err(_) => {
                drop(group);
                return json!({
                    "state": "delivery_unknown",
                    "executed": true,
                    "error": "termux_shell_wait_failed"
                });
            }
        }
    };

    // Reap ordinary surviving process-group descendants on command exit.
    drop(group);
    let (out, out_truncated) = stdout.finish(Duration::from_secs(2));
    let (err, err_truncated) = stderr.finish(Duration::from_secs(2));

    json!({
        "state": if timed_out { "timed_out" } else { "exited" },
        "executed": true,
        "pid": pid,
        "exit_code": exit_code,
        "signal": signal,
        "timed_out": timed_out,
        "duration_ms": started.elapsed().as_millis() as u64,
        "stdout": String::from_utf8_lossy(&out),
        "stderr": String::from_utf8_lossy(&err),
        "stdout_base64": base64_encode(&out),
        "stderr_base64": base64_encode(&err),
        "stdout_truncated": out_truncated,
        "stderr_truncated": err_truncated,
        "cwd": cwd.to_string_lossy()
    })
}

pub fn execute_with(receipt_root: &Path, shell: &Path, args: &Value) -> Value {
    if !args.as_object().is_some_and(|v| {
        v.keys()
            .all(|k| matches!(k.as_str(), "call_id" | "command" | "cwd" | "timeout_ms"))
    }) {
        return fail("termux_arguments_invalid");
    }
    let call_id = match validate_call_id(args) {
        Ok(id) => id,
        Err(_) => return fail("termux_call_id_invalid"),
    };
    let script = match args["command"].as_str() {
        Some(v) if !v.trim().is_empty() && v.len() <= MAX_COMMAND_BYTES && !v.contains('\0') => v,
        _ => return fail("termux_command_invalid"),
    };
    let cwd = match args.get("cwd") {
        Some(Value::String(v)) if Path::new(v).is_absolute() && !v.contains('\0') => {
            PathBuf::from(v)
        }
        None => match env::var_os("HOME").filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => return fail("termux_home_unavailable"),
        },
        _ => return fail("termux_cwd_invalid"),
    };
    if !cwd.is_dir() {
        return fail("termux_cwd_not_found");
    }
    let timeout_ms = match args.get("timeout_ms") {
        None => DEFAULT_TIMEOUT_MS,
        Some(v) => match v.as_u64() {
            Some(ms) if (1000..=MAX_TIMEOUT_MS).contains(&ms) => ms,
            _ => return fail("termux_timeout_invalid"),
        },
    };

    let store = match ReceiptStore::open(
        receipt_root.to_path_buf(),
        RECEIPT_SCHEMA,
        "termux_call_id_payload_conflict",
    ) {
        Ok(s) => s,
        Err(err) => return fail(err),
    };

    let primary_fp = sha256_hex(
        json!([
            "termux-shell",
            script,
            args.get("cwd"),
            args.get("timeout_ms")
        ])
        .to_string()
        .as_bytes(),
    );

    let receipt_path = match store.claim(call_id, &primary_fp, &[&primary_fp]) {
        ReceiptClaim::Acquired(p) => p,
        ReceiptClaim::Existing(v) => return v,
        ReceiptClaim::Rejected(err) => return fail(err),
    };

    let mut outcome = run_shell_command(script, &cwd, timeout_ms, shell);
    outcome["replay_safe"] = json!(true);
    if let Err(err) = store.commit(&receipt_path, &primary_fp, &outcome) {
        outcome["receipt_error"] = json!(err);
        outcome["replay_safe"] = json!(false);
    } else {
        store.prune_settled(512);
    }
    outcome
}

struct TermuxShellServer {
    receipt_root: Option<PathBuf>,
    shell_path: Option<PathBuf>,
}

impl McpHandler for TermuxShellServer {
    fn server_name(&self) -> &str {
        "TermuxShell"
    }

    fn server_version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![tool_def()]
    }

    fn call_tool(&mut self, name: &str, arguments: &Value) -> (Value, bool) {
        let result = if name != "termux_shell" {
            fail("termux_shell_tool_unknown")
        } else {
            let root_res = match &self.receipt_root {
                Some(p) => Ok(p.clone()),
                None => default_receipt_root(),
            };
            let shell_res = match &self.shell_path {
                Some(p) => Ok(p.clone()),
                None => default_shell_path(),
            };
            match (root_res, shell_res) {
                (Ok(root), Ok(shell)) => execute_with(&root, &shell, arguments),
                (Err(e), _) | (_, Err(e)) => fail(e),
            }
        };
        let is_error = matches!(
            result["state"].as_str(),
            Some("rejected" | "delivery_unknown" | "spawn_failed" | "timed_out")
        );
        (result, is_error)
    }
}

fn print_help() {
    println!(
        "termux-shell-mcp {} — independent @TermuxShell MCP stdio worker\n\n\
         Usage:\n\
         \x20 termux-shell-mcp                   Run MCP stdio JSON-RPC 2.0 server\n\
         \x20 termux-shell-mcp --describe-tools  Print static tools/list JSON and exit\n\
         \x20 termux-shell-mcp --version         Print version and exit\n\
         \x20 termux-shell-mcp --help            Show this help",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("-h" | "--help") if args.len() == 1 => {
            print_help();
            return;
        }
        Some("-V" | "--version") if args.len() == 1 => {
            println!("termux-shell-mcp {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--describe-tools") if args.len() == 1 => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({ "tools": [tool_def().to_mcp_json()] }))
                    .unwrap()
            );
            return;
        }
        None => {}
        Some(other) => {
            eprintln!("termux-shell-mcp: unknown argument '{other}' (try --help)");
            std::process::exit(2);
        }
    }

    let mut server = TermuxShellServer {
        receipt_root: None,
        shell_path: None,
    };
    if let Err(err) = serve_stdio(&mut server) {
        eprintln!("termux-shell-mcp: stdio loop error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_kit::{handle_message, serve_io};
    use std::fs::{self, DirBuilder};
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(1);

    fn fixture() -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "termux-shell-mcp-test-{}-{}",
            std::process::id(),
            TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&dir).unwrap();
        dir
    }

    fn sh() -> PathBuf {
        default_shell_path().unwrap()
    }

    #[test]
    fn direct_host_shell_executes_and_exact_retry_never_repeats_effect() {
        let root = fixture();
        let marker = root.join("effect");
        let script = format!(
            "printf 'HOST_MCP_OK\\n'; printf x >> '{}'",
            marker.display()
        );
        let args = json!({"call_id": "same-effect", "command": script, "cwd": root});

        let first = execute_with(&root, &sh(), &args);
        assert_eq!(first["exit_code"], 0);
        assert_eq!(first["stdout"], "HOST_MCP_OK\n");
        assert_eq!(first["replay_safe"], true);

        let replay = execute_with(&root, &sh(), &args);
        assert_eq!(replay["replayed_from_receipt"], true);
        assert_eq!(replay["stdout"], "HOST_MCP_OK\n");
        assert_eq!(fs::read(&marker).unwrap(), b"x");

        let mut changed = args;
        changed["command"] = json!("printf unwanted >> effect");
        assert_eq!(
            execute_with(&root, &sh(), &changed)["error"],
            "termux_call_id_payload_conflict"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn timeout_terminates_owned_private_process_group_and_preserves_partial_output() {
        let root = fixture();
        let pid_file = root.join("child.pid");
        let script = format!(
            "printf 'BEFORE_TIMEOUT\\n'; (sleep 30) & echo $! > '{}'; wait",
            pid_file.display()
        );
        let res = execute_with(
            &root,
            &sh(),
            &json!({"call_id": "to-1", "command": script, "cwd": root, "timeout_ms": 1000}),
        );
        assert_eq!(res["state"], "timed_out");
        assert_eq!(res["timed_out"], true);
        assert!(res["stdout"].as_str().unwrap().contains("BEFORE_TIMEOUT"));
        let bg_pid: i32 = fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        thread::sleep(Duration::from_millis(80));
        let stat = fs::read_to_string(format!("/proc/{bg_pid}/stat"));
        assert!(
            stat.is_err()
                || stat
                    .unwrap()
                    .rsplit_once(')')
                    .unwrap()
                    .1
                    .trim_start()
                    .starts_with('Z')
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn output_truncation_and_corrupt_receipt_handled_fail_closed() {
        let root = fixture();
        let res = execute_with(
            &root,
            &sh(),
            &json!({"call_id": "bounded", "command": "head -c 70000 /dev/zero", "cwd": root}),
        );
        assert_eq!(res["exit_code"], 0);
        assert_eq!(res["stdout"].as_str().unwrap().len(), MAX_OUTPUT_BYTES);
        assert_eq!(res["stdout_truncated"], true);

        // Corrupt receipt -> delivery_unknown
        let bad_path = root.join(format!("{}.json", sha256_hex(b"corrupt")));
        fs::write(&bad_path, b"{\"schema\":\"wrong\",\"state\":\"settled\"}").unwrap();
        fs::set_permissions(&bad_path, fs::Permissions::from_mode(0o600)).unwrap();
        let corrupt = execute_with(
            &root,
            &sh(),
            &json!({"call_id": "corrupt", "command": "echo nope", "cwd": root}),
        );
        assert_eq!(corrupt["state"], "delivery_unknown");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn mcp_stdio_exposes_exactly_one_tool_with_uniform_annotations() {
        let root = fixture();
        let mut srv = TermuxShellServer {
            receipt_root: Some(root.clone()),
            shell_path: Some(sh()),
        };
        let list_resp = handle_message(
            &mut srv,
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
        )
        .unwrap();
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "termux_shell");
        assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
        assert_eq!(tools[0]["annotations"]["destructiveHint"], false);
        assert_eq!(tools[0]["annotations"]["openWorldHint"], false);
        assert_eq!(tools[0]["annotations"]["idempotentHint"], true);

        let input = format!(
            "{}\n{}\n",
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"termux_shell","arguments":{"call_id":"stdio-1","command":"printf ok","cwd":root}}})
        );
        let mut out = Vec::new();
        serve_io(&mut srv, input.as_bytes(), &mut out).unwrap();
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1]["result"]["isError"], false);
        let payload: Value =
            serde_json::from_str(lines[1]["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(payload["stdout"], "ok");
        let _ = fs::remove_dir_all(&root);
    }
}
