//! Idle same-conversation profile re-entry; never stop an existing owner.
use super::*;

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
    // Reuse lifecycle's registry lock so rename/delete cannot invalidate the
    // selected account while native writer cleanup is still in progress.
    let _registry = match &target {
        ProfileTarget::Custom(_) => {
            let dirs = existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?;
            Some(profile::directory_lock(&dirs, libc::LOCK_SH)?)
        }
        ProfileTarget::Default => None,
    };
    // Destination must be valid before even observing the old writer.
    let home = task::destination(context, Some(&target))?;
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
