//! Unix execution-profile selection; Manager remains the sole profile-state owner.
use super::App;
use crate::app_event::AppEvent;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::bottom_pane::popup_consts::accept_cancel_hint_line;
use crate::keymap::ListAction;
use crate::keymap::ListKeymap;
use serde::Deserialize;
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Profiles {
    schema: String,
    profiles: Vec<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    current: Option<String>,
    current_source: CurrentSource,
    saved_default: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CurrentSource {
    Default,
    Saved,
    Inherited,
}

fn decode(bytes: &[u8]) -> Option<Profiles> {
    let view: Profiles = serde_json::from_slice(bytes).ok()?;
    let mut unique = HashSet::new();
    if view.schema != "codex-manager-profiles-v1"
        || view.profiles.is_empty()
        || view.profiles.len() > 256
        || view.profiles[0] != "default"
        || !view.profiles.iter().all(|id| {
            !id.is_empty()
                && id.len() <= 64
                && id.as_bytes()[0].is_ascii_alphanumeric()
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                && !matches!(id.as_str(), "home" | "termux")
                && unique.insert(id.as_str())
        })
        || !unique.contains(view.saved_default.as_str())
        || view
            .current
            .as_ref()
            .is_some_and(|id| !unique.contains(id.as_str()))
    {
        return None;
    }
    match view.current_source {
        CurrentSource::Default
            if view.current.as_deref() != Some("default") || view.saved_default != "default" =>
        {
            None
        }
        CurrentSource::Saved if view.current.as_ref() != Some(&view.saved_default) => None,
        _ => Some(view),
    }
}

async fn load(core: &Path) -> Option<Profiles> {
    let metadata = std::fs::symlink_metadata(core).ok()?;
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return None;
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut child = Command::new(core)
            .args(["termux", "__profile-snapshot-v1"])
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
            .take(65537)
            .read_to_end(&mut bytes)
            .await
            .ok()?;
        if bytes.len() > 65536 || !child.wait().await.ok()?.success() {
            return None;
        }
        decode(&bytes)
    })
    .await
    .ok()
    .flatten()
}

fn picker(view: Profiles, keymap: &ListKeymap) -> SelectionViewParams {
    let subtitle = format!(
        "Current: {} · Default: {}",
        view.current.as_deref().unwrap_or("external home"),
        view.saved_default,
    );
    let items = view
        .profiles
        .into_iter()
        .map(|id| {
            let is_current = view.current.as_ref() == Some(&id);
            let selected = id.clone();
            SelectionItem {
                is_current,
                search_value: Some(id.clone()),
                name: if view.saved_default == id && !is_current {
                    format!("{id} (default)")
                } else {
                    id
                },
                actions: if is_current {
                    Vec::new()
                } else {
                    vec![Box::new(move |tx| {
                        tx.send(AppEvent::SelectExecutionProfile(selected.clone()));
                    })]
                },
                // Keep queued input protected until the different profile is validated.
                dismiss_on_select: is_current,
                require_explicit_confirmation: true,
                ..Default::default()
            }
        })
        .collect();
    SelectionViewParams {
        title: Some("Profiles".to_string()),
        subtitle: Some(subtitle),
        items,
        is_searchable: true,
        search_placeholder: Some("Find a profile".to_string()),
        footer_hint: Some(accept_cancel_hint_line(
            keymap.primary_hint(ListAction::Accept),
            "to switch",
            keymap.primary_hint(ListAction::Cancel),
            "to go back",
        )),
        ..SelectionViewParams::picker()
    }
}

pub(super) async fn open(app: &mut App) {
    let core = std::env::var_os("CODEX_PROFILE_CORE");
    let view = match core.as_deref() {
        Some(core) => load(Path::new(core)).await,
        None => None,
    };
    if let Some(view) = view {
        app.chat_widget
            .show_selection_view(picker(view, &app.keymap.list));
    } else {
        app.chat_widget
            .add_error_message("Profiles are unavailable.".to_string());
        app.chat_widget.maybe_send_next_queued_input();
    }
}

/// Private re-entry intent consumed after ordinary client and terminal cleanup.
#[derive(Debug, thiserror::Error)]
#[error("execution profile re-entry")]
pub(crate) struct ProfileReentry {
    core: PathBuf,
    profile: String,
    thread: codex_protocol::ThreadId,
    cwd: PathBuf,
}

impl ProfileReentry {
    pub(crate) fn from_error(error: &std::io::Error) -> Option<&Self> {
        error.get_ref()?.downcast_ref::<Self>()
    }

    fn command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.core);
        command.args([
            "termux",
            "__profile-resume-v1",
            &self.profile,
            &self.thread.to_string(),
        ]);
        command.current_dir(&self.cwd);
        command
    }

    pub(crate) fn exec(&self) -> std::io::Error {
        self.command().exec()
    }
}

pub(super) async fn prepare(
    app: &mut App,
    app_server: &mut crate::app_server_session::AppServerSession,
    profile: String,
) -> Option<ProfileReentry> {
    let thread = app.chat_widget.thread_id().filter(|thread| {
        matches!(
            app.app_server_target,
            crate::AppServerTarget::LocalDaemon { .. }
        ) && app.chat_widget.composer_is_empty()
            && app.dynamic_tool_tasks.is_empty()
            && app.chat_widget.can_change_working_directory(*thread)
    });
    let Some(thread) = thread else {
        app.chat_widget.add_error_message(
            "Profile switching requires an idle primary chat without pending input or tool calls."
                .to_string(),
        );
        return None;
    };
    let core = std::env::var_os("CODEX_PROFILE_CORE")
        .map(PathBuf::from)
        .unwrap_or_default();
    let view = load(&core).await;
    if !view.is_some_and(|view| view.profiles.contains(&profile)) {
        app.chat_widget
            .add_error_message("Selected profile is unavailable.".to_string());
        return None;
    }
    if !matches!(
        tokio::time::timeout(Duration::from_secs(3), ensure_resumable(app_server, thread)).await,
        Ok(true)
    ) {
        app.chat_widget.add_error_message(
            "Current conversation could not be saved for profile switching.".to_string(),
        );
        return None;
    }
    Some(ProfileReentry {
        core,
        profile,
        thread,
        cwd: app.config.cwd.to_path_buf(),
    })
}

/// Only native durability is needed; saved chats do not hydrate their history.
pub(super) async fn ensure_resumable(
    app_server: &mut crate::app_server_session::AppServerSession,
    thread: codex_protocol::ThreadId,
) -> bool {
    let Ok(view) = app_server.thread_read(thread, false).await else {
        return false;
    };
    let Some(path) = view.path else {
        return false;
    };
    if !std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
        // Upstream persists before hydrating turns. The signed backend can reject
        // paginated hydration after successful persistence; verify its durable
        // rollout directly instead of treating history retrieval as the gate.
        if let Err(error) = app_server.thread_read(thread, true).await
            && !matches!(
                error.downcast_ref::<codex_app_server_client::TypedRequestError>(),
                Some(codex_app_server_client::TypedRequestError::Server { source, .. })
                    if source.code == -32601 && source.message == "list_turns is not supported yet"
            )
        {
            return false;
        }
    }
    std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file())
}

#[cfg(test)]
#[path = "profile_manager_tests.rs"]
mod tests;
