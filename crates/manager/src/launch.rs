//! Profile launch and Android window routing without a terminal multiplexer.
use super::*;
use std::io::IsTerminal;

const ERROR: ManagerError = ManagerError::operation("codex termux: terminal launch failed");
const CANCEL: ManagerError = ManagerError {
    class: ErrorClass::Cancelled,
    message: "codex termux: launch cancelled",
};

fn parse(args: &[OsString]) -> Result<(Option<ProfileTarget>, &[OsString]), ManagerError> {
    let mut index = 0;
    let profile = if args.first().is_some_and(|v| v == "--profile") {
        let target = parse_target(args.get(1).ok_or(ERR_USAGE)?)?;
        index = 2;
        Some(target)
    } else {
        None
    };
    let upstream = if index == args.len() {
        &args[index..]
    } else if args[index] == "--" {
        &args[index + 1..]
    } else {
        return Err(ERR_USAGE);
    };
    if upstream
        .first()
        .is_some_and(|v| matches!(v.to_str(), Some("termux" | "doctor" | "update")))
    {
        return Err(ERR_USAGE);
    }
    Ok((profile, upstream))
}

fn interactive() -> Result<(), ManagerError> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal() {
        Ok(())
    } else {
        Err(ManagerError::operation(
            "codex termux: launch requires an interactive terminal",
        ))
    }
}

struct PickerTerminal(libc::termios);
impl Drop for PickerTerminal {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.0);
        }
        eprint!("\r\x1b[J\x1b[?25h");
        let _ = io::stderr().flush();
    }
}

fn read_key() -> Result<u8, ManagerError> {
    let mut key = 0u8;
    loop {
        let count = unsafe { libc::read(0, (&mut key as *mut u8).cast(), 1) };
        if count == 1 {
            return Ok(key);
        }
        if count < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(if count == 0 { CANCEL } else { ERROR });
    }
}

pub(super) fn choose(context: &Context) -> Result<Option<ProfileTarget>, ManagerError> {
    let snapshot: serde_json::Value =
        serde_json::from_str(&profile_view::snapshot(context)?).map_err(|_| ERR_PROFILE)?;
    let profiles = snapshot["profiles"].as_array().ok_or(ERR_PROFILE)?;
    let current = snapshot["current"].as_str();
    let mut choices = Vec::new();
    if current.is_none() {
        choices.push(("current external home".to_owned(), None));
    }
    for id in profiles {
        let id = id.as_str().ok_or(ERR_PROFILE)?;
        choices.push((id.to_owned(), Some(parse_target(&OsString::from(id))?)));
    }
    let mut selected = choices
        .iter()
        .position(|(label, _)| Some(label.as_str()) == current)
        .unwrap_or(0);
    let mut size = unsafe { std::mem::zeroed::<libc::winsize>() };
    unsafe {
        libc::ioctl(2, libc::TIOCGWINSZ, &mut size);
    }
    let page = usize::from(size.ws_row).saturating_sub(4).max(1);
    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(0, original.as_mut_ptr()) } != 0 {
        return Err(ERROR);
    }
    let original = unsafe { original.assume_init() };
    let mut raw = original;
    unsafe {
        libc::cfmakeraw(&mut raw);
    }
    if unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) } != 0 {
        return Err(ERROR);
    }
    let _guard = PickerTerminal(original);
    eprint!("\x1b[?25l");
    loop {
        eprint!("\r\x1b[JProfile (Up/Down, Enter; Esc cancels)\r\n");
        let start = selected.saturating_sub(page - 1);
        let visible = choices.len().saturating_sub(start).min(page);
        for (index, (label, _)) in choices.iter().enumerate().skip(start).take(visible) {
            eprint!(
                "{} {}\r\n",
                if index == selected { ">" } else { " " },
                label
            );
        }
        eprint!("\x1b[{}A", visible + 1);
        io::stderr().flush().map_err(|_| ERROR)?;
        match read_key()? {
            b'\r' | b'\n' => return Ok(choices.swap_remove(selected).1),
            3 => return Err(CANCEL),
            27 => {
                let mut fd = libc::pollfd {
                    fd: 0,
                    events: libc::POLLIN,
                    revents: 0,
                };
                if unsafe { libc::poll(&mut fd, 1, 80) } <= 0 {
                    return Err(CANCEL);
                }
                let mut suffix = [0u8; 2];
                for byte in &mut suffix {
                    fd.revents = 0;
                    if unsafe { libc::poll(&mut fd, 1, 80) } <= 0 {
                        return Err(CANCEL);
                    }
                    *byte = read_key()?;
                }
                match suffix {
                    [b'[', b'A'] => selected = selected.saturating_sub(1),
                    [b'[', b'B'] => selected = (selected + 1).min(choices.len() - 1),
                    _ => return Err(CANCEL),
                }
            }
            _ => {}
        }
    }
}

pub(super) fn launch(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let (profile, upstream) = parse(args)?;
    interactive()?;
    let profile = match profile {
        Some(profile) => Some(profile),
        None => choose(context)?,
    };
    if let Some(profile) = &profile {
        profile::validate_target(context, profile)?;
    }
    route(context, profile.as_ref(), upstream)
}

pub(super) fn automatic(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    interactive()?;
    route(context, None, args)
}

pub(super) fn client(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    if args.first().is_none_or(|v| v != "off") || args.get(1).is_none_or(|v| v != "--") {
        return Err(ERR_USAGE);
    }
    let program = PathBuf::from(args.get(2).ok_or(ERR_USAGE)?);
    if !is_safe_absolute_path(&program) || !is_existing_executable(&program)? {
        return Err(ERR_USAGE);
    }
    interactive()?;
    if window::inherited_route(context).is_none()
        && origin::capability() == origin::Capability::Stock
    {
        return window::launch(context, None, &args[3..], Some(&program));
    }
    execute_command(&program, &args[3..])
}

fn route(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if window::inherited_route(context).is_none() {
        match origin::capability() {
            origin::Capability::Stock => return window::launch(context, profile, args, None),
            origin::Capability::Unavailable => eprintln!(
                "codex termux: exact window return unavailable; continuing in this terminal"
            ),
            origin::Capability::Native => (),
        }
    }
    execute(context, profile, args)
}

pub(super) fn execute(
    context: &Context,
    profile: Option<&ProfileTarget>,
    upstream: &[OsString],
) -> Result<Option<String>, ManagerError> {
    std::env::set_var(window::READY_ENV, "1");
    if let Some(profile) = profile {
        launch_core(context, profile, upstream)?;
    }
    let _ = Command::new(&context.core_entrypoint)
        .args(upstream)
        .env_remove(CORE_API_ENV)
        .env_remove(CORE_ENTRYPOINT_ENV)
        .exec();
    Err(ERR_LAUNCH)
}

pub(super) fn execute_command(
    program: &Path,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if !is_safe_absolute_path(program) || !is_existing_executable(program)? {
        return Err(ERR_USAGE);
    }
    std::env::set_var(window::READY_ENV, "1");
    let _ = Command::new(program)
        .args(args)
        .env_remove(CORE_API_ENV)
        .env_remove(CORE_ENTRYPOINT_ENV)
        .exec();
    Err(ERR_LAUNCH)
}
