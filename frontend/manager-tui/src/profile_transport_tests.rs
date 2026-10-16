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
    assert!(target.uses_remote_workspace());
    assert!(matches!(target, AppServerTarget::Remote { .. }));
}
