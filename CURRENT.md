# Current product state

Updated: 2026-11-10 KST. This is **current evidence only**, not product
requirements, task instructions, historical chronology or deployment approval.
Source behaviour is governed by [SPEC](SPEC.md); the work order is
[HANDOFF](HANDOFF.md). Completed historical status is retained by Git,
not as a competing current source.

## Selected operating generation (last observed)

- Signed device selection: Codex 0.167.0 **Full159**, generation
  `full-v0.167.0-681ecce-s159`. **Full153** is retained as rollback.
- Installed Core/Manager/TUI, signed Provider, separately configured Provider
  service, profile/home and workspace are *different authorities*. No new
  operating image, signing, activation, profile, service or rollback change
  was made by the current integration source work or documentation refactor.
- These are the latest retained device observations, not a claim of a fresh
  readback after every source commit. Re-observe before operational action.

## Source and exact qualification

Integration branch:
[`integration/codex-native-20261110`](https://github.com/humtr/codex/tree/integration/codex-native-20261110).
Last product-runtime change baseline:
`5384d9386c2acf0e0fd66b10b1be8bc34099b931`.
Most recent completed Source Acceptance baseline *before this review-doc
update*: `0aebdd7f2614a38c08f96e1c266f0a7c2d88609a`.
That exact commit passed CI after documentation/check changes, but those
changes did not repair operating runtime or requalify live-model behavior.
A later review-document commit does not inherit this CI receipt.

| Boundary | Observed evidence | Limit |
| --- | --- | --- |
| [Source Acceptance #42787269906](https://github.com/humtr/codex/actions/runs/42787269906) | Success, 5/5 jobs at tested `0aebdd7f` | Source/CI only, not live model/device acceptance |
| [Optional Provider qualification #42758329834](https://github.com/humtr/codex/actions/runs/42758329834) | Success, 2/2 jobs at `5384d938` | No live-model admission |
| [TUI Native CI #42742653586](https://github.com/humtr/codex/actions/runs/42742653586) | Success at earlier `f1ba6719` | Not exact `5384d938` whole-TUI qualification |
| Focused TUI source tests | 40 Python plus 10 assembly checks passed | Synthetic/local only |
| Synthetic Core → TUI → Chat → Native | Accepted; same thread; three synthetic requests and one synchronization | No real GPT-6 invocation |
| Isolated candidate `/provider` → GPT-6 | Picker selection succeeded; first real input failed `native_thread_owner_unavailable` | No completed model turn |
| Explicit `--profile chatgpt` diagnostic | Previously succeeded in two real GPT-6 turns on one Native thread | Not the required `/provider` entrypoint |

The failed isolated test mixed the operating Provider's installed Core/HOME/
workspace owner binding with a separate candidate Core/home/workspace. This
**environment/ownership mismatch was observed**; neither a Core/Manager defect
nor successful installed-route continuity follows from it. Earlier initial
`cdp_operation_timeout` and HTTP 413 failures retain their original evidence;
there is no proof that the final product route has recovered both. The short
socket-root correction allowed the candidate TUI to start; it did not resolve
its subsequent owner mismatch.

## External and internal review evidence (source-only)

The user-supplied 2026-11-10 review of `49cfab80` contains 109 findings,
re-assessed as **80 true / 28 partial / 1 false positive** as source analysis,
not independently reproduced defects. A separate historical internal review
had **26 reflected / 3 partial / 7 already satisfied / 14 unsupported /
24 unresolved** among 74 differently indexed items. Neither count is
additive. Model transcripts contain no complete native tool-call traces.

Read-only spot-checks at `0aebdd7f` support all ten **P0 candidate
mechanisms** in source: Core rollback guard/redaction/entry parser (U1-2/4/6),
local unauthenticated MCP controls (U3-1), pre-dispatch journal and opaque
claim recovery (U4-3/6), one-shot TUI marker and unbounded outer input wait
(U5-1/4), destructive handoff before accepted RPC (U6b-2) and branch/CI
notification ownership (U7-4). U3-4's parallel-slot reservation gap is
also source-visible. **No patch, device-level failure reproduction,
penetration test, live-model or signed-release test was performed here.**

See [temporary review actions](docs/development/REVIEW_ACTIONS_20261110.md)
for work owners, exclusions, ordering and exact positive/negative
qualification criteria. A finding or CI pass is not proof of deployment.

## Current acceptance and deployment decision

**Not accepted for release** until *all* are independently evidenced:

1. Selected installed/qualified Core dispatches the TUI `/provider` route,
   with configured Provider and the **same actual Native writer**.
2. Real GPT-6 first response and a follow-up response complete on the **same
   Native thread**, and the original history is preserved.
3. Fault-injected SSE cut, HTTP 413, re-entry and result recovery preserve the
   original accepted request/effects, with **zero duplicate submission**.
4. A separate Native work can start and finish while unrelated work holds a
   bounded effect; unsafe conflicting operations are still rejected.
5. Correct exact-source artifact chain, source/Provider/TUI gates, release
   admission and explicit operational authorization before signing/activation.

**Do not** replay an uncertain turn, substitute a synthetic response, install
a candidate, adjust an account/profile or restart services to hide a gap.
No claimed deployment acceptance follows merely from CI success.

## Source organization and limits

- `provider-chatgpt-v...` is the new producer naming; published
  `provider-gpt-v...` identifiers remain readable for historical rollback.
- Native TUI source is `frontend/tui`; historical external identifiers
  `manager-tui-native`, `termux-manager-tui-v1`, and builder aliases remain.
- The historical WIP account draft is not a shipped feature.
- Native and Provider own separate transport vs execution effects; pending or
  lost delivery must be reconciled against **original** receipts.

## Where to find older evidence

Use exact historical Git commits and paths only when additional original
evidence is needed. Historical material does not prove current activation
or accepted model behavior.
