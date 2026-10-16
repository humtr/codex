use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn switch_command_dispatches_without_agent_input_in_idle_busy_side_and_owned_views() {
    for state in ["idle", "busy", "pending", "side", "owned"] {
        let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(None).await;
        chat.thread_id = Some(ThreadId::new());
        match state {
            "busy" => handle_turn_started(&mut chat, "profile-turn"),
            "pending" => chat.input_queue.user_turn_pending_start = true,
            "side" => chat.set_side_conversation_active(true),
            "owned" => chat.set_parent_owned_thread(),
            _ => {}
        }
        chat.bottom_pane
            .set_composer_text("/switch".to_string(), Vec::new(), Vec::new());
        chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let mut opened = 0;
        while let Ok(event) = rx.try_recv() {
            assert!(
                !matches!(
                    event,
                    AppEvent::CodexOp(Op::UserTurn { .. } | Op::Interrupt)
                ),
                "profile command submitted agent work or interruption: {state}"
            );
            if matches!(event, AppEvent::OpenProfilePicker) {
                opened += 1;
            }
        }
        assert_eq!(opened, 1, "{state}");
        assert_eq!(chat.bottom_pane.composer_text(), "", "{state}");
        while let Ok(op) = op_rx.try_recv() {
            assert!(
                !matches!(op, Op::UserTurn { .. } | Op::Interrupt),
                "profile command submitted agent work or interruption: {state}"
            );
        }
        if state == "busy" {
            assert!(chat.turn_lifecycle.agent_turn_running);
            assert!(chat.bottom_pane.is_task_running());
        }
    }
}

#[tokio::test]
async fn switch_command_opens_from_an_unavailable_thread_without_agent_input() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(None).await;
    chat.bottom_pane
        .set_composer_text("/switch".to_string(), Vec::new(), Vec::new());
    chat.handle_restricted_key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        crate::bottom_pane::RestrictedInputMode::UnavailableThread,
    );
    assert!(
        std::iter::from_fn(|| rx.try_recv().ok())
            .any(|event| matches!(event, AppEvent::OpenProfilePicker))
    );
    assert_eq!(chat.bottom_pane.composer_text(), "");
    assert_no_submit_op(&mut op_rx);
}

#[tokio::test]
async fn switch_command_queued_picker_defers_follow_up_instead_of_submitting_it() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(None).await;
    chat.thread_id = Some(ThreadId::new());
    handle_turn_started(&mut chat, "profile-turn");
    for text in ["/switch", "follow up after profiles"] {
        chat.bottom_pane
            .set_composer_text(text.to_string(), Vec::new(), Vec::new());
        chat.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    assert_eq!(chat.input_queue.queued_user_messages.len(), 2);
    handle_turn_completed(&mut chat, "profile-turn", None);
    assert!(
        std::iter::from_fn(|| rx.try_recv().ok())
            .any(|event| matches!(event, AppEvent::OpenProfilePicker))
    );
    assert_eq!(
        chat.queued_user_message_texts(),
        vec!["follow up after profiles".to_string()]
    );
    assert_no_submit_op(&mut op_rx);
}
