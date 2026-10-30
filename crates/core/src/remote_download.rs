//! Bounded HTTPS acquisition and resumable, untrusted payload storage. Signed
//! admission owns identities; this module never selects or activates a release.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(target_arch = "aarch64")]
const NOFOLLOW: i32 = 0x8000;
#[cfg(not(target_arch = "aarch64"))]
const NOFOLLOW: i32 = 0x20000;

extern "C" {
    fn geteuid() -> u32;
}

const HTTP_STATUS: &str = "%{stderr}CODEX_HTTP_STATUS=%{response_code}";

fn transfer_io(source: std::io::Error) -> LocalProductError {
    LocalProductError::Io {
        operation: "access remote transfer output",
        source,
    }
}

fn io(source: std::io::Error) -> LocalProductError {
    LocalProductError::Io {
        operation: "access update download cache",
        source,
    }
}

pub(super) fn fetch(
    roots: &LocalCoreRoots,
    url: &str,
    output: &File,
    max_bytes: u64,
    timeout: &str,
    offset: u64,
    mut progress: Option<&mut dyn FnMut(u64)>,
) -> Result<u64, LocalProductError> {
    ensure_curl_available(&roots.curl)?;
    if max_bytes == 0 || max_bytes > REMOTE_RELEASE_TOTAL_MAX_BYTES || offset >= max_bytes {
        return Err(LocalProductError::Remote(
            "remote release response bound is invalid",
        ));
    }
    let metadata = output.metadata().map_err(transfer_io)?;
    if !metadata.is_file() || metadata.len() != offset {
        return Err(LocalProductError::Remote(
            "remote release output must be a regular file at the requested offset",
        ));
    }
    let mut child_output = output.try_clone().map_err(transfer_io)?;
    child_output
        .seek(SeekFrom::Start(offset))
        .map_err(transfer_io)?;
    let (mut stderr, writer) = UnixStream::pair().map_err(transfer_io)?;
    stderr.set_nonblocking(true).map_err(transfer_io)?;
    let mut tail = Vec::new();
    let mut command = Command::new(&roots.curl);
    command
        .arg("--disable")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--location",
        ])
        .arg("--cacert")
        .arg(&roots.cert_file)
        .arg("--capath")
        .arg(&roots.cert_dir)
        .args([
            "--connect-timeout",
            REMOTE_CONNECT_TIMEOUT_SECONDS,
            "--max-time",
            timeout,
        ]);
    if timeout != REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS {
        command.args(["--speed-limit", "1024", "--speed-time", "60"]);
    }
    if offset != 0 {
        command.arg("--continue-at").arg(offset.to_string());
    }
    command
        .arg("--max-filesize")
        .arg((max_bytes - offset).to_string())
        .arg("--write-out")
        .arg(HTTP_STATUS)
        .arg("--url")
        .arg(url)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::from(child_output))
        .stderr(Stdio::from(OwnedFd::from(writer)));
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|_| LocalProductError::CurlUnavailable)?;
    let result = (|| {
        let mut last = u64::MAX;
        loop {
            let status = child.try_wait().map_err(transfer_io)?;
            loop {
                let mut buffer = [0; 1024];
                match stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        tail.extend_from_slice(&buffer[..n]);
                        if tail.len() > 256 {
                            tail.drain(..tail.len() - 256);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(transfer_io(e)),
                }
            }
            let observed = output.metadata().map_err(transfer_io)?.len();
            if observed > max_bytes {
                return Err(LocalProductError::RemoteResponseTooLarge);
            }
            if observed != last {
                if let Some(callback) = progress.as_mut() {
                    callback(observed);
                }
                last = observed;
            }
            if let Some(status) = status {
                if !status.success() {
                    return Err(LocalProductError::RemoteTransportFailed {
                        code: status.code(),
                        http_status: std::str::from_utf8(&tail)
                            .ok()
                            .and_then(|text| {
                                text.rsplit_once("CODEX_HTTP_STATUS=")
                                    .map(|(_, value)| value)
                            })
                            .and_then(|value| value.parse::<u16>().ok())
                            .filter(|code| (100..600).contains(code)),
                        bytes: observed,
                        elapsed_seconds: started.elapsed().as_secs(),
                    });
                }
                output.sync_all().map_err(transfer_io)?;
                return Ok(observed);
            }
            std::thread::sleep(Duration::from_millis(80));
        }
    })();
    // Reap the exact child on metadata, progress-bound and status failures too.
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn private_directory(path: &Path) -> Result<File, LocalProductError> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(io(e)),
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(NOFOLLOW | 0x800)
        .open(path)
        .map_err(io)?;
    let m = file.metadata().map_err(io)?;
    if !m.is_dir() || m.uid() != unsafe { geteuid() } || m.mode() & 0o077 != 0 {
        return Err(LocalProductError::UnsafeSource(
            "download cache must be an owned private directory",
        ));
    }
    Ok(file)
}

fn cache_file(path: &Path) -> Result<File, LocalProductError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(NOFOLLOW | 0x800)
        .open(path)
        .map_err(io)?;
    let m = file.metadata().map_err(io)?;
    if !m.is_file()
        || m.uid() != unsafe { geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
    {
        return Err(LocalProductError::UnsafeSource(
            "download cache entry must be an owned private regular file",
        ));
    }
    Ok(file)
}

/// Cache is private and disposable; its name/digest is never executable authority.
/// A directory lease serializes downloads and eviction independently of activation.
pub(super) struct PayloadOptions<'a> {
    pub limit: u64,
    pub expected_size: Option<u64>,
    pub timeout: &'a str,
    pub progress: Option<&'a mut dyn FnMut(u64)>,
}

pub(super) fn payload(
    roots: &LocalCoreRoots,
    url: &str,
    destination: &Path,
    entry: &ReleaseFileEntry,
    options: PayloadOptions<'_>,
) -> Result<u64, LocalProductError> {
    let PayloadOptions {
        limit,
        expected_size,
        timeout,
        progress,
    } = options;
    if (limit == 0 && expected_size != Some(0)) || expected_size.is_some_and(|n| n > limit) {
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    ensure_real_directory(
        &roots.state_root,
        "inspect update state",
        "update state is not a real directory",
    )?;
    let cache = roots.state_root.join("downloads-v1");
    let lease = private_directory(&cache)?;
    if unsafe { flock(lease.as_raw_fd(), FLOCK_EX | FLOCK_NB) } != 0 {
        return Err(LocalProductError::Remote(
            "another update owns the download cache; retry after it finishes",
        ));
    }
    let name = format!("{}.part", entry.sha256);
    let path = cache.join(&name);
    // Reserve the authenticated size (or protocol maximum). Evict only recognized
    // private entries under the held lease; unknown files are never deleted.
    let reserve = expected_size.unwrap_or(limit);
    let mut others = Vec::new();
    let mut total = reserve;
    for item in fs::read_dir(&cache).map_err(io)? {
        let item = item.map_err(io)?;
        if item.file_name() == name.as_str() {
            continue;
        }
        let filename = item.file_name();
        let Some(digest) = filename.to_str().and_then(|s| s.strip_suffix(".part")) else {
            continue;
        };
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            continue;
        }
        let f = cache_file(&item.path())?;
        let m = f.metadata().map_err(io)?;
        total = total
            .checked_add(m.len())
            .ok_or(LocalProductError::RemoteResponseTooLarge)?;
        others.push((m.modified().map_err(io)?, item.path(), m.len()));
    }
    others.sort();
    for (_, old, bytes) in others {
        if total <= REMOTE_RELEASE_TOTAL_MAX_BYTES {
            break;
        }
        fs::remove_file(old).map_err(io)?;
        total -= bytes;
    }
    let mut file = cache_file(&path)?;
    let mut size = file.metadata().map_err(io)?.len();
    if size > reserve {
        fs::remove_file(&path).map_err(io)?;
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    let verified = expected_size.is_none_or(|expected| size == expected)
        && openssl_sha256(&roots.openssl, &path)? == entry.sha256;
    if verified {
        if let Some(callback) = progress {
            callback(size);
        }
    } else {
        if size == reserve {
            fs::remove_file(&path).map_err(io)?;
            return Err(LocalProductError::ReleaseDigestMismatch);
        }
        match fetch(roots, url, &file, reserve, timeout, size, progress) {
            Ok(n) => size = n,
            Err(error) => {
                // Some curl versions reject the full200 response against the
                // remaining-byte bound before reporting ignored Range. Keeping
                // that prefix would repeat the same unusable resume forever.
                let ignored_range_size = size != 0
                    && matches!(
                        error,
                        LocalProductError::RemoteTransportFailed {
                            code: Some(63),
                            http_status: Some(200),
                            ..
                        }
                    );
                if ignored_range_size
                    || matches!(
                        error,
                        LocalProductError::RemoteResponseTooLarge
                            | LocalProductError::RemoteTransportFailed {
                                code: Some(33 | 36),
                                ..
                            }
                            | LocalProductError::RemoteTransportFailed {
                                http_status: Some(416),
                                ..
                            }
                    )
                {
                    fs::remove_file(&path).map_err(io)?;
                }
                return Err(error);
            }
        }
    }
    if expected_size.is_some_and(|n| n != size)
        || (!verified && openssl_sha256(&roots.openssl, &path)? != entry.sha256)
    {
        fs::remove_file(&path).map_err(io)?;
        return Err(LocalProductError::ReleaseDigestMismatch);
    }
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut output = create_remote_output(destination)?;
    copy_bounded_file_contents(
        &mut file,
        &mut output,
        size,
        "copy verified download",
        "download cache exceeds authenticated bound",
    )?;
    output.sync_all().map_err(io)?;
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::process::Child;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: std::path::PathBuf,
        roots: LocalCoreRoots,
        server: Option<Child>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Some(mut child) = self.server.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "codex-download-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let prefix = std::env::var_os("PREFIX")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "/usr".into());
        let roots = LocalCoreRoots {
            generation_root: root.clone(),
            state_root: root.clone(),
            config_dir: root.clone(),
            resolver_path: root.join("unused-resolver"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
            openssl: prefix.join("bin/openssl"),
            curl: prefix.join("bin/curl"),
        };
        fs::create_dir(&roots.cert_dir).unwrap();
        Fixture {
            root,
            roots,
            server: None,
        }
    }
    fn entry(f: &Fixture, data: &[u8]) -> ReleaseFileEntry {
        let source = f.root.join("source");
        fs::write(&source, data).unwrap();
        ReleaseFileEntry {
            relative_path: "runtime".into(),
            sha256: openssl_sha256(&f.roots.openssl, &source).unwrap(),
            mode: 0o755,
        }
    }
    // This is a real TLS origin and native curl, not a transport double. Fault is
    // injected at the socket: declared body length exceeds the first sent body.
    const SERVER: &str = r#"
import http.server, ssl, pathlib, sys, socket
root=pathlib.Path(sys.argv[1]); data=(root/'source').read_bytes()
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*a): pass
 def do_GET(self):
  header=self.headers.get('Range','')
  with (root/'requests').open('a') as f: f.write(header+'\n')
  mode=(root/'mode').read_text() if (root/'mode').exists() else ''
  offset=int(header.removeprefix('bytes=').removesuffix('-')) if header else 0
  accepted_range=bool(header) and mode!='ignore-range'
  if not accepted_range: offset=0
  self.send_response(206 if accepted_range else 200)
  if accepted_range: self.send_header('Content-Range',f'bytes {offset}-{len(data)-1}/{len(data)}')
  self.send_header('Content-Length',str(len(data)-offset)); self.end_headers()
  if not header and not mode:
   self.wfile.write(data[:len(data)//3]); self.wfile.flush(); self.connection.shutdown(socket.SHUT_RDWR)
  else: self.wfile.write(bytes(b^1 for b in data[offset:]) if mode=='changed' else data[offset:])
server=http.server.HTTPServer(('127.0.0.1',0),Handler)
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); context.load_cert_chain(root/'cert.pem',root/'key.pem')
server.socket=context.wrap_socket(server.socket,server_side=True)
(root/'port').write_text(str(server.server_port)); server.serve_forever()
"#;

    #[test]
    fn interrupted_https_resumes_and_verified_cache_works_offline() {
        let mut f = fixture();
        let data: Vec<u8> = (0..256 * 1024).map(|n| (n % 251) as u8).collect();
        let entry = entry(&f, &data);
        assert!(Command::new(&f.roots.openssl)
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=127.0.0.1",
                "-addext",
                "subjectAltName=IP:127.0.0.1",
                "-keyout"
            ])
            .arg(f.root.join("key.pem"))
            .arg("-out")
            .arg(&f.roots.cert_file)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        f.server = Some(
            Command::new("python3")
                .args(["-c", SERVER])
                .arg(&f.root)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let started = Instant::now();
        while !f.root.join("port").exists() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "TLS origin did not start"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let url = format!(
            "https://127.0.0.1:{}/runtime",
            fs::read_to_string(f.root.join("port")).unwrap()
        );
        let dest = f.root.join("download");
        let failed = payload(
            &f.roots,
            &url,
            &dest,
            &entry,
            PayloadOptions {
                limit: 512 * 1024,
                expected_size: Some(data.len() as u64),
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .unwrap_err();
        assert!(
            matches!(failed, LocalProductError::RemoteTransportFailed { code: Some(18 | 56), bytes, .. } if bytes == data.len() as u64/3),
            "{failed}"
        );
        assert!(!dest.exists());
        assert_eq!(
            payload(
                &f.roots,
                &url,
                &dest,
                &entry,
                PayloadOptions {
                    limit: 512 * 1024,
                    expected_size: Some(data.len() as u64),
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: None,
                },
            )
            .unwrap(),
            data.len() as u64
        );
        assert_eq!(fs::read(&dest).unwrap(), data);
        let requests = fs::read_to_string(f.root.join("requests")).unwrap();
        assert_eq!(requests, format!("\nbytes={}-\n", data.len() / 3));
        let part = f
            .root
            .join("downloads-v1")
            .join(format!("{}.part", entry.sha256));
        for mode in ["ignore-range", "changed"] {
            fs::remove_file(&dest).unwrap_or(());
            fs::write(&part, &data[..data.len() / 3]).unwrap();
            fs::write(f.root.join("mode"), mode).unwrap();
            let error = payload(
                &f.roots,
                &url,
                &dest,
                &entry,
                PayloadOptions {
                    limit: 512 * 1024,
                    expected_size: Some(data.len() as u64),
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: None,
                },
            )
            .unwrap_err();
            if mode == "ignore-range" {
                assert!(
                    matches!(
                        error,
                        LocalProductError::RemoteTransportFailed {
                            code: Some(33 | 36),
                            ..
                        } | LocalProductError::RemoteTransportFailed {
                            http_status: Some(416),
                            ..
                        } | LocalProductError::RemoteTransportFailed {
                            code: Some(63),
                            http_status: Some(200),
                            ..
                        }
                    ),
                    "{error}"
                );
            } else {
                assert!(
                    matches!(error, LocalProductError::ReleaseDigestMismatch),
                    "{error}"
                );
            }
            assert!(!dest.exists());
            assert!(
                !part.exists(),
                "invalid bytes must be discarded before the next explicit attempt"
            );
            drop(cache_file(&part).unwrap());
        }
        fs::write(&part, &data).unwrap();
        let mut server = f.server.take().unwrap();
        server.kill().unwrap();
        server.wait().unwrap();
        if dest.exists() {
            fs::remove_file(&dest).unwrap();
        }
        payload(
            &f.roots,
            &url,
            &dest,
            &entry,
            PayloadOptions {
                limit: 512 * 1024,
                expected_size: Some(data.len() as u64),
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
    }

    #[test]
    fn ignored_resume_filesize_error_discards_partial_without_masking_cause() {
        use std::os::unix::fs::PermissionsExt;
        let mut f = fixture();
        let entry = entry(&f, b"12345678");
        let cache = f.root.join("downloads-v1");
        drop(private_directory(&cache).unwrap());
        let part = cache.join(format!("{}.part", entry.sha256));
        let mut file = cache_file(&part).unwrap();
        std::io::Write::write_all(&mut file, b"1234").unwrap();
        // Inject the old-curl error at its process boundary. The real TLS test
        // above independently covers actual socket/Range/body handling.
        let shell = f.roots.openssl.parent().unwrap().join("sh");
        f.roots.curl = f.root.join("curl");
        fs::write(
            &f.roots.curl,
            format!(
                "#!{}\nprintf '%s' 'CODEX_HTTP_STATUS=200' >&2\nexit 63\n",
                shell.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&f.roots.curl, fs::Permissions::from_mode(0o700)).unwrap();
        let dest = f.root.join("download");
        let error = payload(
            &f.roots,
            "https://unused.invalid/",
            &dest,
            &entry,
            PayloadOptions {
                limit: 8,
                expected_size: Some(8),
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            LocalProductError::RemoteTransportFailed {
                code: Some(63),
                http_status: Some(200),
                bytes: 4,
                ..
            }
        ));
        assert!(!dest.exists());
        assert!(
            !part.exists(),
            "unusable prefix must not force another resume"
        );
    }

    #[test]
    fn authenticated_empty_payload_needs_no_transport_and_preserves_exact_bytes() {
        let f = fixture();
        let entry = entry(&f, b"");
        for size in [Some(0), None] {
            let dest = f.root.join("empty");
            assert_eq!(
                payload(
                    &f.roots,
                    "https://unused.invalid/",
                    &dest,
                    &entry,
                    PayloadOptions {
                        limit: 512,
                        expected_size: size,
                        timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                        progress: None,
                    },
                )
                .unwrap(),
                0
            );
            assert_eq!(fs::read(&dest).unwrap(), b"");
            fs::remove_file(dest).unwrap();
        }
    }

    #[test]
    fn corrupt_complete_cache_and_symlinks_never_reach_staging() {
        let f = fixture();
        let entry = entry(&f, b"correct");
        let dir = f.root.join("downloads-v1");
        private_directory(&dir).unwrap();
        let part = dir.join(format!("{}.part", entry.sha256));
        let file = cache_file(&part).unwrap();
        drop(file);
        fs::write(&part, b"invalid").unwrap();
        let dest = f.root.join("out");
        assert!(matches!(
            payload(
                &f.roots,
                "https://unused.invalid/",
                &dest,
                &entry,
                PayloadOptions {
                    limit: 7,
                    expected_size: Some(7),
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: None,
                },
            ),
            Err(LocalProductError::ReleaseDigestMismatch)
        ));
        assert!(!part.exists());
        assert!(!dest.exists());
        let protected = f.root.join("protected");
        fs::write(&protected, b"private").unwrap();
        symlink(&protected, &part).unwrap();
        assert!(payload(
            &f.roots,
            "https://unused.invalid/",
            &dest,
            &entry,
            PayloadOptions {
                limit: 7,
                expected_size: Some(7),
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .is_err());
        assert_eq!(fs::read(&protected).unwrap(), b"private");
        fs::remove_file(&part).unwrap();
        fs::remove_dir(&dir).unwrap();
        symlink(&f.root, &dir).unwrap();
        assert!(payload(
            &f.roots,
            "https://unused.invalid/",
            &dest,
            &entry,
            PayloadOptions {
                limit: 7,
                expected_size: Some(7),
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn busy_cache_has_no_writer_or_eviction() {
        let f = fixture();
        let entry = entry(&f, b"correct");
        let dir = f.root.join("downloads-v1");
        let lease = private_directory(&dir).unwrap();
        assert_eq!(unsafe { flock(lease.as_raw_fd(), FLOCK_EX | FLOCK_NB) }, 0);
        let dest = f.root.join("out");
        assert!(matches!(
            payload(
                &f.roots,
                "https://unused.invalid/",
                &dest,
                &entry,
                PayloadOptions {
                    limit: 7,
                    expected_size: Some(7),
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: None,
                },
            ),
            Err(LocalProductError::Remote(_))
        ));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    }
}
