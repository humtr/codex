# Roadmap: Stage 2 Decoupled Architecture and Follow-Up Verification

> **Status:** **Remaining Stage 2 independent component plan, not the active Wrapper feature list.**
> Follow [documentation authority](INDEX.md) and the
> [current implementation/owner map](ARCHITECTURE.md).
> Normative contracts: [SPEC.md](../SPEC.md), [DELIVERY.md](spec/DELIVERY.md),
> [USER_INTERFACES.md](spec/USER_INTERFACES.md), [BRIDGE.md](spec/BRIDGE.md),
> [ORCHESTRATION.md](spec/ORCHESTRATION.md).
> Stage 1 and Stage 2 Slices 1–7 are implemented. See [CURRENT.md](../CURRENT.md)
> for live state, [Stage 2 milestones](history/STAGE2_MILESTONES.md) for
> release and closeout receipts, and [§8](#8-remaining-operational-and-physical-verification)
> for the remaining operational and physical verification.

---

## 1. Ground Rules & Non-Negotiable Constraints

1. **Zero-coupling Stage 1 Wrapper (`$PREFIX/bin/codex`):**
   - `codex` stays strictly an `execve()` Termux runtime wrapper + official upstream updater/rollback/daemon-sync + read-only `doctor` + `codex wrapper status`.
   - Root `Cargo.lock` locks only `codex`, `codex-release-builder`, and `libc` (`=0.2.186`). Stage 2 components live in their own independent `[workspace]` manifests and CI path-triggered workflows.
   - Unused Phase 2.1 Wrapper-embedded addon hooks (`system_config.rs` FD 34 snapshots, `wrapper.toml` `manager`/`tui`/`provider`, `--with`/`--bare`) are retired in Slice 1.
2. **Zero upstream source patching:**
   - Do **not** port `native_route.patch` (~14.5K lines) or `frontend/tui/native.patch` (~4.7K lines). Follow official upstream `bin/codex` and `app-server daemon` via binary ELF adaptation only.
   - External providers (`ais.config.toml`, `chatgpt.config.toml`) do not use OpenAI OAuth `auth.json` and run against canonical `~/.codex` with `-p <profile>` (starting a new thread on provider switch, matching upstream `0.171.0` `threads.model_provider`).
   - OpenAI OAuth multi-account profiles (`CODEX_HOME` + `CODEX_SHARED_HOME`) handle same-thread switching through standalone `codex-ext` (`cdx`).
3. **No second executor or DDPE coupling:**
   - Drop `shared_server`, FD 36 `execution_lease`, `codex-runtime-binding`, and `0.167.2` DDPE multi-worker orchestration from Stage 2. Official `codex app-server daemon` owns Codex tasks; ChatGPT owns ChatGPT sessions; MCP bridges stay thin and stateless (only bounded `call_id` transport deduplication receipts).
4. **Live Termux Device & Service Preservation:**
   - Never overwrite `$PREFIX/bin/codex`, disrupt unrelated services (`tdev*`, `sshd`), or mutate user profiles/credentials during development or tests. Legacy `0.167.2` `codex-chatgpt-*` services were retired and archived under `~/.local/state/codex/legacy-0.167.2-archive/` on 2026-11-22 (see [CURRENT.md](../CURRENT.md)); all Stage 2 components operate under the unified XDG `codex/` namespace.

---

## 2. Stage 2 Component Architecture & Slice Execution Order (Slices 1–7)

```text
Slice 1: Wrapper slimming + universal SHA256SUMS.sig Ed25519 release signing  [COMPLETE & LIVE]
  │
  ├──▶ Slice 2: codex-ext (cdx) + [shared] crates/codex-daemon-client         [COMPLETE & LIVE]
  │
  └──▶ Slice 3: [shared] crates/mcp-kit + @TermuxShell + tunnel-client + runit [COMPLETE & LIVE]
         │
         ├──▶ Slice 4: @TermuxCodex (5 tools greenfield)                      [COMPLETE & LIVE]
         │
         └──▶ [Gate 5-0] → Slice 5: host/chatgpt-desktop + chatgpt-provider   [COMPLETE & LIVE]
                             (multi-lane parallel + /control/connectors*)
                             │
                             └──▶ Slice 6: @TermuxChatGPT (6 tools)           [COMPLETE & LIVE]
                                    │
                                    └──▶ Slice 7 (7A & 7B): mcp/termux-orch   [IMPLEMENTED]
                                           (@TermuxOrch + termux-orch-relay:
                                            F·L·W·V·R, Tier 0–3 Work Ledger +
                                            Typed Inbox + Wait-Set + inject_and_notify)
```

### Slice 1 — Wrapper Slimming & Universal `SHA256SUMS.sig` Signed Releases *(Complete — 2026-11-22)*
- **Goal:** Permanently close `crates/wrapper` by retiring unused Phase 2.1 hooks (`system_config.rs`, `wrapper.toml` `manager`/`tui`/`provider`, `--with`/`--bare`) and introducing one lightweight Ed25519 `SHA256SUMS.sig` signing/verification format (with component-scoped monotonic `sequence` anti-rollback and pinned public-key trust) across `publish_wrapper_release.py`, `install.sh`, and future Stage 2 component releases.
- **Completed:** Merged to `main` (`3a87150e..3ea207cb`), published signed `wrapper` (`sequence=1`, [run 44997871190](https://github.com/humtr/codex/actions/runs/44997871190)), and verified live Termux `--require-signed --skip-upstream` cutover (see [CURRENT.md](../CURRENT.md)).

### Slice 2 — `codex-ext` (`cdx`) + `crates/codex-daemon-client` + §7 / OSC 52 Proof *(Complete — 2026-11-22)*
- **Goal:** Build standalone `crates/codex-ext` (`$PREFIX/bin/codex-ext` + `$PREFIX/bin/cdx` symlink) and shared `crates/codex-daemon-client` (~250 lines, public `app-server daemon` UDS JSON-RPC client):
  - `profile` CRUD (`~/.local/share/codex/manager/profiles/` compatible) & safe `CODEX_SHARED_HOME` launch;
  - `notify` (`Stop` via `~/.codex/config.toml` `notify = [...]`, detailed events via `~/.codex/hooks.json` + `CODEX_SHARED_EXTRA_FILES=hooks.json`, Termux:API delivery);
  - non-PTY parent launcher (`termios` save/restore, zero PTY proxy, 100% direct Termux TTY/OSC 52) supporting in-TUI `!cdx switch <acct>` and `!cdx provider <name>` (matching daemon-spawned `!` commands via `($CODEX_HOME, cwd)` `0600` session registry, pre-checking idle daemon before closing TUI, stopping idle Daemon A while preserving `thread_unload_delay_secs` upstream default).
- **Verified (isolated Android API 37 Termux against official `0.171.0`):** `codex-daemon-client` RFC 6455 WebSocket-over-UDS (`/proc/self/fd/<dirfd>/...` FD 35 resolution), multi-window disambiguation, busy-daemon pre-flight protection, `!cdx switch`/`!cdx provider` `SIGUSR1` handoff, `notify` Option A, §7 writer-lock `flock` release + same-UUID resume context preservation + `session_index.jsonl`/`state_5.sqlite` authority check, and retirement of `contrib/codex-account`. Physical interactive Termux `/copy` clipboard paste-back remains a live-UI observation boundary (upstream `0.171.0` binary confirmed built-in `/dev/tty` OSC 52 `]52;c;` with internal size guard; `codex-ext` interposes zero PTY proxy).

### Slice 3 — `crates/mcp-kit` + `@TermuxShell` + `tunnel-client` + `runit` 3-State Supervision *(Complete — 2026-11-22)*
- **Goal:** Establish the shared MCP foundation (`crates/mcp-kit`: stdio JSON-RPC, uniform immutable annotations `readOnlyHint=true`/`destructiveHint=false`/`openWorldHint=false` + accurate `idempotentHint`, bounded `call_id` transport receipts, disjoint `catalog`/`call` helper), extract independent `mcp/termux-shell` (`@TermuxShell`, 1 tool `termux_shell`, `docs/spec/MCP_SHELL.md`), update `transport/tunnel-client` (`v0.0.21` with `5c495538` HTTP/2 half-open PING patch, GitHub API 403 fallback, prebuilt signed CI), and define opt-in `runit` supervision using stable `current` symlinks with 3-state visibility (`selected`, `running`, `pending_activation`) in `scripts/runit_service.py` + `docs/spec/SUPERVISION.md`.
- **Verified (isolated Android API 37 Termux):** `crates/mcp-kit` & `mcp/termux-shell` (`fmt`/`clippy`/`test` under `umask 0022`/`0077`), native Android/arm64 `tunnel-client` `v0.0.21` Go build & HTTP/2 regression tests, `test_build_tunnel_client.py` (403 fallback), and `test_runit_service.py` (opt-in enrollment, 3-state reporting, busy deferral, failed-readiness auto-rollback).

### Slice 4 — `@TermuxCodex` (`mcp/termux-codex`, 5 Tools Greenfield) *(Complete & Live — 2026-11-22 / Enrolled 2026-11-23)*
- **Goal:** Build stateless thin MCP bridge over `crates/codex-daemon-client` + `crates/mcp-kit`: static core (`codex_status`, `codex_threads` read-only, `codex_turn` `start|steer|interrupt` with 1-shot new-thread start and `call_id` transport receipt) + disjoint extensions (`codex_catalog`, `codex_call`). Zero shadow task DB; independently retirable when upstream `remote-control` matures.
- **Verified (Android API 37 Termux against official `0.171.0`):** `mcp/termux-codex` (`fmt`/`clippy`/`test` under `umask 0022`/`0077`, live `codex 0.171.0` `app-server daemon` empirical E2E, post-audit hardening `ed3d9daf`, canonical install, `termux-codex-tunnel` `runit` `active`, and live remote `@TermuxCodex` ChatGPT connector E2E).

### Slice 5 — `Gate 5-0` + `host/chatgpt-desktop` + `providers/chatgpt` (`page_driver.js` Clean-Room) *(Complete & Live — 2026-11-23)*
- **Goal:** Pass `Gate 5-0` (install latest official ARM64 `chatgpt` `.deb` from `persistent.oaistatic.com/codex-app-prod/linux/deb` into `~/.local/lib/codex/host/chatgpt-desktop/generations/<ver>/rootfs`; test isolated PRoot/Xvnc + CDP), build `host/chatgpt-desktop` generation lifecycle (`current`/`previous` + separated `~/.local/share/codex/chatgpt-desktop/profile`) and slim `providers/chatgpt` (`chatgpt-provider` owning single CDP + clean-room greenfield `page_driver.js` over Electron `connect-app-host` `httpFetch` + dynamically discovered `CompletionClient` + native `local_function_signatures` + multi-lane parallel execution up to 8 lanes + `/v1/responses` + local authenticated `/control/*` including programmatic `/control/connectors*`; retire DDPE/MCP/`shared_server` and redundant `codex-chatgpt-host` keepalive).
- **Verified (Android API 37 Termux against official `chatgpt 26.1117.41037` + `codex 0.171.0`):** `Gate 5-0` isolated CDP probe, 9-case combination E2E, live `gpt-6-sol-wm` (`xhigh`) Project creation + Codex subagent orchestration E2E, post-audit hardening (`ed3d9daf`), programmatic zero-DOM `/control/connectors*` (`64667a35`), multi-lane parallel & 3-lane re-entrant E2E (`336b9c1c`), and `runit` `active` deployment (`0.1.0+multilane`).

### Slice 6 — `@TermuxChatGPT` (`mcp/termux-chatgpt`, 6 Tools Greenfield) & 3-MCP Live Cutover *(Complete & Live — 2026-11-23)*
- **Goal:** Build stateless thin MCP bridge over `chatgpt-provider` `/control/*` + `crates/mcp-kit`: static core (`chatgpt_status`, `chatgpt_chats` read-only, `chatgpt_projects` read-only, `chatgpt_send` `send|observe|stop` with `call_id` receipt and per-`chat_id` multi-lane stop) + disjoint extensions (`chatgpt_catalog`, `chatgpt_call`). Zero local code execution privilege. Slim `docs/spec/BRIDGE.md` around the finalized 3-MCP + `chatgpt-provider` architecture.
- **Verified (Android API 37 Termux against live `runit` `chatgpt-provider` + `chatgpt 26.1117.41037`):** `mcp/termux-chatgpt` (`fmt`/`clippy -D warnings`/`test` under `umask 0022`/`0077`), live `runit` `termux-chatgpt-tunnel` (`0.1.0+multilane`), programmatic registration & schema verification of all 3 remote ChatGPT connectors (`@TermuxShell`, `@TermuxCodex`, `@TermuxChatGPT`), live multi-worker iterative loop E2E (`6b03c20b-...`), and 3-lane parallel re-entrant ChatGPT worker E2E (`6b03c678-...`).

### Slice 7 — Durable Multi-Role Orchestration (`@TermuxOrch` / `mcp/termux-orch`, 7A & 7B) *(7A and 7B implemented)*
- The contract and completed rollout are recorded in [ORCHESTRATION.md](spec/ORCHESTRATION.md)
  and [Stage 2 milestones](history/STAGE2_MILESTONES.md#retired-stage-2-work-order).
- **Goal:** Add an independent, zero-Wrapper-coupling durable orchestration layer (`F · L · W · V · R` roles; Tier 0–3 progressive durability covering both simple single-task chats and multi-hour multi-worker loops; reusable repository or branch ChatGPT Projects and configurable `[F1]`/`[L1]`/`[W1-<slug>]`/`[V1-<slug>]` titles; `Work Ledger` + `Typed Inbox` + declarative `Wait-Set`; seamless `F1 -> Fn` mobile rebinding; default `inject_and_notify` + `termux-notification` with `stream_status` collision protection and hard Circuit Breakers).
  - **Slice 7A *(Shipped & Live)*:** `chatgpt-provider` server `stream_status` in `observeChat` + `mcp/termux-orch` (`termux-orch relay` & `termux-orch mcp`, 6-tool `@TermuxOrch`, Tier 1–3, `Verifier` role, `F1 -> Fn` rebind, `inject_and_notify`, Circuit Breaker, CI, `runit` enrollment, programmatic `@TermuxOrch` connector registration, live Tier 1/2/3 & restart-recovery E2E — receipts in [CURRENT.md](../CURRENT.md)).
  - **Slice 7B:** Automatic and manual ChatGPT Lead rollover, generation fencing, Inbox replay, detached Shell worker receipts, and the optional Stop hook wake push. See [the behavior contract](spec/ORCHESTRATION.md#7-lead-rollover-detached-shell-receipts-and-wake-pushes) and [live evidence](../CURRENT.md).

---

## 3. Phase 2.6 — Read-only Upstream Discovery and Qualification

**User request, 2026-11-16:** Treat recurring official-upstream adoption as a
Stage 2 delivery work item. It must **extend** the single Stage 1
`codex update --upstream` / release-builder owner, not create another CLI,
version selector, updater, daemon, account dependency, or signature authority.
The ordinary standalone update and rollback remain usable independently of this
automation and of every optional module.

**Implemented canary path:** `codex update --check --json` owns official latest/exact
descriptor comparison and immutable archive metadata. It creates no installed
state. Shared `wrapper-verify.yml` owns the existing check/package/Bionic jobs
and explicit account-free qualification with `qualify_wrapper.py`; the ordinary
`wrapper.yml` caller retains publication separately. The on-demand canary uses
`qualify_upstream=true` with `publish_release=false` on `main`. The reusable
verification workflow runs with `contents: read` and retains the input and
qualification reports as CI artifacts. Local `codex update --upstream`
resolves OpenAI releases directly. [Canary invocation](../README.md#wrapper-ci-githubworkflowswrapperyml)

The canary runs the existing adaptation implementation; it does not generate a
new implementation when upstream changes. A compatible new archive needs no
Wrapper revision. An unsupported adaptation requires diagnosis and, when the
rules are insufficient, a qualified Wrapper fix. A failed host-tool, configuration
or old-server check is not by itself evidence that the patch rules must change.
Ordinary local update does not wait for a CI-approved runtime or a new Wrapper
release for each upstream version.

The available Termux environment and existing login were exercised once for this
work. They are not a required recurring phone/account matrix. Optional additional
Android/page-size or external-client evidence is reported by observed boundary;
16-KiB ELF geometry is not actual 16-KiB-kernel execution.

**Publication scope:** Lightweight Ed25519 `SHA256SUMS.sig` signed distribution
is implemented and live for the standalone Wrapper (`sequence=1`, see [CURRENT.md](../CURRENT.md)),
and shared across Stage 2 component workflows via `.github/scripts/publish_stage2_release.py`.
Scheduled/automatic upstream runtime publication or automatic device deployment
is **not** enabled: publication remains an explicit `workflow_dispatch` action,
and base upstream **qualification and any separately authorized Stage 1 publication
must never wait for optional Stage 2 components**. Scheduling is not permission
to publish or activate. Physical-device checks and authenticated/remote
integration are different evidence gates, not retroactive failures of the
already accepted standalone release.

### Implementation work order

1. **Discover (read-only):** manual-dispatch (or optional low-frequency schedule)
   official stable release compatibility check via the supported OpenAI release metadata.
   Resolve an exact version, metadata/descriptor digest, archive URL, archive
   SHA-256, source SHA, wrapper/release-builder SHA, toolchain/NDK and binary
   patch policy. Reject prereleases, missing official digests, divergent
   descriptors, unknown schema, downgrades and changed inputs; deduplicate
   on the complete immutable input tuple. Never infer a target from a CI job
   title, cached tag, mutable `latest` alias or a previous failed run.
2. **Prepare (least privilege):** fetch and digest-check the *complete official*
   `codex-package-aarch64-unknown-linux-musl.tar.gz` including companions and
   resources; invoke the existing rule-based `termux-fd-remap` /
   `termux-uds-fd35` builder, rejecting unsupported site layout with
   actionable evidence. Use pinned toolchains and prepared exact commit; no
   automatic source patching, blind ELF offset heuristics, or fallback to an
   unadapted official binary.
3. **Qualify (read-only):** test the standalone base lane before any optional
   composition: unit/negative tests, Android AArch64 packaging, Ubuntu ARM+Bionic
   smoke, installer rollback faults, real Android/Termux isolated HOME/PREFIX/
   CODEX_HOME/TMPDIR execution with the new official archive, doctor, fast
   help/version, app-server/remote-control boundaries, upgrade and rollback.
   Fail closed when the physical-device gate is required but unavailable.
   Additional TUI/Manager/Provider lanes qualify their own exact artifacts;
   their failure is not a veto of a valid independently usable base.
4. **Publish/promote (isolated authority):** separate unprivileged discovery
   and validation from the signer/publisher. Give PR and scheduler jobs
   `contents: read`; grant content-write and signing keys only to an
   environment-gated immutable-input promotion job, not to pull requests,
   arbitrary uploaded fixtures or self-hosted device smoke. Verify published
   archive bytes, manifest/index signatures (when enabled), SHA-256 and
   canonical download URLs by readback. Use distinct base/full qualification
   provenance and never mistake a skipped release step for a published product.
5. **Update/recovery:** leave the installed generation, sessions, original
   service identities, auth homes and previous/rollback pointers untouched by
   CI. Activation is a separate controlled device operation. On discovery,
   patch, qualification, download or publication failure retain the last
   admitted version, report the exact failed input/stage and retry only with a
   reconciled original receipt. Reuse unchanged successful expensive proofs
   only when source/input/toolchain/test contract are identical; never reuse a
   failed or incomplete gate.

**Exit criteria:** exercise no-new-release, same-input duplicate schedule,
changed official digest, malformed descriptor, absent companion, unexpected
ELF relocation, failed download, interrupted installation, stale proof,
signed-index verification failure, optional-module unavailable, device offline,
failed readback and successful newer release. The workflow must publish a
traceable accepted tuple, and a failed candidate must not change an active
device or existing release pointer. Record exact CI run IDs and physical
device receipts in `CURRENT.md`. No scheduled publication is currently
approved or running. Read-only discovery/qualification is separate and does not
write a release, signature, accepted-input pointer or live device.

---

## 5. CI Verification Environment — Reviewed Options (2026-11-16)

The workspace checks now run on `ubuntu-24.04-arm` so the emitted AArch64
`realpath` machine code is executed against real libc in a disposable child,
in addition to structural ELF tests. Native Termux runs exercise that same test
against Android libc. This does not establish musl/Android lifecycle acceptance
for a new official archive; the restricted physical-device gate below still does.

| Environment | What it truly proves | Gap / cost | Decision |
| --- | --- | --- | --- |
| Current `ubuntu-24.04-arm` with pinned Android `linker64` + Bionic libs | Native AArch64 Bionic ELF loading, installer/bootstrap boundaries, isolated file and config behavior, deterministic faults | Linux kernel and synthetic Termux `/data/data` layout; not Android kernel, SELinux, Termux app lifecycle or genuine UI | **Retain as fast, mandatory PR gate** |
| Android emulator on x86 GitHub runners (e.g. AVD with hardware acceleration) | Real Android OS, package installation, selected app and UI paths | AArch64 upstream binaries require a matching guest architecture; default fast x86_64 emulator is not an AArch64 runtime, ARM translation support is not guaranteed. Higher setup/time cost; must prove Termux APK + shell route before reliance | Optional exploratory lane, never sole ARM64 admission |
| Managed physical-device testing service (e.g. Firebase Test Lab) | Android/SELinux/real device signals, UI and APK instrumentation | Expects app APK + instrumentation/Robo harness; not a drop-in generic shell or Termux CLI runner. Service quotas, credentials and packaging work | Feasibility prototype only |
| Dedicated physical AArch64 Android device with isolated Termux test installation via controlled ADB/relay | Actual Termux UID, kernel/SELinux/path constraints, PTY/TUI, daemon/remote-control, real official archive upgrades/rollback | Hardware upkeep and trusted runner isolation; ordinary public-PR code cannot be executed on a phone carrying personal accounts | **Preferred release-admission tier**; restricted exact-SHA artifacts and explicitly authorized jobs |
| Existing personal handset as a manual acceptance target | Highest fidelity to currently deployed environment | Auth, live services, account ownership and state risk; uptime cannot be assumed | Manual acceptance only, never unattended PR runner |

**Actual device Issue #33 diagnosis & resolution (2026-11-16):** Earlier isolated checks on `a224f223`/`f8284261` failed `app-server daemon start`, `app-server --listen`, and default `codex` PTY with `SUN_LEN`. Root-cause inspection traced this to static musl `realpath` dereferencing `/proc/self/fd/35` and `std::fs::canonicalize` on client control sockets back to full physical paths, compounded by companion ELFs lacking `0x170`-byte `.fini`–`.rodata` alignment caves. After adding adaptive `realpath` interception and multi-gap / trailing-`PT_LOAD` cave allocation in `crates/release-builder/src/runtime_patch.rs`, isolated physical API 37 Termux verification against `0.169.0` (`118`B `TMPDIR`, `130`B `CODEX_HOME`, `173`B `socketPath`, with Termux `Essential` `procps` `ps`) passed `daemon start`/`version`/`stop`, `--listen unix://`, `codex-exec-server --version`, and default `codex` PTY (`doctor --json` confirmed `app_server.status` ok). The physical promotion gate therefore **continues to require default PTY and daemon lifecycle tests**, never treating `--no-daemon` fallback, help/status, or Linux+Bionic smoke alone as sufficient.

**Recommended topology:** keep the present 3 GitHub jobs
(`checks` + `android-package` + `android-smoke`) for every PR.
Add a **restricted, dedicated Android/Termux physical-device gate** for
pre-release / promotion: pull only checksum-pinned artifacts from a passed
exact-source build into owned isolated directories, run deterministic smoke,
record OS API, architecture, candidate SHA, archive digest, exit codes,
timeout, stderr, and rollback equality. No workflow secrets or ChatGPT
accounts on the test device; no arbitrary fork PR code, no implicit activation
of the user's installed `codex`, and no dependence on device availability
for ordinary source PR CI. If a physical device is unavailable, the promotion
gate remains **unverified**, not automatically green.

Investigate emulator/device-farm options only against the same acceptance
matrix; do not add an expensive duplicate CI job merely to report another
`aarch64` cross-compile. The existing Ubuntu/Bionic test is an excellent
ABI compatibility layer, not the real-device operating acceptance by itself.

---

## 6. Pre-Stage-2 Checklist (Already Completed on `rewrite/wrapper`)

The 2026-11-17 P0 installer/recovery, public `wrapper` prerelease source and
asset readback, isolated actual Android qualification and authorized device
Wrapper cutover are recorded in [CURRENT](../CURRENT.md). This closes
standalone publication/operation for that qualified source, **not** the
post-cutover authenticated inference, external remote client or physical
power/16-KiB-kernel/other-device boundaries. (Unused Phase 2.1 Wrapper-embedded
FD 34 addon hooks were retired in Slice 1.)

- Unified naming (`wrapper`, `codex-wrapper`, `codex-local-generation`, `codex-activation-state`, `package-root`, `termux-fd-remap`, `termux-uds-fd35`) across code, tests, [`install.sh`](../install.sh), [`.github/workflows/wrapper.yml`](../.github/workflows/wrapper.yml), and documentation.
- Removed legacy `99b` / `R10` / `helpers/{0,1}` / `RootCodeModeHost` / `upstream_doctor` / version-pinned tests.
- Passed local checks (`ruby` Psych YAML, `sh -n` / `shellcheck`, `cargo fmt`, `clippy`, `cargo test --workspace --locked`, isolated AArch64 mock-ELF E2E) and GitHub Actions `Wrapper CI` (`checks`, `android-package`, `android-smoke`).

---

## 7. Follow-Up Verification: Session Handoff and `session_index.jsonl`

격리된 공식 `0.171.0` 환경에서 Slice 2 실증을 마쳤고, 2026-11-23에 두 실계정에서
동일 UUID 온라인 재개와 네 경로의 이름 변경·삭제 동작을 확인했습니다. 이때
`app-server daemon` RPC를 사용했습니다. 아래 기준은 업스트림 버전 업그레이드
회귀와 추가 동시 작업 검증에 적용합니다.
[실계정 검증 기록](history/STAGE2_MILESTONES.md#r-2-authenticated-uuid-handoff-and-session-lookup)
Stage 1의 FD 34는 스레드 수명을 업스트림 정책에 맡깁니다. 마지막 클라이언트가
연결을 닫은 뒤에도 실제 writer 해제와 업스트림 재개 조건을 확인한 다음 인계합니다.

### 7.1 동일 UUID 세션 인계(`/switch`) 검증 기준
- **합격 조건:**
  1. 계정 A가 스레드 X의 턴을 완료하고, 단순 `thread/unsubscribe` ACK가 아니라 **`thread-writer-locks/`의 실제 writer 해제가 완료된 뒤** 계정 B가 동일 UUID X를 재개합니다.
  2. B가 보낸 다음 추론 요청에 A의 직전 응답 문맥이 포함되고 B의 턴이 완료됩니다.
  3. 같은 시간 동안 A의 다른 스레드 Y 작업이 중단 없이 지속·완료됩니다.
- **실계정 관측:** `janmori101` → `szob` RPC 인계에서 조건 1–3을 충족했습니다.
  설치된 `cdx switch`는 A가 유휴일 때 동일 UUID를 이어받아 새 턴을 완료했습니다.
  별도 A 턴이 활성인 경우에는 종료 코드 1을 반환했습니다.
  런처에서 조건 3을 충족하는 작업이 남았습니다. 현재 런처는 A의 데몬 전체를
  정지해 writer를 해제합니다. 업스트림 `thread/unsubscribe`는 연결의 구독만
  제거하며, 마지막 구독 종료와 스레드 유휴 시간이 충족된 뒤 writer가 해제됩니다.
  [0.171.0 스레드 수명주기](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/app-server/src/request_processors/thread_lifecycle.rs)
- **계정 일치 검사 정책:** `0.170.0`의 `cli_resume_account_check` 등 업스트림 계정 일치 검사가 켜진 환경에서 타 계정 재개가 거절되면, **계정 태그나 정책을 변조해 억지로 통과시키지 말고** 해당 버전/설정의 호환성 결과로 그대로 기록합니다.

### 7.2 `session_index.jsonl` 권위 원천 및 삭제/이름변경 검증안
- **분리 측정 항목:** A/B 동시 기동 상태에서 이름 변경(`rename`) 및 삭제(`delete`) 수행 후 아래 4개 경로의 **UUID 조회·신규 이름 조회·구 이름(alias) 조회**를 각각 측정하고 실제 권위 원천(`state_5.sqlite` vs `session_index.jsonl`)을 기록합니다:
  1. `app-server` `thread/list`
  2. `app-server` `thread/read`
  3. TUI 목록 및 `/resume`
  4. `codex exec resume <NAME|UUID>`
- **제약:** 각 조회의 권위 원천과 동시 쓰기 정합성이 입증되기 전에는 `state_5.sqlite`나 파일 타임스탬프를 역산해 `session_index.jsonl`을 임의 재생성하는 임시 우회를 적용하지 않습니다.
- **실계정 관측** `thread/list`와 TUI는 새 이름을 표시했습니다. `thread/read`는
  UUID만 받았습니다. `codex exec resume`은 기존 UUID와 새 이름으로 이어졌고,
  구 이름 조회 실패 시 새 스레드를 시작했습니다. 삭제 후 네 경로에서 기존 UUID가
  사라졌고 공유 인덱스는 기존 175개 행과 바이트 단위로 일치했습니다. 두 daemon이
  실행 중인 상태에서 변경을 순차 수행했습니다.
  [조회별 결과](history/STAGE2_MILESTONES.md#r-2-authenticated-uuid-handoff-and-session-lookup)

## 8. Remaining operational and physical verification

- **R-3** Termux `/copy` was accepted. Pairing an external
  `codex remote-control` client is a long-term check.
- **R-4** The default-home daemon cache was reclaimed. The retired Provider
  archive is retained for its protected account and session data.
  [Reclamation evidence](history/STAGE2_MILESTONES.md#r-4-legacy-disk-reclamation)
- **R-5** The manual read-only canary and Termux qualification passed for
  `0.171.0`. The physical-device promotion gate remains deferred.
  [Phase 2.6](#3-phase-26--read-only-upstream-discovery-and-qualification)
- **R-6** Installer recovery after an Android Termux force-stop passed.
  Physical power removal and execution on a device with a 16 KiB kernel page
  size are long-term checks.
  [Force-stop recovery evidence](../CURRENT.md#2026-11-23--android-termux-force-stop-recovery)
- **AppHost shutdown.** Cross-device Android behavior remains open.
  [SHM helper follow-up](history/APPHOST_SHUTDOWN_AUDIT_20261123.md#6-sysv-shm-후속-검증)
- **Legacy Work upgrades.** The 7A to 7B fixture preserved an unconfirmed
  delivery without repeating the owner POST. An actual pending item needs
  owner evidence before its outcome can be confirmed.
  [Upgrade evidence](../CURRENT.md#2026-11-24--concurrent-account-handoff-context-rejection-and-upgrade-checks)
- **Slice 7B** Bounded size-error recovery has a compiled loopback fixture.
  Reproduce the observed `input_too_large` error and recovery with a disposable
  authenticated Lead before promoting the candidate.
  [Context evidence](../CURRENT.md#2026-11-24--concurrent-account-handoff-context-rejection-and-upgrade-checks)

## 9. Slice 7 역할 배치와 실행 환경 재설계

사용자는 원하는 결과를 자연어로 요청한다. F·L·W·V는 아래의 사용자
결정을 반영했다. R의 책임과 컴팩션 시 실행 환경별 전환 방식을 아래 계약에 반영한다.
현행 동작은 [Slice 7 계약](spec/ORCHESTRATION.md)에 있다.

| 역할 | 동작 |
| --- | --- |
| F | 요청을 받은 대화가 사용자 창구다. 사용자가 권한이 확인된 새 대화에서 특정 Work를 조회하면 그 대화가 이후 알림 대상이 된다. 전체 목록·내부 조회는 이전을 일으키지 않는다. F 이전은 L을 깨우지 않는다. 이미 발송 여부가 불확실한 알림은 원래 대상에서 확인하며 새 대화로 재발송하지 않는다. |
| L | 사용자 요청을 여러 단계로 이행해야 하면 L이 자동으로 발동한다. 다음 단계가 정해져 있어도 순서·의존성·실패 처리를 조율한다. 현재 세션이 계속 맡을 수 있으면 F가 겸한다. 별도 세션이 필요한 경우 해당 단계의 판단 근거와 권한을 가진 실행 환경을 선택한다. |
| W | L이 독립 범위를 병렬로 배정한다. 같은 파일을 두 워커가 수정해야 하면 L이 순서를 정하고, 앞 워커의 결과를 확인한 뒤 그 상태를 다음 워커에 넘긴다. 기존 변경과 진행 중인 실행을 보존한다. |
| V | 리뷰가 필요한 작업은 자동으로 V를 가동한다. 테스트나 산출물 검사는 실행 증거로 판단하고, 독립적인 판단이 필요하면 리뷰 세션에 동일한 변경본과 완료 조건을 준다. |
| R | 검토안은 기존 `termux-orch relay`를 세션 사이에서 계속 진행해야 하는 약속의 소유자로 두는 것이다. L이 정한 순서와 대기 조건, 시작한 실행의 ID, 검증 결과, 현재 F와 전달 상태를 보존한다. |

### R이 맡을 부분

Slice 7A는 대화가 끊겨도 기존 작업 결과를 수집하고, 결과가 모였을 때
다음 턴을 시작하고, 변경된 사용자 창구에 보고하기 위해 릴레이를 도입했다.
현재 `relay.rs`가 참가자와 Wait-Set을 확인하고, `ledger.rs`가 Work와
Inbox를 보존하며, `owners.rs`가 각각의 실행 주체를 호출한다.
[릴레이 재시작·결과 전달 검증](../CURRENT.md#2026-11-23--stage-2-slice-7a-durable-multi-role-orchestration-termuxorch--mcptermux-orch-f--l--w--v--r--live-e2e)

R은 L의 판단을 보존하고, 실행 주체가 확인한 결과로 약속된 단계만 진행한다.
같은 세션이나 CI가 순서와 결과 전달까지 책임지면 그 실행을 그대로 쓴다.
세션이 닫혀도 다른 실행 결과를 기다렸다가 같은 파일의 다음 담당자를
깨워야 하거나, 다음 대화에 완료를 전달해야 하면 R에 맡긴다. 시작 예정과
대기 조건을 먼저 저장하며 실행 여부가 불확실하면 기존 기록과 대조한다.

### 공통 세션 전환 원칙 제안

Work를 유지하고, 컴팩션으로 기존 문맥이 교체되면 압축된 문맥으로 다음
실행을 이어간다. 다음 실행에 사용자 지시, 완료 조건, 남은 단계, 작업 중인
실행 ID, 마지막으로 전달이 확인된 이벤트 위치를 함께 넘긴다. 이미 시작한
실행과 승인·알림의 상태는 Work에 남는다. 역할별 턴 수는 공통 전환 조건으로
삼지 않는다.

- **공식 Codex.** 컴팩션 뒤 Codex가 압축한 문맥으로 같은 스레드 UUID를
  이어간다. 별도 UUID를 발급하는 정책은 실행 이력과 사용자 재개의 기준까지
  바꾸므로 별도 결정이다.
- **Codex + `chatgpt_desktop`.** 수동·자동 컴팩션 모두 같은 Codex UUID에서
  압축된 문맥을 새 ChatGPT 채팅으로 넘긴 것을 확인했다.
  [실계정 검증](../CURRENT.md#2026-11-24--codex-compaction-through-chatgpt-desktop)
- **직접 ChatGPT.** 현행 커넥터에서 컴팩션 완료 신호를 관측할 수 있는
  근거가 없다. 현재 채팅을 더 이어갈 수 없다는 신호가 확인되면 Work의
  최신 체크포인트로 새 채팅을 연다. `input_too_large` 한 건만으로는 단일
  입력의 크기와 기존 문맥의 한계를 구분할 수 없다. 자체 생성 로그·diff는
  요약과 참조로 줄일 수 있으며 사용자가 준 원문과 요구사항은 보존한다.
  같은 요청에 자동 전환을 반복하지 않는다. 새 채팅에서 축소된 체크포인트도
  같은 크기 오류를 반환하면 `payload_too_large`로 멈추고 원래 오류와 함께
  기록한다. 32 KiB는 릴레이가 생성하는 페이로드 예산이다.
- **셸.** 진행 중인 프로세스와 종료 기록을 계속 추적한다. 명령에 문맥
  컴팩션을 적용하지 않는다.

### 현행 실행 경로 비교

| 실행 환경 | 모듈과 호출 | 제약 | 결과 원본 |
| --- | --- | --- | --- |
| 셸 | `mcp/termux-shell`의 `termux_shell` → `$PREFIX/bin/sh -c` | Termux UID, 비대화형, 기본 60초·최대 600초, stdout/stderr 각각 64 KiB | 종료 코드·시그널·stdout/stderr, `call_id` 영수증, 명령이 만든 파일 |
| 분리 셸 | `orch_dispatch(register_worker, backend="shell")` → `shell_job.rs` → 내부 `termux-orch shell-job` → `sh -c` | 비대화형, 릴레이를 재시작해도 동일 프로세스를 관측한다. Work 취소가 곧 프로세스 트리 종료라는 보장은 검증되지 않았다. | PID·시작 틱, 결합 로그 끝 8 KiB, `exit.json`, 명령이 만든 파일 |
| Codex | `codex`, `codex exec`, `codex resume <UUID>` 또는 `mcp/termux-codex`의 `codex_turn`·`codex_threads`·`codex_call(thread_compact)` → 공식 데몬 | 선택한 프로필과 워크스페이스, Provider, 도구의 승인·샌드박스 정책이 적용된다. 커넥터의 `wait_ms` 최대 120초는 응답을 기다리는 시간이다. | 스레드 UUID·턴 ID·종료 상태·도구 실행 기록, diff/테스트/산출물 |
| 직접 ChatGPT | 앱 채팅 또는 `mcp/termux-chatgpt`의 `chatgpt_send(send/observe/stop)` → `chatgpt-provider /control/*` | 계정·프로젝트와 활성 채팅 확인이 필요하다. Provider 한 인스턴스는 최대 8개 동시 생성이며 같은 채팅에 대한 동시 쓰기를 막는다. 커넥터 `wait_ms`는 최대 300초다. 호스트 파일 작업은 연결된 도구를 통해 수행한다. | 채팅 ID·대화·응답·지원되는 커넥터 결과. 호스트 파일과 테스트의 원본은 실행 환경에 있다. |
| Codex에서 ChatGPT 선택 | `codex -p chatgpt` → 공식 Codex → `providers/chatgpt`의 `/v1/responses` → ChatGPT Desktop | Codex 프로필 설정에서 Provider를 선택한다. 모델 호출은 AppHost와 ChatGPT 로그인 상태에 의존한다. | Codex 실행 기록과 UUID, 별도로 저장된 ChatGPT 채팅 ID와 대화 |

### 격리 검증과 남은 운영 확인

- 참가자와 Lead는 원래 실행 의도와 ID로 복구한다. 릴레이는 고정 셸 단계,
  공유 워킹트리의 쓰기 순서, 별도 Git 워크트리의 변경 범위, 독립 판정을
  검증한다. 같은 Termux UID의 외부 프로세스는 해당 작업 예약을 공유하지
  않는다. 실제 브랜치 반영은 변경을 검토한 Lead가 맡는다.
- 특정 작업 조회와 함께 검증된 채팅 주소 또는 사용자 메시지 마커를 받으면
  알림을 이전한다. 커넥터에 주소가 전달되지 않는 경우의 자연어 이전과
  실제 모델의 자동 도구 선택은 운영 수락 시험에 남는다.
- Provider는 계정·프로젝트·입력 접두사가 유지되면 디스크에 저장한 채팅을
  재사용한다. 격리 환경에서 맵 재개와 접두사 변화, 실제 Codex의 Responses
  수신과 롤아웃을 확인한다. 실계정 재시작과 실제 최신 Electron의
  콜백 내용·타이밍은 별도 수락 시험이다.

### 구현 순서와 합격 조건

격리 후보의 구현·시험 상태는 [Android 후보 기록](../CURRENT.md#2026-11-24--slice-7-redesign-candidate-on-android-termux)에
있다. 자연어 자동 진입과 검증된 호출자에 의한 F 이전은 운영 적용 전 검증한다.

1. **계약과 진입점 확인.** 기존 Work 스키마·명시 설정을 보존한다. 설치된
   Codex 프로필에 Orch MCP가 없는 경우 격리 프로필에서 기존 `termux-orch
   mcp`를 등록한다. 자연어 요청에서 모델이 도구를 선택하는 동작과 도구
   진입 뒤 코드가 보장하는 순서를 구별한다. 커넥터가 없는 ChatGPT 채팅은
   해당 실행에 접근할 수 없다. 배포용 설정은 격리 검증 후 별도로 처리한다.
2. **실행·검증 결함부터 수정.** `mcp.rs`와 `relay.rs`의 실행 요청은 기존
   Work에 실행 의도와 고유 요청 ID를 먼저 저장하는 경로로 합친다. Codex의
   스레드 UUID는 첫 턴을 보내기 전에 기록한다. 응답 유실은 원래 실행을
   대조하며 미확인 요청은 보존한다. 취소는 새 시작을 먼저 차단하고 Work가
   소유한 실행의 중단을 해당 소유자에게 요청한 뒤 실제 종료를 확인한다.
   필수 자동 검증·독립 리뷰는 대상 변경본의 증거로 확인한다. 후속 수정은
   이전 판정을 무효화한다. 필수 검증 실패·증거 없음·실행 중인 워커가 있을
   때 완료 요청을 거절한다. 기존 완료 기록은 소급해 바꾸지 않는다.
3. **F 이전을 연결.** 사용자의 특정 Work 조회에서 검증된 현재 세션을
   기존 `rebind_front` 경로에 연결한다. 계정이나 대화 주소를 확인할 수
   없으면 자동 이전을 보류한다. 다음 알림은 최신 주소를 사용하며 이미
   발송을 시도한 알림은 원래 주소에서만 대조한다. 주소 이전과 사용자
   작업 변경 이벤트를 구분한다. 두 채팅에서 알림이 중복되지 않고 L이
   추가로 깨어나지 않아야 한다.
4. **고정 순서부터 실행.** L이 정한 순서를 Work와 기존 Wait-Set에 남기고,
   이미 정해진 후속 실행 하나를 릴레이가 시작하도록 연결한다. 별도 L
   세션 없이 여러 단계를 완료하는 첫 경로를 검증한 뒤 필요한 의존성만
   추가한다. 같은 작업 디렉터리에서는 L이 정한 선행 쓰기가 끝나기 전에
   후행 쓰기를 시작하지 않는다. 두 단계 도중 릴레이가 재시작돼도 후속
   실행 한 번, 변경 손실 0건, 실패 시 성공 보고 0건이어야 한다.
5. **문맥 전환과 실제 사용자 경로 검증.** Work ID와 대기 중인 실행을
   유지한다. Codex 수동·자동 컴팩션, Provider만 재시작한 경우, 프로젝트
   없는 직접 ChatGPT Work, 입력 크기 오류, 누적 문맥 한계를 각각 검증한다.
   평범한 질문은 별도 Work를 만들지 않고, 짧은 다단계 작업은 현재 세션이
   조율하는지 확인한다. 필요한 경우에만 도구를 자동 호출하는 것을 격리
   Android Codex와 커넥터 ChatGPT에서 검증한 뒤 기존 시작 경로를 정리한다.
6. **공유 원장 전환.** 후보 MCP와 릴레이를 같은 코드 세대로 검증한다.
   이전 릴레이는 새 Wait-Set의 필드를 보존하지 않는다. 배포 순서를 확인할
   때 새 실행 예약을 받기 전에 두 서비스가 같은 세대를 실행하도록 한다.
   롤백 시험은 새 형식의 실행 의도를 보존하거나 먼저 확정할 수 있는지
   확인한다. 읽기 전용 확인만으로 혼합 세대 쓰기의 호환성을 간주하지 않는다.
   `codex -p chatgpt`의 실계정 시험은 후보 MCP의 원장과 ChatGPT 계정에
   붙은 운영 커넥터의 원장을 분리해야 한다. Codex의 실제 `mcp_tool_call`
   기록에서 후보 호출을 확인하고, 실제 작업을 시작하기 전에 시험 연결이
   운영 커넥터로 위임할 수 없는 환경인지 확인한다.
