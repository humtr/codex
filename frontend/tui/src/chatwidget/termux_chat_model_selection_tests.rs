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
    let installed: codex_protocol::openai_models::ModelsResponse =
        serde_json::from_slice(chat_catalog).unwrap();
    let chat_models: Vec<ModelPreset> = installed.models.into_iter().map(Into::into).collect();
    let native_catalog: codex_protocol::openai_models::ModelsResponse =
        serde_json::from_value(native).unwrap();
    let native_models: Vec<ModelPreset> =
        native_catalog.models.into_iter().map(Into::into).collect();
    for (provider, valid, foreign, models) in [
        (
            "openai",
            "native-fixture",
            "chatgpt-desktop/gpt-6",
            native_models,
        ),
        (
            "chatgpt_desktop",
            "chatgpt-desktop/gpt-6",
            "native-fixture",
            chat_models,
        ),
    ] {
        chat.config.model_provider_id = provider.into();
        chat.model_catalog =
            std::sync::Arc::new(crate::model_catalog::ModelCatalog::new(models.clone()));
        assert_eq!(
            crate::termux_chat_models::augment(&chat.config, models.clone()),
            models
        );
        while events.try_recv().is_ok() {}
        chat.handle_resolved_model_command_args(&format!(
            "{foreign} --reasoning=none --speed=standard"
        ));
        assert!(
            !std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(
                event,
                AppEvent::SelectSessionModel { .. } | AppEvent::SelectChatModelProvider { .. }
            ))
        );
        chat.handle_resolved_model_command_args(&format!(
            "{valid} --reasoning=none --speed=standard"
        ));
        assert!(std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(
            event, AppEvent::SelectSessionModel { model: selected, effort: Some(ReasoningEffortConfig::None), service_tier: Some(tier) }
                if selected == valid && tier == "default"
        )));
        assert!(ops.try_recv().is_err());
    }
    chat.handle_model_command_args("chatgpt-desktop/gpt-6 --reasoning=medium");
    assert!(std::iter::from_fn(|| events.try_recv().ok()).any(|event| matches!(event,
        AppEvent::RefreshChatModelCommand { args } if args == "chatgpt-desktop/gpt-6 --reasoning=medium")));
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
            let pending: Vec<_> = std::iter::from_fn(|| events.try_recv().ok()).collect();
            assert_eq!(
                pending.len(),
                usize::from(expected_model || expected_effort)
            );
            for event in pending {
                assert!(matches!(event, AppEvent::SelectSessionModel {
                    model: selected, effort: Some(selected_effort), service_tier: None
                } if selected == model && selected_effort == effort));
            }
            assert!(ops.try_recv().is_err());
            // The async account check happens before Native's existing session
            // action. Constructing picker actions must not mutate saved defaults.
            assert_eq!(chat.current_model(), "chatgpt-desktop/gpt-6");
            assert_eq!(
                chat.effective_reasoning_effort(),
                Some(ReasoningEffortConfig::Medium)
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn termux_provider_slash_uses_owned_model_catalog_and_does_not_guess_names() {
    use std::str::FromStr;
    assert_eq!(
        crate::slash_command::SlashCommand::from_str("provider"),
        Ok(crate::slash_command::SlashCommand::Provider),
    );
    for (model, provider, expected) in [
        ("chatgpt-desktop/app/gpt-6", "chatgpt_desktop", true),
        (
            "chatgpt-desktop/app/gpt-6-thinking",
            "chatgpt_desktop",
            true,
        ),
        ("chatgpt-desktop/app/gpt-6-pro", "chatgpt_desktop", true),
        (
            "chatgpt-desktop/app/gpt-5-6-instant",
            "chatgpt_desktop",
            true,
        ),
        (
            "chatgpt-desktop/app/gpt-5-6-thinking",
            "chatgpt_desktop",
            true,
        ),
        ("chatgpt-desktop/app/gpt-5-6-pro", "chatgpt_desktop", true),
        ("chatgpt-desktop/gpt-6", "chatgpt_desktop", true),
        ("gpt-6.1-sol", "chatgpt_desktop", false),
        ("gpt-6.1-sol", "openai", true),
        ("chatgpt-desktop/app/gpt-6", "openai", false),
        ("gpt-6.1-sol", "unknown-provider", false),
    ] {
        assert_eq!(
            ChatWidget::model_belongs_to_provider(model, provider),
            expected,
            "{model} {provider}",
        );
    }
}
