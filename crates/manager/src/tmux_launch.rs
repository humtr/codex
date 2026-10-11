//! Shared profile launch; tmux is an optional execution container.
use super::*;
use std::io::IsTerminal;
use std::os::unix::ffi::OsStringExt;

const ERROR: ManagerError = ManagerError::operation("codex termux: tmux launch failed");
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

fn quote(value: &OsStr) -> OsString {
    let mut result = vec![b'\''];
    for byte in value.as_bytes() {
        if *byte == b'\'' {
            result.extend_from_slice(b"'\\''");
        } else {
            result.push(*byte);
        }
    }
    result.push(b'\'');
    OsString::from_vec(result)
}

fn workload(
    context: &Context,
    profile: Option<&ProfileTarget>,
    args: &[OsString],
    program: Option<&Path>,
) -> Result<(OsString, PathBuf), ManagerError> {
    let (token, path) = window::execution(context, profile, args, program)?;
    let mut words = vec![
        OsString::from("exec"),
        OsString::from("env"),
        "-u".into(),
        CORE_API_ENV.into(),
        "-u".into(),
        CORE_ENTRYPOINT_ENV.into(),
    ];
    for key in ["HOME", "PATH", "PREFIX", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            let mut assignment = OsString::from(format!("{key}="));
            assignment.push(value);
            words.push(assignment);
        }
    }
    words.extend([
        context.core_entrypoint.as_os_str().to_owned(),
        "termux".into(),
        "__terminal-exec-v1".into(),
        token.into(),
    ]);
    let mut result = OsString::new();
    for (index, word) in words.iter().enumerate() {
        if index != 0 {
            result.push(" ");
        }
        result.push(quote(word));
    }
    Ok((result, path))
}

fn control(args: &[OsString]) -> Result<String, ManagerError> {
    let mut child = Command::new("tmux")
        .arg("-u")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ERROR)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|_| ERROR)?;
                if !status.success() || output.stdout.len() > 4096 {
                    return Err(ERROR);
                }
                return String::from_utf8(output.stdout)
                    .map(|s| s.trim().to_owned())
                    .map_err(|_| ERROR);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ERROR);
            }
        }
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

pub(super) fn run(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    launch(context, args, "hidden")
}
pub(super) fn launch(
    context: &Context,
    args: &[OsString],
    default_mode: &str,
) -> Result<Option<String>, ManagerError> {
    let mut filtered = Vec::new();
    let mut mode = default_mode;
    let mut chosen_mode = false;
    let mut upstream = false;
    for arg in args {
        if arg == "--" {
            upstream = true;
        }
        if !upstream && arg.to_str().is_some_and(|v| v.starts_with("--tmux=")) {
            if chosen_mode {
                return Err(ERR_USAGE);
            }
            mode = arg.to_str().unwrap().strip_prefix("--tmux=").unwrap();
            if !matches!(mode, "off" | "hidden" | "status") {
                return Err(ERR_USAGE);
            }
            chosen_mode = true;
        } else {
            filtered.push(arg.clone());
        }
    }
    let (profile, upstream) = parse(&filtered)?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() || !io::stderr().is_terminal() {
        return Err(ManagerError::operation(
            "codex termux: launch requires an interactive terminal",
        ));
    }
    let profile = match profile {
        Some(profile) => Some(profile),
        None => choose(context)?,
    };
    if let Some(profile) = &profile {
        profile::validate_target(context, profile)?;
    }
    route(context, profile.as_ref(), mode, upstream)
}
pub(super) fn attach(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    interactive()?;
    attach_target(args)?;
    route(context, None, "attach", args)
}
fn attach_target(args: &[OsString]) -> Result<String, ManagerError> {
    let [session] = args else {
        return Err(ERR_USAGE);
    };
    let session = session
        .to_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 64
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'$' | b'_' | b'-'))
        })
        .ok_or(ERR_USAGE)?;
    if std::env::var_os("TMUX").is_some_and(|v| !v.is_empty()) {
        return Err(ERR_USAGE);
    }
    let row = control(&[
        "display-message".into(),
        "-p".into(),
        "-t".into(),
        session.into(),
        "#{session_id}:#{session_attached}:#{@codex_manager_session_v1}".into(),
    ])?;
    let fields: Vec<_> = row.split(':').collect();
    if fields.len() != 3 || fields[1] != "0" || fields[2] != "1" {
        return Err(ERROR);
    }
    Ok(fields[0].to_owned())
}
fn bind_session(context: &Context, session: &str) -> Result<(), ManagerError> {
    let route = window::inherited_route(context);
    let origin = if route.is_none() {
        origin::resolve()
    } else {
        None
    };
    for (option, value) in [
        (
            terminal::ROUTE_OPTION,
            route.map(|v| v.to_string()).unwrap_or_default(),
        ),
        (
            origin::OPTION,
            origin.map(|v| v.record()).unwrap_or_default(),
        ),
    ] {
        control(&[
            "set-option".into(),
            "-t".into(),
            session.into(),
            option.into(),
            value.into(),
        ])?;
    }
    Ok(())
}

pub(super) fn automatic(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    interactive()?;
    route(context, None, "off", args)
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
pub(super) fn client(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let mode = args
        .first()
        .and_then(|v| v.to_str())
        .filter(|v| matches!(*v, "off" | "hidden" | "status"))
        .ok_or(ERR_USAGE)?;
    if args.get(1).is_none_or(|v| v != "--") {
        return Err(ERR_USAGE);
    }
    let program = PathBuf::from(args.get(2).ok_or(ERR_USAGE)?);
    if !is_safe_absolute_path(&program) || !is_existing_executable(&program)? {
        return Err(ERR_USAGE);
    }
    interactive()?;
    if std::env::var_os("TMUX").is_none_or(|v| v.is_empty())
        && window::inherited_route(context).is_none()
        && origin::capability() == origin::Capability::Stock
    {
        return window::launch(context, None, mode, &args[3..], Some(&program));
    }
    execute_command(context, &program, mode, &args[3..])
}
pub(super) fn execute_command(
    context: &Context,
    program: &Path,
    mode: &str,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if !is_safe_absolute_path(program) || !is_existing_executable(program)? {
        return Err(ERR_USAGE);
    }
    execute_work(context, None, mode, args, Some(program))
}

fn route(
    context: &Context,
    profile: Option<&ProfileTarget>,
    mode: &str,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if std::env::var_os("TMUX").is_none_or(|v| v.is_empty())
        && window::inherited_route(context).is_none()
    {
        match origin::capability() {
            origin::Capability::Stock => return window::launch(context, profile, mode, args, None),
            origin::Capability::Unavailable => eprintln!(
                "codex termux: exact window return unavailable; continuing in this terminal"
            ),
            origin::Capability::Native => (),
        }
    }
    execute(context, profile, mode, args)
}
pub(super) fn execute(
    context: &Context,
    profile: Option<&ProfileTarget>,
    mode: &str,
    upstream: &[OsString],
) -> Result<Option<String>, ManagerError> {
    execute_work(context, profile, mode, upstream, None)
}
fn execute_work(
    context: &Context,
    profile: Option<&ProfileTarget>,
    mode: &str,
    upstream: &[OsString],
    program: Option<&Path>,
) -> Result<Option<String>, ManagerError> {
    std::env::set_var(window::READY_ENV, "1");
    if mode == "attach" {
        let session = attach_target(upstream)?;
        bind_session(context, &session)?;
        let _ = Command::new("tmux")
            .args(["attach-session", "-t", &session])
            .exec();
        return Err(ERROR);
    }
    if mode == "off" {
        if program.is_none() {
            if let Some(profile) = profile {
                launch_core(context, profile, upstream)?;
            }
        }
        let _ = Command::new(program.unwrap_or(&context.core_entrypoint))
            .args(upstream)
            .env_remove(CORE_API_ENV)
            .env_remove(CORE_ENTRYPOINT_ENV)
            .exec();
        return Err(ERR_LAUNCH);
    }
    let (command, request) = workload(context, profile, upstream, program)?;
    let cwd = std::env::current_dir().map_err(|_| ERROR)?;
    if std::env::var_os("TMUX").is_some_and(|v| !v.is_empty()) {
        let pane = std::env::var_os("TMUX_PANE").ok_or(ERROR)?;
        if !pane
            .as_bytes()
            .strip_prefix(b"%")
            .is_some_and(|v| !v.is_empty() && v.iter().all(u8::is_ascii_digit))
        {
            return Err(ERROR);
        }
        let session = control(&[
            "display-message".into(),
            "-p".into(),
            "-t".into(),
            pane,
            "#{session_id}".into(),
        ])?;
        let inherited_origin = control(&[
            "show-options".into(),
            "-qv".into(),
            "-t".into(),
            session.clone().into(),
            origin::OPTION.into(),
        ])?;
        if let Some(origin) = origin::Origin::parse(&inherited_origin) {
            if !origin::validate(&origin) {
                return Err(ERROR);
            }
        }
        let window = control(&[
            "new-window".into(),
            "-d".into(),
            "-P".into(),
            "-F".into(),
            "#{pane_id}".into(),
            "-t".into(),
            session.into(),
            "-c".into(),
            cwd.as_os_str().to_owned(),
            command,
        ])?;
        control(&["select-window".into(), "-t".into(), window.into()])?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while request.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        if request.exists() {
            let _ = fs::remove_file(&request);
            return Err(ERROR);
        }
        return Ok(None);
    }
    let ready = format!(
        "codex-launch-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut held = OsString::from("tmux -u wait-for ");
    held.push(quote(OsStr::new(&ready)));
    held.push(" && ");
    held.push(command);
    let session = control(&[
        "new-session".into(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{session_id}".into(),
        "-c".into(),
        cwd.as_os_str().to_owned(),
        held,
    ])?;
    let setup = (|| {
        bind_session(context, &session)?;
        control(&[
            "set-option".into(),
            "-t".into(),
            session.clone().into(),
            "@codex_manager_session_v1".into(),
            "1".into(),
        ])?;
        for (name, value) in [
            ("status", if mode == "status" { "on" } else { "off" }),
            ("set-titles", "on"),
            ("set-titles-string", "#{pane_title}"),
        ] {
            control(&[
                "set-option".into(),
                "-t".into(),
                session.clone().into(),
                name.into(),
                value.into(),
            ])?;
        }
        Ok::<_, ManagerError>(())
    })();
    if let Err(error) = setup {
        let _ = fs::remove_file(&request);
        let _ = control(&["kill-session".into(), "-t".into(), session.into()]);
        return Err(error);
    }
    control(&["wait-for".into(), "-S".into(), ready.into()])?;
    let _ = Command::new("tmux")
        .args(["attach-session", "-t", &session])
        .exec();
    Err(ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tmux_profile_and_native_args_have_separate_grammars() {
        let args: Vec<_> = [
            "--profile",
            "work",
            "--",
            "resume",
            "space arg",
            "--profile",
            "native",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        let (target, upstream) = parse(&args).unwrap();
        assert_eq!(target, Some(ProfileTarget::Custom("work".into())));
        assert_eq!(upstream, &args[3..]);
        for args in [
            vec!["--profile"],
            vec!["work"],
            vec!["--", "termux"],
            vec!["--", "update"],
            vec!["--profile", "../bad"],
        ] {
            assert_eq!(
                parse(&args.into_iter().map(OsString::from).collect::<Vec<_>>()).unwrap_err(),
                ERR_USAGE
            );
        }
    }
    #[test]
    fn tmux_shell_words_preserve_bytes_without_evaluation() {
        assert_eq!(
            quote(OsStr::new("a'b $(false)")),
            OsString::from("'a'\\''b $(false)'")
        );
        assert_eq!(
            quote(&OsString::from_vec(vec![0xff, b'\''])),
            OsString::from_vec(vec![b'\'', 0xff, b'\'', b'\\', b'\'', b'\'', b'\''])
        );
    }
}
