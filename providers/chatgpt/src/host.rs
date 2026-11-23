//! Provider-side CDP connection and control of the supervised AppHost.
use crate::cdp::CdpClient;
use crate::config::{ensure_private_dir, random_hex};
use crate::error::{ProviderError, Result};
use crate::supervisor::{Ready, StartError};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub(crate) fn termux_prefix() -> PathBuf {
    std::env::var_os("PREFIX")
        .filter(|s| !s.is_empty())
        .map_or_else(
            || PathBuf::from("/data/data/com.termux/files/usr"),
            PathBuf::from,
        )
}

pub(crate) fn duplicate_fd(fd: RawFd) -> io::Result<OwnedFd> {
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 64) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

/// Minimal allow-listed Android OAuth browser relay for in-app ChatGPT sign-in.
///
/// Configures `/ui/bin/xdg-open`, `guest_home/.local/share/applications/codex-termux-auth-browser.desktop`,
/// and `guest_home/.config/mimeapps.list` so Electron's `shell.openExternal` / `xdg-open` / `gio open`
/// hands OAuth URLs to the user's real Android browser via `termux-am-socket` instead of a Linux browser.
struct AuthBrowserRelay {
    task: tokio::task::JoinHandle<()>,
}

const AUTH_DESKTOP_NAME: &str = "codex-termux-auth-browser.desktop";
const AUTH_DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Termux authentication browser\nExec=/ui/bin/xdg-open %u\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\n";
const AUTH_OPENER_SCRIPT: &str = "#!/bin/sh\nset -eu\ntest \"$#\" = 1\numask 077\nprintf '%s' \"$1\" > /ui/open-url.tmp\nmv /ui/open-url.tmp /ui/open-url\n";

fn write_private_file(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| ProviderError::internal("auth_browser_setup_failed", "missing parent"))?;
    ensure_private_dir(parent)?;
    let tmp = parent.join(format!(".tmp-{}-{}", std::process::id(), random_hex(4)?));
    let res = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(if executable { 0o700 } else { 0o600 })
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if let Err(e) = res {
        let _ = fs::remove_file(&tmp);
        return Err(ProviderError::internal(
            "auth_browser_setup_failed",
            e.to_string(),
        ));
    }
    Ok(())
}

pub fn merge_mimeapps_defaults(previous: &str) -> String {
    let mut output = String::new();
    let mut in_defaults = false;
    let mut seen = false;
    let entries = format!(
        "x-scheme-handler/http={AUTH_DESKTOP_NAME}\nx-scheme-handler/https={AUTH_DESKTOP_NAME}\n"
    );
    for line in previous.lines() {
        if line.starts_with('[') {
            if in_defaults {
                output.push_str(&entries);
            }
            in_defaults = line == "[Default Applications]";
            seen |= in_defaults;
        } else if in_defaults
            && (line.starts_with("x-scheme-handler/http=")
                || line.starts_with("x-scheme-handler/https="))
        {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if in_defaults {
        output.push_str(&entries);
    }
    if !seen {
        output.push_str("[Default Applications]\n");
        output.push_str(&entries);
    }
    output
}

pub async fn open_android_url(prefix: &Path, url: &str) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(10), launch_android_url(prefix, url))
        .await
        .map_err(|_| {
            ProviderError::unavailable(
                "android_browser_launch_failed",
                "Android browser launch timed out",
            )
        })?
}

async fn launch_android_url(prefix: &Path, url: &str) -> Result<()> {
    if url.is_empty()
        || url.len() > 16_383
        || url.chars().any(|c| {
            c.is_whitespace()
                || c.is_control()
                || matches!(c, '\'' | '"' | ';' | '`' | '\\' | '<' | '>' | '(' | ')')
        })
        || !(url.starts_with("https://") || url.starts_with("http://127.0.0.1:"))
    {
        return Err(ProviderError::bad_request(
            "android_url_rejected",
            "only safe https:// or localhost viewer URLs may be opened on Android",
        ));
    }
    let am_socket = prefix.join("bin/termux-am-socket");
    if am_socket.is_file() {
        let output = Command::new(&am_socket)
            .arg(format!("start -a android.intent.action.VIEW -d {url}"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| {
                ProviderError::unavailable("android_browser_launch_failed", e.to_string())
            })?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output.status.success() && !combined.contains("Error:") {
            return Ok(());
        }
        return Err(ProviderError::unavailable(
            "android_browser_launch_failed",
            combined.trim().to_owned(),
        ));
    }
    let open_url = prefix.join("bin/termux-open-url");
    if open_url.is_file() {
        let status = Command::new(&open_url)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status()
            .await
            .map_err(|e| {
                ProviderError::unavailable("android_browser_launch_failed", e.to_string())
            })?;
        if status.success() {
            return Ok(());
        }
    }
    Err(ProviderError::unavailable(
        "android_browser_unavailable",
        "neither termux-am-socket nor termux-open-url succeeded",
    ))
}

pub(crate) fn prepare_auth_browser(guest_home: &Path, ui_dir: &Path) -> Result<()> {
    let bin = ui_dir.join("bin");
    ensure_private_dir(&bin)?;
    write_private_file(&bin.join("xdg-open"), AUTH_OPENER_SCRIPT.as_bytes(), true)?;

    let apps_dir = guest_home.join(".local/share/applications");
    ensure_private_dir(&guest_home.join(".local"))?;
    ensure_private_dir(&guest_home.join(".local/share"))?;
    ensure_private_dir(&apps_dir)?;
    write_private_file(
        &apps_dir.join(AUTH_DESKTOP_NAME),
        AUTH_DESKTOP_ENTRY.as_bytes(),
        false,
    )?;

    let cfg_dir = guest_home.join(".config");
    ensure_private_dir(&cfg_dir)?;
    let mime_path = cfg_dir.join("mimeapps.list");
    let existing = fs::read_to_string(&mime_path).unwrap_or_default();
    let merged = merge_mimeapps_defaults(&existing);
    write_private_file(&mime_path, merged.as_bytes(), false)?;

    Ok(())
}

impl AuthBrowserRelay {
    fn start(ui_dir: &Path, prefix: &Path, running: Arc<AtomicBool>) -> Self {
        let url_file = ui_dir.join("open-url");
        let prefix_owned = prefix.to_path_buf();
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(250));
            loop {
                ticker.tick().await;
                if !running.load(Ordering::Acquire) {
                    break;
                }
                if let Ok(file) = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&url_file)
                {
                    let mut raw = String::new();
                    let _ = file.take(16_384).read_to_string(&mut raw);
                    let _ = fs::remove_file(&url_file);
                    let url = raw.trim();
                    if is_allowed_oauth_url(url) {
                        let _ = open_android_url(&prefix_owned, url).await;
                    }
                }
            }
        });
        Self { task }
    }
}

impl Drop for AuthBrowserRelay {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub fn is_allowed_oauth_url(url: &str) -> bool {
    if url.is_empty()
        || url.len() > 16_383
        || url.chars().any(|c| c.is_whitespace() || c.is_control())
        || url.contains('#')
    {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let (authority, path_and_query) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    if authority.contains('@') {
        return false;
    }
    let host = match authority.split_once(':') {
        Some((h, "443")) => h,
        Some(_) => return false,
        None => authority,
    };
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };
    for pair in query.split('&') {
        let key = pair.split_once('=').map_or(pair, |(k, _)| k);
        if matches!(key, "id_token" | "access_token" | "refresh_token") {
            return false;
        }
    }
    match host {
        "auth.openai.com" | "auth0.openai.com" => {
            path == "/oauth/authorize"
                || path.starts_with("/api/accounts/")
                || path.starts_with("/u/")
                || path.starts_with("/authorize")
        }
        "chatgpt.com" => path == "/codex/desktop-auth" || path.starts_with("/auth/"),
        "accounts.google.com"
        | "appleid.apple.com"
        | "login.microsoftonline.com"
        | "login.live.com" => true,
        _ => false,
    }
}

pub struct OwnedHost {
    pub client: CdpClient,
    pub display: u16,
    pub rootfs: PathBuf,
    pub running: Arc<AtomicBool>,
    directory: PathBuf,
    close: Option<oneshot::Sender<()>>,
    completion: Option<JoinHandle<Result<()>>>,
    _auth_browser: AuthBrowserRelay,
}

impl OwnedHost {
    pub async fn start(rootfs: &Path, profile: &Path) -> Result<Self> {
        fn pipe_error(error: io::Error) -> ProviderError {
            ProviderError::unavailable("host_pipe_unavailable", error.to_string())
        }
        let (control, peer) = UnixStream::pair().map_err(pipe_error)?;
        let (cdp, guest) = UnixStream::pair().map_err(pipe_error)?;
        let inherited_ctrl = duplicate_fd(peer.as_raw_fd()).map_err(pipe_error)?;
        let inherited_cdp = duplicate_fd(guest.as_raw_fd()).map_err(pipe_error)?;
        let ctrl_fd = inherited_ctrl.as_raw_fd();
        let cdp_fd = inherited_cdp.as_raw_fd();
        let mut command = Command::new(std::env::current_exe().map_err(pipe_error)?);
        command
            .arg("host-supervisor")
            .arg(rootfs)
            .arg(profile)
            .args(["3", "4", "serve"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(ctrl_fd, 3) < 0 || libc::dup2(cdp_fd, 4) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(pipe_error)?;
        drop((inherited_ctrl, inherited_cdp, peer, guest));
        control.set_nonblocking(true).map_err(pipe_error)?;
        cdp.set_nonblocking(true).map_err(pipe_error)?;
        let control = tokio::net::UnixStream::from_std(control).map_err(pipe_error)?;
        let (read, write) = control.into_split();
        let mut read = BufReader::new(read);
        let mut line = String::new();
        let size = tokio::time::timeout(Duration::from_secs(17), read.read_line(&mut line))
            .await
            .map_err(|_| {
                ProviderError::unavailable(
                    "host_start_timeout",
                    "AppHost supervisor did not announce startup",
                )
            })?
            .map_err(pipe_error)?;
        if size == 0 {
            return Err(ProviderError::unavailable(
                "host_supervisor_exited",
                "AppHost supervisor exited during launch",
            ));
        }
        let reply = serde_json::from_str::<std::result::Result<Ready, StartError>>(&line)
            .map_err(|e| ProviderError::internal("host_supervisor_reply", e.to_string()))?;
        let ready = reply.map_err(|error| {
            let code = match error.code.as_str() {
                "host_profile_busy" => "host_profile_busy",
                "host_profile_busy_stale_electron" => "host_profile_busy_stale_electron",
                "host_profile_unavailable" => "host_profile_unavailable",
                "host_rootfs_not_found" => "host_rootfs_not_found",
                "official_app_rootfs_required" => "official_app_rootfs_required",
                _ => "host_supervisor_start_failed",
            };
            ProviderError::unavailable(code, error.detail)
        })?;
        let (rd, wr) = tokio::net::UnixStream::from_std(cdp)
            .map_err(pipe_error)?
            .into_split();
        let client = CdpClient::from_pipe(rd, wr, random_hex(8)?);
        let running = Arc::new(AtomicBool::new(true));
        let state = running.clone();
        let (close, close_requested) = oneshot::channel();
        let completion = tokio::spawn(async move {
            let mut byte = [0u8];
            tokio::select! {
                _ = read.read(&mut byte) => {}
                _ = close_requested => {}
            }
            state.store(false, Ordering::Release);
            // OwnedWriteHalf drop shuts down this direction even while read is open.
            drop(write);
            let status = child.wait().await.map_err(pipe_error)?;
            if status.success() {
                Ok(())
            } else {
                Err(ProviderError::unavailable(
                    "host_supervisor_exited",
                    status.to_string(),
                ))
            }
        });
        let mut host = Self {
            client,
            display: ready.display,
            rootfs: ready.rootfs,
            _auth_browser: AuthBrowserRelay::start(
                &ready.directory,
                &termux_prefix(),
                running.clone(),
            ),
            running,
            directory: ready.directory,
            close: Some(close),
            completion: Some(completion),
        };
        let ready = host.client.request_with_timeout(
            "Browser.getVersion",
            json!({}),
            None,
            Duration::from_secs(30),
        );
        let handshake = tokio::select! {
            result = ready => result,
            result = host.completion.as_mut().unwrap() => {
                let detail = result.ok().and_then(std::result::Result::err)
                    .map_or_else(|| "AppHost exited during CDP startup".to_owned(), |error| error.to_string());
                return Err(ProviderError::unavailable("official_app_start_failed", detail));
            }
        };
        if let Err(error) = handshake {
            let _ = host.shutdown().await;
            let mut tail = String::new();
            if let Ok(mut log) = fs::File::open(profile.join("host.log")) {
                use std::io::{Seek, SeekFrom};
                if let Ok(meta) = log.metadata() {
                    let _ = log.seek(SeekFrom::Start(meta.len().saturating_sub(800)));
                }
                let _ = log.take(800).read_to_string(&mut tail);
            }
            return Err(ProviderError::unavailable(
                "official_app_start_failed",
                format!(
                    "ChatGPT app CDP handshake failed: {error} (host.log tail: {})",
                    tail.trim()
                ),
            ));
        }
        Ok(host)
    }

    pub fn viewer_socket(&self) -> PathBuf {
        self.directory.join("viewer")
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self._auth_browser.task.abort();
        if self.running.swap(false, Ordering::AcqRel) {
            let _ = tokio::time::timeout(
                Duration::from_secs(1),
                self.client.request("Browser.close", json!({}), None),
            )
            .await;
        }
        self.close.take();
        let Some(completion) = self.completion.as_mut() else {
            return Ok(());
        };
        let result = tokio::time::timeout(Duration::from_secs(4), completion)
            .await
            .map_err(|_| {
                ProviderError::unavailable(
                    "host_cleanup_pending",
                    "AppHost cleanup has not completed",
                )
            })?;
        self.completion.take();
        result.map_err(|error| {
            ProviderError::internal("host_supervisor_wait_failed", error.to_string())
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_url_allowlist_blocks_arbitrary_hosts_and_schemes() {
        assert!(is_allowed_oauth_url(
            "https://auth.openai.com/oauth/authorize?state=1"
        ));
        assert!(is_allowed_oauth_url(
            "https://auth.openai.com/api/accounts/login"
        ));
        assert!(is_allowed_oauth_url(
            "https://chatgpt.com/codex/desktop-auth?authorize_url=https%3A%2F%2Fauth.openai.com%2Foauth%2Fauthorize"
        ));
        assert!(is_allowed_oauth_url(
            "https://accounts.google.com/o/oauth2/v2/auth"
        ));
        assert!(!is_allowed_oauth_url(
            "http://auth.openai.com/oauth/authorize"
        ));
        assert!(!is_allowed_oauth_url(
            "https://auth.openai.com/oauth/authorize?access_token=secret"
        ));
        assert!(!is_allowed_oauth_url(
            "https://auth.openai.com/oauth/authorize#frag"
        ));
        assert!(!is_allowed_oauth_url(
            "https://user@auth.openai.com/oauth/authorize"
        ));
        assert!(!is_allowed_oauth_url("https://evil.example.com/phish"));
        assert!(!is_allowed_oauth_url("file:///etc/passwd"));
    }

    #[test]
    fn mimeapps_merge_sets_android_auth_handler_and_preserves_other_entries() {
        let old = "[Default Applications]\ntext/plain=other.desktop\nx-scheme-handler/http=firefox.desktop\n";
        let merged = merge_mimeapps_defaults(old);
        assert!(merged.contains("text/plain=other.desktop"));
        assert!(merged.contains("x-scheme-handler/http=codex-termux-auth-browser.desktop"));
        assert!(merged.contains("x-scheme-handler/https=codex-termux-auth-browser.desktop"));
        assert!(!merged.contains("firefox.desktop"));
    }
}
