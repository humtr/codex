//! Manager-owned upstream hook/configuration preparation; never launches work.
use super::*;

const HEADER: &str = "codex-manager-runtime-config-v1\n";

pub(super) fn prepare(
    context: &Context,
    args: &[OsString],
) -> Result<Option<String>, ManagerError> {
    if args.len() != 1 || !is_safe_absolute_path(Path::new(&args[0])) {
        return Err(ERR_USAGE);
    }
    let config = Path::new(&args[0]);
    if !is_existing_directory(config)? {
        return Err(ERR_PATH);
    }
    let home = context
        .inherited_codex_home
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| context.home.join(".codex"));
    // Same no-replace optional projection as before, now owned by Manager.
    if is_existing_directory(&home).unwrap_or(false) {
        let _ = crate::shared_profiles::project(config, &home);
    }
    let events = match existing_notify_directory(context)? {
        None => Vec::new(),
        Some(directory) => match fs::symlink_metadata(directory.join(NOTIFY_CONFIG)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(_) => return Err(ERR_NOTIFY_CONFIG),
            Ok(_) => match read_notify_config(context)?.hooks {
                NotifyHooks::None => Vec::new(),
                NotifyHooks::All => NOTIFY_EVENTS.iter().map(|s| (*s).to_owned()).collect(),
                NotifyHooks::Events(events) => events,
            },
        },
    };
    let mut result = String::from(HEADER);
    result.push_str(&render(
        &events.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    Ok(Some(result))
}

fn notify_status_message(event: &str) -> &'static str {
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
        "UserInputRequest" => "Codex needs your input",
        _ => "Notify Codex event",
    }
}

fn render(events: &[&str]) -> String {
    let mut output = String::new();
    if events.contains(&"Stop") {
        // Upstream clears native notify for internal memory consolidation.
        output.push_str("notify = [\"codex\", \"termux\", \"notify\", \"emit\", \"Stop\"]\n\n");
    }
    for event in events {
        if *event == "Stop" || (*event == "UserInputRequest" && events.contains(&"PreToolUse")) {
            continue;
        }
        let hook = if *event == "UserInputRequest" {
            "PreToolUse"
        } else {
            event
        };
        output.push_str(&format!("[[hooks.{hook}]]\n"));
        if *event == "UserInputRequest" {
            output.push_str("matcher = \"^(functions\\\\.)?request_user_input(_async)?$\"\n");
        }
        output.push_str(&format!(
            "\n[[hooks.{hook}.hooks]]\ntype = \"command\"\ncommand = \"codex termux notify emit {event}\"\ntimeout = 15\nstatusMessage = \"{}\"\n\n",
            notify_status_message(event)
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_question_matcher_and_completion_callback_preserve_native_policy() {
        let hooks = parse_notify_hooks(
            OsStr::new("Stop,UserInputRequest,PermissionRequest"),
            ERR_NOTIFY_CONFIG,
        )
        .unwrap();
        let NotifyHooks::Events(events) = hooks else {
            panic!("expected events")
        };
        let config = render(&events.iter().map(String::as_str).collect::<Vec<_>>());
        assert!(config.contains(
            "[[hooks.PreToolUse]]\nmatcher = \"^(functions\\\\.)?request_user_input(_async)?$\"\n"
        ));
        assert!(config.contains("command = \"codex termux notify emit UserInputRequest\""));
        assert!(config.contains("[[hooks.PreToolUse.hooks]]"));
        assert!(!config.contains("[[hooks.UserInputRequest"));
        assert_eq!(config.matches("timeout = 15").count(), 2);
        assert!(config.contains("notify = [\"codex\", \"termux\", \"notify\", \"emit\", \"Stop\"]"));
        assert!(!config.contains("hooks.Stop"));
        for events in [
            vec!["PreToolUse", "UserInputRequest"],
            NOTIFY_EVENTS.to_vec(),
        ] {
            let config = render(&events);
            assert_eq!(config.matches("[[hooks.PreToolUse]]").count(), 1);
            assert!(!config.contains("emit UserInputRequest"));
        }
        assert!(render(&[]).is_empty());
        assert!(!render(&NOTIFY_EVENTS).contains("sandbox_mode"));
    }
}
