//! The Core-qualified frontend socket belongs to the local execution account.
use super::AppServerTarget;
use codex_app_server_client::RemoteAppServerEndpoint;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Failure categories contain no raw child stderr, paths, credentials or history.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CoreCallError {
    #[error("qualified Core executable is unavailable")]
    Unavailable,
    #[error("qualified Core executable failed ownership or permission validation")]
    UnsafeExecutable,
    #[error("qualified Core process could not be started")]
    Spawn,
    #[error("qualified Core response could not be read")]
    Read,
    #[error("qualified Core response exceeded its size limit")]
    OutputTooLarge,
    #[error("qualified Core request timed out; existing ownership is unchanged")]
    Timeout,
    #[error("qualified Core rejected the request (exit {code:?}): {reason}")]
    Rejected {
        code: Option<i32>,
        reason: &'static str,
    },
}

fn rejection_reason(stderr: &[u8]) -> &'static str {
    // Only Core-owned bounded categories are public. Never echo arbitrary stderr.
    let text = std::str::from_utf8(stderr).unwrap_or_default();
    let code = text
        .strip_prefix("codex: ")
        .unwrap_or(text)
        .split(':')
        .next()
        .map(str::trim);
    match code {
        Some("native_owner_registry_invalid") => "Native owner registry identity is malformed",
        Some("native_owner_binding_invalid") => {
            "live Native execution binding is unsupported or invalid"
        }
        Some("native_owner_ambiguous") => "multiple Native owners require reconciliation",
        Some("native_owner_writer_unresolved") => {
            "Native writer is held but its qualified owner is unavailable"
        }
        Some("native_owner_unsafe") => "Native owner identity or ownership could not be verified",
        Some("qualified Native owner discovery is unavailable") => {
            "installed Core cannot provide qualified Native owner discovery; check Core/TUI generation compatibility"
        }
        _ if text.trim()
            == "codex: Native owner is unavailable, ambiguous, unsafe or unresponsive" =>
        {
            "installed Core could not verify Native ownership"
        }
        _ => "see the Core diagnostic for this request; no ownership was transferred",
    }
}

async fn bounded_output(
    stream: impl tokio::io::AsyncRead + Unpin,
    limit: u64,
) -> Result<Vec<u8>, CoreCallError> {
    let mut bytes = Vec::new();
    stream
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| CoreCallError::Read)?;
    if bytes.len() as u64 > limit {
        return Err(CoreCallError::OutputTooLarge);
    }
    Ok(bytes)
}

/// Bounded qualified Core IPC; existing callers retain their optional-view API.
pub(crate) async fn call_manager(core: &Path, args: &[String], seconds: u64) -> Option<Vec<u8>> {
    call_manager_result(core, args, seconds).await.ok()
}

pub(crate) async fn call_manager_result(
    core: &Path,
    args: &[String],
    seconds: u64,
) -> Result<Vec<u8>, CoreCallError> {
    let metadata = std::fs::symlink_metadata(core).map_err(|_| CoreCallError::Unavailable)?;
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return Err(CoreCallError::UnsafeExecutable);
    }
    tokio::time::timeout(Duration::from_secs(seconds), async {
        let mut child = Command::new(core)
            .arg("termux")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| CoreCallError::Spawn)?;
        let stdout = child.stdout.take().ok_or(CoreCallError::Read)?;
        let stderr = child.stderr.take().ok_or(CoreCallError::Read)?;
        let (bytes, errors) =
            tokio::try_join!(bounded_output(stdout, 262144), bounded_output(stderr, 4096))?;
        let status = child.wait().await.map_err(|_| CoreCallError::Read)?;
        if !status.success() {
            return Err(CoreCallError::Rejected {
                code: status.code(),
                reason: rejection_reason(&errors),
            });
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| CoreCallError::Timeout)?
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
