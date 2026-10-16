use super::*;

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
                "chatgpt-desktop/gpt-5.6-sol",
                ReasoningEffortConfig::Medium,
                true,
                false,
            ),
            (
                "chatgpt-desktop/gpt-5.6-sol",
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
