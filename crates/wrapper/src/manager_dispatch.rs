//! Optional launch preparation; account/window policy stays in the component.
use super::ManagerArtifact;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Preparation {
    Current,
    Handoff,
    Blocked,
}

fn decode(bytes: &[u8]) -> Option<Preparation> {
    match bytes {
        b"codex-manager-launch-v1\ncurrent\n" => Some(Preparation::Current),
        b"codex-manager-launch-v1\nhandoff\n" => Some(Preparation::Handoff),
        b"codex-manager-launch-v1\nblocked\n" => Some(Preparation::Blocked),
        _ => None,
    }
}

pub(super) fn prepare(
    manager: ManagerArtifact<'_, '_>,
    capability: &str,
    original: &[OsString],
) -> Preparation {
    let ManagerArtifact::Available(selection) = manager else {
        return Preparation::Current;
    };
    let mut args = vec![
        OsStr::new("__prepare-launch-v1"),
        OsStr::new(capability),
        OsStr::new("--"),
    ];
    args.extend(original.iter().map(OsString::as_os_str));
    let result = super::manager_config::exchange(
        Path::new(selection.program_path),
        &args,
        64,
        Duration::from_secs(2),
    )
    .ok()
    .and_then(|bytes| decode(&bytes));
    result.unwrap_or_else(|| {
        eprintln!("codex: optional Manager launch preparation unavailable; keeping the current execution home");
        Preparation::Current
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_exact_prework_decisions_and_never_interprets_policy_records() {
        for (tail, expected) in [
            ("current", Preparation::Current),
            ("handoff", Preparation::Handoff),
            ("blocked", Preparation::Blocked),
        ] {
            assert_eq!(
                decode(format!("codex-manager-launch-v1\n{tail}\n").as_bytes()),
                Some(expected)
            );
        }
        for bytes in [
            &b"codex-manager-launch-v1\ncurrent"[..],
            b"codex-manager-launch-v1\ncurrent\nextra\n",
            b"codex-manager-launch-v2\nhandoff\n",
            b"default-profile-v1\n",
            b"codex-manager-launch-v1\n\xff\n",
        ] {
            assert_eq!(decode(bytes), None);
        }
    }
}
