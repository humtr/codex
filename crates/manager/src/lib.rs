#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

const CORE_API_ENV: &str = "CODEX_TERMUX_CORE_API";
const CORE_ENTRYPOINT_ENV: &str = "CODEX_TERMUX_CORE_ENTRYPOINT";
const CORE_API: &str = "codex-manager-core-v1";
const CORE_REQUEST_ENV: &str = "CODEX_TERMUX_CORE_REQUEST";
const CORE_OPERATION_ENV: &str = "CODEX_TERMUX_CORE_OPERATION";
const CORE_REPAIR_REQUEST: &str = "codex-manager-repair-v1";
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const STATE_FILE: &str = "state-v1";
const PROFILES_DIR: &str = "profiles";
const PROFILE_META: &str = "profile.meta";
const STATE_HEADER: &str = "codex-manager-state-v1";
const PROFILE_HEADER: &str = "codex-manager-profile-v1";
const NOTIFY_DIR: &str = "notifications";
const NOTIFY_CONFIG: &str = "config-v1";
const NOTIFY_HEADER: &str = "codex-manager-notify-v1";
const NOTIFY_DEFAULT_GROUP: &str = "codex-turns";
const RECORD_MAX_BYTES: usize = 4096;
const NOTIFY_INPUT_MAX_BYTES: usize = 64 * 1024;
const NOTIFY_PAYLOAD_MAX_BYTES: usize = 4096;
const NOTIFY_MAX_CHARS: usize = 4096;
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
    "codex termux notify show\n",
    "codex termux notify set [--channel <notification|toast|both>] [--hooks <none|all|EVENT[,EVENT...]>] [--content-chars <0|1..4096>] [--preserve-newlines <0|1>] [--toast-gravity <top|middle|bottom>] [--toast-short <0|1>] [--toast-background <empty|#RRGGBB>] [--toast-color <empty|#RRGGBB>] [--group <GROUP_ID>]\n",
    "codex termux repair plan\n",
    "codex termux repair apply\n",
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
const ERR_HANDOFF: ManagerError = ManagerError::operation("codex termux: Core handoff is invalid");
const ERR_HOME: ManagerError = ManagerError::operation("codex termux: HOME is invalid");
const ERR_PATH: ManagerError = ManagerError::operation("codex termux: Manager path is unsafe");
const ERR_STATE: ManagerError = ManagerError::operation("codex termux: selection state is invalid");
const ERR_PROFILE: ManagerError = ManagerError::operation("codex termux: profile is unavailable");
const ERR_SESSION: ManagerError = ManagerError::operation("codex termux: session is unavailable");
const ERR_COLLISION: ManagerError = ManagerError::operation("codex termux: profile already exists");
const ERR_CREATE: ManagerError = ManagerError::operation("codex termux: profile creation failed");
const ERR_LAUNCH: ManagerError = ManagerError::operation("codex termux: Core launch failed");
const ERR_NOTIFY_CONFIG: ManagerError =
    ManagerError::operation("codex termux: notification configuration is invalid");

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
    NotifyShow,
    NotifySet,
    NotifyEmit,
    RepairPlan,
    RepairApply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCommand {
    kind: CommandKind,
    target: Option<ProfileTarget>,
    upstream_args: Vec<OsString>,
    session_id: Option<String>,
    all: bool,
    notify_patch: Option<NotifyPatch>,
    notify_event: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotifyChannel {
    Notification,
    Toast,
    Both,
}

impl NotifyChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Notification => "notification",
            Self::Toast => "toast",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotifyGravity {
    Top,
    Middle,
    Bottom,
}

impl NotifyGravity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Middle => "middle",
            Self::Bottom => "bottom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NotifyHooks {
    None,
    All,
    Events(Vec<String>),
}

impl NotifyHooks {
    fn as_str(&self) -> String {
        match self {
            Self::None => "none".to_owned(),
            Self::All => "all".to_owned(),
            Self::Events(events) => events.join(","),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NotifyConfig {
    channel: NotifyChannel,
    hooks: NotifyHooks,
    content_chars: usize,
    preserve_newlines: bool,
    toast_gravity: NotifyGravity,
    toast_short: bool,
    toast_background: Option<String>,
    toast_color: Option<String>,
    group: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NotifyPatch {
    channel: Option<NotifyChannel>,
    hooks: Option<NotifyHooks>,
    content_chars: Option<usize>,
    preserve_newlines: Option<bool>,
    toast_gravity: Option<NotifyGravity>,
    toast_short: Option<bool>,
    toast_background: Option<Option<String>>,
    toast_color: Option<Option<String>>,
    group: Option<String>,
}

const NOTIFY_EVENTS: [&str; 10] = [
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "UserPromptSubmit",
    "SubagentStart",
    "SubagentStop",
    "Stop",
];

impl NotifyConfig {
    fn defaults() -> Self {
        Self {
            channel: NotifyChannel::Notification,
            hooks: NotifyHooks::Events(vec!["Stop".to_owned()]),
            content_chars: 0,
            preserve_newlines: true,
            toast_gravity: NotifyGravity::Top,
            toast_short: false,
            toast_background: None,
            toast_color: None,
            group: NOTIFY_DEFAULT_GROUP.to_owned(),
        }
    }

    fn merge(self, patch: NotifyPatch) -> Self {
        Self {
            channel: patch.channel.unwrap_or(self.channel),
            hooks: patch.hooks.unwrap_or(self.hooks),
            content_chars: patch.content_chars.unwrap_or(self.content_chars),
            preserve_newlines: patch.preserve_newlines.unwrap_or(self.preserve_newlines),
            toast_gravity: patch.toast_gravity.unwrap_or(self.toast_gravity),
            toast_short: patch.toast_short.unwrap_or(self.toast_short),
            toast_background: patch.toast_background.unwrap_or(self.toast_background),
            toast_color: patch.toast_color.unwrap_or(self.toast_color),
            group: patch.group.unwrap_or(self.group),
        }
    }
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
        CommandKind::NotifyShow => Ok(Some(format_notify_config(&read_notify_config(&context)?))),
        CommandKind::NotifySet => {
            let patch = command.notify_patch.expect("notify set patch is parsed");
            let config = read_notify_config(&context)?.merge(patch);
            publish_notify_config(&context, &config)?;
            Ok(Some("saved\n".to_owned()))
        }
        CommandKind::NotifyEmit => {
            let event = command
                .notify_event
                .as_deref()
                .expect("notify event is parsed");
            emit_notification(&context, event);
            Ok(None)
        }
        CommandKind::RepairPlan => {
            launch_repair(&context, false)?;
            Ok(None)
        }
        CommandKind::RepairApply => {
            launch_repair(&context, true)?;
            Ok(None)
        }
    }
}

fn parse_command(args: &[OsString]) -> Result<ParsedCommand, ManagerError> {
    if is_exact(args.first(), "repair") {
        return parse_repair_command(args);
    }
    if is_exact(args.first(), "notify") {
        return parse_notify_command(args);
    }
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
            notify_patch: None,
            notify_event: None,
        }),
        Some(action) if action == OsStr::new("current") && args.len() == 2 => Ok(ParsedCommand {
            kind: CommandKind::Current,
            target: None,
            upstream_args: Vec::new(),
            session_id: None,
            all: false,
            notify_patch: None,
            notify_event: None,
        }),
        Some(action) if action == OsStr::new("create") && args.len() == 3 => {
            let id = args.get(2).ok_or(ERR_USAGE)?;
            Ok(ParsedCommand {
                kind: CommandKind::Create,
                target: Some(ProfileTarget::Custom(parse_create_id(id)?)),
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
                notify_patch: None,
                notify_event: None,
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
                notify_patch: None,
                notify_event: None,
            })
        }
        _ => Err(ERR_USAGE),
    }
}

fn parse_repair_command(args: &[OsString]) -> Result<ParsedCommand, ManagerError> {
    let kind = match (args.get(1).map(OsString::as_os_str), args.len()) {
        (Some(action), 2) if action == OsStr::new("plan") => CommandKind::RepairPlan,
        (Some(action), 2) if action == OsStr::new("apply") => CommandKind::RepairApply,
        _ => return Err(ERR_USAGE),
    };
    Ok(ParsedCommand {
        kind,
        target: None,
        upstream_args: Vec::new(),
        session_id: None,
        all: false,
        notify_patch: None,
        notify_event: None,
    })
}

fn parse_notify_command(args: &[OsString]) -> Result<ParsedCommand, ManagerError> {
    match args.get(1).map(OsString::as_os_str) {
        Some(action) if action == OsStr::new("show") && args.len() == 2 => Ok(ParsedCommand {
            kind: CommandKind::NotifyShow,
            target: None,
            upstream_args: Vec::new(),
            session_id: None,
            all: false,
            notify_patch: None,
            notify_event: None,
        }),
        Some(action) if action == OsStr::new("set") => Ok(ParsedCommand {
            kind: CommandKind::NotifySet,
            target: None,
            upstream_args: Vec::new(),
            session_id: None,
            all: false,
            notify_patch: Some(parse_notify_set_args(&args[2..])?),
            notify_event: None,
        }),
        Some(action) if action == OsStr::new("emit") && args.len() == 3 => {
            let event = parse_notify_event(args.get(2).ok_or(ERR_USAGE)?, ERR_USAGE)?;
            Ok(ParsedCommand {
                kind: CommandKind::NotifyEmit,
                target: None,
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
                notify_patch: None,
                notify_event: Some(event),
            })
        }
        _ => Err(ERR_USAGE),
    }
}

fn parse_notify_set_args(args: &[OsString]) -> Result<NotifyPatch, ManagerError> {
    let mut patch = NotifyPatch::default();
    let mut index = 0;
    while index < args.len() {
        let option = args[index].to_str().ok_or(ERR_USAGE)?;
        let value = args.get(index + 1).ok_or(ERR_USAGE)?;
        match option {
            "--channel" => {
                if patch.channel.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.channel = Some(parse_notify_channel(value, ERR_USAGE)?);
            }
            "--hooks" => {
                if patch.hooks.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.hooks = Some(parse_notify_hooks(value, ERR_USAGE)?);
            }
            "--content-chars" => {
                if patch.content_chars.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.content_chars = Some(parse_notify_content_chars(value, ERR_USAGE)?);
            }
            "--preserve-newlines" => {
                if patch.preserve_newlines.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.preserve_newlines = Some(parse_notify_bool(value, ERR_USAGE)?);
            }
            "--toast-gravity" => {
                if patch.toast_gravity.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.toast_gravity = Some(parse_notify_gravity(value, ERR_USAGE)?);
            }
            "--toast-short" => {
                if patch.toast_short.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.toast_short = Some(parse_notify_bool(value, ERR_USAGE)?);
            }
            "--toast-background" => {
                if patch.toast_background.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.toast_background = Some(parse_notify_color(value, ERR_USAGE)?);
            }
            "--toast-color" => {
                if patch.toast_color.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.toast_color = Some(parse_notify_color(value, ERR_USAGE)?);
            }
            "--group" => {
                if patch.group.is_some() {
                    return Err(ERR_USAGE);
                }
                patch.group = Some(parse_notify_group(value, ERR_USAGE)?);
            }
            _ => return Err(ERR_USAGE),
        }
        index += 2;
    }
    Ok(patch)
}

fn parse_notify_channel(value: &OsStr, error: ManagerError) -> Result<NotifyChannel, ManagerError> {
    match value.to_str() {
        Some("notification") => Ok(NotifyChannel::Notification),
        Some("toast") => Ok(NotifyChannel::Toast),
        Some("both") => Ok(NotifyChannel::Both),
        _ => Err(error),
    }
}

fn parse_notify_gravity(value: &OsStr, error: ManagerError) -> Result<NotifyGravity, ManagerError> {
    match value.to_str() {
        Some("top") => Ok(NotifyGravity::Top),
        Some("middle") => Ok(NotifyGravity::Middle),
        Some("bottom") => Ok(NotifyGravity::Bottom),
        _ => Err(error),
    }
}

fn parse_notify_bool(value: &OsStr, error: ManagerError) -> Result<bool, ManagerError> {
    match value.to_str() {
        Some("0") => Ok(false),
        Some("1") => Ok(true),
        _ => Err(error),
    }
}

fn parse_notify_content_chars(value: &OsStr, error: ManagerError) -> Result<usize, ManagerError> {
    let value = value.to_str().ok_or(error)?;
    let parsed = value.parse::<usize>().map_err(|_| error)?;
    if parsed <= NOTIFY_MAX_CHARS {
        Ok(parsed)
    } else {
        Err(error)
    }
}

fn parse_notify_color(value: &OsStr, error: ManagerError) -> Result<Option<String>, ManagerError> {
    let value = value.to_str().ok_or(error)?;
    if value == "empty" {
        return Ok(None);
    }
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(error);
    }
    let normalized = bytes
        .iter()
        .map(|byte| char::from(byte.to_ascii_lowercase()))
        .collect();
    Ok(Some(normalized))
}

fn parse_notify_group(value: &OsStr, error: ManagerError) -> Result<String, ManagerError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(error);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| error)
}

fn parse_notify_event(value: &OsStr, error: ManagerError) -> Result<String, ManagerError> {
    let event = value.to_str().ok_or(error)?;
    if NOTIFY_EVENTS.contains(&event) {
        Ok(event.to_owned())
    } else {
        Err(error)
    }
}

fn parse_notify_hooks(value: &OsStr, error: ManagerError) -> Result<NotifyHooks, ManagerError> {
    let value = value.to_str().ok_or(error)?;
    match value {
        "none" => return Ok(NotifyHooks::None),
        "all" => return Ok(NotifyHooks::All),
        _ => {}
    }
    if value.is_empty() {
        return Err(error);
    }
    let mut events = Vec::new();
    for item in value.split(',') {
        if item.is_empty() || !NOTIFY_EVENTS.contains(&item) {
            return Err(error);
        }
        if events.iter().any(|event: &String| event == item) {
            return Err(error);
        }
        events.push(item.to_owned());
    }
    events.sort_by_key(|event| {
        NOTIFY_EVENTS
            .iter()
            .position(|candidate| candidate == event)
            .expect("event was validated")
    });
    Ok(NotifyHooks::Events(events))
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
                notify_patch: None,
                notify_event: None,
            }),
            [_, _, flag, value] if flag == OsStr::new("--profile") => Ok(ParsedCommand {
                kind: CommandKind::SessionList,
                target: Some(parse_target(value)?),
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
                notify_patch: None,
                notify_event: None,
            }),
            [_, _] => Ok(ParsedCommand {
                kind: CommandKind::SessionList,
                target: None,
                upstream_args: Vec::new(),
                session_id: None,
                all: false,
                notify_patch: None,
                notify_event: None,
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
                notify_patch: None,
                notify_event: None,
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

fn notify_directory_for_create(context: &Context) -> Result<PathBuf, ManagerError> {
    let base = manager_base(context);
    ensure_directory_chain(&base)?;
    let root = base.join("manager");
    ensure_private_directory(&root)?;
    let notifications = root.join(NOTIFY_DIR);
    ensure_private_directory(&notifications)?;
    Ok(notifications)
}

fn existing_notify_directory(context: &Context) -> Result<Option<PathBuf>, ManagerError> {
    let base = manager_base(context);
    if inspect_path(&base)? == PathPresence::Missing {
        return Ok(None);
    }
    let root = base.join("manager");
    if inspect_path(&root)? == PathPresence::Missing {
        return Ok(None);
    }
    let root_metadata = fs::symlink_metadata(&root).map_err(|_| ERR_PATH)?;
    if root_metadata.file_type().is_symlink()
        || !root_metadata.is_dir()
        || root_metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return Err(ERR_PATH);
    }
    let notifications = root.join(NOTIFY_DIR);
    if inspect_path(&notifications)? == PathPresence::Missing {
        return Ok(None);
    }
    let metadata = fs::symlink_metadata(&notifications).map_err(|_| ERR_PATH)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o7777 != PRIVATE_DIR_MODE
    {
        return Err(ERR_PATH);
    }
    Ok(Some(notifications))
}

fn read_notify_config(context: &Context) -> Result<NotifyConfig, ManagerError> {
    let Some(directory) = existing_notify_directory(context)? else {
        return Ok(NotifyConfig::defaults());
    };
    let path = directory.join(NOTIFY_CONFIG);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(NotifyConfig::defaults());
        }
        Err(_) => return Err(ERR_NOTIFY_CONFIG),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE
    {
        return Err(ERR_NOTIFY_CONFIG);
    }
    let bytes = read_bounded(&path).map_err(|_| ERR_NOTIFY_CONFIG)?;
    parse_notify_record(&bytes)
}

fn parse_notify_record(bytes: &[u8]) -> Result<NotifyConfig, ManagerError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ERR_NOTIFY_CONFIG)?;
    let mut lines = text.split('\n');
    if lines.next() != Some(NOTIFY_HEADER) {
        return Err(ERR_NOTIFY_CONFIG);
    }
    let channel = parse_notify_channel_record(lines.next(), "channel")?;
    let hooks = parse_notify_hooks_record(lines.next(), "hooks")?;
    let content_chars = parse_notify_content_record(lines.next(), "content_chars")?;
    let preserve_newlines = parse_notify_bool_record(lines.next(), "preserve_newlines")?;
    let toast_gravity = parse_notify_gravity_record(lines.next(), "toast_gravity")?;
    let toast_short = parse_notify_bool_record(lines.next(), "toast_short")?;
    let toast_background = parse_notify_color_record(lines.next(), "toast_background")?;
    let toast_color = parse_notify_color_record(lines.next(), "toast_color")?;
    let group = parse_notify_group_record(lines.next(), "group")?;
    if lines.next() != Some("") || lines.next().is_some() {
        return Err(ERR_NOTIFY_CONFIG);
    }
    Ok(NotifyConfig {
        channel,
        hooks,
        content_chars,
        preserve_newlines,
        toast_gravity,
        toast_short,
        toast_background,
        toast_color,
        group,
    })
}

fn record_value<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, ManagerError> {
    let prefix = format!("{key}\t");
    line.and_then(|line| line.strip_prefix(&prefix))
        .ok_or(ERR_NOTIFY_CONFIG)
}

fn parse_notify_channel_record(
    line: Option<&str>,
    key: &str,
) -> Result<NotifyChannel, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_channel(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_hooks_record(line: Option<&str>, key: &str) -> Result<NotifyHooks, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_hooks(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_content_record(line: Option<&str>, key: &str) -> Result<usize, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_content_chars(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_bool_record(line: Option<&str>, key: &str) -> Result<bool, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_bool(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_gravity_record(
    line: Option<&str>,
    key: &str,
) -> Result<NotifyGravity, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_gravity(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_color_record(
    line: Option<&str>,
    key: &str,
) -> Result<Option<String>, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_color(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn parse_notify_group_record(line: Option<&str>, key: &str) -> Result<String, ManagerError> {
    let value = record_value(line, key)?;
    parse_notify_group(OsStr::new(value), ERR_NOTIFY_CONFIG)
}

fn notify_record_bytes(config: &NotifyConfig) -> Vec<u8> {
    format!(
        "{NOTIFY_HEADER}\nchannel\t{}\nhooks\t{}\ncontent_chars\t{}\npreserve_newlines\t{}\ntoast_gravity\t{}\ntoast_short\t{}\ntoast_background\t{}\ntoast_color\t{}\ngroup\t{}\n",
        config.channel.as_str(),
        config.hooks.as_str(),
        config.content_chars,
        if config.preserve_newlines { 1 } else { 0 },
        config.toast_gravity.as_str(),
        if config.toast_short { 1 } else { 0 },
        config.toast_background.as_deref().unwrap_or("empty"),
        config.toast_color.as_deref().unwrap_or("empty"),
        config.group,
    )
    .into_bytes()
}

fn format_notify_config(config: &NotifyConfig) -> String {
    format!(
        "channel={}\nhooks={}\ncontent-chars={}\npreserve-newlines={}\ntoast-gravity={}\ntoast-short={}\ntoast-background={}\ntoast-color={}\ngroup={}\n",
        config.channel.as_str(),
        config.hooks.as_str(),
        config.content_chars,
        if config.preserve_newlines { 1 } else { 0 },
        config.toast_gravity.as_str(),
        if config.toast_short { 1 } else { 0 },
        config.toast_background.as_deref().unwrap_or("empty"),
        config.toast_color.as_deref().unwrap_or("empty"),
        config.group,
    )
}

fn publish_notify_config(context: &Context, config: &NotifyConfig) -> Result<(), ManagerError> {
    let directory = notify_directory_for_create(context)?;
    let destination = directory.join(NOTIFY_CONFIG);
    if let Ok(metadata) = fs::symlink_metadata(&destination) {
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.permissions().mode() & 0o7777 != PRIVATE_FILE_MODE
        {
            return Err(ERR_NOTIFY_CONFIG);
        }
    }
    let temporary = directory.join(format!(
        ".config-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        write_new_record(&temporary, &notify_record_bytes(config))
            .map_err(|_| ERR_NOTIFY_CONFIG)?;
        fs::rename(&temporary, &destination).map_err(|_| ERR_NOTIFY_CONFIG)?;
        sync_directory(&directory).map_err(|_| ERR_NOTIFY_CONFIG)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug, Default)]
struct HookText {
    title: Option<String>,
    body: Option<String>,
}

fn emit_notification(context: &Context, event: &str) {
    let Ok(config) = read_notify_config(context) else {
        return;
    };
    if !notify_event_enabled(&config.hooks, event) {
        return;
    }
    let Some(input) = read_hook_input() else {
        return;
    };
    let Some(text) = parse_hook_json(&input) else {
        return;
    };
    let title = normalize_notification_text(
        text.title.as_deref().unwrap_or("Codex"),
        config.preserve_newlines,
        0,
    );
    let body = normalize_notification_text(
        text.body
            .as_deref()
            .unwrap_or_else(|| notify_event_status(event)),
        config.preserve_newlines,
        config.content_chars,
    );
    match config.channel {
        NotifyChannel::Notification => {
            invoke_termux_notification(&config, &title, &body);
        }
        NotifyChannel::Toast => {
            invoke_termux_toast(&config, &body);
        }
        NotifyChannel::Both => {
            invoke_termux_notification(&config, &title, &body);
            invoke_termux_toast(&config, &body);
        }
    }
}

fn notify_event_enabled(hooks: &NotifyHooks, event: &str) -> bool {
    match hooks {
        NotifyHooks::None => false,
        NotifyHooks::All => true,
        NotifyHooks::Events(events) => events.iter().any(|candidate| candidate == event),
    }
}

fn read_hook_input() -> Option<Vec<u8>> {
    let stdin = io::stdin();
    let mut input = Vec::new();
    stdin
        .lock()
        .take((NOTIFY_INPUT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .ok()?;
    (input.len() <= NOTIFY_INPUT_MAX_BYTES).then_some(input)
}

fn notify_event_status(event: &str) -> &'static str {
    match event {
        "SessionStart" => "Notify session start",
        "PreToolUse" => "Notify tool start",
        "PermissionRequest" => "Notify permission request",
        "PostToolUse" => "Notify tool finish",
        "PreCompact" => "Notify before compact",
        "PostCompact" => "Notify after compact",
        "UserPromptSubmit" => "Notify prompt submit",
        "SubagentStart" => "Notify subagent start",
        "SubagentStop" => "Notify subagent stop",
        "Stop" => "Notify turn completion",
        _ => "Notify Codex event",
    }
}

fn normalize_notification_text(
    text: &str,
    preserve_newlines: bool,
    content_chars: usize,
) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    if !preserve_newlines {
        normalized = normalized
            .chars()
            .map(|character| if character == '\n' { ' ' } else { character })
            .collect();
    }
    if content_chars != 0 {
        normalized = normalized.chars().take(content_chars).collect();
    }
    truncate_utf8(&normalized, NOTIFY_PAYLOAD_MAX_BYTES)
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn invoke_termux_notification(config: &NotifyConfig, title: &str, body: &str) {
    let mut command = Command::new("termux-notification");
    command
        .arg("--group")
        .arg(&config.group)
        .arg("--priority")
        .arg("max")
        .arg("--sound")
        .arg("--vibrate")
        .arg("300,150,300")
        .arg("--title")
        .arg(title)
        .arg("--content")
        .arg(body);
    run_bounded_provider(command);
}

fn invoke_termux_toast(config: &NotifyConfig, body: &str) {
    let mut command = Command::new("termux-toast");
    command.arg("-g").arg(config.toast_gravity.as_str());
    if config.toast_short {
        command.arg("-s");
    }
    if let Some(background) = config.toast_background.as_deref() {
        command.arg("-b").arg(background);
    }
    if let Some(color) = config.toast_color.as_deref() {
        command.arg("-c").arg(color);
    }
    command.arg(body);
    run_bounded_provider(command);
}

fn run_bounded_provider(mut command: Command) {
    let Ok(mut child) = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    for _ in 0..200 {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => return,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

struct JsonCursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> JsonCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.index += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.bytes.get(self.index) == Some(&expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn parse_string(&mut self) -> Option<String> {
        if !self.consume(b'"') {
            return None;
        }
        let mut output = Vec::new();
        loop {
            let byte = *self.bytes.get(self.index)?;
            self.index += 1;
            match byte {
                b'"' => return String::from_utf8(output).ok(),
                b'\\' => {
                    let escaped = *self.bytes.get(self.index)?;
                    self.index += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => output.push(escaped),
                        b'b' => output.push(8),
                        b'f' => output.push(12),
                        b'n' => output.push(b'\n'),
                        b'r' => output.push(b'\r'),
                        b't' => output.push(b'\t'),
                        b'u' => self.parse_unicode_escape(&mut output)?,
                        _ => return None,
                    }
                }
                0..=0x1f => return None,
                _ => output.push(byte),
            }
        }
    }

    fn parse_unicode_escape(&mut self, output: &mut Vec<u8>) -> Option<()> {
        let first = self.parse_hex_quad()?;
        let codepoint = if (0xd800..=0xdbff).contains(&first) {
            if self.bytes.get(self.index..self.index + 2) != Some(b"\\u") {
                return None;
            }
            self.index += 2;
            let second = self.parse_hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return None;
            }
            0x1_0000 + ((first - 0xd800) << 10) + (second - 0xdc00)
        } else if (0xdc00..=0xdfff).contains(&first) {
            return None;
        } else {
            first
        };
        let character = char::from_u32(codepoint)?;
        let mut encoded = [0; 4];
        output.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
        Some(())
    }

    fn parse_hex_quad(&mut self) -> Option<u32> {
        let mut value = 0;
        for _ in 0..4 {
            value = (value << 4) | u32::from(hex_value(*self.bytes.get(self.index)?)?);
            self.index += 1;
        }
        Some(value)
    }

    fn skip_value(&mut self, depth: usize) -> Option<()> {
        if depth > 32 {
            return None;
        }
        self.skip_whitespace();
        match self.bytes.get(self.index)? {
            b'"' => {
                self.parse_string()?;
                Some(())
            }
            b'{' => self.skip_object(depth + 1),
            b'[' => self.skip_array(depth + 1),
            b't' if self.consume_literal(b"true") => Some(()),
            b'f' if self.consume_literal(b"false") => Some(()),
            b'n' if self.consume_literal(b"null") => Some(()),
            b'-' | b'0'..=b'9' => {
                self.skip_number()?;
                Some(())
            }
            _ => None,
        }
    }

    fn skip_object(&mut self, depth: usize) -> Option<()> {
        self.consume(b'{');
        self.skip_whitespace();
        if self.consume(b'}') {
            return Some(());
        }
        loop {
            self.skip_whitespace();
            self.parse_string()?;
            self.skip_whitespace();
            if !self.consume(b':') {
                return None;
            }
            self.skip_value(depth)?;
            self.skip_whitespace();
            if self.consume(b'}') {
                return Some(());
            }
            if !self.consume(b',') {
                return None;
            }
        }
    }

    fn skip_array(&mut self, depth: usize) -> Option<()> {
        self.consume(b'[');
        self.skip_whitespace();
        if self.consume(b']') {
            return Some(());
        }
        loop {
            self.skip_value(depth)?;
            self.skip_whitespace();
            if self.consume(b']') {
                return Some(());
            }
            if !self.consume(b',') {
                return None;
            }
        }
    }

    fn consume_literal(&mut self, literal: &[u8]) -> bool {
        if self.bytes.get(self.index..self.index + literal.len()) == Some(literal) {
            self.index += literal.len();
            true
        } else {
            false
        }
    }

    fn skip_number(&mut self) -> Option<()> {
        if self.consume(b'-') && self.bytes.get(self.index).is_none() {
            return None;
        }
        match self.bytes.get(self.index)? {
            b'0' => self.index += 1,
            b'1'..=b'9' => {
                self.index += 1;
                while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                    self.index += 1;
                }
            }
            _ => return None,
        }
        if self.consume(b'.') {
            let start = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if start == self.index {
                return None;
            }
        }
        if self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(byte, b'e' | b'E'))
        {
            self.index += 1;
            if self
                .bytes
                .get(self.index)
                .is_some_and(|byte| matches!(byte, b'+' | b'-'))
            {
                self.index += 1;
            }
            let start = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if start == self.index {
                return None;
            }
        }
        Some(())
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_hook_json(input: &[u8]) -> Option<HookText> {
    let mut cursor = JsonCursor::new(input);
    cursor.skip_whitespace();
    if !cursor.consume(b'{') {
        return None;
    }
    let mut text = HookText::default();
    let mut content = None;
    let mut last_assistant_message = None;
    let mut message = None;
    let mut seen_title = false;
    let mut seen_content = false;
    let mut seen_last_assistant_message = false;
    let mut seen_message = false;
    cursor.skip_whitespace();
    if !cursor.consume(b'}') {
        loop {
            cursor.skip_whitespace();
            let key = cursor.parse_string()?;
            cursor.skip_whitespace();
            if !cursor.consume(b':') {
                return None;
            }
            cursor.skip_whitespace();
            match key.as_str() {
                "title" => {
                    if seen_title {
                        return None;
                    }
                    seen_title = true;
                    if cursor.bytes.get(cursor.index) == Some(&b'"') {
                        text.title = Some(cursor.parse_string()?);
                    } else {
                        cursor.skip_value(0)?;
                    }
                }
                "content" => {
                    if seen_content {
                        return None;
                    }
                    seen_content = true;
                    if cursor.bytes.get(cursor.index) == Some(&b'"') {
                        content = Some(cursor.parse_string()?);
                    } else {
                        cursor.skip_value(0)?;
                    }
                }
                "last_assistant_message" => {
                    if seen_last_assistant_message {
                        return None;
                    }
                    seen_last_assistant_message = true;
                    if cursor.bytes.get(cursor.index) == Some(&b'"') {
                        last_assistant_message = Some(cursor.parse_string()?);
                    } else {
                        cursor.skip_value(0)?;
                    }
                }
                "message" => {
                    if seen_message {
                        return None;
                    }
                    seen_message = true;
                    if cursor.bytes.get(cursor.index) == Some(&b'"') {
                        message = Some(cursor.parse_string()?);
                    } else {
                        cursor.skip_value(0)?;
                    }
                }
                _ => cursor.skip_value(0)?,
            }
            cursor.skip_whitespace();
            if cursor.consume(b'}') {
                break;
            }
            if !cursor.consume(b',') {
                return None;
            }
        }
    }
    cursor.skip_whitespace();
    if cursor.index != input.len() {
        return None;
    }
    text.body = content.or(last_assistant_message).or(message);
    Some(text)
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
    command.env_remove(CORE_REQUEST_ENV);
    command.env_remove(CORE_OPERATION_ENV);
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

fn launch_repair(context: &Context, apply: bool) -> Result<(), ManagerError> {
    let mut command = Command::new(&context.core_entrypoint);
    if apply {
        command.arg("update");
    } else {
        command.args(["doctor", "--json"]);
    }
    command
        .env_remove(CORE_API_ENV)
        .env_remove(CORE_ENTRYPOINT_ENV)
        .env_remove(CODEX_HOME_ENV)
        .env(CORE_REQUEST_ENV, CORE_REPAIR_REQUEST)
        .env(CORE_OPERATION_ENV, if apply { "apply" } else { "plan" });
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
    fn parses_exact_repair_grammar_and_rejects_extra_arguments() {
        assert_eq!(
            parse_command(&[OsString::from("repair"), OsString::from("plan")])
                .unwrap()
                .kind,
            CommandKind::RepairPlan
        );
        assert_eq!(
            parse_command(&[OsString::from("repair"), OsString::from("apply")])
                .unwrap()
                .kind,
            CommandKind::RepairApply
        );
        for args in [
            vec![OsString::from("repair")],
            vec![
                OsString::from("repair"),
                OsString::from("plan"),
                OsString::from("--json"),
            ],
            vec![
                OsString::from("repair"),
                OsString::from("apply"),
                OsString::from("extra"),
            ],
            vec![OsString::from("repair"), OsString::from("rollback")],
        ] {
            assert_eq!(parse_command(&args), Err(ERR_USAGE));
        }
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

    #[test]
    fn notification_parser_canonicalizes_values_and_keeps_exact_record_shape() {
        let parsed = parse_command(&[
            OsString::from("notify"),
            OsString::from("set"),
            OsString::from("--group"),
            OsString::from("ops.v1"),
            OsString::from("--hooks"),
            OsString::from("Stop,SessionStart"),
            OsString::from("--channel"),
            OsString::from("both"),
            OsString::from("--content-chars"),
            OsString::from("42"),
            OsString::from("--preserve-newlines"),
            OsString::from("0"),
            OsString::from("--toast-gravity"),
            OsString::from("bottom"),
            OsString::from("--toast-short"),
            OsString::from("1"),
            OsString::from("--toast-background"),
            OsString::from("#A0b1C2"),
            OsString::from("--toast-color"),
            OsString::from("empty"),
        ])
        .unwrap();
        assert_eq!(parsed.kind, CommandKind::NotifySet);
        let patch = parsed.notify_patch.unwrap();
        assert_eq!(patch.channel, Some(NotifyChannel::Both));
        assert_eq!(
            patch.hooks,
            Some(NotifyHooks::Events(vec![
                "SessionStart".to_owned(),
                "Stop".to_owned()
            ]))
        );
        assert_eq!(patch.content_chars, Some(42));
        assert_eq!(patch.preserve_newlines, Some(false));
        assert_eq!(patch.toast_gravity, Some(NotifyGravity::Bottom));
        assert_eq!(patch.toast_short, Some(true));
        assert_eq!(patch.toast_background, Some(Some("#a0b1c2".to_owned())));
        assert_eq!(patch.toast_color, Some(None));
        assert_eq!(patch.group, Some("ops.v1".to_owned()));

        let config = NotifyConfig::defaults().merge(patch);
        let record = notify_record_bytes(&config);
        assert_eq!(
            record,
            b"codex-manager-notify-v1\nchannel\tboth\nhooks\tSessionStart,Stop\ncontent_chars\t42\npreserve_newlines\t0\ntoast_gravity\tbottom\ntoast_short\t1\ntoast_background\t#a0b1c2\ntoast_color\tempty\ngroup\tops.v1\n"
        );
        assert_eq!(parse_notify_record(&record).unwrap(), config);
        assert_eq!(
            format_notify_config(&config),
            "channel=both\nhooks=SessionStart,Stop\ncontent-chars=42\npreserve-newlines=0\ntoast-gravity=bottom\ntoast-short=1\ntoast-background=#a0b1c2\ntoast-color=empty\ngroup=ops.v1\n"
        );
    }

    #[test]
    fn notification_parser_rejects_duplicates_bounds_and_trailing_arguments() {
        for args in [
            vec!["notify", "set", "--hooks", "Stop,Stop"],
            vec!["notify", "set", "--hooks", "Stop,"],
            vec!["notify", "set", "--hooks", "Unknown"],
            vec!["notify", "set", "--content-chars", "4097"],
            vec!["notify", "set", "--preserve-newlines", "2"],
            vec!["notify", "set", "--toast-background", "#12345"],
            vec!["notify", "set", "--group", "-unsafe"],
            vec!["notify", "set", "--group", "safe", "trailing"],
            vec!["notify", "set", "--group", "safe", "--group", "again"],
        ] {
            let owned = args.into_iter().map(OsString::from).collect::<Vec<_>>();
            assert_eq!(parse_command(&owned), Err(ERR_USAGE));
        }
    }

    #[test]
    fn notification_defaults_are_read_only_until_explicit_publication() {
        let root = TestRoot::new();
        let context = root.context();
        assert_eq!(
            read_notify_config(&context).unwrap(),
            NotifyConfig::defaults()
        );
        assert!(!manager_base(&context).exists());

        let config = NotifyConfig::defaults().merge(NotifyPatch {
            hooks: Some(NotifyHooks::None),
            ..NotifyPatch::default()
        });
        publish_notify_config(&context, &config).unwrap();
        let path = root
            .0
            .join(".local/share/codex/manager/notifications/config-v1");
        assert_eq!(fs::read(&path).unwrap(), notify_record_bytes(&config));
        assert_eq!(
            fs::symlink_metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            PRIVATE_DIR_MODE
        );
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o7777,
            PRIVATE_FILE_MODE
        );
        assert_eq!(read_notify_config(&context).unwrap(), config);
    }

    #[test]
    fn notification_record_rejects_wrong_shape_and_symlink_without_replacement() {
        let root = TestRoot::new();
        let context = root.context();
        let directory = notify_directory_for_create(&context).unwrap();
        let path = directory.join(NOTIFY_CONFIG);
        fs::write(&path, b"not-a-record\n").unwrap();
        set_mode(&path, PRIVATE_FILE_MODE).unwrap();
        assert_eq!(read_notify_config(&context), Err(ERR_NOTIFY_CONFIG));
        assert_eq!(fs::read(&path).unwrap(), b"not-a-record\n");

        fs::remove_file(&path).unwrap();
        let outside = root.0.join("notification-outside");
        fs::write(&outside, b"outside-sentinel").unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert_eq!(read_notify_config(&context), Err(ERR_NOTIFY_CONFIG));
        assert_eq!(
            publish_notify_config(&context, &NotifyConfig::defaults()),
            Err(ERR_NOTIFY_CONFIG)
        );
        assert_eq!(fs::read(&outside).unwrap(), b"outside-sentinel");
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn notification_input_accepts_only_bounded_top_level_text_fields() {
        let input = br#"{"message":"fallback","last_assistant_message":"last","content":"line\none","title":"Title","secret":"do-not-forward","nested":{"content":"wrong"}}"#;
        let text = parse_hook_json(input).unwrap();
        assert_eq!(text.title.as_deref(), Some("Title"));
        assert_eq!(text.body.as_deref(), Some("line\none"));
        assert!(parse_hook_json(br#"{"content":"ok"} trailing"#).is_none());
        assert!(parse_hook_json(br#"{"content": [1,]}"#).is_none());
        assert!(parse_hook_json(br#"{"content":"\ud800"}"#).is_none());
        assert_eq!(normalize_notification_text("a\r\nb\rc", false, 0), "a b c");
        assert_eq!(normalize_notification_text("ééé", true, 2), "éé");
        assert_eq!(truncate_utf8("ééé", 5), "éé");
    }

    #[test]
    fn notification_input_and_payload_limits_are_strict() {
        assert!(parse_hook_json(&vec![b' '; NOTIFY_INPUT_MAX_BYTES + 1]).is_none());
        let long = "é".repeat(NOTIFY_PAYLOAD_MAX_BYTES);
        let normalized = normalize_notification_text(&long, true, 0);
        assert!(normalized.len() <= NOTIFY_PAYLOAD_MAX_BYTES);
        assert!(normalized.is_char_boundary(normalized.len()));
        assert_eq!(notify_event_status("Stop"), "Notify turn completion");
    }
}
