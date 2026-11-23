use crate::ledger::{
    format_lead_title, generate_id, now_unix, truncate_chars, LeadHistoryEntry, LeadReplay,
    LeadRollover, Ledger, WorkRecord,
};
use crate::owners::OwnerClient;
use serde_json::{json, Value};

pub struct Rollover<'a> {
    ledger: &'a Ledger,
    owners: &'a OwnerClient,
}

impl<'a> Rollover<'a> {
    pub fn new(ledger: &'a Ledger, owners: &'a OwnerClient) -> Self {
        Self { ledger, owners }
    }

    fn verify_account(&self, work: &WorkRecord) -> Result<(), String> {
        let pinned = work
            .account_id
            .as_deref()
            .ok_or("pinned_account_required")?;
        if self.owners.chatgpt_account_id()? != pinned {
            return Err("pinned_account_mismatch".to_owned());
        }
        Ok(())
    }

    /// Called under the existing per-Work ledger lock.
    pub fn start(
        &self,
        work: &mut WorkRecord,
        reason: &str,
        checkpoint: Option<&str>,
    ) -> Result<bool, String> {
        if work.is_terminal() {
            return Err("work_already_terminal".to_owned());
        }
        if work.lead_replay.is_some() {
            return Err("lead_replay_in_progress".to_owned());
        }
        if work.lead_rollover.is_some() {
            return self.advance(work);
        }
        let lead = work
            .lead
            .as_ref()
            .filter(|l| l.kind == "chatgpt")
            .ok_or("chatgpt_lead_required")?;
        let project = work
            .project_id
            .as_deref()
            .ok_or("pinned_project_required")?;
        self.verify_account(work)?;
        let chat = self.owners.chatgpt_read_chat(&lead.ref_id, 1)?;
        if chat["state"] != "ok" || chat["gizmo_id"] != project {
            return Err("pinned_project_mismatch".to_owned());
        }

        let checkpoint = match checkpoint {
            Some(text) => text.to_owned(),
            None => self
                .ledger
                .read_checkpoint(&work.work_id)
                .map_err(|e| e.to_string())?
                .unwrap_or_default(),
        };
        let snapshot = format!(
            "Lead L{} handoff — {}\n\n{checkpoint}",
            lead.generation,
            truncate_chars(reason, 240)
        );
        self.ledger
            .write_checkpoint(&work.work_id, &snapshot)
            .map_err(|e| e.to_string())?;
        work.lead_rollover = Some(LeadRollover {
            lane_id: generate_id("rollover"),
            reason: truncate_chars(reason, 240),
            dispatched: false,
            chat_id: None,
        });
        self.save(work)?;
        self.advance(work)
    }

    pub fn automatic_reason(&self, work: &WorkRecord) -> Option<String> {
        let lead = work.lead.as_ref().filter(|l| l.kind == "chatgpt")?;
        if work.account_id.is_none()
            || work.project_id.is_none()
            || work.state == "waiting_user"
            || work.budget.wakeups_used >= work.budget.max_wakeups
            || now_unix().saturating_sub(work.created_at_unix) > work.budget.max_wall_secs
        {
            return None;
        }
        let chat = self.owners.chatgpt_read_chat(&lead.ref_id, 1).ok()?;
        if chat["state"] != "ok" || chat["gizmo_id"].as_str() != work.project_id.as_deref() {
            return None;
        }
        let error = chat["local_turn_error"].as_str().unwrap_or_default();
        if matches!(
            error,
            "context_length_exceeded"
                | "context_window_exceeded"
                | "conversation_too_long"
                | "max_context_length_exceeded"
        ) {
            return Some(format!("context limit ({error})"));
        }
        let count = chat["user_turn_count"]
            .as_u64()?
            .saturating_sub(u64::from(lead.rollover_internal_turns));
        let threshold = u64::from(work.budget.lead_rollover_turns);
        if threshold > 0
            && count >= threshold
            && self
                .owners
                .chatgpt_observe(&lead.ref_id)
                .is_ok_and(|obs| !obs.generating)
        {
            return Some(format!("turn threshold ({count}/{threshold})"));
        }
        None
    }

    /// Reconcile the initial chat creation before swapping generations.
    pub fn advance(&self, work: &mut WorkRecord) -> Result<bool, String> {
        let mut rollover = work.lead_rollover.clone().ok_or("lead_rollover_missing")?;
        let lead = work.lead.as_ref().ok_or("chatgpt_lead_required")?;
        let generation = lead
            .generation
            .checked_add(1)
            .ok_or("lead_generation_exhausted")?;
        let project = work
            .project_id
            .as_deref()
            .ok_or("pinned_project_required")?;
        self.verify_account(work)?;
        let marker = format!("[ORCH_ROLLOVER {}]", rollover.lane_id);
        if !rollover.dispatched {
            let checkpoint = self
                .ledger
                .read_checkpoint(&work.work_id)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            let prompt = format!(
                "{marker}\nResume as Lead L{generation} for work_id={}.\nGoal: {}\n\n{}\n\nAcknowledge this handoff. Wait for the Typed Inbox replay before taking action. Include expected_lead_generation={generation} in all Work mutations.",
                work.work_id,
                truncate_chars(&work.goal, 1000),
                truncate_chars(&checkpoint, 8000),
            );
            rollover.dispatched = true;
            work.lead_rollover = Some(rollover.clone());
            self.save(work)?;
            let receipt = self.owners.chatgpt_send_pinned(
                work,
                work.lead.as_ref().unwrap(),
                None,
                &rollover.lane_id,
                &prompt,
            )?;
            if rejected_before_send(&receipt) {
                work.lead_rollover = None;
                self.save(work)?;
                return Err(receipt["error"]
                    .as_str()
                    .unwrap_or("lead_rollover_rejected")
                    .to_owned());
            }
            rollover.chat_id = receipt["chat_id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .map(ToOwned::to_owned);
        }

        if rollover.chat_id.is_none() {
            if let Ok(lane) = self.owners.chatgpt_observe_lane(&rollover.lane_id) {
                if lane["lane_id"] == rollover.lane_id {
                    rollover.chat_id = lane["chat_id"].as_str().map(ToOwned::to_owned);
                }
            }
        }
        if rollover.chat_id.is_none() {
            // An accepted turn can outlive the provider lane cache. Recover only
            // from the unique marker in the pinned project. Absence never licenses replay.
            let chats = self.owners.chatgpt_project_chats(project)?;
            if let Some(rows) = chats["chats"].as_array() {
                for row in rows {
                    if let Some(id) = row["id"].as_str() {
                        if self.has_marker(id, project, &marker)? {
                            rollover.chat_id = Some(id.to_owned());
                            break;
                        }
                    }
                }
            }
        }
        work.lead_rollover = Some(rollover.clone());
        self.save(work)?;
        let Some(chat_id) = rollover.chat_id.as_deref() else {
            return Ok(false);
        };
        if !self.has_marker(chat_id, project, &marker)? {
            return Ok(false);
        }

        let title = format_lead_title(generation, &work.title);
        let renamed = self.owners.chatgpt_rename_chat(chat_id, &title)?;
        if renamed["state"] != "ok" {
            return Err("lead_rename_failed".to_owned());
        }
        let mut next = work.lead.clone().unwrap();
        if !next
            .history
            .iter()
            .any(|row| row.generation == next.generation)
        {
            next.history.push(LeadHistoryEntry {
                generation: next.generation,
                kind: next.kind.clone(),
                ref_id: next.ref_id.clone(),
                started_at_unix: work.created_at_unix,
            });
        }
        next.generation = generation;
        next.ref_id = chat_id.to_owned();
        next.rollover_internal_turns = 1;
        next.history.push(LeadHistoryEntry {
            generation,
            kind: "chatgpt".to_owned(),
            ref_id: chat_id.to_owned(),
            started_at_unix: now_unix(),
        });
        self.ledger.append_event_if_absent(
            work,
            &format!("rollover:{}", rollover.lane_id),
            "lead.rolled_over",
            &format!("L{generation}"),
            &format!("Lead rolled over to L{generation}: {}", rollover.reason),
            json!({"generation": generation, "previous_ref_id": next.history.iter().rev().nth(1).map(|row| &row.ref_id), "ref_id": chat_id, "reason": rollover.reason}),
        ).map_err(|e| e.to_string())?;
        work.lead = Some(next);
        work.lead_rollover = None;
        work.lead_replay = Some(LeadReplay {
            through_seq: work.last_seq,
            dispatched_until: None,
        });
        // Queued lead prompts carry the old snapshot. The durable replay owns
        // their events after the swap. Front reports retain their own delivery.
        work.pending_deliveries
            .retain(|delivery| delivery.target != "lead");
        for delivery in &mut work.pending_deliveries {
            delivery.advance_seq_to = None;
        }
        self.save(work)?;
        Ok(true)
    }

    pub fn replay(&self, work: &mut WorkRecord) -> Result<(), String> {
        let mut replay = work.lead_replay.clone().ok_or("lead_replay_missing")?;
        let lead = work.lead.as_ref().ok_or("chatgpt_lead_required")?;
        let project = work
            .project_id
            .as_deref()
            .ok_or("pinned_project_required")?;
        let chat_id = lead.ref_id.clone();
        self.verify_account(work)?;
        let after = work.consumed_seq;
        let mut events = self
            .ledger
            .read_first_events(&work.work_id, after, 8)
            .map_err(|e| e.to_string())?;
        events.retain(|event| event.seq <= replay.dispatched_until.unwrap_or(replay.through_seq));
        let Some(last_seq) = events.last().map(|event| event.seq) else {
            return Err("lead_replay_events_missing".to_owned());
        };
        let lane = format!("{}_L{}_inbox_{}", work.work_id, lead.generation, last_seq);
        let marker = format!("[ORCH_INBOX_REPLAY {lane}]");
        if self.has_marker(&chat_id, project, &marker)? {
            work.consumed_seq = last_seq;
            if last_seq >= replay.through_seq {
                work.lead_replay = None;
            } else {
                replay.dispatched_until = None;
                work.lead_replay = Some(replay);
            }
            work.lead.as_mut().unwrap().rollover_internal_turns += 1;
            self.save(work)?;
        } else if replay.dispatched_until.is_none()
            && !self.owners.chatgpt_observe(&chat_id)?.generating
        {
            let payload = serde_json::to_string(&events).map_err(|e| e.to_string())?;
            let instruction = if last_seq >= replay.through_seq {
                "This is the final replay batch. Resume from the current ledger and checkpoint unless the existing Wait-Set is still armed."
            } else {
                "Acknowledge these events and wait for the remaining replay batches."
            };
            let prompt = format!(
                "{marker}\nwork_id={} expected_lead_generation={}\n{instruction}\nTyped Inbox events:\n{payload}",
                work.work_id, lead.generation
            );
            replay.dispatched_until = Some(last_seq);
            work.lead_replay = Some(replay);
            self.save(work)?;
            let receipt = self.owners.chatgpt_send_pinned(
                work,
                work.lead.as_ref().unwrap(),
                Some(&chat_id),
                &lane,
                &prompt,
            )?;
            if rejected_before_send(&receipt) {
                work.lead_replay.as_mut().unwrap().dispatched_until = None;
                self.save(work)?;
            }
        }
        Ok(())
    }

    fn save(&self, work: &WorkRecord) -> Result<(), String> {
        self.ledger.save_work(work).map_err(|e| e.to_string())
    }

    fn has_marker(&self, chat_id: &str, project: &str, marker: &str) -> Result<bool, String> {
        let chat = self.owners.chatgpt_read_chat(chat_id, 100)?;
        Ok(chat["state"] == "ok"
            && chat["gizmo_id"] == project
            && chat["messages"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["role"] == "user"
                        && row["text"]
                            .as_str()
                            .is_some_and(|t| t.lines().next() == Some(marker))
                })
            }))
    }
}

fn rejected_before_send(receipt: &Value) -> bool {
    receipt["state"] == "error"
        && matches!(
            receipt["error"].as_str().unwrap_or_default(),
            "pinned_account_mismatch"
                | "account_changed"
                | "pinned_account_unavailable"
                | "pinned_project_mismatch"
                | "login_required"
                | "too_many_active_generations"
                | "completion_client_unavailable"
                | "prompt_required"
                | "generation_already_active"
        )
}
