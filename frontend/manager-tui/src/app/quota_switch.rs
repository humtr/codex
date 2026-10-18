//! Optional quota UI and idle-boundary integration; Manager owns every policy decision.
use super::App;
use crate::app_event::AppEvent;
use crate::bottom_pane::{SelectionItem, SelectionRowDisplay, SelectionTab, SelectionViewParams};
use codex_app_server_protocol::ServerNotification;
use codex_protocol::ThreadId;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

const VIEW_ID: &str = "codex-manager-quota-v2";

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
    notice: Option<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            uncertain: false,
            generation: 0,
            visible: false,
            settings: false,
            session_off: std::env::var("CODEX_QUOTA_SESSION_OFF").ok().as_deref() == Some("1"),
            turn: None,
            pending: BTreeSet::new(),
            view: None,
            handled: None,
            notice: std::env::var("CODEX_QUOTA_SWITCH_NOTICE")
                .ok()
                .and_then(|v| notice(&v, std::process::id())),
        }
    }
}
fn notice(value: &str, pid: u32) -> Option<String> {
    if value.len() > 512 {
        return None;
    }
    let value: Value = serde_json::from_str(value).ok()?;
    if value["pid"].as_u64() != Some(pid as u64) {
        return None;
    }
    let source = value["source"].as_str()?;
    let target = value["target"].as_str()?;
    if [source, target].iter().any(|id| {
        id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }) {
        return None;
    }
    Some(format!(
        "Account switched: {source} -> {target}. Conversation and settings retained."
    ))
}
pub(super) fn show_notice(app: &mut App) {
    if app.chat_widget.thread_id().is_some()
        && let Some(notice) = app.quota_switch.notice.take()
    {
        app.chat_widget.add_info_message(notice, None);
    }
}
pub(super) fn session_off(app: &App) -> bool {
    app.quota_switch.session_off
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
        search_value: Some(format!(
            "{name} {}",
            description.as_deref().unwrap_or_default()
        )),
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
    let mut tabs = Vec::new();
    if settings {
        let enabled = !policy["enabled"].as_bool().unwrap_or(false);
        items.push(item(
            format!("Auto: {}", if enabled { "off" } else { "on" }),
            None,
            move || AppEvent::EditQuotaPolicy(json!({"enabled":enabled})),
        ));
        items.push(item(
            format!("Here: {}", if session_off { "paused" } else { "allow" }),
            None,
            || AppEvent::ToggleQuotaSession,
        ));
        let policies = ["combined", "5h", "weekly", "remaining", "custom"];
        let current = policy["policy"].as_str().unwrap_or("combined");
        let next = policies
            [(policies.iter().position(|p| *p == current).unwrap_or(0) + 1) % policies.len()]
        .to_owned();
        items.push(item(
            format!(
                "{} · rank",
                match current {
                    "combined" => "balanced",
                    "5h" => "5h reset",
                    "weekly" => "week reset",
                    other => other,
                }
            ),
            None,
            move || AppEvent::EditQuotaPolicy(json!({"policy":next})),
        ));
        for (key, label, step, max) in
            [("reserve", "Reserve", 5, 50), ("proactive", "Early", 5, 50)]
        {
            let next = (policy[key].as_i64().unwrap_or(0) + step) % (max + step);
            let key = key.to_owned();
            items.push(item(
                format!(
                    "{label}: {}",
                    match policy[&key].as_i64().unwrap_or(0) {
                        0 if key == "proactive" => "off".to_owned(),
                        value => format!("{value}%"),
                    }
                ),
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
        let cooldowns = [30, 60, 300, 900, 3600];
        let current = policy["cooldown"].as_u64().unwrap_or(300);
        let next = cooldowns
            [(cooldowns.iter().position(|v| *v == current).unwrap_or(2) + 1) % cooldowns.len()];
        items.push(item(
            format!(
                "Wait: {}",
                if current >= 60 {
                    format!("{}m", current / 60)
                } else {
                    format!("{current}s")
                }
            ),
            None,
            move || AppEvent::EditQuotaPolicy(json!({"cooldown":next})),
        ));
        let mut custom = Vec::new();
        if policy["policy"] == "custom" {
            let weights = policy["weights"]
                .as_array()
                .cloned()
                .unwrap_or_else(|| vec![json!(1); 3]);
            for (index, label) in ["5h", "week", "reset"].iter().enumerate() {
                let mut next = weights.clone();
                let current = weights.get(index).and_then(Value::as_i64).unwrap_or(1);
                let values = [0, 1, 2, 5, 10];
                next[index] = json!(
                    values[(values.iter().position(|v| *v == current).unwrap_or(1) + 1)
                        % values.len()]
                );
                custom.push(item(format!("{current} × {label}"), None, move || {
                    AppEvent::EditQuotaPolicy(json!({"weights":next}))
                }));
            }
            let mut order: Vec<String> = policy["order"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            for id in &members {
                if !order.contains(id) {
                    order.push(id.clone());
                }
            }
            for (rank, id) in order.iter().enumerate() {
                let mut next = order.clone();
                next.retain(|v| v != id);
                next.insert(0, id.clone());
                custom.push(item(format!("{} · {id}", rank + 1), None, move || {
                    AppEvent::EditQuotaPolicy(json!({"order":next}))
                }));
            }
        }
        let mut accounts = Vec::new();
        for row in view["rows"].as_array().into_iter().flatten() {
            let Some(id) = row["id"].as_str() else {
                continue;
            };
            let id = id.to_owned();
            let included = members.contains(&id);
            let mut next = members.clone();
            if included {
                next.retain(|x| x != &id);
            } else {
                next.push(id.clone());
            }
            accounts.push(item(
                format!("[{}] {id}", if included { "x" } else { " " }),
                row["summary"].as_str().map(str::to_owned),
                move || AppEvent::EditQuotaPolicy(json!({"accounts":next})),
            ));
            if included {
                let pause_id = id.clone();
                let seconds = if row["excluded_until"].is_null() {
                    900
                } else {
                    0
                };
                accounts.push(item(
                    format!(
                        "{} · {id}",
                        if seconds == 0 { "Unpause" } else { "Pause 15m" }
                    ),
                    None,
                    move || AppEvent::PauseQuotaAccount(pause_id.clone(), seconds),
                ));
            }
        }
        for (id, label, mut entries) in [
            ("policy", "Policy", items),
            ("accounts", "Accounts", accounts),
            ("custom", "Custom", custom),
        ] {
            if id == "custom" && entries.is_empty() {
                continue;
            }
            entries.push(item("Back to profiles".into(), None, || {
                AppEvent::OpenProfilePicker
            }));
            // Policy mutations refresh this same popup; keep its tab, cursor and
            // filter while the bounded Manager call is in flight.
            for entry in &mut entries {
                entry.dismiss_on_select = false;
            }
            tabs.push(SelectionTab {
                id: id.into(),
                label: label.into(),
                header: Box::new(()),
                items: entries,
            });
        }
        items = Vec::new();
    } else {
        for row in view["rows"].as_array().into_iter().flatten() {
            let Some(id) = row["id"].as_str() else {
                continue;
            };
            let id = id.to_owned();
            let current = view["current"].as_str() == Some(&id);
            let selected = id.clone();
            let mut name = id.clone();
            if !current && view["saved_default"].as_str() == Some(&id) {
                name.push_str(" (default)");
            }
            if view["recommended"].as_str() == Some(&id) {
                name.push_str(" (next)");
            }
            let mut entry = item(
                name,
                row["summary"].as_str().map(str::to_owned),
                move || {
                    if current {
                        AppEvent::CloseQuotaView
                    } else {
                        AppEvent::SelectExecutionProfile(selected.clone())
                    }
                },
            );
            entry.is_current = current;
            entry.dismiss_on_select = current;
            entry.require_explicit_confirmation = true;
            items.push(entry);
        }
        items.push(item(
            "Refresh usage".into(),
            Some("Account/model reads only; no inference.".into()),
            || AppEvent::RefreshQuotaView,
        ));
        items.push(item(
            format!(
                "Auto: {} · settings",
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
        view_id: Some(VIEW_ID),
        title: Some(
            if settings {
                "Switch settings"
            } else {
                "Profiles · Codex usage"
            }
            .into(),
        ),
        subtitle: Some(format!(
            "Current: {} · {}",
            view["current"].as_str().unwrap_or("external"),
            policy["policy"].as_str().unwrap_or("combined")
        )),
        items,
        tabs,
        row_display: SelectionRowDisplay::SingleLine,
        footer_note: settings.then(|| "←/→ section · Enter changes".into()),
        is_searchable: true,
        on_cancel: Some(Box::new(|tx| tx.send(AppEvent::CloseQuotaView))),
        ..SelectionViewParams::picker()
    }
}
fn render(app: &mut App, refresh_only: bool) {
    if let Some(view) = &app.quota_switch.view
        && !app.chat_widget.termux_refresh_quota_view(
            VIEW_ID,
            picker(
                view,
                app.quota_switch.settings,
                app.quota_switch.session_off,
            ),
        )
        && !refresh_only
    {
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
    render(app, false);
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
    if app.quota_switch.visible
        && generation == app.quota_switch.generation
        && let Some(view) = view
    {
        app.quota_switch.view = Some(view);
        render(app, true);
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
    render(app, false);
}
pub(super) async fn pause(app: &mut App, id: String, seconds: u64) {
    let args = vec!["rotation".into(), "exclude".into(), id, seconds.to_string()];
    if super::profile_manager::call(&core(), &args, 3)
        .await
        .is_none()
    {
        app.chat_widget
            .add_error_message("Account pause unchanged; refresh usage first.".into());
    }
    open(app, true).await;
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
    let mut boundary = record(&mut app.quota_switch, method, params);
    if boundary.is_none()
        && method == "turn/completed"
        && params["turn"]["status"] == "failed"
        && params["turn"]["error"]["codexErrorInfo"] == "usageLimitExceeded"
        && let Some(turn) = params["turn"]["id"].as_str()
        && app.quota_switch.handled.as_deref() != Some(turn)
    {
        app.quota_switch.handled = Some(turn.to_owned());
        boundary = Some((turn.to_owned(), "unsafe".to_owned()));
    }
    if let Some((turn, trigger)) = boundary
        && let Some(thread) = app.chat_widget.thread_id()
    {
        app.app_event_tx.send(AppEvent::AutoQuotaHandoff {
            thread,
            turn,
            trigger,
        });
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
    _server: &mut crate::app_server_session::AppServerSession,
    thread: ThreadId,
    turn: String,
    trigger: String,
) -> Option<super::profile_manager::ProfileReentry> {
    if app.quota_switch.session_off
        || app.quota_switch.visible
        || app.chat_widget.thread_id() != Some(thread)
        || app.primary_thread_id != Some(thread)
        || app.chat_widget.config_ref().model_provider_id != "openai"
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
    if trigger == "unsafe"
        || app
            .agent_navigation
            .ordered_threads()
            .iter()
            .any(|(id, entry)| *id != thread && entry.is_running)
        || !app.chat_widget.composer_is_empty()
        || !app.dynamic_tool_tasks.is_empty()
        || !app.chat_widget.can_change_working_directory(thread)
    {
        if trigger != "proactive" {
            app.chat_widget.add_info_message("Automatic switching paused: pending input, running agents/tools or uncertain execution outcomes must settle first. Saved work has not been replayed.".into(),None);
        }
        return None;
    }
    app.chat_widget.termux_prepare_account_handoff();
    if app.chat_widget.has_active_modal() {
        return None;
    }
    let tx = app.app_event_tx.clone();
    if trigger == "quota" {
        app.chat_widget.add_info_message("Quota exhausted; checking enrolled accounts. The saved task remains paused until a safe destination is ready.".into(),None);
    }
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
        tx.send(AppEvent::AutoQuotaReady {
            thread,
            turn,
            trigger,
            view,
        });
    });
    None
}
pub(super) async fn automatic_ready(
    app: &mut App,
    server: &mut crate::app_server_session::AppServerSession,
    thread: ThreadId,
    turn: String,
    trigger: String,
    view: Option<Value>,
) -> Option<super::profile_manager::ProfileReentry> {
    if view
        .as_ref()
        .is_some_and(|v| v["rotation"]["enabled"] == true)
        && !app.quota_switch.session_off
    {
        app.chat_widget.termux_prepare_account_handoff();
    }
    if app.quota_switch.session_off
        || app.quota_switch.visible
        || app.quota_switch.handled.as_deref() != Some(&turn)
        || app.chat_widget.thread_id() != Some(thread)
        || app.primary_thread_id != Some(thread)
        || app.chat_widget.config_ref().model_provider_id != "openai"
        || app.chat_widget.has_active_modal()
        || !app.chat_widget.composer_is_empty()
        || !app.dynamic_tool_tasks.is_empty()
        || app.quota_switch.uncertain
        || !app.quota_switch.pending.is_empty()
        || app
            .agent_navigation
            .ordered_threads()
            .iter()
            .any(|(id, entry)| *id != thread && entry.is_running)
        || !app.chat_widget.can_change_working_directory(thread)
    {
        return None;
    }
    let model = app.chat_widget.config_ref().model.clone()?;
    let Some(view) = view else {
        app.chat_widget.add_error_message(
            "Automatic switching paused: usage refresh failed; work remains saved.".into(),
        );
        return None;
    };
    if view["rotation"]["enabled"] != true {
        return None;
    }
    if view["selected_model"].as_str() != Some(&model) {
        return None;
    }
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
        let valid = json!({"pid":123,"source":"default","target":"next"});
        assert!(
            notice(&valid.to_string(), 123)
                .unwrap()
                .contains("default -> next")
        );
        assert!(notice(&valid.to_string(), 124).is_none());
        assert!(
            notice(
                &json!({"pid":123,"source":"bad\nrow","target":"next"}).to_string(),
                123
            )
            .is_none()
        );
    }
    #[test]
    fn profile_manager_quota_rows_render_and_settings_actions_use_manager_policy() {
        use crate::app_event_sender::AppEventSender;
        use crate::bottom_pane::ListSelectionView;
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
        assert!(screen.contains("Auto:"), "{screen}");
        let settings = picker(&value, true, false);
        assert!(
            settings.tabs[1]
                .items
                .iter()
                .any(|i| i.name == "[ ] default")
        );
        // Use the real upstream renderer: long account IDs and descriptions must
        // not increase list height on a narrow phone terminal.
        let mut custom = value.clone();
        custom["rotation"]["policy"] = json!("custom");
        let long_id = "account-with-a-long-but-valid-identity-0123456789012345678901234";
        assert_eq!(long_id.len(), 64, "fixture exercises the accepted ID limit");
        custom["rows"][0]["id"] = json!(long_id);
        custom["rows"][0]["summary"] = json!("5h 72% · weekly 12% · 계정 사용량 확인 · ready");
        custom["rotation"]["accounts"] = json!([long_id]);
        custom["rotation"]["order"] = json!([long_id]);
        assert!(decode(custom.to_string().as_bytes()).is_some());
        assert_eq!(picker(&custom, true, false).tabs.len(), 3);
        for tab_index in 0..3 {
            let mut params = picker(&custom, true, false);
            let tab = params.tabs.remove(tab_index);
            params.tabs.clear();
            params.items = tab.items;
            params.title = None;
            params.subtitle = None;
            params.footer_note = None;
            params.footer_hint = None;
            params.is_searchable = false;
            params.header_gap = 0;
            params.max_visible_rows = 64;
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let view = ListSelectionView::new(
                params,
                AppEventSender::new(tx),
                RuntimeKeymap::defaults().list,
            );
            for width in [24, 32, 40, 80] {
                assert_eq!(
                    view.desired_height(width),
                    view.desired_height(100),
                    "tab {tab_index}, width {width}"
                );
                let rect = Rect::new(0, 0, width, view.desired_height(width));
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
                if width == 24 {
                    if tab_index == 0 {
                        assert!(
                            screen.contains("Auto: off"),
                            "setting value clipped: {screen}"
                        );
                    } else {
                        assert!(screen.contains('…'), "tab {tab_index}: {screen}");
                    }
                }
            }
        }
        // Display truncation never becomes an account-policy identifier.
        let mut accounts = picker(&custom, true, false).tabs.remove(1);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        accounts.items.remove(0).actions[0](&AppEventSender::new(tx));
        assert!(
            matches!(rx.try_recv().unwrap(), AppEvent::EditQuotaPolicy(value) if value["accounts"] == json!([]))
        );
        let mut bad = value;
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
    #[tokio::test]
    async fn profile_manager_quota_refresh_retains_selection_and_one_escape_closes_the_only_popup()
    {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let (mut app, _events, _ops) = super::super::tests::make_test_app_with_channels().await;
        let mut value = json!({"schema":"codex-manager-profiles-v2","current":"default","rows":[{"id":"default","summary":"5h 80%"},{"id":"next","summary":"5h 60%"}],"rotation":{"enabled":false,"accounts":[]}});
        app.quota_switch.visible = true;
        app.quota_switch.view = Some(value.clone());
        render(&mut app, false);
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(
            app.chat_widget.selected_index_for_active_view(VIEW_ID),
            Some(1)
        );
        value["rows"][1]["summary"] = json!("5h 50%");
        ready(&mut app, 0, Some(value.clone()));
        assert_eq!(
            app.chat_widget.selected_index_for_active_view(VIEW_ID),
            Some(1)
        );
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.chat_widget.has_active_modal());
        close(&mut app);
        ready(&mut app, 0, Some(value.clone()));
        assert!(!app.chat_widget.has_active_modal());
        app.quota_switch.settings = true;
        app.quota_switch.visible = true;
        app.quota_switch.view = Some(value.clone());
        render(&mut app, false);
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let generation = app.quota_switch.generation;
        ready(&mut app, generation, Some(value));
        let screen = crate::chatwidget::tests::helpers::render_bottom_popup(&app.chat_widget, 80);
        assert!(
            screen.contains("[ ] next"),
            "account tab lost on refresh: {screen}"
        );
        assert_eq!(
            app.chat_widget.selected_index_for_active_view(VIEW_ID),
            Some(1)
        );
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            app.chat_widget.has_active_modal(),
            "saving a setting must preserve its tab"
        );
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.chat_widget.has_active_modal());
    }
}
