//! Native thread identity, independent of user-selected terminal title fields.
use super::{canonical_session_id, Context, ManagerError, ERR_USAGE};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const ERROR: ManagerError = ManagerError::operation("codex termux: terminal binding unavailable");

pub(super) fn process(pid: u32) -> Result<(u32, u64), ManagerError> {
    let path = PathBuf::from(format!("/proc/{pid}"));
    if fs::metadata(&path).map_err(|_| ERROR)?.uid() != unsafe { libc::getuid() } {
        return Err(ERROR);
    }
    parse_stat(&fs::read_to_string(path.join("stat")).map_err(|_| ERROR)?)
}

// An inaccessible process is not proof of death. Only absence or a verified
// different kernel start time permits deleting its private records.
pub(super) fn departed(pid: u32, start: u64) -> bool {
    match process(pid) {
        Ok((_, actual)) => actual != start,
        Err(_) => fs::metadata(format!("/proc/{pid}"))
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
    }
}

fn parse_stat(value: &str) -> Result<(u32, u64), ManagerError> {
    let fields: Vec<_> = value
        .rsplit_once(')')
        .ok_or(ERROR)?
        .1
        .split_whitespace()
        .collect();
    Ok((
        fields.get(1).ok_or(ERROR)?.parse().map_err(|_| ERROR)?,
        fields.get(19).ok_or(ERROR)?.parse().map_err(|_| ERROR)?,
    ))
}

fn native(home: &Path, executable: &Path) -> bool {
    let root = home.join(".local/lib/codex");
    if executable == root.join("profile-tui-preview/direct-selection-v1/native") {
        return true;
    }
    let Ok(relative) = executable.strip_prefix(root.join("core/generations")) else {
        return false;
    };
    let parts: Vec<_> = relative.components().collect();
    (parts.len() == 2 && parts[1].as_os_str() == "runtime")
        || (parts.len() == 3 && parts[1].as_os_str() == "helpers" && parts[2].as_os_str() == "2")
}

fn slot(
    context: &Context,
    parent: u32,
    start: u64,
    descriptor: u32,
) -> Result<String, ManagerError> {
    if process(parent)?.1 != start {
        return Err(ERROR);
    }
    let executable = fs::read_link(format!("/proc/{parent}/exe")).map_err(|_| ERROR)?;
    if !native(&context.home, &executable) {
        return Err(ERROR);
    }
    let slot = PathBuf::from(format!("/proc/{parent}/fd/{descriptor}"));
    let link = fs::read_link(&slot).map_err(|_| ERROR)?;
    let name = link
        .file_name()
        .and_then(|v| v.to_str())
        .and_then(|v| v.strip_suffix(" (deleted)"))
        .ok_or(ERROR)?;
    let nonce = name
        .strip_prefix(&format!(".codex-terminal-thread-v1-{parent}-"))
        .ok_or(ERROR)?;
    if nonce.is_empty()
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(ERROR);
    }
    let slot_metadata = fs::metadata(&slot).map_err(|_| ERROR)?;
    if !slot_metadata.is_file()
        || slot_metadata.uid() != unsafe { libc::getuid() }
        || slot_metadata.mode() & 0o777 != 0o600
        || slot_metadata.len() > 36
    {
        return Err(ERROR);
    }
    let mut content = String::new();
    fs::File::open(&slot)
        .map_err(|_| ERROR)?
        .take(37)
        .read_to_string(&mut content)
        .map_err(|_| ERROR)?;
    if !content.is_empty() && !canonical_session_id(&content) {
        return Err(ERROR);
    }
    if process(parent)?.1 != start {
        return Err(ERROR);
    }
    Ok(content)
}

pub(super) fn bind(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let descriptor = args
        .first()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v <= 1048576)
        .ok_or(ERR_USAGE)?;
    if args.len() != 1 {
        return Err(ERR_USAGE);
    }
    let parent = process(std::process::id())?.0;
    let identity = process(parent)?;
    slot(context, parent, identity.1, descriptor)?;
    let route = if let Some(route) = super::window::inherited_route(context) {
        route
    } else {
        let origin = super::origin::resolve().ok_or(ERROR)?;
        if !super::origin::validate(&origin) {
            return Err(ERROR);
        }
        serde_json::json!({"kind":"native","origin":serde_json::from_str::<serde_json::Value>(&origin.record()).map_err(|_| ERROR)?})
    };
    let metadata = fs::metadata(format!("/proc/{parent}/fd/{descriptor}")).map_err(|_| ERROR)?;
    let tty = fs::read_link(format!("/proc/{parent}/fd/0")).map_err(|_| ERROR)?;
    if !tty.starts_with("/dev/pts") {
        return Err(ERROR);
    }
    let record = serde_json::json!({"schema":"codex-terminal-binding-v1","pid":parent,"start":identity.1,"fd":descriptor,"dev":metadata.dev(),"ino":metadata.ino(),"tty":tty,"route":route});
    slot(context, parent, identity.1, descriptor)?;
    let directory = super::window::directory(context)?;
    prune(&directory);
    let path = directory.join(format!("binding-{parent}-{}", identity.1));
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            super::write_new_record(&path, record.to_string().as_bytes())?;
        }
        Ok(_) => {
            let previous = super::window::read(&path)?;
            if previous.as_object().map(|v| v.len()) != record.as_object().map(|v| v.len())
                || previous["schema"] != record["schema"]
                || previous["pid"] != record["pid"]
                || previous["start"] != record["start"]
                || previous["tty"] != record["tty"]
                || previous["route"] != record["route"]
                || previous["fd"].as_u64().is_none_or(|v| v > 1048576)
                || previous["dev"].as_u64().is_none()
                || previous["ino"].as_u64().is_none()
            {
                return Err(ERROR);
            }
            // Exec closes the old foreground FD while preserving PID/start.
            // Publish the newly qualified FD without exposing a second target.
            let temporary = directory.join(format!(
                ".binding-{}-{}",
                std::process::id(),
                super::TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            super::write_new_record(&temporary, record.to_string().as_bytes())?;
            let replaced = (|| {
                slot(context, parent, identity.1, descriptor)?;
                if super::window::read(&path)? != previous {
                    return Err(ERROR);
                }
                fs::rename(&temporary, &path).map_err(|_| ERROR)
            })();
            if replaced.is_err() {
                let _ = fs::remove_file(temporary);
            }
            replaced?;
        }
        Err(_) => return Err(ERROR),
    }
    super::sync_directory(&directory).map_err(|_| ERROR)?;
    Ok(None)
}

struct Target {
    pid: u32,
    start: u64,
    fd: u32,
    route: serde_json::Value,
    file: (u64, u64, PathBuf),
}
pub(super) fn prune(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.take(1024).flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !(name.starts_with("binding-") || name.starts_with("route-")) {
            continue;
        }
        let Ok(v) = super::window::read(&entry.path()) else {
            continue;
        };
        let Some(pid) = v["pid"].as_u64().and_then(|p| u32::try_from(p).ok()) else {
            continue;
        };
        if v["start"]
            .as_u64()
            .is_some_and(|start| departed(pid, start))
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}
fn bare_targets(context: &Context, id: &str) -> Result<Vec<Target>, ManagerError> {
    let directory = super::manager_base(context).join("manager/terminals");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    super::ensure_private_directory(&directory)?;
    prune(&directory);
    let mut targets = Vec::new();
    for (count, entry) in fs::read_dir(&directory).map_err(|_| ERROR)?.enumerate() {
        if count >= 1024 {
            return Err(ERROR);
        }
        let entry = entry.map_err(|_| ERROR)?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with("binding-"))
        {
            continue;
        }
        let Ok(v) = super::window::read(&entry.path()) else {
            continue;
        };
        let Some(pid) = v["pid"].as_u64().and_then(|p| u32::try_from(p).ok()) else {
            continue;
        };
        let Some(start) = v["start"].as_u64() else {
            continue;
        };
        let Some(fd) = v["fd"]
            .as_u64()
            .and_then(|p| u32::try_from(p).ok())
            .filter(|p| *p <= 1048576)
        else {
            continue;
        };
        if v["schema"] != "codex-terminal-binding-v1"
            || slot(context, pid, start, fd).ok().as_deref() != Some(id)
        {
            continue;
        }
        let Some(dev) = v["dev"].as_u64() else {
            continue;
        };
        let Some(ino) = v["ino"].as_u64() else {
            continue;
        };
        let Some(tty) = v["tty"].as_str() else {
            continue;
        };
        let target = Target {
            pid,
            start,
            fd,
            route: v["route"].clone(),
            file: (dev, ino, PathBuf::from(tty)),
        };
        if valid_target(context, &target, id) {
            targets.push(target);
        }
    }
    Ok(targets)
}
fn valid_target(context: &Context, target: &Target, id: &str) -> bool {
    if slot(context, target.pid, target.start, target.fd)
        .ok()
        .as_deref()
        != Some(id)
    {
        return false;
    }
    let (dev, ino, tty) = &target.file;
    let Ok(m) = fs::metadata(format!("/proc/{}/fd/{}", target.pid, target.fd)) else {
        return false;
    };
    if (m.dev(), m.ino()) != (*dev, *ino)
        || fs::read_link(format!("/proc/{}/fd/0", target.pid))
            .ok()
            .as_ref()
            != Some(tty)
    {
        return false;
    }
    true
}
fn native_route(value: &serde_json::Value) -> Option<super::origin::Origin> {
    if value["kind"] != "native" {
        return None;
    }
    super::origin::Origin::parse(&value["origin"].to_string())
}
fn target_execution_home(
    context: &Context,
    target: &Target,
    id: &str,
    home: &Path,
) -> Result<(), ManagerError> {
    // The installed executable, PID/start, live unlinked foreground FD and PTY
    // must qualify before reading this process's bounded execution identity.
    if !valid_target(context, target, id) {
        return Err(ERROR);
    }
    let actual = super::profile::execution_home_or_default(
        fs::File::open(format!("/proc/{}/environ", target.pid)).map_err(|_| ERROR)?,
        true,
    )?
    .ok_or(ERROR)?;
    if actual != home {
        return Err(ERROR);
    }
    codex_runtime_binding::native::private(&actual, true).map_err(|_| ERROR)?;
    if !valid_target(context, target, id) {
        return Err(ERROR);
    }
    Ok(())
}

fn focus_target(
    context: &Context,
    target: &Target,
    id: &str,
    execution_home: Option<&Path>,
) -> Result<Option<String>, ManagerError> {
    if !valid_target(context, target, id) {
        return Err(ERROR);
    }
    if let Some(home) = execution_home {
        target_execution_home(context, target, id, home)?;
    }
    let origin = native_route(&target.route);
    if let Some(origin) = &origin {
        if !super::origin::validate(origin) {
            return Err(ERROR);
        }
    } else if !super::window::valid_route(&target.route) {
        return Err(ERROR);
    }
    if !valid_target(context, target, id) {
        return Err(ERROR);
    }
    if let Some(home) = execution_home {
        target_execution_home(context, target, id, home)?;
    }
    let focused = match origin {
        Some(origin) => super::origin::focus(&origin),
        None => super::window::focus(&target.route),
    };
    if !focused {
        return Err(ERROR);
    }
    Ok(None)
}

pub(super) fn focus(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let [id] = args else {
        return Err(ERR_USAGE);
    };
    let id = id
        .to_str()
        .filter(|id| canonical_session_id(id))
        .ok_or(ERR_USAGE)?;
    let targets = bare_targets(context, id)?;
    let [target] = targets.as_slice() else {
        return Err(ERROR);
    };
    focus_target(context, target, id, None)
}

pub(super) fn focus_display(
    context: &Context,
    id: &str,
    home: &Path,
    route: Option<&serde_json::Value>,
) -> Result<Option<String>, ManagerError> {
    // Owner uncertainty is never treated as an owner-free saved session, even
    // when a terminal binding is already present.
    let _ = super::task::display_command(context, home, id)?;
    let targets = bare_targets(context, id)?;
    let [target] = targets.as_slice() else {
        return Err(super::window::DISPLAY_PENDING);
    };
    if route.is_some_and(|route| route != &target.route) {
        return Err(ERROR);
    }
    focus_target(context, target, id, Some(home))?;
    Ok(Some(
        "Existing terminal focus requested; Android visibility is not confirmed.\n".into(),
    ))
}

// A routing proof, not an Owner registry: bind a notification to the exact
// execution directory, and recheck Native's authoritative Owner at execution.
pub(super) fn display_args(context: &Context, id: &str) -> Result<Vec<OsString>, ManagerError> {
    if !canonical_session_id(id) {
        return Err(ERR_USAGE);
    }
    let home = context
        .inherited_codex_home
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| context.home.join(".codex"));
    codex_runtime_binding::native::private(&home, true).map_err(|_| ERROR)?;
    let metadata = fs::symlink_metadata(&home).map_err(|_| ERROR)?;
    Ok(vec![
        id.into(),
        home.into_os_string(),
        metadata.dev().to_string().into(),
        metadata.ino().to_string().into(),
    ])
}

fn display_identity(
    context: &Context,
    args: &[OsString],
) -> Result<(String, PathBuf), ManagerError> {
    let id = args
        .first()
        .and_then(|v| v.to_str())
        .filter(|id| canonical_session_id(id))
        .ok_or(ERR_USAGE)?;
    let home = match args {
        [_] => {
            codex_runtime_binding::native::owner(
                &context.home,
                id,
                std::time::Instant::now() + std::time::Duration::from_secs(10),
            )
            .map_err(|_| super::task::ERR_TASK)?
            .ok_or(super::task::ERR_TASK)?
            .home
        }
        [_, home, dev, ino] => {
            let home = PathBuf::from(home);
            codex_runtime_binding::native::private(&home, true).map_err(|_| ERROR)?;
            let metadata = fs::symlink_metadata(&home).map_err(|_| ERROR)?;
            if dev.to_str().and_then(|v| v.parse::<u64>().ok()) != Some(metadata.dev())
                || ino.to_str().and_then(|v| v.parse::<u64>().ok()) != Some(metadata.ino())
            {
                return Err(ERROR);
            }
            home
        }
        _ => return Err(ERR_USAGE),
    };
    Ok((id.to_owned(), home))
}

const ERR_FRONTEND: ManagerError = ManagerError::operation(
    "codex termux: exact Native display requires a qualified foreground frontend",
);

fn require_frontend(context: &Context, home: &Path, id: &str) -> Result<(), ManagerError> {
    let mut command = super::task::execution_command(context, home)?;
    command.args(["termux", "__terminal-capabilities-v1", "--", "resume", id]);
    let output = super::origin::capture(command, 5).ok_or(ERR_FRONTEND)?;
    if !output.status.success()
        || !output.stderr.is_empty()
        || output.stdout != b"codex-terminal-capabilities-v1\nidentity=native\n"
    {
        return Err(ERR_FRONTEND);
    }
    Ok(())
}

pub(super) fn show(
    context: &Context,
    args: &[OsString],
    automatic: bool,
) -> Result<Option<String>, ManagerError> {
    if automatic && !super::read_auto_display(context) {
        return Ok(None);
    }
    let (id, home) = display_identity(context, args)?;
    // Admission precedes both existing-terminal selection and construction.
    // Auto ON shares this path; OFF returned before any Owner query above.
    let _ = super::task::display_command(context, &home, &id)?;
    let targets = bare_targets(context, &id)?;
    if !targets.is_empty() {
        if targets.len() != 1 {
            return Err(ERROR);
        }
        return focus_display(context, &id, &home, None);
    }
    // Existing valid bindings remain focusable after an optional frontend is
    // removed. New construction requires Core to admit the actual resume route.
    require_frontend(context, &home, &id)?;
    // The one-shot terminal revalidates the same authority before resume.
    let metadata = fs::symlink_metadata(&home).map_err(|_| ERROR)?;
    let args = vec![
        "termux".into(),
        "__terminal-display-v1".into(),
        id.clone().into(),
        home.clone().into_os_string(),
        metadata.dev().to_string().into(),
        metadata.ino().to_string().into(),
    ];
    super::window::display(context, &id, &home, &args)
}

pub(super) fn reenter(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    use std::os::unix::process::CommandExt;
    let (id, home) = display_identity(context, args)?;
    let mut command = super::task::display_command(context, &home, &id)?;
    require_frontend(context, &home, &id)?;
    let _ = command.exec();
    Err(super::ERR_LAUNCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_process_identity_handles_parentheses_and_bad_fields() {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[1] = "42";
        fields[19] = "1234";
        assert_eq!(
            parse_stat(&format!("7 (name) with ) chars) {}", fields.join(" "))).unwrap(),
            (42, 1234)
        );
        assert!(parse_stat("invalid").is_err());
        fields[19] = "bad";
        assert!(parse_stat(&format!("7 (name) {}", fields.join(" "))).is_err());
    }
    #[test]
    fn terminal_native_paths_are_product_local() {
        let home = Path::new("/owned");
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/core/generations/a/runtime")
        ));
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/profile-tui-preview/direct-selection-v1/native")
        ));
        assert!(native(
            home,
            Path::new("/owned/.local/lib/codex/core/generations/a/helpers/2")
        ));
        for path in [
            "/owned/runtime",
            "/owned/.local/lib/codex/core/generations/runtime",
            "/owned/.local/lib/codex/core/generations/a/b/runtime",
            "/owned/.local/lib/codex/core/generations/a/helpers/0",
            "/owned/.local/lib/codex/core/generations/a/helpers/1",
            "/owned/.local/lib/codex/core/generations/a/b/helpers/2",
            "/owned/.local/lib/codex/core/generations/a/helpers/2/native",
        ] {
            assert!(!native(home, Path::new(path)));
        }
    }
}
