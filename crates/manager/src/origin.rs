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

// Inconclusive transport failure is not evidence that the native API is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Capability {
    Native,
    Stock,
    Unavailable,
}

pub(super) fn command(request: &str, seconds: u64) -> Option<std::process::Output> {
    let prefix = PathBuf::from(std::env::var_os("PREFIX")?);
    let socket = prefix.parent()?.join("apps/com.termux/termux-am/am.sock");
    let metadata = fs::symlink_metadata(socket).ok()?;
    if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::getuid() } {
        return None;
    }
    let mut child = Command::new(prefix.join("bin/termux-am-socket"))
        .arg(request)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    use std::os::fd::AsRawFd;
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    }
    let mut out = Vec::new();
    let mut err = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        if drain(&mut stdout, &mut out).is_err() || drain(&mut stderr, &mut err).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                drain(&mut stdout, &mut out).ok()?;
                drain(&mut stderr, &mut err).ok()?;
                return Some(std::process::Output {
                    status,
                    stdout: out,
                    stderr: err,
                });
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
fn drain(reader: &mut impl Read, bytes: &mut Vec<u8>) -> io::Result<()> {
    let mut chunk = [0u8; 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() > 16384 {
                    return Err(io::Error::other("AM response exceeds bound"));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
}

pub(super) fn capability() -> Capability {
    let Some(output) = command("termux-terminal-v1 capabilities", 3) else {
        return Capability::Unavailable;
    };
    if std::str::from_utf8(&output.stderr)
        .ok()
        .and_then(|s| s.trim_end().lines().last())
        == Some("Error: unknown command 'termux-terminal-v1'")
    {
        return Capability::Stock;
    }
    let parsed: Option<serde_json::Value> = serde_json::from_slice(&output.stdout).ok();
    if output.status.success()
        && output.stdout.len() <= 4096
        && output.stderr.is_empty()
        && parsed.as_ref().is_some_and(|v| {
            v["schema"] == SCHEMA
                && v["result"] == "ok"
                && v["operations"] == "origin,validate,focus"
        })
    {
        Capability::Native
    } else {
        Capability::Unavailable
    }
}

fn request(operation: &str, origin: Option<&Origin>) -> Option<Origin> {
    let mut request = format!("termux-terminal-v1 {operation}");
    if let Some(origin) = origin {
        request.push_str(&format!(
            " {} {} {}",
            origin.handle, origin.pid, origin.start
        ));
    }
    let output = command(&request, 3)?;
    if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() > 4096 {
        return None;
    }
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    if response.get("schema")?.as_str()? != SCHEMA || response.get("result")?.as_str()? != "ok" {
        return None;
    }
    let found = Origin::parse(&response.get("origin")?.to_string())?;
    origin
        .is_none_or(|expected| expected == &found)
        .then_some(found)
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
