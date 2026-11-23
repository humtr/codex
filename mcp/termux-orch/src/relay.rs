//! Deterministic `Runtime (R)` daemon (`termux-orch relay`).
//!
//! Scans active `Work`s in `~/.local/state/codex/orchestration/jobs/`:
//! 1. Observes running `Worker` / `Verifier` participants via existing owners
//!    (`codex app-server daemon` or `chatgpt-provider /control/observe`).
//! 2. Appends idempotent settlement events into `Typed Inbox` (`events.jsonl`).
//! 3. Evaluates declarative `Wait-Set` triggers (`all_of`, `any_of`, `on_user_event`,
//!    `deadline_secs`) under hard Circuit Breakers (`max_wakeups`, `max_wall_secs`,
//!    cooldown).
//! 4. Delivers wake-ups and `Front` reports (`inject_and_notify` default or
//!    `notify_only`) with server `stream_status` collision guard + exponential
//!    backoff + `termux-notification`, advancing `consumed_seq` only on confirmed
//!    dispatch.

use crate::ledger::{now_unix, truncate_chars, InboxEvent, Ledger, PendingDelivery, WorkRecord};
use crate::owners::OwnerClient;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone)]
pub struct TickSummary {
    pub works_scanned: usize,
    pub participants_settled: usize,
    pub wait_sets_fired: usize,
    pub deliveries_sent: usize,
    pub deliveries_deferred: usize,
    pub budget_trips: usize,
}

pub struct RelayEngine {
    ledger: Ledger,
    owners: OwnerClient,
}

impl RelayEngine {
    pub fn new(ledger: Ledger, owners: OwnerClient) -> Self {
        Self { ledger, owners }
    }

    pub fn tick_once(&self) -> TickSummary {
        let mut summary = TickSummary::default();
        let Ok(ids) = self.ledger.list_work_ids() else {
            return summary;
        };
        for work_id in ids {
            if let Ok(part) = self.process_work(&work_id) {
                summary.works_scanned += 1;
                summary.participants_settled += part.participants_settled;
                summary.wait_sets_fired += part.wait_sets_fired;
                summary.deliveries_sent += part.deliveries_sent;
                summary.deliveries_deferred += part.deliveries_deferred;
                summary.budget_trips += part.budget_trips;
            }
        }
        summary
    }

    fn process_work(&self, work_id: &str) -> Result<TickSummary, String> {
        let _lock = self.ledger.lock_work(work_id).map_err(|e| e.to_string())?;
        let mut work = self.ledger.load_work(work_id).map_err(|e| e.to_string())?;
        let mut summary = TickSummary::default();
        if work.is_terminal()
            && work.pending_deliveries.is_empty()
            && (work.lead_replay.is_none() || work.state == "cancelled")
        {
            return Ok(summary);
        }

        let mut dirty = false;
        let now = now_unix();

        // 1. Observe running participants (only if work is not cancelled/completed).
        if !work.is_terminal() {
            let mut newly_settled: Vec<(usize, String, String)> = Vec::new();
            for (idx, p) in work.participants.iter().enumerate() {
                if p.is_terminal() {
                    continue;
                }
                match p.backend.as_str() {
                    "codex" => {
                        if let Ok(obs) = self
                            .owners
                            .codex_observe_thread(p.profile.as_deref(), &p.ref_id)
                        {
                            if !obs.is_active && obs.status_type != "unknown" {
                                let next_state = if obs.status_type == "systemError" {
                                    "failed"
                                } else {
                                    "completed"
                                };
                                newly_settled.push((
                                    idx,
                                    next_state.to_owned(),
                                    obs.last_assistant_text,
                                ));
                            }
                        }
                    }
                    "chatgpt" => {
                        if let Ok(obs) = self.owners.chatgpt_observe(&p.ref_id) {
                            if !obs.generating && obs.state == "settled" {
                                newly_settled.push((
                                    idx,
                                    "completed".to_owned(),
                                    obs.assistant_text,
                                ));
                            } else if obs.state == "error" {
                                let detail =
                                    obs.error.unwrap_or_else(|| "chatgpt_turn_error".to_owned());
                                newly_settled.push((idx, "failed".to_owned(), detail));
                            }
                        }
                    }
                    "shell" => {
                        let result =
                            crate::shell_job::ShellJob::new(&self.ledger, work_id, &p.ref_id)
                                .and_then(|job| job.observe(p.created_at_unix));
                        match result {
                            Ok(Some((state, summary))) => newly_settled.push((idx, state, summary)),
                            Ok(None) => (),
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    std::io::ErrorKind::InvalidData
                                        | std::io::ErrorKind::InvalidInput
                                ) =>
                            {
                                newly_settled.push((
                                    idx,
                                    "failed".to_owned(),
                                    format!("invalid shell receipt: {e}"),
                                ));
                            }
                            Err(_) => (),
                        }
                    }
                    _ => {}
                }
            }

            for (idx, next_state, text) in newly_settled {
                let p = &mut work.participants[idx];
                p.state = next_state.clone();
                p.result_summary = Some(truncate_chars(&text, 1500));
                p.updated_at_unix = now;

                let pid = p.participant_id.clone();
                let ref_id = p.ref_id.clone();
                let role = p.role.clone();
                let slug = p.slug.clone();
                let event_kind = if role == "verifier" {
                    if next_state == "completed" {
                        "verifier.passed"
                    } else {
                        "verifier.rejected"
                    }
                } else if next_state == "completed" {
                    "worker.completed"
                } else {
                    "worker.failed"
                };
                let ev_id = format!("settle:{pid}:{ref_id}");
                let ev_summary = format!(
                    "{role} {pid} ({slug}, {ref_id}) {next_state}: {}",
                    truncate_chars(&text, 400)
                );
                self.ledger
                    .append_event_if_absent(
                        &mut work,
                        &ev_id,
                        event_kind,
                        &pid,
                        &ev_summary,
                        json!({
                            "participant_id": pid,
                            "role": role,
                            "ref_id": ref_id,
                            "state": next_state
                        }),
                    )
                    .map_err(|e| e.to_string())?;
                summary.participants_settled += 1;
                dirty = true;
            }
        }

        if dirty {
            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
            dirty = false;
        }
        let rollover = crate::rollover::Rollover::new(&self.ledger, &self.owners);
        if !work.is_terminal() && work.lead_rollover.is_some() {
            let _ = rollover.advance(&mut work);
            return Ok(summary);
        }
        if work.state != "cancelled" && work.lead_replay.is_some() {
            let _ = rollover.replay(&mut work);
            return Ok(summary);
        }
        if !work.is_terminal() {
            if let Some(reason) = rollover.automatic_reason(&work) {
                let _ = rollover.start(&mut work, &reason, None);
                if work.lead_rollover.is_some() || work.lead_replay.is_some() {
                    return Ok(summary);
                }
            }
        }

        // 2. Evaluate armed Wait-Set.
        if !work.is_terminal() {
            if let Some(ws) = work.wait_set.clone().filter(|w| w.armed) {
                let unconsumed = self
                    .ledger
                    .read_events(&work.work_id, work.consumed_seq, 64)
                    .unwrap_or_default();
                let reason = evaluate_wait_set(&work, &ws, &unconsumed, now);
                if let Some(trigger_reason) = reason {
                    let wall_exceeded =
                        now.saturating_sub(work.created_at_unix) > work.budget.max_wall_secs;
                    let wakeups_exceeded = work.budget.wakeups_used >= work.budget.max_wakeups;
                    if wall_exceeded || wakeups_exceeded {
                        if let Some(w) = work.wait_set.as_mut() {
                            w.armed = false;
                        }
                        work.state = "waiting_user".to_owned();
                        work.updated_at_unix = now;
                        let cause = if wakeups_exceeded {
                            format!("max_wakeups ({}) exhausted", work.budget.max_wakeups)
                        } else {
                            format!("max_wall_secs ({}s) exceeded", work.budget.max_wall_secs)
                        };
                        let budget_ev_id = format!("budget_trip:{}", work.budget.wakeups_used);
                        let _ = self.ledger.append_event_if_absent(
                            &mut work,
                            &budget_ev_id,
                            "budget.exhausted",
                            "runtime",
                            &cause,
                            json!({ "cause": cause }),
                        );
                        let budget_del_id = format!("budget:{}", work.last_seq);
                        let budget_title = format!("[Work] {} — 예산 상한 도달", work.title);
                        let budget_payload = format!(
                            "[ORCH_BUDGET_EXHAUSTED] work_id={}\n사유: {cause}\n\n계속 진행하려면 `orch_call(budget_extend)`로 예산을 연장하거나 `orch_work(cancel)`로 종료해 주세요.",
                            work.work_id
                        );
                        enqueue_delivery_if_absent(
                            &mut work,
                            PendingDelivery {
                                delivery_id: budget_del_id,
                                target: "front".to_owned(),
                                kind: "budget_exhausted".to_owned(),
                                title: budget_title,
                                payload_text: budget_payload,
                                advance_seq_to: None,
                                notified: false,
                                attempts: 0,
                                next_attempt_unix: now,
                                last_error: None,
                                created_at_unix: now,
                            },
                        );
                        summary.budget_trips += 1;
                        dirty = true;
                    } else if now
                        >= work
                            .budget
                            .last_wakeup_unix
                            .saturating_add(work.budget.min_cooldown_secs)
                    {
                        let checkpoint = self
                            .ledger
                            .read_checkpoint(&work.work_id)
                            .ok()
                            .flatten()
                            .unwrap_or_default();
                        let wakeup_text =
                            build_wakeup_prompt(&work, &trigger_reason, &checkpoint, &unconsumed);
                        let target_seq = work.last_seq;
                        let target_kind = ws.wake_target.clone();
                        let wait_id = ws.wait_id.clone();

                        if let Some(w) = work.wait_set.as_mut() {
                            w.armed = false;
                        }
                        work.budget.wakeups_used += 1;
                        work.budget.last_wakeup_unix = now;
                        work.state = "running".to_owned();
                        work.updated_at_unix = now;

                        let wakeup_title = format!("[Work] {} — 라운드 깨우기", work.title);
                        enqueue_delivery_if_absent(
                            &mut work,
                            PendingDelivery {
                                delivery_id: format!("wakeup:{wait_id}:{target_seq}"),
                                target: target_kind,
                                kind: "wakeup".to_owned(),
                                title: wakeup_title,
                                payload_text: wakeup_text,
                                advance_seq_to: Some(target_seq),
                                notified: false,
                                attempts: 0,
                                next_attempt_unix: now,
                                last_error: None,
                                created_at_unix: now,
                            },
                        );
                        summary.wait_sets_fired += 1;
                        dirty = true;
                    }
                }
            }
        }

        // 3. Flush due pending deliveries.
        if !work.pending_deliveries.is_empty() {
            let mut remaining = Vec::new();
            let pending = std::mem::take(&mut work.pending_deliveries);
            for mut del in pending {
                if del.next_attempt_unix > now {
                    remaining.push(del);
                    continue;
                }
                match self.attempt_delivery(&work, &mut del) {
                    DeliveryOutcome::Delivered => {
                        if let Some(seq) = del.advance_seq_to {
                            if seq > work.consumed_seq {
                                work.consumed_seq = seq;
                            }
                        }
                        let ev_kind = if del.target == "front" {
                            "delivery.front_sent"
                        } else {
                            "wakeup.lead_sent"
                        };
                        let _ = self.ledger.append_event_if_absent(
                            &mut work,
                            &format!("delivered:{}", del.delivery_id),
                            ev_kind,
                            "runtime",
                            &format!("{} delivered ({})", del.kind, del.target),
                            json!({
                                "delivery_id": del.delivery_id,
                                "target": del.target,
                                "kind": del.kind
                            }),
                        );
                        summary.deliveries_sent += 1;
                        dirty = true;
                    }
                    DeliveryOutcome::Defer(reason) => {
                        del.attempts += 1;
                        let shift = (del.attempts.saturating_sub(1)).min(4);
                        let backoff = (5u64 << shift).min(60);
                        del.next_attempt_unix = now + backoff;
                        del.last_error = Some(truncate_chars(&reason, 240));
                        remaining.push(del);
                        summary.deliveries_deferred += 1;
                        dirty = true;
                    }
                }
            }
            work.pending_deliveries = remaining;
        }

        if dirty {
            work.updated_at_unix = now_unix();
            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
        }
        Ok(summary)
    }

    fn attempt_delivery(&self, work: &WorkRecord, del: &mut PendingDelivery) -> DeliveryOutcome {
        match del.target.as_str() {
            "front" => {
                if !del.notified {
                    let _ =
                        self.owners
                            .send_notification(&work.work_id, &del.title, &del.payload_text);
                    del.notified = true;
                }
                if work.delivery_mode == "notify_only" {
                    return DeliveryOutcome::Delivered;
                }
                let Some(front_chat) = work.front.latest_chat_id.as_deref() else {
                    return DeliveryOutcome::Delivered;
                };
                match self.owners.chatgpt_observe(front_chat) {
                    Ok(obs) if obs.generating => {
                        return DeliveryOutcome::Defer(format!(
                            "front_chat_busy({})",
                            obs.remote_stream_status.as_deref().unwrap_or("local")
                        ));
                    }
                    Err(e) => return DeliveryOutcome::Defer(e),
                    Ok(_) => {}
                }
                match self.owners.chatgpt_send(
                    Some(front_chat),
                    None,
                    &del.payload_text,
                    None,
                    None,
                    5_000,
                ) {
                    Ok(val) => {
                        let st = val.get("state").and_then(Value::as_str).unwrap_or("");
                        if st == "settled" || st == "in_progress" {
                            DeliveryOutcome::Delivered
                        } else {
                            DeliveryOutcome::Defer(format!("chatgpt_send_state:{st}"))
                        }
                    }
                    Err(e) => DeliveryOutcome::Defer(e),
                }
            }
            "lead" => {
                let Some(ref lead) = work.lead else {
                    return DeliveryOutcome::Defer("no_lead_bound".to_owned());
                };
                match lead.kind.as_str() {
                    "chatgpt" => {
                        match self.owners.chatgpt_observe(&lead.ref_id) {
                            Ok(obs) if obs.generating => {
                                return DeliveryOutcome::Defer("lead_chat_busy".to_owned());
                            }
                            Err(e) => return DeliveryOutcome::Defer(e),
                            Ok(_) => {}
                        }
                        match self.owners.chatgpt_send(
                            Some(&lead.ref_id),
                            None,
                            &del.payload_text,
                            lead.model.as_deref(),
                            lead.reasoning_effort.as_deref(),
                            5_000,
                        ) {
                            Ok(val) => {
                                let st = val.get("state").and_then(Value::as_str).unwrap_or("");
                                if st == "settled" || st == "in_progress" {
                                    DeliveryOutcome::Delivered
                                } else {
                                    DeliveryOutcome::Defer(format!("lead_send_state:{st}"))
                                }
                            }
                            Err(e) => DeliveryOutcome::Defer(e),
                        }
                    }
                    "codex" => {
                        match self
                            .owners
                            .codex_observe_thread(lead.profile.as_deref(), &lead.ref_id)
                        {
                            Ok(obs) if obs.is_active => {
                                return DeliveryOutcome::Defer("lead_codex_active".to_owned());
                            }
                            Err(e) => return DeliveryOutcome::Defer(e),
                            Ok(_) => {}
                        }
                        match self.owners.codex_dispatch_turn(
                            lead.profile.as_deref(),
                            Some(&lead.ref_id),
                            None,
                            &del.payload_text,
                            lead.model.as_deref(),
                            None,
                            None,
                        ) {
                            Ok(_) => DeliveryOutcome::Delivered,
                            Err(e) => DeliveryOutcome::Defer(e),
                        }
                    }
                    other => DeliveryOutcome::Defer(format!("unsupported_lead_kind:{other}")),
                }
            }
            other => DeliveryOutcome::Defer(format!("unsupported_delivery_target:{other}")),
        }
    }
}

enum DeliveryOutcome {
    Delivered,
    Defer(String),
}

pub fn enqueue_delivery_if_absent(work: &mut WorkRecord, del: PendingDelivery) {
    if !work
        .pending_deliveries
        .iter()
        .any(|d| d.delivery_id == del.delivery_id)
    {
        work.pending_deliveries.push(del);
    }
}

fn evaluate_wait_set(
    work: &WorkRecord,
    ws: &crate::ledger::WaitSet,
    unconsumed: &[InboxEvent],
    now: u64,
) -> Option<String> {
    if ws.on_user_event && unconsumed.iter().any(|e| e.kind.starts_with("user.")) {
        return Some("user_event".to_owned());
    }
    if !ws.all_of.is_empty() {
        let all_done = ws.all_of.iter().all(|pid| {
            work.participants
                .iter()
                .find(|p| p.participant_id == *pid)
                .is_some_and(|p| p.is_terminal())
        });
        if all_done {
            return Some(format!("barrier_all_of({})", ws.all_of.join(",")));
        }
    }
    if !ws.any_of.is_empty() {
        if let Some(hit) = ws.any_of.iter().find(|pid| {
            work.participants
                .iter()
                .find(|p| p.participant_id == **pid)
                .is_some_and(|p| p.is_terminal())
        }) {
            return Some(format!("barrier_any_of({hit})"));
        }
    }
    if let Some(dl) = ws.deadline_unix {
        if now >= dl {
            return Some("deadline_watchdog".to_owned());
        }
    }
    None
}

fn build_wakeup_prompt(
    work: &WorkRecord,
    reason: &str,
    checkpoint: &str,
    events: &[InboxEvent],
) -> String {
    let mut out = format!(
        "[ORCH_WAKEUP] work_id={} tier={} title=\"{}\" trigger={}\n",
        work.work_id, work.tier, work.title, reason
    );
    out.push_str(&format!(
        "Goal: {}\nBudget: wakeups {}/{}\n",
        truncate_chars(&work.goal, 400),
        work.budget.wakeups_used + 1,
        work.budget.max_wakeups
    ));
    if !checkpoint.trim().is_empty() {
        out.push_str("\n### Checkpoint\n");
        out.push_str(&truncate_chars(checkpoint.trim(), 2000));
        out.push('\n');
    }
    if !work.participants.is_empty() {
        out.push_str("\n### Participants\n");
        for p in &work.participants {
            out.push_str(&format!(
                "- {} [{} / {} / ref={}] state={}: {}\n",
                p.participant_id,
                p.role,
                p.backend,
                p.ref_id,
                p.state,
                p.result_summary
                    .as_deref()
                    .map(|s| truncate_chars(s, 300))
                    .unwrap_or_else(|| p.task_summary.clone())
            ));
        }
    }
    if !events.is_empty() {
        out.push_str("\n### Unconsumed Inbox Events\n");
        for ev in events.iter().take(20) {
            out.push_str(&format!(
                "- #{} [{}] ({}): {}\n",
                ev.seq,
                ev.kind,
                ev.actor,
                truncate_chars(&ev.summary, 300)
            ));
        }
    }
    out.push_str(
        "\nReview the results above. Continue the next round via `@TermuxCodex`/`@TermuxChatGPT`/`@TermuxShell` + `@TermuxOrch` (`orch_dispatch`), or report to the user via `orch_report`.",
    );
    out
}

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: libc::c_int) {
    STOP_FLAG.store(true, Ordering::Release);
}

pub fn run_relay_daemon(
    ledger: Ledger,
    owners: OwnerClient,
    bind_addr: &str,
    health_url_file: Option<&Path>,
    poll_ms: u64,
) -> Result<(), String> {
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as usize);
        libc::signal(libc::SIGINT, handle_signal as *const () as usize);
    }
    STOP_FLAG.store(false, Ordering::Release);

    let listener = TcpListener::bind(bind_addr)
        .map_err(|e| format!("failed to bind relay health listener on {bind_addr}: {e}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| format!("failed to read relay local_addr: {e}"))?;
    if !local_addr.ip().is_loopback() {
        return Err("termux-orch relay health bind must be loopback".to_owned());
    }
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set nonblocking listener: {e}"))?;

    let mut random = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut random))
        .map_err(|e| format!("cannot generate wake token: {e}"))?;
    let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let token_path = health_url_file.map(|url| url.with_file_name("wake.token"));
    if let (Some(url_file), Some(token_path)) = (health_url_file, &token_path) {
        if let Some(parent) = url_file.parent().filter(|p| !p.as_os_str().is_empty()) {
            crate::ledger::ensure_private_dir(parent).map_err(|e| e.to_string())?;
        }
        crate::ledger::write_atomic_private(token_path, token.as_bytes())
            .map_err(|e| e.to_string())?;
        crate::ledger::write_atomic_private(url_file, format!("http://{local_addr}\n").as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let health = Arc::new(RelayHealth::default());
    let server_health = Arc::clone(&health);
    let state_root_str = ledger.root().display().to_string();
    let (wake_tx, wake_rx) = mpsc::sync_channel(1);
    let health_handle = std::thread::spawn(move || {
        while !STOP_FLAG.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let Some(request) = read_http_head(&mut stream) else {
                        continue;
                    };
                    let mut lines = request.split("\r\n");
                    let mut route = lines.next().unwrap_or_default().split_ascii_whitespace();
                    let (status, ctype, body) = match (route.next(), route.next()) {
                        (Some("GET"), Some("/readyz")) => {
                            ("200 OK", "text/plain", "ready\n".to_owned())
                        }
                        (Some("GET"), Some("/healthz" | "/health")) => {
                            let body = json!({
                                "ok": true,
                                "service": "termux-orch-relay",
                                "version": env!("CARGO_PKG_VERSION"),
                                "ticks": server_health.ticks.load(Ordering::Acquire),
                                "wake_pushes": server_health.wake_pushes.load(Ordering::Acquire),
                                "last_wake_tick_ms": server_health.last_wake_tick_ms.load(Ordering::Acquire),
                                "state_root": state_root_str
                            })
                            .to_string();
                            ("200 OK", "application/json", body)
                        }
                        (Some("POST"), Some("/wake")) if token_path.is_some() => {
                            if authorized_wake(lines, &token) {
                                server_health.wake_pushes.fetch_add(1, Ordering::Release);
                                let _ = wake_tx.try_send(Instant::now());
                                (
                                    "202 Accepted",
                                    "application/json",
                                    r#"{"state":"accepted"}"#.to_owned(),
                                )
                            } else {
                                (
                                    "401 Unauthorized",
                                    "application/json",
                                    r#"{"error":"unauthorized"}"#.to_owned(),
                                )
                            }
                        }
                        _ => (
                            "404 Not Found",
                            "application/json",
                            r#"{"error":"not_found"}"#.to_owned(),
                        ),
                    };
                    let resp = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                    let _ = stream.write_all(resp.as_bytes());
                }
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    });

    let engine = RelayEngine::new(ledger, owners);
    let interval = Duration::from_millis(poll_ms.clamp(250, 60_000));
    let mut wake_requested = None::<Instant>;
    while !STOP_FLAG.load(Ordering::Acquire) {
        if let Some(requested) = wake_requested.take() {
            health
                .last_wake_tick_ms
                .store(requested.elapsed().as_millis() as u64, Ordering::Release);
        }
        let _ = engine.tick_once();
        health.ticks.fetch_add(1, Ordering::Release);
        let deadline = Instant::now() + interval;
        while !STOP_FLAG.load(Ordering::Acquire) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if let Ok(requested) = wake_rx.recv_timeout(remaining.min(Duration::from_millis(200))) {
                wake_requested = Some(requested);
                break;
            }
        }
    }

    let _ = health_handle.join();
    if let Some(url_file) = health_url_file {
        let _ = fs::remove_file(url_file);
    }
    Ok(())
}

#[derive(Default)]
struct RelayHealth {
    ticks: AtomicU64,
    wake_pushes: AtomicU64,
    last_wake_tick_ms: AtomicU64,
}

fn read_http_head(stream: &mut TcpStream) -> Option<String> {
    let deadline = Instant::now() + Duration::from_millis(250);
    let mut buffer = [0u8; 4096];
    let mut used = 0;
    while used < buffer.len() {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        stream.set_read_timeout(Some(remaining)).ok()?;
        let n = stream.read(&mut buffer[used..]).ok()?;
        if n == 0 {
            return None;
        }
        used += n;
        if let Some(end) = buffer[..used].windows(4).position(|w| w == b"\r\n\r\n") {
            return String::from_utf8(buffer[..end].to_vec()).ok();
        }
    }
    None
}

fn authorized_wake<'a>(headers: impl Iterator<Item = &'a str>, token: &str) -> bool {
    let mut auth = headers.filter_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then_some(value.trim())
    });
    match (auth.next(), auth.next()) {
        (Some(value), None) => value.strip_prefix("Bearer ").is_some_and(|given| {
            given.len() == token.len()
                && given
                    .bytes()
                    .zip(token.bytes())
                    .fold(0, |diff, (a, b)| diff | (a ^ b))
                    == 0
        }),
        _ => false,
    }
}
