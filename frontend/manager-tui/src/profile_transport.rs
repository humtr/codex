//! The Core-qualified frontend socket belongs to the local execution account.
use super::AppServerTarget;
use codex_app_server_client::RemoteAppServerEndpoint;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Bounded same-generation Manager IPC; Native retains execution authority.
pub(crate) async fn call_manager(core: &Path, args: &[String], seconds: u64) -> Option<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(core).ok()?;
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return None;
    }
    tokio::time::timeout(Duration::from_secs(seconds), async {
        let mut child = Command::new(core)
            .arg("termux")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .ok()?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()?
            .take(262145)
            .read_to_end(&mut bytes)
            .await
            .ok()?;
        if bytes.len() > 262144 || !child.wait().await.ok()?.success() {
            return None;
        }
        Some(bytes)
    })
    .await
    .ok()
    .flatten()
}

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

/// New local work uses its resolved provider. A verified existing Native work
/// owns its authentication; attachment does not require another local login.
pub(super) fn startup_requires_openai_auth(
    target: &AppServerTarget,
    termux: bool,
    has_live_owner: bool,
    selected_provider_requires_auth: bool,
    backend_requires_auth: bool,
) -> bool {
    if core_owned(target, termux) {
        !has_live_owner && selected_provider_requires_auth
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
