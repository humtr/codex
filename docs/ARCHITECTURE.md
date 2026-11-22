# Current Wrapper: responsibility and implementation map

**Source scope:** `main` standalone Rust `codex`, its `codex-release-builder`,
`install.sh`, lightweight Ed25519 signing scripts (`release_sign.py`,
`publish_wrapper_release.py`), and `wrapper.yml` CI. This is the active
source-to-feature ownership map on `main`, not a new normative specification or
a fresh qualification report. For verified source SHA, device hashes, CI runs
and open boundaries consult [CURRENT](../CURRENT.md). For required policy
consult [SPEC](../SPEC.md) and the [documentation map](INDEX.md).

**Four independent facts must not be conflated:**
(1) source compiled, (2) behavior qualified, (3) artifact publicly published,
and (4) installed runtime or service selected. A working upstream Codex,
Provider process or Linux AppHost on a device is not proof of optional
Wrapper integration.

## 1. Execution architecture and ownership

```text
Operator / shell / existing ChatGPT or terminal client
  |
  +-- install.sh (Ed25519 SHA256SUMS.sig) --> $PREFIX/bin/codex  [installer owns replacement]
  |                                             |                [Wrapper process]
  |                                             +-- update / doctor / wrapper status
  |                                             +-- normal upstream argv -> package-root/bin/codex
  |                                                 |            [official upstream process]
  |                                                 +-- Native threads / approvals / tools / history
  |                                                 +-- app-server daemon / remote-control
  |
  +-- independent Stage 2 tools & services (Slices 2–6, zero Wrapper coupling):
       codex-ext (cdx), @TermuxShell / @TermuxCodex / @TermuxChatGPT, chatgpt-provider, tunnel-client
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
- **Stage 2 standalone components (`codex-ext`/`cdx`, 3 MCPs, `chatgpt-provider`,
  `host/chatgpt-desktop`, `tunnel-client`)** are completely decoupled executables
  and opt-in services with zero Wrapper compilation or launch coupling. They use
  official upstream `app-server daemon` JSON-RPC and `CODEX_SHARED_HOME` rather
  than creating another authoritative conversation index or Wrapper routing hook.

## 2. Implemented feature matrix

**Stage 1 = installed standalone Wrapper path; Slice 1 = shipped lightweight
Ed25519 signed release pipeline & Phase 2.1 hook retirement; deferred = absent
from this source/distribution (Slices 2–6).** Tests quoted here are **test
types**, not a claim that every Android/device/account scenario has passed. See
CURRENT for exact receipts.

| Feature and observed contract | Responsible implementation | State / boundary |
| --- | --- | --- |
| `codex` routes top-level `update`, `doctor`, and `wrapper` to the Wrapper. `codex -c k=v update\|doctor\|wrapper` returns exit `2` with direct-invocation guidance | [main.rs](../crates/wrapper/src/main.rs), [cli.rs](../crates/wrapper/src/cli.rs), [error.rs](../crates/wrapper/src/error.rs), [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs) | **Stage 1 implemented**. Other argv pass to upstream, including `app-server daemon`, `remote-control`, option values, and post-`--` prompts |
| Fast `--version`/`-V`/`--help`/`-h`/`help` and combined flags such as `-hV` bypass daemon settings, shared-home binding, and timezone extraction | [cli.rs](../crates/wrapper/src/cli.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. Fast paths still load the selected generation |
| Unsupported Android sandbox modes, including attached `-sread-only` and `-sworkspace-write`, return exit `2` | [cli.rs](../crates/wrapper/src/cli.rs) | **Stage 1 implemented**. `danger-full-access` is the compatibility default |
| FD 33 DNS, FD 34 Wrapper config, FD 35 temp/socket, Termux environment, TLS, browser helpers, and TZif | [runtime_env.rs](../crates/wrapper/src/runtime_env.rs), [timezone.rs](../crates/wrapper/src/timezone.rs) | **Stage 1 implemented**. Wrapper places `$PREFIX/bin` before the inherited PATH entries and invokes the generation executable by absolute path. Upstream locates its package resources and adds its arg0 helpers |
| `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` binding isolates auth and daemon sockets, preserves distinct lines for renamed threads, and accepts validated `CODEX_SHARED_EXTRA_FILES` | [shared_home.rs](../crates/wrapper/src/shared_home.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. External launchers select homes (`codex-ext` / `cdx` supersedes retired `contrib/codex-account`) |
| Official stable release discovery, exact-version cross-check, read-only `update --check`, unintended downgrade protection, and interactive bare `codex` 15-minute cached startup update prompt (`[y/N]`) | [upstream_metadata.rs](../crates/wrapper/src/upstream_metadata.rs), [update.rs](../crates/wrapper/src/update.rs), [remote_download.rs](../crates/wrapper/src/remote_download.rs), [progress.rs](../crates/wrapper/src/progress.rs) | **Stage 1 implemented**. Startup check fails open when offline. Terminal presentation handles single-key input and updates one progress row in place without full-line erase flicker |
| Upstream updates reuse a compatible active generation when the version and archive SHA-256 match. `--force-rebuild` downloads, verifies, and adapts the archive again. `--path` admits a local archive or a prepared generation. Rollback verifies the retained generation independently. Successful updates, including already-current results, align eligible daemon homes to `standalone/current` | [update.rs](../crates/wrapper/src/update.rs), [daemon_sync.rs](../crates/wrapper/src/daemon_sync.rs), [daemon_settings.rs](../crates/wrapper/src/daemon_settings.rs), [generation.rs](../crates/wrapper/src/generation.rs), [qualification.rs](../crates/wrapper/src/qualification.rs) | **Stage 1 implemented**. Wrapper serializes updates through [daemon synchronization](spec/DELIVERY.md#managed-daemon-synchronization), checks executable identity and readiness after replacement, defers stopping an ancestor daemon of the update process itself, and prints a concise per-profile summary with detailed stderr warnings on failures |
| Static-musl AArch64 FD 33/34 remap, UDS FD 35 `realpath`/socket handling (forward and backward BL), and runtime, helper, and `bin/*` companion SHA-256 digests | [builder](../crates/release-builder/src/lib.rs), [runtime_patch.rs](../crates/release-builder/src/runtime_patch.rs), [sha256.rs](../crates/release-builder/src/sha256.rs), [generation.rs](../crates/wrapper/src/generation.rs) | **Stage 1 implemented**. Wrapper and builder share the pure-Rust SHA-256 implementation. Generations predating companion rows still load |
| Immutable `package-root`, `generation.meta`, authoritative `activation-state`, journal recovery, `standalone/current` symlink reconciliation, and one prior generation | [state.rs](../crates/wrapper/src/state.rs), [generation.rs](../crates/wrapper/src/generation.rs), [update.rs](../crates/wrapper/src/update.rs) | **Stage 1 implemented**. Activation and recovery use lock-protected pointer changes and preserve ordinary files and directories at the link path |
| Installer executable transaction, SHA-256 receipts, and `--recover`. `wrapper status --verify` qualifies an existing generation by Wrapper API, persistent schema, and patch policy. A compatible local Wrapper upgrade retains both generation slots offline | [install.sh](../install.sh), [wrapper_cli.rs](../crates/wrapper/src/wrapper_cli.rs), [generation.rs](../crates/wrapper/src/generation.rs), [installer recovery tests](../.github/scripts/test_install_recovery.py) | **Stage 1 implemented**. `wrapper_artifact_digest` records provenance and provides exact binding when verifying new activation or interrupted bootstrap |
| Bounded generated-artifact retention after explicit updater, prepared catalog lease BEFORE staging, and FD protection on launch | [maintenance.rs](../crates/wrapper/src/maintenance.rs), [update.rs](../crates/wrapper/src/update.rs) | **Stage 1 implemented**; never delete sessions/auth/foreign roots as 'cleanup' |
| Read-only `codex doctor [--json\|--color]` composes bounded, redacted upstream diagnostics and attributed Termux checks. An absent or drifted FD 34 config uses a disposable copy | [doctor.rs](../crates/wrapper/src/doctor.rs), [generation.rs](../crates/wrapper/src/generation.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs) | **Stage 1 implemented**. Generation pointers and shared-home bindings are inspected in place |
| Initial `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` for eligible TUI and daemon start routes, plus `settings.json` `autoUpdateEnabled=false` | [cli.rs](../crates/wrapper/src/cli.rs), [runtime_env.rs](../crates/wrapper/src/runtime_env.rs), [daemon_settings.rs](../crates/wrapper/src/daemon_settings.rs) | **Stage 1 implemented**. Default link preparation preserves existing dedicated selections. Invalid settings block routes that require auto-update suppression. Upstream owns `app-server daemon update --from-cli` |
| Shared bounded JSON handling, child-process capture and reaping, and safe filesystem and locking primitives | [json.rs](../crates/wrapper/src/json.rs), [proc_exec.rs](../crates/wrapper/src/proc_exec.rs), [fs_util.rs](../crates/wrapper/src/fs_util.rs) | Internal utilities used by diagnostics, qualification, download, and state-management paths |
| Source-bound Android NDK build, Linux+Bionic smoke, tests, lint, and official upstream qualification required for release jobs. Publisher checks peeled tag, notes, prerelease flag, and exact asset readback | [wrapper.yml](../.github/workflows/wrapper.yml), [wrapper-verify.yml](../.github/workflows/wrapper-verify.yml), [qualification harness](../.github/scripts/qualify_wrapper.py), [publisher](../.github/scripts/publish_wrapper_release.py) | **Stage 1 CI implemented**. Linux+Bionic qualification is a separate boundary from device activation and authenticated execution |
| Published stable Ed25519-signed `wrapper` release (archive, `.sha256`, `install.sh`, signed `SHA256SUMS`, `SHA256SUMS.sig`) with pinned key trust, anti-rollback `sequence`, and key-rotation verification | [release_sign.py](../.github/scripts/release_sign.py), [publisher](../.github/scripts/publish_wrapper_release.py), [install.sh](../install.sh), [release-public-key.pem](../release/release-public-key.pem) | **Stage 2 Slice 1 shipped & live** per CURRENT; `codex update --upstream` still fetches official OpenAI releases directly with built-in SHA-256 and zero `openssl-tool` dependency |
| Standalone `codex-ext` (`cdx`) non-PTY launcher, `!cdx switch` / `!cdx provider`, `profile` CRUD, `notify` (Option A), and reusable `codex-daemon-client` WebSocket-over-UDS client | [crates/codex-daemon-client](../crates/codex-daemon-client/src/lib.rs), [crates/codex-ext](../crates/codex-ext/src/main.rs), [codex-ext.yml](../.github/workflows/codex-ext.yml) | **Stage 2 Slice 2 shipped & live** in independent workspaces (zero `crates/wrapper` coupling); supersedes retired `contrib/codex-account` |
| Shared `crates/mcp-kit`, independent `@TermuxShell` (`termux-shell-mcp`), patched `tunnel-client` (`v0.0.21`), and opt-in `runit` 3-state generation supervision | [crates/mcp-kit](../crates/mcp-kit/src/lib.rs), [mcp/termux-shell](../mcp/termux-shell/src/main.rs), [build_tunnel_client.py](../scripts/build_tunnel_client.py), [runit_service.py](../scripts/runit_service.py), [termux-shell.yml](../.github/workflows/termux-shell.yml), [tunnel-client.yml](../.github/workflows/tunnel-client.yml) | **Stage 2 Slice 3 implemented** in independent workspaces; zero `crates/wrapper` or Codex/Provider dependency for `@TermuxShell` |

### Not shipped yet (Slices 4–6)

| Capability / owner | Exact current boundary | Required next implementation |
| --- | --- | --- |
| Thin `@TermuxCodex` MCP (`app-server daemon` JSON-RPC bridge) | Not yet built | [Slice 4](ROADMAP.md#slice-4--termuxcodex-mcptermux-codex-5-tools-greenfield) |
| `host/chatgpt-desktop`, clean-room `providers/chatgpt`, and `@TermuxChatGPT` MCP | Not yet built; any live `0.167.2` services remain externally owned | [Slice 5](ROADMAP.md#slice-5--gate-5-0--hostchatgpt-desktop--providerschatgpt-page_driverjs-clean-room) & [Slice 6](ROADMAP.md#slice-6--termuxchatgpt-mcptermux-chatgpt-6-tools-greenfield--optional-cutover) |
| Ongoing official release canary with auto-repair or auto-install | `--check` and account-free qualification exist; no automatic source repair, signing or device deployment | [Phase 2.6](ROADMAP.md#3-phase-26--read-only-upstream-discovery-and-qualification) and explicit subsequent approvals |

## 3. Mutable state, authority and failure recovery

| State or effect | Sole writer / decision authority | Not authorized to do it |
| --- | --- | --- |
| `$PREFIX/bin/codex`, installer receipt, `.codex-install.lock`, previous executable | `install.sh` with verified ownership and exact-byte recovery | `codex update`, Manager, CI and read-only doctor |
| `$HOME/.codex/packages/standalone/{releases/*,current}`, `activation-state`/`activation-journal`, `downloads/` | Installed Wrapper updater / activation transaction, mediated by builder | Installer manually switching pointers, remote publisher, optional Manager, Provider |
| `$HOME/.local/share/codex/wrapper/config/` and resolver cache | Wrapper FD preparation | Native/Provider account or session manager |
| `$CODEX_HOME` auth, config, threads, rollout files, `state_*.db` and active model/approval decisions | Official upstream Codex for that *specific* execution home | Wrapper, release-builder, generic installer, CI, deferred Manager; no credential copying across accounts |
| `$CODEX_HOME/app-server-daemon/settings.json` default `autoUpdateEnabled=false` and initial non-clobbering `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` (fresh home on a TUI route eligible to use a managed daemon or an explicit start route) | Wrapper's `daemon_settings::{ensure, ensure_default_package_link}`, gated by the upstream argv scan in `cli.rs` | Preserves existing package selections, symlinked roots, and modern and legacy launch traces. Actual startup is upstream-owned |
| Daemon package selection, running server, `remote-control` session | Official upstream commands and their per-home process lifecycle. Wrapper update aligns each eligible home to the shared executable via the upstream home operation lock and invokes upstream when a restart is needed | Wrapper consumes upstream status, PID identity and original launch environment for the handoff. Upstream owns each home's process and socket readiness. Dedicated package files remain on disk |
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
2. **Scope first:** change an **active** Stage 1 / Slice 1 implementation here,
   or explicitly stage a **deferred** Slice 2–6 component in its own workspace;
   a planned command spelling does not make a route available.
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
