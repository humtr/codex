use super::*;

#[test]
fn termux_permissions_resume_defaults_override_only_saved_sandbox_selection() {
    let requested = crate::resume_permissions::ResumePermissions::default();
    let resolved = resume_permissions(&PermissionProfile::Disabled, requested, true);
    assert!(resolved.profile);
    assert!(!resolved.approval_policy);
    assert!(!resolved.approvals_reviewer);
    assert!(!resolved.workspace_roots);
    assert_eq!(
        resume_permissions(&PermissionProfile::Disabled, requested, false),
        requested
    );
    assert_eq!(
        resume_permissions(&PermissionProfile::workspace_write(), requested, true),
        requested
    );
    assert_eq!(
        resume_permissions(&PermissionProfile::read_only(), requested, true),
        requested
    );
}

#[test]
fn termux_permissions_reject_restricted_requests_without_weakening_or_changing_linux() {
    for profile in [
        PermissionProfile::read_only(),
        PermissionProfile::workspace_write(),
    ] {
        let before = profile.clone();
        let error = validate(&profile, true).unwrap_err().to_string();
        assert!(error.contains("Termux cannot enforce"));
        assert_eq!(profile, before);
        assert!(validate(&profile, false).is_ok());
    }
    assert!(validate(&PermissionProfile::Disabled, true).is_ok());
}

#[test]
fn termux_permissions_require_an_explicit_absolute_bridge() {
    assert!(!has_bridge(None));
    assert!(!has_bridge(Some(OsStr::new(""))));
    assert!(!has_bridge(Some(OsStr::new("relative"))));
    assert!(has_bridge(Some(OsStr::new("/owned/core"))));
}

#[test]
fn termux_permissions_auto_preserves_reviewer_policy_and_disables_sandbox() {
    let presets = adapt(
        codex_utils_approval_presets::builtin_approval_presets(),
        true,
    );
    let auto = presets.iter().find(|preset| preset.id == "auto").unwrap();
    assert_eq!(auto.active_permission_profile.id, auto_profile_id(true));
    assert_eq!(auto.permission_profile, PermissionProfile::Disabled);
    assert_eq!(
        auto.approval,
        codex_protocol::protocol::AskForApproval::OnRequest
    );
    assert!(auto.description.contains("without a sandbox"));
    let full = presets
        .iter()
        .find(|preset| preset.id == "full-access")
        .unwrap();
    assert_eq!(full.permission_profile, PermissionProfile::Disabled);
    assert_eq!(
        full.approval,
        codex_protocol::protocol::AskForApproval::Never
    );
    assert!(review_description(true).contains("without a sandbox"));
}

#[test]
fn termux_permissions_unselected_upstream_and_custom_presets_are_preserved() {
    let original = codex_utils_approval_presets::builtin_approval_presets();
    let unchanged = adapt(original.clone(), false);
    for (before, after) in original.iter().zip(&unchanged) {
        assert_eq!(before.id, after.id);
        assert_eq!(
            before.active_permission_profile,
            after.active_permission_profile
        );
        assert_eq!(before.permission_profile, after.permission_profile);
        assert_eq!(before.approval, after.approval);
        assert_eq!(before.description, after.description);
    }
    assert_eq!(
        auto_profile_id(false),
        BUILT_IN_PERMISSION_PROFILE_WORKSPACE
    );
    assert_eq!(
        review_description(false),
        "Only ask for actions detected as potentially unsafe"
    );
    let mut custom = original[1].clone();
    custom.id = "custom";
    let adapted = adapt(vec![custom.clone()], true);
    assert_eq!(adapted[0].permission_profile, custom.permission_profile);
    assert_eq!(
        adapted[0].active_permission_profile,
        custom.active_permission_profile
    );
    assert_eq!(adapted[0].description, custom.description);
}
