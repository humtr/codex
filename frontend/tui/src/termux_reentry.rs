//! Process-local launch intent for an existing Termux frontend re-entry.
//!
//! The executable consumes its environment marker before creating the async
//! runtime. Only the typed value reaches initial startup; children and later
//! resume requests cannot acquire the intent from a lingering environment.

pub const ENV: &str = "CODEX_CHAT_MODEL_REENTRY";
pub const OWNER_PID_ENV: &str = "CODEX_CHAT_MODEL_REENTRY_PID";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Select,
    Resume,
}

impl Intent {
    fn parse(value: Option<&std::ffi::OsStr>) -> Option<Self> {
        match value.and_then(std::ffi::OsStr::to_str) {
            Some("select") => Some(Self::Select),
            Some("resume") => Some(Self::Resume),
            _ => None,
        }
    }
}

/// Consume the private marker before arg0 dispatch or an async runtime starts.
///
/// # Safety
/// The process must still be single-threaded, with no concurrent environment
/// readers. Production callers are the two executable `main` entry points.
pub unsafe fn take_before_threads() -> Option<Intent> {
    let value = std::env::var_os(ENV);
    let owner = std::env::var(OWNER_PID_ENV)
        .ok()
        .and_then(|value| value.parse::<u32>().ok());
    // SAFETY: the caller guarantees the executable is still single-threaded.
    unsafe {
        std::env::remove_var(ENV);
        std::env::remove_var(OWNER_PID_ENV);
    }
    // A legacy/unbound or inherited marker cannot override an ordinary child CLI.
    if owner == Some(std::process::id()) {
        Intent::parse(value.as_deref())
    } else {
        None
    }
}

/// Rebind a consumed intent only for a command that replaces this process with exec.
/// Core/Manager/owner handoffs preserve the PID; ordinary spawned children do not.
pub fn configure_exec(command: &mut std::process::Command, intent: Option<Intent>) {
    command.env_remove(ENV).env_remove(OWNER_PID_ENV);
    if let Some(intent) = intent {
        command.env(
            ENV,
            match intent {
                Intent::Select => "select",
                Intent::Resume => "resume",
            },
        );
        command.env(OWNER_PID_ENV, std::process::id().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termux_chat_models_reentry_intent_accepts_only_known_modes() {
        assert_eq!(Intent::parse(Some("select".as_ref())), Some(Intent::Select));
        assert_eq!(Intent::parse(Some("resume".as_ref())), Some(Intent::Resume));
        for value in [None, Some("".as_ref()), Some("other".as_ref())] {
            assert_eq!(Intent::parse(value), None);
        }
    }
}
