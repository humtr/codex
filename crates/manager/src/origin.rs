//! Existing Termux terminal references. Names and tmux IDs are not Android addresses.
use super::*;
use std::os::unix::fs::{FileTypeExt, MetadataExt};

const SCHEMA: &str = "termux-terminal-origin-v1";
pub(super) const OPTION: &str = "@termux_origin_v1";
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Origin {
    pub handle: String,
    pub pid: u32,
    pub start: u64,
}
impl Origin {
    pub(super) fn parse(value: &str) -> Option<Self> {
        if value.len() > 1024 {
            return None;
        }
        let value: serde_json::Value = serde_json::from_str(value).ok()?;
        let fields = value.as_object()?;
        if fields.len() != 3 {
            return None;
        }
        let handle = value.get("handle")?.as_str()?.to_owned();
        let pid = u32::try_from(value.get("pid")?.as_u64()?).ok()?;
        let start = value.get("start")?.as_u64()?;
        if !canonical_session_id(&handle) || pid == 0 || start == 0 {
            return None;
        }
        Some(Self { handle, pid, start })
    }
    pub(super) fn record(&self) -> String {
        serde_json::json!({"handle":self.handle,"pid":self.pid,"start":self.start}).to_string()
    }
}

fn request(operation: &str, origin: Option<&Origin>) -> Option<Origin> {
    let prefix = PathBuf::from(std::env::var_os("PREFIX")?);
    let socket = prefix.parent()?.join("apps/com.termux/termux-am/am.sock");
    let metadata = fs::symlink_metadata(socket).ok()?;
    if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::getuid() } {
        return None;
    }
    let mut request = format!("termux-terminal-v1 {operation}");
    if let Some(origin) = origin {
        request.push_str(&format!(
            " {} {} {}",
            origin.handle, origin.pid, origin.start
        ));
    }
    let mut child = Command::new(prefix.join("bin/termux-am-socket"))
        .arg(request)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().ok()?;
                if !status.success() || !output.stderr.is_empty() || output.stdout.len() > 4096 {
                    return None;
                }
                let response: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
                if response.get("schema")?.as_str()? != SCHEMA
                    || response.get("result")?.as_str()? != "ok"
                {
                    return None;
                }
                let found = Origin::parse(&response.get("origin")?.to_string())?;
                return if origin.is_none_or(|expected| expected == &found) {
                    Some(found)
                } else {
                    None
                };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

pub(super) fn resolve() -> Option<Origin> {
    request("origin", None)
}
pub(super) fn validate(origin: &Origin) -> bool {
    request("validate", Some(origin)).is_some()
}
pub(super) fn focus(origin: &Origin) -> bool {
    request("focus", Some(origin)).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_reference_has_three_separate_live_fields() {
        let value = Origin {
            handle: "01a0fe94-2d11-79f3-8f5b-9c1958465dc2".into(),
            pid: 42,
            start: 901,
        };
        assert_eq!(Origin::parse(&value.record()), Some(value));
        for input in ["{}", "{\"handle\":\"codex-12345678\",\"pid\":42,\"start\":901}", "{\"handle\":\"01a0fe94-2d11-79f3-8f5b-9c1958465dc2\",\"pid\":0,\"start\":901}", "{\"handle\":\"01a0fe94-2d11-79f3-8f5b-9c1958465dc2\",\"pid\":42,\"start\":0}", "{\"handle\":\"01a0fe94-2d11-79f3-8f5b-9c1958465dc2\",\"pid\":42,\"start\":901,\"extra\":true}"] { assert!(Origin::parse(input).is_none()); }
    }
}
