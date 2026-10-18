//! No-work launch decision, followed by an explicit, revalidated handoff.
use super::*;

const HEADER: &str = "codex-manager-launch-v1\n";

fn request(args: &[OsString]) -> Result<(bool, &[OsString]), ManagerError> {
    let local = match args.first().and_then(|arg| arg.to_str()) {
        Some("local-frontend") => true,
        Some("runtime") => false,
        _ => return Err(ERR_USAGE),
    };
    if args.get(1).is_none_or(|arg| arg != "--") {
        return Err(ERR_USAGE);
    }
    if args
        .get(2)
        .is_some_and(|arg| matches!(arg.to_str(), Some("termux" | "doctor" | "update")))
    {
        return Err(ERR_USAGE);
    }
    Ok((local, &args[2..]))
}

fn selected(context: &Context) -> Result<Option<ProfileTarget>, ManagerError> {
    if context
        .inherited_codex_home
        .as_ref()
        .is_some_and(|home| !home.is_empty())
    {
        return Ok(None);
    }
    profile::read_default(context)
}

pub(super) fn prepare(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    let (local, _) = request(args)?;
    let decision = match selected(context) {
        Err(_) => "blocked",
        Ok(Some(_)) => "handoff",
        Ok(None) if local && std::env::var_os(window::READY_ENV).is_none() => "handoff",
        Ok(None) => "current",
    };
    Ok(Some(format!("{HEADER}{decision}\n")))
}

pub(super) fn execute(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    let (local, upstream) = request(args)?;
    if let Some(target) = selected(context)? {
        launch_core(context, &target, upstream)?;
        return Ok(None);
    }
    if local && std::env::var_os(window::READY_ENV).is_none() {
        return tmux_launch::automatic(context, upstream);
    }
    Err(ERR_USAGE)
}
