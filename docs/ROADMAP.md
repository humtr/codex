# Roadmap: Stage 2 Decoupled Architecture and Follow-Up Verification

> **Status:** **Remaining Stage 2 independent component plan, not the active Wrapper feature list.**
> Follow [documentation authority](INDEX.md) and the
> [current implementation/owner map](ARCHITECTURE.md).
> Normative contracts: [SPEC.md](../SPEC.md), [DELIVERY.md](spec/DELIVERY.md),
> [USER_INTERFACES.md](spec/USER_INTERFACES.md), [BRIDGE.md](spec/BRIDGE.md).
> Active design & slice checklist: [HANDOFF.md](../HANDOFF.md).
> **Current implementation state:** **Stage 1** and **Stage 2 Slices 1–6** are complete and live on `main` (see [CURRENT.md](../CURRENT.md)). Stage 2 was reassessed from first principles on **2026-11-22** to keep `codex` Wrapper strictly zero-coupling and source-patch-free, replacing the legacy `c200e676` coupled Phases 2.1–2.5 (`shared_server`, `native_route.patch`, `frontend/tui` fork, CDN `update-index`, DDPE monolith) with **6 independent decoupled slices (Slices 1–6)** — all 6 slices are now implemented and verified; opt-in 3-MCP tunnel enrollment, signed GitHub Release publication, and Phase 2.6 canary remain separate operational/deferred tracks.

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
   - Never overwrite `$PREFIX/bin/codex`, stop or mutate live `0.167.2` `codex-chatgpt-*` / `tdev-tunnel*` services, touch `~/.cache/codex-web-experiment/` (`26.1007.21434`), or modify `~/.config/codex/providers/chatgpt.toml` during development or tests. New Stage 2 paths are completely disjoint from live `0.167.2`.

---

## 2. Stage 2 Component Architecture & Slice Execution Order (Slices 1–6)

```text
Slice 1: Wrapper slimming + universal SHA256SUMS.sig Ed25519 release signing
  │       (closes crates/wrapper + establishes shared signed GitHub Release pipeline)
  │
  ├──▶ Slice 2: codex-ext (cdx) + [shared] crates/codex-daemon-client + §7 / OSC 52 proof
  │      │     (profile CRUD, notify hooks, non-PTY launcher + !cdx switch/provider; retires contrib/codex-account)
  │      │
  └──▶ Slice 3: [shared] crates/mcp-kit + mcp/termux-shell (@TermuxShell) + transport/tunnel-client (v0.0.20+, 403) + runit 3-state supervision
         │     (independent recovery plane + shared MCP & supervision foundation)
         │
         ├──▶ Slice 4: mcp/termux-codex (@TermuxCodex — 5 tools greenfield)
         │             (reuses codex-daemon-client + mcp-kit; mobile Codex control without Linux app)
         │
         └──▶ [Gate 5-0: latest official Linux chatgpt .deb isolated PRoot/Xvnc & CDP probe]
                │
                ▼
              Slice 5: host/chatgpt-desktop (generation rootfs) + providers/chatgpt (clean-room page_driver.js)
                │      (Type 1: codex -p chatgpt via /v1/responses + in-app tool callbacks; exposes /control/*)
                │
                └──▶ Slice 6: mcp/termux-chatgpt (@TermuxChatGPT — 6 tools greenfield) & optional 0.167.2 cutover
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

### Slice 4 — `@TermuxCodex` (`mcp/termux-codex`, 5 Tools Greenfield) *(Complete — 2026-11-22)*
- **Goal:** Build stateless thin MCP bridge over `crates/codex-daemon-client` + `crates/mcp-kit`: static core (`codex_status`, `codex_threads` read-only, `codex_turn` `start|steer|interrupt` with 1-shot new-thread start and `call_id` transport receipt) + disjoint extensions (`codex_catalog`, `codex_call`). Zero shadow task DB; independently retirable when upstream `remote-control` matures.
- **Verified (isolated Android API 37 Termux against official `0.171.0`):** `mcp/termux-codex` (`fmt`/`clippy`/`test` under `umask 0022`/`0077`, live `codex 0.171.0` `app-server daemon` empirical E2E, post-audit hardening, and canonical `~/.local/lib/codex/mcp/termux-codex/current/termux-codex-mcp` + `$PREFIX/bin/termux-codex-mcp` install with `not_enrolled` `runit` status).

### Slice 5 — `Gate 5-0` + `host/chatgpt-desktop` + `providers/chatgpt` (`page_driver.js` Clean-Room) *(Complete — 2026-11-23)*
- **Goal:** Pass `Gate 5-0` (install latest official ARM64 `chatgpt` `.deb` from `persistent.oaistatic.com/codex-app-prod/linux/deb` into `~/.local/lib/codex/host/chatgpt-desktop/generations/<ver>/rootfs`; test isolated PRoot/Xvnc + CDP), build `host/chatgpt-desktop` generation lifecycle (`current`/`previous` + separated `~/.local/share/codex/chatgpt-desktop/profile`) and slim `providers/chatgpt` (`chatgpt-provider` owning single CDP + clean-room greenfield `page_driver.js` over Electron `connect-app-host` `httpFetch` + dynamically discovered `CompletionClient` + native `local_function_signatures` + `/v1/responses` + local authenticated `/control/*`; retire DDPE/MCP/`shared_server` and redundant `codex-chatgpt-host` keepalive).
- **Verified (Android API 37 Termux against official `chatgpt 26.1117.41037` + `codex 0.171.0`):** `Gate 5-0` isolated CDP probe, 9-case combination E2E, live `gpt-6-sol-wm` (`xhigh`) Project creation (`POST /projects`) + multi-role `/control/send` orchestration + real unspoiled secret-nonce `codex 0.171.0 exec` Orchestrator (`collaboration.spawn_agent`/`wait_agent`) → Worker (`exec` V8 Code Mode `tools.exec_command`) → Orchestrator conclusion inside the created Project, post-audit hardening, XDG `codex/` namespace unification, version-suffix cleanup, and canonical `~/.local/lib/codex/providers/chatgpt/current/chatgpt-provider` + `$PREFIX/bin/chatgpt-provider` install (`not_enrolled`).

### Slice 6 — `@TermuxChatGPT` (`mcp/termux-chatgpt`, 6 Tools Greenfield) & Stage 2 Closure *(Complete — 2026-11-23)*
- **Goal:** Build stateless thin MCP bridge over `chatgpt-provider` `/control/*` + `crates/mcp-kit`: static core (`chatgpt_status`, `chatgpt_chats` read-only, `chatgpt_projects` read-only, `chatgpt_send` `send|observe|stop` with `call_id` receipt) + disjoint extensions (`chatgpt_catalog`, `chatgpt_call`). Zero local code execution privilege. Slim `docs/spec/BRIDGE.md` around the finalized 3-MCP + `chatgpt-provider` architecture.
- **Verified (Android API 37 Termux against live `runit` `chatgpt-provider` + `chatgpt 26.1117.41037`):** `mcp/termux-chatgpt` (`fmt`/`clippy -D warnings`/`test` 4 unit + 2 compiled stdio E2E under `umask 0022`/`0077`), live `runit`-supervised `chatgpt-provider` E2E (`gpt-6-sol-wm` `xhigh`, `call_id` replay & conflict, chat read/rename, project list/read, observe/stop/viewer), slimmed `docs/spec/BRIDGE.md` + `docs/spec/MCP_CHATGPT.md`, and canonical `~/.local/lib/codex/mcp/termux-chatgpt/current/termux-chatgpt-mcp` + `$PREFIX/bin/termux-chatgpt-mcp` install (`not_enrolled`).

---

## 3. Phase 2.6 — Read-only Upstream Discovery and Qualification

**User request, 2026-11-16:** Treat recurring official-upstream adoption as a
Stage 2 delivery work item. It must **extend** the single Stage 1
`codex update --upstream` / release-builder owner, not create another CLI,
version selector, updater, daemon, account dependency, or signature authority.
The ordinary standalone update and rollback remain usable independently of this
automation and of every optional module.

**Implemented base path:** `codex update --check --json` owns official latest/exact
descriptor comparison and immutable archive metadata. It creates no installed
state. Shared `wrapper-verify.yml` owns the existing check/package/Bionic jobs
and explicit account-free qualification with `qualify_wrapper.py`; the ordinary
`wrapper.yml` caller retains publication separately. `.github/workflows/wrapper-upstream.yml`
on `main` is strictly a read-only compatibility canary (`permissions: contents: read`)
running in Linux + Bionic emulation (`android_api` is `None`, with no auto-deploy, PR,
issue filing, or device access): local `codex update --upstream` resolves OpenAI releases
directly and has zero functional dependency on CI schedules, so keeping
`wrapper-upstream.yml` manual (`workflow_dispatch`, pointing at the current verified
`rewrite/wrapper` commit) avoids unnecessary daily pinned-commit cron runs while
retaining on-demand compatibility checks; see [current evidence](history/STAGE1_MILESTONES.md#standalone-qualification-and-read-only-upstream-discovery-follow-up-2026-11-16).

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

**Publication scope:** The **checksum-verified standalone Stage 1 prerelease**
and actual public consumer test were accepted on 2026-11-17 (CURRENT). This
does **not** mean Phase 2.2 Ed25519 signed distribution or automatic runtime
deployment is implemented. Before **signed** publication, close its own
signature/key/bootstrap/connected-client gates; read-only discovery and isolated
base qualification are already implemented independently of optional modules.
Base upstream **qualification and any separately authorized Stage 1 publication
must never wait for TUI, Manager or Provider**. Scheduling is not permission
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
power/16-KiB-kernel/other-device boundaries. The Stage 2.1 interface/FD 34
engine is code-complete; a real Manager fragment producer is **not**.

- Unified naming (`wrapper`, `codex-wrapper`, `codex-local-generation`, `codex-activation-state`, `package-root`, `termux-fd-remap`, `termux-uds-fd35`) across code, tests, [`install.sh`](../install.sh), [`.github/workflows/wrapper.yml`](../.github/workflows/wrapper.yml), and documentation.
- Removed legacy `99b` / `R10` / `helpers/{0,1}` / `RootCodeModeHost` / `upstream_doctor` / version-pinned tests.
- Passed local checks (`ruby` Psych YAML, `sh -n` / `shellcheck`, `cargo fmt`, `clippy`, `cargo test --workspace --locked`, isolated AArch64 mock-ELF E2E) and GitHub Actions `Wrapper CI` (`checks`, `android-package`, `android-smoke`).

---

## 7. Follow-Up Verification: Session Handoff and `session_index.jsonl`

이 두 과제는 `contrib/codex-account` 구현과 독립적으로 진행합니다.
Stage 1의 FD 34는 스레드 수명을 업스트림 정책에 맡깁니다. 마지막 클라이언트가
연결을 닫은 뒤에도 실제 writer 해제와 업스트림 재개 조건을 확인한 다음 인계합니다.

### 7.1 동일 UUID 세션 인계(`/switch`) 검증 기준
- **합격 조건:**
  1. 계정 A가 스레드 X의 턴을 완료하고, 단순 `thread/unsubscribe` ACK가 아니라 **`thread-writer-locks/`의 실제 writer 해제가 완료된 뒤** 계정 B가 동일 UUID X를 재개합니다.
  2. B가 보낸 다음 추론 요청에 A의 직전 응답 문맥이 포함되고 B의 턴이 완료됩니다.
  3. 같은 시간 동안 A의 다른 스레드 Y 작업이 중단 없이 지속·완료됩니다.
- **실계정 온라인 시험 계획:** 별도 승인된 새 테스트 스레드에서만 A 기준 턴 → writer 해제 → B 후속 턴을 검증합니다.
- **계정 일치 검사 정책:** `0.170.0`의 `cli_resume_account_check` 등 업스트림 계정 일치 검사가 켜진 환경에서 타 계정 재개가 거절되면, **계정 태그나 정책을 변조해 억지로 통과시키지 말고** 해당 버전/설정의 호환성 결과로 그대로 기록합니다.

### 7.2 `session_index.jsonl` 권위 원천 및 삭제/이름변경 검증안
- **분리 측정 항목:** A/B 동시 기동 상태에서 이름 변경(`rename`) 및 삭제(`delete`) 수행 후 아래 4개 경로의 **UUID 조회·신규 이름 조회·구 이름(alias) 조회**를 각각 측정하고 실제 권위 원천(`state_5.sqlite` vs `session_index.jsonl`)을 기록합니다:
  1. `app-server` `thread/list`
  2. `app-server` `thread/read`
  3. TUI 목록 및 `/resume`
  4. `codex exec resume <NAME|UUID>`
- **제약:** 각 조회의 권위 원천과 동시 쓰기 정합성이 입증되기 전에는 `state_5.sqlite`나 파일 타임스탬프를 역산해 `session_index.jsonl`을 임의 재생성하는 임시 우회를 적용하지 않습니다.
