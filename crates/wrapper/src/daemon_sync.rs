use crate::daemon_settings::{self, DAEMON_PID_FILES};
use crate::fs_util;
use crate::generation::{
    load_activated_generation, termux_browser_helper_paths, with_qualified_loaded_runtime,
    LoadedLocalGeneration, LocalWrapperRoots,
};
use crate::json::{JsonParser, JsonValue};
use crate::proc_exec;
use crate::progress::UpdatePresentation;
use crate::runtime_env::{
    apply_child_env_plan_and_fence, plan_termux_env, prepare_wrapper_config, resolve_codex_home,
    RuntimeFdSources, TermuxProcessEnvSnapshot,
};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, Metadata};
use std::io::{ErrorKind, Read as _};
use std::os::fd::AsRawFd as _;
use std::os::unix::{ffi::OsStringExt as _, fs::MetadataExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const VERSION_TIMEOUT: Duration = Duration::from_secs(15);
// Upstream 0.171 allows up to 300 s configured graceful shutdown, then 10 s
// for forced exit. Allow for PID reservation and the final version query.
const STOP_TIMEOUT: Duration = Duration::from_secs(350);
const START_TIMEOUT: Duration = Duration::from_secs(45);

fn json_fields(bytes: &[u8]) -> Option<BTreeMap<String, JsonValue>> {
    let JsonValue::Object(fields) = JsonParser::new(bytes).parse_document().ok()? else {
        return None;
    };
    Some(fields.into_iter().collect())
}

fn string_field<'a>(fields: &'a BTreeMap<String, JsonValue>, key: &str) -> Option<&'a str> {
    match fields.get(key) {
        Some(JsonValue::String(value)) => Some(value),
        _ => None,
    }
}

fn read_pid_record(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    fs_util::read_bounded_regular(path, 64 * 1024, true, "invalid daemon PID file")
}

fn saved_feature_args(home: &Path) -> Result<Vec<String>, String> {
    let path = home.join("app-server-daemon/settings.json");
    let bytes = fs_util::read_bounded_regular(&path, 1024 * 1024, true, "invalid daemon settings")
        .map_err(|err| err.to_string())?;
    let mut settings = json_fields(&bytes).ok_or("invalid daemon settings")?;
    let mut args = Vec::new();
    let Some(overrides) = settings.remove("featureOverrides") else {
        return Ok(args);
    };
    let JsonValue::Object(overrides) = overrides else {
        return Err("invalid saved daemon feature overrides".into());
    };
    for (name, value) in overrides {
        let JsonValue::Bool(enabled) = value else {
            return Err("invalid saved daemon feature override".into());
        };
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err("invalid saved daemon feature name".into());
        }
        args.push("-c".into());
        args.push(format!("features.{name}={enabled}"));
    }
    Ok(args)
}

struct LivePid {
    file: PathBuf,
    bytes: Vec<u8>,
    process: File,
    exe: Metadata,
}

impl LivePid {
    fn proc_path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.process.as_raw_fd()))
    }

    fn matches(&self, exe: &Metadata) -> bool {
        self.exe.dev() == exe.dev() && self.exe.ino() == exe.ino()
    }

    fn legacy(&self) -> bool {
        self.file.ends_with(DAEMON_PID_FILES[1])
    }

    fn unchanged(&self) -> bool {
        fs::metadata(self.proc_path().join("exe")).is_ok_and(|exe| self.matches(&exe))
            && read_pid_record(&self.file).is_ok_and(|bytes| bytes == self.bytes)
    }

    fn launch_env(&self) -> Result<BTreeMap<OsString, OsString>, String> {
        // The profile's daemon may have a different SQLite binding, provider
        // environment or launch configuration. Replay its own environment for
        // the lifecycle handoff. Never include these values in diagnostics.
        const LIMIT: u64 = 2 * 1024 * 1024;
        let command = fs::read(self.proc_path().join("cmdline"))
            .map_err(|_| "cannot read the running daemon's argv")?;
        let mut argv = command.split(|&byte| byte == 0);
        if argv.nth(1) != Some(b"app-server") {
            return Err("the PID record does not identify an app-server process".into());
        }
        let mut bytes = Vec::new();
        File::open(self.proc_path().join("environ"))
            .and_then(|file| file.take(LIMIT + 1).read_to_end(&mut bytes))
            .map_err(|_| "cannot read the running daemon's launch environment")?;
        if bytes.last() != Some(&0) || bytes.len() as u64 > LIMIT || !self.unchanged() {
            return Err("daemon process or launch environment could not be confirmed".into());
        }
        Ok(bytes
            .split(|&byte| byte == 0)
            .filter_map(|item| {
                let separator = item.iter().position(|&byte| byte == b'=')?;
                (separator > 0).then(|| {
                    (
                        OsString::from_vec(item[..separator].to_vec()),
                        OsString::from_vec(item[separator + 1..].to_vec()),
                    )
                })
            })
            .collect())
    }
}

fn find_live_pid(home: &Path) -> Result<Option<LivePid>, String> {
    let mut live = None;
    for name in DAEMON_PID_FILES {
        let file = home.join("app-server-daemon").join(name);
        let bytes = match read_pid_record(&file) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("could not inspect {name}: {err}")),
        };
        // A zero-byte reservation may mean startup is still in progress. The
        // current upstream CLI expects JSON, but an old dead raw PID is harmless.
        let pid = json_fields(&bytes)
            .and_then(|mut fields| match fields.remove("pid") {
                Some(JsonValue::Number(pid)) => pid.parse::<i32>().ok(),
                _ => None,
            })
            .or_else(|| std::str::from_utf8(&bytes).ok()?.trim().parse().ok())
            .filter(|pid| *pid > 0)
            .ok_or_else(|| format!("cannot verify {name}"))?;
        let process = match File::open(format!("/proc/{pid}")) {
            Ok(process) => process,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("cannot inspect process from {name}: {err}")),
        };
        let exe = match fs::metadata(format!("/proc/self/fd/{}/exe", process.as_raw_fd())) {
            Ok(exe) => exe,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("cannot inspect executable from {name}: {err}")),
        };
        if live.is_some() {
            return Err("both upstream PID records identify live processes".into());
        }
        live = Some(LivePid {
            file,
            bytes,
            process,
            exe,
        });
    }
    Ok(live)
}

struct DaemonUpdate<'a> {
    loaded: &'a LoadedLocalGeneration,
    roots: &'a LocalWrapperRoots,
    process_env: &'a TermuxProcessEnvSnapshot,
    standalone_current: &'a Path,
    exe: Metadata,
}

impl DaemonUpdate<'_> {
    fn command(
        &self,
        home: &Path,
        subcommand: &str,
        timeout: Duration,
        original_env: Option<&BTreeMap<OsString, OsString>>,
        feature_args: &[String],
    ) -> Result<BTreeMap<String, JsonValue>, String> {
        let mut snapshot = self.process_env.clone();
        let mut cmd = Command::new(&self.loaded.runtime_path);
        if let Some(env) = original_env {
            cmd.env_clear().envs(env);
            snapshot.inherited_path = env.get(OsStr::new("PATH")).cloned();
            snapshot.inherited_ssl_cert_file = env.get(OsStr::new("SSL_CERT_FILE")).cloned();
            snapshot.inherited_ssl_cert_dir = env.get(OsStr::new("SSL_CERT_DIR")).cloned();
        } else {
            cmd.env_remove("CODEX_SHARED_HOME")
                .env_remove("CODEX_SQLITE_HOME");
        }
        let env_plan = with_qualified_loaded_runtime(self.loaded, |assets| {
            let helpers = termux_browser_helper_paths(assets.selection()).map_err(|_| {
                crate::error::LocalProductError::LocalUpdate("browser helper unavailable")
            })?;
            plan_termux_env(
                &snapshot,
                helpers,
                self.roots.cert_file.as_os_str(),
                Some(self.roots.cert_dir.as_os_str()),
            )
            .map_err(|_| {
                crate::error::LocalProductError::LocalUpdate("Termux environment unavailable")
            })
        })
        .map_err(|err| err.to_string())?;

        prepare_wrapper_config(&self.roots.config_dir).map_err(|err| err.to_string())?;
        let fds = RuntimeFdSources::open(&self.roots.resolver_path, &self.roots.config_dir)
            .map_err(|err| err.to_string())?;
        let (reader, writer) =
            proc_exec::nonblocking_output_pipe().map_err(|err| err.to_string())?;
        cmd.args(feature_args)
            .args(["app-server", "daemon", subcommand]);
        apply_child_env_plan_and_fence(&mut cmd, Some(&env_plan));
        crate::timezone::configure(&mut cmd);
        if let Some(prefix) = snapshot.prefix.as_ref() {
            cmd.env("PREFIX", prefix);
        }
        cmd.env("CODEX_HOME", home)
            .stdin(Stdio::null())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::null());
        proc_exec::isolate_process_group(&mut cmd);
        fds.configure(&mut cmd);
        let mut child = cmd.spawn().map_err(|err| err.to_string())?;
        drop(cmd);
        let (out, status) =
            proc_exec::collect_bounded_output_and_wait(&mut child, reader, timeout, 64 * 1024)
                .map_err(|err| match err {
                    proc_exec::OutputError::Io(err) => err.to_string(),
                    proc_exec::OutputError::TimedOut(_) => format!("daemon {subcommand} timed out"),
                    proc_exec::OutputError::OutputTooLarge(_) => {
                        format!("daemon {subcommand} output exceeded its bound")
                    }
                })?;
        if !status.success() {
            return Err(format!("upstream daemon {subcommand} returned {status}"));
        }
        json_fields(&out).ok_or_else(|| format!("invalid upstream daemon {subcommand} result"))
    }

    fn sync_home(
        &self,
        home: &Path,
        presentation: &mut UpdatePresentation,
    ) -> Result<Option<String>, String> {
        let lock = daemon_settings::lock_daemon_home(home).map_err(|err| err.to_string())?;
        let package =
            daemon_settings::prepare_daemon_package_dir(home).map_err(|err| err.to_string())?;
        daemon_settings::ensure(home).map_err(|err| err.to_string())?;
        let align = |package: &Path| {
            daemon_settings::align_package_link_to_standalone(package, self.standalone_current)
                .map_err(|err| format!("could not align daemon package link: {err}"))
        };
        let aligned = || format!("Aligned managed daemon package link in {}.", home.display());

        let Some(live) = find_live_pid(home)? else {
            return Ok(align(&package)?.then(aligned));
        };
        let version = self.command(home, "version", VERSION_TIMEOUT, None, &[])?;
        let selected = string_field(&version, "managedCodexPath").map(Path::new);
        let legacy_root = home.join("packages/standalone/current");
        let selection = if live.legacy() {
            legacy_root.as_path()
        } else {
            &package.join("current")
        };
        if string_field(&version, "status") != Some("running")
            || string_field(&version, "backend") != Some("pid")
            || !selected.is_some_and(|path| {
                path == selection.join("bin/codex") || path == selection.join("codex")
            })
            || !live.unchanged()
        {
            return Err(
                "upstream could not verify the running daemon and its PID namespace".into(),
            );
        }
        let active_version = self.loaded.manifest.upstream_package_version.as_str();
        if !live.legacy()
            && live.matches(&self.exe)
            && string_field(&version, "appServerVersion") == Some(active_version)
        {
            return Ok(align(&package)?.then(aligned));
        }

        let original_env = live.launch_env()?;
        let feature_args = saved_feature_args(home)?;
        if !live.legacy() {
            align(&package)?;
        }
        drop(lock);
        if !live.unchanged() {
            return Err("running daemon changed before shutdown".into());
        }
        presentation.transient(&format!(
            "Restarting managed daemon in {} (Codex {active_version})...",
            home.display()
        ));
        let stopped = self.command(home, "stop", STOP_TIMEOUT, Some(&original_env), &[])?;
        let status = string_field(&stopped, "status");
        if !matches!(status, Some("stopped" | "notRunning")) {
            return Err("upstream daemon stop did not confirm its result".into());
        }
        // Stop and start each own upstream's lifecycle lock. In between, check
        // the state under that lock before publishing a new PID namespace.
        let lock = daemon_settings::lock_daemon_home(home).map_err(|err| err.to_string())?;
        if find_live_pid(home)?.is_some() {
            return Err("another daemon became active during synchronization".into());
        }
        let package =
            daemon_settings::prepare_daemon_package_dir(home).map_err(|err| err.to_string())?;
        let changed = align(&package)?;
        drop(lock);
        if status == Some("notRunning") {
            return Ok(changed.then(aligned));
        }

        let started = self.command(
            home,
            "start",
            START_TIMEOUT,
            Some(&original_env),
            &feature_args,
        )?;
        if !matches!(
            string_field(&started, "status"),
            Some("started" | "alreadyRunning")
        ) || string_field(&started, "appServerVersion") != Some(active_version)
            || !find_live_pid(home)?.is_some_and(|pid| !pid.legacy() && pid.matches(&self.exe))
        {
            return Err("daemon startup did not confirm the active executable and version".into());
        }
        Ok(Some(format!(
            "Updated managed daemon in {} (Codex {active_version}).",
            home.display()
        )))
    }
}

pub(super) fn synchronize(
    generation_id: &str,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> (Vec<String>, Vec<String>) {
    let sync = || {
        let (_, loaded) = load_activated_generation(roots).map_err(|err| err.to_string())?;
        if loaded.generation_id != generation_id {
            return Err("the activated generation changed".to_owned());
        }
        let exe = fs::metadata(&loaded.runtime_path).map_err(|err| err.to_string())?;
        Ok((loaded, exe))
    };
    let (loaded, exe) = match sync() {
        Ok(result) => result,
        Err(err) => {
            return (
                Vec::new(),
                vec![format!(
                    "codex update: warning: daemon synchronization skipped: {err}"
                )],
            )
        }
    };
    let standalone = roots
        .generation_root
        .parent()
        .expect("standalone release directory");
    let home = roots
        .state_root
        .ancestors()
        .nth(4)
        .expect("wrapper state HOME");
    let update = DaemonUpdate {
        loaded: &loaded,
        roots,
        process_env,
        standalone_current: &standalone.join("current"),
        exe,
    };
    let mut notices = Vec::new();
    let mut warnings = Vec::new();
    for home in
        daemon_settings::candidate_daemon_homes(home, resolve_codex_home().as_deref(), standalone)
    {
        if daemon_settings::has_daemon_activity(&home) {
            match update.sync_home(&home, presentation) {
                Ok(notice) => notices.extend(notice),
                Err(err) => warnings.push(format!(
                    "codex update: warning: daemon synchronization in {}: {err}",
                    home.display()
                )),
            }
        }
    }
    (notices, warnings)
}
