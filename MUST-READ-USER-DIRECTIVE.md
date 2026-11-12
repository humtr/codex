# MUST READ — user-authorized connectivity contingency (2026-11-12)

**Time-sensitive fact:** The user expects network connectivity to Termux to be lost from approximately **11:00 KST on 2026-11-12**. The actual outage and restoration must be observed. This file addresses the *existing W0–W20 release-framework integration and its Models, Native, Release and Integration workers*. It does not start a new task, authorize installation/deployment, or override actual ownership, approvals or existing product requirements.

**Explicit delivery restriction:** Do **not** enqueue a message, submit new inference, join the thread to send instructions, or deliver direct text to the **Pro orchestrator**. That would consume the user's Pro turn. This file is an indirect instruction to read on the next normal documentation check, compaction, handoff or recovery. It **cannot force an already-running model to read it**, and no one may claim it has been read without evidence.

## Confirmed independent GitHub recovery refs

As of 2026-11-12 10:50 KST, each of these **non-publication GitHub refs** was pushed and its remote SHA verified. On any remote fallback, **read the actual remote HEAD again**; worker commits or snapshots can advance after this index was written. Do not presume a backup branch is an accepted development source.

- Integrated candidate + this notice: `contingency/termux-outage-20261112`
- Models last committed checkpoint: `contingency/termux-outage-20261112-models`
- Native base last committed checkpoint: `contingency/termux-outage-20261112-native-base`
- Native Provider last committed checkpoint: `contingency/termux-outage-20261112-native-provider`
- Release last committed checkpoint: `contingency/termux-outage-20261112-release`
- Native completion producer last committed checkpoint: `contingency/termux-outage-20261112-producer`
- **Unverified WIP safety snapshot only** of Native Provider's 13 local dirty paths (11 modified, 2 newly created Rust files): `contingency/termux-outage-20261112-native-provider-wip` (snapshot `7d42beee`). Created from a consistent read-only diff/new-file copy in a separate worktree; the original worker's dirty files and Git index are unchanged. **Do not merge/promote this WIP snapshot without owner-controlled review/tests.** The worker may continue editing later, so this snapshot is not automatically current.

## Treat loss of connectivity as transport uncertainty, not a dead end

- A failed `@TermuxBridge`, MCP operation, disconnected stream, timeout, failed observer or missing ACK after the outage **does not establish** a Codex bug, `native_resource_busy`, an irrecoverable Stop/Owner barrier, a failed local build or stopped GitHub Actions. Mark the observation as *transport unavailable/uncertain* until source-specific evidence says otherwise.
- Keep the **original** work/thread/source identities, call IDs, reservations and saved receipts. Do not blindly retry unknown-delivery commands with new IDs, re-run effects, manufacture a success, force-stop a Native process, change owner/account/approval policies, or use a new session to bypass uncertain execution.
- Already-running local Cargo/Native tests may finish without a network. GitHub Actions already accepted by GitHub can continue remotely even if this device disconnects. Never kill/cancel either category just because the observer cannot reach it.

## While Termux is available — continue and preserve

1. Continue the approved work normally; do **not** preemptively halt current CI or worker tests. At safe ownership boundaries, commit each completed, coherent, verified change to its existing owned branch. Preserve dirty/unsettled work and evidence; never clean active worktrees.
2. Before device connectivity goes down, preserve a **separate remote GitHub contingency branch** for the latest safe integrated source snapshot. Record its exact SHA and confirm the remote received it. A snapshot may become stale as workers commit later; update it only by non-forced safe push after checking HEAD and other workers' ownership. Do not push unqualified source to `main`, alter protected release indexes, or sign, publish or activate a candidate.
3. Maintain current authority: `AGENTS.md` for workflows; `CURRENT.md` for observed evidence; `HANDOFF.md` for unfinished acceptance; the finite integration record for W-task writers. Do not duplicate their authority in this temporary file or falsely count an older CI run as proof for a newer SHA.

## If TermuxBridge disconnects but a GitHub route remains available

1. Use **only an already authorized independent GitHub connector/API**, if actually accessible to the current orchestrator without new authentication or approval circumvention. Work from the verified remote contingency branch/SHA. Inspect remote commits, CI jobs/results and errors; safely review, propose and commit fixes on a **separate remote fallback branch** when the exact source, CI evidence and file ownership are known. Continue source review and remote CI work that does not depend on unretrieved Termux state.
2. The fallback is conditional, **not a promise of autonomous offline Pro inference**. If Pro model access or GitHub tools are also unavailable, nothing can continue automatically; preserve state and resume after reconnection. Do not claim GitHub source review substitutes for local Native/physical Android proof.
3. Device-local workers may still be writing while unreachable. Do not overwrite their lines or reassign their writer rights. Leave source provenance, branch separation and a conflict-safe integration plan so all remote changes can be compared to local work later.
4. Do not equate unavailable CI status with failure. Preserve exact job IDs, check whether queued/in_progress/completed is actually known, and do not trigger duplicate jobs merely because the observer disconnected.

## Reconnect without replay

- While an existing orchestrator is actually running and has tool access, perform an occasional **non-mutating** TermuxBridge reachability/status probe at reasonable work boundaries (e.g. every 5–10 minutes, not a tight retry loop). No chat/model response can schedule its own work after it has stopped; do not represent this as autonomous monitoring.
- Once connectivity **and the original Native work/receipt** are verified, return to the existing Termux worktrees and original owners. Reread `AGENTS.md`, actual branch/HEAD/status and `CURRENT.md`/`HANDOFF.md`; reconcile original command IDs, pending CI/builds and unknown effects before any continuation. Fetch the remote fallback branch, compare changes, merge/cherry-pick only at valid ownership boundaries, without forced reset, replay, stop, or overriding live local workers.
- Preserve installed **Full S165 / Codex 0.167.1**, rollback generations, services, logins, sdcard and user work. The **0.167.2** source candidate remains unsigned/unpublished until its own final exact-source acceptance and a separately authorized operating release.

**Expiry:** Temporary instructions for this anticipated outage, ending after observed reconnection and state reconciliation. Normal documentation owners may retire this file and its pointers afterward. It is not a permanent SPEC or approval gate.
