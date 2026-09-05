#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const CORE_API_ENV: &str = "CODEX_TERMUX_CORE_API";
const CORE_ENTRYPOINT_ENV: &str = "CODEX_TERMUX_CORE_ENTRYPOINT";
const CORE_API: &str = "codex-manager-core-v1";
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const STATE_FILE: &str = "state-v1";
const PROFILES_DIR: &str = "profiles";
const PROFILE_META: &str = "profile.meta";
const STATE_HEADER: &str = "codex-manager-state-v1";
const PROFILE_HEADER: &str = "codex-manager-profile-v1";
const RECORD_MAX_BYTES: usize = 4096;
const PRIVATE_DIR_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
const SESSION_FILE_SUFFIX: &[u8] = b".jsonl";
const SESSION_REF_MAX_BYTES: usize = 256;
const SESSION_FILE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const SESSION_DISCOVERY_MAX_DEPTH: usize = 8;
const SESSION_DISCOVERY_MAX_ENTRIES: usize = 4096;

const HELP: &str = concat!(
    "codex termux profile list\n",
    "codex termux profile current\n",
    "codex termux profile create <PROFILE_ID>\n",
    "codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]\n",
    "codex termux session list [--all|--profile <PROFILE_ID>]\n",
    "codex termux session resume <SESSION_ID> [--profile <PROFILE_ID>] [--] [UPSTREAM_ARGS...]\n",
);

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorClass {
    Usage,
    Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManagerError {
    class: ErrorClass,
    message: &'static str,
}

impl ManagerError {
    const fn usage(message: &'static str) -> Self {
        Self {
            class: ErrorClass::Usage,
            message,
        }
    }

    const fn operation(message: &'static str) -> Self {
        Self {
            class: ErrorClass::Operation,
            message,
        }
    }

    const fn status(self) -> i32 {
        match self.class {
            ErrorClass::Usage => 2,
            ErrorClass::Operation => 1,
        }
    }
}

const ERR_USAGE: ManagerError = ManagerError::usage("codex termux: invalid command");
const ERR_UNSUPPORTED: ManagerError =
    ManagerError::usage("codex termux: command is unavailable in this Manager build");
const ERR_HANDOFF: ManagerError = ManagerError::operation("codex termux: Core handoff is invalid");
const ERR_HOME: ManagerError = ManagerError::operation("codex termux: HOME is invalid");
const ERR_PATH: ManagerError = ManagerError::operation("codex termux: Manager path is unsafe");
const ERR_STATE: ManagerError = ManagerError::operation("codex termux: selection state is invalid");
const ERR_PROFILE: ManagerError = ManagerError::operation("codex termux: profile is unavailable");
const ERR_SESSION: ManagerError = ManagerError::operation("codex termux: session is unavailable");
const ERR_COLLISION: ManagerError = ManagerError::operation("codex termux: profile already exists");
const ERR_CREATE: ManagerError = ManagerError::operation("codex termux: profile creation failed");
const ERR_LAUNCH: ManagerError = ManagerError::operation("codex termux: Core launch failed");

#[derive(Debug, Clone)]
struct Context {
    home: PathBuf,
    inherited_codex_home: Option<OsString>,
    core_entrypoint: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfileTarget {
    Default,
    Custom(String),
}

impl ProfileTarget {
    fn display(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Custom(id) => id,
        }
    }
}

#[derive(Debug, Clone)]
struct ManagerDirs {
    root: PathBuf,
    profiles: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathPresence {
    Missing,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandKind {
    List,
    Current,
    Create,
    Use,
    SessionList,
    SessionResume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCommand {
    kind: CommandKind,
    target: Option<ProfileTarget>,
    upstream_args: Vec<OsString>,
    session_id: Option<String>,
    all: bool,
}

#[derive(Debug, Clone)]
struct SessionEntry {
    profile: String,
    session_id: String,
    updated_unix_seconds: u64,
    sort_key: Vec<u8>,
}

/// Run the Manager boundary with arguments following the `termux` selector.
/// The function returns the public Manager status and never mutates the
/// caller's environment; `profile use` replaces the process at the Core exec
/// boundary instead.
pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match run_inner(args) {
        Ok(output) => {
            if let Some(output) = output {
                print!("{output}");
            }
            0
        }
        Err(error) => {
            eprintln!("{}", error.message);
            error.status()
        }
    }
}

fn run_inner(args: Vec<OsString>) -> Result<Option<String>, ManagerError> {
    if args.is_empty()
        || (args.len() == 1 && (is_exact(args.first(), "help") || is_exact(args.first(), "--help")))
    {
        capture_context()?;
        return Ok(Some(HELP.to_owned()));
    }
    if is_exact(args.first(), "notify") || is_exact(args.first(), "repair") {
        return Err(ERR_UNSUPPORTED);
    }

    let command = parse_command(&args)?;
    let context = capture_context()?;
    match command.kind {
        CommandKind::List => Ok(Some(format_profile_list(&context)?)),
        CommandKind::Current => Ok(Some(format_current(&context)?)),
        CommandKind::Create => {
            let target = command.target.expect("create target is parsed");
            let ProfileTarget::Custom(id) = target else {
                return Err(ERR_USAGE);
            };
            create_profile(&context, &id)?;
            Ok(Some(format!("created: {id}\n")))
        }
        CommandKind::Use => {
            let target = command.target.expect("use target is parsed");
            use_profile(&context, target, &command.upstream_args)?;
            Ok(None)
        }
        CommandKind::SessionList => Ok(Some(format_session_list(
            &context,
            command.target.as_ref(),
            command.all,
        )?)),
        CommandKind::SessionResume => {
            let target = command.target.as_ref();
            let session_id = command.session_id.as_ref().expect("session ref is parsed");
            resume_session(&context, target, session_id, &command.upstream_args)?;
            Ok(None)
        }
    }
}

fn parse_command(args: &[OsString]) -> Result<ParsedCommand, ManagerError> {
    if is_exact(args.first(), "session") {
        return parse_session_command(args);
    }
    if !is_exact(args.first(), "profile") {
        return Err(ERR_USAGE);
    }
    match args.get(1).map(OsString::as_os_str) {
        Some(action) if action == OsStr::new("list") && args.len() == 2 => Ok(ParsedCommand {
            kind: CommandKind::List,
            target: None,
            upstream_args: Vec::new(),
            session_id: None,
            all: false,
        }),
        Some(action) if action == OsStr::new("current") && args.len() == 2 => Ok(ParsedCommand {
            kind: CommandKind::Current,
            target: None,
            upstream_args: Vec::new(),
            session_id: None,
            all: false,
        }),
        Some(action) if action == OsStr::new("create") && args.len() == 3 => {
            let id = args.get(2).ok_or(ERR_USAGE)?;
            Ok(ParsedCommand {
                kind: CommandKind::Create,
                target: Some(ProfileTarget::Custom(parse_create_id(id)?)),
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
            })
        }
        Some(action) if action == OsStr::new("use") && args.len() >= 3 => {
            let target = parse_target(args.get(2).ok_or(ERR_USAGE)?)?;
            let mut upstream_args = args[3..].to_vec();
            if upstream_args
                .first()
                .is_some_and(|arg| arg == OsStr::new("--"))
            {
                upstream_args.remove(0);
            }
            if upstream_args.first().is_some_and(|arg| {
                arg == OsStr::new("termux")
                    || arg == OsStr::new("doctor")
                    || arg == OsStr::new("update")
            }) {
                return Err(ERR_USAGE);
            }
            Ok(ParsedCommand {
                kind: CommandKind::Use,
                target: Some(target),
                upstream_args,
                session_id: None,
                all: false,
            })
        }
        _ => Err(ERR_USAGE),
    }
}

fn parse_session_command(args: &[OsString]) -> Result<ParsedCommand, ManagerError> {
    match args.get(1).map(OsString::as_os_str) {
        Some(action) if action == OsStr::new("list") => match args {
            [_, _, flag] if flag == OsStr::new("--all") => Ok(ParsedCommand {
                kind: CommandKind::SessionList,
                target: None,
                upstream_args: Vec::new(),
                session_id: None,
                all: true,
            }),
            [_, _, flag, value] if flag == OsStr::new("--profile") => Ok(ParsedCommand {
                kind: CommandKind::SessionList,
                target: Some(parse_target(value)?),
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
            }),
            [_, _] => Ok(ParsedCommand {
                kind: CommandKind::SessionList,
                target: None,
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
            }),
            _ => Err(ERR_USAGE),
        },
        Some(action) if action == OsStr::new("resume") && args.len() >= 3 => {
            let session_id = parse_session_ref(args.get(2).ok_or(ERR_USAGE)?)?;
            let mut index = 3;
            let target = if args
                .get(index)
                .is_some_and(|arg| arg == OsStr::new("--profile"))
            {
                let target = parse_target(args.get(index + 1).ok_or(ERR_USAGE)?)?;
                index += 2;
                Some(target)
            } else {
                None
            };
            if args.get(index).is_some_and(|arg| arg == OsStr::new("--")) {
                index += 1;
            }
            Ok(ParsedCommand {
                kind: CommandKind::SessionResume,
                target,
                upstream_args: args[index..].to_vec(),
                session_id: Some(session_id),
                all: false,
            })
        }
        _ => Err(ERR_USAGE),
    }
}

fn is_exact(value: Option<&OsString>, expected: &str) -> bool {
    value.is_some_and(|value| value == OsStr::new(expected))
}

fn parse_create_id(value: &OsString) -> Result<String, ManagerError> {
    let id = parse_id(value).ok_or(ERR_USAGE)?;
    if is_reserved_custom_id(&id) {
        return Err(ERR_USAGE);
    }
    Ok(id)
}

fn parse_target(value: &OsString) -> Result<ProfileTarget, ManagerError> {
    let id = parse_id(value).ok_or(ERR_USAGE)?;
    match id.as_str() {
        "default" | "home" => Ok(ProfileTarget::Default),
        _ if is_reserved_custom_id(&id) => Err(ERR_USAGE),
        _ => Ok(ProfileTarget::Custom(id)),
    }
}

fn parse_session_ref(value: &OsString) -> Result<String, ManagerError> {
    parse_session_ref_bytes(value.as_os_str().as_bytes())
}

fn parse_session_ref_bytes(bytes: &[u8]) -> Result<String, ManagerError> {
    if bytes.is_empty() || bytes.len() > SESSION_REF_MAX_BYTES || bytes == b"." || bytes == b".." {
        return Err(ERR_USAGE);
    }
    if !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(ERR_USAGE);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| ERR_USAGE)
}

fn parse_id(value: &OsStr) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return None;
    }
    if !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn is_reserved_custom_id(id: &str) -> bool {
    matches!(id, "default" | "home" | "termux" | "." | "..")
}

fn capture_context() -> Result<Context, ManagerError> {
    let home_value = std::env::var_os("HOME").ok_or(ERR_HOME)?;
    let home = PathBuf::from(home_value);
    if !is_safe_absolute_path(&home) || !is_existing_directory(&home)? {
        return Err(ERR_HOME);
    }

    let api = std::env::var_os(CORE_API_ENV).ok_or(ERR_HANDOFF)?;
    if api != OsStr::new(CORE_API) {
        return Err(ERR_HANDOFF);
    }
    let entrypoint_value = std::env::var_os(CORE_ENTRYPOINT_ENV).ok_or(ERR_HANDOFF)?;
    let core_entrypoint = PathBuf::from(entrypoint_value);
    if !is_safe_absolute_path(&core_entrypoint) || !is_existing_executable(&core_entrypoint)? {
        return Err(ERR_HANDOFF);
    }

    Ok(Context {
        home,
        inherited_codex_home: std::env::var_os(CODEX_HOME_ENV),
        core_entrypoint,
    })
}

fn is_safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn inspect_path(path: &Path) -> Result<PathPresence, ManagerError> {
    if !is_safe_absolute_path(path) {
        return Err(ERR_PATH);
    }
    let components: Vec<OsString> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_owned()),
            Component::RootDir => None,
            _ => None,
        })
        .collect();
    let mut current = PathBuf::from("/");
    let component_count = components.len();
    for (index, component) in components.into_iter().enumerate() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || (index + 1 != component_count && !metadata.is_dir())
                {
                    return Err(ERR_PATH);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(PathPresence::Missing);
            }
            Err(_) => return Err(ERR_PATH),
        }
    }
    Ok(PathPresence::Present)
}

fn is_existing_directory(path: &Path) -> Result<bool, ManagerError> {
    if inspect_path(path)? != PathPresence::Present {
        return Ok(false);
    }
    Ok(fs::symlink_metadata(path).map_err(|_| ERR_PATH)?.is_dir())
}

fn is_existing_executable(path: &Path) -> Result<bool, ManagerError> {
    if inspect_path(path)? != PathPresence::Present {
        return Ok(false);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| ERR_HANDOFF)?;
    Ok(metadata.file_type().is_file() && metadata.permissions().mode() & 0o111 != 0)
}

fn ensure_directory_chain(path: &Path) -> Result<(), ManagerError> {
    if !is_safe_absolute_path(path) {
        return Err(ERR_PATH);
    }
    let components: Vec<OsString> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_owned()),
            Component::RootDir => None,
            _ => None,
        })
        .collect();
    let mut current = PathBuf::from("/");
    for component in components {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(ERR_PATH);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|_| ERR_CREATE)?;
                set_mode(&current, PRIVATE_DIR_MODE).map_err(|_| ERR_CREATE)?;
                let metadata = fs::symlink_metadata(&current).map_err(|_| ERR_CREATE)?;
                if metadata.file_type().is_symlink()
                    || !metadata.is_dir()
                    || metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
                {
                    return Err(ERR_PATH);
                }
            }
            Err(_) => return Err(ERR_PATH),
        }
    }
    Ok(())
}

fn ensure_private_directory(path: &Path) -> Result<(), ManagerError> {
    ensure_directory_chain(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| ERR_PATH)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return Err(ERR_PATH);
    }
    Ok(())
}

fn manager_base(context: &Context) -> PathBuf {
    context.home.join(".local").join("share").join("codex")
}

fn manager_dirs_for_create(context: &Context) -> Result<ManagerDirs, ManagerError> {
    let base = manager_base(context);
    ensure_directory_chain(&base)?;
    let root = base.join("manager");
    let profiles = root.join(PROFILES_DIR);
    ensure_private_directory(&root)?;
    ensure_private_directory(&profiles)?;
    Ok(ManagerDirs { root, profiles })
}

fn existing_manager_dirs(context: &Context) -> Result<Option<ManagerDirs>, ManagerError> {
    let base = manager_base(context);
    if inspect_path(&base)? == PathPresence::Missing {
        return Ok(None);
    }
    let root = base.join("manager");
    if inspect_path(&root)? == PathPresence::Missing {
        return Ok(None);
    }
    let root_metadata = fs::symlink_metadata(&root).map_err(|_| ERR_PATH)?;
    if !root_metadata.is_dir() || root_metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE {
        return Err(ERR_PATH);
    }
    let profiles = root.join(PROFILES_DIR);
    if inspect_path(&profiles)? == PathPresence::Missing {
        return Ok(Some(ManagerDirs { root, profiles }));
    }
    let profiles_metadata = fs::symlink_metadata(&profiles).map_err(|_| ERR_PATH)?;
    if !profiles_metadata.is_dir()
        || profiles_metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return Err(ERR_PATH);
    }
    Ok(Some(ManagerDirs { root, profiles }))
}

fn profile_path(dirs: &ManagerDirs, id: &str) -> PathBuf {
    dirs.profiles.join(id)
}

fn profile_home_path(dirs: &ManagerDirs, id: &str) -> PathBuf {
    profile_path(dirs, id).join("home")
}

fn profile_complete(dirs: &ManagerDirs, id: &str) -> bool {
    let profile = profile_path(dirs, id);
    let Ok(profile_metadata) = fs::symlink_metadata(&profile) else {
        return false;
    };
    if profile_metadata.file_type().is_symlink()
        || !profile_metadata.is_dir()
        || profile_metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return false;
    }
    let home = profile.join("home");
    let Ok(home_metadata) = fs::symlink_metadata(&home) else {
        return false;
    };
    if home_metadata.file_type().is_symlink()
        || !home_metadata.is_dir()
        || home_metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return false;
    }
    let meta = profile.join(PROFILE_META);
    let Ok(meta_metadata) = fs::symlink_metadata(&meta) else {
        return false;
    };
    if meta_metadata.file_type().is_symlink()
        || !meta_metadata.is_file()
        || meta_metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE
    {
        return false;
    }
    let Ok(bytes) = read_bounded(&meta) else {
        return false;
    };
    bytes == profile_meta_bytes(id)
}

fn list_custom_profiles(context: &Context) -> Result<Vec<String>, ManagerError> {
    let Some(dirs) = existing_manager_dirs(context)? else {
        return Ok(Vec::new());
    };
    if !dirs.profiles.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(&dirs.profiles).map_err(|_| ERR_PATH)?;
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name();
        let Some(id) = parse_id(&name) else {
            continue;
        };
        if is_reserved_custom_id(&id) || !profile_complete(&dirs, &id) {
            continue;
        }
        ids.push(id);
    }
    ids.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    Ok(ids)
}

fn format_profile_list(context: &Context) -> Result<String, ManagerError> {
    let mut output = String::from("default\n");
    for id in list_custom_profiles(context)? {
        output.push_str(&id);
        output.push('\n');
    }
    Ok(output)
}

fn format_current(context: &Context) -> Result<String, ManagerError> {
    if let Some(inherited) = context.inherited_codex_home.as_ref() {
        let default_home = context.home.join(".codex");
        let target = if inherited == default_home.as_os_str() {
            "default".to_owned()
        } else {
            let mut target = "external".to_owned();
            let Some(dirs) = existing_manager_dirs(context)? else {
                return Ok(format!("current: {target}\nsource: inherited\n"));
            };
            for id in list_custom_profiles(context)? {
                if inherited == profile_home_path(&dirs, &id).as_os_str() {
                    target = id;
                    break;
                }
            }
            target
        };
        return Ok(format!("current: {target}\nsource: inherited\n"));
    }

    let selection = read_selection(context)?;
    Ok(format!(
        "current: {}\nsource: last-selection\n",
        selection.display()
    ))
}

fn format_session_list(
    context: &Context,
    target: Option<&ProfileTarget>,
    all: bool,
) -> Result<String, ManagerError> {
    let mut entries = Vec::new();
    let mut directory_entry_count = 0;
    for (profile, home) in session_targets(context, target, all)? {
        let Some(home) = home else {
            continue;
        };
        discover_sessions_in_home(&home, &profile, &mut entries, &mut directory_entry_count)?;
    }
    entries.sort_by(|left, right| {
        right
            .updated_unix_seconds
            .cmp(&left.updated_unix_seconds)
            .then_with(|| left.profile.as_bytes().cmp(right.profile.as_bytes()))
            .then_with(|| left.session_id.as_bytes().cmp(right.session_id.as_bytes()))
            .then_with(|| left.sort_key.cmp(&right.sort_key))
    });

    let mut output = String::new();
    for entry in entries {
        output.push_str(&entry.profile);
        output.push('\t');
        output.push_str(&entry.session_id);
        output.push('\t');
        output.push_str(&entry.updated_unix_seconds.to_string());
        output.push('\n');
    }
    Ok(output)
}

fn session_targets(
    context: &Context,
    target: Option<&ProfileTarget>,
    all: bool,
) -> Result<Vec<(String, Option<PathBuf>)>, ManagerError> {
    if all {
        let mut targets = vec![(
            "default".to_owned(),
            existing_real_directory(&context.home.join(".codex")),
        )];
        let Some(dirs) = existing_manager_dirs(context)? else {
            return Ok(targets);
        };
        for id in list_custom_profiles(context)? {
            if profile_complete(&dirs, &id) {
                targets.push((id.clone(), Some(profile_home_path(&dirs, &id))));
            }
        }
        return Ok(targets);
    }

    let selected = target
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| read_selection(context))?;
    let profile = selected.display().to_owned();
    let home = match &selected {
        ProfileTarget::Default => existing_real_directory(&context.home.join(".codex")),
        ProfileTarget::Custom(id) => {
            let dirs = existing_manager_dirs(context)?.ok_or(ERR_PROFILE)?;
            if !profile_complete(&dirs, id) {
                return Err(ERR_PROFILE);
            }
            Some(profile_home_path(&dirs, id))
        }
    };
    Ok(vec![(profile, home)])
}

fn existing_real_directory(path: &Path) -> Option<PathBuf> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    Some(path.to_owned())
}

fn discover_sessions_in_home(
    home: &Path,
    profile: &str,
    entries: &mut Vec<SessionEntry>,
    directory_entry_count: &mut usize,
) -> Result<(), ManagerError> {
    let sessions = home.join("sessions");
    if existing_real_directory(&sessions).is_none() {
        return Ok(());
    }
    collect_session_directory(&sessions, &[], 0, profile, entries, directory_entry_count)
}

fn collect_session_directory(
    directory: &Path,
    relative_prefix: &[u8],
    depth: usize,
    profile: &str,
    entries: &mut Vec<SessionEntry>,
    directory_entry_count: &mut usize,
) -> Result<(), ManagerError> {
    let metadata = match fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(());
    }

    let directory_entries = match fs::read_dir(directory) {
        Ok(directory_entries) => directory_entries,
        Err(_) => return Ok(()),
    };
    for directory_entry in directory_entries {
        let directory_entry = match directory_entry {
            Ok(directory_entry) => directory_entry,
            Err(_) => continue,
        };
        *directory_entry_count = directory_entry_count.saturating_add(1);
        if *directory_entry_count > SESSION_DISCOVERY_MAX_ENTRIES {
            return Err(ERR_SESSION);
        }

        let name = directory_entry.file_name();
        let path = directory_entry.path();
        let child_metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if child_metadata.file_type().is_symlink() {
            continue;
        }

        let mut sort_key = Vec::with_capacity(relative_prefix.len() + name.as_bytes().len() + 1);
        sort_key.extend_from_slice(relative_prefix);
        if !sort_key.is_empty() {
            sort_key.push(b'/');
        }
        sort_key.extend_from_slice(name.as_bytes());

        if child_metadata.is_dir() {
            if depth < SESSION_DISCOVERY_MAX_DEPTH {
                collect_session_directory(
                    &path,
                    &sort_key,
                    depth + 1,
                    profile,
                    entries,
                    directory_entry_count,
                )?;
            }
            continue;
        }
        if !child_metadata.is_file()
            || !name.as_bytes().ends_with(SESSION_FILE_SUFFIX)
            || child_metadata.len() > SESSION_FILE_MAX_BYTES
        {
            continue;
        }

        let Some(session_bytes) = name.as_bytes().strip_suffix(SESSION_FILE_SUFFIX) else {
            continue;
        };
        let Ok(session_id) = parse_session_ref_bytes(session_bytes) else {
            continue;
        };
        let Ok(updated) = child_metadata.modified() else {
            continue;
        };
        let Ok(updated) = updated.duration_since(std::time::UNIX_EPOCH) else {
            continue;
        };
        if File::open(&path).is_err() {
            continue;
        }
        entries.push(SessionEntry {
            profile: profile.to_owned(),
            session_id,
            updated_unix_seconds: updated.as_secs(),
            sort_key,
        });
    }
    Ok(())
}

fn resume_session(
    context: &Context,
    target: Option<&ProfileTarget>,
    session_id: &str,
    upstream_args: &[OsString],
) -> Result<(), ManagerError> {
    let selected = target
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| read_selection(context))?;
    let (profile, home) = session_targets(context, Some(&selected), false)?
        .into_iter()
        .next()
        .expect("one selected session target");
    let mut entries = Vec::new();
    let mut directory_entry_count = 0;
    if let Some(home) = home {
        discover_sessions_in_home(&home, &profile, &mut entries, &mut directory_entry_count)?;
    }
    let mut matches = entries
        .into_iter()
        .filter(|entry| entry.session_id == session_id);
    let Some(entry) = matches.next() else {
        return Err(ERR_SESSION);
    };
    if matches.next().is_some() {
        return Err(ERR_SESSION);
    }

    publish_selection(context, &selected)?;
    let mut core_args = Vec::with_capacity(upstream_args.len() + 2);
    core_args.push(OsString::from("resume"));
    core_args.push(OsString::from(entry.session_id));
    core_args.extend_from_slice(upstream_args);
    launch_core(context, &selected, &core_args)
}

fn create_profile(context: &Context, id: &str) -> Result<(), ManagerError> {
    let dirs = manager_dirs_for_create(context)?;
    let destination = profile_path(&dirs, id);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(ERR_COLLISION);
    }
    let temporary = dirs.profiles.join(format!(
        ".create-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|_| ERR_CREATE)?;
    set_mode(&temporary, PRIVATE_DIR_MODE).map_err(|_| {
        let _ = remove_private_tree(&temporary);
        ERR_CREATE
    })?;
    let result = (|| {
        let home = temporary.join("home");
        fs::create_dir(&home).map_err(|_| ERR_CREATE)?;
        set_mode(&home, PRIVATE_DIR_MODE).map_err(|_| ERR_CREATE)?;
        write_new_record(&temporary.join(PROFILE_META), &profile_meta_bytes(id))?;
        sync_directory(&temporary)?;
        rename_noreplace(&temporary, &destination).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                ERR_COLLISION
            } else {
                ERR_CREATE
            }
        })?;
        sync_directory(&dirs.profiles)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = remove_private_tree(&temporary);
    }
    result
}

fn use_profile(
    context: &Context,
    target: ProfileTarget,
    upstream_args: &[OsString],
) -> Result<(), ManagerError> {
    let dirs = existing_manager_dirs(context)?;
    if let ProfileTarget::Custom(id) = &target {
        let Some(dirs) = dirs.as_ref() else {
            return Err(ERR_PROFILE);
        };
        if !profile_complete(dirs, id) {
            return Err(ERR_PROFILE);
        }
    }
    publish_selection(context, &target)?;
    launch_core(context, &target, upstream_args)
}

fn publish_selection(context: &Context, target: &ProfileTarget) -> Result<(), ManagerError> {
    let _ = read_selection(context)?;
    let dirs = manager_dirs_for_create(context)?;
    let state = dirs.root.join(STATE_FILE);
    let bytes = state_bytes(target);
    if let Ok(metadata) = fs::symlink_metadata(&state) {
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE
        {
            return Err(ERR_STATE);
        }
    }
    let temporary = dirs.root.join(format!(
        ".state-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        write_new_record(&temporary, &bytes)?;
        fs::rename(&temporary, &state).map_err(|_| ERR_STATE)?;
        sync_directory(&dirs.root)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_selection(context: &Context) -> Result<ProfileTarget, ManagerError> {
    let Some(dirs) = existing_manager_dirs(context)? else {
        return Ok(ProfileTarget::Default);
    };
    let state = dirs.root.join(STATE_FILE);
    let metadata = match fs::symlink_metadata(&state) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProfileTarget::Default);
        }
        Err(_) => return Err(ERR_STATE),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE
    {
        return Err(ERR_STATE);
    }
    let bytes = read_bounded(&state).map_err(|_| ERR_STATE)?;
    let selection = parse_state_bytes(&bytes)?;
    if let ProfileTarget::Custom(id) = &selection {
        if !profile_complete(&dirs, id) {
            return Err(ERR_STATE);
        }
    }
    Ok(selection)
}

fn parse_state_bytes(bytes: &[u8]) -> Result<ProfileTarget, ManagerError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ERR_STATE)?;
    let expected_prefix = format!("{STATE_HEADER}\nlast_profile\t");
    let id = text
        .strip_prefix(&expected_prefix)
        .and_then(|rest| rest.strip_suffix('\n'))
        .filter(|id| !id.contains('\n') && !id.contains('\r'))
        .ok_or(ERR_STATE)?;
    if id == "default" {
        return Ok(ProfileTarget::Default);
    }
    let id_os = OsString::from(id);
    let parsed = parse_id(&id_os).ok_or(ERR_STATE)?;
    if is_reserved_custom_id(&parsed) {
        return Err(ERR_STATE);
    }
    Ok(ProfileTarget::Custom(parsed))
}

fn profile_meta_bytes(id: &str) -> Vec<u8> {
    format!("{PROFILE_HEADER}\nid\t{id}\n").into_bytes()
}

fn state_bytes(target: &ProfileTarget) -> Vec<u8> {
    format!("{STATE_HEADER}\nlast_profile\t{}\n", target.display()).into_bytes()
}

fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((RECORD_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > RECORD_MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "record exceeds bound",
        ));
    }
    Ok(bytes)
}

fn write_new_record(path: &Path, bytes: &[u8]) -> Result<(), ManagerError> {
    if bytes.len() > RECORD_MAX_BYTES {
        return Err(ERR_CREATE);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .map_err(|_| ERR_CREATE)?;
    file.write_all(bytes).map_err(|_| ERR_CREATE)?;
    file.sync_all().map_err(|_| ERR_CREATE)?;
    let metadata = file.metadata().map_err(|_| ERR_CREATE)?;
    if metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE {
        return Err(ERR_CREATE);
    }
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let mut permissions = fs::symlink_metadata(path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)
}

fn sync_directory(path: &Path) -> Result<(), ManagerError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ERR_CREATE)
}

fn remove_private_tree(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(path),
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn launch_core(
    context: &Context,
    target: &ProfileTarget,
    upstream_args: &[OsString],
) -> Result<(), ManagerError> {
    let mut command = Command::new(&context.core_entrypoint);
    command.args(upstream_args);
    command.env_remove(CORE_API_ENV);
    command.env_remove(CORE_ENTRYPOINT_ENV);
    match target {
        ProfileTarget::Default => {
            command.env_remove(CODEX_HOME_ENV);
        }
        ProfileTarget::Custom(id) => {
            let dirs = existing_manager_dirs(context)?.ok_or(ERR_PROFILE)?;
            let home = profile_home_path(&dirs, id);
            if !profile_complete(&dirs, id) {
                return Err(ERR_PROFILE);
            }
            command.env(CODEX_HOME_ENV, home);
        }
    }
    let error = command.exec();
    let _ = error;
    Err(ERR_LAUNCH)
}

fn rename_noreplace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::ffi::CString;

    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
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
    // SAFETY: both C strings are NUL-terminated and live through the syscall.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let sequence = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "codex-manager-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&root).unwrap();
            set_mode(&root, PRIVATE_DIR_MODE).unwrap();
            Self(root)
        }

        fn context(&self) -> Context {
            Context {
                home: self.0.clone(),
                inherited_codex_home: None,
                core_entrypoint: PathBuf::from("/bin/true"),
            }
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            remove_private_tree(&self.0).unwrap();
        }
    }

    #[test]
    fn parses_exact_profile_grammar_and_keeps_raw_arguments() {
        let args = vec![
            OsString::from("profile"),
            OsString::from("use"),
            OsString::from("work"),
            OsString::from("--"),
            OsString::from("--model"),
            OsString::from("gpt-5"),
        ];
        let parsed = parse_command(&args).unwrap();
        assert_eq!(parsed.kind, CommandKind::Use);
        assert_eq!(
            parsed.target,
            Some(ProfileTarget::Custom("work".to_owned()))
        );
        assert_eq!(
            parsed.upstream_args,
            vec![OsString::from("--model"), OsString::from("gpt-5")]
        );
    }

    #[test]
    fn parser_keeps_non_utf8_upstream_argument_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let raw = OsString::from_vec(vec![0xff, 0x80, b'x']);
        let args = vec![
            OsString::from("profile"),
            OsString::from("use"),
            OsString::from("work"),
            OsString::from("--"),
            raw.clone(),
        ];
        let parsed = parse_command(&args).unwrap();
        assert_eq!(parsed.upstream_args, vec![raw]);
    }

    #[test]
    fn parses_session_grammar_and_preserves_raw_resume_arguments() {
        use std::os::unix::ffi::OsStringExt;

        let list = parse_command(&[
            OsString::from("session"),
            OsString::from("list"),
            OsString::from("--profile"),
            OsString::from("work"),
        ])
        .unwrap();
        assert_eq!(list.kind, CommandKind::SessionList);
        assert_eq!(list.target, Some(ProfileTarget::Custom("work".to_owned())));
        assert!(!list.all);

        let all = parse_command(&[
            OsString::from("session"),
            OsString::from("list"),
            OsString::from("--all"),
        ])
        .unwrap();
        assert_eq!(all.kind, CommandKind::SessionList);
        assert!(all.all);

        let raw = OsString::from_vec(vec![0xff, 0x80, b'x']);
        let resumed = parse_command(&[
            OsString::from("session"),
            OsString::from("resume"),
            OsString::from("rollout-2026-09-05T00-00-00-abc"),
            OsString::from("--profile"),
            OsString::from("work"),
            OsString::from("--"),
            OsString::from("--model"),
            raw.clone(),
        ])
        .unwrap();
        assert_eq!(resumed.kind, CommandKind::SessionResume);
        assert_eq!(
            resumed.session_id,
            Some("rollout-2026-09-05T00-00-00-abc".to_owned())
        );
        assert_eq!(
            resumed.target,
            Some(ProfileTarget::Custom("work".to_owned()))
        );
        assert_eq!(resumed.upstream_args, vec![OsString::from("--model"), raw]);
    }

    #[test]
    fn rejects_ambiguous_session_options_and_unsafe_references() {
        use std::os::unix::ffi::OsStringExt;

        assert_eq!(
            parse_command(&[
                OsString::from("session"),
                OsString::from("list"),
                OsString::from("--all"),
                OsString::from("--profile"),
                OsString::from("work"),
            ]),
            Err(ERR_USAGE)
        );
        assert_eq!(
            parse_command(&[
                OsString::from("session"),
                OsString::from("resume"),
                OsString::from("ref"),
                OsString::from("--profile"),
            ]),
            Err(ERR_USAGE)
        );
        for value in ["../escape", ".", "..", "-leading", "bad/ref", "bad ref"] {
            assert_eq!(
                parse_command(&[
                    OsString::from("session"),
                    OsString::from("resume"),
                    OsString::from(value),
                ]),
                Err(ERR_USAGE),
                "{value}"
            );
        }
        let non_utf8 = OsString::from_vec(vec![0xff, b'x']);
        assert_eq!(parse_session_ref(&non_utf8), Err(ERR_USAGE));
        let oversized = OsString::from("a".repeat(SESSION_REF_MAX_BYTES + 1));
        assert_eq!(parse_session_ref(&oversized), Err(ERR_USAGE));
    }

    #[test]
    fn session_listing_is_bounded_metadata_only_and_deterministic() {
        use std::os::unix::ffi::OsStringExt;

        let root = TestRoot::new();
        let context = root.context();
        let session_root = context.home.join(".codex/sessions");
        let sessions = context.home.join(".codex/sessions/2026/09");
        ensure_directory_chain(&sessions).unwrap();

        fs::write(sessions.join("zeta.jsonl"), b"session-body-secret-sentinel").unwrap();
        fs::write(sessions.join("alpha.jsonl"), b"opaque body").unwrap();
        let nested = sessions.join("nested");
        ensure_directory_chain(&nested).unwrap();
        fs::write(nested.join("nested.jsonl"), b"nested opaque body").unwrap();

        let mut deep = session_root;
        for level in 1..=SESSION_DISCOVERY_MAX_DEPTH {
            deep = deep.join(format!("level-{level}"));
            ensure_directory_chain(&deep).unwrap();
        }
        fs::write(deep.join("deep.jsonl"), b"deep opaque body").unwrap();
        let too_deep = deep.join("level-too-deep");
        ensure_directory_chain(&too_deep).unwrap();
        fs::write(too_deep.join("too-deep.jsonl"), b"must not be visited").unwrap();

        fs::write(sessions.join("-invalid.jsonl"), b"invalid ref").unwrap();
        fs::write(sessions.join("not-a-session.txt"), b"wrong suffix").unwrap();
        fs::write(sessions.join(".jsonl"), b"empty ref").unwrap();
        let non_utf8 = OsString::from_vec(vec![b'v', 0xff, b'.', b'j', b's', b'o', b'n', b'l']);
        fs::write(sessions.join(non_utf8), b"non-utf8 ref").unwrap();

        let outside = root.0.join("outside");
        fs::create_dir(&outside).unwrap();
        set_mode(&outside, PRIVATE_DIR_MODE).unwrap();
        let outside_session = outside.join("outside.jsonl");
        fs::write(&outside_session, b"outside body").unwrap();
        std::os::unix::fs::symlink(&outside_session, sessions.join("linked.jsonl")).unwrap();
        std::os::unix::fs::symlink(&outside, sessions.join("linked-directory")).unwrap();

        let oversized = sessions.join("oversized.jsonl");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&oversized)
            .unwrap()
            .set_len(SESSION_FILE_MAX_BYTES + 1)
            .unwrap();

        let output = format_session_list(&context, None, false).unwrap();
        let rows: Vec<Vec<&str>> = output
            .lines()
            .map(|line| line.split('\t').collect())
            .collect();
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|row| row.len() == 3));
        assert!(rows.iter().all(|row| row[0] == "default"));
        assert!(rows.iter().all(|row| row[2].parse::<u64>().is_ok()));
        let ids: Vec<&str> = rows.iter().map(|row| row[1]).collect();
        assert!(ids.contains(&"alpha"));
        assert!(ids.contains(&"zeta"));
        assert!(ids.contains(&"nested"));
        assert!(ids.contains(&"deep"));
        assert!(!ids.contains(&"too-deep"));
        assert!(!output.contains("session-body-secret-sentinel"));
        assert!(!output.contains(root.0.to_str().unwrap()));
        assert!(!manager_base(&context).exists());
    }

    #[test]
    fn session_listing_fails_closed_at_the_command_entry_bound() {
        let root = TestRoot::new();
        let context = root.context();
        let sessions = context.home.join(".codex/sessions");
        ensure_directory_chain(&sessions).unwrap();
        for index in 0..=SESSION_DISCOVERY_MAX_ENTRIES {
            fs::write(sessions.join(format!("entry-{index:04}.jsonl")), b"body").unwrap();
        }

        assert_eq!(format_session_list(&context, None, false), Err(ERR_SESSION));
        assert!(!manager_base(&context).exists());
    }

    #[test]
    fn rejects_core_selectors_before_any_state_action() {
        for selector in ["termux", "doctor", "update"] {
            let args = vec![
                OsString::from("profile"),
                OsString::from("use"),
                OsString::from("work"),
                OsString::from(selector),
            ];
            assert_eq!(parse_command(&args), Err(ERR_USAGE));
        }
    }

    #[test]
    fn rejects_traversal_reserved_and_non_ascii_ids() {
        for id in ["../escape", ".", "..", "default", "home", "termux", "é"] {
            let args = vec![
                OsString::from("profile"),
                OsString::from("create"),
                OsString::from(id),
            ];
            assert_eq!(parse_command(&args), Err(ERR_USAGE));
        }
    }

    #[test]
    fn accepts_default_and_home_only_as_default_targets() {
        for id in ["default", "home"] {
            let args = vec![
                OsString::from("profile"),
                OsString::from("use"),
                OsString::from(id),
            ];
            let parsed = parse_command(&args).unwrap();
            assert_eq!(parsed.target, Some(ProfileTarget::Default));
        }
    }

    #[test]
    fn state_and_metadata_records_are_exact_and_bounded() {
        assert_eq!(
            profile_meta_bytes("work"),
            b"codex-manager-profile-v1\nid\twork\n"
        );
        assert_eq!(
            state_bytes(&ProfileTarget::Custom("work".to_owned())),
            b"codex-manager-state-v1\nlast_profile\twork\n"
        );
        assert_eq!(
            parse_state_bytes(&state_bytes(&ProfileTarget::Default)).unwrap(),
            ProfileTarget::Default
        );
        assert_eq!(
            parse_state_bytes(b"codex-manager-state-v1\nlast_profile\t../x\n"),
            Err(ERR_STATE)
        );
    }

    #[test]
    fn creates_only_private_profile_material_and_lists_valid_entries() {
        let root = TestRoot::new();
        let context = root.context();

        create_profile(&context, "work").unwrap();
        let dirs = manager_dirs_for_create(&context).unwrap();
        let work_home = profile_home_path(&dirs, "work");
        let auth = work_home.join("auth.json");
        fs::write(&auth, b"opaque-test-sentinel").unwrap();
        set_mode(&auth, PRIVATE_FILE_MODE).unwrap();
        let default_auth = context.home.join(".codex").join("auth.json");
        ensure_directory_chain(default_auth.parent().unwrap()).unwrap();
        fs::write(&default_auth, b"default-sentinel").unwrap();

        create_profile(&context, "Alpha").unwrap();
        let malformed = dirs.profiles.join("bad");
        fs::create_dir(&malformed).unwrap();
        set_mode(&malformed, PRIVATE_DIR_MODE).unwrap();
        fs::write(malformed.join(PROFILE_META), b"malformed\n").unwrap();
        set_mode(&malformed.join(PROFILE_META), PRIVATE_FILE_MODE).unwrap();
        let outside = root.0.join("outside");
        fs::create_dir(&outside).unwrap();
        set_mode(&outside, PRIVATE_DIR_MODE).unwrap();
        std::os::unix::fs::symlink(&outside, dirs.profiles.join("link")).unwrap();

        assert_eq!(
            format_profile_list(&context).unwrap(),
            "default\nAlpha\nwork\n"
        );
        assert_eq!(fs::read(&auth).unwrap(), b"opaque-test-sentinel");
        assert_eq!(fs::read(&default_auth).unwrap(), b"default-sentinel");
        assert!(fs::symlink_metadata(dirs.profiles.join("link"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(create_profile(&context, "work").unwrap_err(), ERR_COLLISION);
        assert_eq!(
            format_profile_list(&context).unwrap(),
            "default\nAlpha\nwork\n"
        );
    }

    #[test]
    fn selection_is_atomic_and_invalid_state_never_launches_core() {
        let root = TestRoot::new();
        let context = root.context();
        create_profile(&context, "work").unwrap();

        publish_selection(&context, &ProfileTarget::Custom("work".to_owned())).unwrap();
        assert_eq!(
            fs::read(
                manager_dirs_for_create(&context)
                    .unwrap()
                    .root
                    .join(STATE_FILE)
            )
            .unwrap(),
            b"codex-manager-state-v1\nlast_profile\twork\n"
        );
        assert_eq!(
            format_current(&context).unwrap(),
            "current: work\nsource: last-selection\n"
        );

        let state = manager_dirs_for_create(&context)
            .unwrap()
            .root
            .join(STATE_FILE);
        fs::write(&state, b"codex-manager-state-v1\nlast_profile\tbroken\n").unwrap();
        set_mode(&state, PRIVATE_FILE_MODE).unwrap();
        assert_eq!(
            use_profile(&context, ProfileTarget::Default, &[]).unwrap_err(),
            ERR_STATE
        );
        assert_eq!(
            fs::read(&state).unwrap(),
            b"codex-manager-state-v1\nlast_profile\tbroken\n"
        );
        let outside = root.0.join("state-target");
        fs::write(&outside, b"do-not-touch").unwrap();
        fs::remove_file(&state).unwrap();
        std::os::unix::fs::symlink(&outside, &state).unwrap();
        assert_eq!(
            use_profile(&context, ProfileTarget::Default, &[]).unwrap_err(),
            ERR_STATE
        );
        assert_eq!(fs::read(&outside).unwrap(), b"do-not-touch");
        assert!(fs::symlink_metadata(&state)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn rejects_symlinked_manager_path_components() {
        let root = TestRoot::new();
        let context = root.context();
        let base = manager_base(&context);
        ensure_directory_chain(&base).unwrap();
        let outside = root.0.join("manager-outside");
        fs::create_dir(&outside).unwrap();
        set_mode(&outside, PRIVATE_DIR_MODE).unwrap();
        std::os::unix::fs::symlink(&outside, base.join("manager")).unwrap();

        assert_eq!(format_profile_list(&context).unwrap_err(), ERR_PATH);
        assert_eq!(create_profile(&context, "work").unwrap_err(), ERR_PATH);
    }
}
