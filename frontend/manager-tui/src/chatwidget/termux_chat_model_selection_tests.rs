use super::*;

#[tokio::test]
async fn termux_chat_models_same_provider_uses_only_changed_session_actions_without_persistence() {
    assert!(crate::termux_permissions::enabled());
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
        chat.current_collaboration_mode = chat.current_collaboration_mode.with_updates(
            Some("chatgpt-desktop/gpt-6".into()),
            Some(Some(ReasoningEffortConfig::Medium)),
            None,
        );
        while events.try_recv().is_ok() {}
        for action in chat.model_selection_actions(model.into(), Some(effort.clone()), false) {
            action(&tx);
        }
        let mut models = 0;
        let mut efforts = 0;
        while let Ok(event) = events.try_recv() {
            match event {
                AppEvent::UpdateModel(value) => {
                    assert_eq!(value, model);
                    models += 1;
                }
                AppEvent::UpdateReasoningEffort(value) => {
                    assert_eq!(value, Some(effort.clone()));
                    efforts += 1;
                }
                _ => panic!("same-provider selection emitted a non-session action"),
            }
        }
        assert_eq!(models, usize::from(expected_model));
        assert_eq!(efforts, usize::from(expected_effort));
        assert!(ops.try_recv().is_err());
    }
}
