//! The Core-qualified frontend socket belongs to the local execution account.
use super::AppServerTarget;
use codex_app_server_client::RemoteAppServerEndpoint;

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
