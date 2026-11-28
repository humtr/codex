use super::*;

impl OrchMcpHandler {
    pub(super) fn handle_report(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(
            obj,
            &[
                "action",
                "call_id",
                "work_id",
                "expected_lead_generation",
                "lead_ref_id",
                "summary",
                "checkpoint_md",
                "evidence",
                "review_cwd",
                "flush_now",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        if !matches!(action, "milestone" | "need_user" | "complete" | "fail") {
            return fail(
                "invalid_arguments",
                format!("unsupported report action '{action}'"),
            );
        }
        self.with_call_id_receipt(args, &format!("orch_report:{action}"), || {
            let Some(wid) = obj
                .get("work_id")
                .and_then(Value::as_str)
                .filter(|s| is_safe_work_id(s))
            else {
                return (fail("invalid_work_id", "valid work_id is required"), false);
            };
            let Some(summary) = obj
                .get("summary")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return (
                    fail("invalid_arguments", "non-empty summary is required"),
                    false,
                );
            };
            let call_id = obj.get("call_id").and_then(Value::as_str).unwrap_or("rep");
            let flush_now = obj
                .get("flush_now")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let evidence: Vec<String> = obj
                .get("evidence")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(|s| truncate_chars(s.trim(), 400))
                        .collect()
                })
                .unwrap_or_default();

            let delivery_id = format!("report:{action}:{call_id}");
            {
                let _lock = match self.ledger.lock_work(wid) {
                    Ok(l) => l,
                    Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
                };
                let mut work = match self.ledger.load_work(wid) {
                    Ok(w) => w,
                    Err(e) => return (fail("work_not_found", e.to_string()), false),
                };
                if let Err(e) = check_lead_fence(&work, obj, true) {
                    return (e, false);
                }
                if !work.can_start() {
                    return (fail("work_already_terminal", &work.state), false);
                }
                if matches!(action, "complete" | "fail")
                    && work
                        .wait_set
                        .as_ref()
                        .is_some_and(|w| w.armed && w.then_verify_shell.is_some())
                {
                    return (
                        fail(
                            "continuation_armed",
                            "The fixed continuation owns the final report",
                        ),
                        false,
                    );
                }
                if action == "fail" && work.participants.iter().any(|p| !p.is_terminal()) {
                    return (
                        fail(
                            "participants_active",
                            "Stop or settle participants before recording failure",
                        ),
                        false,
                    );
                }
                if action == "complete" {
                    if work.steps.iter().any(|step| {
                        !work.participants.iter().any(|p| {
                            p.step_id.as_deref() == Some(&step.step_id) && p.state == "completed"
                        })
                    }) {
                        return (
                            fail(
                                "steps_pending",
                                "The durable plan has unfinished or failed steps",
                            ),
                            false,
                        );
                    }
                    if work.pending_lead.is_some() {
                        return (
                            fail(
                                "lead_dispatch_pending",
                                "Lead launch has not been reconciled",
                            ),
                            false,
                        );
                    }
                    if work.participants.iter().any(|p| !p.is_terminal()) {
                        return (
                            fail("participants_active", "All participants must settle"),
                            false,
                        );
                    }
                    for participant in &work.participants {
                        let Some(tree) = &participant.worktree else {
                            continue;
                        };
                        let scoped = tree.path.try_exists().map_err(|e| e.to_string()).and_then(
                            |retained| {
                                if retained {
                                    tree.declared_changes(&participant.write_paths)?;
                                }
                                Ok(())
                            },
                        );
                        if let Err(error) = scoped {
                            return (fail("worker_workspace_scope_unconfirmed", error), false);
                        }
                    }
                    if !work.verification_passed() {
                        if work.verification_policy == VerificationPolicy::IndependentReview
                            && !work
                                .participants
                                .iter()
                                .any(|p| p.role == "verifier" && p.backend != "shell")
                            && work.steps.is_empty()
                        {
                            let requested = obj
                                .get("review_cwd")
                                .and_then(Value::as_str)
                                .map(std::path::Path::new)
                                .or_else(|| {
                                    work.participants
                                        .iter()
                                        .find_map(|p| p.write_workspace.as_deref())
                                });
                            let Some(cwd) = requested else {
                                return (
                                    fail(
                                        "verification_required",
                                        "Supply review_cwd for the independent review",
                                    ),
                                    false,
                                );
                            };
                            let cwd = match crate::workspace::root(cwd) {
                                Ok(cwd) => cwd,
                                Err(error) => return (fail("invalid_workspace", error), false),
                            };
                            work.review_cwd = Some(cwd);
                            work.state = "waiting".to_owned();
                            if let Err(error) = self.ledger.save_work(&work) {
                                return (fail("ledger_write_failed", error.to_string()), false);
                            }
                        }
                        return (
                            fail(
                                "verification_required",
                                "A current passing verdict is required",
                            ),
                            false,
                        );
                    }
                }
                if let Some(cp) = obj.get("checkpoint_md").and_then(Value::as_str) {
                    if let Err(error) = self.ledger.write_checkpoint(wid, cp) {
                        return (
                            fail("ledger_checkpoint_write_failed", error.to_string()),
                            false,
                        );
                    }
                }
                match action {
                    "need_user" => work.state = "waiting_user".to_owned(),
                    "complete" => {
                        work.state = "completed".to_owned();
                        if let Some(ws) = work.wait_set.as_mut() {
                            ws.armed = false;
                        }
                    }
                    "fail" => {
                        work.state = "failed".to_owned();
                        if let Some(ws) = work.wait_set.as_mut() {
                            ws.armed = false;
                        }
                    }
                    _ => {}
                }
                let ev_lines = if evidence.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n\n### Evidence\n{}",
                        evidence
                            .iter()
                            .map(|e| format!("- {e}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                };
                let payload_text = format!(
                    "[ORCH_REPORT:{action}] work_id={} title=\"{}\"\n\n{}{}",
                    work.work_id, work.title, summary, ev_lines
                );
                let now = now_unix();
                let del_title = format!("[Work:{action}] {}", work.title);
                enqueue_delivery_if_absent(
                    &mut work,
                    PendingDelivery {
                        delivery_id: delivery_id.clone(),
                        target: "front".to_owned(),
                        kind: action.to_owned(),
                        title: del_title,
                        payload_text,
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
                let _ = self.ledger.append_event_if_absent(
                    &mut work,
                    &delivery_id,
                    &format!("report.{action}"),
                    "lead",
                    summary,
                    json!({ "evidence": evidence }),
                );
                if let Err(e) = self.ledger.save_work(&work) {
                    return (fail("ledger_write_failed", e.to_string()), false);
                }
            }

            let mut delivered_immediately = false;
            if flush_now && self.admission_only {
                let _ = crate::wake::request(true);
            } else if flush_now {
                let engine = RelayEngine::new(self.ledger.clone(), self.owners.clone());
                let _ = engine.process_work(wid);
                delivered_immediately = self
                    .ledger
                    .delivery_recorded(wid, &delivery_id)
                    .unwrap_or(false);
            }
            let fresh = self.ledger.load_work(wid).ok();
            let pending_left = fresh
                .as_ref()
                .map(|w| w.pending_deliveries.len())
                .unwrap_or(0);
            (
                json!({
                    "state": "ok",
                    "work_id": wid,
                    "report_action": action,
                    "delivery_id": delivery_id,
                    "delivered_immediately": delivered_immediately,
                    "pending_deliveries": pending_left,
                    "work_state": fresh.map(|w| w.state)
                }),
                true,
            )
        })
    }
}
