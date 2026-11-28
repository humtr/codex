use super::*;

impl OrchMcpHandler {
    pub(super) fn handle_work(&self, args: &Value) -> Value {
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
                "title",
                "goal",
                "tier",
                "completion_criteria",
                "steps",
                "verification_policy",
                "project_id",
                "project_scope",
                "project_name",
                "session_prefixes",
                "front_chat_id",
                "front_marker",
                "delivery_mode",
                "lead_kind",
                "lead_profile",
                "lead_model",
                "lead_model_provider",
                "lead_reasoning_effort",
                "lead_cwd",
                "lead_ref_id",
                "max_wakeups",
                "lead_rollover_turns",
                "max_wall_secs",
                "max_concurrent_workers",
                "reason",
            ],
        ) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        self.with_call_id_receipt(args, &format!("orch_work:{action}"), || match action {
            "create" => self.op_work_create(obj),
            "promote" => self.op_work_promote(obj),
            "rebind_front" => self.op_work_rebind_front(obj),
            "update_scope" => self.op_work_update_scope(obj),
            "cancel" => self.op_work_cancel(obj),
            other => (
                fail("invalid_arguments", format!("unsupported action '{other}'")),
                false,
            ),
        })
    }

    pub(super) fn op_work_create(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(title) = obj
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 120)
        else {
            return (
                fail(
                    "invalid_arguments",
                    "'title' (1..=120 chars) is required for action=create",
                ),
                false,
            );
        };
        let Some(goal) = obj
            .get("goal")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty() && s.len() <= 8192)
        else {
            return (
                fail(
                    "invalid_arguments",
                    "'goal' (1..=8192 chars) is required for action=create",
                ),
                false,
            );
        };
        let tier = obj.get("tier").and_then(Value::as_str).unwrap_or_else(|| {
            if [
                "lead_kind",
                "lead_ref_id",
                "lead_profile",
                "lead_model",
                "lead_model_provider",
                "lead_cwd",
                "lead_reasoning_effort",
            ]
            .iter()
            .any(|k| obj.contains_key(*k))
            {
                "orchestrated"
            } else {
                "direct"
            }
        });
        if tier != "direct" && tier != "orchestrated" {
            return (
                fail("invalid_arguments", "'tier' must be direct|orchestrated"),
                false,
            );
        }
        let mut steps = match obj
            .get("steps")
            .cloned()
            .map(serde_json::from_value::<Vec<StepSpec>>)
            .transpose()
        {
            Ok(plan) => plan.unwrap_or_default(),
            Err(error) => return (fail("invalid_steps", error.to_string()), false),
        };
        if let Err(error) = StepSpec::validate_plan(&mut steps) {
            return (fail("invalid_steps", error), false);
        }
        let verification_policy = match obj
            .get("verification_policy")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
        {
            Ok(Some(policy)) => policy,
            Ok(None)
                if steps.iter().any(|step| {
                    step.write_paths.len() > 1
                        || step
                            .write_paths
                            .iter()
                            .any(|path| path == "SPEC.md" || path.starts_with("docs/spec/"))
                }) || steps
                    .iter()
                    .filter(|step| step.isolation == crate::ledger::Isolation::Worktree)
                    .count()
                    > 1 =>
            {
                VerificationPolicy::IndependentReview
            }
            Ok(None) if steps.iter().any(|step| step.auto_verify) => VerificationPolicy::Automated,
            Ok(None) => VerificationPolicy::None,
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        if !steps.is_empty()
            && verification_policy == VerificationPolicy::Automated
            && !steps.iter().any(|step| step.auto_verify)
        {
            return (
                fail(
                    "invalid_steps",
                    "automated verification needs a verification step",
                ),
                false,
            );
        }
        let delivery_mode = obj
            .get("delivery_mode")
            .and_then(Value::as_str)
            .unwrap_or("inject_and_notify");
        if delivery_mode != "inject_and_notify" && delivery_mode != "notify_only" {
            return (
                fail(
                    "invalid_arguments",
                    "'delivery_mode' must be inject_and_notify|notify_only",
                ),
                false,
            );
        }
        let criteria: Vec<String> = obj
            .get("completion_criteria")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| truncate_chars(s.trim(), 300))
                    .filter(|s| !s.is_empty())
                    .take(20)
                    .collect()
            })
            .unwrap_or_default();

        let session_prefixes = match parse_session_prefixes(obj) {
            Ok(prefixes) => prefixes.unwrap_or_default(),
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let project_scope = match project_label(obj, "project_scope") {
            Ok(scope) => scope,
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let project_label = match project_label(obj, "project_name") {
            Ok(label) => label,
            Err(e) => return (fail("invalid_arguments", e), false),
        };
        let explicit_project = obj
            .get("project_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if tier == "orchestrated"
            && obj.get("lead_kind").and_then(Value::as_str) == Some("codex")
            && obj.get("lead_ref_id").and_then(Value::as_str).is_none()
        {
            if let Err(error) = self
                .owners
                .codex_preflight(obj.get("lead_profile").and_then(Value::as_str))
            {
                return (fail("codex_preflight_failed", error), false);
            }
        }
        let (account_id, project_id, project_name) = match project_scope {
            Some(scope) => match self.bind_project(scope, project_label, explicit_project) {
                Ok((account, id, name)) => (Some(account), Some(id), Some(name)),
                Err(e) => return (fail("project_binding_failed", e), false),
            },
            None if project_label.is_some() => {
                return (
                    fail("invalid_arguments", "project_name requires project_scope"),
                    false,
                )
            }
            None => (
                self.current_account_id(),
                explicit_project.map(ToOwned::to_owned),
                None,
            ),
        };
        let mut warnings = Vec::new();

        // Resolve optional Front chat (explicit id or marker) and apply `[F1] <title>`.
        let mut front_chat_id = obj
            .get("front_chat_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);
        if front_chat_id.is_none() {
            if let Some(marker) = obj
                .get("front_marker")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                match self.resolve_front_chat_by_marker(marker, project_id.as_deref()) {
                    Ok(cid) => front_chat_id = Some(cid),
                    Err(e) => warnings.push(e),
                }
            }
        }

        let now = now_unix();
        let call_id = obj
            .get("call_id")
            .and_then(Value::as_str)
            .expect("validated create call_id");
        let work_id = format!("work_{}", &sha256_hex(call_id.as_bytes())[..32]);
        let mut front_history = Vec::new();
        if let Some(ref fcid) = front_chat_id {
            let ftitle = format_front_title(&session_prefixes, 1, title);
            if let Err(e) = self.owners.chatgpt_rename_chat(fcid, &ftitle) {
                warnings.push(format!("front_rename_skipped: {e}"));
            }
            front_history.push(FrontHistoryEntry {
                generation: 1,
                chat_id: fcid.clone(),
                bound_at_unix: now,
            });
        }

        let mut budget = WorkBudget::default();
        if let Some(turns) = obj.get("lead_rollover_turns").and_then(Value::as_u64) {
            budget.lead_rollover_turns = turns.min(1000) as u32;
        }
        if let Some(mw) = obj.get("max_wakeups").and_then(Value::as_u64) {
            budget.max_wakeups = mw.clamp(1, 200) as u32;
        }
        if let Some(ms) = obj.get("max_wall_secs").and_then(Value::as_u64) {
            budget.max_wall_secs = ms.clamp(60, 86_400);
        }
        if let Some(mc) = obj.get("max_concurrent_workers").and_then(Value::as_u64) {
            budget.max_concurrent_workers = mc.clamp(1, 16) as u32;
        }

        let account_id = account_id.or_else(|| self.current_account_id());
        let mut work = WorkRecord {
            extra: Default::default(),
            schema: WORK_SCHEMA.to_owned(),
            work_id: work_id.clone(),
            title: title.to_owned(),
            tier: tier.to_owned(),
            goal: goal.to_owned(),
            completion_criteria: criteria,
            state: "running".to_owned(),
            account_id,
            project_id: project_id.clone(),
            project_name: project_name.clone(),
            project_scope: project_scope.map(ToOwned::to_owned),
            session_prefixes,
            delivery_mode: delivery_mode.to_owned(),
            front: FrontBinding {
                latest_chat_id: front_chat_id.clone(),
                generation: if front_chat_id.is_some() { 1 } else { 0 },
                history: front_history,
            },
            lead: None,
            pending_lead: None,
            lead_rollover: None,
            lead_replay: None,
            participants: Vec::new(),
            steps,
            verification_policy,
            review_cwd: None,
            wait_set: None,
            pending_deliveries: Vec::new(),
            consumed_seq: 0,
            last_seq: 0,
            budget,
            created_at_unix: now,
            updated_at_unix: now,
        };

        let _lock = match self.ledger.lock_work(&work_id) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        if let Err(e) = self.ledger.append_event_if_absent(
            &mut work,
            "work:created",
            "work.created",
            "front",
            &format!("Work '{title}' created (tier={tier})"),
            json!({ "tier": tier, "project_id": project_id }),
        ) {
            return (fail("ledger_event_write_failed", e.to_string()), false);
        }
        let initial_cp = format!(
            "# Checkpoint: {title}\n\n- work_id: `{work_id}`\n- tier: `{tier}`\n- goal: {goal}\n"
        );
        if let Err(e) = self.ledger.write_checkpoint(&work_id, &initial_cp) {
            return (fail("ledger_checkpoint_write_failed", e.to_string()), false);
        }
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }

        let mut status = "ok";
        if tier == "orchestrated" {
            match self.spawn_or_bind_lead(&mut work, 1, obj, &mut warnings) {
                Ok(true) => (),
                Ok(false) => status = "pending",
                Err(mut failure) => {
                    failure["work_id"] = json!(work_id);
                    return (failure, true);
                }
            }
        }
        (
            json!({
                "state": status,
                "work_id": work_id,
                "title": title,
                "tier": tier,
                "project_id": work.project_id,
                "project_name": work.project_name,
                "project_scope": work.project_scope,
                "session_prefixes": work.session_prefixes,
                "front_chat_id": work.front.latest_chat_id,
                "lead": work.lead,
                "delivery_mode": work.delivery_mode,
                "warnings": warnings
            }),
            true,
        )
    }

    pub(super) fn spawn_or_bind_lead(
        &self,
        work: &mut WorkRecord,
        generation: u32,
        obj: &Map<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<bool, Value> {
        if work.pending_lead.is_some() {
            return Ok(false);
        }
        let kind = obj
            .get("lead_kind")
            .and_then(Value::as_str)
            .unwrap_or("chatgpt");
        if !matches!(kind, "chatgpt" | "codex") {
            return Err(fail(
                "invalid_arguments",
                "'lead_kind' must be chatgpt|codex",
            ));
        }
        let mut lead = LeadBinding {
            kind: kind.to_owned(),
            ref_id: String::new(),
            profile: obj
                .get("lead_profile")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            model: obj
                .get("lead_model")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            reasoning_effort: obj
                .get("lead_reasoning_effort")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            generation,
            history: Vec::new(),
            rollover_internal_turns: 0,
            dispatch_id: None,
            turn_id: None,
        };
        if let Some(existing) = obj
            .get("lead_ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            lead.ref_id = existing.to_owned();
        } else {
            if kind == "chatgpt" {
                let account = self
                    .owners
                    .chatgpt_account_id()
                    .map_err(|e| fail("lead_account_unavailable", e))?;
                if work
                    .account_id
                    .as_ref()
                    .is_some_and(|pinned| pinned != &account)
                {
                    return Err(fail(
                        "account_mismatch",
                        "Work belongs to a different account",
                    ));
                }
                work.account_id = Some(account);
            }
            let id = generate_id("dispatch");
            lead.dispatch_id = Some(id.clone());
            work.pending_lead = Some(lead.clone());
            self.ledger
                .save_work(work)
                .map_err(|e| fail("ledger_write_failed", e.to_string()))?;
            let criteria = work.completion_criteria.join("\n- ");
            let mcp_note = if kind == "chatgpt" {
                " Use connected MCP tools (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`, `@TermuxOrch`) in single-turn mode, avoid retired `@TermuxBridge`, and never target your own currently generating chat via `@TermuxChatGPT`."
            } else {
                ""
            };
            let prompt = format!(
                "[ORCH_DISPATCH {id}]\nYou are Lead [L{generation}] for durable Work `{}` (\"{}\").\n\
                 Goal: {}\nCompletion criteria:\n- {criteria}\n\n\
                 Use the available owner tools to dispatch scoped Workers and Verifiers (`backend`: `codex` | `chatgpt` | `shell`).{mcp_note} Register them and arm \
                 the Wait-Set with @TermuxOrch, then end your turn. Report milestones, blockers, and verified \
                 completion via orch_report. Include expected_lead_generation={generation} in Work mutations.",
                work.work_id, work.title, work.goal
            );
            let result = if kind == "codex" {
                self.owners
                    .codex_dispatch_turn(
                        lead.profile.as_deref(),
                        None,
                        obj.get("lead_cwd").and_then(Value::as_str),
                        &prompt,
                        lead.model.as_deref(),
                        obj.get("lead_model_provider").and_then(Value::as_str),
                        lead.reasoning_effort.as_deref(),
                        None,
                        |tid| {
                            if let Some(pending) = work.pending_lead.as_mut() {
                                pending.ref_id = tid.to_owned();
                            }
                            self.ledger.save_work(work).map_err(|e| e.to_string())
                        },
                    )
                    .and_then(|(tid, turn)| {
                        turn.map(|turn| (tid, Some(turn)))
                            .ok_or("missing turn ID".to_owned())
                    })
            } else {
                self.owners
                    .chatgpt_dispatch_participant(
                        work,
                        &id,
                        &prompt,
                        lead.model.as_deref(),
                        lead.reasoning_effort.as_deref(),
                    )
                    .and_then(|receipt| {
                        if crate::rollover::rejected_before_send(&receipt) {
                            work.pending_lead = None;
                            self.ledger.save_work(work).map_err(|e| e.to_string())?;
                        }
                        receipt["chat_id"]
                            .as_str()
                            .filter(|cid| {
                                !cid.is_empty()
                                    && matches!(
                                        receipt["state"].as_str(),
                                        Some("settled" | "in_progress")
                                    )
                            })
                            .map(|cid| (cid.to_owned(), None))
                            .ok_or("missing confirmed Lead chat ID".to_owned())
                    })
            };
            match result {
                Ok((ref_id, turn_id)) => {
                    lead.ref_id = ref_id;
                    lead.turn_id = turn_id;
                }
                Err(error) if work.pending_lead.is_some() => {
                    warnings.push(format!("lead_dispatch_pending: {error}"));
                    return Ok(false);
                }
                Err(error) => return Err(fail("lead_dispatch_rejected", error)),
            }
        }
        let title = format_lead_title(&work.session_prefixes, generation, &work.title);
        work.bind_lead(lead.clone());
        self.ledger
            .save_work(work)
            .map_err(|e| fail("ledger_write_failed", e.to_string()))?;
        let rename = if kind == "chatgpt" {
            self.owners
                .chatgpt_rename_chat(&lead.ref_id, &title)
                .map(|_| ())
        } else {
            self.owners
                .codex_rename_thread(lead.profile.as_deref(), &lead.ref_id, &title)
        };
        if let Err(error) = rename {
            warnings.push(format!("lead_rename_skipped: {error}"));
        }
        Ok(true)
    }

    pub(super) fn op_work_promote(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (
                fail("invalid_work_id", "valid work_id is required for promote"),
                false,
            );
        };
        let _lock = match self.ledger.lock_work(wid) {
            Ok(l) => l,
            Err(e) => return (fail("ledger_lock_failed", e.to_string()), false),
        };
        let mut work = match self.ledger.load_work(wid) {
            Ok(w) => w,
            Err(e) => return (fail("work_not_found", e.to_string()), false),
        };
        if !work.can_start()
            || work
                .wait_set
                .as_ref()
                .is_some_and(|w| w.armed && w.then_verify_shell.is_some())
        {
            return (fail("work_unavailable", "Work cannot start a Lead"), false);
        }
        if let Err(e) = check_lead_fence(&work, obj, work.lead.is_some()) {
            return (e, false);
        }
        if work.tier == "orchestrated" && work.lead.is_some() {
            return (
                json!({
                    "state": "ok",
                    "work_id": work.work_id,
                    "tier": "orchestrated",
                    "already_promoted": true,
                    "lead": work.lead
                }),
                true,
            );
        }
        let mut warnings = Vec::new();
        match self.spawn_or_bind_lead(&mut work, 1, obj, &mut warnings) {
            Ok(true) => (),
            Ok(false) => {
                return (
                    json!({"state":"pending","work_id":wid,"warnings":warnings}),
                    true,
                )
            }
            Err(error) => return (error, false),
        }
        let lead = work.lead.clone().unwrap();
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:promoted:1",
            "work.promoted",
            "front",
            &format!(
                "Promoted Tier 1 direct work to Tier 3 orchestrated ({}:{})",
                lead.kind, lead.ref_id
            ),
            json!({ "lead": lead }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "tier": work.tier,
                "lead": work.lead,
                "preserved_participants": work.participants.len(),
                "warnings": warnings
            }),
            true,
        )
    }

    pub(super) fn op_work_rebind_front(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (
                fail(
                    "invalid_work_id",
                    "valid work_id is required for rebind_front",
                ),
                false,
            );
        };
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

        let Some(pinned) = work.account_id.as_deref() else {
            return (
                fail("pinned_account_required", "Work has no ChatGPT account pin"),
                false,
            );
        };
        match self.owners.chatgpt_account_id() {
            Ok(current) if current == pinned => (),
            Ok(_) => {
                return (
                    fail("account_mismatch", "Work belongs to a different account"),
                    false,
                )
            }
            Err(_) => {
                return (
                    fail(
                        "account_unavailable",
                        "The active ChatGPT account could not be confirmed",
                    ),
                    false,
                )
            }
        }
        let resolved_chat_id = if let Some(cid) = obj
            .get("front_chat_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            cid.to_owned()
        } else if let Some(marker) = obj
            .get("front_marker")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            match self.resolve_front_chat_by_marker(marker, work.project_id.as_deref()) {
                Ok(cid) => cid,
                Err(e) => return (fail("front_resolve_failed", e), false),
            }
        } else {
            return (
                fail(
                    "invalid_arguments",
                    "rebind_front requires front_chat_id or front_marker",
                ),
                false,
            );
        };

        match self.owners.chatgpt_read_chat(&resolved_chat_id, 1) {
            Ok(chat) if chat["state"] == "ok" && chat["chat_id"] == resolved_chat_id => (),
            _ => {
                return (
                    fail(
                        "front_chat_unverified",
                        "The active account could not read the target chat",
                    ),
                    false,
                )
            }
        }
        if !self
            .owners
            .chatgpt_account_id()
            .is_ok_and(|current| current == pinned)
        {
            return (
                fail(
                    "account_unavailable",
                    "ChatGPT account changed during verification",
                ),
                false,
            );
        }

        let now = now_unix();
        if work.front.latest_chat_id.as_deref() == Some(&resolved_chat_id) {
            return (
                json!({"state":"ok", "work_id":wid, "front_generation":work.front.generation,
                "front_chat_id":resolved_chat_id, "work_state":work.state}),
                true,
            );
        }
        let next_gen = work.front.generation + 1;
        work.front.generation = next_gen;
        work.front.latest_chat_id = Some(resolved_chat_id.clone());
        work.front.history.push(FrontHistoryEntry {
            generation: next_gen,
            chat_id: resolved_chat_id.clone(),
            bound_at_unix: now,
        });

        let mut warnings = Vec::new();
        let ftitle = format_front_title(&work.session_prefixes, next_gen, &work.title);
        if let Err(e) = self.owners.chatgpt_rename_chat(&resolved_chat_id, &ftitle) {
            warnings.push(format!("front_rename_skipped: {e}"));
        }

        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("rebind:{next_gen}:{resolved_chat_id}"),
            "front.rebound",
            &format!("F{next_gen}"),
            &format!("Front rebound to F{next_gen} ({resolved_chat_id})"),
            json!({ "generation": next_gen, "chat_id": resolved_chat_id }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        let cp = self.ledger.read_checkpoint(wid).ok().flatten();
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "front_generation": next_gen,
                "front_chat_id": resolved_chat_id,
                "front_title": ftitle,
                "work_state": work.state,
                "tier": work.tier,
                "participants": work.participants,
                "checkpoint_md": cp,
                "warnings": warnings
            }),
            true,
        )
    }

    pub(super) fn op_work_update_scope(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
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
        match parse_session_prefixes(obj) {
            Ok(Some(prefixes)) => work.session_prefixes = prefixes,
            Ok(None) => {}
            Err(e) => return (fail("invalid_arguments", e), false),
        }
        if let Some(g) = obj
            .get("goal")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            work.goal = g.to_owned();
        }
        if let Some(arr) = obj.get("completion_criteria").and_then(Value::as_array) {
            work.completion_criteria = arr
                .iter()
                .filter_map(Value::as_str)
                .map(|s| truncate_chars(s.trim(), 300))
                .filter(|s| !s.is_empty())
                .take(20)
                .collect();
        }
        let reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("scope updated by user");
        let cid = obj.get("call_id").and_then(Value::as_str).unwrap_or("upd");
        let scope_data = json!({
            "goal": work.goal,
            "completion_criteria": work.completion_criteria,
            "session_prefixes": work.session_prefixes
        });
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("scope:{cid}"),
            "user.scope_updated",
            "front",
            reason,
            scope_data,
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "goal": work.goal,
                "completion_criteria": work.completion_criteria,
                "last_seq": work.last_seq
            }),
            true,
        )
    }

    pub(super) fn op_work_cancel(&self, obj: &Map<String, Value>) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
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
        if work.is_terminal() {
            return (fail("work_already_terminal", &work.state), false);
        }
        work.state = "cancelling".to_owned();
        if let Some(ws) = work.wait_set.as_mut() {
            ws.armed = false;
        }
        work.pending_deliveries
            .retain(|delivery| delivery.dispatch.is_some());
        let reason = obj
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("cancelled by user");
        let _ = self.ledger.append_event_if_absent(
            &mut work,
            "work:cancel_requested",
            "work.cancel_requested",
            "front",
            reason,
            json!({ "reason": reason }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "work_state": work.state
            }),
            true,
        )
    }
}
