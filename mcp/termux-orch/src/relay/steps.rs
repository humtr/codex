use super::*;

impl RelayEngine {
    pub(super) fn audit_worker_tree(
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

    pub(super) fn process_steps(&self, work: &mut WorkRecord, now: u64) -> Result<(), String> {
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

    pub(super) fn start_plan_review(
        &self,
        work: &mut WorkRecord,
        cwd: &Path,
        now: u64,
    ) -> Result<(), String> {
        let profile = work
            .lead
            .as_ref()
            .filter(|lead| lead.kind == "codex")
            .and_then(|lead| lead.profile.clone());
        if let Err(error) = self.owners.codex_preflight(profile.as_deref()) {
            return self.settle_plan(
                work,
                "waiting_user",
                &format!("codex_preflight_failed: {error}"),
                "front",
            );
        }
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
            profile: profile.clone(),
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

    pub(super) fn settle_plan(
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
}
