//! Native thread identity, independent of user-selected terminal title fields.
use super::{canonical_session_id, Context, ManagerError, ERR_USAGE};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const ERROR: ManagerError = ManagerError::operation("codex termux: terminal binding unavailable");

fn process(pid: u32) -> Result<(u32, u64), ManagerError> {
    let path = PathBuf::from(format!("/proc/{pid}"));
    if fs::metadata(&path).map_err(|_| ERROR)?.uid() != unsafe { libc::getuid() } {
        return Err(ERROR);
    }
    parse_stat(&fs::read_to_string(path.join("stat")).map_err(|_| ERROR)?)
}

fn parse_stat(value: &str) -> Result<(u32, u64), ManagerError> {
    let fields: Vec<_> = value
        .rsplit_once(')')
        .ok_or(ERROR)?
        .1
        .split_whitespace()
        .collect();
    Ok((
        fields.get(1).ok_or(ERROR)?.parse().map_err(|_| ERROR)?,
        fields.get(19).ok_or(ERROR)?.parse().map_err(|_| ERROR)?,
    ))
}

fn native(home: &Path, executable: &Path) -> bool {
    let root = home.join(".local/lib/codex");
    if executable == root.join("profile-tui-preview/direct-selection-v1/native") {
        return true;
    }
    let Ok(relative) = executable.strip_prefix(root.join("core/generations")) else {
        return false;
    };
    let parts: Vec<_> = relative.components().collect();
    (parts.len() == 2 && parts[1].as_os_str() == "runtime")
        || (parts.len() == 3 && parts[1].as_os_str() == "helpers" && parts[2].as_os_str() == "2")
}

fn tmux(socket: &Path, args: &[&str]) -> Result<String, ManagerError> {
    let mut child = Command::new("tmux")
        .arg("-S")
        .arg(socket)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ERROR)?;
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let output = child.wait_with_output().map_err(|_| ERROR)?;
                return String::from_utf8(output.stdout)
                    .map(|v| v.trim().to_owned())
                    .map_err(|_| ERROR);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ERROR);
            }
        }
    }
}

pub(super) fn bind(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let descriptor = args
        .first()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v <= 1048576)
        .ok_or(ERR_USAGE)?;
    if args.len() != 1 {
        return Err(ERR_USAGE);
    }
    let parent = process(std::process::id())?.0;
    let identity = process(parent)?;
    let executable = fs::read_link(format!("/proc/{parent}/exe")).map_err(|_| ERROR)?;
    if !native(&context.home, &executable) {
        return Err(ERROR);
    }
    let slot = PathBuf::from(format!("/proc/{parent}/fd/{descriptor}"));
    let link = fs::read_link(&slot).map_err(|_| ERROR)?;
    let name = link
        .file_name()
        .and_then(|v| v.to_str())
        .and_then(|v| v.strip_suffix(" (deleted)"))
        .ok_or(ERROR)?;
    let nonce = name
        .strip_prefix(&format!(".codex-terminal-thread-v1-{parent}-"))
        .ok_or(ERROR)?;
    if nonce.is_empty()
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(ERROR);
    }
    let slot_metadata = fs::metadata(&slot).map_err(|_| ERROR)?;
    if !slot_metadata.is_file()
        || slot_metadata.uid() != unsafe { libc::getuid() }
        || slot_metadata.mode() & 0o777 != 0o600
        || slot_metadata.len() > 36
    {
        return Err(ERROR);
    }
    let mut content = String::new();
    fs::File::open(&slot)
        .map_err(|_| ERROR)?
        .take(37)
        .read_to_string(&mut content)
        .map_err(|_| ERROR)?;
    if !content.is_empty() && !canonical_session_id(&content) {
        return Err(ERROR);
    }
    let pane = std::env::var("TMUX_PANE").map_err(|_| ERROR)?;
    if !pane
        .strip_prefix('%')
        .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(ERROR);
    }
    let environment = std::env::var("TMUX").map_err(|_| ERROR)?;
    let socket = PathBuf::from(environment.rsplitn(3, ',').nth(2).ok_or(ERROR)?);
    if !socket.is_absolute() {
        return Err(ERROR);
    }
    let metadata = fs::metadata(&socket).map_err(|_| ERROR)?;
    if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::getuid() } {
        return Err(ERROR);
    }
    let row = tmux(
        &socket,
        &[
            "display-message",
            "-p",
            "-t",
            &pane,
            "#{pane_pid}:#{pane_dead}:#{pane_tty}",
        ],
    )?;
    let fields: Vec<_> = row.split(':').collect();
    if fields.len() != 3 {
        return Err(ERROR);
    }
    let (root, dead, tty) = (fields[0], fields[1], fields[2]);
    let root_pid: u32 = root.parse().map_err(|_| ERROR)?;
    let root_identity = process(root_pid)?;
    if dead != "0" {
        return Err(ERROR);
    }
    let device = fs::metadata(tty).map_err(|_| ERROR)?.rdev();
    if !tty.starts_with("/dev/pts/")
        || device == 0
        || fs::metadata(format!("/proc/{root_pid}/fd/0"))
            .map_err(|_| ERROR)?
            .rdev()
            != device
    {
        return Err(ERROR);
    }
    if parent != root_pid {
        if identity.0 != root_pid {
            return Err(ERROR);
        }
        let bytes = fs::read(format!("/proc/{root_pid}/cmdline")).map_err(|_| ERROR)?;
        let argv: Vec<_> = bytes.split(|b| *b == 0).filter(|v| !v.is_empty()).collect();
        let supervisor = context.home.join(".config/ai/lib/ai_run.py");
        if argv.len() != 3 || argv[1] != supervisor.as_os_str().as_encoded_bytes() {
            return Err(ERROR);
        }
        let prefix = std::env::var_os("PREFIX").map(PathBuf::from).ok_or(ERROR)?;
        if fs::read_link(format!("/proc/{root_pid}/exe")).map_err(|_| ERROR)?
            != fs::canonicalize(prefix.join("bin/python3")).map_err(|_| ERROR)?
        {
            return Err(ERROR);
        }
        let plan: serde_json::Value = serde_json::from_slice(argv[2]).map_err(|_| ERROR)?;
        if plan.get("auth_recovery").and_then(|v| v.as_bool()) != Some(true)
            || !plan
                .get("argv")
                .and_then(|v| v.get(0))
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == "codex" || Path::new(v) == prefix.join("bin/codex"))
        {
            return Err(ERROR);
        }
        let native_tty = fs::read_link(format!("/proc/{parent}/fd/0")).map_err(|_| ERROR)?;
        if !native_tty.starts_with("/dev/pts") {
            return Err(ERROR);
        }
    } else if fs::metadata(format!("/proc/{parent}/fd/0"))
        .map_err(|_| ERROR)?
        .rdev()
        != device
    {
        return Err(ERROR);
    }
    let current_socket = fs::metadata(&socket).map_err(|_| ERROR)?;
    if (current_socket.dev(), current_socket.ino()) != (metadata.dev(), metadata.ino())
        || process(parent)? != identity
        || process(root_pid)? != root_identity
    {
        return Err(ERROR);
    }
    let value = format!(
        "{parent}:{}:{root_pid}:{}:{descriptor}",
        identity.1, root_identity.1
    );
    let predicate = format!("#{{&&:#{{==:#{{pane_pid}},{root_pid}}},#{{==:#{{pane_dead}},0}}}}");
    let command = format!(
        "set-option -p -t {pane} @codex_terminal_binding_v1 '{value}' ; display-message -p bound"
    );
    if tmux(
        &socket,
        &["if-shell", "-F", "-t", &pane, &predicate, &command],
    )? != "bound"
    {
        return Err(ERROR);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_process_identity_handles_parentheses_and_bad_fields() {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[1] = "42";
        fields[19] = "1234";
        assert_eq!(
            parse_stat(&format!("7 (name) with ) chars) {}", fields.join(" "))).unwrap(),
            (42, 1234)
        );
        assert!(parse_stat("invalid").is_err());
        fields[19] = "bad";
        assert!(parse_stat(&format!("7 (name) {}", fields.join(" "))).is_err());
    }
    #[test]
    fn terminal_native_paths_are_product_local() {
        let home = Path::new("/owned");
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/core/generations/a/runtime")
        ));
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/profile-tui-preview/direct-selection-v1/native")
        ));
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/core/generations/a/helpers/2")
        ));
        for path in [
            "/owned/runtime",
            "/owned/.local/lib/codex/core/generations/runtime",
            "/owned/.local/lib/codex/core/generations/a/b/runtime",
            "/owned/.local/lib/codex/core/generations/a/helpers/0",
            "/owned/.local/lib/codex/core/generations/a/helpers/1",
            "/owned/.local/lib/codex/core/generations/a/b/helpers/2",
            "/owned/.local/lib/codex/core/generations/a/helpers/2/native",
        ] {
            assert!(!native(home, Path::new(path)));
        }
    }
}
