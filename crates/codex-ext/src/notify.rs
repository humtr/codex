use crate::paths::{atomic_write_private, require_owned_regular_file, ExtPaths};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const STOP_NOTIFY_LINE: &str = r#"notify = ["codex-ext", "notify", "emit", "Stop"]"#;
const MAX_STDIN_PAYLOAD: u64 = 64 * 1024;
const KNOWN_HOOK_EVENTS: &[&str] = &[
    "Stop",
    "PermissionRequest",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "SessionStart",
    "SessionEnd",
    "SubagentStart",
    "SubagentStop",
    "Interrupt",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyConfig {
    pub enabled: bool,
    pub backend: String,
    pub events: Vec<String>,
    pub group: String,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: "notification".to_owned(),
            events: vec!["Stop".to_owned()],
            group: "codex".to_owned(),
        }
    }
}

pub fn load_notify_config(paths: &ExtPaths) -> Result<NotifyConfig, String> {
    if fs::symlink_metadata(&paths.notify_config_file).is_err() {
        return Ok(NotifyConfig::default());
    }
    require_owned_regular_file(&paths.notify_config_file, "notify config")?;
    let raw = fs::read_to_string(&paths.notify_config_file)
        .map_err(|e| format!("cannot read notify config: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid notify config JSON: {e}"))
}

pub fn save_notify_config(paths: &ExtPaths, cfg: &NotifyConfig) -> Result<(), String> {
    let body = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("failed to serialize notify config: {e}"))?
        + "\n";
    atomic_write_private(&paths.notify_config_file, body.as_bytes(), 0o600)
}

pub fn validate_backend(backend: &str) -> Result<(), String> {
    if matches!(backend, "notification" | "toast" | "both") {
        Ok(())
    } else {
        Err(format!(
            "invalid notify backend '{backend}'; expected notification, toast, or both"
        ))
    }
}

pub fn validate_event_name(event: &str) -> Result<(), String> {
    if KNOWN_HOOK_EVENTS.contains(&event) {
        Ok(())
    } else {
        Err(format!(
            "unsupported notify event '{event}'; supported: {}",
            KNOWN_HOOK_EVENTS.join(", ")
        ))
    }
}

/// Inspect whether `~/.codex/config.toml` has top-level `codex-ext` `notify = [...]`
/// (supporting both single-line and multi-line TOML array blocks).
pub fn config_toml_has_stop_notify(config_path: &Path) -> Result<bool, String> {
    if fs::symlink_metadata(config_path).is_err() {
        return Ok(false);
    }
    require_owned_regular_file(config_path, "config.toml")?;
    let raw = fs::read_to_string(config_path)
        .map_err(|e| format!("cannot read {}: {e}", config_path.display()))?;
    let lines: Vec<&str> = raw.lines().collect();
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.starts_with('[') {
            break;
        }
        if is_top_level_notify_key(trimmed) {
            let mut block = vec![lines[idx]];
            let mut depth = toml_bracket_delta(lines[idx]);
            while depth > 0 && idx + 1 < lines.len() {
                idx += 1;
                block.push(lines[idx]);
                depth += toml_bracket_delta(lines[idx]);
            }
            if block.join("\n").contains("codex-ext") {
                return Ok(true);
            }
        }
        idx += 1;
    }
    Ok(false)
}

fn is_top_level_notify_key(trimmed: &str) -> bool {
    let Some(rest) = trimmed.strip_prefix("notify") else {
        return false;
    };
    let rest = rest.trim_start();
    rest.starts_with('=')
}

/// Count bracket balance change (`[` vs `]`) on a TOML line outside strings and comments.
fn toml_bracket_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut delta = 0i32;
    let mut in_double = false;
    let mut in_single = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_double {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_double = false;
            }
        } else if in_single {
            if b == b'\'' {
                in_single = false;
            }
        } else {
            match b {
                b'#' => break,
                b'"' => in_double = true,
                b'\'' => in_single = true,
                b'[' => delta += 1,
                b']' => delta -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    delta
}

/// Atomically set or remove top-level `notify = ["codex-ext", "notify", "emit", "Stop"]`
/// in `~/.codex/config.toml` while preserving all comments, formatting, and tables.
/// Correctly handles both single-line and multi-line TOML `notify = [...]` arrays.
pub fn sync_config_toml_stop_notify(config_path: &Path, enable_stop: bool) -> Result<(), String> {
    let existing = if fs::symlink_metadata(config_path).is_ok() {
        require_owned_regular_file(config_path, "config.toml")?;
        fs::read_to_string(config_path)
            .map_err(|e| format!("cannot read {}: {e}", config_path.display()))?
    } else {
        String::new()
    };

    let lines: Vec<&str> = existing.lines().collect();
    let mut out_lines: Vec<String> = Vec::new();
    let mut in_top_level = true;
    let mut inserted = false;
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim();
        if in_top_level && trimmed.starts_with('[') {
            if enable_stop && !inserted {
                out_lines.push(STOP_NOTIFY_LINE.to_owned());
                inserted = true;
            }
            in_top_level = false;
            out_lines.push(line.to_owned());
            idx += 1;
            continue;
        }
        if in_top_level && is_top_level_notify_key(trimmed) {
            // Capture the full value block (single-line or multi-line array).
            let mut block = vec![line];
            let mut depth = toml_bracket_delta(line);
            while depth > 0 && idx + 1 < lines.len() {
                idx += 1;
                block.push(lines[idx]);
                depth += toml_bracket_delta(lines[idx]);
            }
            let block_text = block.join("\n");
            if enable_stop {
                if !inserted {
                    out_lines.push(STOP_NOTIFY_LINE.to_owned());
                    inserted = true;
                }
            } else if !block_text.contains("codex-ext") {
                // Preserve non-codex-ext user notify command when disabling.
                for b_line in block {
                    out_lines.push(b_line.to_owned());
                }
            }
            idx += 1;
            continue;
        }
        out_lines.push(line.to_owned());
        idx += 1;
    }

    if enable_stop && !inserted {
        out_lines.push(STOP_NOTIFY_LINE.to_owned());
    }

    let mut rendered = out_lines.join("\n");
    if !rendered.is_empty() {
        rendered.push('\n');
    }
    if rendered == existing {
        return Ok(());
    }
    atomic_write_private(config_path, rendered.as_bytes(), 0o600)
}

fn is_codex_ext_hook_group(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|arr| {
            !arr.is_empty()
                && arr.iter().all(|h| {
                    h.get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|c| c.starts_with("codex-ext notify emit "))
                })
        })
}

/// Synchronize non-`Stop` hook events into `~/.codex/hooks.json`, preserving any
/// user-defined hooks outside `codex-ext`. Returns `true` if `hooks.json` has active entries.
pub fn sync_hooks_json(hooks_path: &Path, extra_events: &[String]) -> Result<bool, String> {
    let mut root: Value = if fs::symlink_metadata(hooks_path).is_ok() {
        require_owned_regular_file(hooks_path, "hooks.json")?;
        let raw = fs::read_to_string(hooks_path)
            .map_err(|e| format!("cannot read {}: {e}", hooks_path.display()))?;
        if raw.trim().is_empty() {
            json!({"hooks": {}})
        } else {
            serde_json::from_str(&raw)
                .map_err(|e| format!("cannot parse {}: {e}", hooks_path.display()))?
        }
    } else if extra_events.is_empty() {
        return Ok(false);
    } else {
        json!({"hooks": {}})
    };

    let hooks_obj = root
        .as_object_mut()
        .ok_or_else(|| "hooks.json root must be a JSON object".to_owned())?
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "hooks.json 'hooks' field must be an object".to_owned())?;

    // Strip existing codex-ext managed hook groups while preserving user hooks.
    let mut empty_keys = Vec::new();
    for (k, v) in hooks_obj.iter_mut() {
        if let Some(arr) = v.as_array_mut() {
            arr.retain(|g| !is_codex_ext_hook_group(g));
            if arr.is_empty() {
                empty_keys.push(k.clone());
            }
        }
    }
    for k in empty_keys {
        hooks_obj.remove(&k);
    }

    for ev in extra_events {
        let entry = hooks_obj
            .entry(ev.clone())
            .or_insert_with(|| Value::Array(Vec::new()));
        let arr = entry
            .as_array_mut()
            .ok_or_else(|| format!("hooks.json event '{ev}' is not an array"))?;
        let mut group = Map::new();
        if matches!(
            ev.as_str(),
            "PermissionRequest" | "PreToolUse" | "PostToolUse"
        ) {
            group.insert("matcher".to_owned(), Value::String("*".to_owned()));
        }
        group.insert(
            "hooks".to_owned(),
            json!([{
                "type": "command",
                "command": format!("codex-ext notify emit {ev}")
            }]),
        );
        arr.push(Value::Object(group));
    }

    let has_any = !hooks_obj.is_empty();
    let rendered = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("failed to serialize hooks.json: {e}"))?
        + "\n";
    atomic_write_private(hooks_path, rendered.as_bytes(), 0o600)?;
    Ok(has_any)
}

pub fn list_configured_hooks_json_events(hooks_path: &Path) -> Result<Vec<String>, String> {
    if fs::symlink_metadata(hooks_path).is_err() {
        return Ok(Vec::new());
    }
    require_owned_regular_file(hooks_path, "hooks.json")?;
    let raw = fs::read_to_string(hooks_path)
        .map_err(|e| format!("cannot read {}: {e}", hooks_path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let val: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("cannot parse {}: {e}", hooks_path.display()))?;
    let mut events = Vec::new();
    if let Some(obj) = val.get("hooks").and_then(Value::as_object) {
        for (k, v) in obj {
            if v.as_array()
                .is_some_and(|arr| arr.iter().any(is_codex_ext_hook_group))
            {
                events.push(k.clone());
            }
        }
    }
    events.sort();
    Ok(events)
}

fn sanitize_excerpt(raw: &str, max_chars: usize) -> String {
    let collapsed: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = collapsed.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

pub fn format_notification(event: &str, payload: Option<&Value>) -> (String, String) {
    let cwd_base = payload
        .and_then(|v| v.get("cwd"))
        .and_then(Value::as_str)
        .and_then(|p| Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty());

    let title = match cwd_base {
        Some(dir) => format!("Codex ({dir}) — {event}"),
        None => format!("Codex — {event}"),
    };

    let detail = payload.and_then(|v| {
        v.get("last-assistant-message")
            .or_else(|| v.get("last_assistant_message"))
            .or_else(|| v.get("message"))
            .or_else(|| v.get("reason"))
            .or_else(|| v.get("tool_name"))
            .and_then(Value::as_str)
    });

    let body = match detail {
        Some(text) if !text.trim().is_empty() => sanitize_excerpt(text, 160),
        _ => format!("Event: {event}"),
    };
    (title, body)
}

fn run_bounded(bin: &Path, args: &[&str], timeout: Duration) -> Result<(), String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", bin.display()))?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                return Err(format!("{} exited with {status}", bin.display()));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{} timed out", bin.display()));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(format!("wait failed for {}: {err}", bin.display())),
        }
    }
}

pub fn deliver_notification(
    paths: &ExtPaths,
    backend: &str,
    group: &str,
    event: &str,
    title: &str,
    body: &str,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        println!(
            "{}",
            json!({
                "dry_run": true,
                "backend": backend,
                "group": group,
                "event": event,
                "title": title,
                "body": body
            })
        );
        return Ok(());
    }
    let timeout = Duration::from_secs(4);
    let notif_id = format!("codex-ext-{event}");
    if matches!(backend, "notification" | "both") {
        let bin = paths.prefix.join("bin/termux-notification");
        run_bounded(
            &bin,
            &[
                "--id",
                &notif_id,
                "--group",
                group,
                "--title",
                title,
                "--content",
                body,
            ],
            timeout,
        )?;
    }
    if matches!(backend, "toast" | "both") {
        let bin = paths.prefix.join("bin/termux-toast");
        let toast_msg = format!("{title}: {body}");
        run_bounded(&bin, &["-s", &toast_msg], timeout)?;
    }
    Ok(())
}

/// Optional Stop-hook signal. The relay returns a receipt asynchronously.
pub fn nudge_orchestration_relay(paths: &ExtPaths) {
    let Some(home) = paths.shared_home.parent() else {
        return;
    };
    let dir = home.join(".local/state/codex/services/termux-orch-relay");
    let Some(url) = read_wake_file(&dir.join("health.url")) else {
        return;
    };
    let Some(addr) = url
        .trim()
        .strip_prefix("http://")
        .and_then(|s| s.parse::<SocketAddr>().ok())
    else {
        return;
    };
    if !addr.ip().is_loopback() || addr.port() == 0 {
        return;
    }
    let Some(token) = read_wake_file(&dir.join("wake.token")) else {
        return;
    };
    let token = token.trim();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return;
    }
    let timeout = Duration::from_millis(300);
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return;
    };
    if stream.set_write_timeout(Some(timeout)).is_ok() {
        let _ = write!(
            stream,
            "POST /wake HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {token}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
    }
}

fn read_wake_file(path: &Path) -> Option<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.uid() != crate::paths::current_uid() || meta.mode() & 0o077 != 0 {
        return None;
    }
    let mut contents = String::new();
    file.take(257).read_to_string(&mut contents).ok()?;
    (contents.len() <= 256).then_some(contents)
}

pub fn read_event_payload(arg_payload: Option<&str>) -> Option<Value> {
    if let Some(raw) = arg_payload {
        if let Ok(v) = serde_json::from_str::<Value>(raw) {
            return Some(v);
        }
    }
    if !io::stdin().is_terminal() {
        let mut buf = String::new();
        if io::stdin()
            .take(MAX_STDIN_PAYLOAD)
            .read_to_string(&mut buf)
            .is_ok()
        {
            if let Ok(v) = serde_json::from_str::<Value>(&buf) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::now_millis;

    #[test]
    fn config_toml_preserves_comments_and_tables() {
        let dir = std::env::temp_dir().join(format!(
            "cdx-notify-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.toml");
        let original =
            "# user header\nmodel = \"gpt-5.4\"\n\n[tui]\n# keep comment\nfullscreen = true\n";
        fs::write(&cfg, original).unwrap();

        sync_config_toml_stop_notify(&cfg, true).unwrap();
        let after_on = fs::read_to_string(&cfg).unwrap();
        assert!(after_on.contains("# user header\n"));
        assert!(after_on.contains(STOP_NOTIFY_LINE));
        assert!(after_on.contains("[tui]\n# keep comment\nfullscreen = true\n"));
        assert!(config_toml_has_stop_notify(&cfg).unwrap());

        // Idempotent re-enable
        sync_config_toml_stop_notify(&cfg, true).unwrap();
        assert_eq!(fs::read_to_string(&cfg).unwrap(), after_on);

        // Disable removes only our notify line
        sync_config_toml_stop_notify(&cfg, false).unwrap();
        let after_off = fs::read_to_string(&cfg).unwrap();
        assert!(!after_off.contains(STOP_NOTIFY_LINE));
        assert!(after_off.contains("# user header\nmodel = \"gpt-5.4\"\n"));
        assert!(after_off.contains("[tui]\n# keep comment\nfullscreen = true\n"));

        // Multiline existing notify array is cleanly replaced without leaving orphan lines
        let multi = "model = \"gpt-5.4\"\nnotify = [\n  \"old-tool\",\n  \"--arg\" # comment [ignored]\n]\n\n[tui]\nfullscreen = true\n";
        fs::write(&cfg, multi).unwrap();
        sync_config_toml_stop_notify(&cfg, true).unwrap();
        let after_multi = fs::read_to_string(&cfg).unwrap();
        assert!(!after_multi.contains("old-tool"));
        assert!(!after_multi.contains("--arg"));
        assert!(after_multi.contains(STOP_NOTIFY_LINE));
        assert!(after_multi.contains("[tui]\nfullscreen = true\n"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hooks_json_preserves_user_hooks_and_manages_codex_ext_entries() {
        let dir =
            std::env::temp_dir().join(format!("cdx-hooks-{}-{}", std::process::id(), now_millis()));
        fs::create_dir_all(&dir).unwrap();
        let hooks = dir.join("hooks.json");
        fs::write(
            &hooks,
            serde_json::to_string_pretty(&json!({
                "hooks": {
                    "PermissionRequest": [{
                        "matcher": "bash",
                        "hooks": [{"type": "command", "command": "my-custom-hook"}]
                    }]
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let active = sync_hooks_json(
            &hooks,
            &["PermissionRequest".to_owned(), "SubagentStop".to_owned()],
        )
        .unwrap();
        assert!(active);
        let events = list_configured_hooks_json_events(&hooks).unwrap();
        assert_eq!(events, vec!["PermissionRequest", "SubagentStop"]);

        // Disable codex-ext hooks: user hook MUST remain!
        let still_active = sync_hooks_json(&hooks, &[]).unwrap();
        assert!(still_active);
        let parsed: Value = serde_json::from_str(&fs::read_to_string(&hooks).unwrap()).unwrap();
        assert_eq!(
            parsed["hooks"]["PermissionRequest"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            parsed["hooks"]["PermissionRequest"][0]["hooks"][0]["command"],
            "my-custom-hook"
        );
        assert!(parsed["hooks"].get("SubagentStop").is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn format_notification_sanitizes_and_bounds_excerpt() {
        let payload = json!({
            "cwd": "/tmp/my-project",
            "last-assistant-message": "Hello\n\tworld\x1b[31m! ".repeat(20)
        });
        let (title, body) = format_notification("Stop", Some(&payload));
        assert_eq!(title, "Codex (my-project) — Stop");
        assert!(body.chars().count() <= 161);
        assert!(!body.contains('\n'));
        assert!(!body.contains('\x1b'));
    }
}
