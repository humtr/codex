# Current product state

Updated: 2026-11-11 KST. **Observed evidence and proof limits only**.
This file grants no implementation, CI, deployment or account authorization.
Requirements: [SPEC](SPEC.md) and annexes. Unfinished work:
[HANDOFF](HANDOFF.md). Document authority:
[docs/DOCUMENTATION](docs/DOCUMENTATION.md).

## Current source and release boundaries

- The developer integration line is
  [`rewrite/rust-core`](https://github.com/humtr/codex/tree/rewrite/rust-core).
  `integration/codex-native-20261110` HEAD `a55d5bb8ff` is already an
  ancestor. Core compatibility-link [PR #29](https://github.com/humtr/codex/pull/29)
  merged on 2026-11-11 as `afdfb8bd7e574c749132254c55cd3f66204d3289`.
  This record predates the documentation commit; **query the actual current
  HEAD**, and do not inherit tests from an ancestor automatically.
- `main` at `333b3c19e5af` was checked as the publication/approval line.
  It has separate ancestry from `rewrite/rust-core`; this is not an
  instruction to merge the two histories.
- [`release/upstream.json`](release/upstream.json) selects official upstream
  **Codex 0.167.1**, source `0a94398fa60c4fac0e7659e4213a0a36e613dd46`.
  The repository's signed
  [Full163 0.167.0 publication](https://github.com/humtr/codex/releases/tag/full-v0.167.0-c19b61b-s163)
  was published 2026-11-11 UTC as
  `full-v0.167.0-c19b61b-s163`. A signed published generation does not
  prove a new source commit was shipped or activated.
- [PR #29's device report](https://github.com/humtr/codex/pull/29)
  states the installed device's derived `core/current` alias was
  corrected to signed **Full163** and **Full159/Full153** were retained.
  This is the PR's reported operational result, **not a new independent
  local device readback from this documentation integration**. The source
  alias repair merged via PR #29 is **not in signed Full163**; installing
  it would require separate qualification, signing and activation.
- The `core/current` alias is derived compatibility state, never Core
  activation, key, rollback or runtime-selection authority. See
  [delivery state contract](docs/spec/delivery/STATE.md).

## Exact source and CI evidence

| Boundary | Latest specifically observed receipt | Proof limit |
| --- | --- | --- |
| Source at `f9dfc3f` | [#42942784769](https://github.com/humtr/codex/actions/runs/42942784769): success | Source/CI only |
| Core at `f9dfc3f` | [#42942787296](https://github.com/humtr/codex/actions/runs/42942787296): success | Core source/build, not installed route |
| TUI Native at `f9dfc3f` | [#42945712023](https://github.com/humtr/codex/actions/runs/42945712023): **failure** | `pin_action_updates_shared_section_state` assertion failed; do not claim full TUI qualification |
| Earlier TUI at `c19b61b` | [#42929259774](https://github.com/humtr/codex/actions/runs/42929259774): success | Not proof for `f9dfc3f` or new HEAD |
| Source at PR #29 head `a8cb66c` | [#42962985877](https://github.com/humtr/codex/actions/runs/42962985877): success | Before merge/documentation commit |
| Core at PR #29 head `a8cb66c` | [#42962989662](https://github.com/humtr/codex/actions/runs/42962989662): success | Before merge/documentation commit |

Do not infer qualification for the new merged or future document-edit SHA
until the exact commit's checks actually complete. CI, provider mock and
synthetic rounds are not live-model or installed-device acceptance.

## Open P0: Native execution availability

Live TermuxBridge observations on **2026-11-11 KST**:

- Before reboot, multiple stale-looking work phases remained despite
  individually confirmed Native Stop responses; some Host acknowledgments
  failed with `mcp_owner_unavailable` or `app_lane_missing`. Owner status
  and Native-effect settlement were not always equivalent.
- The user fully terminated and restarted Termux. The Bridge/AppHost then
  responded, and a fresh attached Native work created, started and attached.
  But an actual read-only `functions.exec_command` for
  `printf 'TERMUX_RESTART_NATIVE_OK\n'; pwd` returned
  `native_resource_busy`, `executed=false`, `retryable=true`.
- On the affected new work, `native_effects.active=[]`,
  `unconfirmed_effects=0` and `pending_calls=[]`; at least some older work
  records still showed `reconciliation_required`/`stop_requested`.
  This **does not prove** there is no global owner or active external job.
- No command ran in the Termux shell through that Native call and no
  unconditional OS-level kill/lock purge was performed. The Bridge wrapper's
  effective shared-effect admission or recovery must be inspected using a
  distinct real local shell before identifying the root cause or declaring
  recovery. Termux restart alone was not sufficient in this observed trial.

## Remaining acceptance distinctions

- The installed matched Core → TUI `/provider` → ChatGPT GPT-6
  first and follow-up responses on **one original Native thread** are not
  currently independently accepted. A picker choice, prior isolated
  `--profile chatgpt` success or an attached-work ACK cannot replace it.
- HTTP 413, SSE/stream loss, uncertain tool-result settlement,
  Stop/re-entry continuity, real allow/deny reviewer evidence and safe
  independent concurrency require original identity, receipt, execution
  and recovery proof. `native_resource_busy` must not indefinitely block
  unrelated eligible work.
- Source uses `provider-chatgpt-v...` producer naming while required
  `provider-gpt-v...` published/rollback identifiers remain readable.
  `frontend/tui` is the native TUI source; historical externally consumed
  TUI/manager identifiers remain compatible. Draft ChatGPT account work
  is not an accepted installed feature.
- The dated [review matrix](docs/development/REVIEW_ACTIONS_20261110.md)
  is a snapshot of 2026-11-10 source findings, **not newly reproduced device
  failures** or the authority for the current signed generation.

Every later proof belongs under its exact SHA, work/thread, model, device,
source and qualification boundary; preserve failed original receipts.
