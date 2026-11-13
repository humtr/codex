# Handoff — remaining acceptance and operating boundaries

Updated: 2026-11-13 13:54 KST. One current unfinished-work record.
Read [CURRENT](CURRENT.md), [AGENTS](AGENTS.md) and the
[finite integration record](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md).
Continuous authorization covers development, safe branch integration and unsigned
CI. These are not intermediate approval gates.

## Next execution order and mandatory documentation gate

The installed Full S167, selected/running Provider S27 and runit-hosted
Termux tunnel v0.0.20 are the current verified operating baseline; see
[CURRENT](CURRENT.md). The `rewrite/rust-core` source at `0ba058922fd1243f63a7cea2db0653348c64aa1d`
and the isolated tunnel/docs branch `work/tunnel-termux-v0.0.20-20261113`
at `5c495538780e9c84e0c87a0a3c623b0ea3b80498` have **not** yet been
integrated together or signed as a new Full generation. Preserve running
GitHub Actions and existing operating services while doing isolated development.

1. **Complete actual runtime acceptance:** submit a real selected-model
   Provider/AppHost response and bounded Native tool + same-thread reentry;
   inspect old 400/404 behavior on the *actual caller*, not merely malformed
   HTTP probes. Independently verify remote tunnel readiness, ownership and
   no duplicate/replayed effects. Record request/run/owner receipts and
   proof limits in CURRENT. A 200 capability handshake does not prove inference.
2. **Improve CI in parallel:** implement stage dependency ordering and
   immutable exact-input artifact reuse, remove premature Full push failures,
   optimize expensive TUI gates without weakening required upstream/negative
   tests. Keep any already-running qualification intact. Record the changed
   job graph, source/ref scope, cache invariants and regression obligations in
   [the release implementation plan](docs/development/RELEASE_FRAMEWORK_IMPLEMENTATION.md);
   change [the qualification contract](docs/spec/delivery/QUALIFICATION.md)
   only if the approved behavior contract itself changes.
3. **Implement persistent runit generation reconciliation:** follow
   [the focused task](docs/development/RUNIT_GENERATION_ACTIVATION.md).
   Update/rollback should reconcile the opted-in Provider, tunnel and Host
   keeper with the verified selected generations, even after reboot. The
   one-time manual S27/v0.0.20 runit replacement is only operating evidence.
4. **Integrate and accept:** review the isolated tunnel, CI and runit commits
   into the authorized development branch; requalify all required gates for
   the resulting exact source/input identity. Preserve prior successful proofs
   where the contract allows reuse, and do not interpret a running/failed gate
   as success. Only afterward perform authorized official source approval,
   GitHub-native signing, publication, Android installation and rollback smoke.
5. **Documentation is a required deliverable at *each* stage, not a final
   cosmetic step:** before editing, check SPEC and the owning normative annex
   for required behavior and update the appropriate annex only for an actual
   contract change; update the finite implementation plan for CI design and
   implementation ownership; update CURRENT with dated SHA, original CI IDs,
   binary generations, test outputs and explicit unproven limits; keep only
   actionable unfinished items in HANDOFF; update README/component operator
   documentation for supported commands, runit enrollment/update/rollback and
   troubleshooting when the corresponding behavior actually ships. Add test
   fixtures/dependency mappings only for affected real contracts. Run the
   documentation/link check and applicable doc/code consistency tests before
   each cohesive commit. Source, docs, executable behavior and release
   manifest must agree on the final signed source, not an old checkpoint.

**Documentation acceptance:** the task is incomplete if implementation passes
but its owning requirements, operational instructions, actual source/run IDs
or limitations remain stale. Documentation-only updates are not proof of
implementation, live model execution, CI qualification or signed release.

## Remaining source acceptance

1. Complete actual execution on the qualified 1f Runtime and frozen Provider:
   unloaded source-bound observation, cold compiler/Provider restart, selected
   Stop/ACK settlement, independent sibling execution, Core-only default UI,
   independent shell during real Native Busy and existing AutoReview allow/deny
   cases. The complete 53-gate Core qualification is accepted; its original
   package, hook and schema proof is in CURRENT. Actual device execution remains
   separate. Preserve original r1/r2 uncertain IDs/results without resubmission;
   parent-only exit, Stop ACK or absent asynchronous history are not settlement.
2. Finish affected final-composition switch checks after full TUI qualification.
   Earlier J same-thread and old-d2/S165 safe-refusal evidence is limited to its
   original source/inputs. Keep the original user error-screen PID/thread
   attribution separate from the observed read-only Owner; do not infer it.
3. Complete exact-final-source Source, Core/Runtime, Manager, Provider, stock
   Android, full TUI and Full composition qualification. The 1f Core, Manager,
   Provider and stock Android runs succeeded; Source failed in the TLS port
   fixture. Test-only 578a386b corrects atomic port publication and passed six
   download tests plus strict Core Clippy/formatting. TUI 43158539818 failed
   four Clippy diagnostics; the isolated Writer fixed both typed-completion
   expect calls and both redundant attempt closures in the actual Native
   source producer, but hosted requalification remains pending. Final
   Full requires all six exact-source immutable inputs. Preserve one full TUI
   dispatch per source with publication advancement disabled. Final artifact
   digests/producer inputs, not a new SHA label alone, determine evidence reuse.
4. Finish documentation/file ownership accounting, exact-head CI and protected
   ref/asset readbacks. Final source uses the legitimate 0.167.2 recipe; operating
   S165 stays0.167.1. Source acceptance does not require operating activation.
   A running or queued job is not a successful gate.

## Follow-on task: runit generation activation persistence

[Persistent runit generation activation](docs/development/RUNIT_GENERATION_ACTIVATION.md)
tracks the independent, still-unimplemented automatic reconciliation of
Provider and tunnel service runit bindings after verified install, update and
rollback. The one-time S27 service repair is not task completion. Preserve
the existing opted-in runit services and user sessions until their future
transactional update/rollback path is qualified.

## Protected operating acceptance

The candidate is not the operating S165 installation. Actual operating App
inference and same-Chat Native compilation passed. Actual candidate TUI/Native
routing used synthetic inference. Those are separate proof levels.

A private candidate cannot borrow the authenticated operating AppHost owner or
installation-scoped registry/Runtime. Future candidate live-App acceptance needs
separately authorized activation or a separately authenticated private AppHost.
Registry links, owner copying and service replacement are not workarounds.

Stock Android CI proves Activity/session behavior with synthetic Core transport/
frontend. Physical foreground visibility, screen-lock/background restrictions,
the user's original error-screen mapping and a real authenticated candidate
account switch remain separate device/account acceptance.

An early pre-isolation fixture may have created one inert terminal: count one
to two, exact mapping unavailable. Preserve its evidence and do not close an
unidentified operating surface. No later physical focus/close was attempted.

Existing S164/S165 and prior signed generations, stable indexes, formal signing/
publication, operating installation/service changes, account/token changes,
real SQLite/JSONL repairs and active Writer transfer remain separate operations.
Read-only data discrepancies were not repaired or attributed to Native Busy.

The primary e4f9ab5d checkout and other workers' files remain untouched.
Integrated branch classification is not deletion authority. Unknown receipts
retain original identities; uncertain Owner/effects remain preserved.
