use super::*;

impl OrchMcpHandler {
    pub(super) fn handle_call(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(obj, &["action", "arguments"]) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing extension action");
        };
        if self.extensions.get(action).is_none() {
            return self.extensions.validation_error(action, "unknown_action");
        }
        let empty = Map::new();
        let ext_args = match obj.get("arguments") {
            None => &empty,
            Some(Value::Object(m)) => m,
            Some(_) => {
                return self
                    .extensions
                    .validation_error(action, "'arguments' must be a JSON object")
            }
        };
        match action {
            "operation_status" => {
                let id = ext_args
                    .get("operation_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                crate::requests::Requests::open(&self.ledger)
                    .and_then(|requests| requests.status(id))
                    .unwrap_or_else(|error| fail("operation_status_failed", error.to_string()))
            }
            "relay_status" => {
                let works = match self.ledger.list_works() {
                    Ok(works) => works,
                    Err(err) => return fail("ledger_read_failed", err.to_string()),
                };
                let active = works.iter().filter(|w| !w.is_terminal()).count();
                let active_chatgpt: usize = works
                    .iter()
                    .filter(|w| !w.is_terminal())
                    .map(|w| {
                        w.participants
                            .iter()
                            .filter(|p| !p.is_terminal() && p.backend == "chatgpt")
                            .count()
                    })
                    .sum();
                let pending: usize = works.iter().map(|w| w.pending_deliveries.len()).sum();
                let health = crate::wake::request(false).unwrap_or(Value::Null);
                json!({
                    "state": "ok",
                    "state_root": self.ledger.root().display().to_string(),
                    "total_works": works.len(),
                    "active_works": active,
                    "active_chatgpt_workers": active_chatgpt,
                    "max_concurrent_chatgpt": self.owners.max_concurrent_chatgpt(),
                    "pending_deliveries": pending,
                    "relay_reachable": health["ok"] == true,
                    "chatgpt_cooldown_remaining_secs": health["chatgpt_cooldown_remaining_secs"],
                    "rate_limit_hits": health["rate_limit_hits"],
                    "remote_chatgpt_calls": health["remote_chatgpt_calls"],
                    "local_observe_calls": health["local_observe_calls"]
                })
            }
            "reconcile_delivery" => {
                if crate::wake::request(true)
                    .is_some_and(|response| response["state"] == "accepted")
                {
                    json!({"state":"accepted"})
                } else {
                    fail("relay_unreachable", "Relay wake was not accepted")
                }
            }
            "budget_extend" => {
                let Some(wid) = ext_args
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return self
                        .extensions
                        .validation_error(action, "valid work_id required");
                };
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return fail("ledger_lock_failed", e.to_string()),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return fail("work_not_found", e.to_string()),
                };
                if let Err(e) = check_lead_fence(&work, ext_args, true) {
                    return e;
                }
                if let Some(add_w) = ext_args.get("add_wakeups").and_then(Value::as_u64) {
                    work.budget.max_wakeups = work
                        .budget
                        .max_wakeups
                        .saturating_add(add_w.clamp(1, 100) as u32);
                }
                if let Some(add_s) = ext_args.get("add_wall_secs").and_then(Value::as_u64) {
                    work.budget.max_wall_secs = work
                        .budget
                        .max_wall_secs
                        .saturating_add(add_s.clamp(60, 86_400));
                }
                if let Some(mc) = ext_args
                    .get("max_concurrent_workers")
                    .and_then(Value::as_u64)
                {
                    work.budget.max_concurrent_workers = mc.clamp(1, 16) as u32;
                }
                if work.state == "waiting_user" {
                    work.state = "running".to_owned();
                }
                work.updated_at_unix = now_unix();
                if let Err(e) = self.ledger.save_work(&work) {
                    return fail("ledger_write_failed", e.to_string());
                }
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "work_state": work.state,
                    "budget": work.budget
                })
            }
            "rename_session" => {
                let Some(backend) = ext_args
                    .get("backend")
                    .and_then(Value::as_str)
                    .filter(|b| matches!(*b, "chatgpt" | "codex"))
                else {
                    return self
                        .extensions
                        .validation_error(action, "backend must be chatgpt|codex");
                };
                let Some(ref_id) = ext_args
                    .get("ref_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                else {
                    return self.extensions.validation_error(action, "ref_id required");
                };
                let Some(title) = ext_args
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty() && s.len() <= 180)
                else {
                    return self
                        .extensions
                        .validation_error(action, "title (1..=180) required");
                };
                let res = match backend {
                    "chatgpt" => self.owners.chatgpt_rename_chat(ref_id, title).map(|_| ()),
                    "codex" => self.owners.codex_rename_thread(
                        ext_args.get("profile").and_then(Value::as_str),
                        ref_id,
                        title,
                    ),
                    _ => unreachable!(),
                };
                match res {
                    Ok(()) => {
                        json!({ "state": "ok", "backend": backend, "ref_id": ref_id, "title": title })
                    }
                    Err(e) => fail("rename_failed", e),
                }
            }
            "lead_rollover" => {
                let Some(wid) = ext_args
                    .get("work_id")
                    .and_then(Value::as_str)
                    .filter(|s| is_safe_work_id(s))
                else {
                    return self
                        .extensions
                        .validation_error(action, "valid work_id required");
                };
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return fail("ledger_lock_failed", e.to_string()),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return fail("work_not_found", e.to_string()),
                };
                if let Err(e) = check_lead_fence(&work, ext_args, true) {
                    return e;
                }
                if !work.can_start() {
                    return fail("work_already_terminal", &work.state);
                }
                let current_kind = work
                    .lead
                    .as_ref()
                    .map(|lead| lead.kind.as_str())
                    .unwrap_or("chatgpt");
                let next_kind = ext_args
                    .get("lead_kind")
                    .and_then(Value::as_str)
                    .unwrap_or(current_kind);
                if next_kind == "chatgpt" {
                    let rollover = crate::rollover::Rollover::new(&self.ledger, &self.owners);
                    let reason = ext_args
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("manual rollover");
                    return match rollover.start(
                        &mut work,
                        reason,
                        ext_args.get("checkpoint_md").and_then(Value::as_str),
                    ) {
                        Ok(done) => json!({
                            "state": if done { "ok" } else { "pending" },
                            "work_id": wid,
                            "lead": work.lead,
                            "rollover": work.lead_rollover,
                        }),
                        Err(e) => {
                            let code = match e.as_str() {
                                "pinned_account_required"
                                | "chatgpt_pinned_send_unavailable"
                                | "pinned_account_mismatch"
                                | "pinned_project_required"
                                | "pinned_project_mismatch"
                                | "lead_replay_in_progress"
                                | "chatgpt_lead_required" => e.as_str(),
                                _ => "lead_rollover_failed",
                            };
                            fail(code, &e)
                        }
                    };
                }
                if work
                    .wait_set
                    .as_ref()
                    .is_some_and(|w| w.armed && w.then_verify_shell.is_some())
                {
                    return fail("work_unavailable", "Work cannot start a Lead");
                }
                if work.lead_rollover.is_some() || work.lead_replay.is_some() {
                    return fail("lead_rollover_in_progress", "ChatGPT handoff is pending");
                }
                if let Err(error) = self
                    .owners
                    .codex_preflight(ext_args.get("lead_profile").and_then(Value::as_str))
                {
                    return fail("codex_preflight_failed", error);
                }
                if let Some(cp) = ext_args.get("checkpoint_md").and_then(Value::as_str) {
                    if let Err(error) = self.ledger.write_checkpoint(wid, cp) {
                        return fail("ledger_checkpoint_write_failed", error.to_string());
                    }
                }
                let next_gen = work.lead.as_ref().map(|l| l.generation).unwrap_or(0) + 1;
                let mut warnings = Vec::new();
                let mut lead_args = ext_args.clone();
                lead_args.remove("lead_ref_id");
                lead_args.insert("lead_kind".to_owned(), json!("codex"));
                match self.spawn_or_bind_lead(&mut work, next_gen, &lead_args, &mut warnings) {
                    Ok(true) => (),
                    Ok(false) => {
                        return json!({"state":"pending","work_id":wid,"warnings":warnings})
                    }
                    Err(error) => return error,
                }
                let new_lead = work.lead.clone().unwrap();
                let _ = self.ledger.append_event_if_absent(
                    &mut work,
                    &format!("rollover:L{next_gen}:{}", new_lead.ref_id),
                    "lead.rolled_over",
                    &format!("L{next_gen}"),
                    &format!(
                        "Lead rolled over to L{next_gen} ({}:{})",
                        new_lead.kind, new_lead.ref_id
                    ),
                    json!({ "generation": next_gen, "ref_id": new_lead.ref_id }),
                );
                if let Err(e) = self.ledger.save_work(&work) {
                    return fail("ledger_write_failed", e.to_string());
                }
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "lead": work.lead,
                    "warnings": warnings
                })
            }
            _ => self.extensions.validation_error(action, "unknown_action"),
        }
    }
}
