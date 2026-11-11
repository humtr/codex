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
pub(super) const DISPLAY_PENDING: ManagerError =
    ManagerError::operation("codex termux: terminal display is pending; original launch preserved");
const LAUNCH_MAX_BYTES: usize = 64 * 1024;
const ADMISSION_BUSY: ManagerError =
    ManagerError::operation("codex termux: terminal admission is busy; no new launch requested");
// Window descriptors have short filesystem-only critical sections. Never hold
// this lock across Android dispatch, route waiting, focus or workload execution.
// Other profile lifecycle operations keep their existing nonblocking policy.
fn launch_lock(directory: &Path) -> Result<File, ManagerError> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match profile::directory_lock(directory, libc::LOCK_EX) {
            Ok(lock) => return Ok(lock),
            Err(error) if error == profile::ERR_BUSY => {
                if Instant::now() >= deadline {
                    return Err(ADMISSION_BUSY);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}
fn same_display(
    record: &serde_json::Value,
    id: &str,
    token: &str,
    program: Option<&Path>,
    args: &[OsString],
) -> bool {
    record["schema"] == "codex-window-launch-v1"
        && record["thread"] == id
        && record["name"] == format!("codex-{token}")
        && record["pid"].as_u64().is_some_and(|pid| pid > 0 && u32::try_from(pid).is_ok())
        && record["start"].as_u64().is_some()
        && record["profile"].is_null()
        && record["program"] == program.map(|path| bytes(path.as_os_str())).unwrap_or_default()
        // These exact arguments include the execution home's device/inode.
        && record["args"] == serde_json::json!(args.iter().map(|arg| bytes(arg)).collect::<Vec<_>>())
}
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
    let command = format!("startservice -n com.termux/com.termux.app.RunCommandService -a com.termux.RUN_COMMAND --es com.termux.RUN_COMMAND_PATH {} --esa com.termux.RUN_COMMAND_ARGUMENTS {} --es com.termux.RUN_COMMAND_WORKDIR {} --es com.termux.RUN_COMMAND_RUNNER terminal-session --es com.termux.RUN_COMMAND_SHELL_NAME {} --es com.termux.RUN_COMMAND_SHELL_CREATE_MODE no-shell-with-name --es com.termux.RUN_COMMAND_SESSION_ACTION 0", shell_quote(program), shell_quote(&args.join(",")), shell_quote(cwd), shell_quote(name));
    origin::command(&command, 15).is_some_and(|out| {
        out.status.success()
            && out.stderr.is_empty()
            && std::str::from_utf8(&out.stdout).is_ok_and(|s| s.contains("Starting service:"))
    })
}

pub(super) fn launch(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
    program: Option<&Path>,
) -> Result<Option<String>, ManagerError> {
    launch_named(context, profile, args, program, &random()?, None)
}

pub(super) fn display(
    context: &Context,
    id: &str,
    home: &Path,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if !canonical_session_id(id) {
        return Err(ERR_USAGE);
    }
    launch_named(
        context,
        None,
        args,
        Some(&context.core_entrypoint),
        &id.replace('-', ""),
        Some((id, home)),
    )
}

fn launch_named(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
    program: Option<&Path>,
    token: &str,
    display: Option<(&str, &Path)>,
) -> Result<Option<String>, ManagerError> {
    let directory = directory(context)?;
    // Publish the one-shot intent before Android dispatch. A competing click
    // observes that exact intent without waiting for Android or creating work.
    let launch_guard = launch_lock(&directory)?;
    if let Some((id, home)) = display {
        prune_pending(&directory);
        terminal::prune(&directory);
        let route_path = directory.join(format!("route-{token}"));
        if inspect_path(&route_path)? == PathPresence::Present {
            let route = read(&route_path)?;
            if route["thread"] != id || !valid_route(&route) {
                return Err(ERROR);
            }
            // A live service root or old route.thread alone cannot establish
            // the conversation currently shown by this foreground frontend.
            drop(launch_guard);
            return terminal::focus_display(context, id, home, Some(&route));
        }
        for (count, entry) in fs::read_dir(&directory).map_err(|_| ERROR)?.enumerate() {
            if count >= 1024 {
                return Err(ERROR);
            }
            let entry = entry.map_err(|_| ERROR)?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == format!("launch-{token}") || name.starts_with(&format!("consumed-{token}-"))
            {
                if !same_display(&read_launch(&entry.path())?, id, token, program, args) {
                    return Err(ERROR);
                }
                return Err(DISPLAY_PENDING);
            }
        }
    }
    let name = format!("codex-{token}");
    let cwd = std::env::current_dir().map_err(|_| ERROR)?;
    let path = prepare(
        context,
        profile,
        args,
        program,
        token,
        display.map(|(id, _)| id),
    )?;
    sync_directory(&directory).map_err(|_| ERROR)?;
    drop(launch_guard);
    if !service(
        &name,
        &PathBuf::from(std::env::var_os("PREFIX").ok_or(ERROR)?).join("bin/env"),
        &[
            &format!("HOME={}", context.home.to_str().ok_or(ERROR)?),
            context.core_entrypoint.to_str().ok_or(ERROR)?,
            "termux",
            "__terminal-start-v1",
            token,
        ],
        &cwd,
    ) {
        if display.is_some() {
            // Unknown Android delivery is not permission to replace this exact
            // launch. A delayed starter can still consume the original once.
            return Err(DISPLAY_PENDING);
        }
        let _ = fs::remove_file(path);
        return Err(ERROR);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !directory.join(format!("route-{token}")).exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if !directory.join(format!("route-{token}")).exists() {
        if display.is_some() {
            return Err(DISPLAY_PENDING);
        }
        let _ = fs::remove_file(path);
        return Err(ERROR);
    }
    if let Some((id, home)) = display {
        let route = read(&directory.join(format!("route-{token}")))?;
        if route["thread"] != id || !valid_route(&route) {
            return Err(ERROR);
        }
        return terminal::focus_display(context, id, home, Some(&route));
    }
    Ok(None)
}

fn prepare(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
    program: Option<&Path>,
    token: &str,
    thread_id: Option<&str>,
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
    let record = serde_json::json!({"schema":"codex-window-launch-v1","thread":thread_id,"pid":pid,"start":terminal::process(pid)?.1,"name":format!("codex-{token}"),"cwd":bytes(cwd.as_os_str()),"env":environment,"profile":profile.map(profile::target_name),"program":program.map(|p| bytes(p.as_os_str())),"args":args.iter().map(|v| bytes(v)).collect::<Vec<_>>()});
    let directory = directory(context)?;
    prune_pending(&directory);
    let path = directory.join(format!("launch-{token}"));
    write_launch(&path, &record)?;
    Ok(path)
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
                && v.get("thread").is_none_or(serde_json::Value::is_null)
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
pub(super) fn start(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let [id] = args else {
        return Err(ERR_USAGE);
    };
    let id = id.to_str().filter(|id| token(id)).ok_or(ERR_USAGE)?;
    let directory = directory(context)?;
    let request = directory.join(format!("launch-{id}"));
    let pid = std::process::id();
    let start = terminal::process(pid)?.1;
    let consumed = directory.join(format!("consumed-{id}-{pid}-{start}"));
    let mut claimed = false;
    let mut published = false;
    let result = (|| {
        let record = {
            let _guard = launch_lock(&directory)?;
            // Serialize this rename with admission's directory snapshot.
            // Exactly one starter can claim the launch; no hard link is used.
            rename_noreplace(&request, &consumed).map_err(|_| ERROR)?;
            claimed = true;
            read_launch(&consumed)?
        };
        if record["schema"] != "codex-window-launch-v1" || record["name"] != format!("codex-{id}") {
            return Err(ERROR);
        }
        if let Some(thread) = record.get("thread").filter(|v| !v.is_null()) {
            let thread = thread
                .as_str()
                .filter(|v| canonical_session_id(v))
                .ok_or(ERROR)?;
            if thread.replace('-', "") != id {
                return Err(ERROR);
            }
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
        let route = serde_json::json!({"kind":"stock","thread":record.get("thread").cloned().unwrap_or(serde_json::Value::Null),"name":format!("codex-{id}"),"pid":pid,"start":start,"tty":tty});
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
        let program = if record["program"].is_null() {
            None
        } else {
            Some(PathBuf::from(os(&record["program"])?))
        };
        {
            let _guard = launch_lock(&directory)?;
            write_new_record(
                &directory.join(format!("route-{id}")),
                route.to_string().as_bytes(),
            )?;
            published = true;
            terminal::prune(&directory);
            // Admission cannot miss both consumed intent and published route.
            fs::remove_file(&consumed).map_err(|_| ERROR)?;
            sync_directory(&directory).map_err(|_| ERROR)?;
        }
        std::env::set_var(TOKEN_ENV, id);
        std::env::set_var(READY_ENV, "1");
        if let Some(program) = program {
            launch::execute_command(&program, &args)
        } else {
            launch::execute(context, profile.as_ref(), &args)
        }
    })();
    // A duplicate starter that never claimed the descriptor cannot delete the
    // original starter's route. Unavailable cleanup preserves uncertain state.
    if (claimed || published) && result.is_err() {
        if let Ok(_guard) = launch_lock(&directory) {
            let _ = fs::remove_file(consumed);
            let route = directory.join(format!("route-{id}"));
            if published
                && read(&route).is_ok_and(|value| value["pid"] == pid && value["start"] == start)
            {
                let _ = fs::remove_file(route);
            }
        }
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

    // All direct display probes run without an Android transport, including
    // their regression/failure paths. Integration tests supply an owned AM stub.
    fn isolated_display_probe(name: &str) -> bool {
        const CHILD: &str = "CODEX_TEST_ISOLATED_WINDOW_DISPLAY";
        if std::env::var(CHILD).as_deref() == Ok(name) {
            assert!(std::env::var_os("PREFIX").is_none());
            return false;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name])
            .env(CHILD, name)
            .env_remove("PREFIX")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        true
    }

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
            &serde_json::json!({"schema":"codex-window-launch-v1","thread":null,"pid":pid,"start":start}),
        )
        .unwrap();
        let reused = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &reused,
            &serde_json::json!({"schema":"codex-window-launch-v1","thread":null,"pid":pid,"start":start+1}),
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
            &serde_json::json!({"schema":"codex-window-launch-v1","thread":null,"pid":pid}),
        )
        .unwrap();
        let uncertain = directory.join(format!("launch-{}", random().unwrap()));
        write_launch(
            &uncertain,
            &serde_json::json!({"schema":"codex-window-launch-v1","thread":"01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3","pid":pid,"start":start+1}),
        ).unwrap();
        prune_pending(&directory);
        assert!(
            uncertain.exists(),
            "departed requester does not prove Android rejected display"
        );
        assert!(live.exists() && foreign.exists() && malformed.exists());
        assert!(!reused.exists());
        assert!(!terminal::departed(pid, start));
        assert!(terminal::departed(pid, start + 1));
        remove_private_tree(&directory).unwrap();
    }
    #[test]
    fn repeated_thread_display_preserves_pending_launch_without_dispatch() {
        if isolated_display_probe(
            "window::tests::repeated_thread_display_preserves_pending_launch_without_dispatch",
        ) {
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "codex-window-display-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        set_mode(&root, PRIVATE_DIR_MODE).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: PathBuf::from("/not-executed"),
        };
        let id = "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3";
        let token = id.replace('-', "");
        let directory = directory(&context).unwrap();
        let pid = std::process::id();
        let path = directory.join(format!("launch-{token}"));
        let args = vec![
            OsString::from("termux"),
            OsString::from("__terminal-display-v1"),
            id.into(),
            root.as_os_str().to_owned(),
            OsString::from("1"),
            OsString::from("2"),
        ];
        let record = serde_json::json!({"schema":"codex-window-launch-v1","thread":id,
            "name":format!("codex-{token}"),"pid":pid,"start":terminal::process(pid).unwrap().1,
            "profile":null,"program":bytes(context.core_entrypoint.as_os_str()),
            "args":args.iter().map(|v|bytes(v)).collect::<Vec<_>>()});
        write_launch(&path, &record).unwrap();
        let before = fs::read(&path).unwrap();
        for _ in 0..2 {
            assert_eq!(display(&context, id, &root, &args), Err(DISPLAY_PENDING));
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        let mut foreign = args.clone();
        foreign[3] = OsString::from("/foreign-home");
        assert_eq!(display(&context, id, &root, &foreign), Err(ERROR));
        let consumed = directory.join(format!(
            "consumed-{token}-{pid}-{}",
            terminal::process(pid).unwrap().1
        ));
        rename_noreplace(&path, &consumed).unwrap();
        assert_eq!(display(&context, id, &root, &args), Err(DISPLAY_PENDING));
        assert_eq!(display(&context, id, &root, &foreign), Err(ERROR));
        assert_eq!(fs::read(&consumed).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dangling_route_refuses_construction_before_any_android_dispatch() {
        if isolated_display_probe(
            "window::tests::dangling_route_refuses_construction_before_any_android_dispatch",
        ) {
            return;
        }
        let root = std::env::temp_dir().join(format!("codex-window-route-{}", random().unwrap()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, PRIVATE_DIR_MODE).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: PathBuf::from("/not-executed"),
        };
        let directory = directory(&context).unwrap();
        let id = "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3";
        let token = id.replace('-', "");
        let route = directory.join(format!("route-{token}"));
        std::os::unix::fs::symlink(root.join("absent"), &route).unwrap();
        let result = display(&context, id, &root, &[]);
        let unchanged = fs::symlink_metadata(&route)
            .unwrap()
            .file_type()
            .is_symlink()
            && !directory.join(format!("launch-{token}")).exists();
        remove_private_tree(&root).unwrap();
        assert_eq!(result, Err(ERR_PATH));
        assert!(
            unchanged,
            "an unsafe route is not permission for a new launch"
        );
    }

    #[test]
    fn unrelated_lock_contention_never_claims_an_original_display() {
        if isolated_display_probe(
            "window::tests::unrelated_lock_contention_never_claims_an_original_display",
        ) {
            return;
        }
        let root = std::env::temp_dir().join(format!("codex-window-busy-{}", random().unwrap()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, PRIVATE_DIR_MODE).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: PathBuf::from("/not-executed"),
        };
        let directory = directory(&context).unwrap();
        let lock = profile::directory_lock(&directory, libc::LOCK_EX).unwrap();
        assert_eq!(
            display(&context, "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3", &root, &[]),
            Err(ADMISSION_BUSY)
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        drop(lock);
        remove_private_tree(&root).unwrap();
    }

    #[test]
    fn duplicate_starter_cannot_delete_an_original_route() {
        let root = std::env::temp_dir().join(format!("codex-window-replay-{}", random().unwrap()));
        fs::create_dir(&root).unwrap();
        set_mode(&root, PRIVATE_DIR_MODE).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: PathBuf::from("/not-executed"),
        };
        let token = random().unwrap();
        let route = directory(&context).unwrap().join(format!("route-{token}"));
        let record = serde_json::json!({"owned":"original","pid":std::process::id(),
            "start":terminal::process(std::process::id()).unwrap().1});
        write_new_record(&route, record.to_string().as_bytes()).unwrap();
        let before = fs::read(&route).unwrap();
        assert_eq!(start(&context, &[token.into()]), Err(ERROR));
        assert_eq!(fs::read(&route).unwrap(), before);
        remove_private_tree(&root).unwrap();
    }
}
