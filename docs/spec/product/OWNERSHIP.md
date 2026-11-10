# 4. Architecture and ownership

> Normative module of [SPEC.md](../../../SPEC.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

## 4. Architecture and ownership

### 4.1 Core

Core exclusively owns:

- public command dispatch;
- official upstream artifact qualification;
- Termux runtime adaptation and launch environment;
- FD 33 and FD 34 preparation;
- final upstream process execution;
- sandbox capability enforcement;
- immutable generation construction and integrity metadata;
- update, activation, last-known-good selection, and rollback;
- read-only Core diagnosis;
- composition of upstream and Termux doctor results;
- the typed boundary through which Manager requests Core operations.

Core must remain usable when Node.js, Manager, network access, update services,
or optional Termux APIs are unavailable.

### 4.2 Manager

Manager owns:

- `codex termux` command UX;
- profile selection and presentation;
- notification configuration and delivery;
- Manager-local state and UI;
- profile execution handoff to the validated Core entrypoint.

Manager's profile UX and declared profile state are standalone Codex product
capabilities, assessed through the public `codex termux` path. Their completeness
is determined by how they support the official upstream runtime and user tasks,
not by features in a separate convenience product.
Conversation persistence, discovery, selection and resume belong to upstream,
as established by the shared-conversation contract below; Manager does not own
a second session index or picker.

The Manager executable is built separately from Core and is an optional signed
generation asset. Core does not compile it, discover it from `PATH`, or probe
it during ordinary launch. Release-builder owns the bounded artifact probe;
Core consumes only the signed generation and its digest-bound Manager path.

Manager must not directly write Core generations, pointers, manifests, locks,
runtime state, resolver data, or activation journals. A Manager request that
would mutate Core state must use a versioned, runtime-validated Core contract.
The Manager may create and update only its declared state and an explicitly
created profile-home directory, plus the no-replace shared configuration-profile
links in a validated selected upstream home described below. It must not copy,
parse, summarize, or rewrite
authentication files, cookies, OAuth material, arbitrary upstream state, or
session transcript content. Profile execution changes `CODEX_HOME` only in the
child environment and returns through the stable Core entrypoint; it never
changes the caller's environment or persistent Core state. Manager commands
that need an update, rollback, doctor, or other Core operation must request the
corresponding Core-owned command through that entrypoint and must not
reimplement the operation.

The Manager artifact is optional and independently qualified. Its absence,
incompatibility, or failure must not make ordinary upstream launch, Core
doctor, Core update, or Core rollback unavailable. Manager v1 has no external network,
package-manager, OpenSSL, bwrap, resolver, or generation-discovery authority.
Profile registration creates only Manager metadata and a private new account
home. Selection affects only the child invocation; execution readiness and
shared conversation-path preparation belong to Core before upstream launch.
Optional shared configuration-profile projection belongs to Manager preparation.

An installed generation's unavailable Manager is reported at the Manager
boundary and by doctor; it does not invalidate the independently usable Core
runtime. Ordinary dispatch checks the declared Manager path for a regular,
owner-executable file without probing it or requiring OpenSSL. Installed-release
verification still authenticates the signed inventory, descriptor and all
Core/runtime assets; a failed optional Manager file check is excluded from that
verification result and local-build carry forward, without disabling Core update
or rollback. This exception applies only to already-installed generations.
Fresh candidate admission, staging and activation
retain complete Manager verification whenever the candidate declares one. No
generation metadata, signed inventory or user state is rewritten to express absence.

Manager is a separate Rust executable. No Manager runtime or dependency may
become a prerequisite for ordinary upstream launch, Core doctor, update, or rollback.

### 4.3 Shared contracts

Core and Manager may share only explicit versioned data contracts. Compile-time
types are insufficient; every external or cross-layer payload is validated at
runtime. Unknown incompatible schema versions fail without mutation.

### 4.4 Product completeness against upstream

The completeness boundary is the public `codex` and `codex termux` product.
Core supplies the supported official runtime, Termux adaptation, process and
shared-server execution, generation lifecycle and diagnosis. It must preserve
accepted upstream command/configuration behavior and account isolation; optional
convenience failures must not become prerequisites for ordinary execution.
Candidate publication and activation still require the complete signed inventory
and qualification; optionality does not permit admitting an invalid candidate.

Manager supplies standalone user-facing configuration and selection conveniences
above that boundary. A useful convenience must produce a distinct user result,
not another implementation of a Core command. Profile selection must clearly
distinguish the identity supplied to the upstream child from saved selection
history; history alone is not evidence of the identity used by a later ordinary
launch. Authentication, conversation persistence and thread lifecycle remain
upstream-owned. Session conveniences, if selected in a later contract, must use
upstream-supported discovery/resume rather than a second transcript index.

Notification policy, upstream hook rendering and Termux delivery belong to Manager.
Core combines its execution defaults with a bounded configuration fragment from
the admitted Manager; it does not parse Manager notification records or event names.
Delivery failure must
not fail an upstream turn. Public-path proof must establish the intended event
and delivery behavior; a configuration record or generated command alone is not
proof of the user's completed task. Legacy functionality is evaluated against
these user results and ownership boundaries, not as an automatic feature backlog.

### 4.5 Responsibility and compatibility admission criteria

These are target architecture contracts. CURRENT.md owns implemented and proven
scope. An existing file, service, guard or boundary does not acquire authority by
surviving earlier work. Reassess ownership against the complete requested flow,
including failure and recovery, before implementing another local fix.

1. **One owner per fact and mutation.** Native owns conversation/turn identity,
   execution, approvals, writer authority and committed history. A model route
   is a selected inference source, not the identity of the user's task. A remote
   Chat, transport connection, process and installed generation each have their
   own lifetime; changing one does not implicitly replace the others.
2. **Demonstrated interface sufficiency.** Use qualified upstream interfaces
   first. When they cannot deliver the authorized behavior, name the exact
   missing operation/invariant and compare supported composition, a minimal
   Native compatibility extension and replacement. A source guard proves current
   behavior, not that the product must retain it. Compatibility work must keep
   execution and policy decisions at their authoritative boundary; no second
   tool executor, writer registry or transcript authority is introduced.
3. **Owner of selection and persistence.** Entrances submit user intent.
   Native validates and commits each thread's effective inference route and
   model settings, preserving policy, history and other threads. Provider
   supplies discovered/configured routes and capabilities. Frontend, Manager
   and Provider do not independently persist conflicting selections or model
   a route change as their own conversation restart.
4. **Minimal Core.** Core supplies qualified runtime/environment binding and
   generation/update authority. It does not interpret model catalogs, select an
   inference account or acquire an optional host for ordinary Native execution.
   Optional-component interaction occurs only on an explicitly used component
   path and must not make ordinary Core launch depend on that component.
5. **Derived state and real boundaries.** Consumers derive Native execution and
   completion from Native events/receipts and verified live ownership. Provider
   may persist cross-process authorization, exact retry admission, source
   correlation and unfinished external I/O that Native does not own. Do not
   mirror Native phases as another independently advanced task lifecycle.
   Necessary revalidation across process, time, concurrency and authority
   boundaries remains; cosmetic reduction of guard counts is not simplification.
6. **Configuration and lifecycle.** Account/workspace choices, paths and transport
   instances are configuration or verified discovery. Shared implementation has
   no account-name branches or embedded account paths. One configured supervisor
   owns a service lifetime; Provider owns its optional app lifetime. No temporary
   desktop PID is a boot prerequisite for Native/MCP access.
7. **Proof before replacement.** Establish each old path's consumer and required
   behavior. KEEP necessary contracts, MERGE duplicate decisions, MOVE misplaced
   authority, DELETE replaced machinery in the same qualified change. A temporary
   adapter names its actual missing Native capability and removal condition.
   Compare runtime cost and maintenance with the same inputs; do not replace
   measurements with class/file counts.
8. **Whole-flow admission.** Source success, Native execution, host observation,
   live client display, signed delivery and activation are distinct evidence.
   Qualification exercises cold start, ordinary input, settings transitions,
   compaction, reconnect, interrupted/uncertain delivery and recovery at the real
   producer/consumer boundary. An ignored/zero-test gate is incomplete.

Account homes remain upstream execution identities. Termux socket/environment
adaptation is not permission to relocate authentication or own Native storage.
The accepted sharing policy continues to use upstream's canonical formats and
locks. Revised route selection must not weaken administrator policy or imply
authority to use an unavailable account/model. Explicit failures retain the
conversation, draft and observed results; there is no silent model substitution.
