use crate::daemon_settings::{self, DaemonHomeLabel, DAEMON_PID_FILES};
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
use std::collections::{BTreeMap, HashSet};
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonHomeEntry {
    label: String,
    outcome: DaemonHomeOutcome,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct DaemonSyncReport {
    active_version: String,
    entries: Vec<DaemonHomeEntry>,
    warnings: Vec<String>,
}

impl DaemonSyncReport {
    pub(super) fn summary_lines(&self) -> Vec<String> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let mut lines = Vec::with_capacity(self.entries.len() + 1);
        lines.push("  └─ daemons (shared runtime)".to_owned());
        let last_index = self.entries.len() - 1;
        for (idx, entry) in self.entries.iter().enumerate() {
            let branch = if idx == last_index {
                "     └─ "
            } else {
                "     ├─ "
            };
            let detail = match &entry.outcome {
                DaemonHomeOutcome::Restarted { previous_version } => match previous_version {
                    Some(prev) if prev != &self.active_version => {
                        format!("{prev} -> {} (restarted)", self.active_version)
                    }
                    _ => format!("{} (restarted)", self.active_version),
                },
                DaemonHomeOutcome::AlreadyCurrent => {
                    format!("{} (up to date)", self.active_version)
                }
                DaemonHomeOutcome::LinkAligned { running: true } => {
                    format!("{} (up to date, link aligned)", self.active_version)
                }
                DaemonHomeOutcome::LinkAligned { running: false } => {
                    "link aligned (stopped)".to_owned()
                }
                DaemonHomeOutcome::DeferredSelf {
                    previous_version, ..
                } => match previous_version {
                    Some(prev) => format!("{prev} (deferred: current session)"),
                    None => "deferred (current session)".to_owned(),
                },
                DaemonHomeOutcome::Failed => "failed (see warning)".to_owned(),
                DaemonHomeOutcome::UnchangedStopped => continue,
            };
            lines.push(format!("{branch}{}: {detail}", entry.label));
        }
        lines
    }

    pub(super) fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonHomeOutcome {
    Restarted {
        previous_version: Option<String>,
    },
    LinkAligned {
        running: bool,
    },
    AlreadyCurrent,
    UnchangedStopped,
    DeferredSelf {
        previous_version: Option<String>,
        link_aligned: bool,
    },
    Failed,
}

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

fn pid_field(fields: &BTreeMap<String, JsonValue>, key: &str) -> Option<i32> {
    match fields.get(key) {
        Some(JsonValue::Number(value)) => value.parse::<i32>().ok().filter(|pid| *pid > 0),
        _ => None,
    }
}

fn read_pid_record(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    fs_util::read_bounded_regular(path, 64 * 1024, true, "invalid daemon PID file")
}

fn parse_saved_feature_args(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut settings = json_fields(bytes).ok_or("invalid daemon settings")?;
    let mut args = Vec::new();
    let Some(overrides) = settings.remove("featureOverrides") else {
        return Ok(args);
    };
    if matches!(overrides, JsonValue::Null) {
        return Ok(args);
    }
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

fn saved_feature_args(home: &Path) -> Result<Vec<String>, String> {
    let path = home.join("app-server-daemon/settings.json");
    let bytes = fs_util::read_bounded_regular(&path, 1024 * 1024, true, "invalid daemon settings")
        .map_err(|err| err.to_string())?;
    parse_saved_feature_args(&bytes)
}

fn parse_proc_start_time(stat: &str) -> Option<u64> {
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(19)?.parse().ok()
}

fn parse_proc_uid(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        line.strip_prefix("Uid:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

fn parse_proc_stat_ppid(stat: &str) -> Option<i32> {
    let (_, rest) = stat.rsplit_once(')')?;
    let mut fields = rest.split_whitespace();
    let _state = fields.next()?;
    let ppid = fields.next()?.parse::<i32>().ok()?;
    (ppid > 0).then_some(ppid)
}

fn ancestor_pids() -> HashSet<i32> {
    let mut ancestors = HashSet::new();
    let mut current = std::process::id() as i32;
    for _ in 0..128 {
        let Ok(stat) = fs::read_to_string(format!("/proc/{current}/stat")) else {
            break;
        };
        let Some(ppid) = parse_proc_stat_ppid(&stat) else {
            break;
        };
        if ppid <= 1 || !ancestors.insert(ppid) {
            break;
        }
        current = ppid;
    }
    ancestors
}

struct LivePid {
    pid: i32,
    start_time: u64,
    uid: u32,
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
            && fs::read_to_string(self.proc_path().join("stat"))
                .ok()
                .and_then(|s| parse_proc_start_time(&s))
                == Some(self.start_time)
            && fs::read_to_string(self.proc_path().join("status"))
                .ok()
                .and_then(|s| parse_proc_uid(&s))
                == Some(self.uid)
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
            .and_then(|fields| pid_field(&fields, "pid"))
            .or_else(|| std::str::from_utf8(&bytes).ok()?.trim().parse().ok())
            .filter(|pid| *pid > 0)
            .ok_or_else(|| format!("cannot verify {name}"))?;
        let process = match File::open(format!("/proc/{pid}")) {
            Ok(process) => process,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("cannot inspect process from {name}: {err}")),
        };
        let proc_root = PathBuf::from(format!("/proc/self/fd/{}", process.as_raw_fd()));
        let start_time = fs::read_to_string(proc_root.join("stat"))
            .ok()
            .and_then(|s| parse_proc_start_time(&s))
            .ok_or_else(|| format!("cannot verify process start time from {name}"))?;
        let uid = fs::read_to_string(proc_root.join("status"))
            .ok()
            .and_then(|s| parse_proc_uid(&s))
            .ok_or_else(|| format!("cannot verify process uid from {name}"))?;
        let exe = match fs::metadata(proc_root.join("exe")) {
            Ok(exe) => exe,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("cannot inspect executable from {name}: {err}")),
        };
        if live.is_some() {
            return Err("both upstream PID records identify live processes".into());
        }
        live = Some(LivePid {
            pid,
            start_time,
            uid,
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
    ancestors: &'a HashSet<i32>,
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
        let child = cmd.spawn().map_err(|err| err.to_string())?;
        drop(cmd);
        let (out, status) =
            proc_exec::collect_bounded_output_and_wait(child, reader, timeout, 64 * 1024).map_err(
                |err| match err {
                    proc_exec::OutputError::Io(err) => err.to_string(),
                    proc_exec::OutputError::TimedOut(_) => format!("daemon {subcommand} timed out"),
                    proc_exec::OutputError::OutputTooLarge(_) => {
                        format!("daemon {subcommand} output exceeded its bound")
                    }
                },
            )?;
        if !status.success() {
            return Err(format!("upstream daemon {subcommand} returned {status}"));
        }
        json_fields(&out).ok_or_else(|| format!("invalid upstream daemon {subcommand} result"))
    }

    fn sync_home(
        &self,
        home: &Path,
        label: &DaemonHomeLabel,
        presentation: &mut UpdatePresentation,
    ) -> Result<DaemonHomeOutcome, String> {
        let lock = daemon_settings::lock_daemon_home(home).map_err(|err| err.to_string())?;
        let package =
            daemon_settings::prepare_daemon_package_dir(home).map_err(|err| err.to_string())?;
        daemon_settings::ensure(home).map_err(|err| err.to_string())?;
        let align = |package: &Path| {
            daemon_settings::align_package_link_to_standalone(package, self.standalone_current)
                .map_err(|err| format!("could not align daemon package link: {err}"))
        };

        let Some(live) = find_live_pid(home)? else {
            return Ok(if align(&package)? {
                DaemonHomeOutcome::LinkAligned { running: false }
            } else {
                DaemonHomeOutcome::UnchangedStopped
            });
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
            || pid_field(&version, "pid").is_some_and(|pid| pid != live.pid)
            || !live.unchanged()
        {
            return Err(
                "upstream could not verify the running daemon and its PID namespace".into(),
            );
        }
        let active_version = self.loaded.manifest.upstream_package_version.as_str();
        let previous_version = string_field(&version, "appServerVersion").map(str::to_owned);
        if !live.legacy()
            && live.matches(&self.exe)
            && previous_version.as_deref() == Some(active_version)
        {
            return Ok(if align(&package)? {
                DaemonHomeOutcome::LinkAligned { running: true }
            } else {
                DaemonHomeOutcome::AlreadyCurrent
            });
        }

        // Stopping an ancestor daemon terminates this update process mid-flight,
        // leaving that daemon stopped and subsequent profiles un-synchronized.
        if self.ancestors.contains(&live.pid) {
            let link_aligned = if live.legacy() {
                false
            } else {
                align(&package)?
            };
            return Ok(DaemonHomeOutcome::DeferredSelf {
                previous_version,
                link_aligned,
            });
        }

        let original_env = live.launch_env()?;
        let feature_args = saved_feature_args(home)?;
        if !live.legacy() {
            align(&package)?;
        }
        if !live.unchanged() {
            return Err("running daemon changed before shutdown".into());
        }
        let short_label = label.short_name();
        presentation.transient(&format!("Stopping daemon ({short_label})..."));
        if !live.unchanged() {
            return Err("running daemon identity changed immediately before shutdown".into());
        }
        // Upstream stop acquires daemon.lock itself; release our inspection lock
        // immediately before invoking it, then reconcile the resulting PID record.
        drop(lock);
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
            return Ok(if changed {
                DaemonHomeOutcome::LinkAligned { running: false }
            } else {
                DaemonHomeOutcome::UnchangedStopped
            });
        }

        presentation.transient(&format!("Starting daemon ({short_label})..."));
        let started = self
            .command(
                home,
                "start",
                START_TIMEOUT,
                Some(&original_env),
                &feature_args,
            )
            .map_err(|err| {
                format!(
                    "stopped old daemon, but restart failed: {err} (run `codex app-server daemon start` in this profile)"
                )
            })?;
        let started_pid = pid_field(&started, "pid");
        if !matches!(
            string_field(&started, "status"),
            Some("started" | "alreadyRunning")
        ) || string_field(&started, "appServerVersion") != Some(active_version)
            || !find_live_pid(home)?.is_some_and(|pid| {
                !pid.legacy()
                    && pid.matches(&self.exe)
                    && started_pid.is_none_or(|expected| pid.pid == expected)
            })
        {
            return Err(
                "stopped old daemon, but startup did not confirm the active executable and version (run `codex app-server daemon start` in this profile)".into(),
            );
        }
        Ok(DaemonHomeOutcome::Restarted { previous_version })
    }
}

pub(super) fn synchronize(
    generation_id: &str,
    roots: &LocalWrapperRoots,
    process_env: &TermuxProcessEnvSnapshot,
    presentation: &mut UpdatePresentation,
) -> DaemonSyncReport {
    let mut report = DaemonSyncReport::default();
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
            report.warnings.push(format!(
                "codex update: warning: daemon synchronization skipped: {err}"
            ));
            return report;
        }
    };
    report.active_version = loaded.manifest.upstream_package_version.clone();
    let standalone = roots
        .generation_root
        .parent()
        .expect("standalone release directory");
    let home = roots
        .state_root
        .ancestors()
        .nth(4)
        .expect("wrapper state HOME");
    let ancestors = ancestor_pids();
    let update = DaemonUpdate {
        loaded: &loaded,
        roots,
        process_env,
        standalone_current: &standalone.join("current"),
        exe,
        ancestors: &ancestors,
    };
    for candidate in
        daemon_settings::candidate_daemon_homes(home, resolve_codex_home().as_deref(), standalone)
    {
        if daemon_settings::has_daemon_activity(&candidate) {
            let label = daemon_settings::daemon_home_label(home, &candidate);
            let short = label.short_name().to_owned();
            match update.sync_home(&candidate, &label, presentation) {
                Ok(DaemonHomeOutcome::UnchangedStopped) => {}
                Ok(outcome @ DaemonHomeOutcome::DeferredSelf { link_aligned, .. }) => {
                    report.entries.push(DaemonHomeEntry {
                        label: short,
                        outcome,
                    });
                    let detail = if link_aligned {
                        "package link aligned for next start; "
                    } else {
                        ""
                    };
                    report.warnings.push(format!(
                        "codex update: warning: deferred restarting {} because this update is running inside that daemon ({detail}run `codex app-server daemon restart` or rerun `codex update` outside this session)",
                        label.warning_target(&candidate)
                    ));
                }
                Ok(outcome) => {
                    report.entries.push(DaemonHomeEntry {
                        label: short,
                        outcome,
                    });
                }
                Err(err) => {
                    report.entries.push(DaemonHomeEntry {
                        label: short,
                        outcome: DaemonHomeOutcome::Failed,
                    });
                    report.warnings.push(format!(
                        "codex update: warning: managed daemon sync failed for {}: {err}",
                        label.warning_target(&candidate)
                    ));
                }
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_formats_per_profile_tree_for_all_daemon_outcomes() {
        assert!(DaemonSyncReport::default().summary_lines().is_empty());

        let upgraded_three = DaemonSyncReport {
            active_version: "0.171.0".into(),
            entries: vec![
                DaemonHomeEntry {
                    label: "default".into(),
                    outcome: DaemonHomeOutcome::Restarted {
                        previous_version: Some("0.170.0".into()),
                    },
                },
                DaemonHomeEntry {
                    label: "janmori101".into(),
                    outcome: DaemonHomeOutcome::Restarted {
                        previous_version: Some("0.170.0".into()),
                    },
                },
                DaemonHomeEntry {
                    label: "szob".into(),
                    outcome: DaemonHomeOutcome::Restarted {
                        previous_version: Some("0.170.0".into()),
                    },
                },
            ],
            warnings: Vec::new(),
        };
        assert_eq!(
            upgraded_three.summary_lines(),
            vec![
                "  └─ daemons (shared runtime)",
                "     ├─ default: 0.170.0 -> 0.171.0 (restarted)",
                "     ├─ janmori101: 0.170.0 -> 0.171.0 (restarted)",
                "     └─ szob: 0.170.0 -> 0.171.0 (restarted)",
            ]
        );

        let single_up_to_date = DaemonSyncReport {
            active_version: "0.171.0".into(),
            entries: vec![DaemonHomeEntry {
                label: "default".into(),
                outcome: DaemonHomeOutcome::AlreadyCurrent,
            }],
            warnings: Vec::new(),
        };
        assert_eq!(
            single_up_to_date.summary_lines(),
            vec![
                "  └─ daemons (shared runtime)",
                "     └─ default: 0.171.0 (up to date)",
            ]
        );

        let mixed = DaemonSyncReport {
            active_version: "0.171.0".into(),
            entries: vec![
                DaemonHomeEntry {
                    label: "default".into(),
                    outcome: DaemonHomeOutcome::DeferredSelf {
                        previous_version: Some("0.170.0".into()),
                        link_aligned: false,
                    },
                },
                DaemonHomeEntry {
                    label: "janmori101".into(),
                    outcome: DaemonHomeOutcome::Restarted {
                        previous_version: Some("0.171.0".into()),
                    },
                },
                DaemonHomeEntry {
                    label: "szob".into(),
                    outcome: DaemonHomeOutcome::LinkAligned { running: false },
                },
                DaemonHomeEntry {
                    label: "aligned-live".into(),
                    outcome: DaemonHomeOutcome::LinkAligned { running: true },
                },
                DaemonHomeEntry {
                    label: "conflict".into(),
                    outcome: DaemonHomeOutcome::Failed,
                },
            ],
            warnings: Vec::new(),
        };
        assert_eq!(
            mixed.summary_lines(),
            vec![
                "  └─ daemons (shared runtime)",
                "     ├─ default: 0.170.0 (deferred: current session)",
                "     ├─ janmori101: 0.171.0 (restarted)",
                "     ├─ szob: link aligned (stopped)",
                "     ├─ aligned-live: 0.171.0 (up to date, link aligned)",
                "     └─ conflict: failed (see warning)",
            ]
        );
    }

    #[test]
    fn proc_stat_ppid_handles_spaces_and_parentheses_in_comm() {
        assert_eq!(
            parse_proc_stat_ppid("1234 (codex (app) server) S 987 1234 1234 0 -1"),
            Some(987)
        );
        assert_eq!(parse_proc_stat_ppid("1 (init) S 0 1 1 0 -1"), None);
        assert_eq!(parse_proc_stat_ppid("malformed"), None);
    }

    #[test]
    fn saved_feature_args_handles_null_and_valid_overrides() {
        assert_eq!(
            parse_saved_feature_args(
                br#"{"updater":{"autoUpdateEnabled":false},"featureOverrides":null}"#
            )
            .unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            parse_saved_feature_args(br#"{"featureOverrides":{"shell_snapshot":false}}"#).unwrap(),
            vec!["-c", "features.shell_snapshot=false"]
        );
        assert!(parse_saved_feature_args(br#"{"featureOverrides":"bad"}"#).is_err());
    }
}
