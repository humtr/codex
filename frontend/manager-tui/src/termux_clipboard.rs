//! Android clipboard access through Termux's existing authenticated AM socket.
//! The worker owns delivery; terminal-only fallback remains an unconfirmed request.
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const ORIGIN_SCHEMA: &str = "termux-terminal-origin-v1";
const CLIPBOARD_SCHEMA: &str = "termux-terminal-clipboard-v1";
// The app-side clipboard-v1 contract accepts raw UTF-8, not encoded OSC52 bytes.
const MAX_TEXT_BYTES: usize = 100 * 1024;
const RESPONSE_LIMIT: usize = 16384;

struct Transport(PathBuf);

struct RequestProcess(Child);

impl Drop for RequestProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Transport {
    fn discover() -> Result<Option<Self>, String> {
        let Some(prefix) = std::env::var_os("PREFIX").map(PathBuf::from) else {
            return Ok(None);
        };
        if !prefix.is_absolute() {
            return Err("Termux PREFIX must be absolute".into());
        }
        let program = prefix.join("bin/termux-am-socket");
        if !program.exists() {
            return Ok(None);
        }
        let socket = prefix
            .parent()
            .ok_or("Termux prefix has no application root")?
            .join("apps/com.termux/termux-am/am.sock");
        let metadata = std::fs::symlink_metadata(socket)
            .map_err(|error| format!("Termux clipboard socket unavailable: {error}"))?;
        if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err("Termux clipboard socket has an invalid owner or type".into());
        }
        let transport = Self(program);
        let output = transport.command(&["capabilities"])?;
        if !output.status.success()
            && std::str::from_utf8(&output.stderr)
                .ok()
                .and_then(|text| text.trim_end().lines().last())
                == Some("Error: unknown command 'termux-terminal-v1'")
        {
            return Ok(None);
        }
        let value = response(&output, ORIGIN_SCHEMA, None)?;
        let operations = value["operations"]
            .as_str()
            .ok_or("Termux clipboard capabilities have no operations")?;
        Ok(operations
            .split(',')
            .any(|operation| operation == "clipboard-write-text")
            .then_some(transport))
    }

    fn command(&self, args: &[&str]) -> Result<Output, String> {
        let request =
            shlex::try_join(std::iter::once("termux-terminal-v1").chain(args.iter().copied()))
                .map_err(|error| format!("invalid Termux clipboard argument: {error}"))?;
        let child = Command::new(&self.0)
            .arg(request)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Termux clipboard client could not start: {error}"))?;
        let mut child = RequestProcess(child);
        let mut stdout = child.0.stdout.take().ok_or("missing clipboard stdout")?;
        let mut stderr = child.0.stderr.take().ok_or("missing clipboard stderr")?;
        for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                return Err(format!(
                    "clipboard pipe setup failed: {}",
                    io::Error::last_os_error()
                ));
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut output = Vec::new();
        let mut errors = Vec::new();
        loop {
            drain(&mut stdout, &mut output)?;
            drain(&mut stderr, &mut errors)?;
            if let Some(status) = child.0.try_wait().map_err(|error| error.to_string())? {
                drain(&mut stdout, &mut output)?;
                drain(&mut stderr, &mut errors)?;
                return Ok(Output {
                    status,
                    stdout: output,
                    stderr: errors,
                });
            }
            if Instant::now() >= deadline {
                return Err("Termux clipboard response timed out; write outcome unknown".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn drain(reader: &mut impl Read, bytes: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                if bytes.len() + count > RESPONSE_LIMIT {
                    return Err("Termux clipboard response exceeds its bound".into());
                }
                bytes.extend_from_slice(&buffer[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => (),
            Err(error) => return Err(format!("Termux clipboard response failed: {error}")),
        }
    }
}

fn response(output: &Output, schema: &str, operation: Option<&str>) -> Result<Value, String> {
    if !output.status.success() {
        return Err(format!(
            "Termux clipboard request failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid Termux clipboard response: {error}"))?;
    if value["schema"] != schema
        || value["result"] != "ok"
        || operation.is_some_and(|operation| value["operation"] != operation)
    {
        return Err("Termux clipboard response has an invalid schema, operation or result".into());
    }
    Ok(value)
}

/// None means the installed app has no confirmed-write API; keep upstream fallback.
/// Once an advertised operation is attempted, failure must not issue another write.
pub(crate) fn copy(
    text: &str,
    begin_delivery: impl FnOnce() -> Result<(), String>,
) -> Option<Result<(), String>> {
    let transport = match Transport::discover() {
        Ok(Some(transport)) => transport,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some((|| {
        if text.is_empty() {
            return Err("nothing to copy: the selected content is empty".into());
        }
        if text.len() > MAX_TEXT_BYTES {
            return Err(format!(
                "Android clipboard-v1 payload too large ({} bytes; max {MAX_TEXT_BYTES})",
                text.len()
            ));
        }
        let mut file = tempfile::Builder::new()
            .prefix("codex-clipboard-")
            .tempfile()
            .map_err(|error| format!("cannot prepare clipboard input: {error}"))?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("cannot write clipboard input: {error}"))?;
        let path = file
            .path()
            .to_str()
            .ok_or("clipboard input path is not UTF-8")?;
        begin_delivery()?;
        let output = transport.command(&["clipboard-write-text", path])?;
        let value = response(&output, CLIPBOARD_SCHEMA, Some("clipboard-write-text"))?;
        if value["confirmed"] != true {
            return Err("Android clipboard write was not confirmed".into());
        }
        Ok(())
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    #[test]
    fn android_copy_preserves_payload_and_requires_matching_confirmation() {
        if let Some(root) = std::env::var_os("TERMUX_CLIPBOARD_TEST_ROOT") {
            let root = PathBuf::from(root);
            let delivered = root.join("delivered");
            let response = root.join("response");
            let confirm = serde_json::json!({
                "schema":CLIPBOARD_SCHEMA, "operation":"clipboard-write-text",
                "result":"ok", "confirmed":true,
            });
            std::fs::write(&response, confirm.to_string()).unwrap();
            for text in ["x".repeat(100_000), "한글🙂\n".repeat(9000)] {
                copy(&text, || Ok(())).unwrap().unwrap();
                assert_eq!(std::fs::read(&delivered).unwrap(), text.as_bytes());
            }
            let before = std::fs::read(&delivered).unwrap();
            let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
            assert!(
                copy(&oversized, || panic!("oversized delivery"))
                    .unwrap()
                    .is_err()
            );
            assert!(
                copy("cancelled", || Err("abandoned".into()))
                    .unwrap()
                    .is_err()
            );
            assert_eq!(std::fs::read(&delivered).unwrap(), before);

            for invalid in [
                serde_json::json!({"schema":CLIPBOARD_SCHEMA,"operation":"clipboard-write-text","result":"ok","confirmed":false}),
                serde_json::json!({"schema":"other","operation":"clipboard-write-text","result":"ok","confirmed":true}),
                serde_json::json!({"schema":CLIPBOARD_SCHEMA,"operation":"clipboard-read-image","result":"ok","confirmed":true}),
                serde_json::json!({"schema":CLIPBOARD_SCHEMA,"operation":"clipboard-write-text","result":"write-failed","confirmed":true}),
                serde_json::json!({"schema":CLIPBOARD_SCHEMA,"operation":"clipboard-write-text","result":"ok","confirmed":"true"}),
            ] {
                std::fs::write(&response, invalid.to_string()).unwrap();
                assert!(copy("unconfirmed", || Ok(())).unwrap().is_err());
            }
            std::fs::write(&response, "not JSON").unwrap();
            assert!(copy("unknown", || Ok(())).unwrap().is_err());
            std::fs::write(root.join("operations"), "origin,validate,focus").unwrap();
            assert!(copy("legacy", || panic!("legacy API must use upstream fallback")).is_none());
            return;
        }

        let root = tempfile::Builder::new().prefix("cb-").tempdir().unwrap();
        let prefix = root.path().join("usr");
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        let socket = root.path().join("apps/com.termux/termux-am/am.sock");
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let _socket = UnixListener::bind(socket).unwrap();
        std::fs::write(
            root.path().join("operations"),
            "origin,validate,focus,clipboard-write-text",
        )
        .unwrap();
        let script = root.path().join("client.py");
        std::fs::write(&script, r#"import json, os, pathlib, shlex, sys
root=pathlib.Path(os.environ['TERMUX_CLIPBOARD_TEST_ROOT'])
args=shlex.split(sys.argv[1])
assert args[0]=='termux-terminal-v1'
if args[1]=='capabilities':
 print(json.dumps({'schema':'termux-terminal-origin-v1','result':'ok','operations':(root/'operations').read_text()}))
else:
 assert args[1]=='clipboard-write-text' and len(args)==3
 source=pathlib.Path(args[2])
 assert source.stat().st_mode & 0o777 == 0o600
 (root/'delivered').write_bytes(source.read_bytes())
 print((root/'response').read_text())
"#).unwrap();
        let shell = std::env::var("PREFIX")
            .map(|prefix| format!("{prefix}/bin/sh"))
            .unwrap_or_else(|_| "/bin/sh".into());
        let client = prefix.join("bin/termux-am-socket");
        let command = shlex::try_join(["python3", script.to_str().unwrap()]).unwrap();
        std::fs::write(&client, format!("#!{shell}\nexec {command} \"$@\"\n")).unwrap();
        std::fs::set_permissions(&client, std::fs::Permissions::from_mode(0o700)).unwrap();
        let name = std::thread::current().name().unwrap().to_owned();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &name, "--nocapture"])
            .env("TERMUX_CLIPBOARD_TEST_ROOT", root.path())
            .env("PREFIX", &prefix)
            .env("TMPDIR", root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(std::fs::read_dir(root.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("codex-clipboard-")
        }));
    }
}
