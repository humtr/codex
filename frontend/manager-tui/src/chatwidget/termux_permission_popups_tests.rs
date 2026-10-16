use super::*;

#[test]
fn termux_permissions_auto_current_marker_matches_disabled_profile_and_approval() {
    let mut auto = codex_utils_approval_presets::builtin_approval_presets()
        .into_iter()
        .find(|p| p.id == "full-access")
        .unwrap();
    auto.id = "auto";
    auto.approval = codex_protocol::protocol::AskForApproval::OnRequest;
    assert!(ChatWidget::preset_matches_current(
        AskForApproval::OnRequest,
        &PermissionProfile::Disabled,
        std::path::Path::new("/owned"),
        &auto
    ));
    assert!(!ChatWidget::preset_matches_current(
        AskForApproval::Never,
        &PermissionProfile::Disabled,
        std::path::Path::new("/owned"),
        &auto
    ));
    assert!(!ChatWidget::preset_matches_current(
        AskForApproval::OnRequest,
        &PermissionProfile::workspace_write(),
        std::path::Path::new("/owned"),
        &auto
    ));
}
