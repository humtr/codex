# Component production and qualification

> Normative annex of [SPEC](../../../SPEC.md). This file owns component input,
> reuse and proof binding. Publication authorization remains in [SIGNING](SIGNING.md).

## Independent inputs and producers

| Component | Production identity | Consumers and additional proof |
| --- | --- | --- |
| Core | Locked Core dependency closure, shared source, embedded runtime recipe and full embedded lock, Core compiler and Android ABI/NDK/flags | Source acceptance, Core/runtime and Full composition |
| Runtime | Official source/archive, runtime recipe, selected compiler, lock audit and adaptation input | Core/runtime and Full composition |
| Manager | Its locked dependency closure and shared interface source, Core compiler and Android ABI/NDK/flags | Independent Manager qualification and selected Full |
| TUI | Official source, maintained frontend recipe, dependency audit, compiler/profile | Native TUI tests, upstream Retain and Full composition |
| Provider | Its source/locked dependencies, selected runtime binding and app bridge | Independent Provider qualification; not a Core prerequisite |

The canonical Android component producer serves Core and Manager. It serializes
only identical component inputs, retains successful production bytes under their
exact input digest, and revalidates every cached binary, receipt, ELF target and
byte digest. It has no fallback cache prefix. Source acceptance and Core/runtime
qualification consume that producer's Core output instead of separate cross-builds.
Unchanged Manager source does not depend on Core source or TUI compilation. The
current Core build embeds the complete workspace Cargo.lock, so its identity must
include those actual bytes even when the changed lock entry is not a linked
Core dependency. A narrower identity requires changing that producer first.

Production receipts record original producer source/run, component input digest,
artifact digest/size, target, compiler and interface. They do not assert that
consumer tests or Android behavior passed. Qualification runs execute their own
affected checks against the selected bytes and bind their exact source SHA.
Unrelated documentation and output-only frontend registration do not change
production inputs. Required changed source, compiler, ABI, flags, dependencies
or adaptation invalidates affected reuse. Missing or unsafe inventory refuses.

Manager qualification runs independently of Core production and TUI. It requires
the Android Manager interface probe and Manager behavior tests. A successful
Manager proof never substitutes for the selected Core/Manager/TUI combination.
An optional-component failure blocks that Full combination, not eligible Core.
Actions transfer preserves modes in a bounded regular-file-only archive; a loose
cache hit or executable filename is never an admitted artifact.

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

Within an upstream version, an unchanged component tuple reuses its revision;
a changed input, output or interface allocates its next revision. Core and Full
share these module revisions. The exact Core or Full selection has a separate
composition revision. Previously reserved unpublished inputs also consume their
revisions. Conflicting historical use of a revision fails closed. New upstream
versions have separate module revision namespaces. Existing signed generations
and legacy names are never retroactively renamed, rebuilt or assigned revisions.
