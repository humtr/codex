//! Provider source identity controls local disclosure, never native/model history.
use super::App;
use crate::history_cell::{AgentMarkdownCell, HistoryCell};
use codex_protocol::ThreadId;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

#[derive(Default)]
pub(crate) struct State {
    thread: Option<ThreadId>,
    epoch: u64,
    groups: Vec<Group>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Group {
    final_id: String,
    progress_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Readback {
    schema: String,
    thread: String,
    groups: Vec<Group>,
}

pub(super) fn refresh(app: &mut App) {
    let Some(thread) = app.chat_widget.thread_id() else {
        return;
    };
    let Some(provider) = crate::termux_chat_models::installed_command(app.chat_widget.config_ref())
    else {
        return;
    };
    let state = &mut app.provider_presentation;
    if state.thread != Some(thread) {
        state.thread = Some(thread);
        state.groups.clear();
    }
    state.epoch += 1;
    let epoch = state.epoch;
    let tx = app.app_event_tx.clone();
    tokio::spawn(async move {
        let result = read(provider, thread).await;
        tx.send(crate::app_event::AppEvent::ProviderPresentationReady {
            thread,
            epoch,
            result,
        });
    });
}

async fn read(
    provider: crate::termux_chat_models::ProviderCommand,
    thread: ThreadId,
) -> Result<Vec<Group>, String> {
    const LIMIT: u64 = 32 * 1024 * 1024;
    let mut child = tokio::process::Command::new(provider.binary)
        .args(["presentation", "--thread", &thread.to_string(), "--state"])
        .arg(provider.state)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| "provider_presentation_unavailable")?;
    let mut output = Vec::new();
    let mut stdout = child
        .stdout
        .take()
        .ok_or("provider_presentation_unavailable")?
        .take(LIMIT + 1);
    let observed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let (_, status) = tokio::try_join!(stdout.read_to_end(&mut output), child.wait())?;
        Ok::<_, std::io::Error>(status)
    })
    .await
    .map_err(|_| "provider_presentation_timeout")?
    .map_err(|_| "provider_presentation_unavailable")?;
    if !observed.success() || output.len() as u64 > LIMIT {
        return Err("provider_presentation_unavailable".into());
    }
    let result: Readback =
        serde_json::from_slice(&output).map_err(|_| "provider_presentation_invalid")?;
    if result.schema != "codex-chatgpt-presentation-v1" || result.thread != thread.to_string() {
        return Err("provider_presentation_invalid".into());
    }
    Ok(result.groups)
}

pub(super) fn ready(
    app: &mut App,
    tui: &mut crate::tui::Tui,
    thread: ThreadId,
    epoch: u64,
    result: Result<Vec<Group>, String>,
) {
    if app.chat_widget.thread_id() != Some(thread)
        || app.provider_presentation.thread != Some(thread)
        || app.provider_presentation.epoch != epoch
    {
        return;
    }
    match result {
        Ok(groups) => {
            app.provider_presentation.groups = groups;
            apply(app, tui);
        }
        Err(cause) => {
            // Optional presentation failure preserves every visible source cell.
            tracing::warn!(%cause, "Provider progress disclosure unavailable; full content retained");
        }
    }
}

fn replacements(
    cells: &[Arc<dyn HistoryCell>],
    groups: &[Group],
) -> Vec<(usize, Arc<dyn HistoryCell>)> {
    let mut positions = HashMap::new();
    for (index, cell) in cells.iter().enumerate() {
        if let Some(message) = cell.as_any().downcast_ref::<AgentMarkdownCell>() {
            if let Some(id) = message.source_item_id() {
                positions
                    .entry(id)
                    .and_modify(|position| *position = None)
                    .or_insert(Some(index));
            }
        }
    }
    let mut progress = HashMap::new();
    for group in groups {
        let Some(Some(final_index)) = positions.get(group.final_id.as_str()) else {
            continue;
        };
        for id in &group.progress_ids {
            if positions
                .get(id.as_str())
                .is_some_and(|index| index.is_some_and(|index| index < *final_index))
            {
                progress.insert(id.as_str(), *final_index);
            }
        }
    }
    cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            let message = cell.as_any().downcast_ref::<AgentMarkdownCell>()?;
            let folded = message
                .source_item_id()
                .is_some_and(|id| progress.contains_key(id));
            (message.progress_is_folded() != folded).then(|| {
                (
                    index,
                    Arc::new(message.with_progress_folded(folded)) as Arc<dyn HistoryCell>,
                )
            })
        })
        .collect()
}

pub(super) fn apply(app: &mut App, tui: &mut crate::tui::Tui) {
    if app.provider_presentation.thread != app.chat_widget.thread_id() {
        return;
    }
    for (index, replacement) in
        replacements(&app.transcript_cells, &app.provider_presentation.groups)
    {
        // Preserve physical scrollback and unrelated pre-launch output. The
        // retained viewport can repaint/disclose each original position safely.
        app.transcript_view
            .replace_range(&app.transcript_cells, index..index + 1, &replacement);
        if let Some(crate::pager_overlay::Overlay::Transcript(view)) = &mut app.overlay {
            view.regroup_cells(index..index + 1, replacement.clone());
        }
        app.transcript_cells[index] = replacement;
        tui.frame_requester().schedule_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(id: &str, text: &str) -> Arc<dyn HistoryCell> {
        Arc::new(
            AgentMarkdownCell::new(text.into(), std::path::Path::new("/"))
                .with_source_item_id(Some(id.into())),
        )
    }
    #[test]
    fn exact_source_progress_folds_in_place_after_final_without_losing_content() {
        let first = message("p1", "First report.\n```rust\nfn example() {}\n```");
        let tool: Arc<dyn HistoryCell> =
            Arc::new(crate::history_cell::PlainHistoryCell::new(vec![
                ratatui::text::Line::from("Original tool result"),
            ]));
        let second = message("p2", "Second report.");
        let independent = message("other", "First report.\n```rust\nfn example() {}\n```");
        let final_answer = message(
            "final",
            "First report.\nSecond report.\nComplete conclusion with [source](https://example.test).",
        );
        let group = Group {
            final_id: "final".into(),
            progress_ids: vec!["p1".into(), "p2".into()],
        };
        let mut cells = vec![
            first.clone(),
            tool.clone(),
            second.clone(),
            independent.clone(),
        ];
        assert!(
            replacements(&cells, std::slice::from_ref(&group)).is_empty(),
            "no folding before a displayed final"
        );
        cells.push(final_answer.clone());
        let changes = replacements(&cells, std::slice::from_ref(&group));
        assert_eq!(
            changes.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
            [0, 2]
        );
        for (index, replacement) in changes {
            assert_eq!(replacement.copy_source(), cells[index].copy_source());
            assert!(crate::terminal_hyperlinks::lines_with_sources_eq(
                &replacement.transcript_hyperlink_lines(80),
                &cells[index].transcript_hyperlink_lines(80)
            ));
            assert!(
                replacement
                    .as_any()
                    .downcast_ref::<AgentMarkdownCell>()
                    .unwrap()
                    .progress_is_folded()
            );
            cells[index] = replacement;
        }
        assert!(Arc::ptr_eq(&cells[1], &tool));
        assert!(Arc::ptr_eq(&cells[3], &independent));
        assert!(Arc::ptr_eq(&cells[4], &final_answer));
        let mut view = crate::transcript_view::TranscriptView::default();
        let area = ratatui::layout::Rect::new(0, 0, 100, 32);
        let render = |view: &mut crate::transcript_view::TranscriptView| {
            let mut buffer = ratatui::buffer::Buffer::empty(area);
            view.render(area, &mut buffer, &cells);
            buffer
                .content
                .chunks(usize::from(area.width))
                .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
                .collect::<Vec<_>>()
        };
        let folded = render(&mut view);
        let text = folded.join("\n");
        assert_eq!(
            text.matches("fn example()").count(),
            1,
            "the independent identical report remains visible"
        );
        assert!(text.contains("Original tool result") && text.contains("Complete conclusion"));
        let (row, column) = folded
            .iter()
            .enumerate()
            .find_map(|(row, text)| {
                text.find("+ Show details")
                    .map(|column| (row, text[..column].chars().count()))
            })
            .expect("folded progress exposes its disclosure control");
        view.handle_mouse(
            crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: column as u16,
                row: row as u16,
                modifiers: crossterm::event::KeyModifiers::NONE,
            },
            &cells,
        );
        let expanded = render(&mut view).join("\n");
        assert_eq!(
            expanded.matches("fn example()").count(),
            2,
            "expanding reveals the original code without changing final/tool content"
        );
        assert!(
            expanded.contains("Original tool result") && expanded.contains("Complete conclusion")
        );
        assert!(
            replacements(&cells, &[group]).is_empty(),
            "replay does not refold or move entries"
        );
        assert_eq!(
            replacements(&cells, &[]).len(),
            2,
            "withdrawn identity restores full presentation"
        );
    }
}
