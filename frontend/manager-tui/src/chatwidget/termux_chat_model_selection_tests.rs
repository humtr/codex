use super::*;

#[tokio::test]
async fn termux_chat_models_inline_choices_use_the_same_installed_catalog_as_the_picker() {
    if !crate::termux_permissions::enabled() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "chatwidget::model_popups::termux_chat_model_selection_tests::termux_chat_models_inline_choices_use_the_same_installed_catalog_as_the_picker",
                "--nocapture",
            ])
            .env("CODEX_PROFILE_CORE", "/owned/core")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let catalog = home.path().join("catalog.json");
    let chat_catalog = include_bytes!("../fixtures/chat-model-catalog.json");
    std::fs::write(&catalog, chat_catalog).unwrap();
    let mut native: serde_json::Value = serde_json::from_slice(chat_catalog).unwrap();
    native["models"][0]["slug"] = serde_json::json!("native-fixture");
    native["models"].as_array_mut().unwrap().truncate(1);
    let cache = home.path().join("models_cache.json");
    std::fs::write(&cache, serde_json::to_vec(&native).unwrap()).unwrap();
    let profile = home.path().join("chatgpt.config.toml");
    std::fs::write(&profile, format!(
        "# codex-chatgpt-provider generated v1\nmodel_provider=\"chatgpt_desktop\"\nmodel_catalog_json={catalog:?}\n[model_providers.chatgpt_desktop]\nbase_url=\"http://127.0.0.1:12345/v1\"\nwire_api=\"responses\"\nrequires_openai_auth=false\n[model_providers.chatgpt_desktop.auth]\ncommand=\"/owned/provider\"\nargs=[\"token\",\"--state\",\"/owned/state\"]\n"
    )).unwrap();
    for path in [&catalog, &cache, &profile] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let (mut chat, _, mut events, mut ops) =
        crate::chatwidget::tests::make_chatwidget_manual_with_sender().await;
    chat.config.codex_home =
        codex_utils_absolute_path::AbsolutePathBuf::try_from(home.path()).unwrap();
    chat.thread_id = Some(codex_protocol::ThreadId::new());
    chat.model_catalog = std::sync::Arc::new(crate::model_catalog::ModelCatalog::new(vec![]));
    for (provider, model) in [
        ("openai", "chatgpt-desktop/gpt-6"),
        ("chatgpt_desktop", "native-fixture"),
    ] {
        chat.config.model_provider_id = provider.into();
        while events.try_recv().is_ok() {}
        chat.handle_model_command_args(&format!("{model} --reasoning=none --speed=standard"));
        assert!(std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(
            event, AppEvent::SelectSessionModel { model: selected, effort: Some(ReasoningEffortConfig::None), service_tier: Some(tier) }
                if selected == model && tier == "default"
        )));
        assert!(ops.try_recv().is_err());
    }
}

#[tokio::test]
async fn termux_chat_models_same_provider_uses_only_changed_session_actions_without_persistence() {
    if !crate::termux_permissions::enabled() {
        // The ordinary upstream test process deliberately has no Termux bridge.
        // Exercise the actual runtime branch in an isolated child, without
        // mutating process-global environment around async widget construction.
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "chatwidget::model_popups::termux_chat_model_selection_tests::termux_chat_models_same_provider_uses_only_changed_session_actions_without_persistence",
                "--nocapture",
            ])
            .env("CODEX_PROFILE_CORE", "/owned/core")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        return;
    }
    assert!(crate::termux_permissions::enabled());
    for plan_mode in [false, true] {
        for (model, effort, expected_model, expected_effort) in [
            (
                "chatgpt-desktop/gpt-6",
                ReasoningEffortConfig::Medium,
                false,
                false,
            ),
            (
                "chatgpt-desktop/gpt-6",
                ReasoningEffortConfig::High,
                false,
                true,
            ),
            (
                "chatgpt-desktop/configured-model",
                ReasoningEffortConfig::Medium,
                true,
                false,
            ),
            (
                "chatgpt-desktop/configured-model",
                ReasoningEffortConfig::High,
                true,
                true,
            ),
        ] {
            let (mut chat, tx, mut events, mut ops) =
                crate::chatwidget::tests::make_chatwidget_manual_with_sender().await;
            chat.config.model_provider_id = crate::termux_chat_models::PROVIDER.into();
            // Use the same setters as the app so the active collaboration mask and
            // stored defaults agree with the effective state read by the picker.
            chat.set_model("chatgpt-desktop/gpt-6");
            chat.set_reasoning_effort(Some(ReasoningEffortConfig::Medium));
            let mut mode = chat.current_collaboration_mode().clone();
            mode.mode = if plan_mode {
                ModeKind::Plan
            } else {
                ModeKind::Default
            };
            chat.set_effective_collaboration_mode(mode);
            assert_eq!(chat.current_model(), "chatgpt-desktop/gpt-6");
            assert_eq!(
                chat.effective_reasoning_effort(),
                Some(ReasoningEffortConfig::Medium)
            );
            while events.try_recv().is_ok() {}
            while ops.try_recv().is_ok() {}
            for action in chat.model_selection_actions(model.into(), Some(effort.clone()), false) {
                action(&tx);
            }
            let mut models = 0;
            let mut efforts = 0;
            while let Ok(event) = events.try_recv() {
                match event {
                    AppEvent::UpdateModel(value) => {
                        assert_eq!(value, model);
                        chat.set_model(&value);
                        models += 1;
                    }
                    AppEvent::UpdateReasoningEffort(value) => {
                        assert!(!plan_mode);
                        assert_eq!(value, Some(effort.clone()));
                        chat.set_reasoning_effort(value);
                        efforts += 1;
                    }
                    AppEvent::UpdatePlanModeReasoningEffort(value) => {
                        assert!(plan_mode);
                        assert_eq!(value, Some(effort.clone()));
                        chat.set_plan_mode_reasoning_effort(value);
                        efforts += 1;
                    }
                    _ => panic!("same-provider selection emitted a non-session action"),
                }
            }
            assert_eq!(models, usize::from(expected_model));
            assert_eq!(efforts, usize::from(expected_effort));
            assert!(ops.try_recv().is_err());
            assert_eq!(chat.current_model(), model);
            assert_eq!(chat.effective_reasoning_effort(), Some(effort.clone()));
            if plan_mode {
                assert_eq!(
                    chat.current_collaboration_mode().reasoning_effort(),
                    Some(ReasoningEffortConfig::Medium)
                );
            }
        }
    }
}
