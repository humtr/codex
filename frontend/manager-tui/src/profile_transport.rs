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
