# Stage 2 Completed Milestones and Retired Work Order (2026-11-22 – 2026-11-23)

Historical record of the completed Stage 2 slice execution on `main`. The
original work order is preserved in Git with `git show a5f50173:HANDOFF.md`.
See [CURRENT.md](../../CURRENT.md) for live state, [SPEC.md](../../SPEC.md)
for the product contract, and [ROADMAP.md](../ROADMAP.md) for open verification.

## Retired Stage 2 Work Order

- **Slice 1 (`3ea207cb`).** Published the Ed25519-signed `wrapper` release at
  sequence 1 and installed it on Termux with `--require-signed --skip-upstream`.
  [Wrapper release CI](https://github.com/humtr/codex/actions/runs/44997871190)
- **Slice 2 (`aa0b8c09`, `941eb277`).** Shipped the daemon client and standalone
  `codex-ext` / `cdx` profile, `!cdx switch`, provider and notification routes.
  Isolated upstream `0.171.0` verification covered the writer lock and shared
  history. Retired `contrib/codex-account`. [Slice 2 evidence](../../CURRENT.md#2026-11-22--stage-2-slice-2-codex-daemon-client-standalone-codex-ext-cdx-7-proof--live-termux-install)
- **Slices 3 and 4 (`816ab755`, `f0f90d81`, `ed3d9daf`).** Shipped
  `mcp-kit`, patched `tunnel-client v0.0.21`, the three-state `runit`
  supervisor, and independently enrolled and exercised the stateless
  `@TermuxShell` and `@TermuxCodex` bridges.
- **Slices 5 and 6 (`de7b7080`, `5ce5b976`, `64667a35`, `336b9c1c`).**
  Qualified immutable AppHost `26.1117.41037`, enrolled the Provider and
  `@TermuxChatGPT`, and exercised programmatic registration, multi-worker
  iteration and three-lane re-entrant execution through the live connectors.
  [Device evidence](../../CURRENT.md#recent-verified-evidence-newest-first)
- **Slice 7A (`fd3c445d`).** Deployed `@TermuxOrch`, the Work Ledger, Typed
  Inbox, Wait-Set, relay and tiered orchestration. Verified Front rebinding,
  Tier 2 promotion, and Tier 3 restart recovery with two workers and a
  verifier. [Termux-Orch CI](https://github.com/humtr/codex/actions/runs/45131891229)
- **Slice 7B (`b51ade9e`).** Verified live turn-count Lead rollover with
  generation fencing and Inbox replay, detached Shell settlement across a
  relay restart, and Stop-hook wake-up. Natural context exhaustion and a
  continuous Shell run longer than ten minutes remain open.
  [Slice 7B evidence](../../CURRENT.md#2026-11-23--stage-2-slice-7b-resilience-on-android-api-37-termux)

## R-1 Signed Stage 2 releases

The first seven Stage 2 releases are recorded here. The Slice 5 and Slice 7
audit fixes were first published as `0.1.1` from `d6b6247d` with Provider
sequence `8` and Orchestration sequence `5`. The next deployment restored
the development package version `0.1.0+7b` with the audit fixes on both
release tags and the device.
[Initial deployment record](https://github.com/humtr/codex/blob/d8a58a4655390285a5ead5e6fef1be36833cee4c/CURRENT.md#2026-11-23--signed-provider-and-orchestration-011-active-on-termux) ·
[Version restoration record](../../CURRENT.md#2026-11-23--provider-and-orchestration-development-version-restored-on-termux)

All seven stable releases were dispatched from `main` at
`f3c9334ef3e31679ce0323bf5bf9786f69546c41` with `publish_release=true`.
That commit fixes the Termux-Orch publish step's publisher arguments and
signing-key setup. The corresponding push checks passed.
[Termux-Orch push CI](https://github.com/humtr/codex/actions/runs/45139460951)
and [Wrapper push CI](https://github.com/humtr/codex/actions/runs/45139460975).

Each public readback matched the source tag and the four expected assets
(the Android archive, its `.sha256`, `SHA256SUMS`, and `SHA256SUMS.sig`).
OpenSSL independently verified every 64-byte Ed25519 signature using the
repository's [release public key](../../release/release-public-key.pem)
with SHA-256 `0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca`.
The downloaded archives matched both the signed manifest and checksum sidecar.
GitHub's default stable release lookup resolved to `tunnel-client` after
publication. The Wrapper installer uses the explicit `wrapper` tag.
[Installer release selection](../../README.md#option-3-install-directly-from-a-ci-published-github-release)

| Stable release | Successful dispatch | Signed sequence | `SHA256SUMS.sig` SHA-256 |
| --- | --- | ---: | --- |
| [`codex-ext`](https://github.com/humtr/codex/releases/tag/codex-ext) | [45139504172](https://github.com/humtr/codex/actions/runs/45139504172) | 5 | `c1319f1cfc1e5cf3b9caed2a30370ed5e37eeabefdb65fee014d878671cd69b2` |
| [`termux-shell-mcp`](https://github.com/humtr/codex/releases/tag/termux-shell-mcp) | [45139505045](https://github.com/humtr/codex/actions/runs/45139505045) | 4 | `212b7ae707f0f102fec598b84e905b3489223a5ad3c443ae6f952815edf7dcf0` |
| [`termux-codex-mcp`](https://github.com/humtr/codex/releases/tag/termux-codex-mcp) | [45139505666](https://github.com/humtr/codex/actions/runs/45139505666) | 4 | `ca72940e6b5628df5c4d8d92d9831bbf70c841d678191ee2715d2eb5d2d8455e` |
| [`chatgpt-provider`](https://github.com/humtr/codex/releases/tag/chatgpt-provider) | [45139505982](https://github.com/humtr/codex/actions/runs/45139505982) | 7 | `84267323f3b1ca2524d7d921edd5ce88713d0d71dceee46d0568974d742cf637` |
| [`termux-chatgpt-mcp`](https://github.com/humtr/codex/releases/tag/termux-chatgpt-mcp) | [45139505909](https://github.com/humtr/codex/actions/runs/45139505909) | 4 | `fc144fa3b3b01fbbeb2078a6f0411fe31647fdd93823bcea0ccb8c15774fd83c` |
| [`tunnel-client`](https://github.com/humtr/codex/releases/tag/tunnel-client) | [45139505399](https://github.com/humtr/codex/actions/runs/45139505399) | 4 | `d11d6fae82652f390e7ce13a24b1b689a1ff390b738e8057518034eaef3acdb5` |
| [`termux-orch`](https://github.com/humtr/codex/releases/tag/termux-orch) | [45139505805](https://github.com/humtr/codex/actions/runs/45139505805) | 4 | `e8535c357c2935422ba013c998254411ca5ae57e653ef3cd3666a4420044c1ff` |

## R-2 Authenticated UUID handoff and session lookup

On Android API 37, the two existing Codex `0.171.0` daemons reported distinct
ChatGPT identities. The `janmori101` OpenAI `gpt-6.2-sol` turn
`01a2078c-c3c0-7962-8a49-2f46a32d3da7` completed on disposable thread
`01a2078c-c287-7630-a65f-0a7b24b680c9`. A live `flock` probe confirmed
the writer was held before and after `szob` received RPC `-32600`
for an active writer. With a one-second unload delay scoped to the
disposable thread, `thread/unsubscribe` returned `unsubscribed` and the
lock probe cleared after 1,055 ms. `szob` resumed the same UUID and its
completed OpenAI `gpt-6.2-sol` turn `01a2078c-e3b8-7160-a0a9-773ad1d7535e`
returned the exact nonce from the first assistant reply. Both daemons stayed
running. The handoff used their WebSocket-over-UDS RPCs. The `cdx switch`
launcher and an unrelated concurrent A turn were outside this run.

Both daemons were running for the rename, archive, unarchive and delete
readbacks. Renaming from `R2-before-0a7b24b680c9` to
`R2-after-0a7b24b680c9` left two append-only index rows and one current name in
`state_5.sqlite`. Archive moved the rollout and flipped SQLite `archived`
from `0` to `1`. Unarchive reversed both changes. The index retained both
name rows through the archive cycle.

| Lookup path | After rename | After delete |
| --- | --- | --- |
| `thread/list`, both accounts | UUID and new-name search returned the new name. Old-name search found no match. The active and archived filters tracked the archive cycle. | UUID and both name searches found no match in either collection. |
| `thread/read`, both accounts | UUID returned the current name even when archived. Both names returned invalid thread ID. | UUID returned missing thread. Both names still returned invalid thread ID. |
| Upstream TUI in a Termux PTY | Direct UUID and new-name resume restored the transcript on B. Old-name resume exited `1`. The startup and `/resume` pickers showed the renamed entry on A and hid the A-created thread on B under the account filter. | Direct UUID and both names exited `1`. Searching the picker for either name returned no result. |
| Local `codex exec resume` on B | UUID and new name completed online turns on the original UUID. The old name missed and fell back to a fresh test UUID, `01a20792-e83d-7630-8b82-ff919b2cec02`. | The UUID exited `1` with RPC `-32600` and no rollout found. Both names missed and fell back to fresh sessions with `--ephemeral` enabled for those probes. |

The upstream thread store owns the SQLite and rollout lifecycle. The list and
TUI label lookup reported the current title. The append-only name index
retained the old label through rename, with no working alias in this test.
The default-home daemon performed `thread/delete` on both disposable UUIDs
using the shared index path directly. The two account homes retained their
index symlinks. The shared index returned to its original 175 rows
byte-for-byte (SHA-256
`30c74d14fd723ba6bcf60a9de975866b13e5bd43288ae358cb11420d8442357b`).
SQLite `PRAGMA quick_check` returned `ok` and the disposable workspace had
zero remaining thread rows. See the [upstream list and title lookup](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/tui/src/named_session_lookup.rs),
[delete implementation](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/thread-store/src/local/delete_thread.rs)
and [exec resume resolution](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/exec/src/lib.rs).

## R-4 Legacy disk reclamation

Removed only
`/data/data/com.termux/files/home/.codex/packages/app-server-daemon/releases/`
after checking its package selection, accessible symlinks and process
references. Reclaimed 514,213,376 allocated bytes (490.39 MiB).
The default, `janmori101`, and `szob` daemon package selectors pointed to
the shared `standalone/current`. The other five profiles had no daemon
package directory. No named profile had a `releases/` directory.

Retained `~/.local/state/codex/legacy-0.167.2-archive/`
(1,815,784,960 allocated bytes). PIDs 20289 and 20290 mapped files in the
archive, which also held `auth.json`, a session DB and two rollouts.
The process and symlink inventory used Termux-visible paths. Android denied
some sandboxed FD and directory listings.

After reclamation `codex doctor --json` returned `summary.status=healthy`
and `codex wrapper status --verify --json` returned `qualified=true`.
Active `0.171.0-d8c43b-2` and rollback `0.170.0-3f7e1a-2` matched the
pre-check. All three `0.171.0` daemons reported running, and the nine
monitored `runit` services retained their running PIDs.

## Remaining verification

See [current verification results](../../CURRENT.md#unverified--remaining-boundaries)
and the [remaining operational checks](../ROADMAP.md#8-remaining-operational-and-physical-verification).
