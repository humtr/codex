# 1. Product definition

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


The product provides one public `codex` entrypoint that runs the official
upstream Codex CLI correctly on supported Termux environments.

It has four components in one repository: mandatory **Core** for minimal Termux
compatibility and signed runtime lifecycle; optional **Manager** behind
`codex termux` for account/task/window conveniences; optional upstream-based **TUI**
for UI integration; and optional **Provider** for Chat transport/model capabilities.
The minimum execution is upstream plus Core. Tools, approvals, native conversation
records, compaction and writer locks remain upstream-owned. Section4.6 defines the
boundaries; CURRENT distinguishes current signed composition constraints from
source qualification of independently optional components.

Every declared optional artifact must qualify before entering a signed generation.
A missing optional capability cannot disable independently valid Core execution.

This is a clean rewrite. Legacy source is historical evidence, not an
implementation dependency or migration base.

## 2. Product priorities

In descending order:

1. preserve a working, recoverable upstream Codex execution path;
2. never damage system resolver, auth, profile, session, or installed runtime
   state;
3. preserve upstream process behavior at the execution boundary;
4. make installation, update, activation, and rollback crash-safe;
5. keep Core small, fast to build, and fast to execute;
6. add Manager conveniences without giving Manager ownership of Core state;
7. optimize implementation velocity without weakening acceptance gates.

## 3. Public command contract

The launcher classifies only an exact first argument of `update`, `doctor`, or
`termux`. Every invocation with one or more arguments that is not one of those
Core/Manager routes is passed to upstream Codex unchanged after the existing
Termux launch planning. The exact zero-argument bare `codex` invocation has one
narrow Core preflight exception: only when stdin, stdout, and stderr are all
terminals may Core perform the bounded signed-stable update discovery described
below before launching upstream. `update` is a Core-owned safety boundary: the
installed wrapper must never execute the upstream distribution updater, because
that updater can install an unadapted runtime on Termux. The wrapper release
pipeline obtains the official upstream package, applies the accepted Termux
patch, qualifies it, binds the matching Core artifact, and publishes a signed Core-plus-generation bundle; the
installed Core obtains and activates only that signed adapted bundle.

| Command | Owner | Required behavior |
| --- | --- | --- |
| `codex` | Core then upstream | on an all-TTY zero-argument launch, perform only the bounded signed-stable advisory check/prompt below, then execute the active upstream runtime; otherwise execute upstream immediately |
| `codex [UPSTREAM_ARGS...]` | Core | for every non-empty upstream argument vector, execute upstream with original arguments without startup update discovery or prompting |
| `codex --version`, `codex -V` | upstream | print exactly the upstream version output |
| `codex update` | Core | resolve the signed stable wrapper release channel, discover newer official upstream metadata, request its missing hosted build and wait for signed publication; explicit source/transport overrides retain the separately qualified unified acquisition behavior |
| `codex update --help` | Core | print the wrapper-owned update usage without invoking upstream or changing state |
| `codex update --force` | Core | retry exactly the authenticated held public release sequence for this invocation while bypassing only the local rollback-hold comparison; a valid matching hold is required and signature, digest, release-sequence, candidate-probe, and activation checks remain unchanged |
| `codex update --build-local` | Core | resolve the exact official upstream stable metadata/archive, build and probe one local-derived generation, sign with the explicitly configured authorized key or a fresh temporary Ed25519 key, bind to the authenticated public baseline, activate without changing `update_key`, and perform no public upload/promotion |
| `codex update [INVALID_ARGS...]` | Core | reject unsupported updater options or combined selectors without invoking upstream or changing state |
| `codex update --local <DIRECTORY>` | Core | verify, stage, probe, and activate one compatible signed official/operator bundle through ordinary public-authority admission; it is not a local-derived import surface |
| `codex update --remote <HTTPS_BASE_URL>` | Core | acquire one immutable signed Core-plus-generation bundle and activate it through the local coordinated path |
| `codex update --rollback` | Core | atomically activate the retained previous complete signed generation; record a public update hold only when rolling back from an authenticated public generation, while rollback from a local-derived current creates no public hold or rollback guard |
| `codex doctor [OPTIONS]` | Core | combine upstream and Termux diagnostics |
| `codex doctor --color` | Core | explicitly request colored human diagnostics on a TTY, including when an outer Termux wrapper supplied `NO_COLOR` |
| `codex termux [COMMAND]` | Manager boundary | invoke the Manager artifact or report it unavailable |

`codex version` is not introduced. Wrapper/Core/Manager version rows must not
be appended to upstream `--version` or `-V` output.

## Detailed feature and release acceptance

The linked normative annexes define the observable contract for command and
terminal interactions, Bridge transport and conversation continuity, managed
profiles/notifications, release admission and recovery. A successful source
build does not establish installed operation; synthetic reviewers do not
establish live-model approval quality; a signed artifact alone does not establish
client, host or physical-screen effects.

A required user flow is complete only with evidence at every actual owner
boundary it crosses. Preserve original tool results and uncertain operations
instead of guessing from timeouts, resetting identities or duplicating effects.
Use existing qualified Native mechanisms where possible; release-specific
adaptations remain minimal, source-bound and individually qualified.
