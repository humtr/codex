//! Idle same-conversation profile re-entry; never stop an existing owner.
use super::*;
use std::io::{BufRead, IsTerminal};
use std::os::unix::fs::MetadataExt;

pub(super) fn run(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let [profile, id] = args else {
        return Err(ERR_USAGE);
    };
    let id = id
        .to_str()
        .filter(|id| canonical_session_id(id))
        .ok_or(ERR_USAGE)?;
    if profile == "home" {
        return Err(ERR_USAGE);
    }
    let target = parse_target(profile)?;
    // Capture only the explicit original execution home, never the saved default.
    // Holding the directory open prevents replacement from recycling its identity.
    let source = context.inherited_codex_home.as_ref().and_then(|home| {
        if !(io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal())
        {
            return None;
        }
        let home = PathBuf::from(home);
        task::resume_command(context, &home, id, None).ok()?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(&home)
            .ok()?;
        Some((home, directory))
    });
    match switch(context, &target, id) {
        Err(error) => match source {
            Some((home, directory)) => return_to_source(context, id, &home, &directory, error),
            None => Err(error),
        },
        result => result,
    }
}

fn switch(
    context: &Context,
    target: &ProfileTarget,
    id: &str,
) -> Result<Option<String>, ManagerError> {
    // Reuse lifecycle's registry lock so rename/delete cannot invalidate the
    // selected account while native writer cleanup is still in progress.
    let _registry = match target {
        ProfileTarget::Custom(_) => {
            let dirs = existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?;
            Some(profile::directory_lock(&dirs, libc::LOCK_SH)?)
        }
        ProfileTarget::Default => None,
    };
    // Destination must be valid before even observing the old writer.
    let home = task::destination(context, Some(target))?;
    let deadline = Instant::now() + Duration::from_secs(3);
    while !task::writer_free(context, id)? {
        if Instant::now() >= deadline {
            return Err(ManagerError::operation(
                "codex termux: profile switch refused; conversation writer is still owned; reconnect to its owner or explicitly use task takeover",
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }
    // Native resume provides the final atomic writer acquisition if ownership races.
    let _ = task::resume_command(context, &home, id, None)?.exec();
    Err(ERR_LAUNCH)
}

fn return_to_source(
    context: &Context,
    id: &str,
    home: &Path,
    directory: &File,
    error: ManagerError,
) -> Result<Option<String>, ManagerError> {
    eprintln!("{}", error.message);
    eprint!("Enter to return to the original profile; q or other input exits: ");
    io::stderr().flush().map_err(|_| ERR_LAUNCH)?;
    let mut answer = Vec::new();
    io::stdin()
        .lock()
        .take(128)
        .read_until(b'\n', &mut answer)
        .map_err(|_| ERR_LAUNCH)?;
    // EOF and a partial line are not confirmation.
    if answer != b"\n" && answer != b"\r\n" {
        return Err(ManagerError {
            class: ErrorClass::Cancelled,
            message: "codex termux: profile switch cancelled",
        });
    }
    let mut command = task::resume_command(context, home, id, None)?;
    let current = fs::symlink_metadata(home).map_err(|_| ERR_PROFILE)?;
    let original = directory.metadata().map_err(|_| ERR_PROFILE)?;
    if (current.dev(), current.ino()) != (original.dev(), original.ino()) {
        return Err(ManagerError::operation(
            "codex termux: original profile changed; no return started",
        ));
    }
    let _ = command.exec();
    Err(ERR_LAUNCH)
}
