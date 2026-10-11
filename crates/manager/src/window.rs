//! One-shot stock Termux launch. Return carries an inert command, never work.
use super::*;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;

pub(super) const TOKEN_ENV: &str = "CODEX_TERMUX_WINDOW_TOKEN";
pub(super) const READY_ENV: &str = "CODEX_TERMUX_WINDOW_READY";
const ERROR: ManagerError =
    ManagerError::operation("codex termux: dedicated window unavailable (launch not retried)");
// These identify the new execution terminal, not the caller's preferences.
const TERMINAL_ENV: &[&str] = &[
    "TMUX",
    "TMUX_PANE",
    "TERM",
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "TERMUX_SESSION_ID",
    CORE_API_ENV,
    CORE_ENTRYPOINT_ENV,
];

pub(super) fn directory(context: &Context) -> Result<PathBuf, ManagerError> {
    let root = manager_base(context).join("manager");
    ensure_private_directory(&root)?;
    let directory = root.join("terminals");
    ensure_private_directory(&directory)?;
    if fs::symlink_metadata(&directory).map_err(|_| ERROR)?.uid() != unsafe { libc::getuid() } {
        return Err(ERROR);
    }
    Ok(directory)
}
pub(super) fn read(path: &Path) -> Result<serde_json::Value, ManagerError> {
    inspect_path(path)?;
    let m = fs::symlink_metadata(path).map_err(|_| ERROR)?;
    if !m.is_file() || m.uid() != unsafe { libc::getuid() } || m.mode() & 0o7777 != 0o600 {
        return Err(ERROR);
    }
    serde_json::from_slice(&read_bounded(path).map_err(|_| ERROR)?).map_err(|_| ERROR)
}
const LAUNCH_MAX_BYTES: usize = 64 * 1024;
fn write_launch(path: &Path, value: &serde_json::Value) -> Result<(), ManagerError> {
    let bytes = value.to_string();
    if bytes.len() > LAUNCH_MAX_BYTES {
        return Err(ERROR);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .map_err(|_| ERROR)?;
    let result = file
        .write_all(bytes.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| ERROR);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
fn read_launch(path: &Path) -> Result<serde_json::Value, ManagerError> {
    inspect_path(path)?;
    let m = fs::symlink_metadata(path).map_err(|_| ERROR)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::getuid() }
        || m.mode() & 0o7777 != 0o600
        || m.len() > LAUNCH_MAX_BYTES as u64
    {
        return Err(ERROR);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| ERROR)?
        .take((LAUNCH_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ERROR)?;
    if bytes.len() > LAUNCH_MAX_BYTES {
        return Err(ERROR);
    }
    serde_json::from_slice(&bytes).map_err(|_| ERROR)
}
fn token(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn random() -> Result<String, ManagerError> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| ERROR)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn bytes(value: &OsStr) -> serde_json::Value {
    serde_json::json!(value.as_bytes())
}
fn os(value: &serde_json::Value) -> Result<OsString, ManagerError> {
    let bytes = value
        .as_array()
        .ok_or(ERROR)?
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|b| u8::try_from(b).ok())
                .filter(|b| *b != 0)
                .ok_or(ERROR)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OsString::from_vec(bytes))
}

fn service(name: &str, program: &Path, args: &[&str], cwd: &Path) -> bool {
    // AM's own tokenization and string-array extra grammar receive only fixed words,
    // private tokens and validated paths. No raw workload argument enters this intent.
    let Some(program) = program.to_str() else {
        return false;
    };
    let Some(cwd) = cwd.to_str() else {
        return false;
    };
    if program.contains(',') || cwd.contains('\0') || args.iter().any(|arg| arg.contains(',')) {
        return false;
    }
    let command = format!("startservice -n com.termux/com.termux.app.RunCommandService -a com.termux.RUN_COMMAND --es com.termux.RUN_COMMAND_PATH {} --esa com.termux.RUN_COMMAND_ARGUMENTS {} --es com.termux.RUN_COMMAND_WORKDIR {} --es com.termux.RUN_COMMAND_RUNNER terminal-session --es com.termux.RUN_COMMAND_SHELL_NAME {} --es com.termux.RUN_COMMAND_SHELL_CREATE_MODE no-shell-with-name --ei com.termux.RUN_COMMAND_SESSION_ACTION 0", shell_quote(program), shell_quote(&args.join(",")), shell_quote(cwd), shell_quote(name));
    origin::command(&command, 15).is_some_and(|out| {
        out.status.success()
            && out.stderr.is_empty()
            && std::str::from_utf8(&out.stdout).is_ok_and(|s| s.contains("Starting service:"))
    })
}

pub(super) fn launch(
    context: &Context,
    profile: Option<&ProfileTarget>,
    mode: &str,
    args: &[OsString],
    program: Option<&Path>,
) -> Result<Option<String>, ManagerError> {
    let directory = directory(context)?;
    let token = random()?;
    let name = format!("codex-{token}");
    let cwd = std::env::current_dir().map_err(|_| ERROR)?;
    let path = prepare(context, profile, mode, args, program, "window", &token)?;
    sync_directory(&directory).map_err(|_| ERROR)?;
    if !service(
        &name,
        &PathBuf::from(std::env::var_os("PREFIX").ok_or(ERROR)?).join("bin/env"),
        &[
            &format!("HOME={}", context.home.to_str().ok_or(ERROR)?),
            context.core_entrypoint.to_str().ok_or(ERROR)?,
            "termux",
            "__terminal-start-v1",
            &token,
        ],
        &cwd,
    ) {
        // Remove an unconsumed request: a delayed service may fail, but cannot replay it.
        let _ = fs::remove_file(path);
        return Err(ERROR);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !directory.join(format!("route-{token}")).exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if !directory.join(format!("route-{token}")).exists() {
        let _ = fs::remove_file(path);
        return Err(ERROR);
    }
    Ok(None)
}

fn prepare(
    context: &Context,
    profile: Option<&ProfileTarget>,
    mode: &str,
    args: &[OsString],
    program: Option<&Path>,
    purpose: &str,
    token: &str,
) -> Result<PathBuf, ManagerError> {
    let cwd = std::env::current_dir().map_err(|_| ERROR)?;
    let environment: Vec<_> = std::env::vars_os()
        .filter(|(key, _)| {
            !TERMINAL_ENV
                .iter()
                .any(|excluded| key.as_os_str() == OsStr::new(excluded))
        })
        .map(|(key, value)| serde_json::json!([bytes(&key), bytes(&value)]))
        .collect();
    let pid = std::process::id();
    let record = serde_json::json!({"schema":"codex-window-launch-v1","purpose":purpose,"pid":pid,"start":terminal::process(pid)?.1,"name":format!("codex-{token}"),"cwd":bytes(cwd.as_os_str()),"env":environment,"profile":profile.map(profile::target_name),"program":program.map(|p| bytes(p.as_os_str())),"mode":mode,"args":args.iter().map(|v| bytes(v)).collect::<Vec<_>>()});
    let directory = directory(context)?;
    prune_pending(&directory);
    let path = directory.join(format!("launch-{token}"));
    write_launch(&path, &record)?;
    Ok(path)
}
pub(super) fn execution(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
    program: Option<&Path>,
) -> Result<(String, PathBuf), ManagerError> {
    let id = random()?;
    let path = prepare(context, profile, "off", args, program, "exec", &id)?;
    Ok((id, path))
}
fn prune_pending(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.take(1024).flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if let Some(id) = name.strip_prefix("launch-").filter(|id| token(id)) {
            let _ = id;
            let Ok(v) = read_launch(&entry.path()) else {
                continue;
            };
            let Some(pid) = v["pid"].as_u64().and_then(|p| u32::try_from(p).ok()) else {
                continue;
            };
            if v["schema"] == "codex-window-launch-v1"
                && v["start"]
                    .as_u64()
                    .is_some_and(|start| terminal::departed(pid, start))
            {
                let _ = fs::remove_file(entry.path());
            }
        } else if let Some(rest) = name.strip_prefix("consumed-") {
            let fields: Vec<_> = rest.split('-').collect();
            if fields.len() != 3 || !token(fields[0]) {
                continue;
            }
            let (Ok(pid), Ok(start)) = (fields[1].parse::<u32>(), fields[2].parse::<u64>()) else {
                continue;
            };
            if terminal::departed(pid, start) && read_launch(&entry.path()).is_ok() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}
pub(super) fn start(
    context: &Context,
    args: &[OsString],
    window: bool,
) -> Result<Option<String>, ManagerError> {
    let [id] = args else {
        return Err(ERR_USAGE);
    };
    let id = id.to_str().filter(|id| token(id)).ok_or(ERR_USAGE)?;
    let directory = directory(context)?;
    let request = directory.join(format!("launch-{id}"));
    let pid = std::process::id();
    let start = terminal::process(pid)?.1;
    let consumed = directory.join(format!("consumed-{id}-{pid}-{start}"));
    let result = (|| {
        // Exactly one starter can claim the launch; no hard link is used.
        rename_noreplace(&request, &consumed).map_err(|_| ERROR)?;
        let record = read_launch(&consumed)?;
        if record["schema"] != "codex-window-launch-v1"
            || record["name"] != format!("codex-{id}")
            || record["purpose"] != if window { "window" } else { "exec" }
        {
            return Err(ERROR);
        }
        let environment = record["env"].as_array().ok_or(ERROR)?;
        for (key, _) in std::env::vars_os() {
            if !TERMINAL_ENV
                .iter()
                .any(|excluded| key.as_os_str() == OsStr::new(excluded))
            {
                std::env::remove_var(key);
            }
        }
        for pair in environment {
            let pair = pair.as_array().filter(|p| p.len() == 2).ok_or(ERROR)?;
            let key = os(&pair[0])?;
            if key.is_empty()
                || key.as_bytes().contains(&b'=')
                || TERMINAL_ENV
                    .iter()
                    .any(|excluded| key.as_os_str() == OsStr::new(excluded))
            {
                return Err(ERROR);
            }
            std::env::set_var(key, os(&pair[1])?);
        }
        let cwd = PathBuf::from(os(&record["cwd"])?);
        std::env::set_current_dir(cwd).map_err(|_| ERROR)?;
        let pid = std::process::id();
        let start = terminal::process(pid)?.1;
        let tty = fs::read_link("/proc/self/fd/0").map_err(|_| ERROR)?;
        if !tty.starts_with("/dev/pts") {
            return Err(ERROR);
        }
        if window {
            let route = serde_json::json!({"kind":"stock","name":format!("codex-{id}"),"pid":pid,"start":start,"tty":tty});
            write_new_record(
                &directory.join(format!("route-{id}")),
                route.to_string().as_bytes(),
            )?;
            terminal::prune(&directory);
            std::env::set_var(TOKEN_ENV, id);
        }
        std::env::set_var(READY_ENV, "1");
        let args = record["args"]
            .as_array()
            .ok_or(ERROR)?
            .iter()
            .map(os)
            .collect::<Result<Vec<_>, _>>()?;
        let profile = record["profile"]
            .as_str()
            .map(|p| parse_target(&OsString::from(p)))
            .transpose()?;
        let mode = record["mode"]
            .as_str()
            .filter(|m| matches!(*m, "off" | "hidden" | "status" | "attach"))
            .ok_or(ERROR)?;
        // Clean before exec; only the small live route remains.
        fs::remove_file(&consumed).map_err(|_| ERROR)?;
        if record["program"].is_null() {
            tmux_launch::execute(context, profile.as_ref(), mode, &args)
        } else {
            let program = PathBuf::from(os(&record["program"])?);
            tmux_launch::execute_command(context, &program, mode, &args)
        }
    })();
    let _ = fs::remove_file(consumed);
    if window && result.is_err() {
        let _ = fs::remove_file(directory.join(format!("route-{id}")));
    }
    result
}

pub(super) fn inherited_route(context: &Context) -> Option<serde_json::Value> {
    let id = std::env::var(TOKEN_ENV).ok()?;
    if !token(&id) {
        return None;
    }
    let value = read(&directory(context).ok()?.join(format!("route-{id}"))).ok()?;
    valid_route(&value).then_some(value)
}
pub(super) fn valid_route(value: &serde_json::Value) -> bool {
    let Some(name) = value["name"]
        .as_str()
        .and_then(|s| s.strip_prefix("codex-"))
    else {
        return false;
    };
    let Some(pid) = value["pid"].as_u64().and_then(|p| u32::try_from(p).ok()) else {
        return false;
    };
    let Some(tty) = value["tty"].as_str().filter(|v| v.starts_with("/dev/pts/")) else {
        return false;
    };
    value["kind"] == "stock"
        && token(name)
        && terminal::process(pid).ok().map(|p| p.1) == value["start"].as_u64()
        && fs::read_link(format!("/proc/{pid}/fd/0")).ok().as_deref() == Some(Path::new(tty))
}
pub(super) fn focus(value: &serde_json::Value) -> bool {
    if !valid_route(value) {
        return false;
    }
    let Some(prefix) = std::env::var_os("PREFIX").map(PathBuf::from) else {
        return false;
    };
    service(
        value["name"].as_str().unwrap(),
        &prefix.join("bin/true"),
        &[],
        &prefix,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shot_records_preserve_collisions_bound_bytes_and_reject_replay() {
        let directory =
            std::env::temp_dir().join(format!("codex-window-records-{}", random().unwrap()));
        fs::create_dir(&directory).unwrap();
        set_mode(&directory, PRIVATE_DIR_MODE).unwrap();
        let path = directory.join("request");
        let record = serde_json::json!({"owned":"original"});
        write_launch(&path, &record).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o7777, 0o600);
        assert!(write_launch(&path, &serde_json::json!({"owned":"replacement"})).is_err());
        assert_eq!(read_launch(&path).unwrap(), record);
        let oversized = directory.join("oversized");
        assert!(
            write_launch(&oversized, &serde_json::json!("x".repeat(LAUNCH_MAX_BYTES))).is_err()
        );
        assert!(!oversized.exists());
        let link = directory.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(read_launch(&link).is_err());
        assert!(write_launch(&link, &record).is_err());
        assert_eq!(read_launch(&path).unwrap(), record);
        let consumed = directory.join("consumed");
        rename_noreplace(&path, &consumed).unwrap();
        assert!(rename_noreplace(&path, &consumed).is_err());
        assert_eq!(read_launch(&consumed).unwrap(), record);
        remove_private_tree(&directory).unwrap();
    }

    #[test]
    fn pending_cleanup_requires_verified_departure_and_preserves_unowned_records() {
        let directory =
            std::env::temp_dir().join(format!("codex-window-prune-{}", random().unwrap()));
        fs::create_dir(&directory).unwrap();
        set_mode(&directory, PRIVATE_DIR_MODE).unwrap();
        let pid = std::process::id();
        let start = terminal::process(pid).unwrap().1;
        let live = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &live,
            &serde_json::json!({"schema":"codex-window-launch-v1","pid":pid,"start":start}),
        )
        .unwrap();
        let reused = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &reused,
            &serde_json::json!({"schema":"codex-window-launch-v1","pid":pid,"start":start+1}),
        )
        .unwrap();
        let foreign = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &foreign,
            &serde_json::json!({"schema":"foreign","pid":pid,"start":start+1}),
        )
        .unwrap();
        let malformed = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &malformed,
            &serde_json::json!({"schema":"codex-window-launch-v1","pid":pid}),
        )
        .unwrap();
        prune_pending(&directory);
        assert!(live.exists() && foreign.exists() && malformed.exists());
        assert!(!reused.exists());
        assert!(!terminal::departed(pid, start));
        assert!(terminal::departed(pid, start + 1));
        remove_private_tree(&directory).unwrap();
    }
}
