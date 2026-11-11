//! Host-side MCP shell worker, independent of the Provider service and Native Core.
//! This module runs inside the tunnel's separate mcp-stdio process. Its only
//! durable state is a bounded shell-effect receipt, never model or thread history.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        process::{CommandExt, ExitStatusExt},
    },
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

const FLAG: &str = "TERMUX_MCP_SHELL_ENABLED";
const MAX_COMMAND: usize = 32 * 1024;
const MAX_OUTPUT: usize = 64 * 1024;
const MAX_RECEIPT: u64 = 512 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_TIMEOUT_MS: u64 = 600_000;
static SERIAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn enabled() -> bool {
    std::env::var_os(FLAG).as_deref() == Some(std::ffi::OsStr::new("1"))
}

pub(crate) fn definitions() -> Vec<Value> {
    let mut tool = json!({
        "name":"termux_shell",
        "description":"Independent Termux app-UID shell (not Codex Native). Runs host commands without Codex Core, Provider service or shared Native claims. One MCP tool only: keep a stable call_id and resend the exact same arguments after uncertain delivery to retrieve the original receipt without re-executing. Recovery is a priority, not a restriction on uses authorized by the user.",
        "inputSchema":{"type":"object","properties":{
            "call_id":{"type":"string","minLength":1,"maxLength":160},
            "command":{"type":"string","minLength":1,"maxLength":32768},
            "cwd":{"type":"string","description":"Absolute working directory; defaults to Termux HOME"},
            "timeout_ms":{"type":"integer","minimum":1000,"maximum":600000}
        },"required":["call_id","command"],"additionalProperties":false}
    });
    tool["annotations"] = crate::mcp::tool_annotations("termux_shell");
    vec![tool]
}

pub(crate) fn initialize(row: &Value) -> Value {
    json!({"jsonrpc":"2.0","id":row["id"],"result":{
        "protocolVersion":"2025-03-26",
        "capabilities":{"tools":{"listChanged":true}},
        "serverInfo":{"name":"Termux Bridge (independent host worker)","version":"1.0"}
    }})
}

pub(crate) fn tool_list(row: &Value, upstream: Option<Value>) -> Value {
    let mut list = definitions();
    let mut next_cursor = None;
    if let Some(mut tools) = upstream
        .filter(|v| v.get("error").is_none())
        .and_then(|v| v.get("result").cloned())
    {
        if let Some(array) = tools["tools"].as_array_mut() {
            list.extend(
                array
                    .iter()
                    .filter(|tool| tool["name"] != "termux_shell")
                    .cloned(),
            );
            next_cursor = tools.get("nextCursor").cloned();
        }
    }
    let mut result = json!({"tools":list});
    if let Some(cursor) = next_cursor {
        result["nextCursor"] = cursor;
    }
    json!({"jsonrpc":"2.0","id":row["id"],"result":result})
}

pub(crate) fn is_shell_tool(row: &Value) -> bool {
    row["method"] == "tools/call" && row["params"]["name"] == "termux_shell"
}

fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
fn fail(reason: &'static str) -> Value {
    json!({"state":"rejected","executed":false,"error":reason})
}

fn check_call_id(args: &Value) -> Result<&str, &'static str> {
    match args["call_id"].as_str() {
        Some(v) if !v.is_empty() && v.len() <= 160 && !v.contains('\0') => Ok(v),
        _ => Err("termux_call_id_invalid"),
    }
}
fn journal() -> Result<PathBuf, &'static str> {
    let prefix = std::env::var_os("PREFIX").ok_or("termux_prefix_unavailable")?;
    let path = PathBuf::from(prefix).join("var/tmp/termux-mcp-shell-receipts-v1");
    match DirBuilder::new().mode(0o700).create(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("termux_receipt_directory_unavailable"),
    }
    let meta = fs::symlink_metadata(&path).map_err(|_| "termux_receipt_directory_unavailable")?;
    if !meta.file_type().is_dir()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.permissions().mode() & 0o077 != 0
    {
        return Err("termux_receipt_directory_unsafe");
    }
    Ok(path)
}

fn read_record(path: &Path) -> Result<Option<Value>, &'static str> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("termux_receipt_unavailable"),
    };
    let meta = file.metadata().map_err(|_| "termux_receipt_unavailable")?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() > MAX_RECEIPT
    {
        return Err("termux_receipt_unsafe");
    }
    let mut bytes = Vec::new();
    file.take(MAX_RECEIPT)
        .read_to_end(&mut bytes)
        .map_err(|_| "termux_receipt_unavailable")?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "termux_receipt_incomplete")
}
fn sync_parent(path: &Path) -> Result<(), &'static str> {
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(
            path.parent()
                .ok_or("termux_receipt_directory_unavailable")?,
        )
        .map_err(|_| "termux_receipt_directory_unavailable")?;
    directory
        .sync_all()
        .map_err(|_| "termux_receipt_directory_sync_failed")
}
fn write_intent(path: &Path, fingerprint: &str) -> Result<bool, &'static str> {
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(_) => return Err("termux_receipt_create_failed"),
    };
    let bytes = json!({"schema":"termux-mcp-shell-receipt-v1","fingerprint":fingerprint,
        "state":"started","executed":"unknown"})
    .to_string();
    file.write_all(bytes.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "termux_receipt_intent_write_failed")?;
    sync_parent(path)?;
    Ok(true)
}
fn write_complete(path: &Path, fingerprint: &str, outcome: &Value) -> Result<(), &'static str> {
    let nonce = SERIAL.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let complete = json!({"schema":"termux-mcp-shell-receipt-v1",
        "fingerprint":fingerprint,"state":"settled","outcome":outcome});
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)
        .map_err(|_| "termux_receipt_complete_create_failed")?;
    file.write_all(complete.to_string().as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "termux_receipt_complete_write_failed")?;
    drop(file);
    fs::rename(&tmp, path).map_err(|_| "termux_receipt_complete_rename_failed")?;
    sync_parent(path)
}

fn prior(root: &Path, call_id: &str, fingerprint: Option<&str>) -> Value {
    let path = root.join(format!("{}.json", digest(call_id.as_bytes())));
    match read_record(&path) {
        Ok(Some(record)) => {
            if record["schema"] != "termux-mcp-shell-receipt-v1"
                || !matches!(record["state"].as_str(), Some("started" | "settled"))
                || !record["fingerprint"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
                || (record["state"] == "settled" && !record["outcome"].is_object())
            {
                return json!({"state":"delivery_unknown","executed":"unknown","error":"termux_receipt_invalid"});
            }
            if let Some(fp) = fingerprint {
                if record["fingerprint"] != fp {
                    return fail("termux_call_id_payload_conflict");
                }
            }
            if record["state"] == "settled" {
                let mut result = record["outcome"].clone();
                result["replayed_from_receipt"] = json!(true);
                result
            } else {
                json!({"state":"delivery_unknown","executed":"unknown",
                    "error":"termux_original_effect_unsettled_check_host_before_new_call"})
            }
        }
        Ok(None) => json!({"state":"not_found","executed":false}),
        Err(error) => json!({"state":"delivery_unknown","executed":"unknown","error":error}),
    }
}

struct ProcessGroup(Option<i32>);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            // Only the child's private process group. Never kill an unrelated PID.
            if pid > 0 {
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
        }
    }
}
struct Capture {
    task: tokio::task::JoinHandle<std::io::Result<()>>,
    data: Arc<Mutex<(Vec<u8>, bool)>>,
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn capture<R: AsyncRead + Unpin + Send + 'static>(mut stream: R) -> Capture {
    let data = Arc::new(Mutex::new((Vec::new(), false)));
    let result = data.clone();
    let task = tokio::spawn(async move {
        let mut buffer = [0; 8192];
        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                return Ok(());
            }
            let mut value = result.lock().expect("capture buffer");
            let room = MAX_OUTPUT.saturating_sub(value.0.len());
            value.0.extend_from_slice(&buffer[..room.min(n)]);
            value.1 |= n > room;
        }
    });
    Capture { task, data }
}
async fn captured(mut capture: Capture) -> (Vec<u8>, bool) {
    let incomplete = !matches!(
        tokio::time::timeout(Duration::from_secs(2), &mut capture.task).await,
        Ok(Ok(Ok(())))
    );
    if incomplete {
        capture.task.abort();
    }
    let mut value = capture.data.lock().expect("capture buffer");
    (std::mem::take(&mut value.0), value.1 || incomplete)
}

async fn shell_command(script: &str, cwd: &Path, timeout_ms: u64, shell: &Path) -> Value {
    let mut command = Command::new(shell);
    command
        .arg("-c")
        .arg(script)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // A private group enables cleanup on timeout or caller disconnect. Its
    // lifetime is not the Native writer or a Codex session.
    command.as_std_mut().process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            return json!({"state":"spawn_failed","executed":false,
            "error":"termux_shell_spawn_failed"})
        }
    };
    let pid = child.id().unwrap_or(0) as i32;
    let group = ProcessGroup((pid > 0).then_some(pid));
    let stdout = capture(child.stdout.take().expect("piped stdout"));
    let stderr = capture(child.stderr.take().expect("piped stderr"));
    let since = Instant::now();
    let waited = tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait()).await;
    let (exit_code, signal, timed_out) = match waited {
        Ok(Ok(status)) => (status.code(), status.signal(), false),
        Ok(Err(_)) => {
            drop(group);
            return json!({"state":"delivery_unknown","executed":true,
                "error":"termux_shell_wait_failed"});
        }
        Err(_) => {
            if pid > 0 {
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
            }
            let _ = child.wait().await;
            (None, Some(libc::SIGKILL), true)
        }
    };
    // Reap ordinary background descendants too; explicit detached sessions are
    // separate host processes, never Native work or implicit continuations.
    drop(group);
    let (out, out_truncated) = captured(stdout).await;
    let (err, err_truncated) = captured(stderr).await;
    json!({"state":if timed_out {"timed_out"} else {"exited"},
        "executed":true,"pid":pid,"exit_code":exit_code,"signal":signal,
        "timed_out":timed_out,"duration_ms":since.elapsed().as_millis() as u64,
        "stdout":String::from_utf8_lossy(&out),"stderr":String::from_utf8_lossy(&err),
        "stdout_base64":STANDARD.encode(&out),"stderr_base64":STANDARD.encode(&err),
        "stdout_truncated":out_truncated,"stderr_truncated":err_truncated,
        "cwd":cwd.to_string_lossy()})
}

async fn execute_with(root: &Path, shell: &Path, args: &Value) -> Value {
    if !args.as_object().is_some_and(|v| {
        v.keys()
            .all(|k| matches!(k.as_str(), "call_id" | "command" | "cwd" | "timeout_ms"))
    }) {
        return fail("termux_arguments_invalid");
    }
    let call_id = match check_call_id(args) {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    let script = match args["command"].as_str() {
        Some(v) if !v.trim().is_empty() && v.len() <= MAX_COMMAND && !v.contains('\0') => v,
        _ => return fail("termux_command_invalid"),
    };
    let cwd = match args.get("cwd") {
        Some(Value::String(v)) if Path::new(v).is_absolute() && !v.contains('\0') => {
            PathBuf::from(v)
        }
        None => match std::env::var_os("HOME") {
            Some(v) => PathBuf::from(v),
            None => return fail("termux_home_unavailable"),
        },
        _ => return fail("termux_cwd_invalid"),
    };
    let timeout_ms = match args.get("timeout_ms") {
        None => DEFAULT_TIMEOUT_MS,
        Some(v) => match v.as_u64() {
            Some(ms) if (1000..=MAX_TIMEOUT_MS).contains(&ms) => ms,
            _ => return fail("termux_timeout_invalid"),
        },
    };
    let legacy_fingerprint = digest(
        json!(["termux-shell-v1", script, cwd, timeout_ms])
            .to_string()
            .as_bytes(),
    );
    let fingerprint = digest(
        json!([
            "termux-shell-v2",
            script,
            args.get("cwd"),
            args.get("timeout_ms")
        ])
        .to_string()
        .as_bytes(),
    );
    let replay = || {
        let result = prior(root, call_id, Some(&fingerprint));
        if result["error"] == "termux_call_id_payload_conflict" {
            prior(root, call_id, Some(&legacy_fingerprint))
        } else {
            result
        }
    };
    let path = root.join(format!("{}.json", digest(call_id.as_bytes())));
    if path.exists() {
        return replay();
    }
    match write_intent(&path, &fingerprint) {
        Ok(true) => {}
        Ok(false) => return replay(),
        Err(e) => return fail(e),
    }
    let mut outcome = shell_command(script, &cwd, timeout_ms, shell).await;
    outcome["replay_safe"] = json!(true);
    if let Err(error) = write_complete(&path, &fingerprint, &outcome) {
        outcome["receipt_error"] = json!(error);
        outcome["replay_safe"] = json!(false);
    } else {
        outcome["replay_safe"] = json!(true);
    }
    outcome
}

pub(crate) async fn call(row: &Value) -> Value {
    let args = &row["params"]["arguments"];
    let result = match journal() {
        Ok(root) => {
            if row["params"]["name"] != "termux_shell" {
                fail("termux_shell_tool_unknown")
            } else {
                let shell = std::env::var_os("PREFIX")
                    .map(PathBuf::from)
                    .map(|v| v.join("bin/sh"));
                match shell {
                    Some(path) => execute_with(&root, &path, args).await,
                    None => fail("termux_prefix_unavailable"),
                }
            }
        }
        Err(error) => fail(error),
    };
    let is_error = matches!(
        result["state"].as_str(),
        Some("rejected" | "delivery_unknown" | "spawn_failed" | "timed_out")
    );
    json!({"jsonrpc":"2.0","id":row["id"],"result":{
        "content":[{"type":"text","text":result.to_string()}],
        "isError":is_error
    }})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "termux-mcp-shell-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&dir).unwrap();
        dir
    }
    fn shell() -> PathBuf {
        std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .map(|p| p.join("bin/sh"))
            .filter(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("/bin/sh"))
    }
    #[tokio::test]
    async fn direct_host_shell_executes_and_exact_retry_never_repeats_effect() {
        let root = fixture();
        let marker = root.join("effect");
        let script = format!(
            "printf 'HOST_MCP_OK\\n'; printf x >> '{}'",
            marker.display()
        );
        let args = json!({"call_id":"same-effect","command":script,"cwd":root});
        let first = execute_with(&root, &shell(), &args).await;
        assert_eq!(first["exit_code"], 0);
        assert_eq!(first["stdout"], "HOST_MCP_OK\n");
        assert_eq!(first["replay_safe"], true);
        let replay = execute_with(&root, &shell(), &args).await;
        assert_eq!(replay["replayed_from_receipt"], true);
        assert_eq!(fs::read(&marker).unwrap(), b"x");
        assert_eq!(prior(&root, "same-effect", None)["stdout"], "HOST_MCP_OK\n");
        let mut changed = args;
        changed["command"] = json!("printf unwanted >> effect");
        assert_eq!(
            execute_with(&root, &shell(), &changed).await["error"],
            "termux_call_id_payload_conflict"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn uncertain_intent_is_never_replayed_and_invalid_args_never_start() {
        let root = fixture();
        let path = root.join(format!("{}.json", digest(b"unknown-call")));
        write_intent(&path, "unknown").unwrap();
        assert_eq!(
            prior(&root, "unknown-call", None)["state"],
            "delivery_unknown"
        );
        assert_eq!(
            execute_with(
                &root,
                &shell(),
                &json!({
                    "call_id":"test","command":"echo would-not-run","cwd":"relative"
                })
            )
            .await["error"],
            "termux_cwd_invalid"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn timeout_terminates_only_owned_private_process_group() {
        let root = fixture();
        let result = execute_with(
            &root,
            &shell(),
            &json!({
                "call_id":"timeout","command":"sleep 5","cwd":root,"timeout_ms":1000
            }),
        )
        .await;
        assert_eq!(result["state"], "timed_out");
        assert_eq!(result["timed_out"], true);
        assert_eq!(result["replay_safe"], true);
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn concurrent_duplicate_keeps_original_effect_and_conflicting_arguments_fail() {
        let root = fixture();
        let args = json!({"call_id":"concurrent","command":"printf x >> effect; sleep 0.1; printf DONE","cwd":root});
        let shell = shell();
        let (a, b) = tokio::join!(
            execute_with(&root, &shell, &args),
            execute_with(&root, &shell, &args)
        );
        assert!(a["state"] == "exited" || b["state"] == "exited");
        assert_eq!(fs::read(root.join("effect")).unwrap(), b"x");
        let replay = execute_with(&root, &shell, &args).await;
        assert_eq!(replay["stdout"], "DONE");
        assert_eq!(replay["replay_safe"], true);
        assert_eq!(replay["replayed_from_receipt"], true);
        let mut invalid = args.clone();
        invalid["unknown"] = json!(true);
        assert_eq!(
            execute_with(&root, &shell, &invalid).await["error"],
            "termux_arguments_invalid"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn completed_shell_reaps_background_group_and_preserves_output() {
        let root = fixture();
        let result=execute_with(&root,&shell(),&json!({"call_id":"background","command":"sleep 30 & echo $! > child; printf COMPLETED","cwd":root})).await;
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"], "COMPLETED");
        assert_eq!(result["stdout_truncated"], false);
        let pid = fs::read_to_string(root.join("child")).unwrap();
        let stat = fs::read_to_string(format!("/proc/{}/stat", pid.trim()));
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
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn corrupt_receipt_and_output_limits_preserve_uncertainty_and_original_bytes() {
        let root = fixture();
        let result = execute_with(
            &root,
            &shell(),
            &json!({"call_id":"bounded","command":"head -c 70000 /dev/zero","cwd":root}),
        )
        .await;
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"].as_str().unwrap().len(), MAX_OUTPUT);
        assert_eq!(result["stdout_truncated"], true);
        let path = root.join(format!("{}.json", digest(b"corrupt")));
        fs::write(
            &path,
            b"{\"schema\":\"wrong\",\"state\":\"settled\",\"fingerprint\":\"x\",\"outcome\":{}}",
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(prior(&root, "corrupt", None)["state"], "delivery_unknown");
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn cancelled_call_keeps_uncertain_receipt_and_never_resubmits() {
        let root = fixture();
        let args =
            json!({"call_id":"cancelled","command":"printf x >> effect; sleep 30","cwd":root});
        let (owned_root, owned_args) = (root.clone(), args.clone());
        let task =
            tokio::spawn(async move { execute_with(&owned_root, &shell(), &owned_args).await });
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("effect").exists() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(fs::read(root.join("effect")).unwrap(), b"x");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(
            execute_with(&root, &shell(), &args).await["state"],
            "delivery_unknown"
        );
        assert_eq!(fs::read(root.join("effect")).unwrap(), b"x");
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn exact_raw_arguments_and_legacy_receipts_are_both_preserved() {
        let root = fixture();
        let args = json!({"call_id":"exact","command":"printf exact","cwd":root});
        assert_eq!(
            execute_with(&root, &shell(), &args).await["stdout"],
            "exact"
        );
        let mut changed = args.clone();
        changed["timeout_ms"] = json!(DEFAULT_TIMEOUT_MS);
        assert_eq!(
            execute_with(&root, &shell(), &changed).await["error"],
            "termux_call_id_payload_conflict"
        );
        let legacy = digest(
            json!(["termux-shell-v1", "printf exact", root, DEFAULT_TIMEOUT_MS])
                .to_string()
                .as_bytes(),
        );
        let path = root.join(format!("{}.json", digest(b"legacy")));
        write_intent(&path, &legacy).unwrap();
        write_complete(
            &path,
            &legacy,
            &json!({"state":"exited","executed":true,"stdout":"ORIGINAL"}),
        )
        .unwrap();
        let mut old = args;
        old["call_id"] = json!("legacy");
        assert_eq!(
            execute_with(&root, &shell(), &old).await["stdout"],
            "ORIGINAL"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn only_one_shell_tool_with_the_same_common_annotations() {
        let query = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
        let result = tool_list(
            &query,
            Some(json!({"result":{"tools":[
            {"name":"codex_sessions","inputSchema":{"type":"object"}}]}})),
        );
        let tools = result["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["name"], "termux_shell");
        assert_eq!(tools[1]["name"], "codex_sessions");
        assert_eq!(
            tools[0]["annotations"],
            crate::mcp::tool_annotations("local_execute")
        );
        assert_eq!(
            tools[0]["annotations"],
            crate::mcp::tool_annotations("termux_shell")
        );
        let offline = tool_list(&query, None);
        assert_eq!(offline["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(offline["result"]["tools"][0]["name"], "termux_shell");
        assert!(is_shell_tool(
            &json!({"method":"tools/call","params":{"name":"termux_shell"}})
        ));
        assert!(!is_shell_tool(
            &json!({"method":"tools/call","params":{"name":"termux_shell_receipt"}})
        ));
    }
}
