use super::*;

impl OrchMcpHandler {
    pub(super) fn handle_dispatch(&self, args: &Value) -> Value {
        let Some(obj) = args.as_object() else {
            return fail("invalid_arguments", "arguments must be a JSON object");
        };
        if let Err(e) = check_allowed_keys(obj, DISPATCH_ALLOWED_KEYS) {
            return fail("invalid_arguments", e);
        }
        let Some(action) = obj.get("action").and_then(Value::as_str) else {
            return fail("invalid_arguments", "missing action");
        };
        self.with_call_id_receipt(args, &format!("orch_dispatch:{action}"), || match action {
            "register_worker" => self.op_register_participant(obj, "worker"),
            "register_verifier" => self.op_register_participant(obj, "verifier"),
            "wait_set" => self.op_wait_set(obj),
            "wait_clear" => self.op_wait_clear(obj),
            other => (
                fail("invalid_arguments", format!("unsupported action '{other}'")),
                false,
            ),
        })
    }

    pub(super) fn op_register_participant(
        &self,
        obj: &Map<String, Value>,
        role: &str,
    ) -> (Value, bool) {
        let Some(wid) = obj
            .get("work_id")
            .and_then(Value::as_str)
            .filter(|s| is_safe_work_id(s))
        else {
            return (fail("invalid_work_id", "valid work_id is required"), false);
        };
        let Some(backend) = obj
            .get("backend")
            .and_then(Value::as_str)
            .filter(|b| matches!(*b, "codex" | "chatgpt" | "shell"))
        else {
            return (
                fail("invalid_arguments", "'backend' must be codex|chatgpt|shell"),
                false,
            );
        };
        let slug = obj
            .get("slug")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(if role == "verifier" { "verify" } else { "task" });
        let task_summary = obj
            .get("task_summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("scoped subtask");
        let profile = obj
            .get("profile")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let owner_scope = obj
            .get("owner_scope")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let call_id = obj
            .get("call_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

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
            return (
                fail(
                    "work_already_terminal",
                    format!("work '{wid}' is already {}", work.state),
                ),
                false,
            );
        }
        if work.participants.iter().any(|p| p.call_id == call_id) {
            return (
                fail(
                    "participant_call_already_registered",
                    "Use the original participant",
                ),
                false,
            );
        }
        if work
            .wait_set
            .as_ref()
            .is_some_and(|ws| ws.armed && ws.then_verify_shell.is_some())
        {
            return (
                fail(
                    "continuation_armed",
                    "The fixed continuation owns this Work",
                ),
                false,
            );
        }
        if role == "worker" && work.active_worker_count() >= work.budget.max_concurrent_workers {
            return (
                fail(
                    "max_concurrent_workers_reached",
                    format!(
                        "active worker ceiling ({}) reached",
                        work.budget.max_concurrent_workers
                    ),
                ),
                false,
            );
        }

        let index = work.next_role_index(role);
        let tag = if role == "verifier" { "V" } else { "W" };
        let participant_id = format!("{tag}{index}");
        let session_title =
            format_participant_title(&work.session_prefixes, role, index, slug, task_summary);
        let verification = (role == "verifier").then(|| Verification {
            subject: work.verification_subject(),
            challenge: generate_id("verify"),
            verdict: None,
            evidence: Vec::new(),
            blocking_issues: vec![],
        });

        let existing = obj
            .get("ref_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let shell_command = obj.get("command").and_then(Value::as_str);
        let prompt = obj.get("prompt").and_then(Value::as_str);
        let isolation = match obj
            .get("isolation")
            .cloned()
            .map(serde_json::from_value::<Isolation>)
            .transpose()
        {
            Ok(Some(value)) => value,
            Ok(None) if obj.get("read_only").and_then(Value::as_bool) == Some(true) => {
                Isolation::Readonly
            }
            Ok(None) => Isolation::SharedSerial,
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        let write_paths = match obj
            .get("write_paths")
            .cloned()
            .map(serde_json::from_value::<Vec<String>>)
            .transpose()
        {
            Ok(paths) => paths.unwrap_or_default(),
            Err(error) => return (fail("invalid_arguments", error.to_string()), false),
        };
        if let Err(error) = crate::workspace::validate_paths(&write_paths) {
            return (fail("invalid_arguments", error), false);
        }
        if isolation == Isolation::Readonly && !write_paths.is_empty() {
            return (
                fail(
                    "invalid_arguments",
                    "readonly workers cannot declare write_paths",
                ),
                false,
            );
        }
        if role == "verifier"
            && existing.is_some_and(|id| {
                work.lead
                    .as_ref()
                    .is_some_and(|lead| lead.kind == backend && lead.ref_id == id)
                    || work
                        .participants
                        .iter()
                        .any(|p| p.role == "worker" && p.backend == backend && p.ref_id == id)
            })
        {
            return (
                fail(
                    "verifier_not_independent",
                    "The verifier cannot reuse an implementation session",
                ),
                false,
            );
        }
        let cwd = obj.get("cwd").and_then(Value::as_str);
        let scope = if role == "worker" && existing.is_none() && isolation != Isolation::Readonly {
            match cwd
                .map(|path| crate::workspace::root(std::path::Path::new(path)))
                .transpose()
            {
                Ok(scope) => scope,
                Err(error) => return (fail("invalid_workspace", error), false),
            }
        } else {
            None
        };
        let _workspace_lock = if scope.is_some() {
            match self.ledger.lock_writers() {
                Ok(lease) => Some(lease),
                Err(error) => return (fail("workspace_lock_failed", error.to_string()), false),
            }
        } else {
            None
        };
        let (write_workspace, worktree) = if isolation == Isolation::Worktree {
            let Some(repo) = scope
                .as_deref()
                .filter(|path| same_workspace_root(cwd, path))
            else {
                return (
                    fail(
                        "invalid_workspace",
                        "worktree requires a new worker at the repository root",
                    ),
                    false,
                );
            };
            if backend == "chatgpt" || write_paths.is_empty() {
                return (
                    fail(
                        "invalid_workspace",
                        "worktree requires shell or Codex and declared write_paths",
                    ),
                    false,
                );
            }
            if work.participants.iter().any(|p| {
                !p.is_terminal()
                    && p.worktree.as_ref().is_some_and(|tree| tree.repo == repo)
                    && p.write_paths.iter().any(|path| {
                        write_paths
                            .iter()
                            .any(|other| crate::workspace::overlaps(path, other))
                    })
            }) {
                return (
                    fail(
                        "worker_workspace_busy",
                        "An active isolated worker already owns those paths",
                    ),
                    false,
                );
            }
            match crate::workspace::WorkerTree::plan(&self.ledger, wid, &participant_id, repo) {
                Ok(tree) => (None, Some(tree)),
                Err(error) => return (fail("invalid_workspace", error), false),
            }
        } else {
            if let Some(path) = &scope {
                match self.ledger.writer_busy(path) {
                    Ok(false) => (),
                    Ok(true) => {
                        return (
                            fail(
                                "worker_workspace_busy",
                                "An active worker owns this workspace",
                            ),
                            false,
                        )
                    }
                    Err(error) => return (fail("workspace_read_failed", error.to_string()), false),
                }
            }
            (scope, None)
        };
        if shell_command.is_some() && backend != "shell" {
            return (
                fail("invalid_arguments", "command requires backend=shell"),
                false,
            );
        }
        if backend == "shell"
            && shell_command
                .or(prompt)
                .is_some_and(|c| c.trim().is_empty() || c.len() > 65536 || c.contains('\0'))
        {
            return (fail("invalid_arguments", "invalid shell command"), false);
        }
        if backend == "shell"
            && [existing, shell_command, prompt]
                .into_iter()
                .flatten()
                .count()
                != 1
        {
            return (
                fail(
                    "invalid_arguments",
                    "shell worker requires exactly one of ref_id, command, or prompt",
                ),
                false,
            );
        }
        let ref_id = if backend == "shell" {
            let receipt_id = existing.unwrap_or(&participant_id);
            if work
                .participants
                .iter()
                .any(|p| p.backend == "shell" && p.ref_id == receipt_id)
            {
                return (fail("shell_receipt_already_registered", receipt_id), false);
            }
            if let Some(existing) = existing {
                if let Err(e) = crate::shell_job::ShellJob::new(&self.ledger, wid, existing)
                    .and_then(|job| job.validate_attach())
                {
                    return (fail("invalid_shell_receipt", e.to_string()), false);
                }
            }
            receipt_id.to_owned()
        } else {
            existing.unwrap_or_default().to_owned()
        };
        let launch = backend != "shell" && existing.is_none();
        if launch && prompt.is_none_or(|p| p.trim().is_empty()) {
            return (
                fail("invalid_arguments", "provide a prompt or existing ref_id"),
                false,
            );
        }
        if launch && backend == "chatgpt" && work.account_id.is_none() {
            return (
                fail("pinned_account_required", "Work has no ChatGPT account pin"),
                false,
            );
        }
        if launch && backend == "chatgpt" {
            let cooldown = self.owners.chatgpt_cooldown_remaining_secs();
            if cooldown > 0 {
                return (
                    fail(
                        "chatgpt_rate_limited",
                        format!("ChatGPT owner is in 429 cooldown for {cooldown}s; retry after cooldown"),
                    ),
                    false,
                );
            }
            let cap = self.owners.max_concurrent_chatgpt();
            let active_chatgpt: usize = self
                .ledger
                .list_works()
                .unwrap_or_default()
                .iter()
                .filter(|w| !w.is_terminal() && w.account_id == work.account_id)
                .map(|w| {
                    w.participants
                        .iter()
                        .filter(|p| !p.is_terminal() && p.backend == "chatgpt")
                        .count()
                })
                .sum();
            if active_chatgpt >= cap {
                return (
                    fail(
                        "chatgpt_concurrency_limit",
                        format!(
                            "active ChatGPT participants for this account ({active_chatgpt}) reached cap ({cap}); wait for a participant to settle or set ORCH_MAX_CONCURRENT_CHATGPT"
                        ),
                    ),
                    false,
                );
            }
        }
        if launch && backend == "codex" {
            if let Err(error) = self.owners.codex_preflight(profile.as_deref()) {
                return (fail("codex_preflight_failed", error), false);
            }
        }
        let now = now_unix();
        let mut participant = Participant {
            extra: Default::default(),
            participant_id: participant_id.clone(),
            index,
            role: role.to_owned(),
            backend: backend.to_owned(),
            ref_id,
            turn_id: None,
            profile,
            slug: slug.to_owned(),
            task_summary: task_summary.to_owned(),
            owner_scope,
            write_workspace,
            step_id: None,
            worktree,
            changed_paths: Vec::new(),
            write_paths,
            call_id,
            dispatch_id: launch.then(|| generate_id("dispatch")),
            state: if launch { "dispatching" } else { "running" }.to_owned(),
            result_summary: None,
            verification,
            created_at_unix: now,
            updated_at_unix: now,
        };
        if let Some(tree) = &participant.worktree {
            work.verification_policy = VerificationPolicy::IndependentReview;
            work.participants.push(participant.clone());
            if let Err(error) = self.ledger.save_work(&work) {
                return (fail("ledger_write_failed", error.to_string()), false);
            }
            if let Err(error) = tree.prepare() {
                let saved = work.participants.last_mut().unwrap();
                saved.state = "failed".to_owned();
                saved.result_summary = Some(error.clone());
                let _ = self.ledger.save_work(&work);
                return (fail("worktree_prepare_failed", error), true);
            }
            work.participants.pop();
        }
        let cwd = participant
            .worktree
            .as_ref()
            .and_then(|tree| tree.path.to_str())
            .or(cwd);
        if let Some(id) = &participant.dispatch_id {
            let position = work.participants.len();
            work.participants.push(participant.clone());
            if let Err(error) = self.ledger.save_work(&work) {
                return (fail("ledger_write_failed", error.to_string()), false);
            }
            drop(_workspace_lock);
            let mut text = format!(
                "[ORCH_DISPATCH {id}]\n{}",
                prompt.unwrap_or_default().trim()
            );
            if let Some(verification) = &participant.verification {
                let result = json!({
                    "challenge": verification.challenge,
                    "verdict": "pass|fail|inconclusive",
                    "evidence_refs": ["exact artifact or test result inspected"],
                    "blocking_issues": []
                });
                text.push_str(&format!(
                    "\n\nReview the requested subject without making changes. Return only a JSON \
                     object of this shape: {result}. Use one verdict, repeat the challenge verbatim, \
                     and cite concrete evidence in a non-empty array."
                ));
            }
            let model = obj.get("model").and_then(Value::as_str);
            let reasoning_effort = obj.get("reasoning_effort").and_then(Value::as_str);
            let result = if backend == "codex" {
                self.owners
                    .codex_dispatch_turn(
                        participant.profile.as_deref(),
                        None,
                        cwd,
                        &text,
                        model,
                        obj.get("model_provider").and_then(Value::as_str),
                        reasoning_effort,
                        obj.get("approval_policy").and_then(Value::as_str),
                        |tid| {
                            work.participants[position].ref_id = tid.to_owned();
                            self.ledger.save_work(&work).map_err(|e| e.to_string())
                        },
                    )
                    .and_then(|(tid, turn_id)| {
                        turn_id
                            .map(|turn| (tid, Some(turn)))
                            .ok_or("missing turn ID".to_owned())
                    })
            } else {
                self.owners
                    .chatgpt_dispatch_participant(&work, id, &text, model, reasoning_effort)
                    .and_then(|receipt| {
                        if crate::rollover::rejected_before_send(&receipt) {
                            work.participants[position].state = "failed".to_owned();
                            work.participants[position].result_summary =
                                receipt["error"].as_str().map(ToOwned::to_owned);
                            self.ledger.save_work(&work).map_err(|e| e.to_string())?;
                            return Err("owner rejected the request before dispatch".to_owned());
                        }
                        receipt["chat_id"]
                            .as_str()
                            .filter(|id| {
                                !id.is_empty()
                                    && matches!(
                                        receipt["state"].as_str(),
                                        Some("settled" | "in_progress")
                                    )
                            })
                            .map(|id| (id.to_owned(), None))
                            .ok_or("missing confirmed ChatGPT chat ID".to_owned())
                    })
            };
            match result {
                Ok((ref_id, turn_id)) => {
                    participant.ref_id = ref_id;
                    participant.turn_id = turn_id;
                    participant.state = "running".to_owned();
                    work.participants[position] = participant.clone();
                    if let Err(error) = self.ledger.save_work(&work) {
                        return (
                            fail("dispatch_confirmation_pending", error.to_string()),
                            true,
                        );
                    }
                }
                Err(error) => {
                    if backend == "codex" && error.starts_with("codex_preflight_failed: ") {
                        let original = &mut work.participants[position];
                        original.state = "failed".to_owned();
                        original.result_summary = Some(error.clone());
                        if let Err(save_error) = self.ledger.save_work(&work) {
                            return (fail("ledger_write_failed", save_error.to_string()), true);
                        }
                    }
                    let original = &work.participants[position];
                    return (
                        json!({"state":if original.state == "failed" {"error"} else {"pending"},
                            "work_id":wid, "participant":original, "detail":error}),
                        true,
                    );
                }
            }
        } else {
            work.participants.push(participant.clone());
        }
        let ref_id = &participant.ref_id;
        let mut warnings = Vec::new();
        match backend {
            "chatgpt" => {
                if let Err(e) = self.owners.chatgpt_rename_chat(ref_id, &session_title) {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            "codex" => {
                if let Err(e) = self.owners.codex_rename_thread(
                    participant.profile.as_deref(),
                    ref_id,
                    &session_title,
                ) {
                    warnings.push(format!("rename_skipped: {e}"));
                }
            }
            _ => {}
        }

        let _ = self.ledger.append_event_if_absent(
            &mut work,
            &format!("registered:{participant_id}:{ref_id}"),
            &format!("{role}.registered"),
            &participant_id,
            &format!("{session_title} ({backend}:{ref_id})"),
            json!({ "participant_id": participant_id, "backend": backend, "ref_id": ref_id }),
        );
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }

        if backend == "shell" && existing.is_none() {
            let command = shell_command.or(prompt).unwrap_or_default();
            let result = crate::shell_job::ShellJob::new(&self.ledger, wid, ref_id)
                .and_then(|job| job.spawn(self.ledger.root(), wid, command, cwd));
            if let Err(e) = result {
                warnings.push(format!("shell_spawn_failed: {e}"));
                return (
                    json!({
                        "state": "error",
                        "error": "shell_spawn_failed",
                        "work_id": wid,
                        "participant": participant,
                        "warnings": warnings
                    }),
                    true,
                );
            }
        }

        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "participant": participant,
                "session_title": session_title,
                "warnings": warnings
            }),
            true,
        )
    }

    pub(super) fn op_wait_set(&self, obj: &Map<String, Value>) -> (Value, bool) {
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
        if !work.can_start() {
            return (
                fail(
                    "work_already_terminal",
                    "cannot arm Wait-Set on terminal work",
                ),
                false,
            );
        }
        if work
            .wait_set
            .as_ref()
            .is_some_and(|ws| ws.armed && ws.then_verify_shell.is_some())
        {
            return (
                fail(
                    "continuation_armed",
                    "Clear the pending continuation or cancel the Work",
                ),
                false,
            );
        }
        let default_target = if work.lead.is_some() { "lead" } else { "front" };
        let wake_target = obj
            .get("wake_target")
            .and_then(Value::as_str)
            .unwrap_or(default_target);
        if wake_target != "lead" && wake_target != "front" {
            return (
                fail("invalid_arguments", "'wake_target' must be lead|front"),
                false,
            );
        }
        if wake_target == "lead" && work.lead.is_none() {
            return (
                fail(
                    "lead_not_bound",
                    "Tier 1 direct work has no Lead; use wake_target='front' or promote first",
                ),
                false,
            );
        }
        let participants = |key| -> Vec<String> {
            obj.get(key)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        let all_of = participants("all_of");
        let any_of = participants("any_of");
        if let Some(id) = all_of
            .iter()
            .chain(&any_of)
            .find(|id| !work.participants.iter().any(|p| &p.participant_id == *id))
        {
            let known = work
                .participants
                .iter()
                .map(|p| p.participant_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return (
                fail(
                    "unknown_participant_id",
                    format!("Unknown participant_id '{id}'; known participant_ids: [{known}]"),
                ),
                false,
            );
        }
        let then_verify_shell = match obj.get("then_verify_shell") {
            Some(value) => match serde_json::from_value::<ShellContinuation>(value.clone()) {
                Ok(next)
                    if !next.command.trim().is_empty()
                        && next.command.len() <= 65536
                        && !next.command.contains('\0')
                        && next.cwd.as_ref().is_none_or(|cwd| {
                            !cwd.is_empty() && cwd.len() <= 512 && !cwd.contains('\0')
                        }) =>
                {
                    Some(next)
                }
                _ => {
                    return (
                        fail(
                            "invalid_arguments",
                            "Invalid then_verify_shell command or cwd",
                        ),
                        false,
                    )
                }
            },
            None => None,
        };
        let on_user_event = obj
            .get("on_user_event")
            .and_then(Value::as_bool)
            .unwrap_or(then_verify_shell.is_none());
        let now = now_unix();
        let deadline_unix = obj
            .get("deadline_secs")
            .and_then(Value::as_u64)
            .map(|s| now + s.clamp(5, 86_400));
        if all_of.is_empty() && any_of.is_empty() && deadline_unix.is_none() {
            return (
                fail(
                    "invalid_wait_set",
                    "Wait-Set requires at least one of all_of, any_of, or deadline_secs",
                ),
                false,
            );
        }
        if then_verify_shell.is_some()
            && (work.tier != "direct"
                || wake_target != "front"
                || all_of.is_empty()
                || !any_of.is_empty()
                || on_user_event)
        {
            return (
                fail(
                    "invalid_wait_set",
                    "Fixed shell verification needs a direct Work and valid all_of prerequisites",
                ),
                false,
            );
        }
        if let Some(cp) = obj.get("checkpoint_md").and_then(Value::as_str) {
            if let Err(error) = self.ledger.write_checkpoint(wid, cp) {
                return (
                    fail("ledger_checkpoint_write_failed", error.to_string()),
                    false,
                );
            }
        }
        let ws = WaitSet {
            extra: Default::default(),
            wait_id: generate_id("wait"),
            wake_target: wake_target.to_owned(),
            all_of,
            any_of,
            on_user_event,
            deadline_unix,
            armed: true,
            created_at_unix: now,
            then_verify_shell,
            continuation_id: None,
        };
        work.wait_set = Some(ws.clone());
        work.state = "waiting".to_owned();
        work.updated_at_unix = now;
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "work_state": work.state,
                "wait_set": ws
            }),
            true,
        )
    }

    pub(super) fn op_wait_clear(&self, obj: &Map<String, Value>) -> (Value, bool) {
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
        if let Some(ws) = work.wait_set.as_mut() {
            if ws.armed && ws.continuation_id.is_some() {
                return (
                    fail(
                        "continuation_started",
                        "Cancel the Work to stop a started continuation",
                    ),
                    false,
                );
            }
            ws.armed = false;
        }
        if work.state == "waiting" {
            work.state = "running".to_owned();
        }
        work.updated_at_unix = now_unix();
        if let Err(e) = self.ledger.save_work(&work) {
            return (fail("ledger_write_failed", e.to_string()), false);
        }
        (
            json!({
                "state": "ok",
                "work_id": work.work_id,
                "wait_armed": false
            }),
            true,
        )
    }
}
