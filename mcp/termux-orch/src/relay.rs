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

use crate::ledger::{
    generate_id, now_unix, truncate_chars, DeliveryDispatch, InboxEvent, Isolation, Ledger,
    Participant, PendingDelivery, StepFailure, Verdict, Verification, VerificationPolicy, WaitSet,
    WorkRecord,
};
use crate::owners::OwnerClient;
use serde_json::json;
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
    pub errors: usize,
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
            summary.errors += 1;
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
            } else {
                summary.errors += 1;
            }
        }
        summary
    }

    pub(crate) fn process_work(&self, work_id: &str) -> Result<TickSummary, String> {
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

        // A missing dispatch response may still represent accepted work.
        if !work.is_terminal() {
            if let Some(mut lead) = work.pending_lead.clone() {
                if lead.kind == "codex" && lead.ref_id.is_empty() {
                    work.pending_lead = None;
                    self.ledger
                        .append_event_if_absent(
                            &mut work,
                            &format!("dispatch-failed:{}", lead.dispatch_id.unwrap_or_default()),
                            "lead.dispatch_failed",
                            "runtime",
                            "Codex Lead dispatch stopped before turn/start",
                            json!({"generation":lead.generation}),
                        )
                        .map_err(|error| error.to_string())?;
                    dirty = true;
                } else if let Some((ref_id, turn_id)) = self.owners.reconcile_model_dispatch(
                    &lead.kind,
                    lead.profile.as_deref(),
                    &lead.ref_id,
                    lead.dispatch_id.as_deref().unwrap_or_default(),
                    work.account_id.as_deref(),
                    work.project_id.as_deref(),
                ) {
                    lead.ref_id = ref_id;
                    lead.turn_id = turn_id;
                    work.bind_lead(lead);
                    self.ledger.save_work(&work).map_err(|e| e.to_string())?;
                }
            }
            for participant in &mut work.participants {
                if participant.state != "dispatching" {
                    continue;
                }
                let Some(id) = participant.dispatch_id.as_deref() else {
                    continue;
                };
                if let Some((ref_id, turn_id)) = self.owners.reconcile_model_dispatch(
                    &participant.backend,
                    participant.profile.as_deref(),
                    &participant.ref_id,
                    id,
                    work.account_id.as_deref(),
                    work.project_id.as_deref(),
                ) {
                    participant.ref_id = ref_id;
                    participant.turn_id = turn_id;
                    participant.state = "running".to_owned();
                    dirty = true;
                }
            }
            if work.state == "cancelling" {
                self.progress_cancellation(&mut work)?;
            }
            let mut newly_settled: Vec<(usize, String, String)> = Vec::new();
            for (idx, p) in work.participants.iter().enumerate() {
                // The dispatcher holds this lock and saves the thread UUID
                // before turn/start. An empty UUID cannot hide an accepted turn.
                if p.state == "dispatching" && p.backend == "codex" && p.ref_id.is_empty() {
                    newly_settled.push((
                        idx,
                        "failed".to_owned(),
                        "Codex dispatch stopped before turn/start".to_owned(),
                    ));
                    continue;
                }
                if !work.can_start()
                    || p.is_terminal()
                    || matches!(p.state.as_str(), "dispatching" | "starting")
                {
                    continue;
                }
                match p.backend.as_str() {
                    "codex" => {
                        if let Ok(obs) = self.owners.codex_observe_turn(
                            p.profile.as_deref(),
                            &p.ref_id,
                            p.turn_id.as_deref(),
                        ) {
                            let settled = match obs.turn_status.as_deref() {
                                Some("completed") => Some(("completed", obs.last_assistant_text)),
                                Some(status @ ("failed" | "interrupted")) => {
                                    Some(("failed", format!("Codex turn {status}")))
                                }
                                _ if obs.status_type == "systemError" => {
                                    Some(("failed", "Codex thread systemError".to_owned()))
                                }
                                _ => None,
                            };
                            if let Some((state, detail)) = settled {
                                newly_settled.push((idx, state.to_owned(), detail));
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

            // A worker settling in this tick changes the subject of an earlier review.
            newly_settled.sort_by_key(|(idx, _, _)| work.participants[*idx].role == "verifier");
            for (idx, mut next_state, mut text) in newly_settled {
                if let Some(tree) = &work.participants[idx].worktree {
                    match self.audit_worker_tree(&work, idx, tree) {
                        Ok(paths) => work.participants[idx].changed_paths = paths,
                        Err(error) => {
                            next_state = "failed".to_owned();
                            text = error;
                        }
                    }
                }
                let review = (work.participants[idx].role == "verifier").then(|| {
                    verifier_result(
                        &work.participants[idx],
                        &work.verification_subject(),
                        &next_state,
                        &text,
                    )
                });
                let p = &mut work.participants[idx];
                p.state = next_state.clone();
                p.result_summary = Some(truncate_chars(&text, 1500));
                p.updated_at_unix = now;

                let pid = p.participant_id.clone();
                let ref_id = p.ref_id.clone();
                let role = p.role.clone();
                let slug = p.slug.clone();
                let event_kind = match review {
                    Some((verdict, evidence, blocking_issues)) => {
                        if let Some(verification) = p.verification.as_mut() {
                            verification.verdict = Some(verdict);
                            verification.evidence = evidence;
                            verification.blocking_issues = blocking_issues;
                        }
                        match verdict {
                            Verdict::Pass => "verifier.passed",
                            Verdict::Fail => "verifier.rejected",
                            Verdict::Inconclusive => "verifier.inconclusive",
                        }
                    }
                    None if next_state == "completed" => "worker.completed",
                    None => "worker.failed",
                };
                if role == "verifier" {
                    self.ledger
                        .append_event_if_absent(
                            &mut work,
                            &format!("verifier-settled:{pid}:{ref_id}"),
                            "verifier.settled",
                            &pid,
                            "Verifier turn settled",
                            json!({"state":next_state,"ref_id":ref_id}),
                        )
                        .map_err(|error| error.to_string())?;
                }
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
        if work.can_start() {
            if let Some(wait) = work
                .wait_set
                .clone()
                .filter(|w| w.armed && w.then_verify_shell.is_some())
            {
                self.continue_with_shell(&mut work, &wait, now)?;
            }
        }
        if work.can_start() && work.state != "waiting_user" && !work.steps.is_empty() {
            self.process_steps(&mut work, now)?;
        }
        if work.can_start()
            && work.state != "waiting_user"
            && work.steps.is_empty()
            && work.verification_policy != VerificationPolicy::None
            && work.participants.iter().all(Participant::is_terminal)
        {
            let last = work
                .participants
                .iter()
                .rev()
                .find(|p| p.role == "verifier");
            if let Some((verdict, detail)) = last.and_then(|p| {
                let verdict = p.verification.as_ref()?.verdict?;
                Some((
                    verdict,
                    p.result_summary
                        .clone()
                        .unwrap_or_else(|| "No review result".into()),
                ))
            }) {
                let disposition = match verdict {
                    Verdict::Fail => Some("failed_verification"),
                    Verdict::Inconclusive => Some("waiting_user"),
                    Verdict::Pass => None,
                };
                if let Some(state) = disposition {
                    self.settle_plan(&mut work, state, &detail, "front")?;
                }
            }
            if work.can_start()
                && work.state != "waiting_user"
                && !work
                    .participants
                    .iter()
                    .any(|p| p.role == "verifier" && p.backend != "shell")
            {
                if let Some(cwd) = work.review_cwd.clone() {
                    self.start_plan_review(&mut work, &cwd, now)?;
                }
            }
        }
        let rollover = crate::rollover::Rollover::new(&self.ledger, &self.owners);
        if work.can_start() && work.pending_lead.is_none() && work.lead_rollover.is_some() {
            rollover
                .advance(&mut work)
                .map_err(|e| format!("lead rollover advance failed: {e}"))?;
            return Ok(summary);
        }
        if !matches!(work.state.as_str(), "cancelling" | "cancelled") && work.lead_replay.is_some()
        {
            rollover
                .replay(&mut work)
                .map_err(|e| format!("lead replay failed: {e}"))?;
            return Ok(summary);
        }
        if work.can_start() && work.pending_lead.is_none() {
            if let Some(reason) = rollover.automatic_reason(&work) {
                rollover
                    .start(&mut work, &reason, None)
                    .map_err(|e| format!("automatic lead rollover failed: {e}"))?;
                if work.lead_rollover.is_some() || work.lead_replay.is_some() {
                    return Ok(summary);
                }
            }
        }

        // 2. Evaluate armed Wait-Set.
        if work.can_start() && work.pending_lead.is_none() {
            // Only one delivery may own the next Inbox prefix. Otherwise a
            // newly armed Wait-Set can advance past an earlier deferred wake.
            if let Some(ws) = work.wait_set.clone().filter(|w| {
                w.armed
                    && w.then_verify_shell.is_none()
                    && !work.pending_deliveries.iter().any(|d| d.kind == "wakeup")
            }) {
                let user_event = ws.on_user_event
                    && self
                        .ledger
                        .has_user_event_after(&work.work_id, work.consumed_seq)
                        .map_err(|e| e.to_string())?;
                let reason = evaluate_wait_set(&work, &ws, user_event, now);
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
                                dispatch: None,
                                receipts_tracked: true,
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
                        let mut unconsumed = self
                            .ledger
                            .read_first_events(&work.work_id, work.consumed_seq, 20)
                            .map_err(|e| e.to_string())?;
                        let next = work.consumed_seq.saturating_add(1);
                        let prefix_len = unconsumed
                            .iter()
                            .enumerate()
                            .take_while(|(offset, event)| {
                                event.seq == next.saturating_add(*offset as u64)
                            })
                            .count();
                        unconsumed.truncate(prefix_len);
                        let wakeup_text = loop {
                            let text = build_wakeup_prompt(
                                &work,
                                &trigger_reason,
                                &checkpoint,
                                &unconsumed,
                            );
                            if text.len() <= (32 * 1024 - 256) || unconsumed.is_empty() {
                                break text;
                            }
                            unconsumed.pop();
                        };
                        let target_seq = unconsumed
                            .last()
                            .map_or(work.consumed_seq, |event| event.seq);
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
                        let advance_seq_to = (!(ws.wake_target == "front"
                            && work.delivery_mode == "notify_only"))
                            .then_some(target_seq);
                        enqueue_delivery_if_absent(
                            &mut work,
                            PendingDelivery {
                                delivery_id: format!("wakeup:{wait_id}:{target_seq}"),
                                target: target_kind,
                                kind: "wakeup".to_owned(),
                                title: wakeup_title,
                                payload_text: wakeup_text,
                                advance_seq_to,
                                dispatch: None,
                                receipts_tracked: true,
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

        if dirty {
            work.updated_at_unix = now_unix();
            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
        }
        // Persist each dispatch before contacting its owner, and settle it
        // before starting another delivery.
        let mut index = 0;
        while index < work.pending_deliveries.len() {
            let mut delivery = work.pending_deliveries[index].clone();
            if delivery.next_attempt_unix > now {
                index += 1;
                continue;
            }
            match self.attempt_delivery(&mut work, index, &mut delivery)? {
                DeliveryOutcome::Delivered => {
                    if let Some(seq) = delivery.advance_seq_to {
                        work.consumed_seq = work.consumed_seq.max(seq);
                    }
                    let kind = if delivery.target == "front" {
                        "delivery.front_sent"
                    } else {
                        "wakeup.lead_sent"
                    };
                    self.ledger.append_event_if_absent(
                        &mut work,
                        &format!("delivered:{}", delivery.delivery_id),
                        kind,
                        "runtime",
                        &format!("{} delivered ({})", delivery.kind, delivery.target),
                        json!({"delivery_id": delivery.delivery_id, "target": delivery.target, "kind": delivery.kind}),
                    ).map_err(|e| e.to_string())?;
                    work.pending_deliveries.remove(index);
                    summary.deliveries_sent += 1;
                }
                DeliveryOutcome::Defer(reason) => {
                    delivery.attempts = delivery.attempts.saturating_add(1);
                    let shift = delivery.attempts.saturating_sub(1).min(4);
                    let backoff = (5u64 << shift).min(60);
                    delivery.next_attempt_unix = now + backoff;
                    delivery.last_error = Some(truncate_chars(&reason, 240));
                    work.pending_deliveries[index] = delivery;
                    index += 1;
                    summary.deliveries_deferred += 1;
                }
            }
            work.updated_at_unix = now_unix();
            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
        }
        Ok(summary)
    }

    fn attempt_delivery(
        &self,
        work: &mut WorkRecord,
        index: usize,
        delivery: &mut PendingDelivery,
    ) -> Result<DeliveryOutcome, String> {
        use DeliveryOutcome::{Defer, Delivered};
        if !delivery.receipts_tracked {
            // Older wakeups could contain only a fragment of the acknowledged range.
            delivery.advance_seq_to = None;
            if delivery.attempts != 0
                && delivery.dispatch.is_none()
                && !delivery.last_error.as_deref().is_some_and(|error| {
                    error.starts_with("front_chat_busy(")
                        || matches!(
                            error,
                            "lead_chat_busy" | "lead_codex_active" | "no_lead_bound"
                        )
                })
            {
                return Ok(Defer("legacy_delivery_receipt_unconfirmed".to_owned()));
            }
            delivery.receipts_tracked = true;
        }
        if delivery.target == "front" {
            if !delivery.notified {
                delivery.notified = self.owners.send_notification(
                    &work.work_id,
                    &delivery.title,
                    &delivery.payload_text,
                );
            }
            if delivery.dispatch.is_none()
                && (work.delivery_mode == "notify_only" || work.front.latest_chat_id.is_none())
            {
                delivery.advance_seq_to = None;
                return Ok(if delivery.notified {
                    Delivered
                } else {
                    Defer("front_notification_failed".to_owned())
                });
            }
        }
        if let Some(dispatch) = &delivery.dispatch {
            let marker = format!("[ORCH_DELIVERY {}]", dispatch.lane_id);
            let confirmation = match dispatch.backend.as_str() {
                "chatgpt" => {
                    let account_matches = match work.account_id.as_deref() {
                        Some(account) => self
                            .owners
                            .chatgpt_account_id()
                            .is_ok_and(|actual| actual == account),
                        None => false,
                    };
                    if !account_matches {
                        return Ok(Defer("delivery_account_unverified".to_owned()));
                    }
                    self.owners.chatgpt_has_delivery_marker(
                        &dispatch.ref_id,
                        if delivery.target == "lead" {
                            work.project_id.as_deref()
                        } else {
                            None
                        },
                        &marker,
                    )
                }
                "codex" => self.owners.codex_has_delivery_marker(
                    dispatch.profile.as_deref(),
                    &dispatch.ref_id,
                    &marker,
                ),
                _ => return Ok(Defer("unsupported_delivery_backend".to_owned())),
            };
            return Ok(match confirmation {
                Ok(true) => Delivered,
                Ok(false) => Defer("delivery_receipt_unconfirmed".to_owned()),
                Err(error) => Defer(error),
            });
        }

        let lead = match delivery.target.as_str() {
            "front" => None,
            "lead" => match &work.lead {
                Some(lead) => Some(lead.clone()),
                None => return Ok(Defer("no_lead_bound".to_owned())),
            },
            _ => return Ok(Defer("unsupported_delivery_target".to_owned())),
        };
        let (backend, ref_id, profile) = match &lead {
            Some(lead) => (
                lead.kind.as_str(),
                lead.ref_id.clone(),
                lead.profile.clone(),
            ),
            None => (
                "chatgpt",
                work.front
                    .latest_chat_id
                    .clone()
                    .expect("front target checked"),
                None,
            ),
        };
        if backend == "chatgpt" && work.account_id.is_none() {
            return Ok(Defer("delivery_account_unverified".to_owned()));
        }
        let ready = match backend {
            "chatgpt" => self.owners.chatgpt_observe(&ref_id).and_then(|obs| {
                if obs.generating {
                    Err(format!(
                        "{}_chat_busy({})",
                        delivery.target,
                        obs.remote_stream_status.as_deref().unwrap_or("local")
                    ))
                } else {
                    Ok(())
                }
            }),
            "codex" => self
                .owners
                .codex_observe_thread(profile.as_deref(), &ref_id)
                .and_then(|obs| {
                    if obs.is_active {
                        Err("lead_codex_active".to_owned())
                    } else {
                        Ok(())
                    }
                }),
            _ => return Ok(Defer("unsupported_delivery_backend".to_owned())),
        };
        if let Err(error) = ready {
            return Ok(Defer(error));
        }
        let lane_id = generate_id("delivery");
        let marker = format!("[ORCH_DELIVERY {lane_id}]");
        delivery.dispatch = Some(DeliveryDispatch {
            backend: backend.to_owned(),
            ref_id: ref_id.clone(),
            profile: profile.clone(),
            lane_id: lane_id.clone(),
        });
        work.pending_deliveries[index] = delivery.clone();
        self.ledger.save_work(work).map_err(|e| e.to_string())?;
        let prompt = if delivery.target == "lead" && backend == "chatgpt" {
            crate::rollover::bounded_payload(&format!("{marker}\n{}", delivery.payload_text))
        } else {
            format!("{marker}\n{}", delivery.payload_text)
        };
        Ok(match backend {
            "chatgpt" => match self.owners.chatgpt_send_pinned(
                work,
                lead.as_ref(),
                Some(&ref_id),
                &lane_id,
                &prompt,
            ) {
                Ok(receipt) if crate::rollover::rejected_before_send(&receipt) => {
                    delivery.dispatch = None;
                    Defer(
                        receipt["error"]
                            .as_str()
                            .unwrap_or("delivery_rejected")
                            .to_owned(),
                    )
                }
                Ok(receipt)
                    if receipt["chat_id"] == ref_id
                        && matches!(receipt["state"].as_str(), Some("settled" | "in_progress")) =>
                {
                    Delivered
                }
                Ok(_) => Defer("delivery_receipt_unconfirmed".to_owned()),
                Err(error) => Defer(error),
            },
            "codex" => match self.owners.codex_dispatch_turn(
                profile.as_deref(),
                Some(&ref_id),
                None,
                &prompt,
                lead.as_ref().and_then(|lead| lead.model.as_deref()),
                None,
                lead.as_ref()
                    .and_then(|lead| lead.reasoning_effort.as_deref()),
                None,
                |_| Ok(()),
            ) {
                Ok((thread, Some(_))) if thread == ref_id => Delivered,
                Ok(_) => Defer("delivery_receipt_unconfirmed".to_owned()),
                Err(error) => Defer(error),
            },
            _ => unreachable!("unsupported backend rejected before dispatch"),
        })
    }
    fn progress_cancellation(&self, work: &mut WorkRecord) -> Result<(), String> {
        for participant in work.participants.iter_mut().filter(|p| !p.is_terminal()) {
            let stopped = if participant.backend == "shell" {
                crate::shell_job::ShellJob::new(&self.ledger, &work.work_id, &participant.ref_id)
                    .and_then(|job| job.confirmed_exit())
                    .unwrap_or(false)
            } else {
                self.stop_model_launch(
                    &participant.backend,
                    &participant.ref_id,
                    participant.profile.as_deref(),
                    participant.dispatch_id.as_deref(),
                    participant.turn_id.as_deref(),
                    work.account_id.as_deref(),
                )
            };
            if stopped {
                participant.state = "cancelled".to_owned();
                participant.updated_at_unix = now_unix();
            }
        }
        let lead_settled = work.pending_lead.is_none()
            && work.lead.as_ref().is_none_or(|lead| {
                self.stop_model_launch(
                    &lead.kind,
                    &lead.ref_id,
                    lead.profile.as_deref(),
                    lead.dispatch_id.as_deref(),
                    lead.turn_id.as_deref(),
                    work.account_id.as_deref(),
                )
            });
        if lead_settled && work.participants.iter().all(Participant::is_terminal) {
            work.state = "cancelled".to_owned();
            self.ledger
                .append_event_if_absent(
                    work,
                    "work:cancelled",
                    "work.cancelled",
                    "runtime",
                    "Work cancellation settled",
                    json!({}),
                )
                .map_err(|e| e.to_string())?;
        }
        self.ledger.save_work(work).map_err(|e| e.to_string())
    }

    fn stop_model_launch(
        &self,
        backend: &str,
        ref_id: &str,
        profile: Option<&str>,
        dispatch_id: Option<&str>,
        turn_id: Option<&str>,
        account: Option<&str>,
    ) -> bool {
        // A pre-existing model session has no Work-owned launch to stop.
        let Some(dispatch) = dispatch_id else {
            return true;
        };
        match backend {
            "codex" => {
                let Some(turn) = turn_id else { return false };
                let Ok(observed) = self.owners.codex_observe_turn(profile, ref_id, Some(turn))
                else {
                    return false;
                };
                match observed.turn_status.as_deref() {
                    Some("completed" | "failed" | "interrupted") => true,
                    Some("inProgress") => {
                        let _ = self.owners.codex_interrupt_turn(profile, ref_id, turn);
                        false
                    }
                    _ => false,
                }
            }
            "chatgpt"
                if account.is_some_and(|pinned| {
                    self.owners
                        .chatgpt_account_id()
                        .is_ok_and(|actual| actual == pinned)
                }) =>
            {
                let Ok(observed) = self.owners.chatgpt_observe_lane(dispatch) else {
                    return false;
                };
                if ref_id.is_empty()
                    || observed["lane_id"] != dispatch
                    || observed["chat_id"] != ref_id
                {
                    return false;
                }
                if observed["generating"] == true {
                    let _ = self.owners.chatgpt_stop_lane(dispatch, ref_id);
                    false
                } else {
                    matches!(observed["state"].as_str(), Some("settled" | "error"))
                }
            }
            _ => false,
        }
    }

    fn audit_worker_tree(
        &self,
        work: &WorkRecord,
        index: usize,
        tree: &crate::workspace::WorkerTree,
    ) -> Result<Vec<String>, String> {
        let participant = &work.participants[index];
        let paths = tree.declared_changes(&participant.write_paths)?;
        let mut conflicts = tree.conflicting_paths(&paths)?;
        for other in &work.participants {
            if other.participant_id == participant.participant_id
                || other
                    .worktree
                    .as_ref()
                    .is_none_or(|other| other.repo != tree.repo)
            {
                continue;
            }
            for path in &paths {
                if other
                    .changed_paths
                    .iter()
                    .any(|old| crate::workspace::overlaps(path, old))
                {
                    conflicts.push(path.clone());
                }
            }
        }
        conflicts.sort();
        conflicts.dedup();
        if !conflicts.is_empty() {
            return Err(format!(
                "Changes in {} overlap another edit: {conflicts:?}",
                tree.path.display()
            ));
        }
        Ok(paths)
    }

    fn process_steps(&self, work: &mut WorkRecord, now: u64) -> Result<(), String> {
        let plan = work.steps.clone();
        let active = work.participants.iter().any(|p| !p.is_terminal());
        for step in &plan {
            let participant = work
                .participants
                .iter()
                .find(|p| p.step_id.as_deref() == Some(&step.step_id));
            if let Some(p) = participant.filter(|p| p.state == "failed") {
                if active {
                    return Ok(());
                }
                let detail = p
                    .result_summary
                    .clone()
                    .unwrap_or_else(|| format!("{} failed", step.step_id));
                let verdict = p.verification.as_ref().and_then(|v| v.verdict);
                let state = if verdict == Some(Verdict::Fail) {
                    "failed_verification"
                } else if step.on_failure == StepFailure::WakeLead
                    || verdict == Some(Verdict::Inconclusive)
                {
                    "waiting_user"
                } else {
                    "failed"
                };
                let target = if step.on_failure == StepFailure::WakeLead && work.lead.is_some() {
                    "lead"
                } else {
                    "front"
                };
                return self.settle_plan(work, state, &detail, target);
            }
        }
        if now.saturating_sub(work.created_at_unix) > work.budget.max_wall_secs {
            if !active {
                return self.settle_plan(
                    work,
                    "waiting_user",
                    "The step plan exceeded its time budget.",
                    "front",
                );
            }
            return Ok(());
        }
        for step in &plan {
            let participant = work
                .participants
                .iter()
                .position(|p| p.step_id.as_deref() == Some(&step.step_id));
            let idx = if let Some(index) = participant {
                if work.participants[index].state != "starting" {
                    continue;
                }
                index
            } else {
                if work.active_worker_count() >= work.budget.max_concurrent_workers {
                    continue;
                }
                let completed = |id: &str| {
                    work.participants
                        .iter()
                        .any(|p| p.step_id.as_deref() == Some(id) && p.state == "completed")
                };
                if !step.depends_on.iter().all(|id| completed(id))
                    || (step.auto_verify
                        && (active
                            || plan
                                .iter()
                                .filter(|s| !s.auto_verify)
                                .any(|s| !completed(&s.step_id))))
                {
                    continue;
                }
                let lease = self.ledger.lock_writers().map_err(|e| e.to_string())?;
                let workspace = crate::workspace::root(&step.cwd)?;
                let (write_workspace, worktree) = match step.isolation {
                    Isolation::Readonly => (None, None),
                    Isolation::SharedSerial => {
                        if self
                            .ledger
                            .writer_busy(&workspace)
                            .map_err(|e| e.to_string())?
                        {
                            continue;
                        }
                        (Some(workspace), None)
                    }
                    Isolation::Worktree => {
                        if work
                            .participants
                            .iter()
                            .filter(|p| !p.is_terminal())
                            .any(|p| {
                                p.worktree
                                    .as_ref()
                                    .is_some_and(|tree| tree.repo == workspace)
                                    && plan
                                        .iter()
                                        .find(|spec| p.step_id.as_deref() == Some(&spec.step_id))
                                        .is_some_and(|spec| {
                                            spec.write_paths.iter().any(|a| {
                                                step.write_paths
                                                    .iter()
                                                    .any(|b| crate::workspace::overlaps(a, b))
                                            })
                                        })
                            })
                        {
                            continue;
                        }
                        let tree = match crate::workspace::WorkerTree::plan(
                            &self.ledger,
                            &work.work_id,
                            &step.step_id,
                            &workspace,
                        ) {
                            Ok(tree) => tree,
                            Err(error) => {
                                return self.settle_plan(work, "waiting_user", &error, "front")
                            }
                        };
                        (None, Some(tree))
                    }
                };
                let role = if step.auto_verify {
                    "verifier"
                } else {
                    "worker"
                };
                let index = work.next_role_index(role);
                let id = if step.auto_verify {
                    format!("V{index}")
                } else {
                    format!("W{index}")
                };
                work.participants.push(Participant {
                    extra: Default::default(),
                    participant_id: id.clone(),
                    index,
                    role: role.to_owned(),
                    backend: "shell".to_owned(),
                    ref_id: id,
                    turn_id: None,
                    profile: None,
                    slug: step.step_id.clone(),
                    task_summary: step.step_id.clone(),
                    owner_scope: None,
                    write_workspace,
                    step_id: Some(step.step_id.clone()),
                    worktree,
                    changed_paths: vec![],
                    write_paths: step.write_paths.clone(),
                    call_id: None,
                    dispatch_id: None,
                    state: "starting".to_owned(),
                    result_summary: None,
                    verification: step.auto_verify.then(|| Verification {
                        subject: work.verification_subject(),
                        challenge: generate_id("verify"),
                        verdict: None,
                        evidence: vec![],
                        blocking_issues: vec![],
                    }),
                    created_at_unix: now,
                    updated_at_unix: now,
                });
                self.ledger.save_work(work).map_err(|e| e.to_string())?;
                drop(lease);
                work.participants.len() - 1
            };
            let p = &work.participants[idx];
            let job = crate::shell_job::ShellJob::new(&self.ledger, &work.work_id, &p.ref_id)
                .map_err(|e| e.to_string())?;
            if job.has_owner().map_err(|e| e.to_string())? {
                work.participants[idx].state = "running".to_owned();
                self.ledger.save_work(work).map_err(|e| e.to_string())?;
                continue;
            }
            let launch = (|| -> Result<(), String> {
                let cwd = if let Some(tree) = &p.worktree {
                    tree.prepare()?;
                    &tree.path
                } else {
                    &step.cwd
                };
                job.spawn(
                    self.ledger.root(),
                    &work.work_id,
                    &step.command,
                    cwd.to_str(),
                )
                .map_err(|e| e.to_string())
            })();
            if let Err(error) = launch {
                work.participants[idx].state = "failed".to_owned();
                work.participants[idx].result_summary = Some(error);
                self.ledger.save_work(work).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
        if work.participants.iter().any(|p| !p.is_terminal())
            || plan.iter().any(|step| {
                !work
                    .participants
                    .iter()
                    .any(|p| p.step_id.as_deref() == Some(&step.step_id) && p.state == "completed")
            })
        {
            return Ok(());
        }
        if work.verification_policy == VerificationPolicy::IndependentReview
            && !work
                .participants
                .iter()
                .any(|p| p.role == "verifier" && p.backend != "shell")
        {
            return self.start_plan_review(work, &plan[0].cwd, now);
        }
        let mut detail = String::new();
        for p in &work.participants {
            if let Some(result) = &p.result_summary {
                detail.push_str(&format!(
                    "{} {}: {}\n",
                    p.participant_id,
                    p.slug,
                    truncate_chars(result, 180)
                ));
            }
            if let Some(tree) = &p.worktree {
                detail.push_str(&format!(
                    "{} changes retained in {} ({})\n",
                    p.participant_id,
                    tree.path.display(),
                    tree.branch
                ));
            }
        }
        if let Some(v) = work.participants.iter().rev().find_map(|p| {
            p.verification
                .as_ref()
                .filter(|v| matches!(v.verdict, Some(Verdict::Fail | Verdict::Inconclusive)))
        }) {
            return self.settle_plan(
                work,
                if v.verdict == Some(Verdict::Fail) {
                    "failed_verification"
                } else {
                    "waiting_user"
                },
                &detail,
                "front",
            );
        }
        if work.verification_passed() {
            self.settle_plan(work, "completed", &detail, "front")?;
        } else {
            self.settle_plan(
                work,
                "waiting_user",
                "verification_inconclusive: missing or stale evidence",
                "front",
            )?;
        }
        Ok(())
    }

    fn start_plan_review(&self, work: &mut WorkRecord, cwd: &Path, now: u64) -> Result<(), String> {
        let index = work.next_role_index("verifier");
        let id = format!("V{index}");
        let dispatch = generate_id("dispatch");
        let verification = Verification {
            subject: work.verification_subject(),
            challenge: generate_id("verify"),
            verdict: None,
            evidence: vec![],
            blocking_issues: vec![],
        };
        let position = work.participants.len();
        work.participants.push(Participant {
            extra: Default::default(),
            participant_id: id,
            index,
            role: "verifier".to_owned(),
            backend: "codex".to_owned(),
            ref_id: String::new(),
            turn_id: None,
            profile: work
                .lead
                .as_ref()
                .filter(|lead| lead.kind == "codex")
                .and_then(|lead| lead.profile.clone()),
            slug: "plan-review".to_owned(),
            task_summary: "Independent review of the completed steps".to_owned(),
            owner_scope: None,
            write_workspace: None,
            step_id: None,
            worktree: None,
            changed_paths: vec![],
            write_paths: vec![],
            call_id: None,
            dispatch_id: Some(dispatch.clone()),
            state: "dispatching".to_owned(),
            result_summary: None,
            verification: Some(verification.clone()),
            created_at_unix: now,
            updated_at_unix: now,
        });
        self.ledger.save_work(work).map_err(|e| e.to_string())?;
        let evidence: Vec<_> = work.participants.iter().filter(|p| p.role == "worker")
            .map(|p| json!({"task":p.slug,"output":p.result_summary,"worktree":p.worktree,"paths":p.changed_paths})).collect();
        let prompt = format!(
            "[ORCH_DISPATCH {dispatch}]\nIndependently review the actual work and test receipts without editing.\nGoal: {}\nCriteria: {:?}\nWork artifacts: {evidence:?}\nReturn only this JSON object shape: {}",
            work.goal, work.completion_criteria,
            json!({"challenge":verification.challenge,"verdict":"pass|fail|inconclusive","evidence_refs":["path or receipt"],"blocking_issues":[]}),
        );
        let profile = work.participants[position].profile.clone();
        let reply = self.owners.codex_dispatch_turn(
            profile.as_deref(),
            None,
            cwd.to_str(),
            &prompt,
            None,
            None,
            None,
            Some("never"),
            |id| {
                work.participants[position].ref_id = id.to_owned();
                self.ledger.save_work(work).map_err(|e| e.to_string())
            },
        );
        if let Ok((thread, Some(turn))) = reply {
            let p = &mut work.participants[position];
            p.ref_id = thread;
            p.turn_id = Some(turn);
            p.state = "running".to_owned();
            self.ledger.save_work(work).map_err(|e| e.to_string())?;
        } else if work.participants[position].ref_id.is_empty() {
            let p = &mut work.participants[position];
            p.state = "failed".to_owned();
            p.result_summary = Some(
                "verification_inconclusive: independent Codex owner was unavailable".to_owned(),
            );
            p.verification.as_mut().unwrap().verdict = Some(Verdict::Inconclusive);
            self.ledger.save_work(work).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn settle_plan(
        &self,
        work: &mut WorkRecord,
        state: &str,
        detail: &str,
        target: &str,
    ) -> Result<(), String> {
        let delivery_id = "plan:finished".to_owned();
        let detail = truncate_chars(detail, 4000);
        self.ledger
            .append_event_if_absent(
                work,
                &delivery_id,
                "plan.finished",
                "runtime",
                &detail,
                json!({"state":state}),
            )
            .map_err(|e| e.to_string())?;
        work.state = state.to_owned();
        if let Some(wait) = work.wait_set.as_mut() {
            wait.armed = false;
        }
        enqueue_delivery_if_absent(
            work,
            PendingDelivery {
                delivery_id,
                target: target.to_owned(),
                kind: state.to_owned(),
                title: format!("{} — {state}", work.title),
                payload_text: format!("Work {} {state}\n{detail}", work.work_id),
                advance_seq_to: None,
                dispatch: None,
                receipts_tracked: true,
                notified: false,
                attempts: 0,
                next_attempt_unix: now_unix(),
                last_error: None,
                created_at_unix: now_unix(),
            },
        );
        self.ledger.save_work(work).map_err(|e| e.to_string())
    }

    fn continue_with_shell(
        &self,
        work: &mut WorkRecord,
        wait: &WaitSet,
        now: u64,
    ) -> Result<(), String> {
        let Some(check) = &wait.then_verify_shell else {
            return Ok(());
        };
        let index = if let Some(id) = &wait.continuation_id {
            work.participants
                .iter()
                .position(|p| &p.participant_id == id)
                .ok_or("missing shell continuation participant")?
        } else {
            if wait.deadline_unix.is_some_and(|deadline| now >= deadline)
                || now.saturating_sub(work.created_at_unix) > work.budget.max_wall_secs
            {
                return self.finish_shell_continuation(
                    work,
                    wait,
                    "waiting_user",
                    "The shell check did not start before its deadline.",
                );
            }
            if work.participants.iter().any(|p| !p.is_terminal()) {
                return Ok(());
            }
            if wait.all_of.iter().any(|id| {
                !work
                    .participants
                    .iter()
                    .any(|p| &p.participant_id == id && p.state == "completed")
            }) || !work.verification_passed()
            {
                return self.finish_shell_continuation(
                    work,
                    wait,
                    "failed",
                    "A prerequisite did not pass. The shell check was not started.",
                );
            }
            let index = work.next_role_index("verifier");
            let id = format!("V{index}");
            let participant = Participant {
                extra: Default::default(),
                participant_id: id.clone(),
                index,
                role: "verifier".to_owned(),
                backend: "shell".to_owned(),
                ref_id: id.clone(),
                turn_id: None,
                profile: None,
                slug: wait.wait_id.clone(),
                task_summary: "Scheduled shell check".to_owned(),
                owner_scope: None,
                write_workspace: None,
                step_id: None,
                worktree: None,
                changed_paths: vec![],
                write_paths: vec![],
                call_id: None,
                dispatch_id: None,
                state: "starting".to_owned(),
                result_summary: None,
                verification: Some(Verification {
                    subject: work.verification_subject(),
                    challenge: generate_id("verify"),
                    verdict: None,
                    evidence: Vec::new(),
                    blocking_issues: vec![],
                }),
                created_at_unix: now,
                updated_at_unix: now,
            };
            work.wait_set.as_mut().unwrap().continuation_id = Some(id);
            work.participants.push(participant);
            work.state = "running".to_owned();
            self.ledger.save_work(work).map_err(|e| e.to_string())?;
            work.participants.len() - 1
        };
        let participant = &work.participants[index];
        if participant.is_terminal() {
            let result = participant
                .result_summary
                .as_deref()
                .unwrap_or("No verification receipt")
                .to_owned();
            let passed = work.verification_passed();
            return self.finish_shell_continuation(
                work,
                wait,
                if passed { "completed" } else { "failed" },
                &result,
            );
        }
        if participant.state == "starting" {
            let job =
                crate::shell_job::ShellJob::new(&self.ledger, &work.work_id, &participant.ref_id)
                    .map_err(|e| e.to_string())?;
            if job.has_owner().map_err(|e| e.to_string())? {
                work.participants[index].state = "running".to_owned();
                self.ledger.save_work(work).map_err(|e| e.to_string())?;
            } else if let Err(error) = job.spawn(
                self.ledger.root(),
                &work.work_id,
                &check.command,
                check.cwd.as_deref(),
            ) {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    let detail = format!("The shell check could not start: {error}");
                    work.participants[index].state = "failed".to_owned();
                    work.participants[index].result_summary = Some(detail.clone());
                    return self.finish_shell_continuation(work, wait, "failed", &detail);
                }
            }
        }
        Ok(())
    }

    fn finish_shell_continuation(
        &self,
        work: &mut WorkRecord,
        wait: &WaitSet,
        state: &str,
        detail: &str,
    ) -> Result<(), String> {
        let delivery_id = format!("continuation:{}", wait.wait_id);
        let now = now_unix();
        let detail = truncate_chars(detail, 1500);
        self.ledger
            .append_event_if_absent(
                work,
                &delivery_id,
                "continuation.finished",
                "runtime",
                &detail,
                json!({"state":state,"verifier":wait.continuation_id}),
            )
            .map_err(|e| e.to_string())?;
        work.state = state.to_owned();
        work.wait_set.as_mut().unwrap().armed = false;
        enqueue_delivery_if_absent(
            work,
            PendingDelivery {
                delivery_id,
                target: "front".to_owned(),
                kind: state.to_owned(),
                title: format!("{} — {state}", work.title),
                payload_text: format!("Work {} {state}\n{detail}", work.work_id),
                advance_seq_to: None,
                dispatch: None,
                receipts_tracked: true,
                notified: false,
                attempts: 0,
                next_attempt_unix: now,
                last_error: None,
                created_at_unix: now,
            },
        );
        self.ledger.save_work(work).map_err(|e| e.to_string())
    }
}

enum DeliveryOutcome {
    Delivered,
    Defer(String),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelVerdict {
    challenge: String,
    verdict: Verdict,
    #[serde(alias = "evidence_refs")]
    evidence: Vec<String>,
    #[serde(default)]
    blocking_issues: Vec<String>,
}

fn verifier_result(
    p: &Participant,
    subject: &str,
    state: &str,
    text: &str,
) -> (Verdict, Vec<String>, Vec<String>) {
    let unverified = (Verdict::Inconclusive, Vec::new(), Vec::new());
    let Some(verification) = p.verification.as_ref().filter(|v| v.subject == subject) else {
        return unverified;
    };
    if p.backend == "shell" {
        let verdict = if state == "completed" && text.starts_with("exit_code=0\n") {
            Verdict::Pass
        } else if text.starts_with("exit_code=") {
            Verdict::Fail
        } else {
            return unverified;
        };
        return (
            verdict,
            vec![format!("shell/{}.exit.json", p.ref_id)],
            vec![],
        );
    }
    if state != "completed" {
        return unverified;
    }
    let Ok(result) = serde_json::from_str::<ModelVerdict>(text.trim()) else {
        return unverified;
    };
    if result.challenge != verification.challenge
        || result.evidence.is_empty()
        || result.evidence.len() > 20
        || result
            .evidence
            .iter()
            .any(|r| r.trim().is_empty() || r.len() > 400)
    {
        return unverified;
    }
    if result.verdict == Verdict::Pass && !result.blocking_issues.is_empty() {
        return unverified;
    }
    if result.blocking_issues.len() > 20
        || result
            .blocking_issues
            .iter()
            .any(|issue| issue.is_empty() || issue.len() > 400)
    {
        return unverified;
    }
    (result.verdict, result.evidence, result.blocking_issues)
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
    user_event: bool,
    now: u64,
) -> Option<String> {
    if user_event {
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
        let start = work.participants.len().saturating_sub(12);
        if start > 0 {
            out.push_str("Earlier participants are available from orch_works(read).\n");
        }
        for p in &work.participants[start..] {
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
        for ev in events {
            out.push_str(&format!(
                "- #{} [{}] ({}): {}\n",
                ev.seq,
                ev.kind,
                ev.actor,
                truncate_chars(&ev.summary, 300)
            ));
        }
    }
    if let Some(last) = events.last() {
        out.push_str(&format!(
            "\nUse orch_works(inbox) with after_seq={} to read later events.\n",
            last.seq
        ));
    }
    out.push_str(
        "\nReview the results above. Continue the next round via `@TermuxCodex`/`@TermuxChatGPT`/`@TermuxShell` + `@TermuxOrch` (`orch_dispatch`), or report to the user via `orch_report` (never target your own currently generating chat via `@TermuxChatGPT` and avoid retired `@TermuxBridge`).",
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
                                "last_tick_errors": server_health.last_tick_errors.load(Ordering::Acquire),
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
        let tick = engine.tick_once();
        health
            .last_tick_errors
            .store(tick.errors as u64, Ordering::Release);
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

    health_handle
        .join()
        .map_err(|_| "health server thread panicked")?;
    if let Some(url_file) = health_url_file {
        match fs::remove_file(url_file) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(format!("cannot remove owned health URL file: {error}")),
        }
    }
    Ok(())
}

#[derive(Default)]
struct RelayHealth {
    ticks: AtomicU64,
    last_tick_errors: AtomicU64,
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
