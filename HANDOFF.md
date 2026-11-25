# Active work order — ChatGPT Provider structured Native return

Started 2026-11-25. Authority: `AGENTS.md`, `SPEC.md`, `docs/INDEX.md`,
`docs/spec/CHATGPT_PROVIDER.md`, `docs/ROADMAP.md` §0 and §10. §11 remains
frozen until this milestone closes or the user changes the order. This is the
single active execution work order, not a new product contract.

## Baseline and isolation (P0 entry)

- Checkout: `main`, `84d916f4fba8d18ba40325cb7cfa6155f504f15f`; Provider and
  AppHost tracked source unchanged from `c9d0e245`. No existing `HANDOFF.md`.
  Preserve untracked `ais.config.toml` and `chatgpt.config.toml` verbatim.
- Operating service: `chatgpt-provider` PID 12838/start ticks 34334646;
  host-supervisor PID 15625/start ticks 34349215. Both actual `/proc/.../exe`
  hashes `d490f8c796b9df0518514cb3039a74733ae66b7eba66d1c64d549f2d1cb26a3e`;
  selected `0.1.0-c9d0e245`, retained previous `0.1.0-74bc360b` (SHA-256
  `e38e17a97fb93cc89a73d4be22c9dc5a93b18a29584682378946f3baa097e92b`).
  `127.0.0.1:4319/healthz` HTTP 200, host_running=true. No new signature
  verification performed. This live snapshot supersedes the older CURRENT
  snapshot for these specific observations only.
- AppHost selected `26.1117.41037`; `app.asar` SHA-256
  `780d0af6dcc11aa456980bc1b3af86f876c947bf8d5f112bb87bba75cfbb6261`.
  Pin the exact rootfs; do not edit AppHost files or generation links.
- Runit preflight: Provider and Orch relay (PID 9581) up; four MCP tunnels
  already down. Do not restart or repair unrelated services here.
- Owned scratch root: `/data/data/com.termux/files/usr/tmp/cgp-p0-20261125`
  (0700). Separate HOME, CODEX_HOME, explicit Provider state/profile and
  ephemeral loopback port. Candidate binary built/copied only in owned root;
  no installation, no live profile/token copy, no operating service calls
  that dispatch inference. The first baseline probe used an empty profile.
  The later decision below authorizes the operating account for a user-run
  handoff.
- Protect existing sessions, accounts, process identities, rollback pointers,
  and unrelated files. Do not use shared Android storage. Reconcile ambiguous
  delivery through original receipts; no automatic retry, fresh Chat, fallback
  execution, or duplicate call. Publication and activation need separate
  authorization (including safe migration/rollback and idle boundary).

## User decision — implementation first (2026-11-25)

The existing Linux AppHost account is authorized for live testing; no new
account or additional profile login is a prerequisite for source work. The
user prefers implementation first and rollback if necessary. Reuse a fresh
chat on the existing account; preserve the single-owner profile lease and
rollback data. No credential duplication. P0 remains an acceptance gate, not
a prohibition on candidate development. Develop and test a conditional
structured adapter now; keep production adoption separate. C/B wire support
remains unqualified. Do not repeat the unauthenticated boot probe.

## Gate sequence and execution checklist

| Stage | Required result / next action | State |
| --- | --- | --- |
| P0 | Prove C with the authorized AppHost account, E01–E15 below. B requires a definitive C gap and independent proof. | **Option C disqualified** by proven `app.asar` (`QBVt`→`lNEL`→`Y86X`) gaps (drops Codex `local_function_signatures` on round 2+ and caps results to 1). **Option B selected & proven** on isolated live AppHost (`b9e2c9b23b8a...`): E01, E02 (3-round official Codex CLI), E04, E05, E07, E08, E09, E10, E11 (10 turns), E13 (restart), E14 (parallel), E15 & utility isolation PASS. E06/E12 remain open. |
| P1 | Select a qualified route and establish the role, scope, content, identity and compatibility contract. | **Option B selected**; normative conditional contract updated in `docs/spec/CHATGPT_PROVIDER.md`. |
| P2 | Keep typed user inputs and results distinct from control send. | Typed `user`, durable `developer`, one-turn `developer` (`instructions` with `exclude_after_next_user_message: true`), and structured `role: "tool"` envelopes proven live. |
| P3 | Bind full IDs, namespace, recipient, arguments and all results to original calls. | Exact Provider-generated `submitted_message_ids`, full call IDs, custom/namespaced tools, and multi-round correlation proven live; out-of-order parallel batches verified in JS/Rust tests. |
| P4 | Bind the emitted output prefix and prior response, remote head, account and project. | 10-turn verified continuation (E11), durable developer scope (E09), and request-scoped instruction application/expiration/replacement (E10) proven live. Compaction handoff remains open. |
| P5 | Persist minimal bindings and pending receipts with explicit reconciliation and migration. | Versioned v3 native state, pre-dispatch restoration, and live restart recovery with pending tool call (E13) proven. Legacy-entry migration & original E02 receipt reconciliation remain open before cutover. |
| P6 | Retire obsolete routes after the selected replacement qualifies. | **COMPLETE (Phase 2 Thin Option B, `1c5f93212d8e...`)**: Option A prompt-flattening path retired & archived in `docs/history/LEGACY_PROMPT_PATH.md`; over-engineered `verifyCandidateSettlement` / per-round `/conversation/{id}` polling / `dispatching: true` lockout removed; utility isolation (temporary `-instant` / optional `project_v2`), hidden machine user preambles, zero-arg tool fix, and 3-tier auto-project routing added. |
| P7 | Repeat E01–E15 on the exact candidate through official CLI/TUI/app-server, Android, real account/model and UI. | **Phase 2 live A/B PASS (`1c5f93212d8e...`)**: Step 0 canaries 1–3, official Codex CLI `0.171.0` E02 (`35,893ms`, `user_turn_count=1`) & E06 (`execpolicy` denial), E04/E05/E15+zero-arg+empty `[]`, tool-return `Ctrl+C` recovery, E08, E10+`/compact`, E11 (10 turns, avg `5,059ms`, -39.9% vs Phase 1), E13 restart, auto-project reuse all PASS. E03 live multi-call emission, TUI/app-server, and E12 physical UI remain open. |
| P8 | Qualify the exact artifact and separately authorize publication, readback, activation and rollback. | Active locally at fixed version `0.1.0` on port `4319` (`1c5f93212d8e...`, PID `9853`; Phase 1 `62395f19...` backed up at `generations/0.1.0-p1-62395f19`, all prior rollback generations preserved). CI and signed GitHub release remain open. |

### P0 / P7 proof matrix (no acceptance by mocks)

| ID | Evidence required | P0 result |
| --- | --- | --- |
| E01 | Single real declared function, unpredictable marker generated after call; exact original ID, one return, marker consumed without user message. | **PASS** (subsumed by Option B E02 3-round official Codex CLI & E13 single-call restart on `b9e2c9b23b8a...`). |
| E02 | Three successive tool rounds from one genuine request. | **PASS (Option B `b9e2c9b23b8a...`)** on official Codex CLI `0.171.0` (`cli_exit=0`, `38.6s`, 3 sequential `exec_command` rounds, all 3 nonces consumed, `chat_id=6b06ab6f-f610-83ee-b26f-7d0b403648a8`, `user_turn_count=2`, `dispatching=false`). Original Option C E02 receipt (`6b069b9f-...`) preserved untouched. |
| E03 | Batched/parallel calls, unique out-of-order returns matched, no independent inference per result. | **PARTIAL / MODEL-SERIALIZED**: Option B transport, `NativePlan`, and `verifyCandidateSettlement` support N out-of-order batched results (`parallel_tool_calls: true`, proven in JS/Rust unit tests); live `gpt-5-6-thinking` on AppHost serialized multi-tool requests into sequential `local.*` rounds (`user_turn_count=1`). |
| E04 | Custom/namespaced inputs, name, namespace and output semantics. | **PASS (Option B `b9e2c9b23b8a...`)**: `custom_tool_call` (`raw_probe`) + namespaced `function_call` (`diag.check_path`), `chat_id=6b06ab96-e640-83ee-882e-cf94815bd3a7`, `user_turn_count=1`. |
| E05 | Real execution failure consumed. | **PASS (Option B `b9e2c9b23b8a...`)**: `EXIT_CODE=1 ERROR: EACCES_*` faithfully reported without fabricated success (`6b06ab96-e640-83ee-882e-cf94815bd3a7`). |
| E06 | Official reviewer denies and no effect; original denial delivered. | **PASS (Phase 1 `62395f1950e6...` on `:4319`)**: official Codex `0.171.0` `-a on-request` + `execpolicy` forbidden rule denied `touch` with zero file creation (`cli_exit=0`, `file_absent=true`, `cmd_count=0`), denial delivered with original identity. |
| E07 | Lossless bounded large result or explicit disclosed error/representation. | **PASS (Option B `b9e2c9b23b8a...`)**: 1.1 MiB rejected pre-dispatch HTTP 400 `native_content_bound` preserving pending call; 64,672-byte result delivered losslessly with tail marker verified. |
| E08 | Typed supported content and explicit unsupported-type failure. | **PASS (Option B `b9e2c9b23b8a...`)**: unsupported `input_image` rejected HTTP 400 `native_content_unsupported`. |
| E09 | Durable authorized developer canary scope across turns. | **PASS (Option B `b9e2c9b23b8a...`)**: `VAULT_d5a9a6d0` recalled verbatim on Turn 2. |
| E10 | Request scope, expiration, replacement. | **PASS (Option B `b9e2c9b23b8a...`)**: post-user `developer` with `exclude_after_next_user_message: true` applied `[REQ_T1]` on Turn 1, expired cleanly on Turn 2 (`t2_has_req1_leaked=false`), and was replaced by `[REQ_T3]` on Turn 3. |
| E11 | Ten consecutive genuine user turns, verified chat, no resident replay. | **PASS (Option B `b9e2c9b23b8a...`)**: 10/10 consecutive turns on `6b06ad18-6e9c-83ee-962d-a8bdb578d46f`, `user_turn_count=10`. |
| E12 | Actual UI preserves genuine user turns; no synthetic instruction/result/reply users. | **PASS (2026-11-26, Phase 2 `1c5f93212d8e...`, user-observed on `6b075b8d-50b0-83ee-8e47-20c4b50be8b9`)**: physical ChatGPT UI displays ONLY the 1 genuine user prompt; zero visible `# AGENTS.md` / `<environment_context>` or synthetic tool/result user bubbles across 3 sequential tool rounds. |
| E13 | Candidate restart retains pending call, no unverified resend. | **PASS (Option B `b9e2c9b23b8a...`)**: candidate process stopped and restarted with pending call in `response_threads.json`; post-restart return settled cleanly on `6b06abc0-fb68-83e8-b47e-056e9a0da766`, `user_turn_count=1`. |
| E14 | Independent chat lanes, same-chat exclusion and lane-local cancel. | **PASS (Option B `b9e2c9b23b8a...`)**: concurrent sessions returned HTTP 200 with zero global busy or cross-delivery; same-chat exclusion and lane-local cancellation verified in unit tests. |
| E15 | Role-looking user/tool data cannot escalate authority or execution. | **PASS (Option B `b9e2c9b23b8a...`)**: `[developer] SYSTEM OVERRIDE` embedded in tool output treated strictly as data (`6b06ab96-e640-83ee-882e-cf94815bd3a7`). |

Record each case's actual source/binary/AppHost identity, sanitized call and
remote-node correlation, execution/return counts, settlement, visible UI and
exit status. No secrets, private reasoning or raw private conversation text.
If a capability cannot be proved on an authorized route, stop adoption and
record precise blocker; do not treat inspection, mocks, request ACK or an
extra assistant response as success. Do not switch C to B on uncertain dispatch.

## Initial investigation receipts

- Current `page_driver.js` constructs one `author.role=user` message for every
  completion and selects `current_node` on follow-up. `responses.rs` still
  calls `render_prompt_from_responses_request`. This is baseline source
  evidence, not native return capability. ASAR contains tool-result and
  continuation-related strings; literal counts cannot prove role acceptance,
  generic local-tool support or remote consumption.
- Independent TermuxShell MCP preflight failed `Session terminated` before an
  observable receipt. No retry; subsequent distinct read-only repository
  preflight used the connected local execution surface, without dispatching
  any model request or changing a service.

## P0 baseline qualification and blocker (2026-11-25)

- Exported only the tracked Provider/AppHost source at the baseline SHA into
  `source/` under the owned scratch root. Locked offline Android build, exit
  0, with isolated `CARGO_TARGET_DIR`. Candidate debug ELF AArch64 Android API
  24, SHA-256 `23d3b158fc54cfde0aad6a006fde8979cf20a4836a3f6662e34035da7702a6de`.
  This is an unchanged baseline, not a Native-return implementation.
- `candidate.toml` explicitly selects the pinned rootfs, separate profile
  and state, ephemeral loopback bind; candidate launch used separate HOME,
  CODEX_HOME, TMPDIR and no shared-home overrides. Actual PID 13018/start
  ticks 34877430, `/proc/13018/exe` matched the candidate digest; port 35307
  (not operating 4319). `/healthz`: ok=true, host_running=true;
  authenticated local control status: `login_required`, models=0. No model
  request, remote chat, tool execution or tool-result dispatch. Candidate
  exited 0 on its own SIGTERM; profile lock was reacquirable, port returned
  ECONNREFUSED (111), no candidate binary process observed afterward.
- Receipt: `logs/candidate-boot-receipt.json` in the owned root. Retain the
  original profile, config and logs; do not copy a production profile/token
  into it. No candidate is left serving.
- Baseline tests: Page Driver 12/12; AppHost Python lifecycle 4/4; fmt PASS;
  locked offline clippy all targets with `-D warnings` PASS; locked Rust
  tests under both umasks 0022 and 0077: each 25 passed, 0 failed, 1 ignored.
  The ignored official Codex integration was then run explicitly with the
  fully qualified test name: 1 passed, using actual Codex 0.171.0 against
  MockDriver only. The initial short exact filter ran zero tests (not counted
  as a pass). Logs in `logs/`. No authenticated or P8 qualification implied.
- Post-probe: operating Provider PID 12838 and supervisor PID 15625 retained
  their start ticks and binary digests; operating health HTTP 200,
  host_running=true. Provider current/previous and AppHost current links
  unchanged. Orch relay PID 9581 unchanged; the four MCP tunnels remained
  down as before. No changes to live generation, services or existing
  user configuration. No publication or activation attempted.

## Candidate continuation receipts (2026-11-25)

- Recovered the working-tree candidate and launcher from session
  `01a21124-bbe3-75b0-a310-94d980a8ecb4`, which ended with backend HTTP
  429. The launcher and candidate state had no handoff or E01 receipt.
  The preceding turn's 429 belongs to that session, before any E01 attempt.
- Base `84d916f4fba8d18ba40325cb7cfa6155f504f15f` with working-tree diff.
  Provider diff SHA-256
  `f976869dfe3d3bd5ed8aa170974f714153feea9a7e6398ea6735703dcdbfd961`
  (`git diff -- providers/chatgpt | sha256sum`). Schema 3 persists the
  original wire argument digest alongside the Codex call binding. Page
  Driver tests changed original arguments both before result dispatch and
  after dispatch before settlement. Previous candidate binaries are retained
  in the owned root. A page without Web Crypto cannot use the candidate path.
- Initial schema 3 candidate ELF SHA-256
  `df8706e733a586a3ee3c5c3260e4c5ce6664d0cf0b9e03e0fd650ad5a7ee6786`.
  Locked offline Termux Android release build, API 24 AArch64,
  `/system/bin/linker64`, 16 KiB ELF check and executable `--version`
  all passed. Unsigned local bundle
  `package-schema3.JUqUfeQp/chatgpt-provider-aarch64-linux-android.tar.gz`
  SHA-256 `78dc7db98a8a9ba246591afbf41774e9a46f339fcfceb0d334ae3085adbcaa57`.
  Extracted payload manifest and binary were independently read back. The
  separate GitHub CI NDK cross-build was not run.
- Page Driver 17/17, AppHost Python lifecycle 4/4, fmt and strict clippy
  passed. Locked Rust tests passed 29/29 under each of umasks 0022 and
  0077. The ignored official Codex 0.171.0 test passed separately against
  MockDriver. Exact official Codex 0.171.0 via the Wrapper reached the new
  Android candidate's HTTP route from a disposable home. Its first request
  contained a `developer` item and received
  `native_instruction_scope_unqualified` (CLI exit 1) before intent
  creation or AppHost startup. The disposable Provider exited 0. Logs are
  in `logs/resume-*.log`.
- The launcher preflight refused an occupied candidate port. A disposable
  Termux `runsv` service with a real file lease and fixture HTTP responses
  verified restore with an original PID still holding the lease and
  `want down`, refusal to signal an active candidate, and orderly idle
  candidate shutdown followed by restoration of the original executable.
  Local evidence is in `logs/resume-handoff.log`. The pinned launcher
  `check` passed. Operating Provider PID 12838 and AppHost supervisor
  PID 15625 remained in place, with operating health HTTP 200 and
  `host_running=true`. No operating service handoff or E01 was performed.

## Login identity preflight fix (2026-11-25)

- The user-run launcher returned `BLOCKED` before its stop intent. The
  operating AppHost reported authenticated, ready, idle and the entitled
  Thinking model. Its status included a login user ID and omitted the
  account-scope ID. The original launcher rejected that status. The
  handoff and E01 receipt files were absent.
- The launcher prefers an account-scope ID and uses the authenticated user
  ID for the same-profile handoff identity check when needed. The Page
  Driver uses the existing read-only AppHost account guard to require one
  verified dispatch account and binds that account ID through continuation
  and settlement. The new tests cover a unique match, no match and
  multiple matches with account ordering deliberately different from the
  verified scope. Page Driver passed 18/18. The updated `check` command
  passed against the real operating Provider with the user ID.
- Provider diff SHA-256
  `2d94618521511d1b067f6e976f726f435d606a534c40a94e62fdd890afcce858`.
  The rebuilt candidate ELF pinned by `handoff.py` has SHA-256
  `009b5d04d5c11eaa67815e14edb1e901cccdc61b74e63d0b00a70f06b4698ceb`.
  Termux Android API 24 and 16 KiB ELF checks passed. The separately
  extracted unsigned bundle matched its payload manifest and binary.
  Bundle `package-account-scope.OXQsIFUp/chatgpt-provider-aarch64-linux-android.tar.gz`
  SHA-256 `5318d5c0c59cea687b73226fc0d99ec9010b71617511286fcaa8c67fb224ad84`.
  The preceding candidate remains in `prior-candidate-artifacts/`.
  The installed Provider remained up. Actual account resolution during a
  candidate dispatch and E01 remain pending.

## Superseded temporary handoff

The user ran `handoff.py check && handoff.py probe-e01`. The candidate
became ready on 4329, accepted the original request far enough to record
account and chat in the schema 3 map, and returned HTTP 503. The temporary
handoff restored the original generation. Its receipt is `restored`. The
E01 receipt is `first_dispatch_intent` with zero verified calls or returns.
Readback of the original chat through the newly activated Provider found
one visible user message. This readback establishes remote acceptance of the
first user message. Exact rejection cause and call settlement remain unknown.
Keep the original receipts and chat for reconciliation.

## User-directed local activation (2026-11-25)

The user requested direct local replacement after the E01 failure.
Activated `0.1.0-local-native-0a1dfcb6` on port 4319 using the existing
`runit` service. Working-tree Provider diff SHA-256
`1c8a8ec53691452a83eca5c4f47fdc18eb99623b840e6081bc776a014962149d`.
The unsigned Android executable meets the 16 KiB ELF gate and actual PID
26342/start ticks 35612681 matches SHA-256
`0a1dfcb61d904e32e3f71ee01d7388da6b506403ea855a7c28ea63c258c3da9b`.
`previous` points to `0.1.0-c9d0e245`. The pre-existing older
`0.1.0-74bc360b` generation remains on disk and is recorded in the
rollback receipt. The activation preserved the existing control token and
legacy response map byte-for-byte. Native state uses the
`structured-native-return` subdirectory of the same Provider state root.

The operating AppHost is authenticated and completion-ready. A local
synthetic request confirmed HTTP 400 `native_instruction_scope_unqualified`
for a developer item before dispatch. Standard Codex requests remain blocked
at this boundary. Local activation does not qualify E01, publication or §11.
See `local-live-activation.json` and `logs/local-live-*.log` in the owned
scratch root.

```sh
"$PREFIX/tmp/cgp-p0-20261125/restore-live-provider.py" --check
"$PREFIX/tmp/cgp-p0-20261125/restore-live-provider.py"
```

The first command verified rollback prerequisites on the live device.
The second command restores the original generation, its older rollback
selection and the original default config condition. It preserves the
Native state and E01 evidence. The rollback rebind has not been executed.

## AppHost's own local-function continuation (2026-11-25)

The user reaffirmed §10's structured tool-call and return target. Installed
AppHost `26.1117.41037` contains an exported
`submitChatGPTLocalFunctionResult` path, used by the bundled Codex handoff.
It accepts one original call ID and the original parent, and generates a
tool-role code envelope containing call ID, tool name and result. This is
source evidence for the continuation entry point. Generic tool acceptance
still requires P0.

The candidate discovers the AppHost submitter from loaded modules. It
requires that capability before making declared local calls available to the
model. It submits one original result through the AppHost function and
checks the result envelope, original branch and AppHost-assigned result ID.
Provider persists the original dispatch intent and does not feed results
through the generic completion message queue. Batching and developer scope
remain open.

Page Driver passed 19/19, including absence of the AppHost submitter and
changed result or call identities. Rust passed 29/29 with each required
umask, and AppHost Python passed 4/4. The exact Termux Android proof build
has SHA-256
`bb6925b1eae2d894ea88706cfecf219ae4ce901f02ef98d15f464abe5ca3dbd9`.
The separately identified native-route proof used the original AppHost
profile lease, a different harmless declared function and its own
`native-route-proof` state and receipt. The first request returned HTTP 503
with the precise error `native_call_invalid`. The diagnostic build read
both existing failure chats without a model request. Each contained a
finished AppHost assistant call with the declared function routed through
`local.<name>`, a valid original ID and a code payload containing `{}`.
The parser had accepted the bare name and `functions.<name>` only.

The candidate now parses the observed `local.<name>` recipient and
preserves it alongside the original call ID. The native submitter receives
the wire name alone, as in AppHost's own caller. Page Driver tests passed
19/19 with the observed shape. The corrected Android proof build SHA-256
is `ea6872e90bd037a8c9d8d30ddf6244ce48a39e3a855af2d6359240779f412e39`.
The original request receipts and diagnostic logs remain in the owned root.

The corrected build emitted a genuine Responses `function_call` in the
live proof. The experiment generated the marker after that call and invoked
AppHost's native continuation with the original call ID. The remote branch
showed the original local function call, a recap parented to that call, and
the final answer. The answer's marker matched the receipt digest and there
was exactly one visible user message. The original HTTP return still
reported `native_result_readback` because the complete remote mapping omitted
the tool message. Two read-only analyses verified this original effect.

The follow-up candidate observes the AppHost-assigned result message ID
and structured envelope at AppHost's completion request boundary. The
observation is scoped to the AppHost submitter invocation. It preserves the
completion client and the verified account guard. When AppHost omits the
result node, the candidate verifies completion and readback on the original
branch. A result node present in readback must match the original receipt.
The Page Driver fixture covers both persisted and omitted result nodes.
The follow-up proof build SHA-256 is
`546a2e5c66ecc64685de2ff00f1da850a3e13551e4b527dc1d94667bdf34549e`.

This build passed the direct authenticated round trip. The first Responses
request emitted one declared `function_call`. The second submitted one
`function_call_output` for the same ID. The AppHost completion boundary
verified the structured tool envelope and result ID, and the second Responses
request completed with the unpredictable marker generated after the original
call. The persisted AppHost result ID, settled remote head and absence of
pending calls were independently checked against the original proof receipt.
The remote chat contained one user message and one answer matching the
marker digest. See `native-route-settlement-proof/native-proof.json` and
`logs/c-route-observed-submit-*.log`.

Provider diff SHA-256
`b1bab8a80ee1a7da3f036936e715ded82e185f2713cdc5cc4cf4eebb2c82780a`.
Page Driver passed 20/20, Rust passed 29/29 under each umask, and fmt, strict
clippy and doc links passed. The proof handoff restored the selected operating
generation. This build has not replaced that local generation. The official
Codex developer path, batched returns, and the rest of P0/P7 remain open.

## Codex CLI developer context and native tool execution

Codex CLI 0.171.0's first request contains a `developer` message carrying
skill instructions and permission context, followed by user-role
environment context and the user prompt. The root request instructions
field was empty. The candidate preserves message roles and order.

The first integrated CLI attempt reached AppHost and then failed Provider
settlement before a command could execute. Read-only inspection of the
original chat found both submitted user-role messages by their original IDs
and a finished `local.exec_command` at the remote head. AppHost had omitted
the developer input from the graph. The candidate now handles this omission
while checking the role, content and original branch of any retained copy.

The second integrated check used a new disposable CLI home and work
directory. Only the developer message supplies the exact command to run.
The command creates an unpredictable one-use marker. Acceptance requires
one completed Codex command, a structured return and a matching final answer.

The second check passed on actual Codex CLI 0.171.0 through the Wrapper,
candidate Provider and authenticated AppHost. Codex issued one completed
`exec_command`, then sent `function_call_output` in its second request.
The CLI exited 0 and the final answer matched the marker from the actual
command. Independent readback confirmed the committed remote head, one
settled AppHost result, and the same two user-role entries originally sent
by Codex. The candidate proof ended with no pending tool calls.

Provider diff SHA-256
`d5f55a63b0776930c19661643fbd670856f81aba5ed782496c6e2bcd81a7abe2`.
Exact Android candidate SHA-256
`b6e3a5db9579187cf953ea358d2f30c10788690d3c8f0cdc26889f97c2244242`.
See `codex-role-proof/codex-proof.json` and `logs/codex-role-*.log`.
Page Driver passed 21/21 and locked Rust tests passed 29/29 on both umasks.
The proof restored the operating generation at the end of that run.
Durable developer scope and batch admission remain open.

## E01 proof binary activated on port 4319 (2026-11-25)

The user authorized local activation of the exact E01 binary. The runit
service switched from `0.1.0-local-native-0a1dfcb6` (PID 14147) to local
generation `0.1.2` (PID 18649). The new running executable and installed
payload matched the E01 SHA-256 above, and `/healthz` passed. The package
continues to report version `0.1.0`. The generation number identifies the
local installation of the existing binary.

AppHost was idle before the stop. The configuration, control token and
continuation maps matched their pre-activation snapshot at startup. The
Native map had no persisted entries before activation. `previous` now
points to `0.1.0-local-native-0a1dfcb6`; the former rollback executable
`0.1.0-c9d0e245` remains installed. The earlier rollback helper is scoped
to the preceding activation. The current activation receipt is
`codex-role-4319-hsna631q/activation.json` under the owned scratch root.

An isolated official Codex CLI `0.171.0` home reached the activated endpoint
and returned the exact marker supplied only in its developer message.
The CLI exited 0 with zero local commands and no recorded error codes.
AppHost was authenticated, completion-ready and idle after the check.
The Provider process identity was unchanged throughout the smoke check.
See `codex-role-4319-hsna631q/smoke.json`.

## Local installation version fixed at 0.1.0 (2026-11-25)

The user directed that the Provider package and local installation stay at
`0.1.0` until they explicitly start versioning. The temporary installation
name was retired. The same E01 binary now runs from
`generations/0.1.0/chatgpt-provider` on port 4319 at PID 17215. The
configuration, control token and continuation maps matched across the
restart. The historical binary formerly at `0.1.0` was retained, and
`previous` still points to `0.1.0-local-native-0a1dfcb6`.
See `codex-role-4319-hsna631q/fixed-install-version.json` for the retained
path and verified running identity.

## Follow-up review fixes and E02 attempt (2026-11-25)

The source candidate restores the original binding only when the Page Driver
explicitly confirms that it rejected the request before dispatch. An
ambiguous outcome retains its intent. The model catalog and Responses
metadata advertise serial tool calls in structured mode. A verified turn
with multiple calls fails before Codex receives executable tool frames.
The existing encoder streams committed public output up to the first
pending call. It releases the call and subsequent frames only after
settlement is verified and saved.

The E02 check used official Codex CLI `0.171.0`, a disposable Codex home
and work directory, a fresh chat, and the installed E01 executable on port
4319. Each of three planned commands required the preceding value as an
argument. The first tool executed exactly once and exited 0. The first
return then failed Page Driver settlement with
`native_reconciliation_required`. Codex exited 1. The original chat readback
showed an advanced head, the two original Codex user-role inputs, and no
visible head message. The active Native map still holds one original
pending call and an unresolved return intent. Result consumption and the
new head's identity require further evidence.

Receipts are under `$PREFIX/tmp/provider-e02.PH0Uxeq2` in `receipt.json`
and `readback.json`. Keep the original thread, chat, tool result and intent
for reconciliation. No replay, service rebind, or candidate activation
took place. Continue E03 capability qualification after reconciling E02.
Schema migration, retention policy and release qualification remain open.

Candidate source diff SHA-256
`d2d493032814893090d81ec725d5112cfcc831cf0515b8d451bf8992ec1ae700`.
Android AArch64 release binary SHA-256
`cc2e45166565403b0d18d4b3f7d5286645e9bade8ed520081d7af2dec74a0ee6`.
Page Driver passed 23/23, including a three-round mock with retained and
omitted AppHost result nodes, and live commentary before mock result
settlement. Rust passed 31 tests under each umask. The normally ignored
official Codex CLI test passed separately against MockDriver. Formatting,
strict Clippy, AppHost lifecycle and document links passed. The executable passed the 16 KiB ELF check
and booted on an isolated loopback port. The boot probe found all 28
existing catalog entries marked serial with their other fields retained.
An unsupported request received HTTP 400 before AppHost startup or intent
creation. See `final-boot-receipt.json` in the same scratch root.
