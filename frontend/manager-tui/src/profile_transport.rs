//! The Core-qualified frontend socket belongs to the local execution account.
use super::AppServerTarget;
use codex_app_server_client::RemoteAppServerEndpoint;

/// Core owns the server lifecycle for its explicit qualified local endpoint.
pub(super) fn core_owned(target: &AppServerTarget, termux: bool) -> bool {
    termux
        && matches!(
            target,
            AppServerTarget::LocalDaemon {
                allow_embedded_fallback: false,
                ..
            }
        )
}

/// A shared backend reports its default account policy; local threads select
/// their own resolved provider. Remote account policy stays server-owned.
pub(super) fn startup_requires_openai_auth(
    target: &AppServerTarget,
    termux: bool,
    selected_provider_requires_auth: bool,
    backend_requires_auth: bool,
) -> bool {
    if core_owned(target, termux) {
        selected_provider_requires_auth
    } else {
        backend_requires_auth
    }
}

/// Native package lifecycle cannot replace a Core-qualified server or generation.
pub(super) fn daemon_action_blocked(
    target: &AppServerTarget,
    event: &crate::app_event::AppEvent,
    termux: bool,
) -> bool {
    core_owned(target, termux)
        && matches!(
            event,
            crate::app_event::AppEvent::ConfirmDaemonRestart
                | crate::app_event::AppEvent::RunDaemonRestart
                | crate::app_event::AppEvent::ConfirmDaemonUpdate(_)
                | crate::app_event::AppEvent::RunDaemonUpdate(_)
        )
}

pub(super) fn target(endpoint: RemoteAppServerEndpoint, termux: bool) -> AppServerTarget {
    if termux && matches!(endpoint, RemoteAppServerEndpoint::UnixSocket { .. }) {
        AppServerTarget::LocalDaemon {
            endpoint,
            allow_embedded_fallback: false,
        }
    } else {
        AppServerTarget::Remote { endpoint }
    }
}

#[cfg(test)]
#[path = "profile_transport_tests.rs"]
mod tests;
