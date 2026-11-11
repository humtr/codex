use super::*;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::BottomPaneView;
use crate::bottom_pane::ListSelectionView;
use crate::keymap::RuntimeKeymap;
use crate::render::renderable::Renderable;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn record() -> serde_json::Value {
    json!({
        "schema":"codex-manager-profiles-v1",
        "profiles":["default","external","work"],
        "current":"work","current_source":"saved","saved_default":"work"
    })
}

#[test]
fn profile_manager_decodes_saved_default_inherited_and_external_identity() {
    let mut value = record();
    let mut expected = Profiles {
        schema: "codex-manager-profiles-v1".to_string(),
        profiles: vec!["default".into(), "external".into(), "work".into()],
        current: Some("work".to_string()),
        current_source: CurrentSource::Saved,
        saved_default: "work".to_string(),
    };
    assert_eq!(decode(value.to_string().as_bytes()), Some(expected));
    value["current_source"] = json!("inherited");
    value["current"] = serde_json::Value::Null;
    expected = Profiles {
        schema: "codex-manager-profiles-v1".to_string(),
        profiles: vec!["default".into(), "external".into(), "work".into()],
        current: None,
        current_source: CurrentSource::Inherited,
        saved_default: "work".to_string(),
    };
    assert_eq!(decode(value.to_string().as_bytes()), Some(expected));
    value["current_source"] = json!("default");
    value["current"] = json!("default");
    value["saved_default"] = json!("default");
    assert!(decode(value.to_string().as_bytes()).is_some());
}

#[test]
fn profile_manager_rejects_unknown_malformed_unregistered_and_unbounded_records() {
    for (field, invalid) in [
        ("schema", json!("v2")),
        ("current_source", json!("unknown")),
        ("extra", json!(true)),
        ("profiles", json!([])),
        ("profiles", json!(["work", "default"])),
        ("profiles", json!(["default", "default"])),
        ("profiles", json!(["default", "home"])),
        ("profiles", json!(["default", "termux"])),
        ("profiles", json!(["default", "../bad"])),
        ("profiles", json!(["default", "한글"])),
        ("profiles", json!(["default", "a".repeat(65)])),
        ("profiles", json!(["default", ""])),
        (
            "profiles",
            json!(
                std::iter::once("default".to_string())
                    .chain((0..256).map(|i| format!("p{i}")))
                    .collect::<Vec<_>>()
            ),
        ),
        ("current", json!("missing")),
        ("current", serde_json::Value::Null),
        ("saved_default", json!("missing")),
        ("current_source", json!("default")),
    ] {
        let mut value = record();
        value[field] = invalid;
        assert_eq!(
            decode(value.to_string().as_bytes()),
            None,
            "{field}: {value}"
        );
    }
    for field in [
        "schema",
        "profiles",
        "current",
        "current_source",
        "saved_default",
    ] {
        let mut value = record();
        value.as_object_mut().unwrap().remove(field);
        assert_eq!(
            decode(value.to_string().as_bytes()),
            None,
            "missing {field}"
        );
    }
    for bytes in [b"{bad".as_slice(), b"{} {}", b"null"] {
        assert_eq!(decode(bytes), None);
    }
}

fn bridge(root: &TempDir, body: &str) -> std::path::PathBuf {
    let path = root.path().join("core");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[tokio::test]
async fn profile_manager_bridge_uses_exact_argv_and_validates_transport() {
    let root = TempDir::new().unwrap();
    let body = format!(
        "test \"$#\" = 2 && test \"$1\" = termux && test \"$2\" = __profile-snapshot-v1 || exit 2\nprintf '%s\\n' '{}'",
        record()
    );
    let path = bridge(&root, &body);
    assert_eq!(load(&path).await, decode(record().to_string().as_bytes()));
    for body in [
        "printf '{}'",
        "printf '%s' 'not json'",
        "exit 1",
        "dd if=/dev/zero bs=65537 count=1 2>/dev/null",
    ] {
        let path = bridge(&root, body);
        assert_eq!(load(&path).await, None, "{body}");
    }
    let path = bridge(&root, &format!("printf '%s' '{}'; exit 1", record()));
    assert_eq!(load(&path).await, None);
}

#[tokio::test]
async fn profile_manager_bridge_rejects_missing_relative_symlink_and_unsafe_modes() {
    let root = TempDir::new().unwrap();
    assert_eq!(load(&root.path().join("missing")).await, None);
    assert_eq!(load(Path::new("relative-core")).await, None);
    let path = bridge(&root, &format!("printf '%s' '{}'", record()));
    let current = std::env::current_dir().unwrap();
    let mut relative = std::path::PathBuf::new();
    for _ in current.components().skip(1) {
        relative.push("..");
    }
    relative.push(path.strip_prefix("/").unwrap());
    assert!(std::fs::metadata(&relative).unwrap().is_file());
    assert_eq!(load(&relative).await, None);
    assert!(load(&path).await.is_some());
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert_eq!(load(&link).await, None);
    assert_eq!(load(root.path()).await, None);
    for mode in [0o600, 0o720, 0o702] {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(load(&path).await, None);
    }
}

#[tokio::test]
async fn profile_manager_bridge_times_out_and_kills_its_owned_child() {
    let root = TempDir::new().unwrap();
    let path = bridge(&root, "printf '%s' \"$$\" > \"$0.pid\"; exec sleep 30");
    let start = std::time::Instant::now();
    assert_eq!(load(&path).await, None);
    assert!(start.elapsed() < Duration::from_secs(6));
    let pid: i32 = std::fs::read_to_string(root.path().join("core.pid"))
        .unwrap()
        .parse()
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed-out bridge survived"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn render(view: &ListSelectionView) -> String {
    let area = Rect::new(0, 0, 72, view.desired_height(72));
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn profile_manager_picker_direct_arrow_or_search_enter_dispatches_without_another_menu() {
    for search in [false, true] {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let keymap = RuntimeKeymap::defaults();
        let mut view = ListSelectionView::new(
            picker(
                decode(record().to_string().as_bytes()).unwrap(),
                &keymap.list,
            ),
            AppEventSender::new(tx),
            keymap.list,
        );
        if !search {
            insta::assert_snapshot!("profile_manager_saved", render(&view));
            view.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        } else {
            for c in "external".chars() {
                view.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            assert!(render(&view).contains("› external"));
        }
        view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            matches!(rx.try_recv().unwrap(), AppEvent::SelectExecutionProfile(id) if id == "external")
        );
        assert!(
            rx.try_recv().is_err(),
            "one Enter must dispatch one direct transition"
        );
        assert!(!view.is_complete(), "validation protects queued input");
        view.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(view.is_complete());
    }
}

#[test]
fn profile_manager_current_profile_and_cancel_close_without_reentry() {
    for key in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('c')] {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let keymap = RuntimeKeymap::defaults();
        let mut view = ListSelectionView::new(
            picker(
                decode(record().to_string().as_bytes()).unwrap(),
                &keymap.list,
            ),
            AppEventSender::new(tx),
            keymap.list,
        );
        for c in "work".chars() {
            view.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert!(render(&view).contains("› work (current)"));
        if key == KeyCode::Char('c') {
            // BottomPane routes Ctrl-C to the active view's cancellation handler.
            view.on_ctrl_c();
        } else {
            view.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE));
        }
        assert!(view.is_complete());
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn profile_manager_picker_distinguishes_external_home_from_registered_external() {
    let mut value = record();
    value["current"] = serde_json::Value::Null;
    value["current_source"] = json!("inherited");
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let keymap = RuntimeKeymap::defaults();
    let view = ListSelectionView::new(
        picker(decode(value.to_string().as_bytes()).unwrap(), &keymap.list),
        AppEventSender::new(tx),
        keymap.list,
    );
    insta::assert_snapshot!("profile_manager_external_home", render(&view));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn profile_manager_bridge_rejects_the_hosted_nonowner_fixture() {
    let path = std::path::PathBuf::from(
        std::env::var_os("CODEX_PROFILE_NONOWNER_CORE")
            .expect("hosted ownership fixture must be prepared"),
    );
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.is_file());
    assert_ne!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(metadata.mode() & 0o777, 0o755);
    assert_eq!(load(&path).await, None);
}

#[tokio::test]
async fn profile_manager_unavailable_bridge_resumes_follow_up_after_a_queued_picker() {
    assert_eq!(std::env::var_os("CODEX_PROFILE_CORE"), Some("".into()));
    let (mut app, mut events, mut ops) = super::super::tests::make_test_app_with_channels().await;
    let thread = codex_protocol::ThreadId::new();
    let cwd = app.config.cwd.to_path_buf();
    app.chat_widget
        .handle_thread_session(super::super::tests::test_thread_session(thread, cwd));
    app.chat_widget.handle_server_notification(
        super::super::tests::turn_started_notification(thread, "profile-turn"),
        None,
    );
    while events.try_recv().is_ok() {}
    while ops.try_recv().is_ok() {}
    for text in ["/switch", "follow up after profiles"] {
        app.chat_widget.apply_external_edit(text.to_string());
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    app.chat_widget.handle_server_notification(
        super::super::tests::turn_completed_notification(
            thread,
            "profile-turn",
            codex_app_server_protocol::TurnStatus::Completed,
        ),
        None,
    );
    assert!(
        std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, crate::app_event::AppEvent::OpenProfilePicker))
    );
    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["follow up after profiles"]
    );
    assert!(ops.try_recv().is_err());
    open(&mut app).await;
    assert!(app.chat_widget.queued_user_message_texts().is_empty());
    let submitted = std::iter::from_fn(|| ops.try_recv().ok())
        .filter(|op| matches!(op, crate::app_command::AppCommand::UserTurn { .. }))
        .count();
    assert_eq!(submitted, 1);
}

#[test]
fn profile_manager_reentry_preserves_typed_exit_and_exact_profile_thread_argv() {
    use std::ffi::OsStr;
    let id = "12345678-1234-1234-1234-123456789abc";
    let request = ProfileReentry {
        core: "/owned/core".into(),
        profile: "work".to_string(),
        thread: codex_protocol::ThreadId::from_string(id).unwrap(),
        cwd: "/owned/current-workspace".into(),
        flags: Vec::new(),
        handoff: None,
        session_off: false,
        permissions: None,
    };
    let report = color_eyre::eyre::Report::new(std::io::Error::other(request));
    let error = report.downcast::<std::io::Error>().unwrap();
    let command = ProfileReentry::from_error(&error).unwrap().command();
    assert_eq!(command.get_program(), OsStr::new("/owned/core"));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        ["termux", "__profile-resume-v2", "work", id, "[]"].map(OsStr::new)
    );
    assert_eq!(
        command.get_current_dir(),
        Some(std::path::Path::new("/owned/current-workspace"))
    );
    assert!(command.get_envs().any(|(key, value)| {
        key == crate::termux_reentry::ENV && value == Some(OsStr::new("resume"))
    }));
    assert!(command.get_envs().any(|(key, value)| {
        key == crate::termux_reentry::OWNER_PID_ENV
            && value == Some(OsStr::new(&std::process::id().to_string()))
    }));
    assert!(ProfileReentry::from_error(&std::io::Error::other("unrelated")).is_none());
}

#[tokio::test]
async fn profile_manager_reentry_rejects_busy_queued_side_and_unavailable_without_work() {
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
            for text in ["/switch", "keep this input"] {
                app.chat_widget.apply_external_edit(text.to_string());
                app.chat_widget
                    .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
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
            prepare(&mut app, &mut server, "work".to_string())
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
                AppEvent::CodexOp(
                    crate::app_command::AppCommand::UserTurn { .. }
                        | crate::app_command::AppCommand::Interrupt
                )
            ));
        }
    }
    proxy.abort();
}

#[tokio::test]
async fn profile_manager_persistence_materializes_blank_and_skips_saved_history()
-> color_eyre::Result<()> {
    use super::super::tests::session_lifecycle_requests::recorded_params;
    use super::super::tests::session_lifecycle_requests::start_recording_app_server;
    let (app, _events, _ops) = super::super::tests::make_test_app_with_channels().await;
    let (mut server, requests, proxy) = start_recording_app_server(&app.config, None, None).await?;
    let started = server.start_thread(&app.config).await?;
    let thread = started.session.thread_id;
    let path = server.thread_read(thread, false).await?.path.unwrap();
    assert!(
        !path.exists(),
        "fixture must be a genuinely blank native thread"
    );
    requests.lock().unwrap().clear();
    assert!(ensure_resumable(&mut server, thread).await);
    assert!(path.is_file());
    let reads = recorded_params(&requests, "thread/read")
        .into_iter()
        .map(serde_json::from_value::<codex_app_server_protocol::ThreadReadParams>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        reads,
        vec![
            codex_app_server_protocol::ThreadReadParams {
                thread_id: thread.to_string(),
                include_turns: false
            },
            codex_app_server_protocol::ThreadReadParams {
                thread_id: thread.to_string(),
                include_turns: true
            },
        ]
    );
    requests.lock().unwrap().clear();
    assert!(ensure_resumable(&mut server, thread).await);
    let reads = recorded_params(&requests, "thread/read")
        .into_iter()
        .map(serde_json::from_value::<codex_app_server_protocol::ThreadReadParams>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        reads,
        vec![codex_app_server_protocol::ThreadReadParams {
            thread_id: thread.to_string(),
            include_turns: false
        },]
    );
    for method in [
        "turn/start",
        "thread/archive",
        "thread/unarchive",
        "thread/metadata/update",
    ] {
        assert!(recorded_params(&requests, method).is_empty());
    }
    proxy.abort();
    Ok(())
}

#[tokio::test]
async fn profile_manager_persistence_rejects_missing_and_unwritable_without_work()
-> color_eyre::Result<()> {
    use super::super::tests::session_lifecycle_requests::recorded_params;
    use super::super::tests::session_lifecycle_requests::start_recording_app_server;
    let (app, _events, _ops) = super::super::tests::make_test_app_with_channels().await;
    let (mut server, requests, proxy) = start_recording_app_server(&app.config, None, None).await?;
    assert!(!ensure_resumable(&mut server, codex_protocol::ThreadId::new()).await);
    let mut ephemeral = app.config.clone();
    ephemeral.ephemeral = true;
    let temporary_thread = server.start_thread(&ephemeral).await?.session.thread_id;
    assert!(!ensure_resumable(&mut server, temporary_thread).await);
    let started = server.start_thread(&app.config).await?;
    let thread = started.session.thread_id;
    let path = server.thread_read(thread, false).await?.path.unwrap();
    assert!(!path.exists());
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    let original = std::fs::metadata(parent)?.permissions();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500))?;
    let resumable = ensure_resumable(&mut server, thread).await;
    std::fs::set_permissions(parent, original)?;
    assert!(
        !resumable,
        "failed persistence must not allow client cleanup"
    );
    assert!(!path.exists());
    assert!(recorded_params(&requests, "turn/start").is_empty());
    assert!(recorded_params(&requests, "thread/unsubscribe").is_empty());
    proxy.abort();
    Ok(())
}

#[tokio::test]
async fn profile_manager_preserves_current_selection_instead_of_launch_defaults()
-> color_eyre::Result<()> {
    use codex_protocol::openai_models::ReasoningEffort;
    let (mut chat, _events, _rx, _ops) =
        crate::chatwidget::tests::make_chatwidget_manual_with_sender().await;
    let mut session = super::super::tests::test_thread_session(
        codex_protocol::ThreadId::new(),
        chat.config_ref().cwd.to_path_buf(),
    );
    session.model_provider_id = crate::termux_chat_models::PROVIDER.into();
    session.model = "chatgpt-desktop/app/previous-server-model".into();
    session.reasoning_effort = Some(ReasoningEffort::Low);
    chat.handle_thread_session(session);
    let launch_model = chat.config_ref().model.clone();
    let launch_effort = chat.config_ref().model_reasoning_effort.clone();
    chat.set_model("chatgpt-desktop/app/current-server-model");
    chat.set_service_tier(Some("default".into()));
    for effort in [
        Some(ReasoningEffort::High),
        Some(ReasoningEffort::None),
        None,
    ] {
        chat.set_reasoning_effort(effort.clone());
        let selection = chat.effective_collaboration_mode();
        let flags = preserved_flags(chat.config_ref(), &selection).unwrap();
        assert_eq!(
            flags
                .chunks_exact(2)
                .find(|pair| pair[0] == "--model")
                .unwrap()[1],
            "chatgpt-desktop/app/current-server-model"
        );
        let parsed = codex_utils_cli::CliConfigOverrides {
            raw_overrides: flags
                .chunks_exact(2)
                .filter(|pair| pair[0] == "-c")
                .map(|pair| pair[1].clone())
                .collect(),
        }
        .parse_overrides()
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
        let lookup = |key: &str| {
            parsed
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
        };
        assert_eq!(
            lookup("model_provider").and_then(toml::Value::as_str),
            Some(crate::termux_chat_models::PROVIDER)
        );
        assert_eq!(
            lookup("model_reasoning_effort").and_then(toml::Value::as_str),
            effort.as_ref().map(ReasoningEffort::as_str)
        );
        assert_eq!(
            lookup("service_tier").and_then(toml::Value::as_str),
            Some("default")
        );
        assert_eq!(chat.config_ref().model, launch_model);
        assert_eq!(chat.config_ref().model_reasoning_effort, launch_effort);
    }
    Ok(())
}

#[tokio::test]
async fn profile_manager_replaces_stale_session_flags_and_keeps_permission_authority()
-> color_eyre::Result<()> {
    use codex_protocol::config_types::{CollaborationMode, ModeKind, Settings};
    use codex_protocol::openai_models::ReasoningEffort;
    let home = tempfile::tempdir()?;
    let catalog = include_str!("../fixtures/chat-model-catalog.json");
    std::fs::write(home.path().join("models.json"), catalog)?;
    let mut config = crate::legacy_core::config::ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .cli_overrides(vec![
            ("model_providers".into(), "[chatgpt_desktop]\nname='Owned Chat fixture'\nwire_api='responses'\nbase_url='http://127.0.0.1:1/v1'\nrequires_openai_auth=false\n".parse::<toml::Value>()?),
            ("model".into(), "launch-model".into()),
            ("model_provider".into(), "openai".into()),
            ("model_reasoning_effort".into(), "low".into()),
            ("service_tier".into(), "priority".into()),
            ("approval_policy".into(), "on-request".into()),
            ("approvals_reviewer".into(), "user".into()),
            ("sandbox_mode".into(), "read-only".into()),
            (
                "model_catalog_json".into(),
                home.path()
                    .join("models.json")
                    .to_string_lossy()
                    .into_owned()
                    .into(),
            ),
        ])
        .build()
        .await?;
    config.model_provider_id = crate::termux_chat_models::PROVIDER.into();
    config.service_tier = Some("default".into());
    let selection = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model: "chatgpt-desktop/app/current-server-model".into(),
            reasoning_effort: Some(ReasoningEffort::High),
            developer_instructions: None,
        },
    };
    let flags = preserved_flags(&config, &selection).unwrap();
    let parsed = codex_utils_cli::CliConfigOverrides {
        raw_overrides: flags
            .chunks_exact(2)
            .filter(|pair| pair[0] == "-c")
            .map(|pair| pair[1].clone())
            .collect(),
    }
    .parse_overrides()
    .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
    let lookup = |key: &str| {
        let values: Vec<_> = parsed.iter().filter(|(name, _)| name == key).collect();
        assert_eq!(values.len(), 1, "exactly one current {key}");
        values[0].1.as_str().unwrap().to_string()
    };
    assert_eq!(
        lookup("model_provider"),
        crate::termux_chat_models::PROVIDER
    );
    assert_eq!(lookup("model_reasoning_effort"), "high");
    assert_eq!(lookup("service_tier"), "default");
    assert_eq!(lookup("approval_policy"), "on-request");
    assert_eq!(lookup("approvals_reviewer"), "user");
    if let Some(profile) = config.permissions.active_permission_profile() {
        assert_eq!(lookup("default_permissions"), profile.id);
    } else {
        assert_eq!(lookup("sandbox_mode"), "read-only");
    }
    assert_eq!(
        lookup("model_catalog_json"),
        home.path().join("models.json").to_string_lossy()
    );
    assert_eq!(config.model.as_deref(), Some("launch-model"));
    assert_eq!(config.model_reasoning_effort, Some(ReasoningEffort::Low));
    let destination = tempfile::tempdir()?;
    let target_defaults = "model='destination-default'\nmodel_provider='openai'\nmodel_reasoning_effort='low'\nservice_tier='priority'\napproval_policy='never'\nsandbox_mode='danger-full-access'\n";
    std::fs::write(destination.path().join("config.toml"), target_defaults)?;
    let mut target_overrides = parsed;
    let selected_model = flags
        .chunks_exact(2)
        .find(|pair| pair[0] == "--model")
        .unwrap()[1]
        .clone();
    target_overrides.push(("model".into(), selected_model.into()));
    let target = crate::legacy_core::config::ConfigBuilder::default()
        .codex_home(destination.path().to_path_buf())
        .cli_overrides(target_overrides)
        .build()
        .await?;
    assert_eq!(target.model.as_deref(), Some(selection.model()));
    assert_eq!(
        target.model_provider_id,
        crate::termux_chat_models::PROVIDER
    );
    assert_eq!(target.model_reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(target.service_tier.as_deref(), Some("default"));
    assert_eq!(
        target.permissions.approval_policy.value(),
        config.permissions.approval_policy.value()
    );
    assert_eq!(
        target.permissions.effective_permission_profile(),
        config.permissions.effective_permission_profile()
    );
    assert_eq!(target.approvals_reviewer, config.approvals_reviewer);
    assert_eq!(target.model_catalog_source, config.model_catalog_source);
    assert_eq!(
        std::fs::read_to_string(destination.path().join("config.toml"))?,
        target_defaults
    );
    Ok(())
}

#[tokio::test]
async fn profile_manager_requires_native_ack_before_preserving_current_selection()
-> color_eyre::Result<()> {
    use super::super::tests::session_lifecycle_requests::{
        recorded_params, start_recording_app_server,
    };
    use codex_protocol::openai_models::ReasoningEffort;
    let (mut app, _events, _ops) = super::super::tests::make_test_app_with_channels().await;
    let (mut server, requests, proxy) = start_recording_app_server(&app.config, None, None).await?;
    let started = server.start_thread(&app.config).await?;
    let thread = started.session.thread_id;
    app.chat_widget.handle_thread_session(started.session);
    let original = app.chat_widget.effective_collaboration_mode();
    requests.lock().unwrap().clear();
    assert!(
        current_selection_saved(&mut server, thread, app.chat_widget.config_ref(), &original).await
    );
    app.chat_widget.set_model("unacknowledged-model");
    let unsaved = app.chat_widget.effective_collaboration_mode();
    assert!(
        !current_selection_saved(&mut server, thread, app.chat_widget.config_ref(), &unsaved).await
    );
    assert_eq!(
        server.thread_read(thread, false).await?.model.as_deref(),
        Some(original.model())
    );
    app.chat_widget.set_model(original.model());
    for effort in [
        Some(ReasoningEffort::High),
        Some(ReasoningEffort::None),
        None,
    ] {
        let selection = original.with_updates(None, Some(effort.clone()), None);
        assert!(
            server
                .thread_settings_update(codex_app_server_protocol::ThreadSettingsUpdateParams {
                    thread_id: thread.to_string(),
                    effort: effort.clone(),
                    collaboration_mode: Some(selection.clone()),
                    ..Default::default()
                })
                .await?
        );
        app.chat_widget.set_reasoning_effort(effort);
        assert!(
            current_selection_saved(
                &mut server,
                thread,
                app.chat_widget.config_ref(),
                &selection
            )
            .await
        );
    }
    let mut wrong_provider = app.chat_widget.config_ref().clone();
    wrong_provider.model_provider_id = "foreign-provider".into();
    assert!(
        !current_selection_saved(
            &mut server,
            thread,
            &wrong_provider,
            &app.chat_widget.effective_collaboration_mode()
        )
        .await
    );
    for method in [
        "turn/start",
        "thread/resume",
        "thread/unsubscribe",
        "thread/archive",
        "thread/unarchive",
    ] {
        assert!(
            recorded_params(&requests, method).is_empty(),
            "read/ack must not call {method}"
        );
    }
    proxy.abort();
    Ok(())
}
