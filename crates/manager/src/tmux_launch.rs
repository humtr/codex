//! Direct caller-terminal tmux launch. No Android window constructor or AI dependency.
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

fn workload(context: &Context, profile: Option<&ProfileTarget>, args: &[OsString]) -> OsString {
    let mut words = vec![
        OsString::from("exec"),
        OsString::from("env"),
        OsString::from("-u"),
        OsString::from(CORE_API_ENV),
        OsString::from("-u"),
        OsString::from(CORE_ENTRYPOINT_ENV),
    ];
    let mut assignments = Vec::new();
    for key in [
        "HOME",
        "PATH",
        "PREFIX",
        CODEX_HOME_ENV,
        "NO_COLOR",
        "FORCE_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "COLORTERM",
        "COLORFGBG",
    ] {
        if let Some(value) = std::env::var_os(key) {
            let mut assignment = OsString::from(format!("{key}="));
            assignment.push(value);
            assignments.push(assignment);
        } else {
            words.push("-u".into());
            words.push(key.into());
        }
    }
    words.extend(assignments);
    words.push(context.core_entrypoint.as_os_str().to_owned());
    if let Some(profile) = profile {
        words.extend([
            "termux".into(),
            "profile".into(),
            "use".into(),
            profile::target_name(profile).into(),
            "--".into(),
        ]);
    }
    words.extend_from_slice(args);
    let mut result = OsString::new();
    for (index, word) in words.iter().enumerate() {
        if index != 0 {
            result.push(" ");
        }
        result.push(quote(word));
    }
    result
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

fn choose(context: &Context) -> Result<Option<ProfileTarget>, ManagerError> {
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
    let (profile, upstream) = parse(args)?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() || !io::stderr().is_terminal() {
        return Err(ManagerError::operation(
            "codex termux: tmux requires an interactive terminal",
        ));
    }
    let profile = match profile {
        Some(profile) => Some(profile),
        None => choose(context)?,
    };
    if let Some(profile) = profile.as_ref() {
        profile::validate_target(context, profile)?;
    }
    let command = workload(context, profile.as_ref(), upstream);
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
        return Ok(None);
    }
    let origin = origin::resolve();
    let session = control(&[
        "new-session".into(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{session_id}".into(),
        "-c".into(),
        cwd.as_os_str().to_owned(),
        command,
    ])?;
    let setup = (|| {
        if let Some(origin) = &origin {
            control(&[
                "set-option".into(),
                "-t".into(),
                session.clone().into(),
                origin::OPTION.into(),
                origin.record().into(),
            ])?;
        }
        for (name, value) in [
            ("status", "off"),
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
        let _ = control(&["kill-session".into(), "-t".into(), session.into()]);
        return Err(error);
    }
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
