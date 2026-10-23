//! Re-enter the same durable thread because upstream provider updates are launch-scoped.
use super::App;
use crate::app_server_session::AppServerSession;
use crate::termux_chat_models;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort;
use serde_json::Value;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Default)]
pub(crate) struct State(Option<Pending>);
struct Pending {
    thread: ThreadId,
    model: String,
    effort: ReasoningEffort,
    source_model: String,
    turn: Option<String>,
    compacted: bool,
    ready: bool,
}
pub(super) fn cancel(app: &mut App) {
    if app.chat_model_switch.0.take().is_some() {
        app.chat_widget.add_error_message("Model provider unchanged: context preparation was interrupted or could not be confirmed.".into());
    }
}
pub(super) fn observe(app: &mut App, event: &codex_app_server_protocol::ServerNotification) {
    let Ok(value) = serde_json::to_value(event) else {
        return;
    };
    let Some(pending) = app.chat_model_switch.0.as_mut() else {
        return;
    };
    let params = &value["params"];
    if params["threadId"] != pending.thread.to_string() {
        return;
    }
    match value["method"].as_str() {
        Some("turn/started") if pending.turn.is_none() => {
            pending.turn = params["turn"]["id"].as_str().map(str::to_owned)
        }
        Some("item/completed")
            if params["turnId"].as_str() == pending.turn.as_deref()
                && params["item"]["type"] == "contextCompaction" =>
        {
            pending.compacted = true
        }
        Some("error") => cancel(app),
        Some("turn/completed") if params["turn"]["id"].as_str() == pending.turn.as_deref() => {
            if params["turn"]["status"] != "completed" || !pending.compacted {
                cancel(app);
                return;
            }
            if !pending.ready {
                pending.ready = true;
                app.app_event_tx
                    .send(crate::app_event::AppEvent::FinishChatModelProvider {
                        thread: pending.thread,
                        turn: pending.turn.clone().unwrap(),
                    });
            }
        }
        _ => (),
    }
}
pub(super) async fn finish(
    app: &mut App,
    server: &mut AppServerSession,
    thread: ThreadId,
    turn: String,
) -> Option<ChatModelReentry> {
    let pending = app.chat_model_switch.0.as_ref()?;
    if pending.thread != thread || pending.turn.as_deref() != Some(&turn) || !pending.ready {
        return None;
    }
    let pending = app.chat_model_switch.0.take()?;
    if app.chat_widget.thread_id() != Some(thread)
        || app.chat_widget.current_model() != pending.source_model
    {
        app.chat_widget.add_error_message("Model provider unchanged: the displayed thread or source model changed during context preparation.".into());
        return None;
    }
    plan(app, server, pending.model, Some(pending.effort), false).await
}

async fn encode_checkpoint(
    config: &crate::legacy_core::config::Config,
    snapshot: &Value,
    thread: ThreadId,
    model: &str,
    effort: &ReasoningEffort,
) -> Option<codex_protocol::models::ResponseItem> {
    let routing = termux_chat_models::installed_routing(config)?;
    let binary = std::path::PathBuf::from(
        routing
            .get("model_providers")?
            .get(termux_chat_models::PROVIDER)?
            .get("auth")?
            .get("command")?
            .as_str()?,
    );
    let metadata = std::fs::symlink_metadata(&binary).ok()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return None;
    }
    let bytes = serde_json::to_vec(snapshot).ok()?;
    const LIMIT: u64 = 32 * 1024 * 1024;
    if bytes.len() as u64 > LIMIT {
        return None;
    }
    let mut child = tokio::process::Command::new(binary)
        .args([
            "checkpoint",
            "--thread",
            &thread.to_string(),
            "--from-model",
            config.model.as_deref()?,
            "--model",
            model,
            "--effort",
            effort.as_str(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let mut stdout = stdout.take(LIMIT + 1);
    let mut output = Vec::new();
    let execution = async {
        let (_, _, status) = tokio::try_join!(
            async move {
                stdin.write_all(&bytes).await?;
                drop(stdin); // EOF is required by the Provider's bounded stdin reader.
                Ok::<_, std::io::Error>(())
            },
            stdout.read_to_end(&mut output),
            child.wait()
        )?;
        Ok::<_, std::io::Error>(status)
    };
    let status = tokio::time::timeout(Duration::from_secs(15), execution)
        .await
        .ok()?
        .ok()?;
    if !status.success() || output.len() as u64 > LIMIT {
        return None;
    }
    serde_json::from_slice(&output).ok()
}

#[derive(Debug, thiserror::Error)]
#[error("Chat model provider re-entry")]
pub(crate) struct ChatModelReentry {
    core: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    home: PathBuf,
    return_profile: Option<String>,
}
impl ChatModelReentry {
    pub(crate) fn from_error(error: &std::io::Error) -> Option<&Self> {
        error.get_ref()?.downcast_ref::<Self>()
    }
    fn command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.core);
        command.env_remove("CODEX_QUOTA_SWITCH_NOTICE");
        command.env_remove("CODEX_PROFILE_PERMISSION_SNAPSHOT");
        command
            .args(&self.args)
            .current_dir(&self.cwd)
            .env("CODEX_HOME", &self.home);
        if let Some(profile) = &self.return_profile {
            command.env("CODEX_CHAT_RETURN_PROFILE", profile);
        } else {
            command.env_remove("CODEX_CHAT_RETURN_PROFILE");
        }
        command
    }
    pub(crate) fn exec(&self) -> std::io::Error {
        self.command().exec()
    }
}
pub(super) async fn prepare(
    app: &mut App,
    server: &mut AppServerSession,
    model: String,
    effort: Option<ReasoningEffort>,
    request_summary: bool,
) -> Option<ChatModelReentry> {
    if app
        .chat_model_switch
        .0
        .as_ref()
        .is_some_and(|v| Some(v.thread) != app.chat_widget.thread_id())
    {
        cancel(app)
    }
    if app.chat_model_switch.0.is_some() {
        app.chat_widget.add_error_message("A model provider switch is already preparing context. Wait or interrupt the current compaction.".into());
        return None;
    }
    plan(app, server, model, effort, request_summary).await
}

fn blank_history_read_error(error: &color_eyre::Report, thread: ThreadId) -> bool {
    matches!(
        error.downcast_ref::<codex_app_server_client::TypedRequestError>(),
        Some(codex_app_server_client::TypedRequestError::Server { method, source })
            if method == "thread/read"
                && ((source.code == -32601 && source.message == "list_turns is not supported yet")
                    || (source.code == -32600 && source.message == format!(
                        "thread {thread} is not materialized yet; includeTurns is unavailable before first user message"
                    )))
    )
}

// Metadata-only thread reads omit turns even for populated threads. The backend
// error alone is never evidence of emptiness: require the complete canonical
// rollout to contain exactly one root session metadata record, using upstream's
// decoder and without changing history or asking a model to materialize it.
fn blank_rollout(
    config: &crate::legacy_core::config::Config,
    snapshot: &Value,
    thread: ThreadId,
) -> bool {
    let Some(reported_path) = snapshot["path"].as_str().map(PathBuf::from) else {
        return false;
    };
    let Ok(sessions) = std::fs::canonicalize(config.codex_home.join("sessions")) else {
        return false;
    };
    // Account homes share sessions through a directory symlink. Resolve the
    // parent, retaining the leaf for O_NOFOLLOW rather than following file links.
    let Some(parent) = reported_path.parent() else {
        return false;
    };
    let (Ok(directory), Some(name)) = (std::fs::canonicalize(parent), reported_path.file_name())
    else {
        return false;
    };
    let path = directory.join(name);
    if !reported_path.is_absolute()
        || snapshot["id"] != thread.to_string()
        || snapshot["status"]["type"] != "idle"
        || snapshot["ephemeral"] != false
        || snapshot["turns"]
            .as_array()
            .is_none_or(|turns| !turns.is_empty())
        || !directory.starts_with(&sessions)
        || std::fs::canonicalize(&path).ok().as_ref() != Some(&path)
    {
        return false;
    }
    let Ok(file) = std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)
    else {
        return false;
    };
    let Ok(before) = file.metadata() else {
        return false;
    };
    const LIMIT: u64 = 32 * 1024 * 1024;
    if !before.is_file()
        || before.uid() != unsafe { libc::geteuid() }
        || before.mode() & 0o022 != 0
        || before.len() > LIMIT
    {
        return false;
    }
    let mut bytes = Vec::new();
    if (&file).take(LIMIT + 1).read_to_end(&mut bytes).is_err()
        || bytes.len() as u64 != before.len()
    {
        return false;
    }
    let Ok(after) = std::fs::symlink_metadata(&path) else {
        return false;
    };
    if (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return false;
    }
    if std::fs::canonicalize(parent).ok().as_ref() != Some(&directory) {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    let Ok(line) = codex_rollout::decode_rollout_line(value) else {
        return false;
    };
    matches!(line.item, codex_rollout::RolloutItem::SessionMeta(meta)
        if meta.meta.id == thread && meta.meta.session_id == thread.into()
            && meta.meta.forked_from_id.is_none()
            && meta.meta.forked_from_ordinal_exclusive.is_none()
            && meta.meta.parent_thread_id.is_none()
            && meta.meta.history_base.is_none()
            && meta.meta.subagent_history_start_ordinal.is_none())
}

async fn public_snapshot(
    server: &mut AppServerSession,
    config: &crate::legacy_core::config::Config,
    thread: ThreadId,
) -> Option<Value> {
    match server.thread_read(thread, true).await {
        Ok(snapshot) => serde_json::to_value(snapshot).ok(),
        Err(error) if blank_history_read_error(&error, thread) => {
            let snapshot =
                serde_json::to_value(server.thread_read(thread, false).await.ok()?).ok()?;
            blank_rollout(config, &snapshot, thread).then_some(snapshot)
        }
        Err(_) => None,
    }
}

fn settled_for_provider_switch(snapshot: &Value) -> bool {
    let Some(turns) = snapshot["turns"].as_array() else {
        return false;
    };
    if turns.iter().any(|turn| turn["status"] == "inProgress") {
        return false;
    }
    match snapshot["status"]["type"].as_str() {
        Some("idle") => true,
        Some("systemError") => turns.last().is_some_and(|turn| turn["status"] == "failed"),
        _ => false,
    }
}

async fn plan(
    app: &mut App,
    server: &mut AppServerSession,
    model: String,
    effort: Option<ReasoningEffort>,
    request_summary: bool,
) -> Option<ChatModelReentry> {
    let thread = app.chat_widget.thread_id().filter(|thread| {
        matches!(
            app.app_server_target,
            crate::AppServerTarget::LocalDaemon { .. }
        ) && app.chat_widget.composer_is_empty()
            && app.dynamic_tool_tasks.is_empty()
            && app.chat_widget.can_change_working_directory(*thread)
    });
    let Some(thread) = thread else {
        app.chat_widget.add_error_message("Provider switching requires an idle primary conversation without pending input or tool calls.".into());
        return None;
    };
    let config = app.chat_widget.config_ref();
    let entering = model.starts_with("chatgpt-desktop/");
    let source_model = app.chat_widget.current_model().to_owned();
    let effort = effort.unwrap_or(ReasoningEffort::None);
    if !termux_chat_models::needs_reentry(&config.model_provider_id, &model)
        || config
            .config_layer_stack
            .required_model_provider()
            .is_some()
        || !termux_chat_models::admits(config, &model, &effort)
    {
        app.chat_widget
            .add_error_message("Selected model/provider is unavailable.".into());
        return None;
    }
    let core = PathBuf::from(std::env::var_os("CODEX_PROFILE_CORE").unwrap_or_default());
    if !std::fs::symlink_metadata(&core).is_ok_and(|m| {
        core.is_absolute()
            && m.is_file()
            && m.uid() == unsafe { libc::geteuid() }
            && m.mode() & 0o022 == 0
            && m.mode() & 0o111 != 0
    }) {
        return None;
    }
    let route = termux_chat_models::reentry_route(
        config,
        &model,
        &effort,
        std::env::var("CODEX_CHAT_RETURN_PROFILE").ok(),
    )?;
    let mut args = route.args;
    args.extend([
        "resume".into(),
        thread.to_string(),
        "--model".into(),
        model.clone(),
    ]);
    let request = ChatModelReentry {
        core,
        args,
        cwd: config.cwd.to_path_buf(),
        home: config.codex_home.to_path_buf(),
        return_profile: route.return_profile,
    };
    if !matches!(
        tokio::time::timeout(
            Duration::from_secs(3),
            super::profile_manager::ensure_resumable(server, thread)
        )
        .await,
        Ok(true)
    ) {
        app.chat_widget.add_error_message(
            "Current conversation could not be saved for model provider switching.".into(),
        );
        return None;
    }
    let snapshot = match tokio::time::timeout(
        Duration::from_secs(10),
        public_snapshot(server, app.chat_widget.config_ref(), thread),
    )
    .await
    {
        Ok(Some(snapshot)) => snapshot,
        _ => {
            app.chat_widget.add_error_message(
                "Model provider unchanged: the complete public conversation could not be read."
                    .into(),
            );
            return None;
        }
    };
    if !settled_for_provider_switch(&snapshot) {
        app.chat_widget.add_error_message(
            "Model provider unchanged: conversation work is not settled.".into(),
        );
        return None;
    }
    let empty = snapshot["turns"].as_array().is_some_and(Vec::is_empty);
    if empty {
        // No history needs encoding, including when leaving a never-used Chat
        // route. Confirm the source did not acquire a turn while preparing.
        let confirmed = tokio::time::timeout(
            Duration::from_secs(10),
            public_snapshot(server, app.chat_widget.config_ref(), thread),
        )
        .await
        .ok()??;
        if confirmed["status"]["type"] != "idle" || confirmed["turns"] != snapshot["turns"] {
            app.chat_widget.add_error_message(
                "Model provider unchanged: the conversation changed during context preparation."
                    .into(),
            );
            return None;
        }
    } else if entering {
        let Some(checkpoint) = encode_checkpoint(
            app.chat_widget.config_ref(),
            &snapshot,
            thread,
            &model,
            &effort,
        )
        .await
        else {
            app.chat_widget.add_error_message("Model provider unchanged: the installed Provider could not prepare readable public context.".into());
            return None;
        };
        let confirmed =
            match tokio::time::timeout(Duration::from_secs(10), server.thread_read(thread, true))
                .await
            {
                Ok(Ok(snapshot)) => serde_json::to_value(snapshot).ok()?,
                _ => return None,
            };
        if confirmed["status"]["type"] != "idle" || confirmed["turns"] != snapshot["turns"] {
            app.chat_widget.add_error_message(
                "Model provider unchanged: the conversation changed during context preparation."
                    .into(),
            );
            return None;
        }
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(3),
                server.thread_inject_items(thread, vec![checkpoint])
            )
            .await,
            Ok(Ok(_))
        ) {
            app.chat_widget.add_error_message("Model provider unchanged: context persistence could not be confirmed. Existing history remains readable.".into());
            return None;
        }
    } else if request_summary && snapshot["turns"].as_array().is_some_and(|v| !v.is_empty()) {
        // Retain explicit source-summary preparation for future callers. Ordinary
        // model selection resumes the same upstream-owned history without it.
        app.chat_model_switch.0 = Some(Pending {
            thread,
            model,
            effort,
            source_model,
            turn: None,
            compacted: false,
            ready: false,
        });
        app.chat_widget.add_info_message("Preparing readable context before changing model provider. This requests one summary from the current ChatGPT model; interrupting keeps the current provider.".into(),None);
        if !matches!(
            tokio::time::timeout(Duration::from_secs(3), server.thread_compact_start(thread)).await,
            Ok(Ok(()))
        ) {
            cancel(app)
        }
        return None;
    }
    Some(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn termux_chat_models_failed_turn_can_leave_provider_without_admitting_active_work() {
        for (status, turns, expected) in [
            ("idle", json!([]), true),
            ("idle", json!([{"status":"completed"}]), true),
            ("systemError", json!([{"status":"failed"}]), true),
            ("systemError", json!([]), false),
            ("systemError", json!([{"status":"completed"}]), false),
            ("systemError", json!([{"status":"inProgress"}]), false),
            ("active", json!([{"status":"failed"}]), false),
            ("notLoaded", json!([{"status":"failed"}]), false),
            ("idle", json!([{"status":"inProgress"}]), false),
        ] {
            assert_eq!(
                settled_for_provider_switch(&json!({"status":{"type":status},"turns":turns})),
                expected,
                "{status}"
            );
        }
    }

    #[test]
    fn termux_chat_models_blank_history_errors_are_exact_and_thread_bound() {
        use codex_app_server_client::TypedRequestError;
        use codex_app_server_protocol::JSONRPCErrorError;
        let thread = ThreadId::new();
        let blank = format!(
            "thread {thread} is not materialized yet; includeTurns is unavailable before first user message"
        );
        for (method, code, message, accepted) in [
            (
                "thread/read",
                -32601,
                "list_turns is not supported yet".to_string(),
                true,
            ),
            ("thread/read", -32600, blank.clone(), true),
            ("thread/read", -32603, blank.clone(), false),
            (
                "thread/read",
                -32600,
                blank.replace(&thread.to_string(), &ThreadId::new().to_string()),
                false,
            ),
            (
                "thread/read",
                -32601,
                "thread/items/list is not supported yet".into(),
                false,
            ),
            ("thread/read", -32600, "no rollout found".into(), false),
            (
                "thread/resume",
                -32601,
                "list_turns is not supported yet".into(),
                false,
            ),
        ] {
            let error: color_eyre::Report = TypedRequestError::Server {
                method: method.into(),
                source: JSONRPCErrorError {
                    code,
                    message,
                    data: None,
                },
            }
            .into();
            assert_eq!(blank_history_read_error(&error, thread), accepted);
        }
        assert!(!blank_history_read_error(
            &color_eyre::eyre::eyre!("transport lost"),
            thread
        ));
    }

    #[tokio::test]
    async fn termux_chat_models_blank_rollout_requires_only_owned_root_metadata() {
        use std::os::unix::fs::PermissionsExt;
        let (app, _, _) = super::super::tests::make_test_app_with_channels().await;
        let root = tempfile::tempdir().unwrap();
        let mut config = app.config.clone();
        config.codex_home =
            codex_utils_absolute_path::AbsolutePathBuf::try_from(root.path()).unwrap();
        let sessions = root.path().join("sessions");
        std::fs::create_dir(&sessions).unwrap();
        let path = sessions.join("blank.jsonl");
        let thread = ThreadId::new();
        let meta = codex_protocol::protocol::SessionMeta {
            id: thread,
            session_id: thread.into(),
            ..Default::default()
        };
        let record =
            json!({"timestamp":"2026-10-23T00:00:00Z","type":"session_meta","payload":meta});
        let snapshot = json!({"id":thread.to_string(),"status":{"type":"idle"},"ephemeral":false,"path":path,"turns":[]});
        let bytes = serde_json::to_vec(&record).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(blank_rollout(&config, &snapshot, thread));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        for extra in [
            json!({"type":"response_item","payload":{"type":"message","role":"user"}}),
            record.clone(),
        ] {
            std::fs::write(&path, format!("{record}\n{extra}\n")).unwrap();
            assert!(!blank_rollout(&config, &snapshot, thread));
        }
        for field in ["forked_from_id", "parent_thread_id"] {
            let mut inherited = record.clone();
            inherited["payload"][field] = json!(ThreadId::new());
            std::fs::write(&path, inherited.to_string()).unwrap();
            assert!(!blank_rollout(&config, &snapshot, thread));
        }
        std::fs::write(&path, b"{truncated").unwrap();
        assert!(!blank_rollout(&config, &snapshot, thread));
        std::fs::write(&path, &bytes).unwrap();
        for (field, value) in [
            ("id", json!(ThreadId::new())),
            ("status", json!({"type":"active"})),
            ("ephemeral", json!(true)),
            ("turns", json!([{"id":"turn"}])),
        ] {
            let mut changed = snapshot.clone();
            changed[field] = value;
            assert!(!blank_rollout(&config, &changed, thread));
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert!(!blank_rollout(&config, &snapshot, thread));
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let alias = sessions.join("alias.jsonl");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        let mut linked = snapshot.clone();
        linked["path"] = json!(alias);
        assert!(!blank_rollout(&config, &linked, thread));
        std::fs::remove_file(&path).unwrap();
        assert!(!blank_rollout(&config, &snapshot, thread));
    }

    #[tokio::test]
    async fn termux_chat_models_blank_rollout_accepts_shared_sessions_directory_only() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::fs::symlink;
        let (app, _, _) = super::super::tests::make_test_app_with_channels().await;
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared-sessions");
        let home = root.path().join("account");
        std::fs::create_dir(&shared).unwrap();
        std::fs::create_dir(&home).unwrap();
        symlink(&shared, home.join("sessions")).unwrap();
        let mut config = app.config.clone();
        config.codex_home =
            codex_utils_absolute_path::AbsolutePathBuf::try_from(home.clone()).unwrap();
        let thread = ThreadId::new();
        let meta = codex_protocol::protocol::SessionMeta {
            id: thread,
            session_id: thread.into(),
            ..Default::default()
        };
        let record =
            json!({"timestamp":"2026-10-23T00:00:00Z","type":"session_meta","payload":meta});
        let path = shared.join("blank.jsonl");
        std::fs::write(&path, record.to_string()).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut snapshot = json!({"id":thread.to_string(),"status":{"type":"idle"},"ephemeral":false,"path":home.join("sessions/blank.jsonl"),"turns":[]});
        assert!(blank_rollout(&config, &snapshot, thread));
        snapshot["path"] = json!(path);
        assert!(blank_rollout(&config, &snapshot, thread));
        symlink(&path, shared.join("leaf.jsonl")).unwrap();
        snapshot["path"] = json!(home.join("sessions/leaf.jsonl"));
        assert!(!blank_rollout(&config, &snapshot, thread));
        let outside = root.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::copy(&path, outside.join("blank.jsonl")).unwrap();
        symlink(&outside, shared.join("escape")).unwrap();
        snapshot["path"] = json!(home.join("sessions/escape/blank.jsonl"));
        assert!(!blank_rollout(&config, &snapshot, thread));
        std::fs::write(&path, format!("{record}\n{record}\n")).unwrap();
        snapshot["path"] = json!(home.join("sessions/blank.jsonl"));
        assert!(!blank_rollout(&config, &snapshot, thread));
    }

    #[tokio::test]
    async fn termux_chat_models_checkpoint_child_receives_eof_without_starting_auth() {
        use std::os::unix::fs::PermissionsExt;
        let (app, _, _) = super::super::tests::make_test_app_with_channels().await;
        let root = tempfile::tempdir().unwrap();
        let mut config = app.config.clone();
        config.codex_home =
            codex_utils_absolute_path::AbsolutePathBuf::try_from(root.path()).unwrap();
        config.model = Some("gpt-6.1-sol".into());
        let binary = root.path().join("provider");
        let response = json!({"type":"message","role":"user","content":[{"type":"input_text","text":"readable fixture context"}]});
        std::fs::write(
            &binary,
            format!(
                r#"#!/bin/sh
[ "$1" = checkpoint ] || exit 1
cat >/dev/null
printf '%s\n' '{response}'
"#
            ),
        )
        .unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        let profile = root.path().join("chatgpt.config.toml");
        std::fs::write(
            &profile,
            format!(
                r#"# codex-chatgpt-provider generated v1
model_provider="chatgpt_desktop"
model_catalog_json="/owned/models.json"
[model_providers.chatgpt_desktop]
base_url="http://127.0.0.1:12345/v1"
wire_api="responses"
requires_openai_auth=false
[model_providers.chatgpt_desktop.auth]
command={:?}
args=["token","--state","/owned/state"]
"#,
                binary.to_str().unwrap()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&profile, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            encode_checkpoint(
                &config,
                &json!({"public":"snapshot"}),
                ThreadId::new(),
                "chatgpt-desktop/gpt-6",
                &ReasoningEffort::None
            )
            .await
            .is_some()
        );
    }
    #[tokio::test]
    async fn termux_chat_models_explicit_summary_matches_completion_and_cancels_failures() {
        use crate::app_event::AppEvent;
        use codex_app_server_protocol::ServerNotification;
        for outcome in [
            "completed",
            "interrupted",
            "failed",
            "missing-checkpoint",
            "error",
            "lost",
        ] {
            let (mut app, mut events, _) = super::super::tests::make_test_app_with_channels().await;
            let thread = ThreadId::new();
            app.chat_model_switch.0 = Some(Pending {
                thread,
                model: "gpt-6.1-sol".into(),
                effort: ReasoningEffort::High,
                source_model: "chatgpt-desktop/gpt-6".into(),
                turn: None,
                compacted: false,
                ready: false,
            });
            let started = super::super::tests::turn_started_notification(thread, "compact-turn");
            observe(&mut app, &started);
            let checkpoint = |thread: ThreadId, turn: &str| {
                serde_json::from_value::<ServerNotification>(json!({"method":"item/completed","params":{"threadId":thread.to_string(),"turnId":turn,"item":{"type":"contextCompaction","id":"context"},"completedAtMs":0}})).unwrap()
            };
            observe(&mut app, &checkpoint(ThreadId::new(), "compact-turn"));
            observe(&mut app, &checkpoint(thread, "other-turn"));
            assert!(!app.chat_model_switch.0.as_ref().unwrap().compacted);
            if outcome != "missing-checkpoint" {
                observe(&mut app, &checkpoint(thread, "compact-turn"))
            }
            while events.try_recv().is_ok() {}
            if outcome == "lost" {
                cancel(&mut app)
            }
            if outcome == "error" {
                let error=serde_json::from_value::<ServerNotification>(json!({"method":"error","params":{"threadId":thread.to_string(),"turnId":"compact-turn","error":{"message":"fixture","codexErrorInfo":null,"additionalDetails":null},"willRetry":false}})).unwrap();
                observe(&mut app, &error);
            }
            let status = match outcome {
                "interrupted" => codex_app_server_protocol::TurnStatus::Interrupted,
                "failed" => codex_app_server_protocol::TurnStatus::Failed,
                _ => codex_app_server_protocol::TurnStatus::Completed,
            };
            let done =
                super::super::tests::turn_completed_notification(thread, "compact-turn", status);
            observe(&mut app, &done);
            observe(&mut app, &done);
            let finished = std::iter::from_fn(|| events.try_recv().ok())
                .filter(|event| matches!(event, AppEvent::FinishChatModelProvider { .. }))
                .count();
            assert_eq!(finished, usize::from(outcome == "completed"), "{outcome}");
            assert_eq!(app.chat_model_switch.0.is_some(), outcome == "completed");
        }
    }
    #[test]
    fn termux_chat_models_session_flags_preserve_nested_toml_values() {
        let table: toml::Table = toml::from_str(
            "approval_policy = \"on-request\"\n[features]\nmemories = false\n[shell_environment_policy]\ninherit = \"none\"\nset = { PATH = \"/owned/bin\", PROOF = \"quoted \\\"value\\\"\" }\n",
        ).unwrap();
        for (key, value) in table {
            let flag = format!("{key}={value}");
            let (path, raw) = flag.split_once('=').unwrap();
            let parsed: toml::Table = toml::from_str(&format!("_x_ = {raw}")).unwrap();
            assert_eq!(path, key);
            assert_eq!(parsed.get("_x_"), Some(&value));
        }
    }
    #[test]
    fn termux_chat_models_reentry_keeps_same_home_cwd_thread_and_configuration_identity() {
        let request = ChatModelReentry {
            core: PathBuf::from("/owned/codex"),
            args: vec![
                "--profile".into(),
                "chatgpt".into(),
                "resume".into(),
                "owned-thread".into(),
                "--model".into(),
                "chatgpt-desktop/gpt-6".into(),
            ],
            cwd: PathBuf::from("/owned/work"),
            home: PathBuf::from("/owned/account"),
            return_profile: Some("api".into()),
        };
        let cmd = request.command();
        assert_eq!(
            cmd.get_current_dir(),
            Some(std::path::Path::new("/owned/work"))
        );
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            request
                .args
                .iter()
                .map(std::ffi::OsStr::new)
                .collect::<Vec<_>>()
        );
        assert!(
            cmd.get_envs().any(
                |(k, v)| k == "CODEX_HOME" && v == Some(std::ffi::OsStr::new("/owned/account"))
            )
        );
        assert!(cmd.get_envs().any(
            |(k, v)| k == "CODEX_CHAT_RETURN_PROFILE" && v == Some(std::ffi::OsStr::new("api"))
        ));
    }
    #[tokio::test]
    async fn termux_chat_models_provider_reentry_refuses_busy_queued_side_draft_and_pending_tools()
    {
        assert_eq!(std::env::var_os("CODEX_PROFILE_CORE"), Some("".into()));
        let (fixture, _events, _ops) = super::super::tests::make_test_app_with_channels().await;
        let (mut server, _, proxy) =
            super::super::tests::session_lifecycle_requests::start_recording_app_server(
                &fixture.config,
                None,
                None,
            )
            .await
            .unwrap();
        for state in [
            "busy",
            "queued",
            "side",
            "embedded",
            "unavailable",
            "draft",
            "tool",
            "no-thread",
        ] {
            let (mut app, mut events, mut ops) =
                super::super::tests::make_test_app_with_channels().await;
            let thread = codex_protocol::ThreadId::new();
            let cwd = app.config.cwd.to_path_buf();
            if state != "no-thread" {
                app.chat_widget
                    .handle_thread_session(super::super::tests::test_thread_session(
                        thread,
                        cwd.clone(),
                    ));
            }
            if state != "embedded" {
                app.app_server_target = crate::AppServerTarget::LocalDaemon {
                    allow_embedded_fallback: false,
                    endpoint: crate::RemoteAppServerEndpoint::UnixSocket {
                        socket_path: codex_utils_absolute_path::AbsolutePathBuf::try_from(
                            cwd.join("owned-profile.sock"),
                        )
                        .unwrap(),
                    },
                };
            }
            if matches!(state, "busy" | "queued") {
                app.chat_widget.handle_server_notification(
                    super::super::tests::turn_started_notification(thread, "profile-turn"),
                    None,
                );
            }
            if state == "queued" {
                for text in ["/model", "keep this input"] {
                    app.chat_widget.apply_external_edit(text.to_string());
                    app.chat_widget
                        .handle_key_event(crossterm::event::KeyEvent::new(
                            crossterm::event::KeyCode::Tab,
                            crossterm::event::KeyModifiers::NONE,
                        ));
                }
                app.chat_widget.handle_server_notification(
                    super::super::tests::turn_completed_notification(
                        thread,
                        "profile-turn",
                        codex_app_server_protocol::TurnStatus::Completed,
                    ),
                    None,
                );
            }
            if state == "side" {
                app.chat_widget.set_side_conversation_active(true);
            }
            if state == "draft" {
                app.chat_widget
                    .apply_external_edit("keep my draft".to_string());
            }
            if state == "tool" {
                app.dynamic_tool_tasks.insert(
                    codex_app_server_protocol::RequestId::String("owned-tool".to_string()),
                    ("owned".to_string(), tokio::spawn(std::future::pending())),
                );
            }
            while events.try_recv().is_ok() {}
            while ops.try_recv().is_ok() {}
            let queued = app.chat_widget.queued_user_message_texts();
            if state == "queued" {
                assert_eq!(queued, vec!["keep this input"]);
            }
            if state == "unavailable" {
                assert!(app.chat_widget.can_change_working_directory(thread));
            }
            assert!(
                prepare(
                    &mut app,
                    &mut server,
                    "chatgpt-desktop/gpt-6".to_string(),
                    Some(ReasoningEffort::Medium),
                    false,
                )
                .await
                .is_none()
            );
            assert_eq!(app.chat_widget.queued_user_message_texts(), queued);
            if state == "draft" {
                assert_eq!(
                    app.chat_widget.composer_text_with_pending(),
                    "keep my draft"
                );
            }
            if state == "tool" {
                let (_, task) = app
                    .dynamic_tool_tasks
                    .remove(&codex_app_server_protocol::RequestId::String(
                        "owned-tool".to_string(),
                    ))
                    .unwrap();
                assert!(!task.is_finished());
                task.abort(); // Only the fixture's own pending task is cleaned up.
            }
            assert!(
                ops.try_recv().is_err(),
                "must not submit or interrupt work: {state}"
            );
            for event in std::iter::from_fn(|| events.try_recv().ok()) {
                assert!(!matches!(
                    event,
                    crate::app_event::AppEvent::CodexOp(
                        crate::app_command::AppCommand::UserTurn { .. }
                            | crate::app_command::AppCommand::Interrupt
                    )
                ));
            }
        }
        proxy.abort();
    }
}
