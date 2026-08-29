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
- Slice 0 closed green on SPEC SHA-256
  `6094a595b78ceb67b8ed2bbf3de605e1936fdb82625e0c6f2db45515eba6f6d2`
  and HEAD-relative SPEC-diff SHA-256
  `29f19f24b73cad1a9d6da3e32d03e09774d99bae40d7c193aae0565f8fd71509`.
  The first exact public-local baseline command omitted the `tests::` module
  prefix and executed zero tests, so it is explicitly excluded. The corrected
  exact invocation passed 1/1, public rollback passed 2/2, the locked workspace
  build was warning-free, and the SPEC/diff review is green. Product source
  remains the accepted B4 SHA-256
  `ea8c840a7f4bff1dcbe3fd3ae36b16b5acb4e0389b8ec8304a069584a1fa49ba`.
- Slice 1 is stop-on-red. URL/transport component tests ran 3/3 and all affected
  B4 tests ran 14/14, but the non-test build exposed 17 dead-path warnings because
  the proposed horizontal transport slice was reachable only from tests. Direct
  product-path review also found that mode-blind release v1 cannot reproduce
  runtime/helper/compat executable metadata over file-addressed transport.
- Root-cause disposition: KEEP the bounded URL value, pinned curl child, and
  their named component regressions; COLLAPSE the former horizontal transport,
  control, and content slices into one real public remote happy-path slice;
  REPLACE mode-blind `codex-release-v1` with one strict v2 inventory binding
  digest plus exact safe regular-file mode; DELETE any v1 compatibility parser,
  blanket remote chmod rule, proof-only dispatcher, or second updater. The SPEC
  was revised before continuing product code.
- Slice 1 repair closed green on source SHA-256
  `f87620391664bce6bb792bf980ec8bab70369b9f1cb429662a8c45d7da4f7d98`
  and HEAD-relative source-diff SHA-256
  `000b7f8fd246a4fcb5dfe6d508747a09736f2fa3360bb16bc4cd537fba36b251`.
  The exact public remote happy path passed 1/1 and proved signed-mode
  reconstruction plus acquisition removal before candidate probes; the URL,
  curl, and private create-new path group passed 4/4; all affected B4 tests
  passed 14/14; formatting/diff checks and the locked non-test build were
  warning-free. Direct diff review retained one updater and one B4 admission/
  staging/activation path, replaced the stale digest-only record name, renamed
  the local-only dispatcher, rejected malformed IPv6 through the standard
  parser, and found no v1 parser, blanket executable chmod, proof-only remote
  dispatcher, redirect/retry/fallback, or second updater. Slice 2 is selected.
- Slice 2 closed green on source SHA-256
  `923834624589f8779845a89f9ac8cfdaa6573d37204913d3d4d1e16d6a8b6f4e`
  and HEAD-relative source-diff SHA-256
  `ac9c6d3aac53d77906f50e99cb8a70e1dd2c9a729ce05ec2489637ac6428fb17`.
  One exact public failure matrix passed 1/1 while exercising 11 isolated cases:
  invalid scheme, missing curl, first-control transport failure, missing
  signature transport, manifest/signature response bounds, malformed manifest,
  wrong pinned key, bad signature, rejected channel, and signed/base identity
  mismatch. Every ordinary failure removed its acquisition directory and left
  generation/state/config absent. A separate real permission-induced cleanup
  failure passed 1/1: cleanup failure replaced the earlier acquisition error,
  activation state remained absent, and only the expected inert `.acquire-*`
  remained before test-owned restoration/removal. The normative cleanup wording
  was corrected before this proof on SPEC SHA-256
  `50c512f897fde8a8daa0073ca1f7449fdabd7dd743363dd8f7c5bb15e2e3685f`.
  Slice 2 added no production branch; formatting/diff checks and the locked
  non-test build were warning-free. Slice 3 is selected.
- Slice 3 closed green on source SHA-256
  `705e65b25f0a10ea85c26d9d9a24330cee7e33b47a81953476c9eacbfb2e3b69`
  and HEAD-relative source-diff SHA-256
  `8b456fd4de36cfc917a9cee8a4d9ba1ca32d53bd01a0646189f4f491204b9912`.
  The expanded public success test passed 1/1 with a signed UTF-8/special-byte
  nested path requested only through canonical percent encoding, exact 0710,
  0640, and 0600 mode reconstruction, and a server-only unsigned file neither
  requested nor installed. The public content failure matrix passed 1/1 across
  missing content, partial/nonzero transport, content digest mismatch, a signed
  non-executable runtime mode, and a signed file/directory output collision;
  every ordinary failure cleaned acquisition and left generation/state absent.
  The actual `fetch_remote_resource` path passed 1/1 for both response cap and
  remaining aggregate cap without a proof-only production parameter or giant
  fixture. The shared fake curl refactor was followed by a grouped public remote
  rerun at 4/4. Slice 3 added no production branch; formatting/diff checks and
  the locked non-test build were warning-free, and direct review found no
  mode-blind or blanket-chmod path. Slice 4 is selected.
- Slice 4 closed green on source SHA-256
  `c8310bacf0839d9dc63fb41821c5b400c10bc9754774f2ccd9d767e4c5bc0f03`
  and HEAD-relative source-diff SHA-256
  `eb16a1696ca9461625d073170488957efb8a8d9e7d49d586bd51cd1c89066879`.
  One exact public integration sequence passed 1/1 across local seq1
  activation, remote seq2 forward activation, equal-sequence remote rejection,
  remote seq3 candidate-probe failure with complete inactive staging, acquisition
  cleanup before every remote probe, and exact signed rollback to the retained
  local generation. The authoritative state and transaction-file assertions
  held at every boundary. Affected foundational groups passed M2-B1 12/12,
  M2-B2 6/6, M2-B3 6/6, and M2-B4 14/14; the locked non-test build remained
  warning-free and formatting/diff checks passed. Direct production review
  classified KEEP for one bounded remote acquisition adapter, pinned curl, v2
  admission, and B1 transaction; COLLAPSE for exact local/remote dispatch into
  `run_core_update` plus shared `prepare_signed_local_release` and
  `activate_prepared_local_release`; DELETE/absent for a second verifier,
  stager, activator, updater, archive, discovery, redirect, retry, mirror, or
  fallback. One malformed `rg` audit invocation was excluded and its corrected
  no-hit command passed. Slice 5 is selected.
- Slice 5 is stop-on-red after its first grouped batch. The serial suite passed
  69/0/1-ignored, three default-parallel suites each passed 69/0/1, the explicit
  live read-only smoke passed 1/1, and the locked release build was warning-free;
  protected identities were unchanged. Post-batch residue inspection nevertheless
  found one test root,
  `/data/data/com.termux/files/usr/tmp/codex-r2-b4-inventory-staged-copy-13685-5`.
  Acceptance excludes that otherwise-green batch. The class audit found that
  retained tests discard every terminal `remove_dir_all` result, allowing a
  cleanup failure to masquerade as green. Freeze acceptance, replace the whole
  test-owned temporary-root cleanup class with one strict NotFound-tolerant
  helper, rerun the affected cleanup test, then rerun grouped acceptance on the
  repaired source. No product behavior is added by this repair.
- Slice 5 repair and grouped acceptance are green on final candidate source
  SHA-256
  `d5c5f69da6ce8d7f52b20ce8d426d3948e3452566a3a4f075dca67fd1e773dca`
  and HEAD-relative source-diff SHA-256
  `747c4cc5fd3086454696179713b2e3120c967947c907ee00add3e75636e63196`.
  The entire test-module cleanup class now uses one strict helper that tolerates
  only `NotFound`; the originally affected B4 cleanup regression passed 1/1 and
  residue inspection returned zero. The replacement grouped batch passed the
  full serial suite at 69 passed / 0 failed / 1 explicit smoke ignored, three
  complete default-parallel suites each at 69/0/1, the explicit real-Termux
  read-only smoke at 1/1, formatting/diff checks, and a warning-free locked
  release build. Post-batch test-owned `codex-r2-*` roots were absent. The live
  resolver remained SHA-256
  `7e8ad76e0d200e93918ca2e93c99ff8ecd02071953bf1479819db3ac0dbb6d07`
  with stable stat `65089:94666:8180:10379:10379:38:1787846643`; the installed
  launcher remained SHA-256
  `0b0284155f2672263836029f760ba06a0cb284b7ca3a8e600ad399b43af36aff`
  with stable stat `65089:1260183:81ed:10379:10379:7512:1787848098`. No live
  network, generation, activation, Manager, resolver, auth/profile/session,
  package, launcher, or publication state was mutated. M2-B5 is an accepted
  candidate pending authority synchronization and commit.

#### vertical proof map

| Slice | Exact outcome | Exit gate | State |
| --- | --- | --- | --- |
| 0 — contract and baseline | Define exact `--remote` semantics in `SPEC.md`; bind URL, curl, bounds, temporary source, cleanup, and unchanged B4 ownership; establish runnable B4 baseline | SPEC diff reviewed before product mutation; warning-free locked build; nonzero B4 public update/rollback baseline | green — 2026-08-29 |
| 1 — real remote happy path | Replace mode-blind release v1 with strict signed digest+mode v2, then route one valid fake-curl `update --remote` from exact public argv through control/content acquisition, B4 staging/probes, pre-activation acquisition cleanup, and initial atomic activation | named v2 mode and real public remote tests pass; all existing B4 tests pass; warning-free non-test build; every new production definition is reachable and diff-inspected | green — 2026-08-29 |
| 2 — control and transport failures | Exhaustively close invalid URL/curl, control size/transport, malformed manifest, wrong key/signature/policy/base identity, and cleanup failures without generation activation | named public failure matrix passes; ordinary acquisition failures clean their partial directory; an explicit cleanup failure is terminal and leaves only an inert dot-prefixed partial; authoritative state never changes | green — 2026-08-29 |
| 3 — content, bound, and metadata failures | Close missing/digest/extra-path, nested path, exact signed mode, per-file/aggregate bound, interruption, and unsafe output cases through the same acquisition path | named public/content matrix passes; result is one verified B4 source or absent and no mode-blind/blanket-chmod path survives | green — 2026-08-29 |
| 4 — update integration closure | Prove remote forward update, anti-rollback, candidate-probe failure, acquisition cleanup, and unchanged exact local/rollback behavior; disposition the whole updater class KEEP/COLLAPSE/DELETE | named public integration cases and all affected B1-B4 tests pass nonzero; no second admission/staging/activation implementation survives direct review | green — 2026-08-29 |
| 5 — grouped acceptance | Add no new product behavior; run final bundle proof and synchronize authority | full serial and three complete parallel suites, explicit live read-only smoke, format/diff, warning-free locked release, protected identities unchanged, GOAL update, commit | green — 2026-08-29; commit pending |

#### contract to encode before implementation

- The remote base is a bounded ASCII `https://` URL ending in `/`, with no
  credentials, query, fragment, control/space, backslash, or redirect. After the
  manifest is admitted, its generation identity must match the base's final path
  component.
- Manifest and signature URLs are exactly `<base>release.manifest` and
  `<base>release.sig`. Generation-file URLs are formed only by percent-encoding
  the already validated signed inventory path and appending it to the same base;
  every resulting resource URL retains the 4,096-byte bound.
- Transport is exactly `$PREFIX/bin/curl`; Core disables curl config, clears the
  child environment, permits HTTPS only, supplies the Termux CA paths, applies
  explicit connect/total timeouts and byte bounds, and never follows redirects.
- Core checks the pinned key/OpenSSL/curl contract before acquisition. It admits
  the signed manifest before fetching generation content and verifies the fully
  assembled local bundle again through B4 before staging.
- Acquisition uses one create-new private directory beneath the Core generation
  root and one create-new output file per resource. Cleanup is attempted after
  every outcome and must succeed before activation; a cleanup error is terminal
  and preserves authoritative state. A killed process or failed cleanup may
  leave a non-generation dot-prefixed temporary; it is never scanned, launched,
  or activated and does not justify a retry, registry, or fallback mechanism.
- Manifest/signature retain B4's 128 KiB / 1 KiB bounds. Generation acquisition
  must have one explicit per-transfer/aggregate cap enforced both in curl and on
  observed regular-file sizes; exact constants are fixed and tested in slice 1
  before content acquisition begins.
- The release manifest is strict `codex-release-v2`. Every file record binds its
  canonical four-octal-digit regular-file mode as well as path and SHA-256; local
  source, staged generation, and remote reconstruction all use this one check.
  There is no v1 parser or blanket executable-mode fallback.

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
