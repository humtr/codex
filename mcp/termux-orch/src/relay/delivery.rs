use super::*;
use crate::ledger::DeliveryDispatch;
use crate::owners::{parse_retry_after_secs, DEFAULT_429_COOLDOWN_SECS};

const MAX_UNCONFIRMED_READBACK_ATTEMPTS: u32 = 5;

impl RelayEngine {
    pub(super) fn attempt_delivery(
        &self,
        work: &mut WorkRecord,
        index: usize,
        delivery: &mut PendingDelivery,
    ) -> Result<DeliveryOutcome, String> {
        use DeliveryOutcome::{Defer, Delivered, Escalate};
        let defer = |reason: String| Defer {
            reason,
            min_backoff_secs: 0,
        };
        let defer_with = |reason: String, min_backoff_secs: u64| Defer {
            reason,
            min_backoff_secs,
        };
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
                return Ok(defer("legacy_delivery_receipt_unconfirmed".to_owned()));
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
                    defer("front_notification_failed".to_owned())
                });
            }
        }
        if let Some(dispatch) = &delivery.dispatch {
            let marker = format!("[ORCH_DELIVERY {}]", dispatch.lane_id);
            let confirmation = match dispatch.backend.as_str() {
                "chatgpt" => {
                    let rem = self.owners.chatgpt_cooldown_remaining_secs();
                    if rem > 0 {
                        return Ok(defer_with("backend_http_429".to_owned(), rem));
                    }
                    let account_matches = match work.account_id.as_deref() {
                        Some(account) => self
                            .owners
                            .chatgpt_account_id()
                            .is_ok_and(|actual| actual == account),
                        None => false,
                    };
                    if !account_matches {
                        return Ok(defer("delivery_account_unverified".to_owned()));
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
                _ => return Ok(defer("unsupported_delivery_backend".to_owned())),
            };
            return Ok(match confirmation {
                Ok(true) => Delivered,
                Ok(false) => {
                    if delivery.attempts >= MAX_UNCONFIRMED_READBACK_ATTEMPTS {
                        Escalate(format!(
                            "delivery_receipt_unconfirmed after {} verified readbacks",
                            delivery.attempts
                        ))
                    } else {
                        defer("delivery_receipt_unconfirmed".to_owned())
                    }
                }
                Err(error) => {
                    let backoff = if error.contains("backend_http_429")
                        || error.contains("retry_after_cooldown")
                    {
                        self.owners
                            .chatgpt_cooldown_remaining_secs()
                            .max(DEFAULT_429_COOLDOWN_SECS)
                    } else {
                        0
                    };
                    defer_with(error, backoff)
                }
            });
        }

        let lead = match delivery.target.as_str() {
            "front" => None,
            "lead" => match &work.lead {
                Some(lead) => Some(lead.clone()),
                None => return Ok(defer("no_lead_bound".to_owned())),
            },
            _ => return Ok(defer("unsupported_delivery_target".to_owned())),
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
            return Ok(defer("delivery_account_unverified".to_owned()));
        }
        if backend == "chatgpt" {
            let rem = self.owners.chatgpt_cooldown_remaining_secs();
            if rem > 0 {
                return Ok(defer_with("backend_http_429".to_owned(), rem));
            }
        }
        let ready = match backend {
            "chatgpt" => self.owners.chatgpt_observe(&ref_id).and_then(|obs| {
                if obs.state == "error"
                    && matches!(
                        obs.error.as_deref(),
                        Some("backend_http_429" | "retry_after_cooldown" | "local_rate_limit")
                    )
                {
                    Err(format!(
                        "backend_http_429:retry_after={}",
                        obs.retry_after_secs.unwrap_or(DEFAULT_429_COOLDOWN_SECS)
                    ))
                } else if obs.state == "error" {
                    Err(obs
                        .error
                        .unwrap_or_else(|| "chat_readiness_unconfirmed".to_owned()))
                } else if obs.generating {
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
                .codex_preflight(profile.as_deref())
                .and_then(|()| {
                    self.owners
                        .codex_observe_thread(profile.as_deref(), &ref_id)
                })
                .and_then(|obs| {
                    if obs.is_active {
                        Err("lead_codex_active".to_owned())
                    } else {
                        Ok(())
                    }
                }),
            _ => return Ok(defer("unsupported_delivery_backend".to_owned())),
        };
        if let Err(error) = ready {
            let backoff = if error.starts_with("backend_http_429") {
                self.owners
                    .chatgpt_cooldown_remaining_secs()
                    .max(DEFAULT_429_COOLDOWN_SECS)
            } else {
                0
            };
            return Ok(defer_with(error, backoff));
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
                    let err_code = receipt["error"]
                        .as_str()
                        .unwrap_or("delivery_rejected")
                        .to_owned();
                    let min_backoff = if matches!(
                        err_code.as_str(),
                        "backend_http_429" | "retry_after_cooldown" | "local_rate_limit"
                    ) {
                        parse_retry_after_secs(&receipt)
                            .unwrap_or(DEFAULT_429_COOLDOWN_SECS)
                            .max(self.owners.chatgpt_cooldown_remaining_secs())
                    } else {
                        0
                    };
                    defer_with(err_code, min_backoff)
                }
                Ok(receipt)
                    if receipt["state"] == "error"
                        && delivery.target == "lead"
                        && crate::rollover::is_context_limit_error(
                            receipt["error"].as_str().unwrap_or_default(),
                        ) =>
                {
                    delivery.dispatch = None;
                    self.invalidate_rollover_check(&work.work_id);
                    defer_with(
                        receipt["error"]
                            .as_str()
                            .unwrap_or("context_limit")
                            .to_owned(),
                        2,
                    )
                }
                Ok(receipt)
                    if receipt["chat_id"] == ref_id
                        && matches!(receipt["state"].as_str(), Some("settled" | "in_progress")) =>
                {
                    Delivered
                }
                Ok(_) => defer("delivery_receipt_unconfirmed".to_owned()),
                Err(error) => defer(error),
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
                Ok(_) => defer("delivery_receipt_unconfirmed".to_owned()),
                Err(error) if error.starts_with("codex_preflight_failed: ") => {
                    delivery.dispatch = None;
                    defer(error)
                }
                Err(error) => defer(error),
            },
            _ => unreachable!("unsupported backend rejected before dispatch"),
        })
    }
}

pub(super) enum DeliveryOutcome {
    Delivered,
    Defer {
        reason: String,
        min_backoff_secs: u64,
    },
    Escalate(String),
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
