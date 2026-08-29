# Rust Core Workboard

This file owns only the current implementation target. Completed detail belongs
in Git history and the `GOAL.md` acceptance ledger, not here.

## Current routing

- Repository: `humtr/codex`
- Active branch: `rewrite/rust-core`
- Normative owner: `SPEC.md`
- Acceptance owner: `GOAL.md`
- Current milestone state: Milestone 2 active; M2-B4 is accepted at
  `8483d2b2db488af032d6f9829639f971e2b5ff3f`; M2-B5 is the selected
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

- Add one remote acquisition adapter to the accepted B4 updater. Do not create a
  remote updater and a local updater with separate admission or activation logic.
- Keep Core dependency-free unless the current slice proves the pinned Termux
  transport cannot satisfy the contract. Do not install packages during product
  development or update.
- Prefer signed exact file acquisition over adding an archive format before the
  official upstream-adaptation bundle requires one.
- Do not add discovery retries, mirrors, redirects, fallback URLs, locks, leases,
  fencing, or background checking without a demonstrated product requirement.
- Tests are evidence, not architecture. A fake transport may prove the real
  public Core path, but no proof-only transport wrapper may close B5 by itself.

## Mandatory bundle execution method

- Bind branch, HEAD, dirty state, source identity, and authorities at every
  resume. A dirty resume records all red gates here before product mutation.
- Each slice closes vertically with its production behavior, named regression,
  nonzero focused invocation, relevant warning-free build/test result, and Lead
  diff inspection.
- A failed compile, zero-test or rejected test command, stale assertion, warning,
  unmapped production branch, or mismatched revision freezes new behavior until
  the whole affected class is dispositioned KEEP/COLLAPSE/DELETE.
- Cheap compile and focused gates run at slice boundaries. The full serial suite,
  three complete default-parallel runs, explicit live read-only smoke, protected
  identity check, and locked release build run only after all behavior slices are
  green.
- On acceptance, reduce this proof map into `GOAL.md`, replace this Workboard item,
  and commit. Do not preserve a parallel roadmap or evidence hierarchy.

## Selected next action

### M2-B5 — immutable HTTPS release acquisition

#### outcome

Extend the real Core update surface with one explicit immutable HTTPS release
base. Core fetches a bounded signed manifest and signature through the pinned
Termux curl, verifies them with the exact B4 pinned-key admission, acquires only
the signed generation inventory into one private temporary source, and hands that
source to the exact B4 stage/probe/activation function. Local update and rollback
remain unchanged; normal launch never performs network I/O.

The first slice changes `SPEC.md` before product code because this adds a public
command and transport/state contract. The intended command is exact
`codex update --remote <HTTPS_BASE_URL>` alongside the accepted exact `--local`
and `--rollback` forms.

#### accepted input

- B4 implementation: `8483d2b2db488af032d6f9829639f971e2b5ff3f`.
- B4 source SHA-256:
  `ea8c840a7f4bff1dcbe3fd3ae36b16b5acb4e0389b8ec8304a069584a1fa49ba`.
- B4 final evidence: focused activation 4/4; public rollback 2/2; full serial
  59/0/1-ignored; three complete default-parallel runs each 59/0/1; explicit
  live smoke 1/1; warning-free locked release; protected live identities exact.
- Core has no Rust dependencies. Current Termux exposes curl 8.21.0 at
  `$PREFIX/bin/curl` with HTTPS and OpenSSL support. It is a separate system
  transport, not the patched upstream runtime or its compatibility resolver.
- No release endpoint or remote asset is currently authoritative. B5 acceptance
  uses a pinned fake curl and fixture bytes inside temporary HOME/PREFIX roots;
  it does not treat a live network response as deterministic proof.

#### current checkpoint

- Bound branch/HEAD: clean
  `rewrite/rust-core@8483d2b2db488af032d6f9829639f971e2b5ff3f`, ahead of its remote by two
  commits. No push or live cutover is authorized.
- Read-only feasibility confirmed the dependency-free workspace and exact Termux
  curl/tar/unzip/OpenSSL executables. B5 selects curl only; tar/unzip and archive
  extraction are outside this bundle.
- Slice 0 is selected. No B5 product mutation has begun. Its first action is to
  encode the exact public command, pinned transport, immutable URL, bounded
  acquisition, cleanup, and B4-reuse rules in `SPEC.md`, then rerun the accepted
  B4 baseline before moving to transport code.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — contract and baseline | Define exact `--remote` semantics in `SPEC.md`; bind URL, curl, bounds, temporary source, cleanup, and unchanged B4 ownership; establish runnable B4 baseline | SPEC diff reviewed before product mutation; warning-free locked build; nonzero B4 public update/rollback baseline | selected |
| 1 — URL and pinned transport | Validate one bounded HTTPS base, deterministically encode signed relative paths, and run only `$PREFIX/bin/curl` with config/environment disabled, HTTPS-only/no-redirect policy, explicit CA/time/size bounds, and caller-owned output files | named URL/argv/failure tests with fake curl pass; no release admission, generation, or activation is added in this slice | blocked by slice 0 |
| 2 — signed control acquisition | Create one private acquisition root, fetch only `release.manifest` and `release.sig`, and reuse the B4 pinned-key parser/signature/policy path before trusting inventory | named missing/wrong transport, oversize, bad signature/policy, malformed URL, and cleanup tests pass with no generation/state mutation | blocked by slice 1 |
| 3 — exact signed file acquisition | Derive every file URL only from the admitted signed inventory, create safe fixed-layout parents, enforce per-transfer and aggregate bounds, verify the assembled bundle through B4, and remove partial acquisition state on every ordinary failure | named complete/missing/digest/extra-path/size/transport-interruption tests pass; output is one valid B4 source or absent | blocked by slice 2 |
| 4 — one public updater path | Route exact `update --remote` through acquisition and the existing B4 activation function; prove initial activation, forward update, anti-rollback, probe failure, cleanup, and unchanged local/rollback behavior via the real subprocess path | named public fake-curl cases and all affected B4 tests pass nonzero; no second admission/staging/activation implementation survives diff review | blocked by slice 3 |
| 5 — grouped acceptance | Add no new product behavior; run final bundle proof and synchronize authority | full serial and three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, protected identities unchanged, GOAL update, commit | blocked by slice 4 |

#### contract to encode before implementation

- The remote base is a bounded ASCII `https://` URL ending in `/`, with no
  credentials, query, fragment, control/space, backslash, or redirect. After the
  manifest is admitted, its generation identity must match the base's final path
  component.
- Manifest and signature URLs are exactly `<base>release.manifest` and
  `<base>release.sig`. Generation-file URLs are formed only by percent-encoding
  the already validated signed inventory path and appending it to the same base.
- Transport is exactly `$PREFIX/bin/curl`; Core disables curl config, clears the
  child environment, permits HTTPS only, supplies the Termux CA paths, applies
  explicit connect/total timeouts and byte bounds, and never follows redirects.
- Core checks the pinned key/OpenSSL/curl contract before acquisition. It admits
  the signed manifest before fetching generation content and verifies the fully
  assembled local bundle again through B4 before staging.
- Acquisition uses one create-new private directory beneath the Core generation
  root, one create-new output file per resource, and cleanup on every normal
  return. A killed process may leave a non-generation dot-prefixed temporary; it
  is never scanned, launched, or activated and does not justify a registry or
  fallback mechanism.
- Manifest/signature retain B4's 128 KiB / 1 KiB bounds. Generation acquisition
  must have one explicit per-transfer/aggregate cap enforced both in curl and on
  observed regular-file sizes; exact constants are fixed and tested in slice 1
  before content acquisition begins.

#### protected surfaces

- `$PREFIX/etc/resolv.conf`, installed `$PREFIX/bin/codex`, live generations and
  activation state, Manager state, auth/profile/session data, package state, and
  publication branches remain read-only.
- All transport and filesystem mutation tests use temporary roots. The explicit
  live smoke remains resolver/installed-launcher read-only and makes no network
  request.

#### stop lines

- no live endpoint selection, DNS/HTTP integration acceptance, release upload,
  branch publication, install, or cutover;
- no archive download/extraction, official upstream adaptation, or package build;
- no release discovery/latest index, automatic check, mirror, redirect, retry,
  proxy/config inheritance, or fallback URL;
- no key rotation/revocation/transparency mechanism in B5;
- no Rust networking/TLS dependency or package installation unless the pinned
  curl path is concretely falsified and `SPEC.md` is revised first;
- no worker, planner, or reviewer unless the user explicitly turns worker mode
  on.

## Next action after M2-B5

After the exact remote signed-generation path is accepted, select the smallest
remaining Milestone 2 bundle from fresh authority. Expected remaining work still
includes official upstream artifact acquisition/safe adaptation, prebuilt Core
release/bootstrap, key rotation policy, isolated fresh/legacy qualification, and
the final independent product review; B5 does not pre-order or claim those gates.
