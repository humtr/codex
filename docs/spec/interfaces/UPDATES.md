# Bare interactive launch update discovery

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


The exact zero-argument bare `codex` launch may perform update discovery only
when stdin, stdout, and stderr are all terminals. Any non-empty argument vector,
including `update`, `doctor`, `termux`, `exec`, `--version`, `-V`, and
all other upstream commands/options, bypasses this discovery path completely.
A bare launch with any non-terminal standard stream also bypasses it completely.
Those bypasses perform no startup-update network request, prompt, or advisory
state write.

Discovery is advisory and never becomes update authority. It authenticates only
the normal signed stable index pair under the already installed official
`update_key` and compares the signed generation identity with the active
generation. It does not fetch release payloads, stage a candidate, probe a
candidate, change activation state, or execute an untrusted candidate. The
startup fetch is separately bounded to at most two seconds per signed-index
resource. Discovery failures, malformed advisory cache state, clock failure, or
unavailable transport fail open to the existing active upstream runtime without
a user-visible error; the explicit `codex update` command retains its existing
fail-closed update semantics.

A successful startup discovery is cached for six hours. Transport or discovery
failure suppresses another startup network attempt for thirty minutes. When a
different signed generation is available, Core may show one transient all-TTY
prompt:

```text
Codex update available. Update now? [y/N] 5s
```

The prompt names no candidate version before authenticated generation metadata
is available through the normal update path. On Linux/Android TTYs, Core enters
a bounded non-canonical, no-echo prompt mode only for this advisory: a single
`y` or `Y` key selects update immediately without Enter, while `n`/`N`, Enter,
or any other single input byte keeps the current runtime immediately. Five
seconds with no input also keeps the current runtime. Core restores the exact
prior terminal mode before update execution or upstream launch, consumes the
prompt byte so it cannot leak into upstream Codex input, and clears the
countdown line in place on every normal exit. A keep/timeout decision snoozes
that exact signed generation identity for six hours; a different signed
generation is not suppressed by the old snooze. An effective rollback hold or
rollback-Core guard suppresses the startup prompt entirely; explicit
`codex update` / `--force` remains the recovery authority.

Selecting update re-enters the ordinary no-argument `codex update` path from
the beginning, including fresh signed-index authentication and all normal
signature, digest, mode, anti-rollback, candidate-probe, atomic-activation, LKG,
and rollback protections. On success, bare launch then loads and executes the
newly active upstream runtime. If that explicit-in-response-to-the-prompt update
fails, its ordinary concise failure remains visible but bare launch still
continues with the unchanged active runtime. Startup advisory cache/snooze state
is Core-owned convenience state only and is never trusted as release,
activation, rollback, or signing authority.

Rollback is an explicit Core operation, not an ordinary-launch fallback and not
a search through generation history. It selects the one retained previous complete
generation, including explicit reactivation of a newer previous generation.
After an authenticated public rollback to a lower release sequence commits, Core
atomically records a separate bounded exact-format `update-hold` file containing
only the authenticated generation identity and signed release sequence that were
current before rollback. Activation-state v3 is not extended. The hold file must
be a regular file, must reject malformed/oversized/symlink state, and must use
temporary-create, file sync, atomic rename, and parent-directory sync semantics.
A failed rollback must not create or change the hold. Explicit selection of an
equal or newer previous sequence creates no hold for the older generation and
clears obsolete hold/guard controls only after the existing activation transaction
commits; an existing hold for that selected sequence remains authenticated while
its rollback guard is removed. It never weakens qualification of that retained target.

The first rollback from a hold-aware Core to an older Core that does not advertise
exact `codex-update-hold-v1` capability must not lose hold enforcement. Core may
therefore retain the rolled-back-from signed Core launcher as a bounded rollback
control-plane guard while the runtime, Manager, helpers, and authoritative current
pointer roll back to the retained previous signed generation. The guard is a
separate exact-format Core-owned state record; it must bind the rollback target
generation, the held generation, the held signed release sequence, and the held
generation's signed Core digest. The stable launcher is accepted under this
exception only when the authoritative pointer pair matches the guard, the held
previous generation verifies under its retained verifier key, and the launcher
digest exactly equals that held signed Core digest. No unsigned or arbitrary Core
path is authorized. A target Core that advertises `codex-update-hold-v1` uses the
normal full Core rollback instead and does not retain the guard.

Core writes the guard intent durably before the pointer swap but treats it as an
effective hold only after the authoritative pointer becomes the exact guarded
rollback pair. This closes the commit-to-hold crash window without treating a
failed rollback as held. After the committed rollback, Core writes the normal
`update-hold`; a hold-aware target then removes the temporary guard, while a
legacy target retains it until a force retry or greater-sequence activation
restores a normal Core/generation pairing. Malformed, mismatched, stale, or
unverifiable guard state fails closed whenever it is needed to justify a launcher
mismatch.

Ordinary update applies signature, digest, and release-sequence validation before
the hold. After rollback, an otherwise eligible candidate that does not exceed the
held signed release sequence must not stage, probe, or activate; the hold therefore
prevents both the exact bad sequence and any intermediate sequence from displacing
the retained evidence. A differently named same-sequence repack cannot bypass it.
A greater release sequence is eligible normally. After a greater-sequence
activation commits, Core removes the now-obsolete hold and guard while still
inside the activation writer boundary; a failed activation must not clear them.
`codex update --force` requires a valid authenticated hold and may retry exactly
that held sequence for that invocation. It bypasses only the hold comparison; a
same-held-sequence force retry retains the normal hold while removing any now-stale
rollback-Core guard after the normal Core/generation pairing is restored. Force
never enters the transport-unavailable local-build fallback and cannot target an
unheld or greater sequence. Neither path weakens signature, digest, anti-rollback,
candidate-probe, or atomic-activation rules.

Internal release IDs, component digests, API versions, and schema versions are
still mandatory for update, diagnosis, and rollback. They may appear only on a
clearly Termux-specific status or redacted machine-diagnostic surface.
