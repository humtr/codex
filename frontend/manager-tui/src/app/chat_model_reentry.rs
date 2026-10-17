//! Re-enter the same durable thread because upstream provider updates are launch-scoped.
use super::App;
use crate::app_server_session::AppServerSession;
use crate::termux_chat_models;
use codex_protocol::openai_models::ReasoningEffort;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::time::Duration;

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
    args.extend(["resume".into(), thread.to_string(), "--model".into(), model]);
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
    Some(request)
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    Some(ReasoningEffort::Medium)
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
