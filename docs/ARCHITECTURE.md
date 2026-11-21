# Current Wrapper: responsibility and implementation map

**Source scope:** `rewrite/wrapper` standalone Rust `codex`, its
`codex-release-builder`, `install.sh` and `wrapper.yml` CI. This is the
active source-to-feature ownership map on `rewrite/wrapper`, not a
new normative specification or a fresh qualification report. For verified
source SHA, device hashes, CI runs and open boundaries consult
[CURRENT](../CURRENT.md). For required policy consult
[SPEC](../SPEC.md) and the [documentation map](INDEX.md).

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
| `codex` routes top-level `update`, `doctor`, and `wrapper` to the Wrapper. `codex -c k=v update\|doctor\|wrapper` returns exit `2` with direct-invocation guidance. `wrapper --bare` also blocks upstream `update` | [main.rs](../crates/wrapper/src/main.rs), [cli.rs](../crates/wrapper/src/cli.rs), [error.rs](../crates/wrapper/src/error.rs), [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs) | **Stage 1 implemented**. Other argv pass to upstream, including `app-server daemon`, `remote-control`, option values, and post-`--` prompts |
| Fast `--version`/`-V`/`--help`/`-h`/`help` and combined flags such as `-hV` bypass daemon settings, shared-home binding, timezone extraction, and addon selection | [cli.rs](../crates/wrapper/src/cli.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. Fast paths still load the selected generation |
| Unsupported Android sandbox modes, including attached `-sread-only` and `-sworkspace-write`, return exit `2` | [cli.rs](../crates/wrapper/src/cli.rs) | **Stage 1 implemented**. `danger-full-access` is the compatibility default |
| FD 33 DNS, FD 34 Wrapper config, FD 35 temp/socket, Termux environment, TLS, browser helpers, and TZif | [runtime_env.rs](../crates/wrapper/src/runtime_env.rs), [timezone.rs](../crates/wrapper/src/timezone.rs) | **Stage 1 implemented**. Wrapper places `$PREFIX/bin` before the inherited PATH entries and invokes the generation executable by absolute path. Upstream locates its package resources and adds its arg0 helpers |
| `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` binding isolates auth and daemon sockets, preserves distinct lines for renamed threads, and accepts validated `CODEX_SHARED_EXTRA_FILES` | [shared_home.rs](../crates/wrapper/src/shared_home.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. External launchers select homes. The optional [codex-account](../contrib/codex-account) script selects existing profiles |
| Official stable release discovery, exact-version cross-check, read-only `update --check`, unintended downgrade protection, and interactive bare `codex` 15-minute cached startup update prompt (`[y/N]`) | [upstream_metadata.rs](../crates/wrapper/src/upstream_metadata.rs), [update.rs](../crates/wrapper/src/update.rs), [remote_download.rs](../crates/wrapper/src/remote_download.rs), [progress.rs](../crates/wrapper/src/progress.rs) | **Stage 1 implemented**. Update progress reports download bytes and patch sections; startup check fails open when offline |
| Upstream updates reuse a compatible active generation when the version and archive SHA-256 match. `--force-rebuild` downloads, verifies, and adapts the archive again. `--path` admits a local archive or a prepared generation. Rollback verifies the retained generation independently. Updates and rollbacks align initialized daemon homes to `standalone/current` | [update.rs](../crates/wrapper/src/update.rs), [daemon_sync.rs](../crates/wrapper/src/daemon_sync.rs), [daemon_settings.rs](../crates/wrapper/src/daemon_settings.rs), [generation.rs](../crates/wrapper/src/generation.rs), [qualification.rs](../crates/wrapper/src/qualification.rs) | **Stage 1 implemented**. Wrapper serializes updates through daemon synchronization, verifies PID-backed daemons through upstream, and checks executable identity and readiness after replacement |
| Static-musl AArch64 FD 33/34 remap, UDS FD 35 `realpath`/socket handling (forward and backward BL), and runtime, helper, and `bin/*` companion SHA-256 digests | [builder](../crates/release-builder/src/lib.rs), [runtime_patch.rs](../crates/release-builder/src/runtime_patch.rs), [sha256.rs](../crates/release-builder/src/sha256.rs), [generation.rs](../crates/wrapper/src/generation.rs) | **Stage 1 implemented**. Wrapper and builder share the pure-Rust SHA-256 implementation. Generations predating companion rows still load |
| Immutable `package-root`, `generation.meta`, authoritative `activation-state`, journal recovery, `standalone/current` symlink reconciliation, and one prior generation | [state.rs](../crates/wrapper/src/state.rs), [generation.rs](../crates/wrapper/src/generation.rs), [update.rs](../crates/wrapper/src/update.rs) | **Stage 1 implemented**. Activation and recovery use lock-protected pointer changes and preserve ordinary files and directories at the link path |
| Installer executable transaction, SHA-256 receipts, and `--recover`. `wrapper status --verify` qualifies an existing generation by Wrapper API, persistent schema, and patch policy. A compatible local Wrapper upgrade retains both generation slots offline | [install.sh](../install.sh), [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs), [generation.rs](../crates/wrapper/src/generation.rs), [installer recovery tests](../.github/scripts/test_install_recovery.py) | **Stage 1 implemented**. `wrapper_artifact_digest` records provenance and provides exact binding when verifying new activation or interrupted bootstrap |
| Bounded generated-artifact retention after explicit updater, prepared catalog lease BEFORE staging, and FD protection on launch | [maintenance.rs](../crates/wrapper/src/maintenance.rs), [update.rs](../crates/wrapper/src/update.rs) | **Stage 1 implemented**; never delete sessions/auth/foreign roots as 'cleanup' |
| Read-only `codex doctor [--json\|--color]` composes bounded, redacted upstream diagnostics and attributed Termux checks. An absent or drifted FD 34 config uses a disposable copy | [doctor.rs](../crates/wrapper/src/doctor.rs), [generation.rs](../crates/wrapper/src/generation.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. Generation pointers and shared-home bindings are inspected in place |
| Initial `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` for eligible TUI and daemon start routes, plus `settings.json` `autoUpdateEnabled=false` | [cli.rs](../crates/wrapper/src/cli.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs), [daemon_settings.rs](../crates/wrapper/src/daemon_settings.rs) | **Stage 1 implemented**. Existing dedicated selections persist. Invalid settings block routes that require auto-update suppression. Upstream owns `app-server daemon update --from-cli` |
| Strict `wrapper.toml` and `wrapper setup`. `setup --bare` alone can recover a malformed regular file by restoring bare defaults. For valid settings it disables addons and retains `signed_update` | [extensions.rs](../crates/wrapper/src/extensions.rs), [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs) | **Phase 2.1 interface implemented**. Ordinary launch and inspection report invalid settings. Other writes and `--with` reject them |
| `codex wrapper --bare` forces upstream. `--with MODULES` validates names and reports unavailable (exit `1`) | [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs) | **Phase 2.1 dispatch boundary implemented**. Persistent unavailable selections warn and bare upstream continues |
| Shared bounded JSON handling, child-process capture and reaping, and safe filesystem and locking primitives | [json.rs](../crates/wrapper/src/json.rs), [proc_exec.rs](../crates/wrapper/src/proc_exec.rs), [fs_util.rs](../crates/wrapper/src/fs_util.rs) | Internal utilities used by diagnostics, qualification, download, and state-management paths |
| Immutable FD 34 fragment snapshot prepare/validation, shared lease and safe pruning | [system_config.rs](../crates/wrapper/src/system_config.rs) | **Phase 2.1 engine implemented/tested**; normal production launch passes *empty* extension input; no admitted Manager fragment producer is connected |
| Source-bound Android NDK build, Linux+Bionic smoke, tests, lint, and official upstream qualification required for release jobs. Publisher checks peeled tag, notes, prerelease flag, and exact asset readback | [wrapper.yml](../.github/workflows/wrapper.yml), [wrapper-verify.yml](../.github/workflows/wrapper-verify.yml), [qualification harness](../.github/scripts/qualify_wrapper.py), [publisher](../.github/scripts/publish_wrapper_release.py) | **Stage 1 CI implemented**. Linux+Bionic qualification is a separate boundary from device activation and authenticated execution |
| Published standalone `wrapper` prerelease (archive, digest, `install.sh`, `SHA256SUMS`) | [publisher](../.github/scripts/publish_wrapper_release.py), [install.sh](../install.sh) | **Stage 1 P0 published and on-device installed** per CURRENT; HTTPS + SHA-256 integrity, **not** an Ed25519-signed channel |

### Not shipped by the active Wrapper

| Capability / owner | Exact current boundary | Required next implementation |
| --- | --- | --- |
| Ed25519-signed Wrapper/addon index, key rotation, `codex update --signed` and `codex-release-builder publish` | `signed_update` is settings **only**; Stage 1 `--signed` is invalid | [Phase 2.2](ROADMAP.md#phase-22--signed-wrapper--addon-distribution---signed--installsh-verification) |
| Manager profiles, account selection, notifications, rotation, shared-server FD 36 | No Manager crate/executable/commands in this branch's Stage 1 workspace | [Phase 2.3](ROADMAP.md#phase-23--optional-codex-manager--cratesruntime-binding-reattachment) |
| Enhanced Termux TUI, quota picker, `/switch`, clipboard enhancements | Runs upstream Codex TUI, not `frontend/tui` fork | [Phase 2.4](ROADMAP.md#phase-24--optional-enhanced-tui-frontendtui--upstream-upgrade-immunity) |
| Wrapper-managed ChatGPT Provider, Linux app account binding, `codex wrapper provider ...`, independent O/W orchestration | No Provider module/command or bridge lifecycle inside the current Wrapper | [Phase 2.5](ROADMAP.md#phase-25--optional-providerschatgpt--transporttunnel-client) |
| Ongoing official release canary with auto-repair or auto-install | `--check` and account-free qualification exist; no automatic source repair, signing or device deployment | [Phase 2.6](ROADMAP.md#4-phase-26--read-only-upstream-discovery-and-qualification) and explicit subsequent approvals |

## 3. Mutable state, authority and failure recovery

| State or effect | Sole writer / decision authority | Not authorized to do it |
| --- | --- | --- |
| `$PREFIX/bin/codex`, installer receipt, `.codex-install.lock`, previous executable | `install.sh` with verified ownership and exact-byte recovery | `codex update`, Manager, CI and read-only doctor |
| `$HOME/.codex/packages/standalone/{releases/*,current}`, `activation-state`/`activation-journal`, `downloads/` | Installed Wrapper updater / activation transaction, mediated by builder | Installer manually switching pointers, remote publisher, optional Manager, Provider |
| `$HOME/.local/share/codex/wrapper/config/`, resolver cache and `wrapper.toml` | Wrapper FD preparation and explicit `codex wrapper setup` | Native/Provider account or session manager |
| `$CODEX_HOME` auth, config, threads, rollout files, `state_*.db` and active model/approval decisions | Official upstream Codex for that *specific* execution home | Wrapper, release-builder, generic installer, CI, deferred Manager; no credential copying across accounts |
| `$CODEX_HOME/app-server-daemon/settings.json` default `autoUpdateEnabled=false` and initial non-clobbering `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` (fresh home on a TUI route eligible to use a managed daemon or an explicit start route) | Wrapper's `daemon_settings::{ensure, ensure_default_package_link}`, gated by the upstream argv scan in `cli.rs` | Preserves existing package selections, symlinked roots, and modern and legacy launch traces. Actual startup is upstream-owned |
| Daemon executable pin, running server, `remote-control` session | Official upstream commands and their running process lifecycle. Wrapper update aligns eligible package selections via the upstream home operation lock and invokes upstream lifecycle commands when a restart is needed | Wrapper consumes the upstream status, PID identity and original launch environment for the handoff. Upstream owns socket readiness and shutdown |
| Git source branch, CI artifact, public Git tag/Release, production activation | Git maintainers / scoped CI publisher / independently authorized device operation respectively | None of those steps implicitly approves the next |

**Failure and observation discipline:**

- Installer recovery and generation journal recovery are **separate**. Either
  can reject ambiguous state. An executable swap does not prove an upstream
  update completed, nor does a changed generation authorize reverting the
  executable without receipt reconciliation.
- Doctor inspects installed state and shared-home bindings without recovering
  journals, creating links, or merging logs. It uses an ephemeral directory under
  `$TMPDIR` for diagnostic runtime preparation and preserves `$HOME`,
  `$CODEX_HOME`, `$CODEX_SHARED_HOME`, and `$PREFIX`.
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
   [SPEC](../SPEC.md) / relevant annex (behavior changed), this source
   implementation map (active code/state changed), user [README](../README.md)
   (command changed), [Stage 2 plan](ROADMAP.md) (remaining work
   changed), and [CURRENT](../CURRENT.md) (new proof actually obtained).
   Change history only to correct a historical error, not to announce a new
   deployment.
