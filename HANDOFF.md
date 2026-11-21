# Stage 2 Pre-Design & Execution Work Order (`Phases 2.2–2.5` + `§7`)

- **Branch:** `stage2/phase-2.2` (branched from Stage 1 stable `main` at `8e7d1db5`)
- **Lifecycle:** Ephemeral execution work order per [docs/INDEX.md](docs/INDEX.md). Archive completed receipts to [CURRENT.md](CURRENT.md) / [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md) and retire this file when Stage 2 closes.
- **Normative authorities:** [SPEC.md](SPEC.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md), [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md), [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/ROADMAP.md](docs/ROADMAP.md).

---

## 1. Baseline & Non-Negotiable Stage 1 Invariants

Stage 1 standalone Wrapper (`ce439293` / CI [run 44921454220](https://github.com/humtr/codex/actions/runs/44921454220)) and Phase 2.1 extension interfaces (`wrapper.toml`, `codex wrapper setup`/`--bare`/`--with`, conditional FD 34 snapshot engine in [system_config.rs](crates/wrapper/src/system_config.rs)) are complete on `main`. Every Stage 2 phase must preserve:

1. **Zero prerequisite on bare `codex`:** `codex`, `codex --version`/`--help`, `codex update --upstream`/`--path`/`--rollback`/`--check`, `codex doctor`, and `codex wrapper status` must run with zero dependency on Node.js, Go, Manager, enhanced TUI, Provider, signing keys, or remote indices.
2. **Zero-write bare fast path:** When no optional module is enabled in `wrapper.toml`, bare launch spawns no Manager IPC, opens no FD 36 lease, writes no FD 34 snapshot, and runs no `features list` probe.
3. **Dependency isolation for bare `crates/wrapper`:** `crates/wrapper` (`codex`) stays free of `serde`, `serde_json`, `tungstenite`, `tokio`, `axum`, `syn`, and `toml` (depending only on `codex-release-builder`, `libc = "=0.2.186"`, and at most `codex-runtime-binding` whose sole dependency is `libc = "=0.2.186"`).
4. **Live device preservation:** Never modify `$PREFIX/bin/codex`, `~/.config/codex/providers/chatgpt.toml`, live `runit` services (`codex-chatgpt-*`, `tdev-tunnel*`), or live `@TermuxBridge` bindings during development/testing. All tests run in disposable `$HOME`/`$PREFIX`/`$CODEX_HOME`/`$TMPDIR` roots.
5. **Source lineage rule:** Take Stage 2 reference logic from `c200e676` (`rewrite/rust-core`), **never** wholesale from `ebb0b519`. Port only verified transition diffs from `ebb0b519` (`window.rs` `--ei com.termux.RUN_COMMAND_SESSION_ACTION 0`, `PackageRoot` `bin/codex` matching, `CODEX_PROFILE_WRAPPER` naming).

---

## 2. Branch Topology & Workspace/CI Isolation Strategy

### Branching Plan

`main` remains the clean, always-releasable Stage 1 (and graduated Stage 2) production branch:

```text
main (Stage 1 stable: 8e7d1db5)
  ├── stage2/phase-2.2              # Signed Wrapper/addon distribution (--signed + install.sh Ed25519)
  ├── stage2/phase-2.3              # runtime-binding + codex-manager reattachment (from merged 2.2)
  ├── stage2/phase-2.4              # Optional enhanced frontend/tui rebase & capability probe
  ├── stage2/phase-2.5              # providers/chatgpt + transport/tunnel-client + Provider CI
  └── stage2/session-handoff-verify # Independent §7 verification (/switch writer-lock + session_index.jsonl)
```

Each phase merges to `main` only after passing its full exit gate (workspace lint/tests, isolated Android API 37 qualification, and Bionic CI smoke) without regressing bare Stage 1 `wrapper.yml`.

### Cargo Workspace & CI Boundary Design

| Component | Workspace location | Direct Cargo dependencies | CI workflow lane |
| --- | --- | --- | --- |
| `codex` (Wrapper) + `codex-release-builder` | Root [Cargo.toml](Cargo.toml) | `libc = "=0.2.186"` (+ optional `codex-runtime-binding` in 2.3) | Existing [wrapper.yml](.github/workflows/wrapper.yml) / [wrapper-verify.yml](.github/workflows/wrapper-verify.yml) |
| `codex-runtime-binding` (Phase 2.3) | Root `Cargo.toml` (`crates/runtime-binding`) | `libc = "=0.2.186"` only | Verified inside `wrapper-verify.yml` (zero bloat) |
| `codex-manager` (Phase 2.3) | **Separate workspace** (`crates/manager/Cargo.toml` with `[workspace]`, path-depending on `../runtime-binding`) | `codex-runtime-binding`, `libc`, `serde`, `serde_json`, `tungstenite` | Dedicated `manager` CI job/workflow; keeps root `Cargo.lock` bare |
| `frontend/tui` source-builder (Phase 2.4) | Separate workspace / workflow under `frontend/tui/` | Upstream `codex-rs` + `syn` tooling | Dedicated opt-in TUI build workflow |
| `providers/chatgpt` + `tunnel-client` (Phase 2.5) | **Separate workspace** (`providers/chatgpt/Cargo.toml` already has `[workspace]`) + Go module | `codex-runtime-binding`, `axum`, `tokio`, `tokio-tungstenite`, `serde`, `toml`, `zstd`, Go stdlib | Dedicated `provider` CI workflow triggered only on `providers/chatgpt/**`, `transport/tunnel-client/**`, `crates/runtime-binding/**` |

> **Why separate `[workspace]` for `crates/manager`?** Just like `c200e676:providers/chatgpt/Cargo.toml`, giving `crates/manager` its own `[workspace]` and `Cargo.lock` keeps root `Cargo.lock` locked strictly to bare Stage 1 crates (`codex`, `codex-release-builder`, `codex-runtime-binding`, `libc`), guaranteeing zero transitive dependency creep into `codex`.

---

## 3. Target Stage 2 Architecture & Ownership Map

```text
User / Shell / External Client
  │
  ├── install.sh ──► $PREFIX/bin/codex                          [Installer: executable swap + Phase 2.2 Ed25519 verify]
  │                    │
  │                    ├── codex update [--upstream|--path|--rollback|--check]
  │                    │     └─► Official OpenAI upstream generations (SHA-256 + local ELF patch, LOCAL_ZERO)
  │                    │         + post-update managed daemon synchronization
  │                    │
  │                    ├── codex update --signed / wrapper update --signed          [Phase 2.2]
  │                    │     └─► Signed Wrapper + optional addon bundles (Ed25519 + sequence anti-rollback)
  │                    │         (distinct authority from upstream generations!)
  │                    │
  │                    ├── bare codex [UPSTREAM_ARGS...]                            [Stage 1 fast path]
  │                    │     └─► package-root/bin/codex (FD 33/34/35, zero addon IPC/locks)
  │                    │
  │                    ├── wrapper.toml / --with manager | codex wrapper profile|task|notify|...  [Phase 2.3]
  │                    │     └─► codex-manager + runtime-binding (FD 34 snapshot + FD 36 lease + shared server)
  │                    │
  │                    ├── wrapper.toml / --with tui                                [Phase 2.4]
  │                    │     └─► optional ELF-patched enhanced TUI (capability-probed, decoupled from version pin)
  │                    │
  │                    └── codex wrapper provider ...                               [Phase 2.5]
  │                          └─► providers/chatgpt + codex-termux-mcp-worker + tunnel-client (@TermuxBridge)
  │
  └── Upstream Codex (`package-root/bin/codex`)                                     [Sole Native authority]
        └─► auth.json, state_*.db, rollouts, approvals, tools, writer locks, managed daemons
```

### State & Authority Boundaries

| State / Effect | Authoritative Owner | Must Survive Failure / Independence Rule |
| --- | --- | --- |
| Upstream generations (`~/.codex/packages/standalone/{releases/*,current}`) + `activation-state` (`LOCAL_ZERO`) | Stage 1 Wrapper updater (`update.rs`, `state.rs`) | Never bricked or downgraded by signed Wrapper/addon bundle state |
| Signed Wrapper/addon trust & sequence (`~/.local/lib/codex/wrapper/release-public-key.pem`, signed bundle state/receipt) | Phase 2.2 signed updater + `install.sh` | Separate key/sequence authority from upstream `LOCAL_ZERO`; key rotation never rolls back `update_key` |
| Opt-in selection (`~/.local/share/codex/wrapper/wrapper.toml`) & FD 34 snapshots (`config/snapshots/<id>/`) | Phase 2.1 (`extensions.rs`, `system_config.rs`) | Missing/broken optional addon degrades only that addon; bare `codex` remains healthy |
| Shared servers & writer inspection (`~/.local/share/codex/wrapper/{servers,catalogs}`, FD 36 lease) | Phase 2.3 (`runtime-binding`, `codex-manager`) | Native retains sole conversation/writer authority; no shadow transcript DB |
| Provider config (`~/.config/codex/providers/chatgpt.toml`), work/catalog records (`termux-codex-operator-work`, `codex-chatgpt-catalog`) | Phase 2.5 (`providers/chatgpt`) | Single suffix-free schema (no `v1`/`v2`/`v3` legacy parsers); live services untouched until explicit cutover |

---

## 4. Open Design Decisions to Confirm Before Phase 2.2 Coding

| # | Phase | Decision | Recommended Default | Impact |
| --- | --- | --- | --- | --- |
| D1 | 2.2 | Separate signed Wrapper/addon bundle state file vs sharing upstream `activation-state` | **Separate** signed bundle state (e.g. `bundle-state` under `state_root`) while preserving Stage 1 `activation-state` (`LOCAL_ZERO`) strictly for upstream generations | Prevents `codex update --upstream` / `--rollback` from colliding with Ed25519 bundle sequence/key state |
| D2 | 2.2 | Ed25519 signing key provenance for `CODEX_RELEASE_SIGNING_KEY` | Generate a fresh dedicated Ed25519 keypair for `wrapper` / Stage 2 bundles, store private key in GitHub Actions secret, pin public key SHA-256 in `install.sh` + repo trust seed | Clean trust root with zero dependence on legacy `rewrite/rust-core` (`0.167.2`) keys |
| D3 | 2.2 | Ed25519 verification backend in Wrapper / `install.sh` | Use Termux `openssl` (`pkeyutl -verify`) gated strictly behind `--signed` / `signed_update = true` (Stage 1 bare path keeps zero `openssl-tool` dependency) | Preserves Stage 1 zero-dependency invariant while matching `DELIVERY.md` |
| D4 | 2.3 | `settings_identity` hash domain tag ([system_config.rs](crates/wrapper/src/system_config.rs#L59)) | Cut cleanly from `"core-system-config-v1"` → `"wrapper-system-config"` in both `system_config.rs` and `runtime-binding` (fresh `servers/<hex16>` namespace) | Matches unified suffix-free naming policy with no legacy server-retire ambiguity |
| D5 | 2.3 | `requirements.toml` generation via Manager | Keep FD 34 `requirements.toml` support only if cross-profile shared SQLite still needs it on `0.171.0+`; otherwise omit by default | Avoids unnecessary config overrides when upstream `CODEX_SQLITE_HOME` suffices |
| D6 | 2.4 | Enhanced TUI failure policy when `tui = true` in `wrapper.toml` after an upstream update | `--with tui` fails closed (`exit 1`); persistent `tui = true` warns and falls back to bare adapted `bin/codex` (matching existing unavailable-addon invariant in `SPEC.md`) | Upstream updates never brick interactive `codex` launches |

---

## 5. Phase-by-Phase Execution Plan & Checklists

### Phase 2.2 — Signed Wrapper & Addon Distribution (`--signed` & `install.sh` Verification)

**Goal:** Sign and verify self-built distributed bundles (`codex`, `codex-release-builder`, `install.sh`, optional `manager`/`tui`/`provider`) with Ed25519 + monotonic sequence anti-rollback, while keeping official OpenAI upstream tarballs on Stage 1's SHA-256 + local ELF adaptation path.

**Reference files (`c200e676`, read-only):**
- `crates/core/src/{remote_release.rs,release_display.rs,rollback_guard.rs,component_metadata.rs}`
- `crates/release-builder/src/lib.rs` (`publish` subcommand)
- `.github/scripts/{release_signing.py,release_publication.py}`

**Port vs. drop rules:**
- **Adopt:** Ed25519 `release.manifest` + `release.sig` + `release-authority.sig` rotation, `update-index`/`update-index-base` verification, bounded transfer/manifest limits, monotonic sequence anti-rollback, canonical HTTPS origin + asset-map resolution.
- **Drop:** Legacy whole-generation pre-patched `bin/codex` bundling (`runtime`, `helpers/{0,1,2}`), device-local signers, `config/build-signing-key`, `-v1`/`-v2`/`-v3` format suffixes (`asset-map.json`, `update-index`, `codex-release`).

**Checklist:**
- [ ] Lock decisions D1–D3 and update [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) if any signed-bundle state path is finalized.
- [ ] Add `codex-release-builder publish` subcommand for `Wrapper + Addon` bundles (no upstream `bin/codex` inside signed bundle).
- [ ] Implement Ed25519 trust seed enrollment (`~/.local/lib/codex/wrapper/release-public-key.pem` pinned by SHA-256), `release.manifest` / `release.sig` / `release-authority.sig` verification, and sequence anti-rollback in `crates/wrapper`.
- [ ] Wire `codex update --signed [--version ... | --remote <HTTPS> | --path <DIR>]` and `wrapper.toml` `signed_update = true` while keeping default `codex update` on Stage 1 `--upstream`.
- [ ] Upgrade [install.sh](install.sh) to verify downloaded Wrapper bundles via Ed25519 signature + pinned key digest before executing/swapping (retaining pre-execution verification invariant).
- [ ] Add signed publication scripts & negative/rotation/rollback/tamper unit + mock HTTPS tests.
- [ ] **Exit gate:** Full `publish` → `install.sh` / `--signed --path` / mock HTTPS `--remote` matrix passes (initial seed, rotation, tamper rejection, sequence downgrade rejection, `LOCAL_ZERO` ↔ signed coexistence, missing optional addon non-bricking) + all Stage 1 offline tests pass with zero `release-public-key.pem`.

---

### Phase 2.3 — Optional `codex-manager` & `crates/runtime-binding` Reattachment

**Goal:** Reattach shared server coordination, FD 36 execution lease, and `codex-manager` (`profile`, `task`, `notify`, `usage`, `rotation`, `launch`) strictly behind Phase 2.1 opt-in activation.

**Reference files (`c200e676` primary; selective `ebb0b519` diffs only):**
- `c200e676:crates/runtime-binding/` (retain `writer_free_until`, `writer_directories`, structured error codes; port `PackageRoot` `bin/codex` matching cleanly)
- `c200e676:crates/manager/` (preserve `launch_lock`, `same_display`, `origin::capture`, `validate_private_reentry`; apply ONLY `ebb0b519`'s `--ei com.termux.RUN_COMMAND_SESSION_ACTION 0` fix in `window.rs`)
- `c200e676:crates/core/src/{execution_lease.rs,shared_layout.rs,shared_server.rs,native_task.rs,manager_config.rs,manager_dispatch.rs}`

**Checklist:**
- [ ] Lock decisions D4–D5 (`settings_identity` domain tag, `requirements.toml` necessity).
- [ ] Add `crates/runtime-binding` (`libc`-only) to root workspace and update `system_config.rs` to share `settings_identity`.
- [ ] Port `crates/manager` in its own `[workspace]` (`crates/manager/Cargo.toml`) so root `Cargo.lock` stays bare.
- [ ] Complete rebrand & suffix cleanup: `codex termux` → `codex wrapper`, `CODEX_TERMUX_CORE_*` → `CODEX_TERMUX_WRAPPER_*`, `CODEX_PROFILE_CORE` → `CODEX_PROFILE_WRAPPER`, suffix-free IPC/formats (`__prepare-runtime`, `__prepare-launch`, `__task-*`, `__terminal-*`, `codex-manager-profile`, `notifications/config`), notify hook `["codex", "wrapper", "notify", "emit", "Stop"]`.
- [ ] Wire opt-in Manager dispatch, FD 34 fragment feeding into [system_config.rs](crates/wrapper/src/system_config.rs), FD 36 execution lease, `shared_server` (`setsid`, `SO_PEERCRED`), and optional `codex doctor` Manager section strictly behind `manager = true` / `--with manager` / explicit `codex wrapper <manager-subcmd>`.
- [ ] Add dedicated CI job for `crates/manager` while keeping bare `wrapper.yml` fast and dependency-isolated.
- [ ] **Exit gate:** Ported `runtime-binding` + `codex-manager` test suite passes in isolated `$HOME`/`$TMPDIR`; bare `codex` runs cleanly when `manager` binary is missing/corrupt/disabled; `--with manager` proves multi-profile isolation, FD 34 snapshot + FD 36 lease inheritance, shared server reuse, and Termux AM notify dry-run.

---

### Phase 2.4 — Optional Enhanced TUI (`frontend/tui`) & Upstream Upgrade Immunity

**Goal:** Reintroduce opt-in source-patched TUI (`/switch`, quota switching, permission picker, clipboard confirmation, foreground terminal FD bind) completely decoupled from `codex update --upstream` version pins.

**Reference files (`c200e676`, read-only):**
- `frontend/tui/` (`apply.py`, `native.patch`, `check_command_boundary.py`, `check_upstream_features.py`, `src/*.rs`)
- `crates/core/src/tui.rs`, `crates/release-builder/src/source*`

**Checklist:**
- [ ] Lock decision D6 (fallback policy on TUI incompatibility after upstream update).
- [ ] Rebase `frontend/tui/native.patch` + `syn` permission rewriter from `0.167.0` (`086ccc8...`) onto current upstream (`>= 0.171.0`).
- [ ] Replace `tui.rs`'s exact `upstream_package_version` match with capability/API probe (`--termux-backend-capabilities` → `codex-termux-resolved-startup`, `CODEX_TERMUX_BACKEND_STARTUP=resolved`).
- [ ] Cut over callbacks (`codex termux` → `codex wrapper`, `CODEX_PROFILE_WRAPPER`, `CODEX_TERMUX_WRAPPER_*`) while keeping `termux_reentry.rs` + `CODEX_CHAT_MODEL_REENTRY_PID` guards.
- [ ] Adapt optional TUI binary with existing `termux-fd-remap` + `termux-uds-fd35` in an isolated TUI build workflow.
- [ ] **Exit gate:** `frontend/tui` Python suites (`test_apply.py`, `test_command_boundary.py`, `test_upstream_features.py`) pass on rebased upstream; `--with tui` verifies clipboard/permission/`/switch`/terminal FD; `codex update --upstream` onto a newer mock upstream release succeeds even without a matching enhanced TUI.

---

### Phase 2.5 — Optional `providers/chatgpt` & `transport/tunnel-client`

**Goal:** Reattach ChatGPT Provider (`codex-chatgpt-provider`, `codex-termux-mcp-worker`) and Go `tunnel-client` under `codex wrapper provider ...` with zero disruption to live `@TermuxBridge` services.

**Reference files (`c200e676`, read-only):**
- `providers/chatgpt/`, `transport/tunnel-client/`, `scripts/build_tunnel_client.py`
- `crates/core/src/provider.rs`, `crates/release-builder/src/provider.rs`, `.github/scripts/{provider_signing,provider_publication}.py`

**Checklist:**
- [ ] Port `providers/chatgpt/` paths (`.local/share/codex/wrapper`, `generations/<id>/bin/codex`), config (`execution.wrapper` with clear migration error if `execution.core` is present), and callbacks (`wrapper __terminal-auto-show`, `CODEX_TERMUX_WRAPPER_*`).
- [ ] Cut cleanly to single suffix-free schemas (`termux-codex-operator-work`, `operator-work`, `work-route-transfer`, `codex-chatgpt-catalog`, `app-model-catalog`, `config`/`catalog`); drop legacy `v1`/`v2`/`v3`/`provider14`/`provider21`/`migrate-inference` parsers per [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md).
- [ ] Wire `codex wrapper provider install|update|status|rollback|chatgpt ...` behind Phase 2.1–2.3 boundaries.
- [ ] Add independent Provider + Go `tunnel-client` CI workflow (`android/arm64` cross-compile + Rust/Node tests) isolated from `wrapper.yml`.
- [ ] **Exit gate:** `providers/chatgpt` tests + provider packaging + Ed25519 verify + `qualification.schema.json` + Go `tunnel-client` `android/arm64` build pass in disposable roots; live `~/.config/codex/providers/chatgpt.toml` and `runit` services remain untouched until explicit operator cutover.

---

### Independent Track — `§7` Follow-Up Verification (`/switch` & `session_index.jsonl`)

Can run at any time in parallel with Phase 2.2–2.3 without blocking code integration:

- [ ] **§7.1 Same-UUID session handoff (`/switch`):** On an explicitly approved fresh test thread, verify account A turn completion → observed `thread-writer-locks/` release (not just `thread/unsubscribe` ACK) → account B resume with context preserved while account A's concurrent thread Y continues uninterrupted. Record exact `cli_resume_account_check` behavior honestly without mutating policies.
- [ ] **§7.2 `session_index.jsonl` authority matrix:** Under concurrent A/B execution, measure `rename` and `delete` visibility across `app-server thread/list`, `app-server thread/read`, TUI `/resume`, and `codex exec resume <NAME|UUID>` (UUID / new name / old alias) and record authoritative source (`state_5.sqlite` vs `session_index.jsonl`).

---

## 6. Immediate Next Step

1. Review open decisions **D1–D6** in Section 4 above.
2. Confirm whether to proceed immediately into **Phase 2.2 implementation** on this branch (`stage2/phase-2.2`) or run **§7 verification** in parallel.
