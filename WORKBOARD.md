# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B5 is accepted at
  `c39c338d6238d3e8aba128d8fa522d0e9de66d83`; M2-B6 is the selected
  implementation target
- Worker mode: user-controlled; current state OFF. Only an explicit user command
  may change it
- Additional agents/workers/reviewers: disabled while worker mode is OFF
- Live product cutover/publication: not authorized
- Execution discipline: follow `AGENTS.md` outcome-first closure rules; close one
  vertical proof slice before beginning another independent contract, stop on
  every red/nonzero-proof failure, and reserve grouped acceptance for the stable
  bundle

## Product-speed policy

- Establish one release-producing path from one explicitly immutable official
  upstream artifact to one qualified runtime/assets result. Do not create both a
  release-time builder and an on-device adapter unless Slice 0 proves both are
  product requirements.
- Resolve the real executable ownership boundary before code. An unwired parser,
  extractor, patcher, or test-only entrypoint cannot close B6.
- Keep accepted B1-B5 launch, signed release, update, activation, and rollback
  machinery unchanged. B6 supplies qualified artifact content; it does not add a
  second updater or weaken v2 admission.
- Prefer one exact official artifact format and one exact patch policy over a
  generic package manager, archive framework, format fallback, or heuristic
  search.
- Keep Core dependency-free unless a vertical slice proves the existing Termux
  tools plus bounded Rust code cannot meet the accepted contract. Do not install
  packages during development.

## Mandatory bundle execution method

- Bind branch, HEAD, dirty state, source identity, and authorities at every
  resume. A dirty resume records every red gate here before product mutation.
- Update `SPEC.md` before any change to artifact authority, executable ownership,
  archive semantics, patch policy, persistent release content, or public command.
- Each slice closes vertically with its production behavior, named regression,
  nonzero focused invocation, relevant warning-free build/test result, and Lead
  diff inspection.
- A failed compile, zero-test or rejected test command, stale assertion, warning,
  leaked temporary root, unmapped production branch, or mismatched revision
  freezes new behavior until the whole affected class is dispositioned
  KEEP/COLLAPSE/DELETE.
- Cheap compile and focused gates run at slice boundaries. The full serial suite,
  three complete default-parallel runs, explicit live read-only smoke, protected
  identity check, and locked release build run only after all behavior slices are
  green.
- On acceptance, reduce this proof map into `GOAL.md`, replace this Workboard
  item, and commit. Do not preserve a parallel roadmap or evidence hierarchy.

## Selected next action

### M2-B6 — official upstream artifact acquisition and safe adaptation

#### outcome

Define and implement the smallest real release-producing boundary that accepts
one explicitly pinned official upstream artifact, proves its immutable identity,
extracts only the accepted target content without path escape, applies one exact
drift-detecting Termux adaptation policy, and emits the qualified runtime/helper
evidence required by the existing generation manifest. The resulting content is
an input to the existing signed-release pipeline; B6 does not activate it or
introduce another update path.

Slice 0 owns an unresolved architectural fact: whether this work runs only in the
release-production environment or also on a supported device. It must resolve the
owner, executable entrypoint, exact official publication surface, artifact format,
and immutable identity in `SPEC.md` before product code. No public command or
second binary is assumed merely to make component code reachable.

#### accepted input

- B5 implementation:
  `c39c338d6238d3e8aba128d8fa522d0e9de66d83`.
- B5 source SHA-256:
  `d5c5f69da6ce8d7f52b20ce8d426d3948e3452566a3a4f075dca67fd1e773dca`.
- B5 final evidence: full serial 69/0/1-ignored; three complete
  default-parallel runs each 69/0/1; explicit live read-only smoke 1/1;
  warning-free locked release; zero test-root residue; protected live identities
  exact.
- The existing generation descriptor already binds upstream package identity and
  version, immutable source-artifact digest, platform/architecture, patch-policy
  identity/report, runtime/helper/Core/Manager digests, API/schema compatibility,
  qualification, and creation metadata.
- Current production has no official artifact locator, downloader, archive
  parser/extractor, binary patcher, patch report producer, or real release-builder
  entrypoint. Manifest fields and historical interfaces are not implementation
  proof.
- Legacy may be inspected only for observable failure cases and safety constraints;
  no legacy Bash, Python, C, tests, generated data, or internal model may be copied.

#### current checkpoint

- Bound branch/HEAD: clean
  `rewrite/rust-core@c39c338d6238d3e8aba128d8fa522d0e9de66d83`, ahead of its remote by four
  commits. No push, live acquisition, publication, install, or cutover is
  authorized.
- Slice 0 is selected. No B6 product mutation has begun. Its first work is
  read-only authority discovery: identify the current official upstream artifact
  publication and exact supported Android/Termux input, inspect only the minimum
  accepted legacy behavior needed for failure constraints, decide the release-
  time/device-time owner and real entrypoint, update `SPEC.md`, and establish a
  runnable B5 baseline.
- If no immutable official artifact can be identified, or the required format/
  patch policy cannot be stated exactly, stop before extractor or patcher code.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — authority, owner, and baseline | Resolve the official immutable artifact identity, accepted format/target, release-time versus device-time owner, and real executable entrypoint; encode them in `SPEC.md`; establish a runnable B5 baseline | primary-source/authorized local evidence is exact; SPEC diff reviewed before product mutation; warning-free build and nonzero B5 public baseline | selected |
| 1 — bounded safe extraction | Admit exactly the selected artifact format and extract only the declared target files into a private test-owned root; reject absolute/traversal paths, escaping links, special files, duplicates, and layout drift | named real-entrypoint extraction success/failure matrix passes; no output escapes and no generic/fallback format survives review | blocked by slice 0 |
| 2 — immutable acquisition and source binding | Acquire or accept the exact selected official artifact through the Slice 0 owner, enforce transport/size/digest identity before adaptation, and leave no usable partial on failure | named identity/transport/bound/cleanup matrix passes through the real entrypoint; observed bytes equal the declared immutable digest | blocked by slice 1 |
| 3 — exact Termux adaptation | Verify expected unpatched occurrences, reject already-patched/unexpected layouts, apply one exact patch policy, and emit a deterministic patch report plus runtime/helper digests | named drift/already-patched/count/output-digest regressions pass; no heuristic search, optional patch branch, or fallback policy remains | blocked by slice 2 |
| 4 — qualified generation integration | Bind artifact identity, extraction result, patch report, runtime/helpers, and existing Core compatibility fields into one generation input consumed by the existing release path without activation | named end-to-end fixture and affected B1-B5 groups pass nonzero; direct review proves one manifest authority and no second updater/activator | blocked by slice 3 |
| 5 — grouped acceptance | Add no new product behavior; run final bundle proof and synchronize authority | full serial and three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, zero residue, protected identities unchanged, GOAL update, commit | blocked by slice 4 |

#### contract questions Slice 0 must close

- Which official OpenAI publication, package/version coordinate, asset name,
  platform, and architecture form the sole upstream artifact authority?
- Which immutable value binds acquisition: exact digest plus immutable versioned
  URL/release identity, with no mutable latest/discovery fallback?
- Does acquisition/adaptation execute in release production, on device, or both;
  which existing or new executable surface reaches it without exposing a new
  unsupported public `codex` command?
- What exact archive/container format and byte/file/count bounds are accepted?
- Which target files and source occurrences are expected, what exact bytes are
  transformed, and what deterministic patch-policy identity/report binds the
  result?
- How does the qualified result feed the existing generation/release manifest
  without introducing signing-key material into Core or on-device state?

#### protected surfaces

- `$PREFIX/etc/resolv.conf`, installed `$PREFIX/bin/codex`, live generations and
  activation state, Manager state, auth/profile/session data, package state, and
  publication branches remain read-only.
- Artifact and extraction tests use repository-external temporary roots. Until a
  later gate explicitly authorizes otherwise, official-source discovery is
  read-only and no artifact is installed, activated, uploaded, or published.

#### stop lines

- no mutable `latest` authority, package-manager install, generic dependency
  resolver, mirror, retry ladder, redirect fallback, or multiple archive formats;
- no legacy source copying or translation;
- no signing private key in Core, the repository, fixtures committed to Git, or
  on-device product state;
- no live generation creation, activation, rollback, launcher replacement,
  release upload, branch publication, or device cutover;
- no public command, second binary, networking dependency, or external package
  until Slice 0 assigns the exact owner and `SPEC.md` is revised first;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.

## Next action after M2-B6

After the official artifact path is accepted, select the smallest remaining
Milestone 2 bundle from fresh authority. Remaining gates still include prebuilt
Core release/bootstrap, signed key-rotation policy, isolated fresh-Termux and
upgrade-from-legacy qualification, and the final independent product review;
M2-B6 does not pre-order or claim those gates.
