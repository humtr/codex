use crate::cli::UpstreamArgScan;
use crate::error::RuntimeLaunchError;
use crate::fs_util::{self, fcntl, LOCAL_STAGING_COUNTER};
use crate::generation::{termux_browser_helper_paths, LocalWrapperRoots, QualifiedRuntimeAssets};
use crate::{daemon_settings, timezone};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Seek as _, Write as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
use std::os::raw::c_int;
use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

pub const RESOLVER_FD: c_int = 33;
pub const CONFIG_DIR_FD: c_int = 34;
pub const TEMP_ROOT_FD: c_int = 35;

pub(super) const SAFE_MIN_FD: c_int = 64;
const F_DUPFD_CLOEXEC: c_int = 1030;
const O_NONBLOCK_LINUX: c_int = 0o4000;
const F_SETFD: c_int = 2;
const RESOLVER_SYNTHESIS_MAX_BYTES: usize = 64 * 1024;
const RESOLVER_DEFAULT_OPTIONS: &[u8] = b"options timeout:2 attempts:10\n";

// FD 34 configuration loaded by upstream Codex via inherited dirfd.
pub(super) const TERMUX_WRAPPER_CONFIG_CONTENT: &str = "# codex-termux-wrapper\nsandbox_mode = \"danger-full-access\"\nmcp_oauth_credentials_store = \"file\"\n";
static RESOLVER_SYNTHESIS_COUNTER: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
}

pub(super) struct RuntimeFdSources {
    resolver: OwnedFd,
    config_dir: OwnedFd,
    temporary_root: OwnedFd,
}

pub(super) fn dup_above_safe_min(fd: c_int) -> io::Result<OwnedFd> {
    let duplicated = unsafe { fcntl(fd, F_DUPFD_CLOEXEC, SAFE_MIN_FD) };
    if duplicated < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicated) })
}

fn resolver_has_options_directive(bytes: &[u8]) -> bool {
    bytes.split(|&byte| byte == b'\n').any(|raw_line| {
        let trimmed_end = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        let start = trimmed_end
            .iter()
            .position(|&byte| !matches!(byte, b' ' | b'\t'))
            .unwrap_or(trimmed_end.len());
        let line = &trimmed_end[start..];
        if line.is_empty() || matches!(line[0], b'#' | b';') {
            return false;
        }
        line.strip_prefix(b"options").is_some_and(|rest| {
            rest.first()
                .is_some_and(|&byte| matches!(byte, b' ' | b'\t'))
        })
    })
}

fn read_regular_file_if_exact_inode(
    path: &Path,
    limit: usize,
) -> io::Result<Option<(File, Vec<u8>)>> {
    let mut ro = match OpenOptions::new()
        .read(true)
        .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
        .open(path)
    {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let ro_meta = ro.metadata()?;
    let sym_meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    if !sym_meta.file_type().is_file()
        || ro_meta.nlink() != 1
        || ro_meta.uid() != fs_util::current_uid()?
        || sym_meta.dev() != ro_meta.dev()
        || sym_meta.ino() != ro_meta.ino()
    {
        return Err(io::Error::other(
            "cached resolver path must be a regular non-symlink file",
        ));
    }
    let mut existing = Vec::new();
    io::Read::by_ref(&mut ro)
        .take(limit as u64 + 1)
        .read_to_end(&mut existing)?;
    Ok(Some((ro, existing)))
}

pub(super) fn open_synthesized_resolver(bytes: &[u8], cache: bool) -> io::Result<File> {
    let mut synthesized = Vec::with_capacity(bytes.len() + RESOLVER_DEFAULT_OPTIONS.len() + 1);
    synthesized.extend_from_slice(bytes);
    if !synthesized.is_empty() && !synthesized.ends_with(b"\n") {
        synthesized.push(b'\n');
    }
    synthesized.extend_from_slice(RESOLVER_DEFAULT_OPTIONS);

    if let Some(home) = cache
        .then(|| std::env::var_os("HOME").filter(|v| !v.is_empty()))
        .flatten()
    {
        let state_dir = PathBuf::from(home).join(".local/share/codex/wrapper");
        if fs::symlink_metadata(&state_dir).is_ok_and(|meta| meta.file_type().is_dir()) {
            let cache_path = state_dir.join("resolv.conf");
            // Fast lock-free path: avoid RW open, flock, or fsync when cache already matches.
            if let Some((mut ro, existing)) =
                read_regular_file_if_exact_inode(&cache_path, RESOLVER_SYNTHESIS_MAX_BYTES)?
            {
                if existing == synthesized {
                    ro.seek(io::SeekFrom::Start(0))?;
                    return Ok(ro);
                }
            }
            let lock_path = state_dir.join(".resolv.conf.lock");
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
                .open(&lock_path)?;
            let lock_meta = lock.metadata()?;
            if !lock_meta.is_file()
                || lock_meta.nlink() != 1
                || lock_meta.uid() != fs_util::current_uid()?
            {
                return Err(io::Error::other("resolver cache lock is unsafe"));
            }
            fs_util::flock_with_timeout(
                &lock,
                fs_util::LOCK_EX,
                std::time::Duration::from_secs(5),
                "timed out locking resolver cache",
            )?;
            if let Some((mut ro, existing)) =
                read_regular_file_if_exact_inode(&cache_path, RESOLVER_SYNTHESIS_MAX_BYTES)?
            {
                if existing == synthesized {
                    ro.seek(io::SeekFrom::Start(0))?;
                    return Ok(ro);
                }
            }
            let seq = RESOLVER_SYNTHESIS_COUNTER.fetch_add(1, Ordering::Relaxed);
            let tmp_path = state_dir.join(format!(".resolv.conf.{}.{seq}.tmp", std::process::id()));
            let mut tmp = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
                .open(&tmp_path)?;
            let result = (|| -> io::Result<File> {
                tmp.write_all(&synthesized)?;
                tmp.sync_all()?;
                tmp.seek(io::SeekFrom::Start(0))?;
                fs::rename(&tmp_path, &cache_path)?;
                fs_util::sync_directory(&state_dir)?;
                Ok(tmp)
            })();
            if result.is_err() {
                let _ = fs::remove_file(&tmp_path);
            }
            return result;
        }
    }

    let tmp_root = std::env::temp_dir();
    for _ in 0..32 {
        let seq = RESOLVER_SYNTHESIS_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp_path = tmp_root.join(format!(".codex-resolv.{}.{seq}.tmp", std::process::id()));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp_path)
        {
            Ok(mut file) => {
                let write_res = (|| -> io::Result<()> {
                    file.write_all(&synthesized)?;
                    file.seek(io::SeekFrom::Start(0))?;
                    Ok(())
                })();
                let _ = fs::remove_file(&tmp_path);
                write_res?;
                return Ok(file);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    Err(io::Error::other("could not create temporary resolver file"))
}

impl RuntimeFdSources {
    pub(super) fn open<R: AsRef<Path>, C: AsRef<Path>>(
        resolver_path: R,
        config_dir: C,
    ) -> io::Result<Self> {
        let config = OpenOptions::new()
            .read(true)
            .custom_flags(fs_util::O_NOFOLLOW | fs_util::O_DIRECTORY)
            .open(config_dir)?;
        Self::open_with_config_file(resolver_path, &config)
    }

    pub(super) fn open_with_config_file<R: AsRef<Path>>(
        resolver_path: R,
        config: &File,
    ) -> io::Result<Self> {
        Self::open_with_resolver_cache(resolver_path, config, true)
    }

    pub(super) fn open_with_resolver_cache<R: AsRef<Path>>(
        resolver_path: R,
        config: &File,
        cache: bool,
    ) -> io::Result<Self> {
        if !config.metadata()?.is_dir() {
            return Err(io::Error::from_raw_os_error(20));
        }
        let mut source_resolver = OpenOptions::new()
            .read(true)
            .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
            .open(resolver_path)?;
        if !source_resolver.metadata()?.is_file() {
            return Err(io::Error::from_raw_os_error(21));
        }
        let mut resolver_bytes = Vec::new();
        io::Read::by_ref(&mut source_resolver)
            .take(RESOLVER_SYNTHESIS_MAX_BYTES as u64 + 1)
            .read_to_end(&mut resolver_bytes)?;
        let resolver_file = if resolver_bytes.len() <= RESOLVER_SYNTHESIS_MAX_BYTES
            && !resolver_has_options_directive(&resolver_bytes)
        {
            open_synthesized_resolver(&resolver_bytes, cache)?
        } else {
            source_resolver.seek(io::SeekFrom::Start(0))?;
            source_resolver
        };
        let temporary = OpenOptions::new()
            .read(true)
            .custom_flags(fs_util::O_NOFOLLOW | fs_util::O_DIRECTORY)
            .open(std::env::temp_dir())?;
        if !temporary.metadata()?.is_dir() {
            return Err(io::Error::from_raw_os_error(20));
        }

        // Move every descriptor >= SAFE_MIN_FD so dup2 to 33/34/35 cannot clobber each other.
        Ok(Self {
            resolver: dup_above_safe_min(resolver_file.as_raw_fd())?,
            config_dir: dup_above_safe_min(config.as_raw_fd())?,
            temporary_root: dup_above_safe_min(temporary.as_raw_fd())?,
        })
    }

    pub(super) fn configure(&self, command: &mut Command) {
        let resolver_fd = self.resolver.as_raw_fd();
        let config_fd = self.config_dir.as_raw_fd();
        let temp_fd = self.temporary_root.as_raw_fd();
        let keepalive = (
            dup_above_safe_min(self.resolver.as_raw_fd()).expect("duplicate resolver fd"),
            dup_above_safe_min(self.config_dir.as_raw_fd()).expect("duplicate config fd"),
            dup_above_safe_min(self.temporary_root.as_raw_fd()).expect("duplicate temp fd"),
        );
        // SAFETY: install_fd only calls async-signal-safe dup2/fcntl in the post-fork child.
        unsafe {
            command.pre_exec(move || {
                let _keepalive = &keepalive;
                fn install_fd(src: c_int, dst: c_int) -> io::Result<()> {
                    let rc = if src == dst {
                        unsafe { fcntl(dst, F_SETFD, 0) }
                    } else {
                        unsafe { dup2(src, dst) }
                    };
                    if rc < 0 {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                }
                install_fd(resolver_fd, RESOLVER_FD)?;
                install_fd(config_fd, CONFIG_DIR_FD)?;
                install_fd(temp_fd, TEMP_ROOT_FD)?;
                Ok(())
            });
        }
    }
}

pub(super) fn prepare_wrapper_config(config_dir: &Path) -> io::Result<()> {
    fs_util::private_dir(config_dir)?;

    let config_path = config_dir.join("config.toml");
    let expected = TERMUX_WRAPPER_CONFIG_CONTENT.as_bytes();
    let up_to_date = match fs::symlink_metadata(&config_path) {
        Ok(meta) => {
            if !meta.file_type().is_file() {
                return Err(io::Error::other(
                    "wrapper config.toml path must be a regular non-symlink file",
                ));
            }
            if meta.len() == expected.len() as u64 {
                fs::read(&config_path).is_ok_and(|bytes| bytes == expected)
            } else {
                false
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(err) => return Err(err),
    };

    if !up_to_date {
        let seq = LOCAL_STAGING_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        let tmp_path = config_dir.join(format!(".config.toml.{}.{seq}.tmp", std::process::id()));
        let mut tmp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp_path)?;
        if let Err(err) = tmp.write_all(expected).and_then(|()| tmp.sync_all()) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }
        drop(tmp);
        if let Err(err) = fs::rename(&tmp_path, &config_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err);
        }
        let _ = fs_util::sync_directory(config_dir);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TermuxProcessEnvSnapshot {
    pub(super) wsl_kernel: Option<bool>,
    pub(super) prefix: Option<OsString>,
    pub(super) tmpdir: Option<OsString>,
    pub(super) inherited_path: Option<OsString>,
    pub(super) inherited_ssl_cert_file: Option<OsString>,
    pub(super) inherited_ssl_cert_dir: Option<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TermuxBaseEnvPlan {
    pub(super) assignments: Vec<(OsString, OsString)>,
    pub(super) removals: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TermuxProcessEnvError {
    UnsupportedHost(&'static str),
    MissingEnv(&'static str),
    RelativeEnv(&'static str),
    NulEnv(&'static str),
    InvalidPathComponent(&'static str),
    MissingQualifiedHelper(&'static str),
}

impl std::fmt::Display for TermuxProcessEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermuxProcessEnvError::UnsupportedHost(host) => {
                write!(f, "Termux runtime does not support execution on {host}")
            }
            TermuxProcessEnvError::MissingEnv(name) => {
                write!(f, "required Termux environment variable {name} is missing")
            }
            TermuxProcessEnvError::RelativeEnv(name) => {
                write!(
                    f,
                    "Termux environment variable {name} must be an absolute path"
                )
            }
            TermuxProcessEnvError::NulEnv(name) => {
                write!(f, "Termux environment variable {name} contains NUL")
            }
            TermuxProcessEnvError::InvalidPathComponent(name) => {
                write!(f, "Termux path component '{name}' is invalid")
            }
            TermuxProcessEnvError::MissingQualifiedHelper(id) => {
                write!(f, "qualified generation is missing required helper '{id}'")
            }
        }
    }
}

impl std::error::Error for TermuxProcessEnvError {}

pub(super) fn capture_termux_process_env() -> TermuxProcessEnvSnapshot {
    let wsl_kernel = fs::read("/proc/version").ok().map(|bytes| {
        let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
        lower.windows(9).any(|w| w == b"microsoft") || lower.windows(3).any(|w| w == b"wsl")
    });
    TermuxProcessEnvSnapshot {
        wsl_kernel,
        prefix: std::env::var_os("PREFIX"),
        tmpdir: std::env::var_os("TMPDIR"),
        inherited_path: std::env::var_os("PATH"),
        inherited_ssl_cert_file: std::env::var_os("SSL_CERT_FILE"),
        inherited_ssl_cert_dir: std::env::var_os("SSL_CERT_DIR"),
    }
}

fn required_process_env<'a>(
    value: &'a Option<OsString>,
    name: &'static str,
) -> Result<&'a OsStr, TermuxProcessEnvError> {
    let value = value
        .as_deref()
        .filter(|v| !v.is_empty())
        .ok_or(TermuxProcessEnvError::MissingEnv(name))?;
    if value.as_bytes().contains(&0) {
        return Err(TermuxProcessEnvError::NulEnv(name));
    }
    if !Path::new(value).is_absolute() {
        return Err(TermuxProcessEnvError::RelativeEnv(name));
    }
    Ok(value)
}

fn valid_path_component(value: &OsStr) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty() && !bytes.contains(&0) && !bytes.contains(&b':')
}

fn valid_browser_helper_path(value: &OsStr) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && Path::new(value).is_absolute()
        && !bytes.contains(&0)
        && !bytes.contains(&b':')
        && !bytes.iter().any(|b| b.is_ascii_whitespace())
}

pub(super) fn plan_termux_env(
    snapshot: &TermuxProcessEnvSnapshot,
    browser_helpers: Option<(&OsStr, &OsStr)>,
    cert_file: &OsStr,
    cert_dir: Option<&OsStr>,
) -> Result<TermuxBaseEnvPlan, TermuxProcessEnvError> {
    if snapshot.wsl_kernel == Some(true) {
        return Err(TermuxProcessEnvError::UnsupportedHost("WSL"));
    }
    let prefix = required_process_env(&snapshot.prefix, "PREFIX")?;
    let temp_dir = required_process_env(&snapshot.tmpdir, "TMPDIR")?;
    let prefix_bin = PathBuf::from(prefix).join("bin");
    if !valid_path_component(prefix_bin.as_os_str()) {
        return Err(TermuxProcessEnvError::InvalidPathComponent("prefix_bin"));
    }
    if let Some((open, manual)) = browser_helpers {
        if !valid_browser_helper_path(open) {
            return Err(TermuxProcessEnvError::InvalidPathComponent(
                "browser_open_helper",
            ));
        }
        if !valid_browser_helper_path(manual) {
            return Err(TermuxProcessEnvError::InvalidPathComponent(
                "browser_manual_helper",
            ));
        }
    }

    // Keep `$PREFIX/bin` ahead of inherited PATH so bare `codex` always enters
    // the Wrapper (even when inheriting a stale generation directory), without
    // adding the active generation root (upstream resolves `bin/*`,
    // `codex-path/`, and `codex-resources/` from its own executable path).
    let prefix_bin_bytes = prefix_bin.as_os_str().as_bytes();
    let inherited_path = snapshot.inherited_path.as_deref().unwrap_or_default();
    let mut path = Vec::with_capacity(prefix_bin_bytes.len() + inherited_path.as_bytes().len() + 1);
    path.extend_from_slice(prefix_bin_bytes);
    if !inherited_path.is_empty() {
        for segment in inherited_path.as_bytes().split(|&b| b == b':') {
            if segment == prefix_bin_bytes {
                continue;
            }
            path.push(b':');
            path.extend_from_slice(segment);
        }
    }

    let mut assignments = Vec::with_capacity(11);
    for name in ["TMPDIR", "TMP", "TEMP", "SQLITE_TMPDIR"] {
        assignments.push((OsString::from(name), temp_dir.to_os_string()));
    }
    assignments.push((
        OsString::from("SSL_CERT_FILE"),
        snapshot
            .inherited_ssl_cert_file
            .as_deref()
            .filter(|v| !v.is_empty())
            .unwrap_or(cert_file)
            .to_os_string(),
    ));
    if let Some(value) = snapshot
        .inherited_ssl_cert_dir
        .as_deref()
        .filter(|v| !v.is_empty())
        .or_else(|| cert_dir.filter(|v| !v.is_empty()))
    {
        assignments.push((OsString::from("SSL_CERT_DIR"), value.to_os_string()));
    }
    assignments.push((OsString::from("PATH"), OsString::from_vec(path)));

    if let Some((open, manual)) = browser_helpers {
        let opener = prefix_bin.join("termux-open-url");
        let mut browser = Vec::with_capacity(open.as_bytes().len() + manual.as_bytes().len() + 9);
        browser.extend_from_slice(open.as_bytes());
        browser.extend_from_slice(b" %s:");
        browser.extend_from_slice(manual.as_bytes());
        browser.extend_from_slice(b" %s");
        assignments.push((OsString::from("BROWSER"), OsString::from_vec(browser)));
        assignments.push((
            OsString::from("CODEX_TERMUX_URL_OPENER"),
            opener.into_os_string(),
        ));
    }

    // Fence X11/Wayland/WSL GUI and distro discovery in Android/Termux children.
    assignments.push((OsString::from("DISPLAY"), OsString::new()));
    assignments.push((
        OsString::from("WAYLAND_DISPLAY"),
        OsString::from("/__codex_termux_unavailable__"),
    ));

    Ok(TermuxBaseEnvPlan {
        assignments,
        removals: vec![
            OsString::from("WSL_DISTRO_NAME"),
            OsString::from("WSL_INTEROP"),
        ],
    })
}

pub(super) fn apply_child_env_plan_and_fence(
    cmd: &mut Command,
    env_plan: Option<&TermuxBaseEnvPlan>,
) {
    if let Some(plan) = env_plan {
        for (k, v) in &plan.assignments {
            cmd.env(k, v);
        }
        for name in &plan.removals {
            cmd.env_remove(name);
        }
    }
    cmd.env_remove("CODEX_MANAGED_BY_NPM")
        .env_remove("CODEX_MANAGED_BY_BUN")
        .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
        .env_remove("LD_PRELOAD")
        .env_remove("LD_LIBRARY_PATH");
}

pub(super) fn resolve_codex_home() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("CODEX_HOME").filter(|v| !v.is_empty()) {
        let path = PathBuf::from(value);
        return if path.is_absolute() {
            Some(path)
        } else {
            std::env::current_dir().ok().map(|cwd| cwd.join(path))
        };
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| PathBuf::from(home).join(".codex"))
}

fn exec_runtime_with_sources(
    program: &OsStr,
    args: &[OsString],
    runtime_fds: RuntimeFdSources,
    env_plan: &TermuxBaseEnvPlan,
    sqlite_home_override: Option<&Path>,
    help_or_version: bool,
) -> io::Error {
    let mut command = Command::new(program);
    command.args(args);
    apply_child_env_plan_and_fence(&mut command, Some(env_plan));
    if !help_or_version {
        timezone::configure(&mut command);
    }
    if let Some(sqlite_home) = sqlite_home_override {
        command.env("CODEX_SQLITE_HOME", sqlite_home);
    }
    runtime_fds.configure(&mut command);
    command.exec()
}

struct UpstreamRuntimeHost<'a> {
    cert_file: &'a OsStr,
    cert_dir: Option<&'a OsStr>,
    resolver_path: &'a Path,
    config_dir: &'a Path,
    generation_root: &'a Path,
}

pub(super) fn prepare_shared_home_from_env() -> io::Result<Option<PathBuf>> {
    shared_home_from_env(crate::shared_home::prepare_shared_home)
}

pub(super) fn inspect_shared_home_from_env() -> io::Result<Option<PathBuf>> {
    shared_home_from_env(crate::shared_home::inspect_shared_home)
}

fn shared_home_from_env(
    handle_binding: impl FnOnce(&Path, &OsStr) -> io::Result<Option<PathBuf>>,
) -> io::Result<Option<PathBuf>> {
    let Some(codex_home) = resolve_codex_home() else {
        return Ok(None);
    };
    let Some(shared_home) = std::env::var_os("CODEX_SHARED_HOME").filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    match handle_binding(&codex_home, &shared_home)? {
        Some(shared_dir) if std::env::var_os("CODEX_SQLITE_HOME").is_none_or(|v| v.is_empty()) => {
            Ok(Some(shared_dir))
        }
        _ => Ok(None),
    }
}

fn launch_upstream_runtime_with_plan(
    assets: QualifiedRuntimeAssets<'_, '_>,
    process_env: &TermuxProcessEnvSnapshot,
    host: UpstreamRuntimeHost<'_>,
    args: &[OsString],
    scan: &UpstreamArgScan,
) -> RuntimeLaunchError {
    let selection = assets.selection();
    let browser_helpers = match termux_browser_helper_paths(selection) {
        Ok(paths) => paths,
        Err(err) => return RuntimeLaunchError::Environment(err),
    };
    let env_plan =
        match plan_termux_env(process_env, browser_helpers, host.cert_file, host.cert_dir) {
            Ok(plan) => plan,
            Err(err) => return RuntimeLaunchError::Environment(err),
        };

    let mut sqlite_home_override = None;
    if !scan.help_or_version {
        match prepare_shared_home_from_env() {
            Ok(override_dir) => sqlite_home_override = override_dir,
            Err(err) => return RuntimeLaunchError::Config(err),
        }
        if let Some(codex_home) = resolve_codex_home() {
            if let Err(err) = daemon_settings::ensure(&codex_home) {
                if scan.upstream_invocation_requires_daemon_settings {
                    return RuntimeLaunchError::Config(err);
                }
                eprintln!(
                    "codex: warning: daemon settings unavailable ({err}); continuing non-daemon command"
                );
            } else if scan.upstream_invocation_initializes_managed_daemon {
                if let Some(standalone_root) = host.generation_root.parent() {
                    let standalone_current = standalone_root.join("current");
                    if let Err(err) = daemon_settings::ensure_default_package_link(
                        &codex_home,
                        &standalone_current,
                        standalone_root,
                    ) {
                        return RuntimeLaunchError::Config(err);
                    }
                }
            }
        }
    }
    if let Err(err) = prepare_wrapper_config(host.config_dir) {
        return RuntimeLaunchError::Config(err);
    }
    let runtime_fds = match RuntimeFdSources::open(host.resolver_path, host.config_dir) {
        Ok(fds) => fds,
        Err(err) => return RuntimeLaunchError::Exec(err),
    };
    let exec_err = exec_runtime_with_sources(
        selection.runtime.program_path,
        args,
        runtime_fds,
        &env_plan,
        sqlite_home_override.as_deref(),
        scan.help_or_version,
    );
    RuntimeLaunchError::Exec(exec_err)
}

pub(super) fn launch_upstream_runtime(
    assets: QualifiedRuntimeAssets<'_, '_>,
    process_env: &TermuxProcessEnvSnapshot,
    roots: &LocalWrapperRoots,
    args: &[OsString],
    scan: &UpstreamArgScan,
) -> RuntimeLaunchError {
    launch_upstream_runtime_with_plan(
        assets,
        process_env,
        UpstreamRuntimeHost {
            cert_file: roots.cert_file.as_os_str(),
            cert_dir: Some(roots.cert_dir.as_os_str()),
            resolver_path: &roots.resolver_path,
            config_dir: &roots.config_dir,
            generation_root: &roots.generation_root,
        },
        args,
        scan,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{make_test_roots, remove_temp_root, temp_root, EnvRestoreGuard};
    use std::io::Read;

    #[test]
    fn resolver_fd33_cache_hits_lock_free_on_matching_bytes_and_updates_when_stale() {
        let root = temp_root("fd33");
        let _env_guard = EnvRestoreGuard::new(&[("HOME", Some(&root))]);
        fs::create_dir_all(root.join(".local/share/codex")).unwrap();
        fs_util::private_dir(&root.join(".local/share/codex/wrapper")).unwrap();

        let f1 = open_synthesized_resolver(b"nameserver 8.8.8.8\n", true).unwrap();
        let ino1 = f1.metadata().unwrap().ino();
        let mut s1 = String::new();
        Read::read_to_string(&mut &f1, &mut s1).unwrap();
        assert_eq!(s1, "nameserver 8.8.8.8\noptions timeout:2 attempts:10\n");
        drop(f1);

        let f2 = open_synthesized_resolver(b"nameserver 8.8.8.8\n", true).unwrap();
        assert_eq!(f2.metadata().unwrap().ino(), ino1);
        drop(f2);

        let f3 = open_synthesized_resolver(b"nameserver 1.1.1.1\n", true).unwrap();
        assert_ne!(f3.metadata().unwrap().ino(), ino1);
        let mut old_snapshot = String::new();
        let old_fd = OpenOptions::new()
            .read(true)
            .custom_flags(fs_util::O_NOFOLLOW | O_NONBLOCK_LINUX)
            .open(root.join(".local/share/codex/wrapper/resolv.conf"))
            .unwrap();
        Read::read_to_string(&mut &old_fd, &mut old_snapshot).unwrap();
        assert!(old_snapshot.contains("nameserver 1.1.1.1"));
        let mut s3 = String::new();
        Read::read_to_string(&mut &f3, &mut s3).unwrap();
        assert_eq!(s3, "nameserver 1.1.1.1\noptions timeout:2 attempts:10\n");

        drop(_env_guard);
        remove_temp_root(root);
    }

    #[test]
    fn wrapper_config_writes_managed_toml_and_preserves_other_files() {
        let root = temp_root("fd34");
        let expected = "# codex-termux-wrapper\nsandbox_mode = \"danger-full-access\"\nmcp_oauth_credentials_store = \"file\"\n";
        for (name, old) in [
            ("fresh", None),
            (
                "legacy",
                Some(format!("{expected}thread_unload_delay_secs = 0\n")),
            ),
            ("stale", Some("# stale\nfoo = 1\n".to_owned())),
        ] {
            let cfg = root.join(name);
            let path = cfg.join("config.toml");
            let requirements = cfg.join("requirements.toml");
            if let Some(old) = old {
                fs_util::private_dir(&cfg).unwrap();
                fs::write(&path, old).unwrap();
                fs::write(&requirements, b"user_setting = true\n").unwrap();
            }

            prepare_wrapper_config(&cfg).unwrap();
            assert_eq!(fs::read_to_string(&path).unwrap(), expected);
            let first = fs::metadata(&path).unwrap();
            prepare_wrapper_config(&cfg).unwrap();
            let second = fs::metadata(&path).unwrap();
            assert_eq!(first.ino(), second.ino());
            assert_eq!(first.modified().unwrap(), second.modified().unwrap());
            if name != "fresh" {
                assert_eq!(
                    fs::read_to_string(requirements).unwrap(),
                    "user_setting = true\n"
                );
            }
        }
        remove_temp_root(root);
    }

    #[test]
    fn runtime_fd_sources_move_descriptors_above_safe_min_and_install_33_34_35() {
        let _guard = EnvRestoreGuard::new(&[]);
        let root = temp_root("fd-install");
        let roots = make_test_roots(&root);
        let sources = RuntimeFdSources::open(&roots.resolver_path, &roots.config_dir).unwrap();
        let sh = if Path::new("/system/bin/sh").is_file() {
            "/system/bin/sh"
        } else {
            "/bin/sh"
        };
        let mut cmd = Command::new(sh);
        cmd.args([
            "-c",
            "test -d /proc/$$/fd/34 && test -d /proc/$$/fd/35 && exec cat /proc/$$/fd/33",
        ]);
        sources.configure(&mut cmd);
        let out = cmd.output().unwrap();
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "nameserver 1.1.1.1\noptions timeout:2 attempts:10\n"
        );
        remove_temp_root(root);
    }

    #[test]
    fn plan_termux_env_sets_certs_temp_browser_and_gui_fence() {
        let snap = TermuxProcessEnvSnapshot {
            wsl_kernel: Some(false),
            prefix: Some(OsString::from("/data/data/com.termux/files/usr")),
            tmpdir: Some(OsString::from("/data/data/com.termux/files/usr/tmp")),
            inherited_path: Some(OsString::from("/system/bin")),
            inherited_ssl_cert_file: None,
            inherited_ssl_cert_dir: None,
        };
        let plan = plan_termux_env(
            &snap,
            Some((
                OsStr::new("/gen/browser/open/curl"),
                OsStr::new("/gen/browser/manual/curl"),
            )),
            OsStr::new("/data/data/com.termux/files/usr/etc/tls/cert.pem"),
            Some(OsStr::new("/data/data/com.termux/files/usr/etc/tls/certs")),
        )
        .unwrap();
        let get = |k: &str| {
            plan.assignments
                .iter()
                .find(|(ak, _)| ak == k)
                .map(|(_, av)| av.to_string_lossy().into_owned())
        };
        assert_eq!(
            get("PATH").as_deref(),
            Some("/data/data/com.termux/files/usr/bin:/system/bin")
        );
        assert_eq!(
            get("TMPDIR").as_deref(),
            Some("/data/data/com.termux/files/usr/tmp")
        );
        assert_eq!(
            get("SQLITE_TMPDIR").as_deref(),
            Some("/data/data/com.termux/files/usr/tmp")
        );
        assert_eq!(get("DISPLAY").as_deref(), Some(""));
        assert_eq!(
            get("WAYLAND_DISPLAY").as_deref(),
            Some("/__codex_termux_unavailable__")
        );
        assert!(get("BROWSER")
            .unwrap()
            .contains("/gen/browser/open/curl %s:"));
        assert!(plan.removals.contains(&OsString::from("WSL_DISTRO_NAME")));
    }

    #[test]
    fn plan_termux_env_places_prefix_bin_first_and_deduplicates_without_generation_root() {
        let cases: &[(Option<&[u8]>, &[u8])] = &[
            (None, b"/prefix/bin"),
            (Some(b""), b"/prefix/bin"),
            (Some(b"/prefix/bin"), b"/prefix/bin"),
            (
                Some(b"/prefix/bin:/system/bin"),
                b"/prefix/bin:/system/bin",
            ),
            (
                Some(b"/home/.codex/packages/standalone/releases/0.170.0-3f7e1a:/prefix/bin:/system/bin"),
                b"/prefix/bin:/home/.codex/packages/standalone/releases/0.170.0-3f7e1a:/system/bin",
            ),
            (
                Some(b"/prefix/bin:/custom/\xff/bin:/prefix/bin:/system/bin"),
                b"/prefix/bin:/custom/\xff/bin:/system/bin",
            ),
        ];
        for (inherited, expected) in cases {
            let snap = TermuxProcessEnvSnapshot {
                wsl_kernel: Some(false),
                prefix: Some(OsString::from("/prefix")),
                tmpdir: Some(OsString::from("/prefix/tmp")),
                inherited_path: inherited.map(|b| OsString::from_vec(b.to_vec())),
                inherited_ssl_cert_file: None,
                inherited_ssl_cert_dir: None,
            };
            let plan =
                plan_termux_env(&snap, None, OsStr::new("/prefix/etc/tls/cert.pem"), None).unwrap();
            let path = plan
                .assignments
                .iter()
                .find(|(k, _)| k == "PATH")
                .map(|(_, v)| v.as_os_str().as_bytes())
                .unwrap();
            assert_eq!(path, *expected);
        }
    }
}
