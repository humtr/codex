# Current Wrapper: responsibility and implementation map

**Source scope:** `rewrite/wrapper` standalone Rust `codex`, its
`codex-release-builder`, `install.sh` and `wrapper.yml` CI. This is a
source-to-feature map for the 2026-11-17 Stage 1 P0 operating baseline, not a
new normative specification or a fresh qualification report. For verified
source SHA, device hashes, CI runs and the live warning consult
[CURRENT](../../CURRENT.md). For required policy consult
[SPEC](../../SPEC.md) and the [documentation map](../INDEX.md).

**Four independent facts must not be conflated:**
(1) source compiled, (2) behavior qualified, (3) artifact publicly published,
and (4) installed runtime or service selected. A working upstream Codex,
Provider process or Linux AppHost on a device is not proof of optional
Wrapper integration.

## 1. Execution architecture and ownership

```text
Operator / shell / existing ChatGPT or terminal client
  |
  +-- install.sh --> $PREFIX/bin/codex                    [installer owns replacement]
  |                    |                                 [Wrapper process]
  |                    +-- update / doctor / wrapper ...  [Wrapper-owned routes]
  |                    +-- normal upstream argv ----------> package-root/bin/codex
  |                        |                              [official upstream process]
  |                        +-- Native threads / approvals / tools / history
  |                        +-- app-server daemon / remote-control
  |
  +-- optional future Stage 2 routing: Manager / enhanced TUI / Provider
       [2.1 configuration surface exists, executable attachment does not]
```

- **Installer** owns the `$PREFIX/bin/codex` executable swap and its
  interrupted-install receipt. It may invoke the Wrapper's upstream updater,
  but does not own the updater's activation pointer or journal. One operation
  is **not** an atomic two-component transaction.
- **Wrapper** owns public-route classification, upstream release discovery,
  archive verification, local ELF compatibility transforms, generation
  activation/rollback and Termux process setup. It `exec`s the **actual
  official upstream CLI** and does not replace its conversation engine.
- **Release-builder** owns preparing/adapting a complete official
  `package-root` and its manifest/bytes. It is not a second upstream update
  controller; the Wrapper invokes it along the single update path.
- **Upstream Codex** owns its conversation/session/rollout database,
  authentication, model selection, approvals, Native tool and process effects,
  compaction, daemon package selection and Remote Control protocol.
- **Provider / ChatGPT AppHost / TermuxBridge**, where independently installed
  or running from another lineage, own their own accounts, transport,
  inference correlation and service lifecycle. Stage 1 does **not** build,
  launch, authenticate, configure, migrate or update these services. A
  Bridge/Native work contract in an annex is **not** an active Stage 1 route.
- **Stage 2 Manager / enhanced TUI / Provider** are future opt-in consumers
  of Wrapper interfaces, not Wrapper substitutes for Native authority. They
  must use upstream-supported session APIs rather than create another
  authoritative conversation index or independent execution owner.

## 2. Implemented feature matrix

**Stage 1 = installed standalone path; 2.1 = implemented boundary, not
attached module; deferred = absent from this source/distribution.** Tests
quoted here are **test types**, not a claim that every Android/device/account
scenario has passed. See CURRENT for exact receipts.

| Feature and observed contract | Responsible implementation | State / boundary |
| --- | --- | --- |
| `codex` argv routing: `update`, `doctor`, `wrapper`; other argv including `app-server daemon` and `remote-control` pass to upstream | [Wrapper dispatcher](../../crates/wrapper/src/main.rs) | **Stage 1 implemented**, device passthrough accepted; daemon/remote protocols upstream-owned |
| Fast `--version`/`-V`/`--help`/`-h`/`help` bypass daemon-settings setup and addon selection | [main.rs](../../crates/wrapper/src/main.rs) | **Stage 1 implemented**; zero *addon* startup dependency, not a claim that any pending activation journal will be ignored |
| Unsupported Android sandbox modes rejected; no simulated Landlock/namespace isolation | [main.rs](../../crates/wrapper/src/main.rs) | **Stage 1 implemented**; `danger-full-access` is a compatibility default, **not** a security sandbox |
| FD 33 DNS, FD 34 Wrapper config, FD 35 temp/socket, safe env/TLS/browser helpers and TZif | [main.rs](../../crates/wrapper/src/main.rs), [timezone.rs](../../crates/wrapper/src/timezone.rs), [builder](../../crates/release-builder/src/lib.rs) | **Stage 1 implemented**, real Android E2E; config belongs to Wrapper, user `CODEX_HOME` data remains upstream-owned |
| Official stable release metadata, exact SHA-256, `--check` without activation and downgrade guard | [update.rs](../../crates/wrapper/src/update.rs), [remote_download.rs](../../crates/wrapper/src/remote_download.rs) | **Stage 1 implemented**; signed self-built bundles **not** part of this chain |
| Local `--upstream`/`--version`/`--path` candidate build, bounded account-free probes and explicit `--rollback` | [update.rs](../../crates/wrapper/src/update.rs), [qualification.rs](../../crates/wrapper/src/qualification.rs), [release-builder](../../crates/release-builder/src/lib.rs) | **Stage 1 implemented**; no mandatory CI result or enhanced TUI |
| Adaptive static-musl AArch64 FD 33/34 remap, UDS FD 35 `realpath`/socket handling, companion `bin/*` files and resources | [release-builder](../../crates/release-builder/src/lib.rs), [runtime_patch.rs](../../crates/release-builder/src/runtime_patch.rs) | **Stage 1 implemented**; candidate rejection on incompatible layout, **not** a guarantee of future upstream compatibility |
| Immutable `package-root`, `generation.meta`, activation pointer/journal, complete-state recovery, one prior generation | [state.rs](../../crates/wrapper/src/state.rs), [main.rs](../../crates/wrapper/src/main.rs), [update.rs](../../crates/wrapper/src/update.rs) | **Stage 1 implemented**; interrupted I/O faults qualified, real physical power outage **not** |
| Installer executable transaction, SHA receipts, writer ownership, conservative `--recover` and separate updater settlement | [install.sh](../../install.sh), [install recovery tests](../../.github/scripts/test_install_recovery.py) | **Stage 1 P0 implemented / device fault suite accepted**; no unconditional stale-lock deletion or updater replay |
| Bounded generated-artifact retention after explicit updater, lease/FD protection on launch | [maintenance.rs](../../crates/wrapper/src/maintenance.rs) | **Stage 1 implemented**; never delete sessions/auth/foreign roots as 'cleanup' |
| Upstream `doctor` capture + independent Wrapper report in `codex doctor [--json|--color]` | [doctor.rs](../../crates/wrapper/src/doctor.rs), [main.rs](../../crates/wrapper/src/main.rs) | **Stage 1 implemented**; upstream warning does not become a Wrapper repair action |
| Upstream-selected daemon package is **not** automatically replaced by Wrapper update | [main.rs](../../crates/wrapper/src/main.rs), [daemon_settings.rs](../../crates/wrapper/src/daemon_settings.rs) | **Stage 1 behavior**: Wrapper disables upstream daemon auto-update in settings, upstream retains explicit `app-server daemon update --from-cli` authority |
| Strict `wrapper.toml` (`manager`, `tui`, `provider`, `signed_update`) and `wrapper setup show|--enable|--disable|--bare|--signed-update` | [extensions.rs](../../crates/wrapper/src/extensions.rs), [main.rs](../../crates/wrapper/src/main.rs) | **Phase 2.1 interface implemented**; storing true does **not** install an addon or enable Ed25519 |
| `codex wrapper --bare` forces upstream; `--with MODULES` validates names then returns unavailable (exit 1) | [main.rs](../../crates/wrapper/src/main.rs) | **Phase 2.1 dispatch boundary implemented**; **no executable module routing today**. Persistent unavailable selections warn and bare upstream continues |
| Immutable FD 34 fragment snapshot prepare/validation, shared lease and safe pruning | [system_config.rs](../../crates/wrapper/src/system_config.rs) | **Phase 2.1 engine implemented/tested**; normal production launch passes *empty* extension input; no admitted Manager fragment producer is connected |
| Source-bound Android NDK build, Linux+Bionic smoke, test/lint, optional official upstream qualification and manually gated public byte-readback | [wrapper.yml](../../.github/workflows/wrapper.yml), [wrapper-verify.yml](../../.github/workflows/wrapper-verify.yml), [qualification harness](../../.github/scripts/qualify_wrapper.py), [publisher](../../.github/scripts/publish_wrapper_release.py) | **Stage 1 CI implemented**; CI is not an Android kernel, authorized activation or model/account test |
| Published standalone `wrapper` prerelease (archive, digest, `install.sh`, `SHA256SUMS`) | [publisher](../../.github/scripts/publish_wrapper_release.py), [install.sh](../../install.sh) | **Stage 1 P0 published and on-device installed** per CURRENT; HTTPS + SHA-256 integrity, **not** an Ed25519-signed channel |

### Not shipped by the active Wrapper

| Capability / owner | Exact current boundary | Required next implementation |
| --- | --- | --- |
| Ed25519-signed Wrapper/addon index, key rotation, `codex update --signed` and `codex-release-builder publish` | `signed_update` is settings **only**; Stage 1 `--signed` is invalid | [Phase 2.2](../plan/STAGE2_PLAN.md#phase-22--signed-wrapper--addon-distribution---signed--installsh-verification) |
| Manager profiles, account selection, notifications, rotation, shared-server FD 36 | No Manager crate/executable/commands in this branch's Stage 1 workspace | [Phase 2.3](../plan/STAGE2_PLAN.md#phase-23--optional-codex-manager--cratesruntime-binding-reattachment) |
| Enhanced Termux TUI, quota picker, `/switch`, clipboard enhancements | Runs upstream Codex TUI, not `frontend/tui` fork | [Phase 2.4](../plan/STAGE2_PLAN.md#phase-24--optional-enhanced-tui-frontendtui--upstream-upgrade-immunity) |
| Wrapper-managed ChatGPT Provider, Linux app account binding, `codex wrapper provider ...`, independent O/W orchestration | No Provider module/command or bridge lifecycle inside the current Wrapper | [Phase 2.5](../plan/STAGE2_PLAN.md#phase-25--optional-providerschatgpt--transporttunnel-client) |
| Ongoing official release canary with auto-repair or auto-install | `--check` and account-free qualification exist; no automatic source repair, signing or device deployment | [Phase 2.6](../plan/STAGE2_PLAN.md#4-phase-26--read-only-upstream-discovery-and-qualification) and explicit subsequent approvals |

## 3. Mutable state, authority and failure recovery

| State or effect | Sole writer / decision authority | Not authorized to do it |
| --- | --- | --- |
| `$PREFIX/bin/codex`, installer receipt, `.codex-install.lock`, previous executable | `install.sh` with verified ownership and exact-byte recovery | `codex update`, Manager, CI and read-only doctor |
| `$HOME/.local/lib/codex/wrapper/generations/*`, `activation-state`/`activation-journal`, `downloads/` | Installed Wrapper updater / activation transaction, mediated by builder | Installer manually switching pointers, remote publisher, optional Manager, Provider |
| `$HOME/.local/share/codex/wrapper/config/`, resolver cache and `wrapper.toml` | Wrapper FD preparation and explicit `codex wrapper setup` | Native/Provider account or session manager |
| `$CODEX_HOME` auth, config, threads, rollout files, `state_*.db` and active model/approval decisions | Official upstream Codex for that *specific* execution home | Wrapper, release-builder, generic installer, CI, deferred Manager; no credential copying across accounts |
| `$CODEX_HOME/app-server-daemon/settings.json` default `autoUpdateEnabled=false` | Wrapper's limited `daemon_settings::ensure` on normal launch | This is **not** a daemon package selection or process restart |
| Daemon executable pin, running server, `remote-control` session | Official upstream commands and their running process lifecycle | Wrapper update or doc changes do not implicitly rebind them |
| Git source branch, CI artifact, public Git tag/Release, production activation | Git maintainers / scoped CI publisher / independently authorized device operation respectively | None of those steps implicitly approves the next |

**Failure and observation discipline:**

- Installer recovery and generation journal recovery are **separate**. Either
  can reject ambiguous state. An executable swap does not prove an upstream
  update completed, nor does a changed generation authorize reverting the
  executable without receipt reconciliation.
- "Read-only doctor" means **no upstream transcript/DB mutation or automatic
  runtime upgrade**; launch and doctor may still perform bounded Wrapper-owned
  config preparation and recovery of a pending activation journal. Use
  `codex update --check` for the explicitly non-activating release metadata
  inspection contract. Do not advertise every doctor/help invocation as
  literally zero filesystem writes under fault conditions.
- `codex doctor --json` is a Wrapper envelope; `upstream.output` holds the
  **upstream doctor's serialized output**, including its own warnings. A
  Wrapper summary of `healthy` can coexist with an upstream
  `state.rollout_db_parity` **warning**, as in CURRENT. Wrapper does not own
  upstream session DB reindex, deletion or cross-profile repairs.
- `HOME` chooses the shared Wrapper generation/state root; `CODEX_HOME`
  chooses an upstream account/session home. Neither implies the selected
  ChatGPT Linux AppHost identity, and a saved profile label is not verified
  authentication. Cross-home changes require their actual owner and evidence.
- Physical power cut, 16 KiB kernel execution, another Android device,
  external Remote Control pairing and authenticated **post-cutover** inference
  are **not** closed by current Stage 1 acceptance. 16 KiB ELF alignment,
  Linux+Bionic, actual API 37 Termux E2E and prior authenticated tests each
  prove a different boundary.

## 4. When source or behavior changes

1. **Owner first:** name the producer, authoritative writer, consumer,
   failure settlement and expected persisted state. If it belongs to
   upstream Native or an external Provider, do not duplicate it in Wrapper.
2. **Scope first:** change an **active** Stage 1 or 2.1 implementation here,
   or explicitly stage a **deferred** 2.2–2.5 integration; a planned command
   spelling does not make a route available.
3. **Verify the actual path:** compare source contract, CLI parsing,
   negative/positive tests, exact artifact bytes, actual Android runtime and
   integrated account/client evidence as separate checks. A skipped CI job
   cannot be called a pass at its boundary.
4. **Update documents in this order:** normative
   [SPEC](../../SPEC.md) / relevant annex (behavior changed), this source
   implementation map (active code/state changed), user [README](../../README.md)
   (command changed), [Stage 2 plan](../plan/STAGE2_PLAN.md) (remaining work
   changed), and [CURRENT](../../CURRENT.md) (new proof actually obtained).
   Change history only to correct a historical error, not to announce a new
   deployment.
