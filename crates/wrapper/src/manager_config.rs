//! Bounded optional configuration exchange with the admitted Manager.
//! Core knows the envelope, not Manager policy records, hooks or profile registry.
use super::{validated_core_entrypoint, ManagerArtifact, MANAGER_ARTIFACT_PROBE_ENV};
use super::{MANAGER_CORE_API, MANAGER_CORE_API_ENV, MANAGER_CORE_ENTRYPOINT_ENV};
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const HEADER: &[u8] = b"codex-manager-runtime-config-v1\n";
// Leave room for Core defaults within the existing 16-KiB server config bound.
const LIMIT: usize = 8 * 1024;
const DEADLINE: Duration = Duration::from_secs(2);

pub(super) fn optional(manager: ManagerArtifact<'_, '_>, config: &Path) -> Vec<u8> {
    let ManagerArtifact::Available(selection) = manager else {
        return Vec::new();
    };
    match query(Path::new(selection.program_path), config, DEADLINE) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("codex: optional Manager configuration unavailable; continuing with Termux defaults");
            Vec::new()
        }
    }
}

fn invalid() -> io::Error {
    io::Error::other("invalid Manager configuration response")
}

fn fragment(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.len() > LIMIT || bytes.contains(&0) || std::str::from_utf8(bytes).is_err() {
        return Err(invalid());
    }
    bytes
        .strip_prefix(HEADER)
        .map(Vec::from)
        .ok_or_else(invalid)
}

fn query(program: &Path, config: &Path, deadline: Duration) -> io::Result<Vec<u8>> {
    let bytes = exchange(
        program,
        &[
            std::ffi::OsStr::new("__prepare-runtime-v1"),
            config.as_os_str(),
        ],
        LIMIT,
        deadline,
    )?;
    fragment(&bytes)
}

/// Bounded admitted-component exchange. No request may inherit workload input.
pub(super) fn exchange(
    program: &Path,
    args: &[&std::ffi::OsStr],
    limit: usize,
    deadline: Duration,
) -> io::Result<Vec<u8>> {
    let (mut reader, writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let mut child = Command::new(program)
        .args(args)
        .env(MANAGER_CORE_API_ENV, MANAGER_CORE_API)
        .env(MANAGER_CORE_ENTRYPOINT_ENV, validated_core_entrypoint()?)
        .env_remove(MANAGER_ARTIFACT_PROBE_ENV)
        .env_remove("LD_PRELOAD")
        .env_remove("LD_LIBRARY_PATH")
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    let result = (|| {
        let mut bytes = Vec::new();
        let mut eof = false;
        loop {
            let mut buffer = [0; 4096];
            match reader.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(n) => {
                    bytes.extend_from_slice(&buffer[..n]);
                    if bytes.len() > limit {
                        return Err(invalid());
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(invalid());
                }
                if eof {
                    return Ok(bytes);
                }
            }
            if started.elapsed() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Manager configuration timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    })();
    // Reap our exact child on every path. The endpoint itself never starts jobs.
    let _ = child.kill();
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn rejects_unknown_envelopes_and_unsafe_bytes_without_parsing_policy() {
        assert_eq!(fragment(HEADER).unwrap(), b"");
        let mut bytes = HEADER.to_vec();
        bytes.extend_from_slice(b"# any upstream configuration\n");
        assert_eq!(fragment(&bytes).unwrap(), b"# any upstream configuration\n");
        for invalid in [
            b"v2\n".to_vec(),
            [HEADER, b"\0"].concat(),
            [HEADER, b"\xff"].concat(),
            vec![b'x'; LIMIT + 1],
        ] {
            assert!(fragment(&invalid).is_err());
        }
    }

    #[test]
    fn real_child_exchange_is_bounded_and_preserves_parent_state() {
        let root = crate::tests::temp_root("manager-config");
        let shell = crate::tests::resolve_test_tool("sh").unwrap();
        let program = root.join("manager");
        let config = root.join("config");
        for (body, valid) in [
            (
                "printf 'codex-manager-runtime-config-v1\\n# optional\\n'",
                true,
            ),
            ("printf 'codex-manager-runtime-config-v1\\n'; exit 2", false),
            ("printf 'old-api\\n'", false),
            (
                "while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done",
                false,
            ),
            ("exec sleep 5", false),
        ] {
            std::fs::write(&program, format!("#!{}\n{body}\n", shell.to_string_lossy())).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
            let started = Instant::now();
            let response = query(&program, &config, Duration::from_millis(150));
            assert_eq!(response.is_ok(), valid, "{body}: {response:?}");
            assert!(started.elapsed() < Duration::from_secs(2));
        }
        assert!(!config.exists());
        crate::tests::remove_temp_root(root);
    }
}
