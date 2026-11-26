//! Bounded HTTPS acquisition and resumable, untrusted payload storage. Signed
//! admission owns identities; this module never selects or activates a release.
use crate::error::LocalProductError;
use crate::fs_util::{ensure_real_directory, flock_fd, LOCK_EX, LOCK_NB};
use crate::generation::{sha256_regular_file, LocalWrapperRoots};
use crate::proc_exec;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::fs_util::O_NOFOLLOW as NOFOLLOW;

extern "C" {
    fn geteuid() -> u32;
}

pub(super) const REMOTE_RELEASE_TOTAL_MAX_BYTES: u64 = 1024 * 1024 * 1024;
pub(super) const REMOTE_CONNECT_TIMEOUT_SECONDS: &str = "15";
pub(super) const REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS: &str = "30";
pub(super) const REMOTE_TRANSFER_TIMEOUT_SECONDS: &str = "1800";
#[derive(Debug, Clone)]
pub(super) struct ReleaseFileEntry {
    #[allow(dead_code)]
    pub(super) relative_path: String,
    pub(super) sha256: String,
    #[allow(dead_code)]
    pub(super) mode: u32,
}

pub(super) fn ensure_curl_available(curl: &Path) -> Result<(), LocalProductError> {
    ensure_regular_file_tool(curl)
        .then_some(())
        .ok_or(LocalProductError::CurlUnavailable)
}

fn ensure_regular_file_tool(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|meta| meta.file_type().is_file() && meta.permissions().mode() & 0o111 != 0)
}

pub(super) fn create_remote_output(path: &Path) -> Result<File, LocalProductError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(LocalProductError::UnsafeSource(
                "remote output destination must not be a symlink or directory",
            ))
        }
        Ok(_) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LocalProductError::Io {
                operation: "inspect remote output path",
                source,
            })
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(NOFOLLOW | proc_exec::O_NONBLOCK_LINUX)
        .open(path)
        .map_err(|source| LocalProductError::Io {
            operation: "create remote download output",
            source,
        })?;
    let meta = file.metadata().map_err(|source| LocalProductError::Io {
        operation: "verify remote output",
        source,
    })?;
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != unsafe { geteuid() } {
        return Err(LocalProductError::UnsafeSource(
            "remote output must be an owned, unlinked regular file",
        ));
    }
    file.set_len(0).map_err(|source| LocalProductError::Io {
        operation: "truncate remote output",
        source,
    })?;
    Ok(file)
}

pub(super) fn copy_bounded_file_contents<R: io::Read, W: io::Write>(
    reader: &mut R,
    writer: &mut W,
    limit: u64,
    operation: &'static str,
    over_limit_message: &'static str,
) -> Result<u64, LocalProductError> {
    let mut total = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if n == 0 {
            return Ok(total);
        }
        total = total
            .checked_add(n as u64)
            .filter(|&sum| sum <= limit)
            .ok_or(LocalProductError::Remote(over_limit_message))?;
        writer
            .write_all(&buf[..n])
            .map_err(|source| LocalProductError::Io { operation, source })?;
    }
}

const HTTP_STATUS: &str = "%{stderr}CODEX_HTTP_STATUS=%{response_code}";
const STDERR_TAIL_BYTES: usize = 1024;
const REDIRECT_HEADER_MAX_BYTES: usize = 64 * 1024;
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Clone, Copy)]
pub(super) enum TransferTimeout<'a> {
    Fixed(&'a str),
    Quick(Instant),
    Progress(Duration),
}

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

fn parse_total_size_from_headers(header_path: &Path, offset: u64, limit: u64) -> Option<u64> {
    let bytes = fs::read(header_path).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let mut last_length: Option<u64> = None;
    let mut last_range_total: Option<u64> = None;
    for block in text.split("\r\n\r\n") {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut lines = trimmed.lines();
        let status_line = lines.next()?;
        let code = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())?;
        if (300..400).contains(&code) {
            last_length = None;
            last_range_total = None;
            continue;
        }
        if code != 200 && code != 206 {
            continue;
        }
        last_length = None;
        last_range_total = None;
        for line in lines {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if name.trim().eq_ignore_ascii_case("content-range") {
                if let Some((_, total_str)) = value.trim().rsplit_once('/') {
                    if let Ok(total) = total_str.trim().parse::<u64>() {
                        if total <= limit {
                            last_range_total = Some(total);
                        }
                    }
                }
            } else if name.trim().eq_ignore_ascii_case("content-length") {
                if let Ok(len) = value.trim().parse::<u64>() {
                    let computed = if code == 206 {
                        offset.checked_add(len)
                    } else {
                        Some(len)
                    };
                    if let Some(total) = computed.filter(|t| *t <= limit) {
                        last_length = Some(total);
                    }
                }
            }
        }
    }
    last_range_total.or(last_length)
}

fn parse_redirect_location(headers: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(headers).ok()?;
    let mut lines = text.lines();
    let status_line = lines.next()?;
    let code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())?;
    if !matches!(code, 301 | 302 | 303 | 307 | 308) {
        return None;
    }
    let mut location = None;
    for line in lines {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.trim().eq_ignore_ascii_case("location") {
                if location.is_some() {
                    return None;
                }
                let trimmed_val = value.trim();
                if trimmed_val.is_empty() {
                    return None;
                }
                location = Some(trimmed_val.to_owned());
            }
        }
    }
    location
}

pub(super) fn fetch_redirect_location(
    roots: &LocalWrapperRoots,
    url: &str,
    timeout: TransferTimeout<'_>,
) -> Result<String, LocalProductError> {
    ensure_curl_available(&roots.curl)?;
    let (reader, writer) = proc_exec::nonblocking_output_pipe().map_err(transfer_io)?;
    let mut command = Command::new(&roots.curl);
    let connect_timeout = match timeout {
        TransferTimeout::Quick(_) => "3",
        _ => REMOTE_CONNECT_TIMEOUT_SECONDS,
    };
    let wait_timeout = match timeout {
        TransferTimeout::Fixed(seconds) => {
            let secs = seconds.parse::<u64>().unwrap_or(30);
            command.args(["--max-time", seconds]);
            Duration::from_secs(secs.saturating_add(5))
        }
        TransferTimeout::Quick(deadline) => {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let millis = remaining.as_millis();
            if millis == 0 {
                return Err(transfer_io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "startup release check timed out",
                )));
            }
            let seconds = format!("{}.{:03}", millis / 1000, millis % 1000);
            command.args(["--max-time", &seconds]);
            remaining
        }
        TransferTimeout::Progress(idle) => {
            command.args(["--max-time", REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS]);
            idle
        }
    };
    command
        .arg("--disable")
        .args([
            "--silent",
            "--show-error",
            "--head",
            "--proto",
            "=https",
            "--max-redirs",
            "0",
            "-A",
            "codex-termux-wrapper",
        ])
        .arg("--cacert")
        .arg(&roots.cert_file)
        .arg("--capath")
        .arg(&roots.cert_dir)
        .args(["--connect-timeout", connect_timeout])
        .arg("--url")
        .arg(url)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::from(writer))
        .stderr(Stdio::null());
    proc_exec::isolate_process_group(&mut command);
    let started = Instant::now();
    let spawned = command.spawn();
    drop(command);
    let mut child = spawned.map_err(|_| LocalProductError::CurlUnavailable)?;
    let (out, status) = proc_exec::collect_bounded_output_and_wait(
        &mut child,
        reader,
        wait_timeout,
        REDIRECT_HEADER_MAX_BYTES,
    )
    .map_err(|err| match err {
        proc_exec::OutputError::Io(source) => transfer_io(source),
        proc_exec::OutputError::TimedOut(_) => transfer_io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "release redirect discovery timed out",
        )),
        proc_exec::OutputError::OutputTooLarge(_) => LocalProductError::RemoteResponseTooLarge,
    })?;
    if !status.success() {
        return Err(LocalProductError::RemoteTransportFailed {
            code: status.code(),
            http_status: None,
            bytes: out.len() as u64,
            elapsed_seconds: started.elapsed().as_secs(),
        });
    }
    parse_redirect_location(&out).ok_or(LocalProductError::LocalUpdate(
        "official latest release redirect response is invalid",
    ))
}

pub(super) fn fetch(
    roots: &LocalWrapperRoots,
    url: &str,
    output: &File,
    max_bytes: u64,
    timeout: TransferTimeout<'_>,
    offset: u64,
    progress: Option<&mut dyn FnMut(u64)>,
) -> Result<u64, LocalProductError> {
    let mut wrapped = progress;
    let mut bridge = wrapped
        .as_mut()
        .map(|cb| move |bytes: u64, _total: Option<u64>| cb(bytes));
    fetch_with_headers(
        roots,
        url,
        output,
        max_bytes,
        timeout,
        offset,
        None,
        None,
        bridge
            .as_mut()
            .map(|cb| cb as &mut dyn FnMut(u64, Option<u64>)),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_with_headers(
    roots: &LocalWrapperRoots,
    url: &str,
    output: &File,
    max_bytes: u64,
    timeout: TransferTimeout<'_>,
    offset: u64,
    expected_total: Option<u64>,
    header_path: Option<&Path>,
    mut progress: Option<&mut dyn FnMut(u64, Option<u64>)>,
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
    let connect_timeout = match timeout {
        TransferTimeout::Quick(_) => "3",
        _ => REMOTE_CONNECT_TIMEOUT_SECONDS,
    };
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
            "-A",
            "codex-termux-wrapper",
        ])
        .arg("--cacert")
        .arg(&roots.cert_file)
        .arg("--capath")
        .arg(&roots.cert_dir)
        .args(["--connect-timeout", connect_timeout]);
    if url.starts_with("https://api.github.com/") {
        command.args(["-H", "Accept: application/vnd.github+json"]);
    }
    if let Some(hdr) = header_path {
        command.arg("--dump-header").arg(hdr);
    }
    let quick_timeout = || {
        transfer_io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "startup release check timed out",
        ))
    };
    match timeout {
        TransferTimeout::Fixed(seconds) => {
            command.args(["--max-time", seconds]);
            if seconds != REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS {
                command.args(["--speed-limit", "1024", "--speed-time", "60"]);
            }
        }
        TransferTimeout::Quick(deadline) => {
            let millis = deadline
                .saturating_duration_since(Instant::now())
                .as_millis();
            if millis == 0 {
                return Err(quick_timeout());
            }
            let seconds = format!("{}.{:03}", millis / 1000, millis % 1000);
            command.args(["--max-time", &seconds]);
            proc_exec::isolate_process_group(&mut command);
        }
        TransferTimeout::Progress(_) => {
            // Expose file growth immediately to the idle timer.
            command.arg("--no-buffer");
        }
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
    let spawned = command.spawn();
    drop(command);
    let mut child = spawned.map_err(|_| LocalProductError::CurlUnavailable)?;
    let result = (|| {
        let mut last = u64::MAX;
        let mut last_total = None;
        let mut known_total = expected_total;
        let mut pending_headers = if known_total.is_none() {
            header_path
        } else {
            None
        };
        let mut last_received = offset;
        let mut last_progress = started;
        loop {
            let status = child.try_wait().map_err(transfer_io)?;
            loop {
                if matches!(timeout, TransferTimeout::Quick(until) if Instant::now() >= until) {
                    return Err(quick_timeout());
                }
                let mut buffer = [0; 1024];
                match stderr.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        tail.extend_from_slice(&buffer[..n]);
                        if tail.len() > STDERR_TAIL_BYTES {
                            tail.drain(..tail.len() - STDERR_TAIL_BYTES);
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
            // Curl has written the final response headers before emitting
            // body bytes. Wait for that boundary or exit so a proxy or
            // redirect response cannot supply the cached total.
            if observed > offset || status.is_some() {
                if let Some(path) = pending_headers.take() {
                    known_total = parse_total_size_from_headers(path, offset, max_bytes);
                }
            }
            if observed > last_received {
                last_received = observed;
                last_progress = Instant::now();
            }
            if observed != last || known_total != last_total {
                if let Some(callback) = progress.as_mut() {
                    callback(observed, known_total);
                }
                last = observed;
                last_total = known_total;
            }
            if let Some(status) = status {
                if !status.success() {
                    return Err(LocalProductError::RemoteTransportFailed {
                        code: status.code(),
                        // The suffix may start inside a UTF-8 sequence. Lossy
                        // decoding preserves the trailing ASCII status marker.
                        http_status: String::from_utf8_lossy(&tail)
                            .rsplit_once("CODEX_HTTP_STATUS=")
                            .and_then(|(_, value)| value.parse::<u16>().ok())
                            .filter(|code| (100..600).contains(code)),
                        bytes: observed,
                        elapsed_seconds: started.elapsed().as_secs(),
                    });
                }
                output.sync_all().map_err(transfer_io)?;
                return Ok(observed);
            }
            match timeout {
                TransferTimeout::Progress(idle) if last_progress.elapsed() >= idle => {
                    return Err(LocalProductError::RemoteStalled {
                        bytes: observed,
                        idle_seconds: idle.as_secs(),
                    });
                }
                _ => {}
            }
            let poll_interval = Duration::from_millis(80);
            let wait = match timeout {
                TransferTimeout::Quick(until) => {
                    poll_interval.min(until.saturating_duration_since(Instant::now()))
                }
                _ => poll_interval,
            };
            std::thread::sleep(wait);
        }
    })();
    // Reap the exact child on metadata, progress-bound and status failures too.
    if result.is_err() {
        if matches!(timeout, TransferTimeout::Quick(_)) {
            proc_exec::reap_process_group(&mut child);
        } else {
            proc_exec::kill_and_wait(&mut child);
        }
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
    pub force_download: bool,
    pub expected_size: Option<u64>,
    pub timeout: &'a str,
    pub progress: Option<&'a mut dyn FnMut(u64, Option<u64>)>,
}

pub(super) fn payload(
    roots: &LocalWrapperRoots,
    url: &str,
    destination: &Path,
    entry: &ReleaseFileEntry,
    options: PayloadOptions<'_>,
) -> Result<u64, LocalProductError> {
    let PayloadOptions {
        limit,
        force_download,
        expected_size,
        timeout,
        progress,
    } = options;
    if (limit == 0 && expected_size != Some(0)) || expected_size.is_some_and(|n| n > limit) {
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    let idle = timeout
        .parse::<u64>()
        .ok()
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .ok_or(LocalProductError::Remote("invalid payload idle timeout"))?;
    ensure_real_directory(
        &roots.state_root,
        "inspect update state",
        "update state is not a real directory",
    )?;
    let cache = roots.state_root.join("downloads");
    let lease = private_directory(&cache)?;
    if flock_fd(&lease, LOCK_EX | LOCK_NB).is_err() {
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
    struct HeaderCleanup(std::path::PathBuf);
    impl Drop for HeaderCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let header_path = HeaderCleanup(cache.join(format!("{}.hdr", entry.sha256)));
    match fs::remove_file(&header_path.0) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(io(error)),
    }
    let mut file = cache_file(&path)?;
    if force_download {
        file.set_len(0).map_err(io)?;
    }
    let mut size = file.metadata().map_err(io)?.len();
    if size > reserve {
        fs::remove_file(&path).map_err(io)?;
        return Err(LocalProductError::RemoteResponseTooLarge);
    }
    let verified = expected_size.is_none_or(|expected| size == expected)
        && if size == 0 {
            entry.sha256 == EMPTY_SHA256
        } else {
            sha256_regular_file(&path)? == entry.sha256
        };
    if verified {
        if let Some(callback) = progress {
            callback(size, expected_size.or(Some(size)));
        }
    } else {
        if size == reserve {
            fs::remove_file(&path).map_err(io)?;
            return Err(LocalProductError::ReleaseDigestMismatch);
        }
        match fetch_with_headers(
            roots,
            url,
            &file,
            reserve,
            TransferTimeout::Progress(idle),
            size,
            expected_size,
            progress.is_some().then_some(header_path.0.as_path()),
            progress,
        ) {
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
        || (!verified && sha256_regular_file(&path)? != entry.sha256)
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
        roots: LocalWrapperRoots,
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
        let prefix = {
            let _env = crate::tests::ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::env::var_os("PREFIX")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "/usr".into())
        };
        let roots = LocalWrapperRoots {
            generation_root: root.clone(),
            state_root: root.clone(),
            config_dir: root.clone(),
            resolver_path: root.join("unused-resolver"),
            cert_file: root.join("cert.pem"),
            cert_dir: root.join("certs"),
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
            sha256: sha256_regular_file(&source).unwrap(),
            mode: 0o755,
        }
    }
    // This is a real TLS origin and native curl, not a transport double. Fault is
    // injected at the socket: declared body length exceeds the first sent body.
    const SERVER: &str = r#"
import http.server, ssl, pathlib, sys, socket, time
root=pathlib.Path(sys.argv[1]); data=(root/'source').read_bytes()
class Handler(http.server.BaseHTTPRequestHandler):
 def log_message(self,*a): pass
 def do_GET(self):
  header=self.headers.get('Range','')
  with (root/'requests').open('a') as f: f.write(header+'\n')
  mode=(root/'mode').read_text() if (root/'mode').exists() else ''
  if mode=='redirect' and self.path=='/runtime':
   self.send_response(302)
   self.send_header('Location','/final')
   self.send_header('Content-Length','0')
   self.end_headers()
   return
  offset=int(header.removeprefix('bytes=').removesuffix('-')) if header else 0
  accepted_range=bool(header) and mode!='ignore-range'
  if not accepted_range: offset=0
  self.send_response(206 if accepted_range else 200)
  if accepted_range: self.send_header('Content-Range',f'bytes {offset}-{len(data)-1}/{len(data)}')
  self.send_header('Content-Length',str(len(data)-offset)); self.end_headers()
  if not header and not mode:
   self.wfile.write(data[:len(data)//3]); self.wfile.flush(); self.connection.shutdown(socket.SHUT_RDWR)
  elif mode=='slow-progress':
   for start in range(offset,len(data),128):
    self.wfile.write(data[start:start+128]); self.wfile.flush(); time.sleep(0.4)
  elif mode=='stall':
   self.wfile.write(data[:len(data)//2]); self.wfile.flush(); time.sleep(2)
  else: self.wfile.write(bytes(b^1 for b in data[offset:]) if mode=='changed' else data[offset:])
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); context.load_cert_chain(root/'cert.pem',root/'key.pem')
server.socket=context.wrap_socket(server.socket,server_side=True)
(root/'port').write_text(str(server.server_port)); server.serve_forever()
"#;

    fn serve(f: &mut Fixture) -> String {
        assert!(Command::new("openssl")
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
        format!(
            "https://127.0.0.1:{}/runtime",
            fs::read_to_string(f.root.join("port")).unwrap()
        )
    }

    #[test]
    fn slow_https_progress_extends_payload_timeout_but_stalls_fail() {
        let mut f = fixture();
        let data = vec![42; 2048];
        let entry = entry(&f, &data);
        let url = serve(&mut f);
        fs::write(f.root.join("mode"), "stall").unwrap();
        let path = f.root.join("download");
        let error = payload(
            &f.roots,
            &url,
            &path,
            &entry,
            PayloadOptions {
                limit: data.len() as u64,
                force_download: false,
                expected_size: Some(data.len() as u64),
                timeout: "1",
                progress: None,
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, LocalProductError::RemoteStalled { bytes, idle_seconds: 1 } if bytes == data.len() as u64 / 2),
            "{error}"
        );
        let cached = f
            .root
            .join("downloads")
            .join(format!("{}.part", entry.sha256));
        assert_eq!(fs::read(&cached).unwrap(), &data[..data.len() / 2]);
        assert!(!path.exists());

        fs::write(f.root.join("mode"), "slow-progress").unwrap();
        let started = Instant::now();
        assert_eq!(
            payload(
                &f.roots,
                &url,
                &path,
                &entry,
                PayloadOptions {
                    limit: data.len() as u64,
                    force_download: false,
                    expected_size: Some(data.len() as u64),
                    timeout: "1",
                    progress: None,
                },
            )
            .unwrap(),
            data.len() as u64
        );
        assert!(started.elapsed() > Duration::from_secs(1));
        assert_eq!(fs::read(&path).unwrap(), data);
        assert!(fs::read_to_string(f.root.join("requests"))
            .unwrap()
            .lines()
            .any(|line| line == format!("bytes={}-", data.len() / 2)));
    }

    #[test]
    fn interrupted_https_resumes_and_verified_cache_works_offline() {
        let mut f = fixture();
        let data: Vec<u8> = (0..256 * 1024).map(|n| (n % 251) as u8).collect();
        let entry = entry(&f, &data);
        let url = serve(&mut f);
        let dest = f.root.join("download");
        let failed = payload(
            &f.roots,
            &url,
            &dest,
            &entry,
            PayloadOptions {
                limit: 512 * 1024,
                force_download: false,
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
                    force_download: false,
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
            .join("downloads")
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
                    force_download: false,
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
                force_download: false,
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
        let cache = f.root.join("downloads");
        drop(private_directory(&cache).unwrap());
        let part = cache.join(format!("{}.part", entry.sha256));
        let mut file = cache_file(&part).unwrap();
        std::io::Write::write_all(&mut file, b"1234").unwrap();
        // Inject the old-curl error at its process boundary. The real TLS test
        // above independently covers actual socket/Range/body handling.
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        f.roots.curl = f.root.join("curl");
        fs::write(
            &f.roots.curl,
            format!(
                "#!{}\nprintf '%s' 'CODEX_HTTP_STATUS=200' >&2\nexit 63\n",
                shell
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
                force_download: false,
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
    fn rate_limit_hint_survives_multibyte_stderr_and_removes_headers() {
        use std::os::unix::fs::PermissionsExt;
        let mut f = fixture();
        let entry = entry(&f, b"expected release bytes");
        let shell = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let script = r#"
agent=
accept=
while [ "$#" -gt 0 ]; do
    case "$1" in
        -A) shift; agent=$1 ;;
        -H) shift; accept=$1 ;;
        --dump-header) shift; printf 'HTTP/2 %s\r\n\r\n' @STATUS@ > "$1" ;;
    esac
    shift
done
[ "$agent" = codex-termux-wrapper ] || exit 57
[ "$accept" = 'Accept: application/vnd.github+json' ] || exit 58
printf '%s' MULTIBYTE >&2
printf 'CODEX_HTTP_STATUS=%s' @STATUS@ >&2
exit 22
"#;
        f.roots.curl = f.root.join("curl");
        let header = f
            .root
            .join("downloads")
            .join(format!("{}.hdr", entry.sha256));
        for status in [403, 429] {
            let script = script
                .replace("@STATUS@", &status.to_string())
                .replace("MULTIBYTE", &"é".repeat(710));
            fs::write(&f.roots.curl, format!("#!{shell}\n{script}")).unwrap();
            fs::set_permissions(&f.roots.curl, fs::Permissions::from_mode(0o700)).unwrap();
            let dest = f.root.join("download");
            let error = payload(
                &f.roots,
                "https://api.github.com/repos/openai/codex/releases/latest",
                &dest,
                &entry,
                PayloadOptions {
                    limit: 512,
                    force_download: false,
                    expected_size: None,
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: Some(&mut |_, _| {}),
                },
            )
            .unwrap_err();
            assert!(
                matches!(error, LocalProductError::RemoteTransportFailed {
                    code: Some(22), http_status: Some(actual), ..
                } if actual == status),
                "{error}"
            );
            let message = error.to_string();
            assert!(message.contains("GitHub API rate limit"));
            assert!(message.contains("codex update --version <X.Y.Z>"));
            assert!(!header.exists());
            assert!(!dest.exists());
        }
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
                        force_download: false,
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
        let dir = f.root.join("downloads");
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
                    force_download: false,
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
                force_download: false,
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
                force_download: false,
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
        let dir = f.root.join("downloads");
        let lease = private_directory(&dir).unwrap();
        use crate::fs_util::flock;
        use std::os::fd::AsRawFd as _;
        assert_eq!(unsafe { flock(lease.as_raw_fd(), LOCK_EX | LOCK_NB) }, 0);
        let dest = f.root.join("out");
        assert!(matches!(
            payload(
                &f.roots,
                "https://unused.invalid/",
                &dest,
                &entry,
                PayloadOptions {
                    limit: 7,
                    force_download: false,
                    expected_size: Some(7),
                    timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                    progress: None,
                },
            ),
            Err(LocalProductError::Remote(_))
        ));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[test]
    fn progress_callback_receives_total_bytes_across_redirect_and_content_range() {
        let mut f = fixture();
        let redirect_headers =
            b"HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\n\r\n\
HTTP/2 206 Partial Content\r\nContent-Range: bytes 10-99/100\r\nContent-Length: 90\r\n\r\n";
        let header_path = f.root.join("headers.tmp");
        fs::write(&header_path, redirect_headers).unwrap();
        assert_eq!(
            parse_total_size_from_headers(&header_path, 10, 512 * 1024),
            Some(100)
        );

        let data = vec![7u8; 4096];
        let entry = entry(&f, &data);
        let url = serve(&mut f);
        fs::write(f.root.join("mode"), "redirect").unwrap();
        let dest = f.root.join("redirected-download");
        let mut observed = Vec::new();
        let mut cb = |downloaded: u64, total: Option<u64>| {
            observed.push((downloaded, total));
        };
        payload(
            &f.roots,
            &url,
            &dest,
            &entry,
            PayloadOptions {
                limit: 512 * 1024,
                force_download: false,
                expected_size: None,
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: Some(&mut cb),
            },
        )
        .unwrap();
        let header = f
            .root
            .join("downloads")
            .join(format!("{}.hdr", entry.sha256));
        assert!(!header.exists());
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert!(
            observed
                .iter()
                .any(|&(bytes, total)| bytes == data.len() as u64
                    && total == Some(data.len() as u64)),
            "expected progress callback to observe final total size across redirect: {observed:?}"
        );
        let requests = f.root.join("requests");
        assert_eq!(fs::read_to_string(&requests).unwrap().lines().count(), 2);
        fs::remove_file(&dest).unwrap();
        payload(
            &f.roots,
            &url,
            &dest,
            &entry,
            PayloadOptions {
                limit: 512 * 1024,
                force_download: true,
                expected_size: None,
                timeout: REMOTE_TRANSFER_TIMEOUT_SECONDS,
                progress: None,
            },
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&requests).unwrap().lines().count(), 4);
        assert_eq!(fs::read(&dest).unwrap(), data);
    }
}
