#[cfg(unix)]
mod daemon_settings;
#[cfg(unix)]
mod doctor;
#[cfg(unix)]
mod extensions;
#[cfg(unix)]
mod fs_util;
#[cfg(unix)]
mod maintenance;
#[cfg(unix)]
mod remote_download;
#[cfg(unix)]
mod state;
#[cfg(unix)]
mod timezone;
#[cfg(unix)]
mod update;

#[cfg(unix)]
use state::generation_state;

#[cfg(unix)]
mod unix_app {
    use super::{
        daemon_settings, doctor, extensions, fs_util, generation_state, maintenance, timezone,
        update,
    };
    use std::ffi::{OsStr, OsString};
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, Read as _, Seek as _, Write as _};
    use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
    use std::os::raw::c_int;
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};
    use std::os::unix::fs::{
        DirBuilderExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
    };
    use std::os::unix::process::CommandExt as _;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub const RESOLVER_FD: c_int = 33;
    pub const CONFIG_DIR_FD: c_int = 34;
    pub const TEMP_ROOT_FD: c_int = 35;

    pub(super) const SAFE_MIN_FD: c_int = 64;
    pub(super) const F_DUPFD_CLOEXEC: c_int = 1030;
    const F_SETFD: c_int = 2;
    pub(super) const FLOCK_EX: c_int = 2;
    pub(super) const FLOCK_NB: c_int = 4;
    pub(super) const FLOCK_UN: c_int = 8;

    pub(super) const DOCTOR_OUTPUT_MAX_BYTES: usize = 64 * 1024;
    pub(super) const DEFAULT_UPSTREAM_LATEST_URL: &str =
        "https://api.github.com/repos/openai/codex/releases/latest";
    pub(super) const UPSTREAM_RELEASE_METADATA_BASE: &str =
        "https://releases.openai.com/codex/releases";
    pub(super) const UPSTREAM_PACKAGE_ASSET: &str =
        "codex-package-aarch64-unknown-linux-musl.tar.gz";
    pub(super) const UPSTREAM_RELEASE_METADATA_MAX_BYTES: usize = 1024 * 1024;
    pub(super) const REMOTE_RELEASE_URL_MAX_BYTES: usize = 4096;
    pub(super) const REMOTE_RELEASE_TOTAL_MAX_BYTES: u64 = 1024 * 1024 * 1024;
    pub(super) const REMOTE_CONNECT_TIMEOUT_SECONDS: &str = "15";
    pub(super) const REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS: &str = "30";
    pub(super) const REMOTE_TRANSFER_TIMEOUT_SECONDS: &str = "1800";

    const LOCAL_GENERATION_FORMAT: &str = "codex-local-generation";
    const WRAPPER_RUNTIME_PATH: &str = "bin/codex";
    const WRAPPER_CODE_MODE_HOST_PATH: &str = "bin/codex-code-mode-host";
    const WRAPPER_EXEC_SERVER_PATH: &str = "bin/codex-exec-server";
    const WRAPPER_PACKAGE_JSON_PATH: &str = "codex-package.json";

    const WRAPPER_API_IDENTITY: &str = "wrapper-api";
    const PERSISTENT_SCHEMA_IDENTITY: &str = "schema";
    const EXPECTED_PLATFORM: &str = "android";
    const EXPECTED_ARCHITECTURE: &str = "aarch64";

    const TERMUX_BROWSER_OPEN_HELPER_IDENTITY: &str = "termux-browser-open";
    const TERMUX_BROWSER_MANUAL_HELPER_IDENTITY: &str = "termux-browser-manual";

    const LOCAL_GENERATION_MAX_BYTES: usize = 256 * 1024;
    const LOCAL_RELEASE_MAX_FILES: usize = 64;
    const RESOLVER_SYNTHESIS_MAX_BYTES: usize = 64 * 1024;
    const RESOLVER_DEFAULT_OPTIONS: &[u8] = b"options timeout:2 attempts:10\n";

    // FD 34 configuration loaded by upstream Codex via inherited dirfd.
    const TERMUX_WRAPPER_CONFIG_CONTENT: &str =
        "# codex-termux-wrapper\nsandbox_mode = \"danger-full-access\"\nmcp_oauth_credentials_store = \"file\"\n";

    pub(super) static LOCAL_STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);
    static RESOLVER_SYNTHESIS_COUNTER: AtomicU64 = AtomicU64::new(0);

    unsafe extern "C" {
        fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        pub(super) fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
        pub(super) fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum PublicDispatchRoute {
        Update(Vec<OsString>),
        Doctor(Vec<OsString>),
        Wrapper(Vec<OsString>),
        Upstream(Vec<OsString>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum DispatchPlanError {
        UnsupportedSandboxMode(String),
        UnsupportedSandboxCommand,
    }

    impl std::fmt::Display for DispatchPlanError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DispatchPlanError::UnsupportedSandboxMode(mode) => write!(
                    f,
                    "sandbox mode '{mode}' is unsupported on Termux because Android kernels lack Linux Landlock/user-namespace isolation; use 'danger-full-access' (the wrapper default)"
                ),
                DispatchPlanError::UnsupportedSandboxCommand => f.write_str(
                    "'codex sandbox linux' is unsupported on Termux because Android kernels lack Linux Landlock/seccomp-user-namespace sandboxing",
                ),
            }
        }
    }

    impl std::error::Error for DispatchPlanError {}

    fn upstream_global_option_takes_separate_value(value: &str) -> bool {
        matches!(
            value,
            "-c" | "--config"
                | "--enable"
                | "--disable"
                | "-s"
                | "--sandbox"
                | "-a"
                | "--ask-for-approval"
                | "-m"
                | "--model"
                | "-p"
                | "--profile"
                | "-C"
                | "--cd"
                | "-i"
                | "--image"
                | "--local-provider"
                | "--oss"
                | "--search"
                | "--add-dir"
        )
    }

    fn upstream_dispatch_global_option_takes_separate_value(value: &str) -> bool {
        matches!(value, "-c" | "--config" | "--enable" | "--disable")
    }

    fn unsupported_sandbox_mode(value: &str) -> Option<&str> {
        match value {
            "read-only" | "workspace-write" => Some(value),
            _ => None,
        }
    }

    pub(super) fn plan_passthrough_args(args: &[OsString]) -> Result<(), DispatchPlanError> {
        let mut positionals: Vec<&str> = Vec::with_capacity(2);
        let mut index = 0usize;
        while index < args.len() {
            let Some(arg) = args[index].to_str() else {
                index += 1;
                continue;
            };
            if arg == "--" {
                break;
            }
            if let Some(value) = arg
                .strip_prefix("--sandbox=")
                .or_else(|| arg.strip_prefix("-s="))
            {
                if let Some(mode) = unsupported_sandbox_mode(value) {
                    return Err(DispatchPlanError::UnsupportedSandboxMode(mode.to_owned()));
                }
                index += 1;
                continue;
            }
            if matches!(arg, "--sandbox" | "-s") {
                if let Some(next) = args.get(index + 1).and_then(|v| v.to_str()) {
                    if let Some(mode) = unsupported_sandbox_mode(next) {
                        return Err(DispatchPlanError::UnsupportedSandboxMode(mode.to_owned()));
                    }
                    index += 2;
                    continue;
                }
            }
            if upstream_global_option_takes_separate_value(arg) {
                index += 2;
                continue;
            }
            if arg.starts_with('-') {
                index += 1;
                continue;
            }
            if positionals.len() < 2 {
                positionals.push(arg);
            }
            index += 1;
        }
        if matches!(positionals.as_slice(), ["sandbox", "linux"]) {
            return Err(DispatchPlanError::UnsupportedSandboxCommand);
        }
        Ok(())
    }

    pub(super) fn plan_public_dispatch(
        args: Vec<OsString>,
    ) -> Result<PublicDispatchRoute, DispatchPlanError> {
        let mut index = 0usize;
        while index < args.len() {
            let Some(arg) = args[index].to_str() else {
                break;
            };
            if arg == "--" {
                break;
            }
            if upstream_dispatch_global_option_takes_separate_value(arg) {
                if index + 1 >= args.len() {
                    break;
                }
                index += 2;
                continue;
            }
            if arg.starts_with('-') {
                index += 1;
                continue;
            }
            if index == 0 {
                match arg {
                    "update" => return Ok(PublicDispatchRoute::Update(args[1..].to_vec())),
                    "doctor" => return Ok(PublicDispatchRoute::Doctor(args[1..].to_vec())),
                    "wrapper" => return Ok(PublicDispatchRoute::Wrapper(args[1..].to_vec())),
                    _ => {}
                }
            }
            break;
        }
        plan_passthrough_args(&args)?;
        Ok(PublicDispatchRoute::Upstream(args))
    }

    pub(super) fn is_help_or_version_invocation(args: &[OsString]) -> bool {
        let mut index = 0usize;
        while index < args.len() {
            let Some(arg) = args[index].to_str() else {
                return false;
            };
            if arg == "--" {
                return false;
            }
            if upstream_global_option_takes_separate_value(arg) {
                index += 2;
                continue;
            }
            if matches!(arg, "--version" | "-V" | "--help" | "-h" | "help") {
                return true;
            }
            if arg.starts_with('-') {
                index += 1;
                continue;
            }
            return false;
        }
        false
    }

    pub(super) struct RuntimeFdSources {
        resolver: OwnedFd,
        config_dir: OwnedFd,
        temporary_root: OwnedFd,
    }

    fn dup_above_safe_min(fd: c_int) -> io::Result<OwnedFd> {
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
        let mut ro = match File::open(path) {
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

    pub(super) fn open_synthesized_resolver(bytes: &[u8]) -> io::Result<File> {
        let mut synthesized = Vec::with_capacity(bytes.len() + RESOLVER_DEFAULT_OPTIONS.len() + 1);
        synthesized.extend_from_slice(bytes);
        if !synthesized.is_empty() && !synthesized.ends_with(b"\n") {
            synthesized.push(b'\n');
        }
        synthesized.extend_from_slice(RESOLVER_DEFAULT_OPTIONS);

        if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
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

                if let Ok(existing_meta) = fs::symlink_metadata(&cache_path) {
                    if !existing_meta.file_type().is_file() {
                        return Err(io::Error::other(
                            "cached resolver path must be a regular non-symlink file",
                        ));
                    }
                }
                let mut rw = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .mode(0o644)
                    .open(&cache_path)?;
                let rw_meta = rw.metadata()?;
                let sym_meta = fs::symlink_metadata(&cache_path)?;
                if !sym_meta.file_type().is_file()
                    || sym_meta.dev() != rw_meta.dev()
                    || sym_meta.ino() != rw_meta.ino()
                {
                    return Err(io::Error::other("cached resolver file changed during open"));
                }
                if unsafe { flock(rw.as_raw_fd(), FLOCK_EX) } != 0 {
                    return Err(io::Error::last_os_error());
                }
                let mut existing = Vec::new();
                io::Read::by_ref(&mut rw)
                    .take(RESOLVER_SYNTHESIS_MAX_BYTES as u64 + 1)
                    .read_to_end(&mut existing)?;
                if existing != synthesized {
                    // Rewrite in-place (never rename) so cached resolv.conf preserves its inode.
                    rw.seek(io::SeekFrom::Start(0))?;
                    rw.write_all(&synthesized)?;
                    rw.set_len(synthesized.len() as u64)?;
                    rw.sync_all()?;
                }
                let ro = File::open(&cache_path)?;
                let ro_meta = ro.metadata()?;
                if ro_meta.dev() != rw_meta.dev() || ro_meta.ino() != rw_meta.ino() {
                    return Err(io::Error::other(
                        "cached resolver inode changed while syncing",
                    ));
                }
                drop(rw);
                return Ok(ro);
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
            let mut source_resolver = File::open(resolver_path)?;
            if source_resolver.metadata()?.is_dir() {
                return Err(io::Error::from_raw_os_error(21));
            }
            let mut resolver_bytes = Vec::new();
            io::Read::by_ref(&mut source_resolver)
                .take(RESOLVER_SYNTHESIS_MAX_BYTES as u64 + 1)
                .read_to_end(&mut resolver_bytes)?;
            let resolver_file = if resolver_bytes.len() <= RESOLVER_SYNTHESIS_MAX_BYTES
                && !resolver_has_options_directive(&resolver_bytes)
            {
                open_synthesized_resolver(&resolver_bytes)?
            } else {
                source_resolver.seek(io::SeekFrom::Start(0))?;
                source_resolver
            };
            let config = File::open(config_dir)?;
            if !config.metadata()?.is_dir() {
                return Err(io::Error::from_raw_os_error(20));
            }
            let temporary = File::open(std::env::temp_dir())?;
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
            // SAFETY: install_fd only calls async-signal-safe dup2/fcntl in the post-fork child.
            unsafe {
                command.pre_exec(move || {
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
            let tmp_path =
                config_dir.join(format!(".config.toml.{}.{seq}.tmp", std::process::id()));
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
            if let Ok(dir) = File::open(config_dir) {
                let _ = dir.sync_all();
            }
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
        generation_dir: &OsStr,
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
        if !valid_path_component(generation_dir) {
            return Err(TermuxProcessEnvError::InvalidPathComponent(
                "generation_dir",
            ));
        }
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

        let inherited_path = snapshot.inherited_path.as_deref().unwrap_or_default();
        let mut path = Vec::with_capacity(
            generation_dir.as_bytes().len()
                + prefix_bin.as_os_str().as_bytes().len()
                + inherited_path.as_bytes().len()
                + 2,
        );
        path.extend_from_slice(generation_dir.as_bytes());
        path.push(b':');
        path.extend_from_slice(prefix_bin.as_os_str().as_bytes());
        if !inherited_path.is_empty() {
            path.push(b':');
            path.extend_from_slice(inherited_path.as_bytes());
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
            let mut browser =
                Vec::with_capacity(open.as_bytes().len() + manual.as_bytes().len() + 9);
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
            timezone::configure(cmd);
        }
        cmd.env_remove("CODEX_MANAGED_BY_NPM")
            .env_remove("CODEX_MANAGED_BY_BUN")
            .env_remove("CODEX_MANAGED_PACKAGE_ROOT")
            .env_remove("LD_PRELOAD")
            .env_remove("LD_LIBRARY_PATH");
    }

    fn resolve_codex_home() -> Option<PathBuf> {
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum GenerationLayout {
        PackageRoot,
    }

    impl GenerationLayout {
        pub(super) fn as_str(self) -> &'static str {
            match self {
                GenerationLayout::PackageRoot => "package-root",
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct GenerationHelperDigest {
        pub(super) identity: String,
        pub(super) digest: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct GenerationManifest {
        pub(super) upstream_package_identity: String,
        pub(super) upstream_package_version: String,
        pub(super) source_artifact_digest: String,
        pub(super) expected_platform: String,
        pub(super) expected_architecture: String,
        pub(super) patch_policy_id: String,
        pub(super) patch_report: String,
        pub(super) runtime_digest: String,
        pub(super) helper_digests: Vec<GenerationHelperDigest>,
        pub(super) wrapper_artifact_digest: String,
        pub(super) wrapper_api_identity: String,
        pub(super) persistent_schema_identity: String,
        pub(super) creation_metadata: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum GenerationManifestError {
        EmptyRequired(&'static str),
        PlatformMismatch,
        ArchitectureMismatch,
        WrapperApiMismatch,
        PersistentSchemaMismatch,
        DuplicateHelperIdentity { first: usize, duplicate: usize },
    }

    impl std::fmt::Display for GenerationManifestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                GenerationManifestError::EmptyRequired(name) => {
                    write!(f, "required generation manifest field '{name}' is empty")
                }
                GenerationManifestError::PlatformMismatch => {
                    f.write_str("generation manifest platform is not android")
                }
                GenerationManifestError::ArchitectureMismatch => {
                    f.write_str("generation manifest architecture is not aarch64")
                }
                GenerationManifestError::WrapperApiMismatch => {
                    f.write_str("generation manifest wrapper API identity is incompatible")
                }
                GenerationManifestError::PersistentSchemaMismatch => {
                    f.write_str("generation manifest persistent schema identity is incompatible")
                }
                GenerationManifestError::DuplicateHelperIdentity { first, duplicate } => write!(
                    f,
                    "generation manifest helper {duplicate} duplicates helper {first}"
                ),
            }
        }
    }

    impl std::error::Error for GenerationManifestError {}

    fn qualify_generation_manifest(
        manifest: &GenerationManifest,
    ) -> Result<(), GenerationManifestError> {
        for (name, value) in [
            (
                "upstream_package_identity",
                &manifest.upstream_package_identity,
            ),
            (
                "upstream_package_version",
                &manifest.upstream_package_version,
            ),
            ("source_artifact_digest", &manifest.source_artifact_digest),
            ("patch_policy_id", &manifest.patch_policy_id),
            ("patch_report", &manifest.patch_report),
            ("runtime_digest", &manifest.runtime_digest),
            ("wrapper_artifact_digest", &manifest.wrapper_artifact_digest),
            ("creation_metadata", &manifest.creation_metadata),
        ] {
            if value.is_empty() {
                return Err(GenerationManifestError::EmptyRequired(name));
            }
        }
        if manifest.expected_platform != EXPECTED_PLATFORM {
            return Err(GenerationManifestError::PlatformMismatch);
        }
        if manifest.expected_architecture != EXPECTED_ARCHITECTURE {
            return Err(GenerationManifestError::ArchitectureMismatch);
        }
        if manifest.wrapper_api_identity != WRAPPER_API_IDENTITY {
            return Err(GenerationManifestError::WrapperApiMismatch);
        }
        if manifest.persistent_schema_identity != PERSISTENT_SCHEMA_IDENTITY {
            return Err(GenerationManifestError::PersistentSchemaMismatch);
        }
        for i in 0..manifest.helper_digests.len() {
            for j in (i + 1)..manifest.helper_digests.len() {
                if manifest.helper_digests[i].identity == manifest.helper_digests[j].identity {
                    return Err(GenerationManifestError::DuplicateHelperIdentity {
                        first: i,
                        duplicate: j,
                    });
                }
            }
        }
        Ok(())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct RuntimeAssetBinding<'a> {
        pub(super) program_path: &'a OsStr,
        pub(super) observed_digest: &'a str,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct HelperAssetBinding<'a> {
        pub(super) identity: &'a str,
        pub(super) asset_path: &'a OsStr,
        pub(super) observed_digest: &'a str,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct RuntimeAssetSelection<'a> {
        pub(super) runtime: RuntimeAssetBinding<'a>,
        pub(super) generation_dir: &'a OsStr,
        pub(super) helpers: &'a [HelperAssetBinding<'a>],
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum RuntimeAssetError {
        EmptyPath(&'static str),
        RelativePath(&'static str),
        NulPath(&'static str),
        InvalidGenerationDir,
        EmptyRuntimeDigest,
        DuplicateHelperIdentity { first: usize, duplicate: usize },
    }

    impl std::fmt::Display for RuntimeAssetError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RuntimeAssetError::EmptyPath(name) => {
                    write!(f, "runtime asset path '{name}' is empty")
                }
                RuntimeAssetError::RelativePath(name) => {
                    write!(f, "runtime asset path '{name}' must be absolute")
                }
                RuntimeAssetError::NulPath(name) => {
                    write!(f, "runtime asset path '{name}' must not contain NUL")
                }
                RuntimeAssetError::InvalidGenerationDir => {
                    f.write_str("generation directory is not a valid PATH component")
                }
                RuntimeAssetError::EmptyRuntimeDigest => {
                    f.write_str("runtime asset digest is empty")
                }
                RuntimeAssetError::DuplicateHelperIdentity { first, duplicate } => write!(
                    f,
                    "selected helper asset {duplicate} duplicates selected helper asset {first}"
                ),
            }
        }
    }

    impl std::error::Error for RuntimeAssetError {}

    #[derive(Debug, Clone, Copy)]
    pub(super) struct QualifiedRuntimeAssets<'selection, 'asset> {
        selection: &'selection RuntimeAssetSelection<'asset>,
    }

    impl<'selection, 'asset> QualifiedRuntimeAssets<'selection, 'asset> {
        pub(super) fn selection(self) -> &'selection RuntimeAssetSelection<'asset> {
            self.selection
        }
    }

    pub(super) fn termux_browser_helper_paths<'asset>(
        selection: &RuntimeAssetSelection<'asset>,
    ) -> Result<Option<(&'asset OsStr, &'asset OsStr)>, TermuxProcessEnvError> {
        let find = |identity: &'static str| {
            selection
                .helpers
                .iter()
                .find(|h| h.identity == identity)
                .map(|h| h.asset_path)
        };
        match (
            find(TERMUX_BROWSER_OPEN_HELPER_IDENTITY),
            find(TERMUX_BROWSER_MANUAL_HELPER_IDENTITY),
        ) {
            (None, None) => Ok(None),
            (Some(open), Some(manual)) => Ok(Some((open, manual))),
            (None, Some(_)) => Err(TermuxProcessEnvError::MissingQualifiedHelper(
                TERMUX_BROWSER_OPEN_HELPER_IDENTITY,
            )),
            (Some(_), None) => Err(TermuxProcessEnvError::MissingQualifiedHelper(
                TERMUX_BROWSER_MANUAL_HELPER_IDENTITY,
            )),
        }
    }

    fn validate_absolute_runtime_asset_path(
        path: &OsStr,
        name: &'static str,
    ) -> Result<(), RuntimeAssetError> {
        if path.is_empty() {
            return Err(RuntimeAssetError::EmptyPath(name));
        }
        if path.as_bytes().contains(&0) {
            return Err(RuntimeAssetError::NulPath(name));
        }
        if !Path::new(path).is_absolute() {
            return Err(RuntimeAssetError::RelativePath(name));
        }
        Ok(())
    }

    fn qualify_runtime_assets<'selection, 'asset>(
        selection: &'selection RuntimeAssetSelection<'asset>,
    ) -> Result<QualifiedRuntimeAssets<'selection, 'asset>, RuntimeAssetError> {
        validate_absolute_runtime_asset_path(selection.runtime.program_path, "runtime_program")?;
        validate_absolute_runtime_asset_path(selection.generation_dir, "generation_dir")?;
        if !valid_path_component(selection.generation_dir) {
            return Err(RuntimeAssetError::InvalidGenerationDir);
        }
        if selection.runtime.observed_digest.is_empty() {
            return Err(RuntimeAssetError::EmptyRuntimeDigest);
        }
        for i in 0..selection.helpers.len() {
            validate_absolute_runtime_asset_path(selection.helpers[i].asset_path, "helper_asset")?;
            if selection.helpers[i].observed_digest.is_empty() {
                return Err(RuntimeAssetError::EmptyRuntimeDigest);
            }
            for j in (i + 1)..selection.helpers.len() {
                if selection.helpers[i].identity == selection.helpers[j].identity {
                    return Err(RuntimeAssetError::DuplicateHelperIdentity {
                        first: i,
                        duplicate: j,
                    });
                }
            }
        }
        Ok(QualifiedRuntimeAssets { selection })
    }

    #[derive(Debug, Clone)]
    pub(super) struct ReleaseFileEntry {
        #[allow(dead_code)]
        pub(super) relative_path: String,
        pub(super) sha256: String,
        #[allow(dead_code)]
        pub(super) mode: u32,
    }

    #[derive(Debug, Clone)]
    pub(super) struct LocalWrapperRoots {
        pub(super) generation_root: PathBuf,
        pub(super) state_root: PathBuf,
        pub(super) config_dir: PathBuf,
        pub(super) resolver_path: PathBuf,
        pub(super) cert_file: PathBuf,
        pub(super) cert_dir: PathBuf,
        pub(super) openssl: PathBuf,
        pub(super) curl: PathBuf,
    }

    fn required_absolute_env_path(name: &'static str) -> Result<PathBuf, LocalProductError> {
        let value = std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .ok_or(LocalProductError::MissingEnvironment(name))?;
        let path = PathBuf::from(value);
        if !fs_util::wrapper_safe_absolute_path(&path) {
            return Err(LocalProductError::InvalidEnvironmentPath(name));
        }
        Ok(path)
    }

    impl LocalWrapperRoots {
        pub(super) fn from_environment() -> Result<Self, LocalProductError> {
            let home = required_absolute_env_path("HOME")?;
            let prefix = required_absolute_env_path("PREFIX")?;
            let state_root = home.join(".local/share/codex/wrapper");
            Ok(Self {
                generation_root: home.join(".local/lib/codex/wrapper/generations"),
                config_dir: state_root.join("config"),
                state_root,
                resolver_path: prefix.join("etc/resolv.conf"),
                cert_file: prefix.join("etc/tls/cert.pem"),
                cert_dir: prefix.join("etc/tls/certs"),
                openssl: prefix.join("bin/openssl"),
                curl: prefix.join("bin/curl"),
            })
        }
    }

    #[derive(Debug)]
    pub(super) enum LocalProductError {
        MissingEnvironment(&'static str),
        InvalidEnvironmentPath(&'static str),
        StateFormat(generation_state::StateFormatError),
        State(generation_state::ActivationTransactionError),
        NoCurrentGeneration,
        Io {
            operation: &'static str,
            source: io::Error,
        },
        Descriptor(&'static str),
        UnsafeSource(&'static str),
        GenerationCollision,
        OpenSslUnavailable,
        CurlUnavailable,
        OpenSslFailed(&'static str),
        Remote(&'static str),
        RemoteTransportFailed {
            code: Option<i32>,
            http_status: Option<u16>,
            bytes: u64,
            elapsed_seconds: u64,
        },
        RemoteStalled {
            bytes: u64,
            idle_seconds: u64,
        },
        RemoteResponseTooLarge,
        LocalUpdate(&'static str),
        Builder(codex_release_builder::BuilderError),
        ReleaseDigestMismatch,
        CandidateProbe(&'static str),
        Manifest(GenerationManifestError),
        Runtime(RuntimeAssetError),
        Dispatch(PublicDispatchExecutionError),
    }

    impl std::fmt::Display for LocalProductError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LocalProductError::MissingEnvironment(name) => {
                    write!(f, "required environment variable {name} is missing")
                }
                LocalProductError::InvalidEnvironmentPath(name) => {
                    write!(f, "environment variable {name} must be a normalized absolute path")
                }
                LocalProductError::StateFormat(err) => err.fmt(f),
                LocalProductError::State(err) => err.fmt(f),
                LocalProductError::NoCurrentGeneration => f.write_str(
                    "no Codex generation is currently installed; run 'codex update' to install an upstream release",
                ),
                LocalProductError::Io { operation, source } => write!(f, "{operation}: {source}"),
                LocalProductError::Descriptor(msg) => f.write_str(msg),
                LocalProductError::UnsafeSource(msg) => f.write_str(msg),
                LocalProductError::GenerationCollision => {
                    f.write_str("target generation directory already exists")
                }
                LocalProductError::OpenSslUnavailable => {
                    f.write_str("required 'openssl' binary is unavailable in Termux PREFIX")
                }
                LocalProductError::CurlUnavailable => {
                    f.write_str("required 'curl' binary is unavailable in Termux PREFIX")
                }
                LocalProductError::OpenSslFailed(msg) => f.write_str(msg),
                LocalProductError::Remote(msg) => f.write_str(msg),
                LocalProductError::RemoteTransportFailed {
                    code,
                    http_status,
                    bytes,
                    elapsed_seconds,
                } => write!(
                    f,
                    "download failed (exit={}, http={}, bytes={}, elapsed={}s)",
                    code.map_or_else(|| "-".to_owned(), |c| c.to_string()),
                    http_status.map_or_else(|| "-".to_owned(), |s| s.to_string()),
                    bytes,
                    elapsed_seconds
                ),
                LocalProductError::RemoteStalled { bytes, idle_seconds } => write!(
                    f,
                    "download stalled after {bytes} bytes ({idle_seconds}s without progress)"
                ),
                LocalProductError::RemoteResponseTooLarge => {
                    f.write_str("remote response exceeded allowed size bound")
                }
                LocalProductError::LocalUpdate(msg) => f.write_str(msg),
                LocalProductError::Builder(err) => err.fmt(f),
                LocalProductError::ReleaseDigestMismatch => {
                    f.write_str("downloaded release archive SHA-256 digest mismatch")
                }
                LocalProductError::CandidateProbe(msg) => f.write_str(msg),
                LocalProductError::Manifest(err) => err.fmt(f),
                LocalProductError::Runtime(err) => err.fmt(f),
                LocalProductError::Dispatch(err) => err.fmt(f),
            }
        }
    }

    impl std::error::Error for LocalProductError {}

    #[derive(Debug)]
    pub(super) enum RuntimeLaunchError {
        Environment(TermuxProcessEnvError),
        Config(io::Error),
        Exec(io::Error),
    }

    impl std::fmt::Display for RuntimeLaunchError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RuntimeLaunchError::Environment(err) => err.fmt(f),
                RuntimeLaunchError::Config(err) => {
                    write!(f, "failed to prepare Termux config: {err}")
                }
                RuntimeLaunchError::Exec(err) => {
                    write!(f, "failed to exec upstream runtime: {err}")
                }
            }
        }
    }

    impl std::error::Error for RuntimeLaunchError {}

    #[derive(Debug)]
    pub(super) enum PublicDispatchExecutionError {
        Upstream(RuntimeLaunchError),
        Doctor(doctor::LocalDoctorCommandError),
    }

    impl std::fmt::Display for PublicDispatchExecutionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PublicDispatchExecutionError::Upstream(err) => err.fmt(f),
                PublicDispatchExecutionError::Doctor(err) => err.fmt(f),
            }
        }
    }

    impl std::error::Error for PublicDispatchExecutionError {}

    #[derive(Debug, Clone, Copy)]
    pub(super) struct LocalPublicDispatchContext<'context, 'runtime_selection, 'runtime_asset> {
        pub(super) runtime_assets: QualifiedRuntimeAssets<'runtime_selection, 'runtime_asset>,
        pub(super) process_env: &'context TermuxProcessEnvSnapshot,
        pub(super) cert_file: &'context OsStr,
        pub(super) cert_dir: Option<&'context OsStr>,
        pub(super) resolver_path: &'context Path,
        pub(super) config_dir: &'context Path,
        pub(super) wrapper_doctor_status: doctor::WrapperDoctorStatus,
        pub(super) generation_id: &'context str,
        pub(super) generation_layout: GenerationLayout,
        pub(super) has_code_mode_host: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum PublicDispatchCompletion {
        Doctor(doctor::DoctorCommandOutcome),
    }

    #[derive(Debug, Clone)]
    pub(super) struct LoadedLocalGeneration {
        pub(super) generation_id: String,
        pub(super) manifest: GenerationManifest,
        pub(super) generation_layout: GenerationLayout,
        pub(super) runtime_path: PathBuf,
        pub(super) code_mode_host_path: Option<PathBuf>,
        pub(super) generation_dir: PathBuf,
        pub(super) helper_paths: Vec<PathBuf>,
    }

    fn valid_nonnegative_decimal(value: &str) -> bool {
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
    }

    fn descriptor_field<'a>(
        line: Option<&'a str>,
        expected: &'static str,
    ) -> Result<&'a str, LocalProductError> {
        let line = line.ok_or(LocalProductError::Descriptor(
            "generation descriptor is incomplete",
        ))?;
        let (name, value) = line.split_once('\t').ok_or(LocalProductError::Descriptor(
            "generation descriptor field is malformed",
        ))?;
        if name != expected || value.is_empty() {
            return Err(LocalProductError::Descriptor(
                "generation descriptor field is invalid",
            ));
        }
        Ok(value)
    }

    fn parse_generation_helper_path(identity: &str) -> Option<&'static str> {
        match identity {
            TERMUX_BROWSER_OPEN_HELPER_IDENTITY => Some("browser/open/curl"),
            TERMUX_BROWSER_MANUAL_HELPER_IDENTITY => Some("browser/manual/curl"),
            _ => None,
        }
    }

    pub(super) fn load_local_generation(
        generation_dir: &Path,
    ) -> Result<LoadedLocalGeneration, LocalProductError> {
        ensure_real_directory(
            generation_dir,
            "read activated generation descriptor",
            "activated generation directory must be a real directory",
        )?;
        let descriptor_path = generation_dir.join("generation.meta");
        let bytes = read_bounded_regular_file(
            &descriptor_path,
            LOCAL_GENERATION_MAX_BYTES,
            "read activated generation descriptor",
            LocalProductError::Descriptor("generation descriptor is too large"),
            LocalProductError::UnsafeSource("generation descriptor must be a regular file"),
        )?;
        if !bytes.ends_with(b"\n") {
            return Err(LocalProductError::Descriptor(
                "generation descriptor is missing its final newline",
            ));
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| LocalProductError::Descriptor("generation descriptor is not UTF-8"))?;
        let mut lines = text.lines();
        let generation_layout = match lines.next() {
            Some(LOCAL_GENERATION_FORMAT) => GenerationLayout::PackageRoot,
            _ => {
                return Err(LocalProductError::Descriptor(
                    "generation descriptor format is unsupported",
                ))
            }
        };

        let generation_id = descriptor_field(lines.next(), "generation_id")?;
        generation_state::validate_generation_identity(generation_id, "generation_id")
            .map_err(LocalProductError::StateFormat)?;
        let upstream_package_identity =
            descriptor_field(lines.next(), "upstream_package_identity")?;
        let upstream_package_version = descriptor_field(lines.next(), "upstream_package_version")?;
        let source_artifact_digest = descriptor_field(lines.next(), "source_artifact_digest")?;
        let expected_platform = descriptor_field(lines.next(), "expected_platform")?;
        let expected_architecture = descriptor_field(lines.next(), "expected_architecture")?;
        let patch_policy_id = descriptor_field(lines.next(), "patch_policy_id")?;
        let patch_report = descriptor_field(lines.next(), "patch_report")?;
        let runtime_digest = descriptor_field(lines.next(), "runtime_digest")?;
        let wrapper_artifact_digest = descriptor_field(lines.next(), "wrapper_artifact_digest")?;
        let wrapper_api_identity = descriptor_field(lines.next(), "wrapper_api_identity")?;
        let persistent_schema_identity =
            descriptor_field(lines.next(), "persistent_schema_identity")?;
        if descriptor_field(lines.next(), "qualification")? != "qualified" {
            return Err(LocalProductError::Descriptor(
                "activated generation is not qualified",
            ));
        }
        let creation_metadata = descriptor_field(lines.next(), "creation_metadata")?;
        let helper_count_str = descriptor_field(lines.next(), "helper_count")?;
        if !valid_nonnegative_decimal(helper_count_str) {
            return Err(LocalProductError::Descriptor(
                "generation helper count is invalid",
            ));
        }
        let helper_count: usize = helper_count_str
            .parse()
            .map_err(|_| LocalProductError::Descriptor("generation helper count is invalid"))?;
        if helper_count > LOCAL_RELEASE_MAX_FILES.saturating_sub(2) {
            return Err(LocalProductError::Descriptor(
                "generation helper count is outside the supported bound",
            ));
        }

        let mut helper_digests = Vec::with_capacity(helper_count);
        let mut helper_paths = Vec::with_capacity(helper_count);
        for _ in 0..helper_count {
            let line = lines.next().ok_or(LocalProductError::Descriptor(
                "generation helper list is incomplete",
            ))?;
            let mut parts = line.split('\t');
            if parts.next() != Some("helper") {
                return Err(LocalProductError::Descriptor(
                    "generation helper entry is invalid",
                ));
            }
            let identity =
                parts
                    .next()
                    .filter(|v| !v.is_empty())
                    .ok_or(LocalProductError::Descriptor(
                        "generation helper identity is invalid",
                    ))?;
            let digest =
                parts
                    .next()
                    .filter(|v| !v.is_empty())
                    .ok_or(LocalProductError::Descriptor(
                        "generation helper digest is invalid",
                    ))?;
            if parts.next().is_some() {
                return Err(LocalProductError::Descriptor(
                    "generation helper entry is invalid",
                ));
            }
            if let Some(rel) = parse_generation_helper_path(identity) {
                helper_digests.push(GenerationHelperDigest {
                    identity: identity.to_owned(),
                    digest: digest.to_owned(),
                });
                helper_paths.push(generation_dir.join(rel));
            }
        }
        if lines.next().is_some() {
            return Err(LocalProductError::Descriptor(
                "generation descriptor has unexpected trailing fields",
            ));
        }

        let manifest = GenerationManifest {
            upstream_package_identity: upstream_package_identity.to_owned(),
            upstream_package_version: upstream_package_version.to_owned(),
            source_artifact_digest: source_artifact_digest.to_owned(),
            expected_platform: expected_platform.to_owned(),
            expected_architecture: expected_architecture.to_owned(),
            patch_policy_id: patch_policy_id.to_owned(),
            patch_report: patch_report.to_owned(),
            runtime_digest: runtime_digest.to_owned(),
            helper_digests,
            wrapper_artifact_digest: wrapper_artifact_digest.to_owned(),
            wrapper_api_identity: wrapper_api_identity.to_owned(),
            persistent_schema_identity: persistent_schema_identity.to_owned(),
            creation_metadata: creation_metadata.to_owned(),
        };

        ensure_real_directory(
            &generation_dir.join("bin"),
            "inspect activated generation bin directory",
            "activated generation bin directory must be a real directory",
        )?;
        let runtime_path = generation_dir.join(WRAPPER_RUNTIME_PATH);
        if !runtime_path.is_file() {
            return Err(LocalProductError::Descriptor(
                "activated generation runtime is missing",
            ));
        }
        ensure_regular_file(
            &runtime_path,
            "inspect activated generation runtime",
            "activated generation runtime must be a regular file",
        )?;

        let code_mode_host = generation_dir.join(WRAPPER_CODE_MODE_HOST_PATH);
        let code_mode_host_path = match fs::symlink_metadata(&code_mode_host) {
            Ok(_) => {
                ensure_regular_file(
                    &code_mode_host,
                    "inspect activated generation code-mode host",
                    "activated generation code-mode host must be a regular file",
                )?;
                Some(code_mode_host)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "inspect activated generation code-mode host",
                    source,
                })
            }
        };
        match fs::symlink_metadata(generation_dir.join(WRAPPER_EXEC_SERVER_PATH)) {
            Ok(_) => ensure_regular_file(
                &generation_dir.join(WRAPPER_EXEC_SERVER_PATH),
                "inspect activated generation exec server",
                "activated generation exec server must be a regular file",
            )?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "inspect activated generation exec server",
                    source,
                })
            }
        }
        ensure_regular_file(
            &generation_dir.join(WRAPPER_PACKAGE_JSON_PATH),
            "inspect activated generation package metadata",
            "activated generation package metadata must be a regular file",
        )?;
        for forbidden in ["compat", "runtime", "codex-code-mode-host"] {
            match fs::symlink_metadata(generation_dir.join(forbidden)) {
                Ok(_) => {
                    return Err(LocalProductError::UnsafeSource(
                        "generation contains unexpected legacy path",
                    ))
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(LocalProductError::Io {
                        operation: "inspect activated generation layout",
                        source,
                    })
                }
            }
        }
        for subdir in ["codex-path", "codex-resources"] {
            ensure_real_directory_if_present(
                &generation_dir.join(subdir),
                "inspect activated generation package directory",
                "activated generation package directory must be a real directory",
            )?;
        }

        for (index, helper_path) in helper_paths.iter().enumerate() {
            if !helper_path.is_file() {
                return Err(LocalProductError::Descriptor(
                    "activated generation helper is missing",
                ));
            }
            let identity = manifest.helper_digests[index].identity.as_str();
            if matches!(
                identity,
                TERMUX_BROWSER_OPEN_HELPER_IDENTITY | TERMUX_BROWSER_MANUAL_HELPER_IDENTITY
            ) {
                ensure_real_directory(
                    &generation_dir.join("browser"),
                    "inspect activated generation browser helper root",
                    "activated generation browser helper root must be a real directory",
                )?;
                let leaf = if identity == TERMUX_BROWSER_OPEN_HELPER_IDENTITY {
                    "open"
                } else {
                    "manual"
                };
                ensure_real_directory(
                    &generation_dir.join("browser").join(leaf),
                    "inspect activated generation browser helper directory",
                    "activated generation browser helper directory must be a real directory",
                )?;
            }
            ensure_regular_file(
                helper_path,
                "inspect activated generation helper",
                "activated generation helper must be a regular file",
            )?;
        }

        Ok(LoadedLocalGeneration {
            generation_id: generation_id.to_owned(),
            manifest,
            generation_layout,
            runtime_path,
            code_mode_host_path,
            generation_dir: generation_dir.to_path_buf(),
            helper_paths,
        })
    }

    fn load_activated_generation(
        roots: &LocalWrapperRoots,
    ) -> Result<
        (
            generation_state::GenerationPointerState,
            LoadedLocalGeneration,
        ),
        LocalProductError,
    > {
        let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root)
            .map_err(LocalProductError::StateFormat)?;
        let pointer_state = match fs::symlink_metadata(&state_paths.activation_state) {
            Ok(_) => generation_state::recover_activation_state(&state_paths)
                .map_err(LocalProductError::State)?
                .ok_or(LocalProductError::NoCurrentGeneration)?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(LocalProductError::NoCurrentGeneration)
            }
            Err(source) => {
                return Err(LocalProductError::Io {
                    operation: "inspect activation state",
                    source,
                })
            }
        };
        let loaded = load_local_generation(&roots.generation_root.join(&pointer_state.current))?;
        if loaded.generation_id != pointer_state.current {
            return Err(LocalProductError::Descriptor(
                "activated generation descriptor does not match pointer",
            ));
        }
        Ok((pointer_state, loaded))
    }

    pub(super) fn with_qualified_loaded_runtime<'loaded, T, F>(
        loaded: &'loaded LoadedLocalGeneration,
        operation: F,
    ) -> Result<T, LocalProductError>
    where
        F: for<'selection, 'asset> FnOnce(
            QualifiedRuntimeAssets<'selection, 'asset>,
        ) -> Result<T, LocalProductError>,
    {
        qualify_generation_manifest(&loaded.manifest).map_err(LocalProductError::Manifest)?;
        let helper_bindings: Vec<_> = loaded
            .manifest
            .helper_digests
            .iter()
            .zip(&loaded.helper_paths)
            .map(|(helper, path)| HelperAssetBinding {
                identity: &helper.identity,
                asset_path: path.as_os_str(),
                observed_digest: &helper.digest,
            })
            .collect();
        let runtime_selection = RuntimeAssetSelection {
            runtime: RuntimeAssetBinding {
                program_path: loaded.runtime_path.as_os_str(),
                observed_digest: &loaded.manifest.runtime_digest,
            },
            generation_dir: loaded.generation_dir.as_os_str(),
            helpers: &helper_bindings,
        };
        let runtime_assets =
            qualify_runtime_assets(&runtime_selection).map_err(LocalProductError::Runtime)?;
        operation(runtime_assets)
    }

    pub(super) fn probe_release_candidate(
        loaded: &LoadedLocalGeneration,
        roots: &LocalWrapperRoots,
        process_env: &TermuxProcessEnvSnapshot,
    ) -> Result<(), LocalProductError> {
        with_qualified_loaded_runtime(loaded, |assets| {
            if !doctor::probe_qualified_upstream_command(
                assets,
                process_env,
                roots.cert_file.as_os_str(),
                Some(roots.cert_dir.as_os_str()),
                &roots.resolver_path,
                &roots.config_dir,
                &["--version"],
            )
            .map_err(|_| LocalProductError::CandidateProbe("candidate version probe failed"))?
            {
                return Err(LocalProductError::CandidateProbe(
                    "candidate version probe was unhealthy",
                ));
            }
            Ok(())
        })
    }

    fn exec_runtime<P, I, S, R, C>(
        program: P,
        args: I,
        resolver_path: R,
        config_dir: C,
        env_plan: Option<&TermuxBaseEnvPlan>,
    ) -> io::Error
    where
        P: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        R: AsRef<Path>,
        C: AsRef<Path>,
    {
        let runtime_fds = match RuntimeFdSources::open(resolver_path, config_dir) {
            Ok(fds) => fds,
            Err(err) => return err,
        };
        let mut command = Command::new(program.as_ref());
        command.args(args);
        apply_child_env_plan_and_fence(&mut command, env_plan);
        runtime_fds.configure(&mut command);
        command.exec()
    }

    fn launch_upstream_runtime(
        assets: QualifiedRuntimeAssets<'_, '_>,
        process_env: &TermuxProcessEnvSnapshot,
        cert_file: &OsStr,
        cert_dir: Option<&OsStr>,
        resolver_path: &Path,
        config_dir: &Path,
        args: &[OsString],
    ) -> RuntimeLaunchError {
        let selection = assets.selection();
        let browser_helpers = match termux_browser_helper_paths(selection) {
            Ok(paths) => paths,
            Err(err) => return RuntimeLaunchError::Environment(err),
        };
        let env_plan = match plan_termux_env(
            process_env,
            selection.generation_dir,
            browser_helpers,
            cert_file,
            cert_dir,
        ) {
            Ok(plan) => plan,
            Err(err) => return RuntimeLaunchError::Environment(err),
        };

        if !is_help_or_version_invocation(args) {
            if let Some(codex_home) = resolve_codex_home() {
                if let Err(err) = daemon_settings::ensure(&codex_home) {
                    return RuntimeLaunchError::Config(err);
                }
            }
        }
        if let Err(err) = prepare_wrapper_config(config_dir) {
            return RuntimeLaunchError::Config(err);
        }

        let exec_err = exec_runtime(
            selection.runtime.program_path,
            args,
            resolver_path,
            config_dir,
            Some(&env_plan),
        );
        RuntimeLaunchError::Exec(exec_err)
    }

    pub(super) fn execute_activated_route(
        route: PublicDispatchRoute,
        roots: &LocalWrapperRoots,
        process_env: &TermuxProcessEnvSnapshot,
    ) -> Result<PublicDispatchCompletion, LocalProductError> {
        let _lease = maintenance::launch_lease(&roots.generation_root).map_err(|source| {
            LocalProductError::Io {
                operation: "lease installed generation during launch",
                source,
            }
        })?;
        let (_, loaded) = load_activated_generation(roots)?;
        with_qualified_loaded_runtime(&loaded, |runtime_assets| {
            let context = LocalPublicDispatchContext {
                runtime_assets,
                process_env,
                cert_file: roots.cert_file.as_os_str(),
                cert_dir: Some(roots.cert_dir.as_os_str()),
                resolver_path: &roots.resolver_path,
                config_dir: &roots.config_dir,
                wrapper_doctor_status: doctor::WrapperDoctorStatus::Healthy,
                generation_id: &loaded.generation_id,
                generation_layout: loaded.generation_layout,
                has_code_mode_host: loaded.code_mode_host_path.is_some(),
            };
            match route {
                PublicDispatchRoute::Doctor(args) => {
                    let outcome = doctor::run_local_doctor_command(args, context)
                        .map_err(PublicDispatchExecutionError::Doctor)
                        .map_err(LocalProductError::Dispatch)?;
                    Ok(PublicDispatchCompletion::Doctor(outcome))
                }
                PublicDispatchRoute::Upstream(args) => {
                    let err = launch_upstream_runtime(
                        context.runtime_assets,
                        context.process_env,
                        context.cert_file,
                        context.cert_dir,
                        context.resolver_path,
                        context.config_dir,
                        &args,
                    );
                    Err(LocalProductError::Dispatch(
                        PublicDispatchExecutionError::Upstream(err),
                    ))
                }
                PublicDispatchRoute::Update(_) | PublicDispatchRoute::Wrapper(_) => {
                    unreachable!("handled before execute_activated_route")
                }
            }
        })
    }

    fn print_wrapper_usage() {
        println!("usage: codex wrapper <status [--json] | setup [show|--bare|--enable MODULES|--disable MODULES|--signed-update true|false] | --bare [--] [UPSTREAM_ARGS...] | --with MODULES [--] [UPSTREAM_ARGS...] | --help>");
        println!("  status [--json]   show active generation, paths, and opt-in modules");
        println!("  setup             read or change the persistent optional-module selection");
        println!(
            "  --bare            launch upstream without optional modules for this invocation"
        );
        println!(
            "  --with MODULES    opt in only for this invocation (unavailable modules fail closed)"
        );
        println!("  --help            show this help");
    }

    fn run_wrapper_setup(args: Vec<OsString>) -> i32 {
        let words = args
            .iter()
            .map(|value| value.to_str())
            .collect::<Option<Vec<_>>>();
        let Some(words) = words else {
            eprintln!("codex wrapper setup: arguments must be UTF-8");
            return 2;
        };
        let roots = match LocalWrapperRoots::from_environment() {
            Ok(roots) => roots,
            Err(err) => {
                eprintln!("codex wrapper setup: {err}");
                return 1;
            }
        };
        let result = match words.as_slice() {
            [] | ["show"] => extensions::WrapperSettings::read(&roots.state_root),
            ["--bare"] => extensions::WrapperSettings::update(&roots.state_root, |settings| {
                *settings = extensions::WrapperSettings::default();
                Ok(())
            }),
            ["--enable", modules] => {
                extensions::WrapperSettings::update(&roots.state_root, |settings| {
                    settings.set_modules(modules, true)
                })
            }
            ["--disable", modules] => {
                extensions::WrapperSettings::update(&roots.state_root, |settings| {
                    settings.set_modules(modules, false)
                })
            }
            ["--signed-update", "true"] => {
                extensions::WrapperSettings::update(&roots.state_root, |settings| {
                    settings.signed_update = true;
                    Ok(())
                })
            }
            ["--signed-update", "false"] => {
                extensions::WrapperSettings::update(&roots.state_root, |settings| {
                    settings.signed_update = false;
                    Ok(())
                })
            }
            ["-h" | "--help"] => {
                print_wrapper_usage();
                return 0;
            }
            _ => {
                eprintln!("codex wrapper setup: invalid arguments");
                print_wrapper_usage();
                return 2;
            }
        };
        match result {
            Ok(settings) => {
                println!(
                    "config: {}",
                    roots.state_root.join("wrapper.toml").display()
                );
                println!("addons: {}", settings.addons_label());
                println!("signed_update: {}", settings.signed_update);
                0
            }
            Err(err) => {
                eprintln!("codex wrapper setup: {err}");
                if err.kind() == io::ErrorKind::InvalidData {
                    2
                } else {
                    1
                }
            }
        }
    }

    fn run_bare_upstream(args: Vec<OsString>) -> i32 {
        if let Err(err) = plan_passthrough_args(&args) {
            eprintln!("codex wrapper --bare: {err}");
            return 2;
        }
        let roots = match LocalWrapperRoots::from_environment() {
            Ok(roots) => roots,
            Err(err) => {
                eprintln!("codex wrapper --bare: {err}");
                return 1;
            }
        };
        let env = capture_termux_process_env();
        if let Err(err) = execute_activated_route(PublicDispatchRoute::Upstream(args), &roots, &env)
        {
            eprintln!("codex wrapper --bare: {err}");
            return 1;
        }
        0
    }

    fn run_wrapper_command(args: Vec<OsString>) -> i32 {
        let mut iter = args.into_iter();
        let Some(subcommand) = iter.next() else {
            print_wrapper_usage();
            return 0;
        };
        let rest: Vec<OsString> = iter.collect();
        match subcommand.to_str() {
            Some("-h" | "--help" | "help") if rest.is_empty() => {
                print_wrapper_usage();
                0
            }
            Some("setup") => run_wrapper_setup(rest),
            Some("--bare") => {
                let mut args = rest;
                if args.first().is_some_and(|value| value == "--") {
                    args.remove(0);
                }
                run_bare_upstream(args)
            }
            Some("--with") => {
                let Some(names) = rest.first().and_then(|value| value.to_str()) else {
                    eprintln!("codex wrapper --with: expected module list");
                    return 2;
                };
                let mut settings = extensions::WrapperSettings::default();
                if let Err(err) = settings.set_modules(names, true) {
                    eprintln!("codex wrapper --with: {err}");
                    return 2;
                }
                let mut args = rest[1..].to_vec();
                if args.first().is_some_and(|value| value == "--") {
                    args.remove(0);
                }
                if let Err(err) = plan_passthrough_args(&args) {
                    eprintln!("codex wrapper --with: {err}");
                    return 2;
                }
                eprintln!(
                    "codex wrapper --with: optional module(s) {} are not installed",
                    settings.addons_label()
                );
                1
            }
            Some("status") => {
                let mut json = false;
                for arg in rest {
                    match arg.to_str() {
                        Some("--json") if !json => json = true,
                        Some("-h" | "--help") => {
                            print_wrapper_usage();
                            return 0;
                        }
                        _ => {
                            eprintln!("codex wrapper status: unexpected argument {:?}", arg);
                            return 2;
                        }
                    }
                }
                run_wrapper_status(json)
            }
            _ => {
                eprintln!("codex wrapper: unknown subcommand {:?}", subcommand);
                print_wrapper_usage();
                2
            }
        }
    }

    fn run_wrapper_status(json: bool) -> i32 {
        let roots = match LocalWrapperRoots::from_environment() {
            Ok(roots) => roots,
            Err(err) => {
                eprintln!("codex wrapper status: {err}");
                return 1;
            }
        };
        let addons = match extensions::WrapperSettings::read(&roots.state_root) {
            Ok(settings) => settings.addons_label(),
            Err(err) => {
                eprintln!("codex wrapper status: invalid wrapper.toml: {err}");
                return 1;
            }
        };
        let codex_home = resolve_codex_home();
        let loaded = load_activated_generation(&roots);
        if json {
            let body = match &loaded {
                Ok((pointer, gen)) => {
                    let prev_json = pointer
                        .previous
                        .as_deref()
                        .map(|p| format!("\"{}\"", doctor::json_escape(p)))
                        .unwrap_or_else(|| "null".to_owned());
                    let codex_home_json = codex_home
                        .as_ref()
                        .map(|p| format!("\"{}\"", doctor::json_escape(&p.to_string_lossy())))
                        .unwrap_or_else(|| "null".to_owned());
                    format!(
                        "{{\"status\":\"active\",\"generation_id\":\"{}\",\"previous_generation_id\":{},\"upstream_version\":\"{}\",\"layout\":\"{}\",\"runtime_path\":\"{}\",\"has_code_mode_host\":{},\"patch_policy_id\":\"{}\",\"patch_report\":\"{}\",\"creation_metadata\":\"{}\",\"daemon_passthrough\":\"supported\",\"addons\":\"{}\",\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                        doctor::json_escape(&gen.generation_id),
                        prev_json,
                        doctor::json_escape(&gen.manifest.upstream_package_version),
                        doctor::json_escape(gen.generation_layout.as_str()),
                        doctor::json_escape(&gen.runtime_path.to_string_lossy()),
                        gen.code_mode_host_path.is_some(),
                        doctor::json_escape(&gen.manifest.patch_policy_id),
                        doctor::json_escape(&gen.manifest.patch_report),
                        doctor::json_escape(&gen.manifest.creation_metadata),
                        doctor::json_escape(&addons),
                        doctor::json_escape(&roots.generation_root.to_string_lossy()),
                        doctor::json_escape(&roots.state_root.to_string_lossy()),
                        doctor::json_escape(&roots.config_dir.to_string_lossy()),
                        codex_home_json,
                    )
                }
                Err(LocalProductError::NoCurrentGeneration) => {
                    let codex_home_json = codex_home
                        .as_ref()
                        .map(|p| format!("\"{}\"", doctor::json_escape(&p.to_string_lossy())))
                        .unwrap_or_else(|| "null".to_owned());
                    format!(
                        "{{\"status\":\"uninitialized\",\"generation_id\":null,\"previous_generation_id\":null,\"daemon_passthrough\":\"supported\",\"addons\":\"{}\",\"generation_root\":\"{}\",\"state_root\":\"{}\",\"config_dir\":\"{}\",\"codex_home\":{}}}\n",
                        doctor::json_escape(&addons),
                        doctor::json_escape(&roots.generation_root.to_string_lossy()),
                        doctor::json_escape(&roots.state_root.to_string_lossy()),
                        doctor::json_escape(&roots.config_dir.to_string_lossy()),
                        codex_home_json,
                    )
                }
                Err(err) => {
                    eprintln!("codex wrapper status: {err}");
                    return 1;
                }
            };
            print!("{body}");
            return if loaded.is_ok() { 0 } else { 1 };
        }

        match loaded {
            Ok((pointer, gen)) => {
                println!("status: active");
                println!("generation_id: {}", gen.generation_id);
                println!(
                    "previous_generation_id: {}",
                    pointer.previous.as_deref().unwrap_or("-")
                );
                println!(
                    "upstream_version: {}",
                    gen.manifest.upstream_package_version
                );
                println!("layout: {}", gen.generation_layout.as_str());
                println!("runtime_path: {}", gen.runtime_path.display());
                println!("has_code_mode_host: {}", gen.code_mode_host_path.is_some());
                println!("patch_policy_id: {}", gen.manifest.patch_policy_id);
                println!("patch_report: {}", gen.manifest.patch_report);
                println!("creation_metadata: {}", gen.manifest.creation_metadata);
                println!("daemon_passthrough: supported");
                println!("addons: {addons}");
                println!("generation_root: {}", roots.generation_root.display());
                println!("state_root: {}", roots.state_root.display());
                println!("config_dir: {}", roots.config_dir.display());
                println!(
                    "codex_home: {}",
                    codex_home
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "-".to_owned())
                );
                0
            }
            Err(LocalProductError::NoCurrentGeneration) => {
                println!("status: uninitialized");
                println!("generation_root: {}", roots.generation_root.display());
                println!("state_root: {}", roots.state_root.display());
                println!("config_dir: {}", roots.config_dir.display());
                println!("addons: {addons}");
                println!("Run 'codex update' or 'codex update --path <FILE_OR_DIR>' to install a generation.");
                1
            }
            Err(err) => {
                eprintln!("codex wrapper status: {err}");
                1
            }
        }
    }

    pub(super) fn valid_sha256_hex(value: &str) -> bool {
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
    }

    pub(super) fn ensure_curl_available(curl: &Path) -> Result<(), LocalProductError> {
        ensure_regular_file_tool(curl)
            .then_some(())
            .ok_or(LocalProductError::CurlUnavailable)
    }

    pub(super) fn ensure_openssl_available(openssl: &Path) -> Result<(), LocalProductError> {
        ensure_regular_file_tool(openssl)
            .then_some(())
            .ok_or(LocalProductError::OpenSslUnavailable)
    }

    fn ensure_regular_file_tool(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .is_ok_and(|meta| meta.file_type().is_file() && meta.permissions().mode() & 0o111 != 0)
    }

    pub(super) fn openssl_sha256(openssl: &Path, path: &Path) -> Result<String, LocalProductError> {
        ensure_openssl_available(openssl)?;
        ensure_regular_file(
            path,
            "inspect file for SHA-256 verification",
            "SHA-256 verification target must be a regular file",
        )?;
        let output = Command::new(openssl)
            .args(["dgst", "-sha256", "-r"])
            .arg(path)
            .stdin(Stdio::null())
            .output()
            .map_err(|source| LocalProductError::Io {
                operation: "run openssl dgst -sha256",
                source,
            })?;
        if !output.status.success() {
            return Err(LocalProductError::OpenSslFailed(
                "openssl dgst -sha256 failed",
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hex = stdout
            .split_whitespace()
            .next()
            .map(|s| s.to_ascii_lowercase())
            .filter(|s| valid_sha256_hex(s))
            .ok_or(LocalProductError::OpenSslFailed(
                "openssl dgst -sha256 returned malformed digest output",
            ))?;
        Ok(hex)
    }

    pub(super) fn ensure_real_directory(
        path: &Path,
        operation: &'static str,
        message: &'static str,
    ) -> Result<(), LocalProductError> {
        let meta = fs::symlink_metadata(path)
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if !meta.file_type().is_dir() {
            return Err(LocalProductError::UnsafeSource(message));
        }
        Ok(())
    }

    pub(super) fn ensure_real_directory_if_present(
        path: &Path,
        operation: &'static str,
        message: &'static str,
    ) -> Result<(), LocalProductError> {
        match fs::symlink_metadata(path) {
            Ok(meta) => {
                if !meta.file_type().is_dir() {
                    Err(LocalProductError::UnsafeSource(message))
                } else {
                    Ok(())
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(LocalProductError::Io { operation, source }),
        }
    }

    pub(super) fn ensure_real_directory_tree(
        path: &Path,
        operation: &'static str,
        message: &'static str,
    ) -> Result<(), LocalProductError> {
        if !fs_util::wrapper_safe_absolute_path(path) {
            return Err(LocalProductError::UnsafeSource(message));
        }
        let mut cursor = PathBuf::new();
        for component in path.components() {
            cursor.push(component);
            if component == std::path::Component::RootDir {
                continue;
            }
            match fs::symlink_metadata(&cursor) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        // Allow Termux /data/data -> /data/user/0 or /tmp ancestors only outside leaf? Wait: leaf must not be symlink.
                        if cursor == path {
                            return Err(LocalProductError::UnsafeSource(message));
                        }
                        let target_meta = fs::metadata(&cursor)
                            .map_err(|source| LocalProductError::Io { operation, source })?;
                        if !target_meta.is_dir() {
                            return Err(LocalProductError::UnsafeSource(message));
                        }
                    } else if !meta.file_type().is_dir() {
                        return Err(LocalProductError::UnsafeSource(message));
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    let mut builder = fs::DirBuilder::new();
                    builder.mode(0o700);
                    builder
                        .create(&cursor)
                        .map_err(|source| LocalProductError::Io { operation, source })?;
                }
                Err(source) => return Err(LocalProductError::Io { operation, source }),
            }
        }
        Ok(())
    }

    pub(super) fn ensure_regular_file(
        path: &Path,
        operation: &'static str,
        message: &'static str,
    ) -> Result<(), LocalProductError> {
        let meta = fs::symlink_metadata(path)
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if !meta.file_type().is_file() {
            return Err(LocalProductError::UnsafeSource(message));
        }
        Ok(())
    }

    pub(super) fn read_bounded_regular_file(
        path: &Path,
        limit: usize,
        operation: &'static str,
        over_limit: LocalProductError,
        not_regular: LocalProductError,
    ) -> Result<Vec<u8>, LocalProductError> {
        let sym_meta = fs::symlink_metadata(path)
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if !sym_meta.file_type().is_file() {
            return Err(not_regular);
        }
        let file =
            File::open(path).map_err(|source| LocalProductError::Io { operation, source })?;
        let file_meta = file
            .metadata()
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if file_meta.dev() != sym_meta.dev() || file_meta.ino() != sym_meta.ino() {
            return Err(not_regular);
        }
        let mut bytes = Vec::new();
        io::Read::take(file, limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| LocalProductError::Io { operation, source })?;
        if bytes.len() > limit {
            return Err(over_limit);
        }
        Ok(bytes)
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
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|source| LocalProductError::Io {
                operation: "create remote download output",
                source,
            })
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

    pub(super) fn copy_local_directory_tree(
        source: &Path,
        destination: &Path,
    ) -> Result<(), LocalProductError> {
        ensure_real_directory(
            source,
            "inspect source directory for copy",
            "source tree must contain real directories only",
        )?;
        let src_meta = fs::symlink_metadata(source).map_err(|source| LocalProductError::Io {
            operation: "inspect source directory permissions",
            source,
        })?;
        fs::DirBuilder::new()
            .mode(src_meta.permissions().mode() & 0o777)
            .create(destination)
            .map_err(|source| LocalProductError::Io {
                operation: "create destination directory during copy",
                source,
            })?;

        let entries = fs::read_dir(source).map_err(|source| LocalProductError::Io {
            operation: "read source directory for copy",
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| LocalProductError::Io {
                operation: "iterate source directory for copy",
                source,
            })?;
            let src_child = entry.path();
            let dst_child = destination.join(entry.file_name());
            let meta =
                fs::symlink_metadata(&src_child).map_err(|source| LocalProductError::Io {
                    operation: "inspect entry in source tree",
                    source,
                })?;
            if meta.file_type().is_dir() {
                copy_local_directory_tree(&src_child, &dst_child)?;
            } else if meta.file_type().is_file() {
                let mut reader =
                    File::open(&src_child).map_err(|source| LocalProductError::Io {
                        operation: "open source file for copy",
                        source,
                    })?;
                let mut writer = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(meta.permissions().mode() & 0o777)
                    .open(&dst_child)
                    .map_err(|source| LocalProductError::Io {
                        operation: "create destination file during copy",
                        source,
                    })?;
                io::copy(&mut reader, &mut writer).map_err(|source| LocalProductError::Io {
                    operation: "copy file contents",
                    source,
                })?;
                writer.sync_all().map_err(|source| LocalProductError::Io {
                    operation: "sync copied file",
                    source,
                })?;
            } else {
                return Err(LocalProductError::UnsafeSource(
                    "generation tree must contain only regular files and directories",
                ));
            }
        }
        sync_directory(destination)
    }

    pub(super) fn rename_noreplace(source: &Path, destination: &Path) -> io::Result<()> {
        use std::ffi::CString;

        const AT_FDCWD: i32 = -100;
        const RENAME_NOREPLACE: u32 = 1;
        #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
        unsafe extern "C" {
            fn renameat2(
                olddirfd: i32,
                oldpath: *const std::ffi::c_char,
                newdirfd: i32,
                newpath: *const std::ffi::c_char,
                flags: u32,
            ) -> i32;
        }
        let source = CString::new(source.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "source path contains NUL"))?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "destination path contains NUL")
        })?;
        // Android API 24 does not export renameat2 in libc, though the arm64 kernel provides it.
        #[cfg(all(target_os = "android", target_arch = "aarch64"))]
        let result = unsafe {
            unsafe extern "C" {
                fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
            }
            const SYS_RENAMEAT2_AARCH64: std::ffi::c_long = 276;
            syscall(
                SYS_RENAMEAT2_AARCH64,
                AT_FDCWD as std::ffi::c_long,
                source.as_ptr() as std::ffi::c_long,
                AT_FDCWD as std::ffi::c_long,
                destination.as_ptr() as std::ffi::c_long,
                RENAME_NOREPLACE as std::ffi::c_long,
            ) as i32
        };
        #[cfg(not(all(target_os = "android", target_arch = "aarch64")))]
        let result = unsafe {
            renameat2(
                AT_FDCWD,
                source.as_ptr(),
                AT_FDCWD,
                destination.as_ptr(),
                RENAME_NOREPLACE,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(super) fn sync_directory(parent: &Path) -> Result<(), LocalProductError> {
        let directory = File::open(parent).map_err(|source| LocalProductError::Io {
            operation: "open directory for sync",
            source,
        })?;
        directory
            .sync_all()
            .map_err(|source| LocalProductError::Io {
                operation: "sync directory",
                source,
            })
    }

    pub(super) fn run_public_main<I, S>(args: I) -> i32
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let original: Vec<OsString> = args.into_iter().map(Into::into).collect();
        let route = match plan_public_dispatch(original) {
            Ok(route) => route,
            Err(err) => {
                eprintln!("codex: {err}");
                return 2;
            }
        };
        match route {
            PublicDispatchRoute::Update(args) => {
                let is_help = args
                    .iter()
                    .any(|a| matches!(a.to_str(), Some("-h" | "--help")));
                let code = update::run_update_command(args);
                if code == 0 && !is_help {
                    if let Ok(roots) = LocalWrapperRoots::from_environment() {
                        let _ = maintenance::prune(&roots);
                    }
                }
                code
            }
            PublicDispatchRoute::Wrapper(args) => run_wrapper_command(args),
            route @ (PublicDispatchRoute::Doctor(_) | PublicDispatchRoute::Upstream(_)) => {
                let roots = match LocalWrapperRoots::from_environment() {
                    Ok(roots) => roots,
                    Err(err) => {
                        eprintln!("codex: {err}");
                        return 1;
                    }
                };
                if let PublicDispatchRoute::Upstream(ref args) = route {
                    // Direct help/version remains a zero-write, zero-addon fast path.
                    if !is_help_or_version_invocation(args) {
                        let settings = match extensions::WrapperSettings::read(&roots.state_root) {
                            Ok(settings) => settings,
                            Err(err) => {
                                eprintln!("codex: invalid wrapper.toml: {err}");
                                return 1;
                            }
                        };
                        if let Some(modules) = settings.unavailable_addons() {
                            eprintln!("codex: enabled optional module(s) {modules} are unavailable; use 'codex wrapper --bare -- ...' or disable them with 'codex wrapper setup --disable {modules}'");
                            return 1;
                        }
                    }
                }
                let process_env = capture_termux_process_env();
                match execute_activated_route(route, &roots, &process_env) {
                    Ok(PublicDispatchCompletion::Doctor(outcome)) => {
                        print!("{}", outcome.output);
                        doctor::doctor_exit_code(outcome.exit_class)
                    }
                    Err(err) => {
                        eprintln!("codex: {err}");
                        1
                    }
                }
            }
        }
    }

    pub(super) fn entry() -> ! {
        let mut args = std::env::args_os();
        let _ = args.next();
        std::process::exit(run_public_main(args));
    }
}

#[cfg(unix)]
use unix_app::*;

#[cfg(unix)]
fn main() {
    unix_app::entry();
}

#[cfg(not(unix))]
fn main() {
    eprintln!("codex: this build requires a Unix/Termux target");
    std::process::exit(1);
}

#[cfg(all(test, unix))]
pub(crate) mod tests {
    use super::generation_state;
    use super::unix_app::*;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    pub(crate) fn temp_root(label: &str) -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "codex-wrapper-test-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub(crate) fn remove_temp_root(path: impl AsRef<Path>) {
        let _ = fs::remove_dir_all(path);
    }

    fn os_vec(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    fn make_test_roots(root: &Path) -> LocalWrapperRoots {
        let prefix = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr"));
        let state_root = root.join("state");
        let config_dir = state_root.join("config");
        let generation_root = root.join("generations");
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(&generation_root).unwrap();
        let resolver_path = root.join("resolv.conf");
        fs::write(&resolver_path, b"nameserver 1.1.1.1\n").unwrap();
        LocalWrapperRoots {
            generation_root,
            state_root,
            config_dir,
            resolver_path,
            cert_file: prefix.join("etc/tls/cert.pem"),
            cert_dir: prefix.join("etc/tls/certs"),
            openssl: prefix.join("bin/openssl"),
            curl: prefix.join("bin/curl"),
        }
    }

    fn write_sample_generation(dir: &Path, generation_id: &str, with_code_mode_host: bool) {
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::create_dir_all(dir.join("browser/open")).unwrap();
        fs::create_dir_all(dir.join("browser/manual")).unwrap();
        fs::create_dir_all(dir.join("codex-resources/sub")).unwrap();
        fs::write(dir.join("bin/codex"), b"#!/bin/sh\necho codex 1.2.3\n").unwrap();
        fs::set_permissions(dir.join("bin/codex"), fs::Permissions::from_mode(0o755)).unwrap();
        if with_code_mode_host {
            fs::write(dir.join("bin/codex-code-mode-host"), b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(
                dir.join("bin/codex-code-mode-host"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        fs::write(dir.join("codex-package.json"), b"{\"version\":\"1.2.3\"}\n").unwrap();
        fs::write(dir.join("browser/open/curl"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            dir.join("browser/open/curl"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::write(dir.join("browser/manual/curl"), b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            dir.join("browser/manual/curl"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();

        let digest = "a".repeat(64);
        let meta = format!(
            "codex-local-generation\n\
             generation_id\t{generation_id}\n\
             upstream_package_identity\topenai/codex:codex-package-aarch64-unknown-linux-musl.tar.gz\n\
             upstream_package_version\t1.2.3\n\
             source_artifact_digest\t{digest}\n\
             expected_platform\tandroid\n\
             expected_architecture\taarch64\n\
             patch_policy_id\ttermux-fd-remap\n\
             patch_report\ttermux-fd-remap;changed_bytes=0\n\
             runtime_digest\t{digest}\n\
             wrapper_artifact_digest\t{digest}\n\
             wrapper_api_identity\twrapper-api\n\
             persistent_schema_identity\tschema\n\
             qualification\tqualified\n\
             creation_metadata\tcodex-wrapper\n\
             helper_count\t2\n\
             helper\ttermux-browser-open\t{digest}\n\
             helper\ttermux-browser-manual\t{digest}\n"
        );
        fs::write(dir.join("generation.meta"), meta).unwrap();
    }

    #[test]
    fn public_dispatch_routes_wrapper_commands_and_passes_daemon_and_remote_control() {
        assert_eq!(
            plan_public_dispatch(os_vec(&["update", "--upstream"])).unwrap(),
            PublicDispatchRoute::Update(os_vec(&["--upstream"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["doctor", "--json"])).unwrap(),
            PublicDispatchRoute::Doctor(os_vec(&["--json"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["wrapper", "status", "--json"])).unwrap(),
            PublicDispatchRoute::Wrapper(os_vec(&["status", "--json"]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["app-server", "daemon", "--listen", "stdio://"]))
                .unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&[
                "app-server",
                "daemon",
                "--listen",
                "stdio://"
            ]))
        );
        assert_eq!(
            plan_public_dispatch(os_vec(&["remote-control"])).unwrap(),
            PublicDispatchRoute::Upstream(os_vec(&["remote-control"]))
        );
    }

    #[test]
    fn optional_module_cli_rejects_unknown_or_unavailable_modules() {
        assert_eq!(run_wrapper_command(os_vec(&["--with", "manager"])), 1);
        assert_eq!(run_wrapper_command(os_vec(&["--with", "tui,provider"])), 1);
        assert_eq!(run_wrapper_command(os_vec(&["--with", "not-a-module"])), 2);
        assert_eq!(
            run_wrapper_command(os_vec(&[
                "--with",
                "manager",
                "--",
                "--sandbox",
                "read-only"
            ])),
            2
        );
    }

    #[test]
    fn passthrough_sandbox_guard_blocks_unsupported_linux_sandboxes() {
        assert!(plan_public_dispatch(os_vec(&["exec", "--sandbox", "read-only", "hi"])).is_err());
        assert!(plan_public_dispatch(os_vec(&["-s=workspace-write", "exec", "hi"])).is_err());
        assert!(plan_public_dispatch(os_vec(&["sandbox", "linux", "true"])).is_err());
        assert!(
            plan_public_dispatch(os_vec(&["--sandbox", "danger-full-access", "exec", "hi"]))
                .is_ok()
        );
        assert!(plan_public_dispatch(os_vec(&["exec", "--", "--sandbox", "read-only"])).is_ok());
    }

    #[test]
    fn help_and_version_fast_path_skips_daemon_settings() {
        assert!(is_help_or_version_invocation(&os_vec(&["--version"])));
        assert!(is_help_or_version_invocation(&os_vec(&["-V"])));
        assert!(is_help_or_version_invocation(&os_vec(&[
            "-c", "foo=bar", "--help"
        ])));
        assert!(is_help_or_version_invocation(&os_vec(&["help", "exec"])));
        assert!(!is_help_or_version_invocation(&os_vec(&[
            "exec",
            "--version"
        ])));
        assert!(!is_help_or_version_invocation(&os_vec(&[
            "app-server",
            "daemon"
        ])));
        assert!(!is_help_or_version_invocation(&[]));
    }

    #[test]
    fn resolver_fd33_cache_hits_lock_free_on_matching_bytes_and_updates_when_stale() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("fd33");
        let prev_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &root) };
        fs::create_dir_all(root.join(".local/share/codex")).unwrap();
        super::fs_util::private_dir(&root.join(".local/share/codex/wrapper")).unwrap();

        let f1 = open_synthesized_resolver(b"nameserver 8.8.8.8\n").unwrap();
        let ino1 = f1.metadata().unwrap().ino();
        let mut s1 = String::new();
        Read::read_to_string(&mut &f1, &mut s1).unwrap();
        assert_eq!(s1, "nameserver 8.8.8.8\noptions timeout:2 attempts:10\n");
        drop(f1);

        let f2 = open_synthesized_resolver(b"nameserver 8.8.8.8\n").unwrap();
        assert_eq!(f2.metadata().unwrap().ino(), ino1);
        drop(f2);

        let f3 = open_synthesized_resolver(b"nameserver 1.1.1.1\n").unwrap();
        assert_eq!(f3.metadata().unwrap().ino(), ino1);
        let mut s3 = String::new();
        Read::read_to_string(&mut &f3, &mut s3).unwrap();
        assert_eq!(s3, "nameserver 1.1.1.1\noptions timeout:2 attempts:10\n");

        match prev_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        remove_temp_root(root);
    }

    #[test]
    fn wrapper_config_writes_managed_toml_and_preserves_other_files() {
        let root = temp_root("fd34");
        let cfg = root.join("config");
        super::fs_util::private_dir(&cfg).unwrap();
        fs::write(cfg.join("config.toml"), b"# stale\nfoo = 1\n").unwrap();
        fs::write(cfg.join("requirements.toml"), b"user_setting = true\n").unwrap();

        prepare_wrapper_config(&cfg).unwrap();
        let written = fs::read_to_string(cfg.join("config.toml")).unwrap();
        assert_eq!(
            written,
            "# codex-termux-wrapper\nsandbox_mode = \"danger-full-access\"\nmcp_oauth_credentials_store = \"file\"\n"
        );
        assert_eq!(
            fs::read_to_string(cfg.join("requirements.toml")).unwrap(),
            "user_setting = true\n"
        );
        remove_temp_root(root);
    }

    #[test]
    fn load_generation_validates_package_root_layout_and_rejects_legacy_or_bad_metadata() {
        let root = temp_root("gen-load");
        let g1 = root.join("gen-with-cmh");
        let g2 = root.join("gen-without-cmh");
        write_sample_generation(&g1, "gen-with-cmh", true);
        write_sample_generation(&g2, "gen-without-cmh", false);

        let l1 = load_local_generation(&g1).unwrap();
        assert!(l1.code_mode_host_path.is_some());
        assert_eq!(l1.generation_layout.as_str(), "package-root");
        with_qualified_loaded_runtime(&l1, |_| Ok(())).unwrap();

        let l2 = load_local_generation(&g2).unwrap();
        assert!(l2.code_mode_host_path.is_none());

        // Unexpected legacy files inside a package-root generation are rejected.
        fs::create_dir_all(g2.join("compat")).unwrap();
        assert!(load_local_generation(&g2).is_err());

        // Incompatible wrapper API identity is rejected at qualification time.
        let bad_api = root.join("gen-bad-api");
        write_sample_generation(&bad_api, "gen-bad-api", false);
        let original = fs::read_to_string(bad_api.join("generation.meta")).unwrap();
        fs::write(
            bad_api.join("generation.meta"),
            original.replace("wrapper-api\n", "wrapper-api-old\n"),
        )
        .unwrap();
        let loaded_bad = load_local_generation(&bad_api).unwrap();
        assert!(with_qualified_loaded_runtime(&loaded_bad, |_| Ok(())).is_err());

        remove_temp_root(root);
    }

    #[test]
    fn runtime_fd_sources_move_descriptors_above_safe_min_and_install_33_34_35() {
        let _guard = ENV_LOCK.lock().unwrap();
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
            OsStr::new("/gen"),
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
    fn e2e_activation_doctor_status_and_rollback_in_isolated_roots() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("e2e-update");
        let gen1 = root.join("src-g1");
        let gen2 = root.join("src-g2");
        write_sample_generation(&gen1, "gen-1", false);
        write_sample_generation(&gen2, "gen-2", true);

        let prev_home = std::env::var_os("HOME");
        let prev_prefix = std::env::var_os("PREFIX");
        let prev_tmpdir = std::env::var_os("TMPDIR");
        let syn_prefix = root.join("prefix");
        fs::create_dir_all(syn_prefix.join("bin")).unwrap();
        fs::create_dir_all(syn_prefix.join("etc/tls/certs")).unwrap();
        fs::write(syn_prefix.join("etc/resolv.conf"), b"nameserver 1.1.1.1\n").unwrap();
        fs::write(syn_prefix.join("etc/tls/cert.pem"), b"").unwrap();
        let syn_home = root.join("home");
        fs::create_dir_all(syn_home.join(".local/lib/codex/wrapper/generations")).unwrap();
        fs::create_dir_all(syn_home.join(".local/share/codex")).unwrap();

        unsafe {
            std::env::set_var("HOME", &syn_home);
            std::env::set_var("PREFIX", &syn_prefix);
            std::env::set_var("TMPDIR", &root);
        }

        let roots = LocalWrapperRoots::from_environment().unwrap();
        let state_paths = generation_state::WrapperStatePaths::new(&roots.state_root).unwrap();
        generation_state::prepare_wrapper_state_paths(&state_paths).unwrap();
        prepare_wrapper_config(&roots.config_dir).unwrap();
        copy_local_directory_tree(&gen1, &roots.generation_root.join("gen-1")).unwrap();
        copy_local_directory_tree(&gen2, &roots.generation_root.join("gen-2")).unwrap();
        let lock = generation_state::acquire_activation_lock(&state_paths).unwrap();
        let mut io = generation_state::FsActivationIo;
        let s1 = generation_state::plan_initial_pointer_state("gen-1").unwrap();
        generation_state::activate_pointer_state_locked(&state_paths, None, &s1, &mut io, &lock)
            .unwrap();
        let s2 = generation_state::plan_activation_pointer_state(&s1, "gen-2").unwrap();
        generation_state::activate_pointer_state_locked(
            &state_paths,
            Some(&s1),
            &s2,
            &mut io,
            &lock,
        )
        .unwrap();
        drop(lock);

        let loaded = load_local_generation(&roots.generation_root.join("gen-2")).unwrap();
        let snap = capture_termux_process_env();
        probe_release_candidate(&loaded, &roots, &snap).unwrap();

        assert_eq!(run_public_main(os_vec(&["wrapper", "status", "--json"])), 0);
        let outcome = execute_activated_route(
            PublicDispatchRoute::Doctor(os_vec(&["--json"])),
            &roots,
            &snap,
        )
        .unwrap();
        let PublicDispatchCompletion::Doctor(doc) = outcome;
        assert!(doc.output.contains("\"schema_version\":1"));
        assert!(doc.output.contains("\"layout\":\"package-root\""));
        assert!(doc.output.contains("\"status\":\"healthy\""));

        assert_eq!(run_public_main(os_vec(&["update", "--rollback"])), 0);
        let recovered = generation_state::recover_activation_state(&state_paths)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.current, "gen-1");
        assert_eq!(run_public_main(os_vec(&["doctor", "--json"])), 0);

        match prev_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        match prev_prefix {
            Some(v) => unsafe { std::env::set_var("PREFIX", v) },
            None => unsafe { std::env::remove_var("PREFIX") },
        }
        match prev_tmpdir {
            Some(v) => unsafe { std::env::set_var("TMPDIR", v) },
            None => unsafe { std::env::remove_var("TMPDIR") },
        }
        remove_temp_root(root);
    }
}
