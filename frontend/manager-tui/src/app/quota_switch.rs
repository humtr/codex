//! Optional quota UI and idle-boundary integration; Manager owns every policy decision.
use super::App;
use crate::app_event::AppEvent;
use crate::bottom_pane::{SelectionItem, SelectionViewParams};
use codex_app_server_protocol::ServerNotification;
use codex_protocol::ThreadId;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct State {
    pub(super) uncertain: bool,
    generation: u64,
    visible: bool,
    settings: bool,
    session_off: bool,
    turn: Option<String>,
    pending: BTreeSet<String>,
    view: Option<Value>,
    handled: Option<String>,
}
fn core() -> PathBuf {
    std::env::var_os("CODEX_PROFILE_CORE")
        .map(PathBuf::from)
        .unwrap_or_default()
}
fn decode(bytes: &[u8]) -> Option<Value> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    if value["schema"] != "codex-manager-profiles-v2"
        || value["rows"].as_array()?.len() > 256
        || !value["rotation"].is_object()
    {
        return None;
    }
    let mut seen = BTreeSet::new();
    for row in value["rows"].as_array()? {
        let id = row["id"].as_str()?;
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || !seen.insert(id)
        {
            return None;
        }
        let summary = row["summary"].as_str()?;
        if summary.len() > 2048 || summary.chars().any(char::is_control) {
            return None;
        }
    }
    Some(value)
}
pub(super) fn close(app: &mut App) {
    app.quota_switch.generation = app.quota_switch.generation.wrapping_add(1);
    app.quota_switch.visible = false;
}
fn item(
    name: String,
    description: Option<String>,
    event: impl Fn() -> AppEvent + Send + Sync + 'static,
) -> SelectionItem {
    SelectionItem {
        name,
        description,
        actions: vec![Box::new(move |tx| tx.send(event()))],
        dismiss_on_select: true,
        ..Default::default()
    }
}
fn picker(view: &Value, settings: bool, session_off: bool) -> SelectionViewParams {
    let policy = &view["rotation"];
    let mut items = Vec::new();
    if settings {
        let enabled = !policy["enabled"].as_bool().unwrap_or(false);
        items.push(item(
            format!(
                "Automatic switching: {}",
                if enabled { "off" } else { "on" }
            ),
            Some("Only explicitly enrolled accounts; safe idle boundaries.".into()),
            move || AppEvent::EditQuotaPolicy(json!({"enabled":enabled})),
        ));
        items.push(item(
            format!(
                "This session: {}",
                if session_off { "off" } else { "enabled" }
            ),
            None,
            || AppEvent::ToggleQuotaSession,
        ));
        let policies = ["combined", "5h", "weekly", "remaining", "custom"];
        let current = policy["policy"].as_str().unwrap_or("combined");
        let next = policies
            [(policies.iter().position(|p| *p == current).unwrap_or(0) + 1) % policies.len()]
        .to_owned();
        items.push(item(
            format!("Ranking: {current}"),
            Some("Enter cycles combined, 5h reset, weekly reset, remaining and custom.".into()),
            move || AppEvent::EditQuotaPolicy(json!({"policy":next})),
        ));
        for (key, label, step, max) in [
            ("reserve", "Minimum reserve", 5, 50),
            ("proactive", "Proactive threshold (0 = off)", 5, 50),
        ] {
            let next = (policy[key].as_i64().unwrap_or(0) + step) % (max + step);
            let key = key.to_owned();
            items.push(item(
                format!("{label}: {}%", policy[&key]),
                None,
                move || AppEvent::EditQuotaPolicy(json!({(key.clone()):next})),
            ));
        }
        let members: Vec<String> = policy["accounts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        for row in view["rows"].as_array().into_iter().flatten() {
            let id = row["id"].as_str().unwrap().to_owned();
            let included = members.contains(&id);
            let mut next = members.clone();
            if included {
                next.retain(|x| x != &id);
            } else {
                next.push(id.clone());
            }
            items.push(item(
                format!("[{}] {id}", if included { "x" } else { " " }),
                row["summary"].as_str().map(str::to_owned),
                move || AppEvent::EditQuotaPolicy(json!({"accounts":next})),
            ));
        }
        items.push(item(
            "Back to profiles".into(),
            Some("Custom order/weights/cooldown: codex termux rotation set JSON".into()),
            || AppEvent::OpenProfilePicker,
        ));
    } else {
        for row in view["rows"].as_array().into_iter().flatten() {
            let id = row["id"].as_str().unwrap().to_owned();
            let current = view["current"].as_str() == Some(&id);
            let selected = id.clone();
            let mut entry = item(id, row["summary"].as_str().map(str::to_owned), move || {
                if current {
                    AppEvent::CloseQuotaView
                } else {
                    AppEvent::SelectExecutionProfile(selected.clone())
                }
            });
            entry.is_current = current;
            items.push(entry);
        }
        items.push(item(
            "Refresh usage".into(),
            Some("Account/model reads only; no inference.".into()),
            || AppEvent::RefreshQuotaView,
        ));
        items.push(item(
            format!(
                "Automatic switching: {} · settings",
                if policy["enabled"] == true && !session_off {
                    "on"
                } else {
                    "off"
                }
            ),
            None,
            || AppEvent::OpenQuotaSettings,
        ));
    }
    SelectionViewParams {
        title: Some(
            if settings {
                "Switch settings"
            } else {
                "Profiles · quota"
            }
            .into(),
        ),
        subtitle: Some(format!(
            "Current: {} · {}",
            view["current"].as_str().unwrap_or("external"),
            policy["policy"].as_str().unwrap_or("combined")
        )),
        items,
        is_searchable: true,
        on_cancel: Some(Box::new(|tx| tx.send(AppEvent::CloseQuotaView))),
        ..SelectionViewParams::picker()
    }
}
fn render(app: &mut App) {
    if let Some(view) = &app.quota_switch.view {
        app.chat_widget.show_selection_view(picker(
            view,
            app.quota_switch.settings,
            app.quota_switch.session_off,
        ));
    }
}
pub(super) async fn open(app: &mut App, settings: bool) -> bool {
    let Some(bytes) = super::profile_manager::call(
        &core(),
        &[
            "__switch-view-v2".into(),
            "--model".into(),
            app.chat_widget
                .config_ref()
                .model
                .clone()
                .unwrap_or_default(),
        ],
        3,
    )
    .await
    else {
        return false;
    };
    let Some(view) = decode(&bytes) else {
        return false;
    };
    app.quota_switch.visible = true;
    app.quota_switch.settings = settings;
    app.quota_switch.view = Some(view);
    app.quota_switch.generation = app.quota_switch.generation.wrapping_add(1);
    render(app);
    refresh(app);
    true
}
pub(super) fn refresh(app: &mut App) {
    let generation = app.quota_switch.generation;
    let tx = app.app_event_tx.clone();
    let model = app
        .chat_widget
        .config_ref()
        .model
        .clone()
        .unwrap_or_default();
    tokio::spawn(async move {
        let args = vec![
            "__switch-view-v2".into(),
            "--refresh".into(),
            "--model".into(),
            model,
        ];
        let view = super::profile_manager::call(&core(), &args, 25)
            .await
            .and_then(|b| decode(&b));
        tx.send(AppEvent::QuotaViewReady { generation, view });
    });
}
pub(super) fn ready(app: &mut App, generation: u64, view: Option<Value>) {
    if app.quota_switch.visible && generation == app.quota_switch.generation {
        if let Some(view) = view {
            app.quota_switch.view = Some(view);
            render(app);
        }
    }
}
pub(super) async fn edit(app: &mut App, value: Value) {
    let args = vec!["rotation".into(), "set".into(), value.to_string()];
    if super::profile_manager::call(&core(), &args, 3)
        .await
        .is_none()
    {
        app.chat_widget.add_error_message("Settings unchanged: refresh usage and enroll identifiable accounts before enabling rotation.".into());
    }
    open(app, true).await;
}
pub(super) fn toggle_session(app: &mut App) {
    app.quota_switch.session_off = !app.quota_switch.session_off;
    render(app);
}

fn event(value: &Value) -> Option<(&str, &Value)> {
    Some((value["method"].as_str()?, &value["params"]))
}
fn record(state: &mut State, method: &str, params: &Value) -> Option<(String, String)> {
    if method == "turn/started" {
        state.turn = params["turn"]["id"].as_str().map(str::to_owned);
        state.pending.clear();
        state.uncertain = false;
        return None;
    }
    if method == "serverRequest/resolved" {
        if let Some(id) = params.get("requestId") {
            state.pending.remove(&format!("request:{id}"));
        }
        return None;
    }
    if matches!(method, "item/started" | "item/completed") {
        let item = &params["item"];
        let kind = item["type"].as_str().unwrap_or("");
        if matches!(
            kind,
            "commandExecution"
                | "fileChange"
                | "mcpToolCall"
                | "dynamicToolCall"
                | "collabAgentToolCall"
        ) {
            if state.turn.as_deref() != params["turnId"].as_str() {
                state.uncertain = true;
                return None;
            }
            if let Some(id) = item["id"].as_str() {
                if method == "item/started" {
                    state.pending.insert(id.into());
                } else {
                    if !state.pending.remove(id) {
                        state.uncertain = true;
                    }
                    if item["status"].as_str() != Some("completed") {
                        state.uncertain = true;
                    }
                }
            }
        }
        return None;
    }
    if method != "turn/completed" {
        return None;
    }
    let turn = params["turn"]["id"].as_str()?;
    if state.handled.as_deref() == Some(turn)
        || state.turn.as_deref() != Some(turn)
        || state.uncertain
        || !state.pending.is_empty()
    {
        return None;
    }
    state.handled = Some(turn.to_owned());
    let status = params["turn"]["status"].as_str()?;
    let trigger = if status == "failed"
        && params["turn"]["error"]["codexErrorInfo"] == "usageLimitExceeded"
    {
        "quota"
    } else if status == "completed" {
        "proactive"
    } else {
        return None;
    };
    Some((turn.into(), trigger.into()))
}
pub(super) fn observe(app: &mut App, notification: &ServerNotification) {
    let Ok(value) = serde_json::to_value(notification) else {
        return;
    };
    let Some((method, params)) = event(&value) else {
        return;
    };
    if params["threadId"].as_str()
        != app
            .chat_widget
            .thread_id()
            .map(|id| id.to_string())
            .as_deref()
    {
        return;
    }
    if let Some((turn, trigger)) = record(&mut app.quota_switch, method, params) {
        if let Some(thread) = app.chat_widget.thread_id() {
            app.app_event_tx.send(AppEvent::AutoQuotaHandoff {
                thread,
                turn,
                trigger,
            });
        }
    }
}
pub(super) fn request(app: &mut App, request: &codex_app_server_protocol::ServerRequest) {
    if let Ok(value) = serde_json::to_value(request) {
        app.quota_switch
            .pending
            .insert(format!("request:{}", value["id"]));
    }
}
pub(super) async fn automatic(
    app: &mut App,
    server: &mut crate::app_server_session::AppServerSession,
    thread: ThreadId,
    turn: String,
    trigger: String,
) -> Option<super::profile_manager::ProfileReentry> {
    if app.quota_switch.session_off
        || app.quota_switch.visible
        || app.chat_widget.has_active_modal()
        || app
            .agent_navigation
            .ordered_threads()
            .iter()
            .any(|(id, entry)| *id != thread && entry.is_running)
        || app.chat_widget.thread_id() != Some(thread)
        || app.chat_widget.config_ref().model_provider_id != "openai"
        || !app.chat_widget.composer_is_empty()
        || !app.dynamic_tool_tasks.is_empty()
        || !app.chat_widget.can_change_working_directory(thread)
    {
        return None;
    }
    // No ordinary-startup network: only a completed turn reaches this handler.
    let model = app.chat_widget.config_ref().model.clone()?;
    let args = vec!["__switch-view-v2".into(), "--model".into(), model.clone()];
    let view = super::profile_manager::call(&core(), &args, 3)
        .await
        .and_then(|b| decode(&b))?;
    if view["rotation"]["enabled"] != true
        || trigger == "proactive" && view["rotation"]["proactive"].as_i64().unwrap_or(0) == 0
    {
        return None;
    }
    let args = vec![
        "__switch-view-v2".into(),
        "--refresh".into(),
        "--model".into(),
        model.clone(),
    ];
    let view = super::profile_manager::call(&core(), &args, 25)
        .await
        .and_then(|b| decode(&b))?;
    let Some(profile) = view["recommended"].as_str() else {
        if trigger == "quota" {
            app.chat_widget.add_error_message("Automatic switching paused: no fresh enrolled account with available quota; work remains saved.".into());
        }
        return None;
    };
    if trigger == "proactive" {
        let threshold = view["rotation"]["proactive"].as_i64()?;
        let current = view["current"].as_str()?;
        if !view["rows"]
            .as_array()?
            .iter()
            .find(|r| r["id"] == current)?["windows"]
            .as_array()?
            .iter()
            .any(|w| w["remaining"].as_i64().is_some_and(|r| r <= threshold))
        {
            return None;
        }
    }
    let mut request = super::profile_manager::prepare(app, server, profile.to_owned()).await?;
    let handoff = json!({"profile":profile,"thread":thread.to_string(),"turn":turn,"model":model,"trigger":trigger,"flags":super::profile_manager::flags(&request)});
    super::profile_manager::automatic(&mut request, handoff);
    Some(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_manager_quota_wire_contract_matches_real_upstream_notifications() {
        let thread = ThreadId::new();
        let notification = super::super::tests::turn_started_notification(thread, "native-turn");
        let value = serde_json::to_value(notification).unwrap();
        let (method, params) = event(&value).unwrap();
        let mut state = State::default();
        record(&mut state, method, params);
        assert_eq!(state.turn.as_deref(), Some("native-turn"));
    }
    #[test]
    fn profile_manager_quota_rows_render_and_settings_actions_use_manager_policy() {
        use crate::app_event_sender::AppEventSender;
        use crate::bottom_pane::{BottomPaneView, ListSelectionView};
        use crate::keymap::RuntimeKeymap;
        use crate::render::renderable::Renderable;
        use ratatui::{buffer::Buffer, layout::Rect};
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let value = json!({"schema":"codex-manager-profiles-v2","current":"default","rows":[{"id":"default","summary":"5h 72% · weekly 12% · ready"}],"rotation":{"enabled":false,"policy":"combined","accounts":[],"reserve":5,"proactive":0}});
        assert!(decode(value.to_string().as_bytes()).is_some());
        let view = ListSelectionView::new(
            picker(&value, false, false),
            AppEventSender::new(tx),
            RuntimeKeymap::defaults().list,
        );
        let rect = Rect::new(0, 0, 90, view.desired_height(90));
        let mut buffer = Buffer::empty(rect);
        view.render(rect, &mut buffer);
        let screen = (0..rect.height)
            .map(|y| {
                (0..rect.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(screen.contains("5h 72%"), "{screen}");
        assert!(screen.contains("weekly 12%"), "{screen}");
        assert!(screen.contains("Automatic switching"), "{screen}");
        let settings = picker(&value, true, false);
        assert!(settings.items.iter().any(|i| i.name == "[ ] default"));
        let mut bad = value.clone();
        bad["rows"][0]["summary"] = json!("forged\nrow");
        assert!(decode(bad.to_string().as_bytes()).is_none());
    }
    #[test]
    fn profile_manager_quota_only_terminal_confirmed_quota_and_settled_live_tools_trigger() {
        let mut state = State::default();
        let start = json!({"turn":{"id":"t"}});
        record(&mut state, "turn/started", &start);
        let tool =
            json!({"turnId":"t","item":{"id":"c","type":"commandExecution","status":"completed"}});
        record(&mut state, "item/started", &tool);
        let done = json!({"turn":{"id":"t","status":"failed","error":{"codexErrorInfo":"usageLimitExceeded"}}});
        assert!(record(&mut state, "turn/completed", &done).is_none());
        record(&mut state, "item/completed", &tool);
        assert_eq!(
            record(&mut state, "turn/completed", &done),
            Some(("t".into(), "quota".into()))
        );
        assert!(record(&mut state, "turn/completed", &done).is_none());
        for error in [
            "rateLimitExceeded",
            "connectionFailed",
            "streamDisconnected",
        ] {
            let mut s = State::default();
            record(&mut s, "turn/started", &start);
            let mut d = done.clone();
            d["turn"]["error"]["codexErrorInfo"] = json!(error);
            assert!(record(&mut s, "turn/completed", &d).is_none());
        }
        let mut s = State::default();
        assert!(record(&mut s, "turn/completed", &done).is_none());
        record(&mut s, "turn/started", &start);
        s.uncertain = true;
        assert!(record(&mut s, "turn/completed", &done).is_none());
    }
}
