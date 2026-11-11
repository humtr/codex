# Base qualification

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


### Independent unsigned base qualification

The reusable producer also has an explicit `base_only` qualification mode. It
constructs only the qualified upstream runtime, Core, Code Mode host and browser
helpers through the existing release-builder. Manager and TUI are absent;
their compilation, downloads and qualification do not gate this mode. Complete
base inventory, digest/mode checks and actual Android/AArch64 Core/runtime smoke
remain required. A base result is not an integrated product qualification.

This mode is read-only with respect to publication. It refuses `publish=true`
before artifact acquisition; signing, Release/Pages staging, stable promotion and
device activation are never reachable. Its unsigned CI artifacts have distinct
base-qualification identities and cannot replace an integrated candidate. It
adds no update channel, installed pointer, runtime mode or public CLI command.
The main publication path still requires the complete qualified backend/frontend
pair and retains the current generation on an unsupported update.

Base qualification may run from an exact rewrite/rust-core push or an explicit
manual `workflow_dispatch` on a development branch with read-only repository
permissions. This deliberate manual source-validation boundary uses the actual
`refs/heads/` event ref and requires source to equal the event's immutable commit;
it accepts no substituted source/ref input. Tags, pull requests, other events,
pushes outside rewrite/rust-core and `publish=true` are rejected. Every admitted
branch runs the same compilation, inventory and Android execution qualification
and produces the existing unsigned base-qualification artifacts. This validates
that source; it is not public release admission. The Core completion relay runs
only for an explicitly requested release-intent dispatch on rewrite/rust-core. Public exact-run admission, main's accepted source
and signing/promotion still require their existing rewrite proofs, so a successful
manual development-branch run neither replaces them nor triggers publication.
`rebuild_current` selects the version and source archive digest from the
authenticated stable descriptor independently of TUI pins; otherwise the
ordinary official upstream discovery is used. An unsupported upstream artifact
still fails existing builder qualification, rather than weakening patch admission.
Skipped integrated construction distinguishes a non-newer release from missing
frontend qualification or an archive mismatch; it must not report all three as
already up to date. Independent base qualification and feature-preserving public
promotion are different guarantees; section4.6 defines their final boundary.
### Automated upstream preparation and base/full publication

Upstream handling must resolve one exact official version/archive/source tuple,
prepare its Termux compatibility inputs, and drive separate base and full release
lanes automatically. A scheduled discovery controller resolves a newly published
stable upstream version, exact source and official archive digest, updates the
single input authority without force, and explicitly starts its read-only proofs.
Qualified base publication must not wait for Manager/enhanced-TUI porting or proof.
Unsupported transformations stop with actionable evidence and keep the last known
good head; source edits are never invented by a publishing script. Failed optional
frontend work cannot block source/Core qualification or basic publication. Device
activation follows its existing user/update policy, not a forced running-session
replacement. Base contains Core plus the adapted upstream default UI;
full adds admitted Manager and enhanced TUI. The opt-in Chat provider remains
independent. Both lanes require actual Android execution, affected behavior proof,
signing, immutable assets, complete public readback, disposable update/rollback
proof and non-forced promotion. Source preparation or a green host build alone
cannot authorize publication. An unsupported or ambiguous transformation stops
that candidate with actionable evidence and preserves the accepted installation.

Version/source/toolchain and qualified frontend references have one machine-
readable authority. Preparation must not guess binary offsets or accept an
unqualified version by removing its guard. Prefer repeatable source/semantic
transformations; any generated artifact policy remains bound to exact inputs and
Android proof. Historic policies needed by retained generations remain supported.

Qualification build/test jobs remain read-only. An isolated completion relay may
only enqueue the accepted-input workflow with its run ID after an explicit
`advance_release=true` dispatch; it has no content-write
permission or signing key. The receiver waits for that run to finish, then checks
owned exact-source qualification independently. Completion delivery is not proof.
Native trigger metadata and the exact isolated relay are excluded from recipe
comparison only while build/test code consumes no event/dispatch input. The literal
wall-clock budget of the native build job is scheduling metadata, not a compiler
input. Its change alone may reuse a completed successful run; it cannot admit a
failed, canceled or incomplete run. Other global build settings and every build/
test command remain bound. The required focused-test count is an acceptance
criterion: reuse checks the retained log against the current required count,
without treating an earlier misdeclared count as a source change. Source blobs,
upstream/toolchains, dependency audit, executed tests, lint and artifact identity
must still agree. Native builds start through
the accepted-input controller, reusing a matching build already in progress.

Provider-only source changes under providers/chatgpt use the independent optional
provider gate and do not rebuild unchanged Core/Manager or make Bun a Core
prerequisite. Qualify the Rust package and minimum app-side bridge together.
After qualifying the active Provider Rust package and retiring old experimental
paths, legacy differential source/tests remain in preserved experiment history;
the active gate covers Rust, the app bridge, CDP qualification and package tools. Missing Rust/device/bridge gates are unfinished, not a reason to fall back
to legacy admission. The provider gate must not require Bun/Python merely to
execute the production adapter. Scope Cargo lock/toolchain/
shared-build changes by their actual consumers; do not exclude changed shared
inputs from Core qualification. Classification still requires a successful accepted predecessor;
otherwise the conservative full Core gate runs. Changes to Core/shared build inputs
or mixed unknown paths retain full qualification. Provider qualification is separate
from Core acceptance and must be required before future signed provider admission.

Base and full have independent signed public heads. The existing update-index-v1
remains the full head for compatibility; the base head is update-index-base-v1.
The default update head follows the authenticated public baseline of the selected
generation, including a local-derived generation; an explicit update URL remains
an operator override. Full users must never be silently downgraded when only base qualifies. Both public
heads and retained last-known-good assets stay available in their immutable
publication origins; current releases do not replace the legacy Pages site.
Promotion coordinates both heads with non-forced compare-and-swap and increasing
release sequence; a stale concurrent run re-evaluates rather than overwriting
another lane. No device activation or user-profile/provider mutation is implied
by public publication. Qualification-only workflows remain non-publishing.

codex-runtime-binding is dependency-free shared **source**, statically compiled
into its callers, for the existing bounded execution-record codec. Core depends
on release-builder and this primitive, not on Manager, frontend or provider.
It creates no installed shared library, service, registry or optional-component
runtime prerequisite. Core configuration queries remain bounded and optional;
failure skips conveniences while preserving independent execution.

Independence has distinct observable levels:

- **Build and qualification:** Core/base construction and Android execution do not
  compile, acquire or qualify Manager, enhanced TUI or provider. Unsupported native
  runtime revisions still require their own Termux qualification.
- **Installed execution and lifecycle:** a valid Core-only generation supports
  ordinary upstream execution and Core doctor/update/rollback. Unavailable optional
  assets in an already-installed generation do not disable those operations.
  Doctor may separately report missing authentication or conveniences. Fresh
  candidate admission still rejects every missing/changed declared asset.
- **Component reuse:** the builder accepts an independently admitted Manager;
  local reconstruction already carries forward its bytes after the Core-API
  probe. A compatible unchanged Manager need not be rebuilt to change Core.
  The integrated CI build may still compile both; that is not a Core dependency.
- **Feature-preserving public updates:** stable remains one signed combination.
  Preserving /switch and the enhanced /model/permission UI requires its frontend
  to pass the qualified backend protocol/behavior gate. Exact backend-version
  admission remains until a broader pair is actually proven. Failure retains the
  accepted generation; base qualification alone never silently removes features.
  There is no automatic downgrade from full to base; each edition follows its
  own signed public head.

The retained TUI patch contains only upstream call sites/types, supported-permission
integration, startup compatibility and actual test access; standalone policy lives
under frontend/tui/src. Upstream lacks a qualified general extension API
for these UI hooks. Moving them into Core or a separate launcher would not remove
revision maintenance. The official-runtime binary adaptation remains exact-digest
and version-bound for FD/socket/config paths and no-sandbox permission choices;
the source-built frontend's FD remap is separately qualified. Never remove either
gate merely to admit a newer upstream version.

Responsibility realignment is complete when these ownership and independent
execution/lifecycle guarantees pass. It does not claim that future upstream
compatibility, public integration/promotion, same-provider in-place selection or
provider feature acceptance is already complete. Those changes belong to their
respective components and require affected-path proof.
