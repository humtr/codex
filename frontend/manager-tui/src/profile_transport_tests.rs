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
    assert!(!daemon_action_blocked(&local, &AppEvent::OpenDaemonMenu, true));
    assert!(!daemon_action_blocked(&local, &AppEvent::OpenWarnings, true));
}
