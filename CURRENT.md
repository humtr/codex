# Current product state

Updated: 2026-11-10 KST. This is **current evidence only**, not product
requirements, task instructions, historical chronology or deployment approval.
Source behaviour is governed by [SPEC](SPEC.md); the work order is
[HANDOFF](HANDOFF.md). Previous rolling status (2,168 lines) is
[archived, not authoritative](docs/legacy/2026-11-10/CURRENT.md).

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
Last code-qualified commit: `5384d9386c2acf0e0fd66b10b1be8bc34099b931`.
A subsequent documentation-only commit `49cfab806cb99d47a6de0d918ed13777db8e11a5`
does not inherit new exact-HEAD CI by assertion.

| Boundary | Observed evidence | Limit |
| --- | --- | --- |
| [Source Acceptance #42758325312](https://github.com/humtr/codex/actions/runs/42758325312) | Success, 5/5 jobs at `5384d938` | Source, not device activation |
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

Original audits, old generations, prior failures and long-form work logs live
in [the legacy index](docs/legacy/README.md), including the exact former
CURRENT.md. Their occurrence in Git history is not evidence of current
activation or accepted model behavior.
