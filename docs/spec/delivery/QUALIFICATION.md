# Component production and qualification

> Normative annex of [SPEC](../../../SPEC.md). This file owns component input,
> reuse and proof binding. Publication authorization remains in [SIGNING](SIGNING.md).

## Independent inputs and producers

| Component | Production identity | Consumers and additional proof |
| --- | --- | --- |
| Core | Locked Core dependency closure, shared source, embedded runtime recipe and full embedded lock, Core compiler and Android ABI/NDK/flags | Source acceptance, Core/runtime and Full composition |
| Runtime | Official source/archive, normalized compiler input, selected compiler, lock audit and adaptation input | Core/runtime and Full composition |
| Manager | Its locked dependency closure and shared interface source, Core compiler and Android ABI/NDK/flags | Independent Manager qualification and selected Full |
| TUI | Official source, maintained frontend recipe, dependency audit, compiler/profile | Native TUI tests, upstream Retain and Full composition |
| Provider | Its source/locked dependencies, selected runtime binding and app bridge | Independent Provider qualification; not a Core prerequisite |
| Independent MCP worker | Its source/locked dependencies, shared runtime binding, Android ABI/toolchain and exact executable digest | Separate unsigned candidate and isolated worker execution; [MCP contract](../interfaces/MCP_SHELL.md) |

The canonical Android component producer serves Core and Manager. It serializes
only identical component inputs, retains successful production bytes under their
exact input digest, and revalidates every cached binary, receipt, ELF target and
byte digest. It has no fallback cache prefix. Source acceptance and Core/runtime
qualification consume that producer's Core output instead of separate cross-builds.
Unchanged Manager source does not depend on Core source or TUI compilation. The
current Core build embeds the complete workspace Cargo.lock, so its identity must
include those actual bytes even when the changed lock entry is not a linked
Core dependency. A narrower identity requires changing that producer first.

Formal component consumers admit the completed exact-source Core or Manager
qualification independently of branch-local build caches. They bind the immutable
artifact ID, current GitHub archive digest and byte count, then revalidate the
actual transfer, input identity, modes and ELF target. Original production source
and run remain unchanged. Missing or invalid qualified bytes refuse; formal
consumption never falls back to another compilation. Developer production still
uses the single canonical producer and its exact-input cache.
A Full input plan retains each original component artifact ID and successful
run. Acquisition and later reassembly consume those same IDs, verify current
source/run/role and actual archive bytes, and recheck run qualification. The
retained Full proof binds that plan to its component runs; a newer run or an
artifact name alone cannot replace a selected input.

Raw Runtime reuse selects the original immutable GitHub artifact only after both
source trees have equal complete normalized production inputs. It verifies that
source's actual completed compiler/upload job, archive ID/digest and current
Rust raw-artifact checks; a source/run label or old cache key alone cannot qualify
bytes. Original producer provenance is retained. Raw production success does not
inherit the parent run's Core qualification or grant current Full acceptance.
The official development Core source run alone can register a newly reusable raw
origin. Other approved reusable-workflow parents and explicit branch proofs keep
their existing verified candidate provenance; this new label is neither required
for their production nor permission to claim official Core source acceptance.

Core and Manager input identities conservatively include complete selected
package trees and their Cargo.lock graph closure. They are exact identities of
that declared inventory, not the minimal activated compilation graph. A dedicated
crate-test edit or inactive dependency can therefore miss production reuse and
allocate a new component revision even when output bytes coincide. Unrelated
packages and non-embedded repository documentation do not thereby become inputs
to every component. Narrowing an inventory requires proof of the actual Cargo,
build-script and embedded-file consumers; a filename-based exclusion is not proof.
Qualification still records and executes its independent current-source tests.

Production receipts record original producer source/run, component input digest,
artifact digest/size, target, compiler and interface. They do not assert that
consumer tests or Android behavior passed. Qualification runs execute their own
affected checks against the selected bytes and bind their exact source SHA.
Unrelated documentation and output-only frontend registration do not change
production inputs. Required changed source, compiler, ABI, flags, dependencies
or adaptation invalidates affected reuse. Missing or unsafe inventory refuses.

Manager qualification runs independently of Core production and TUI. It requires
the Android Manager interface probe and Manager behavior tests. Linux CI runs
ownership-sensitive Manager tests under the real non-root runner UID in a private
proc view with a short private temporary root; this does not assert namespace
or bwrap support on Android/Termux. A successful
Manager proof never substitutes for the selected Core/Manager/TUI combination.
An optional-component failure blocks that Full combination, not eligible Core.
Actions transfer preserves modes in a bounded regular-file-only archive; a loose
cache hit or executable filename is never an admitted artifact.

Core/runtime qualification independently executes the Native completion,
original-wait, hook-admission and durable-history proofs against the same basic
upstream frontend preparation used by the Runtime producer. It binds the exact
wrapper/upstream source, complete dependency audit, production input identities
and retained logs. Reusing a signed or raw Runtime and a no-op candidate decision
do not waive this proof. The final qualification requires both the applicable
Android execution smoke and successful Native Runtime proof. Enhanced TUI keeps
its own changed-consumer checks; Core does not depend on that optional frontend's
qualification. Test compilation and compiler caches are not second release
artifact producers. A retained partial compile cache carries no qualification.

The development branch also runs complete unsigned Enhanced TUI qualification
on push. Missing dispatch inputs mean the full scope; explicit manual focused
scopes remain separate evidence. A push cannot signal release advancement.
Only a successful full manual run with explicit release intent on the qualified
development ref may use the existing publication controller relay. All component
and Full consumers still verify exact source, workflow and completed success.

## Proof and publication boundaries

Source tests, component builds, native Android probes, live model/UI acceptance,
Full composition, signing, publication and device activation are separate proofs.
Each downstream gate must verify the source, component input identities, selected
bytes and successful owned run. It cannot infer success from a pending run,
request acknowledgement, stale logs or a component revision number.

## Component revisions and immutable issues

An upstream version, exact source commit, component revision `rN`, byte digest,
interface contract, qualification run, composition revision, signed release
sequence and installed generation identify different facts. A revision never
substitutes for qualification or implies byte equality by itself. New signed
`component-input-v1` sidecars may carry `components` and `composition_revision`
together. Each selected Core, Runtime, Manager and TUI entry binds its own positive
revision, exact production input digest, selected artifact digest and interface.
The metadata remains `codex-component-input-v1`, bounded to 16 KiB and bound to the
signed manifest. Existing sidecars remain valid without these optional fields;
readers report their revision as unknown instead of deriving it from the issue
sequence. Runtime's artifact digest is the runtime executable; the full signed
inventory separately binds its companion, helpers and descriptor.

Runtime `rN` uses the normalized compiler inputs plus actual Runtime adaptation
sources, toolchain and locked dependencies. A change only to transport validation
remains in the current qualification provenance and does not by itself allocate a
new Runtime revision. The historical top-level `runtime_inputs_sha256` field keeps
its conservative full-recipe meaning for existing signed metadata compatibility.
Core actually embeds that complete local recipe, so changing those packaged bytes
can change the Core production input, binary and revision while reusing the raw
Runtime. Neither case is a retroactive rewrite of a signed component or sequence.

Within an upstream version, an unchanged component tuple reuses its revision;
a changed input, output or interface allocates its next revision. Core and Full
share these module revisions. The exact Core or Full selection has a separate
composition revision. Previously reserved unpublished inputs also consume their
revisions. Conflicting historical use of a revision fails closed. New upstream
versions have separate module revision namespaces. Existing signed generations
and legacy names are never retroactively renamed, rebuilt or assigned revisions.

## Unsigned Full composition

Full source acceptance consumes the completed owned Core/runtime, independent
Manager and full TUI qualification runs for the exact development source SHA.
It admits each run's unexpired named artifacts, Core/Manager production receipts,
Manager behavior/probe evidence and the existing TUI executed-scope/Retain proof.
It reuses those bytes through the existing composition/adaptation path. A pending,
failed, focused-only, foreign-source or ambiguous run cannot supply a component.
The complete selected inventory, descriptor interfaces and helper/companion hashes
are checked; a TUI artifact's upstream version alone is insufficient.

The result uses an unpublishable `full-qualification-*` generation identity.
The job executes the actual selected Android Core/Manager, runtime and adapted TUI
probes and binds their exits, bounded stdout/stderr and before/after payload digest.
Runtime and TUI probes receive a fresh private home beneath an explicitly
selected owned, protected parent outside their canonical temporary root. They
receive only that home and an owned canonical short temporary root, without user
credentials or configuration. Aliases cannot bypass the upstream refusal to create
helpers beneath the temporary root. Cleanup removes only probe-created directories,
preserving the selected parent and a borrowed private Termux socket root.
Unexpected diagnostics, a failed process or changed byte leaves the candidate
unqualified. Its final proof binds exact source, component runs, component input
and artifact digests, normalized descriptor, full inventory and composition digest.
This is unsigned composition evidence, not device Activity or live Provider proof.
A future authorized Full reservation must admit this successful exact-source proof
and match every selected payload and descriptor contract; only the new generation
identity may differ. Independent Core reservation does not require optional Full.

The development push registers the read-only Full workflow. If component runs are
not yet successful, that run refuses with their missing acceptance boundary; rerun
that same source run after they complete. No Full qualification step creates refs,
reserves issues, signs, stages public artifacts, changes indices or activates a
user device. Publication still admits its separately authenticated Core base and
retains the existing complete signed-composition and public-consumer gates.
