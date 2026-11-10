# Continuous upstream upgrades

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.


This product is a Termux wrapper for upstream Codex. Continuously following new
official stable releases is a permanent requirement, including every release
after the current upgrade target. Upgrade adoption is settled; engineering work
determines how to absorb upstream changes with minimal compatibility friction
and repeatable automation. Preserve upstream features and behavior, retain only
necessary Termux adaptations and remove local replacements made obsolete by
upstream. Adapt this wrapper to evolving upstream interfaces.

Qualification determines when an update can be activated. A compatibility
failure is work to repair the adaptation and existing release pipeline, not a
reason to freeze development on an older version. Preserve the operating and
rollback generations during that repair. Extend the existing official-release
discovery, source adaptation, qualification, signing, promotion and update path;
do not add another version selector or release controller. The current target
recorded in CURRENT.md does not limit this ongoing commitment to that version.

The common release builder owns source adaptation for both hosted and local
executors. Locate supported operations through Rust syntax and their call/data
relationships, preserving unrelated source and upstream additions; private
function names, local binding names, formatting and file relocation alone are
not compatibility failures. Prefer upstream configuration/APIs and remove
superseded adaptations. Generated protocol outputs come from the adapted type
definitions through upstream's generator, not a previous version's binary diff.
Ambiguous or changed contracts remain explicit failures rather than guessed edits.
The upstream generator owns `app-server-protocol/schema`; frontend scope admits
those outputs and captures the compiled-source proof after generation. Format
and lint compare that same prepared tree, and the existing generator archive
retains the complete generated payload.
Native and optional TUI integration patches each bind their original official
base in the patch itself. Executors acquire that exact base; the common Rust
producer uses Git's three-way content/rename merge with a temporary index.
Conflicts leave the checkout and real index unchanged. Frontend assembly owns
payload collision checks and copying, not an independent patch engine. Do not
backdate source files to force compiler-cache reuse; preserve Cargo's normal
freshness checks and reuse qualified artifacts at their existing admission boundary.


An upgrade candidate binds its official source, wrapper source and component
build inputs once. Build results and qualification references are outputs;
registering them must not change source inputs and repeat their qualification.
Reuse independently admitted artifacts with identical production inputs, and
rerun changed verification without rebuilding unchanged executables. Keep source
admission, signing, immutable public readback and non-forced promotion as separate
authority boundaries. The existing controller implements this flow; do not put
a second release manager above it.

The controller registers an accepted composition in main's `release-frontend-v1`
with its exact `release-source-v1` source. The pinned main caller passes that result
to the existing producer; the producer checks the qualified run, source recipe,
artifact and proof. This result never rewrites development build inputs or starts
their qualification again. Base can advance independently while a new frontend
is still qualifying; Full remains on its previous accepted composition until its
new frontend result is admitted. Historical embedded frontend references remain
readable for retained sources; new results use the composition record.

Frontend proof admission checks the nonzero successful summary against unique
actual PASS records and requires every focus scope declared by the qualified
workflow. Added upstream tests do not invalidate that proof; missing scopes or
incomplete execution do. The schema1 focused_tests field remains readable for
retained input/parser consumers, not as the current artifact's exact test count.

The hosted executor can restore a completed raw build only by its exact existing
production-input fingerprint. The common verifier checks the complete raw
inventory, binary digest, official source/version and dependency audit before
transfer. The candidate builder retains its static AArch64 ELF check when it
consumes the runtime. A failed cache check fails the job; it does not silently rebuild or
admit the artifact. Cache reuse skips compilation, not source admission, Android
qualification, signing or public readback. Public signed Base retention remains
the durable runtime reuse boundary. Each executor owns its compiler profile;
the shared schema writer inherits that profile and never substitutes another
component's code-generation settings.

Cross-workflow delivery uses the raw runtime artifact from a successful owned
Core qualification of the exact selected source. Branch-scoped caches are only
an optimization, not the handoff to main publication. Discover the qualified
run through the shared exact-run admission rules, download its unexpired unique
runtime artifact and use the common raw verifier before transfer. A missing or
expired artifact requires normal production; a failed advertised download or
verification fails the job. Artifact reuse skips native compilation and musl
dependency preparation; Android, signing and public admission remain required.

Local and hosted builds derive the same official source tuple. Explicit executor
toolchain profiles retain their own qualification; they must not independently
pin different upstream versions. Core-only admission and activation never acquire
Manager, the enhanced TUI or Provider. Configured optional compositions have their
own integration gates; live connector refresh applies to an installed Bridge,
not to a Core-only installation with no Provider.
