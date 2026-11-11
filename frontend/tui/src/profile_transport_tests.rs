use super::*;
use codex_utils_absolute_path::AbsolutePathBuf;

fn unix() -> RemoteAppServerEndpoint {
    RemoteAppServerEndpoint::UnixSocket {
        socket_path: AbsolutePathBuf::from_absolute_path("/owned/socket").unwrap(),
    }
}

#[test]
fn profile_transport_core_socket_keeps_local_cwd_and_permission_parameters_without_fallback() {
    let target = target(unix(), true);
    assert!(core_owned(&target, true));
    assert!(!core_owned(&target, false));
    assert!(!core_owned(
        &AppServerTarget::LocalDaemon {
            endpoint: unix(),
            allow_embedded_fallback: true,
        },
        true
    ));
    assert!(matches!(
        target,
        AppServerTarget::LocalDaemon {
            allow_embedded_fallback: false,
            ..
        }
    ));
    assert!(!target.uses_remote_workspace());
    assert!(matches!(
        target.thread_params_mode(),
        crate::app_server_session::ThreadParamsMode::Embedded
    ));
}

#[test]
fn profile_transport_unselected_upstream_unix_target_stays_remote() {
    let target = target(unix(), false);
    assert!(!core_owned(&target, true));
    assert!(target.uses_remote_workspace());
    assert!(matches!(target, AppServerTarget::Remote { .. }));
}

#[test]
fn profile_transport_login_uses_selected_local_provider_and_preserves_remote_authority() {
    let local = target(unix(), true);
    let remote = target(unix(), false);
    let fallback = AppServerTarget::LocalDaemon {
        endpoint: unix(),
        allow_embedded_fallback: true,
    };
    for selected in [false, true] {
        for backend in [false, true] {
            assert_eq!(
                startup_requires_openai_auth(&local, true, false, selected, backend),
                selected
            );
            assert!(!startup_requires_openai_auth(
                &local, true, true, selected, backend
            ));
            for (target, termux) in [
                (&local, false),
                (&remote, true),
                (&fallback, true),
                (&AppServerTarget::Embedded, true),
            ] {
                assert_eq!(
                    startup_requires_openai_auth(target, termux, false, selected, backend),
                    backend
                );
            }
        }
    }
}

#[test]
fn profile_transport_websocket_stays_remote_even_with_preview_bridge() {
    let target = target(
        RemoteAppServerEndpoint::WebSocket {
            websocket_url: "ws://localhost:9/".into(),
            auth_token: None,
        },
        true,
    );
    assert!(!core_owned(&target, true));
    assert!(target.uses_remote_workspace());
    assert!(matches!(target, AppServerTarget::Remote { .. }));
}

#[test]
fn profile_transport_core_daemon_actions_refuse_without_affecting_remote_or_read_only_views() {
    use crate::app_event::AppEvent;
    use crate::update_action::DaemonUpdateSource;
    let local = target(unix(), true);
    let remote = target(unix(), false);
    for event in [
        AppEvent::ConfirmDaemonRestart,
        AppEvent::RunDaemonRestart,
        AppEvent::ConfirmDaemonUpdate(DaemonUpdateSource::PublicStable),
        AppEvent::RunDaemonUpdate(DaemonUpdateSource::ThisCli),
    ] {
        assert!(daemon_action_blocked(&local, &event, true));
        assert!(!daemon_action_blocked(&local, &event, false));
        assert!(!daemon_action_blocked(&remote, &event, true));
    }
    assert!(!daemon_action_blocked(
        &local,
        &AppEvent::OpenDaemonMenu,
        true
    ));
    assert!(!daemon_action_blocked(
        &local,
        &AppEvent::OpenWarnings,
        true
    ));
}

#[test]
fn profile_transport_owner_diagnostics_exclude_private_stderr() {
    assert_eq!(
        rejection_reason(b"codex: native_owner_writer_unresolved: details"),
        "Native writer is held but its qualified owner is unavailable"
    );
    assert_eq!(
        rejection_reason(b"codex: native_owner_binding_invalid: details"),
        "live Native execution binding is unsupported or invalid"
    );
    let secret = b"private token=secret conversation content";
    assert!(!rejection_reason(secret).contains("secret"));
    assert!(!rejection_reason(secret).contains("conversation"));
}

#[tokio::test]
async fn profile_transport_core_errors_preserve_absence_rejection_and_limits() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let core = root.path().join("core");
    assert!(matches!(
        call_manager_result(&core, &[], 2).await,
        Err(CoreCallError::Unavailable)
    ));
    // The current test executable cannot act as a Core; executable validation
    // and every child outcome remain distinct from a valid null owner reply.
    std::fs::write(&core, b"unexecutable").unwrap();
    assert!(matches!(
        call_manager_result(&core, &[], 2).await,
        Err(CoreCallError::UnsafeExecutable)
    ));
    let shell = std::env::var_os("PREFIX")
        .map(|prefix| Path::new(&prefix).join("bin/sh"))
        .filter(|path| path.is_absolute() && path.is_file())
        .unwrap_or_else(|| Path::new("/bin/sh").to_path_buf());
    let script = |body: &str| {
        std::fs::write(&core, format!("#!{}\n{body}\n", shell.display())).unwrap();
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700)).unwrap();
    };
    script("printf '%s\n' '{\"schema\":\"codex-manager-task-connection-v1\",\"connection\":null}'");
    let bytes = call_manager_result(&core, &[], 2).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["connection"],
        serde_json::Value::Null
    );
    script(
        "printf '%s\n' 'codex: native_owner_ambiguous: private token must not escape' >&2; exit 7",
    );
    let error = call_manager_result(&core, &[], 2).await.unwrap_err();
    assert!(matches!(
        error,
        CoreCallError::Rejected { code: Some(7), .. }
    ));
    assert!(!error.to_string().contains("token"));
    script(
        "while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done",
    );
    assert!(matches!(
        call_manager_result(&core, &[], 2).await,
        Err(CoreCallError::OutputTooLarge)
    ));
    script("sleep 2");
    assert!(matches!(
        call_manager_result(&core, &[], 0).await,
        Err(CoreCallError::Timeout)
    ));
}
