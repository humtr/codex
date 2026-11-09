//! Permission presentation for the explicit Termux frontend experiment.
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use codex_protocol::models::PermissionProfile;
use codex_protocol::permissions::FileSystemSandboxKind;
use codex_utils_approval_presets::ApprovalPreset;
use std::ffi::OsStr;
use std::path::Path;

fn has_bridge(core: Option<&OsStr>) -> bool {
    core.is_some_and(|core| Path::new(core).is_absolute())
}

pub(crate) fn enabled() -> bool {
    has_bridge(std::env::var_os("CODEX_PROFILE_CORE").as_deref())
}

fn supported(profile: &PermissionProfile) -> bool {
    let file_system = profile.file_system_sandbox_policy();
    let external = file_system.kind == FileSystemSandboxKind::ExternalSandbox;
    external
        || profile.network_sandbox_policy().is_enabled() && file_system.has_full_disk_write_access()
}

pub(crate) fn validate(profile: &PermissionProfile, termux: bool) -> color_eyre::Result<()> {
    static REENTRY_VALIDATED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if REENTRY_VALIDATED.get().is_none()
        && let Ok(expected) = std::env::var("CODEX_PROFILE_PERMISSION_SNAPSHOT")
    {
        check_reentry(profile, &expected, std::process::id())?;
        // This guards initial re-entry, not later explicit /permissions changes.
        let _ = REENTRY_VALIDATED.set(());
    }
    if termux && !supported(profile) {
        color_eyre::eyre::bail!(
            "Termux cannot enforce Linux filesystem/network sandbox permissions. Select Ask for approval, Approve for me or Full Access in /permissions; no sandbox policy has been weakened."
        );
    }
    Ok(())
}

// An account can define the same permission name differently. Compare native
// resolved permissions before any continuation; never copy account config or
// silently weaken/replace the destination's managed requirements.
fn check_reentry(profile: &PermissionProfile, expected: &str, pid: u32) -> color_eyre::Result<()> {
    if expected.len() > 65536 {
        color_eyre::eyre::bail!(
            "Account switch paused: permission snapshot exceeds its bound; work remains saved."
        );
    }
    let value: serde_json::Value = serde_json::from_str(expected)?;
    if value["pid"].as_u64() == Some(pid as u64)
        && value["profile"] != serde_json::to_value(profile)?
    {
        color_eyre::eyre::bail!(
            "Account switch paused: destination permissions differ from the saved source selection; align the account permission definitions before resuming. Work remains saved."
        );
    }
    Ok(())
}

pub(crate) fn resume_permissions(
    profile: &PermissionProfile,
    mut requested: crate::resume_permissions::ResumePermissions,
    termux: bool,
) -> crate::resume_permissions::ResumePermissions {
    // The qualified local launch selection wins over obsolete saved Linux
    // sandbox settings. Approval policy/reviewer remain independently restored.
    if termux && supported(profile) {
        requested.profile = true;
    }
    requested
}

pub(crate) fn auto_profile_id(termux: bool) -> &'static str {
    if termux {
        BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS
    } else {
        BUILT_IN_PERMISSION_PROFILE_WORKSPACE
    }
}

pub(crate) fn review_description(termux: bool) -> &'static str {
    if termux {
        "Auto-review approval requests without a sandbox."
    } else {
        "Only ask for actions detected as potentially unsafe"
    }
}

fn adapt(mut presets: Vec<ApprovalPreset>, termux: bool) -> Vec<ApprovalPreset> {
    if termux {
        for preset in &mut presets {
            if preset.id == "auto" {
                preset.active_permission_profile =
                    ActivePermissionProfile::new(auto_profile_id(true));
                preset.permission_profile = PermissionProfile::Disabled;
                preset.description = "Run without a sandbox; the selected reviewer handles approval requests required by command rules and policy.";
            }
        }
    }
    presets
}

pub(crate) fn builtin_approval_presets() -> Vec<ApprovalPreset> {
    adapt(
        codex_utils_approval_presets::builtin_approval_presets(),
        enabled(),
    )
}

#[cfg(test)]
#[path = "termux_permissions_tests.rs"]
mod tests;
