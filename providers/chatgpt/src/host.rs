//! Single provider-owned Linux ChatGPT AppHost lifecycle (PRoot + Xvnc + CDP pipe).
//!
//! No separate `codex-chatgpt-host` keepalive service: `chatgpt-provider` owns
//! its single `OwnedHost` directly, leases `~/.local/share/codex/chatgpt-desktop/profile`
//! via non-blocking `flock`, allocates a free X11 display, and cleans up process
//! groups and verified X11 lock/socket artifacts on drop.

use crate::cdp::CdpClient;
use crate::config::{ensure_private_dir, random_hex};
use crate::error::{ProviderError, Result};
use serde_json::json;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};

struct ChildGroup {
    child: Child,
    pid: u32,
}

impl ChildGroup {
    fn spawn(command: &mut Command) -> Result<Self> {
        command.kill_on_drop(true);
        let parent = unsafe { libc::getpid() };
        unsafe {
            command.pre_exec(move || {
                // Start a new session (pgid == pid, detached from any controlling TTY)
                // so background proot/Electron children can never be stopped by SIGTTIN/SIGTTOU.
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::other("parent unavailable"));
                }
                Ok(())
            });
        }
        let child = command
            .spawn()
            .map_err(|e| ProviderError::unavailable("host_process_start_failed", e.to_string()))?;
        let pid = child.id().ok_or_else(|| {
            ProviderError::unavailable("host_process_start_failed", "child exited immediately")
        })?;
        Ok(Self { child, pid })
    }

    async fn stop(&mut self) {
        if self.pid == 0 {
            return;
        }
        unsafe {
            libc::kill(-(self.pid as i32), libc::SIGTERM);
        }
        if tokio::time::timeout(Duration::from_secs(3), self.child.wait())
            .await
            .is_err()
        {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
            let _ = self.child.wait().await;
        }
        self.pid = 0;
    }
}

impl Drop for ChildGroup {
    fn drop(&mut self) {
        if self.pid != 0 {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
        }
    }
}

struct TemporaryDir(PathBuf);
impl Drop for TemporaryDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct DisplayArtifacts(Vec<(PathBuf, u64, u64)>);
impl DisplayArtifacts {
    fn capture(prefix: &Path, display: u16, pid: u32) -> Result<Self> {
        let lock = prefix.join(format!("tmp/.X{display}-lock"));
        if fs::read_to_string(&lock)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            != Some(pid)
        {
            return Err(ProviderError::unavailable(
                "host_display_owner_unconfirmed",
                format!("X display :{display} lock PID mismatch"),
            ));
        }
        let mut records = Vec::new();
        for path in [lock, prefix.join(format!("tmp/.X11-unix/X{display}"))] {
            let m = fs::symlink_metadata(&path).map_err(|_| {
                ProviderError::unavailable(
                    "host_display_owner_unconfirmed",
                    format!("missing X artifact {}", path.display()),
                )
            })?;
            if m.uid() != unsafe { libc::geteuid() } {
                return Err(ProviderError::unavailable(
                    "host_display_owner_unconfirmed",
                    "X artifact UID mismatch",
                ));
            }
            records.push((path, m.dev(), m.ino()));
        }
        Ok(Self(records))
    }
}

impl Drop for DisplayArtifacts {
    fn drop(&mut self) {
        for (path, dev, ino) in &self.0 {
            if fs::symlink_metadata(path).is_ok_and(|m| m.dev() == *dev && m.ino() == *ino) {
                let _ = fs::remove_file(path);
            }
        }
    }
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

impl AuthBrowserRelay {
    fn start(guest_home: &Path, ui_dir: &Path, prefix: &Path) -> Result<Self> {
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

        let url_file = ui_dir.join("open-url");
        let prefix_owned = prefix.to_path_buf();
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(250));
            loop {
                ticker.tick().await;
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
        Ok(Self { task })
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

fn write_xauthority(path: &Path, display: u16) -> Result<()> {
    let raw = random_hex(16)?;
    let cookie: Vec<u8> = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
        .collect();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&65535_u16.to_be_bytes());
    for field in [
        b"".as_slice(),
        display.to_string().as_bytes(),
        b"MIT-MAGIC-COOKIE-1".as_slice(),
        &cookie,
    ] {
        bytes.extend_from_slice(&(field.len() as u16).to_be_bytes());
        bytes.extend_from_slice(field);
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| ProviderError::internal("host_authority_failed", e.to_string()))
}

struct WindowAdjuster(tokio::task::JoinHandle<()>);
impl Drop for WindowAdjuster {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct OwnedHost {
    pub client: CdpClient,
    pub display: u16,
    pub rootfs: PathBuf,
    app: ChildGroup,
    window_manager: Option<ChildGroup>,
    window_adjuster: Option<WindowAdjuster>,
    display_child: ChildGroup,
    _display_artifacts: DisplayArtifacts,
    _auth_browser: AuthBrowserRelay,
    _profile_lease: File,
    temp_dir: TemporaryDir,
}

impl OwnedHost {
    pub async fn start(rootfs: &Path, profile_dir: &Path) -> Result<Self> {
        let canonical_rootfs = fs::canonicalize(rootfs).map_err(|_| {
            ProviderError::unavailable(
                "host_rootfs_not_found",
                format!("rootfs not found: {}", rootfs.display()),
            )
        })?;
        if !canonical_rootfs.join("usr/bin/chatgpt").is_file()
            || !canonical_rootfs
                .join("usr/lib/chatgpt/resources/app.asar")
                .is_file()
        {
            return Err(ProviderError::unavailable(
                "official_app_rootfs_required",
                format!(
                    "rootfs missing chatgpt binary or app.asar: {}",
                    canonical_rootfs.display()
                ),
            ));
        }

        let prefix = PathBuf::from(
            std::env::var_os("PREFIX")
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "/data/data/com.termux/files/usr".into()),
        );
        ensure_private_dir(profile_dir)?;

        // Exclusive non-blocking flock lease on a sibling `<profile_dir>.lock` file
        // so the Chromium user-data-dir itself is never directly flock'd.
        let mut lock_os = profile_dir.as_os_str().to_os_string();
        lock_os.push(".lock");
        let lock_path = PathBuf::from(lock_os);
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|e| ProviderError::unavailable("host_profile_unavailable", e.to_string()))?;
        if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(ProviderError::unavailable(
                "host_profile_busy",
                format!(
                    "profile directory is already locked by another host: {}",
                    profile_dir.display()
                ),
            ));
        }

        // Persist BOTH Electron's Chromium profile (`/root/chromium`) AND its embedded
        // Codex app-server OAuth credentials (`/root/.codex`) inside `profile_dir` so
        // authentication survives restarts and rootfs generation upgrades/rollbacks.
        let chromium_dir = profile_dir.join("chromium");
        let codex_home_dir = profile_dir.join(".codex");
        ensure_private_dir(&chromium_dir)?;
        ensure_private_dir(&codex_home_dir)?;
        // Safe to clear stale Chromium singleton artifacts because we hold the exclusive flock.
        for stale in ["SingletonLock", "SingletonSocket", "SingletonCookie"] {
            let _ = fs::remove_file(chromium_dir.join(stale));
        }

        let tmp_path = std::env::temp_dir().join(format!("cgp-{}", random_hex(4)?));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&tmp_path)
            .map_err(|e| ProviderError::internal("host_temp_failed", e.to_string()))?;
        let temp_dir = TemporaryDir(tmp_path);
        let path = &temp_dir.0;
        for name in ["tmp", "shm", "links"] {
            ensure_private_dir(&path.join(name))?;
        }
        let auth_browser = AuthBrowserRelay::start(profile_dir, path, &prefix)?;

        let socket_root = prefix.join("tmp/.X11-unix");
        let display = (130..1950_u16)
            .find(|id| {
                !socket_root.join(format!("X{id}")).exists()
                    && !prefix.join(format!("tmp/.X{id}-lock")).exists()
            })
            .ok_or_else(|| {
                ProviderError::unavailable("host_display_unavailable", "no free X11 display")
            })?;

        let auth = path.join("Xauthority");
        write_xauthority(&auth, display)?;

        let mut xserver = Command::new(prefix.join("bin/Xvnc"));
        xserver
            .env_clear()
            .env("HOME", path)
            .env("PATH", prefix.join("bin"))
            .env("PREFIX", &prefix)
            .env("TMPDIR", path)
            .args([
                format!(":{display}"),
                "-geometry".into(),
                "480x785".into(),
                "-depth".into(),
                "24".into(),
                "-nolisten".into(),
                "tcp".into(),
                "-auth".into(),
                auth.to_string_lossy().into_owned(),
                "-rfbport".into(),
                "-1".into(),
                "-rfbunixpath".into(),
                path.join("viewer").to_string_lossy().into_owned(),
                "-rfbunixmode".into(),
                "0600".into(),
                "-SecurityTypes".into(),
                "None".into(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut display_child = ChildGroup::spawn(&mut xserver)?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if tokio::net::UnixStream::connect(socket_root.join(format!("X{display}")))
                .await
                .is_ok()
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                display_child.stop().await;
                return Err(ProviderError::unavailable(
                    "host_display_start_timeout",
                    "Xvnc did not accept connections within 10s",
                ));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let display_artifacts = DisplayArtifacts::capture(&prefix, display, display_child.pid)?;

        let openbox_bin = prefix.join("bin/openbox");
        let window_manager = if openbox_bin.is_file() {
            let mut wm = Command::new(openbox_bin);
            wm.env_clear()
                .env("HOME", path)
                .env("PATH", prefix.join("bin"))
                .env("PREFIX", &prefix)
                .env("DISPLAY", format!(":{display}"))
                .env("XAUTHORITY", &auth)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            ChildGroup::spawn(&mut wm).ok()
        } else {
            None
        };

        let (parent, child) = UnixStream::pair()
            .map_err(|e| ProviderError::internal("host_pipe_unavailable", e.to_string()))?;
        parent
            .set_nonblocking(true)
            .map_err(|e| ProviderError::internal("host_pipe_unavailable", e.to_string()))?;
        let dup_fd = unsafe { libc::fcntl(child.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 64) };
        if dup_fd < 0 {
            return Err(ProviderError::internal(
                "host_pipe_unavailable",
                "fcntl F_DUPFD_CLOEXEC failed",
            ));
        }
        let dup_owned = unsafe { OwnedFd::from_raw_fd(dup_fd) };
        let source_fd = dup_owned.as_raw_fd();
        let root_file = File::open(&canonical_rootfs)
            .map_err(|e| ProviderError::unavailable("host_rootfs_open_failed", e.to_string()))?;
        let root_fd = root_file.as_raw_fd();

        let mut cmd = Command::new(prefix.join("bin/proot"));
        cmd.env_clear()
            .env("PATH", prefix.join("bin"))
            .env("HOME", profile_dir)
            .env("TMPDIR", path)
            .env("PROOT_TMP_DIR", path)
            .env("PROOT_L2S_DIR", path.join("links"))
            .args([
                "--kill-on-exit",
                "--link2symlink",
                "--sysvipc",
                "-L",
                "--change-id=0:0",
                "--rootfs=.",
                "--cwd=/root",
                "--bind=/dev",
                "--bind=/proc",
                "--bind=/sys",
                "--bind=/dev/urandom:/dev/random",
            ])
            .arg(format!("--bind={}:/root", profile_dir.display()))
            .arg(format!("--bind={}:/tmp", path.join("tmp").display()))
            .arg(format!("--bind={}:/dev/shm", path.join("shm").display()))
            .arg(format!("--bind={}:/tmp/.X11-unix", socket_root.display()))
            .arg(format!("--bind={}:/ui", path.display()))
            .args([
                "/usr/bin/env",
                "-i",
                "HOME=/root",
                "CODEX_HOME=/root/.codex",
                "PATH=/ui/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "BROWSER=/ui/bin/xdg-open",
                "LANG=C.UTF-8",
                "XAUTHORITY=/ui/Xauthority",
            ])
            .arg(format!("DISPLAY=:{display}"))
            .args([
                "/usr/bin/dbus-run-session",
                "--",
                "/usr/bin/chatgpt",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--user-data-dir=/root/chromium",
                "--remote-debugging-pipe",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(move || {
                if libc::fchdir(root_fd) != 0
                    || libc::dup2(source_fd, 3) < 0
                    || libc::dup2(source_fd, 4) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let app = ChildGroup::spawn(&mut cmd)?;
        drop(child);
        drop(dup_owned);
        drop(root_file);

        let tokio_pipe = tokio::net::UnixStream::from_std(parent)
            .map_err(|e| ProviderError::internal("host_pipe_unavailable", e.to_string()))?;
        let (rd, wr) = tokio_pipe.into_split();
        let client = CdpClient::from_pipe(rd, wr, random_hex(8)?);

        let mut host = Self {
            client,
            display,
            rootfs: canonical_rootfs,
            app,
            window_manager,
            window_adjuster: None,
            display_child,
            _display_artifacts: display_artifacts,
            _auth_browser: auth_browser,
            _profile_lease: lease,
            temp_dir,
        };

        if let Err(err) = host
            .client
            .request_with_timeout(
                "Browser.getVersion",
                json!({}),
                None,
                Duration::from_secs(30),
            )
            .await
        {
            host.shutdown().await;
            return Err(ProviderError::unavailable(
                "official_app_start_failed",
                format!("ChatGPT app CDP handshake failed: {err}"),
            ));
        }

        let wmctrl_bin = prefix.join("bin/wmctrl");
        if wmctrl_bin.is_file() {
            let display_str = format!(":{display}");
            let auth_path = auth.clone();
            let bin_dir = prefix.join("bin");
            host.window_adjuster = Some(WindowAdjuster(tokio::spawn(async move {
                let until = tokio::time::Instant::now() + Duration::from_secs(45);
                loop {
                    let ok = Command::new(&wmctrl_bin)
                        .env_clear()
                        .env("PATH", &bin_dir)
                        .env("DISPLAY", &display_str)
                        .env("XAUTHORITY", &auth_path)
                        .args(["-r", "ChatGPT", "-b", "add,maximized_vert,maximized_horz"])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await
                        .is_ok_and(|s| s.success());
                    if ok || tokio::time::Instant::now() >= until {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            })));
        }

        Ok(host)
    }

    pub fn viewer_socket(&self) -> PathBuf {
        self.temp_dir.0.join("viewer")
    }

    pub async fn shutdown(&mut self) {
        self.window_adjuster.take();
        let _ = tokio::time::timeout(
            Duration::from_secs(3),
            self.client.request("Browser.close", json!({}), None),
        )
        .await;
        self.app.stop().await;
        if let Some(ref mut wm) = self.window_manager {
            wm.stop().await;
        }
        self.display_child.stop().await;
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
