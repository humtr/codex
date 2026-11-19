# Current Operating State and Verified Evidence

[SPEC.md](SPEC.md) and its annexes (`docs/spec/*`) own required behavior;
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) maps what the current codebase
implements; [docs/ROADMAP.md](docs/ROADMAP.md) tracks deferred Stage 2 work;
[docs/INDEX.md](docs/INDEX.md) defines document authority and status vocabulary.
This file records **current live state, active open work, and recent verified
evidence (newest-first)**. Earlier 2026-11-16 – 2026-11-18 Stage 1 checkpoints
and retired work orders are archived in
[docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md).

---

## Current Live Operating Snapshot (2026-11-19)

| Boundary | Observed State | Notes |
| --- | --- | --- |
| **Active branch / source** | `rewrite/wrapper` (post-`addffcba`) | `origin/main` (`fcefb8b3`) still holds legacy `0.167.2` pointer files; promotion deferred until Stage 1 completion sign-off |
| **Installed Wrapper (`$PREFIX/bin/codex`)** | Published `wrapper` prerelease (`addffcba`) | HTTPS + SHA-256 checksum verified (not Stage 2.2 Ed25519) |
| **Active / rollback upstream generation** | Active: `0.170.0-3f7e1a` (`ino=9717389`, SHA-256 `096ac747...`)<br>Rollback (`previous`): `0.169.1-3de502` | Under `$HOME/.codex/packages/standalone/releases/`; `standalone/current -> releases/0.170.0-3f7e1a` |
| **Live managed daemons (`default` / `janmori101` / `szob`)** | All 3 homes linked to `$HOME/.codex/packages/standalone/current` | Independent PIDs (`20823`, `21394`, `21500`) and control sockets sharing `ino=9717389` with updated FD 34 (no `thread_unload_delay_secs = 0`) |
| **Live `codex doctor --json` (all 3 homes)** | `termux_wrapper = healthy`; rollout/DB parity improved from `70/3/1/75` → `0/0/0/3` | 72 broken paths fixed & duplicate pair isolated (2026-11-19); 3 missing-file `/usr/tmp/*` DB rows retained untouched |
| **External services (`runit` & `167.2`)** | All 5 `runit` services running; `167.2` preserved in original generation (`ino=489102`) | Independently owned; excluded from Wrapper binary unification |
| **Stage 2.1 boundary / Stage 2.2–2.5 addons** | 2.1 settings & FD 34 snapshot engine implemented; 2.2–2.5 deferred | Zero Manager/TUI/Provider executables attached |
| **Active Stage 1 completion work** | Tracked in root `HANDOFF.md` (WP1–WP6) | Phase A correctness & `0.171+` resilience, Phase B structural refactoring, Phase C CI gates & graduation |

### Unverified / Remaining Boundaries

- Physical power removal (as distinct from userspace `SIGKILL` and injected I/O faults).
- Execution on an actual 16 KiB kernel page-size device or other Android API/hardware (only 16 KiB ELF geometry on Android API 37 / 4 KiB pages is verified).
- External Remote Control client pairing and post-cutover interactive model inference across all accounts.
- Deferred Stage 2 `/switch` cross-account UUID handoff and `session_index.jsonl` authority matrix (see [docs/ROADMAP.md](docs/ROADMAP.md#7-follow-up-verification-session-handoff-and-session_indexjsonl)).
- Post-retention disk reclamation of legacy per-home `packages/app-server-daemon/releases/` (~2.82 GiB) and `$HOME/.local/share/codex/wrapper/downloads-v1` (~960 MiB), retained until rollback window sign-off.
- Promotion of `rewrite/wrapper` to `origin/main` and switching the GitHub `wrapper` release from `prerelease: true` to stable.

---

## Recent Verified Evidence (Newest-First)

### 2026-11-19 — Live Device Maintenance: Shared Daemon Unification & DB Reconciliation (actual Termux API 37)

Executed against the live Termux device following the archived maintenance runbook (`git show b7b48fca:docs/plan/STAGE1_DEVICE_MAINTENANCE_WORK_ORDER.md`):

- **Clean shutdown & backup:** Stopped all 47 Codex / `167.2` / DB-holding processes cleanly (`sv down` on `tdev-tunnel`, `tdev-tunnel-conn_*`, `codex-chatgpt-tunnel`, `codex-chatgpt-provider`, `codex-chatgpt-host`; `app-server daemon stop` on 3 homes; identity-verified `SIGTERM` on orphaned `167.2` processes; 0 `SIGKILL`; verified 5 s quiet with 0 open DB writers). Full SQLite backup API snapshots, rollout manifests, and pre/post state receipts archived under owner-only (`0700`) `$HOME/.local/state/codex-stage1-maint-20261119T0825KST/` (854 MiB).
- **Clone pre-verification:** Tested link cutover/rollback, 72-path SQLite single-transaction update, `167.2` 62-migration DB schema compatibility, and `session_index.jsonl` append-only rename semantics in an isolated disposable clone before touching live state.
- **Live DB & rollout cleanup:** Repaired 72 broken `~/.codex-profiles/*` rollout paths in one SQLite transaction (69 active, 3 archived; `PRAGMA quick_check = ok`; `wal_checkpoint(TRUNCATE)` clean); isolated duplicate rollout pair `01a0fe94-...` by moving the unreferenced 144 MiB copy outside `sessions/` while keeping the DB-referenced 417 MiB file; left 3 `/usr/tmp/*` missing-file DB rows and `session_index.jsonl` untouched.
- **Shared binary cutover & restoration:** Atomically linked `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` (`0.170.0-3f7e1a`, `dev=65098, ino=9717389`) on `default`, `janmori101`, and `szob`, restarted sequentially (PIDs `20823`, `21394`, `21500` sharing `ino=9717389` with isolated control sockets and updated FD 34), verified `codex doctor --json` across all 3 homes (`missing_active=0`, `missing_archived=0`, `dup_ids=0`, `stale_rows=3`, `termux_wrapper=healthy`), and restored all 5 `runit` services (`167.2` preserved at `ino=489102`).

### 2026-11-19 — Upstream Thread Lifetime & Rollback Recovery Candidate (isolated API 37 Termux)

Qualified on `rewrite/wrapper` above `0391865acfd9` (later included in `addffcba`). Leaves idle thread unload delay (`thread_unload_delay_secs`) to upstream while removing the historical global FD 34 `= 0` override:

- Disposable API 37 Termux homes ran managed daemons from `0.170.0-3f7e1a` against a local Responses HTTP server (closing sender immediately on `thread/queue/add` receipt): candidate completed **5/5** with fresh FD 34 and **5/5** after replacing legacy managed FD 34, whereas old FD 34 `0` and explicit user-config `0` produced **5/5** `turn_aborted`.
- Normal launch replaced legacy managed FD 34 config and preserved inode/mtime/ctime on subsequent launches; upstream `config/read` confirmed user-config and `-c` precedence (including explicit `0`).
- Release-binary fault tests verified rollback recovery after `update --path` reselection + journal-delete failure, pre/post-rename termination, state-dir sync failure, and ambiguous legacy journal recovery.
- Workspace gates passed: `cargo fmt`, `clippy -D warnings`, doc links, and `cargo test --workspace --locked` (`72` Wrapper + `9` release-builder tests under both umask `0077` and `0022`). Evidence root: `$PREFIX/tmp/wrap-idle.1tyULG/`.

### 2026-11-19 — Default Daemon Package Selection, rev 2 (isolated API 37 Termux)

Qualified on `rewrite/wrapper` above `0391865acfd9` with official `0.169.1-3de502` and `0.170.0-3f7e1a` archives in disposable Termux homes:

- Verified parity with unmodified `0.170.0` selection precedence across all 4 legacy and 4 modern PID/stderr traces and existing/broken dedicated `current` links.
- Two fresh homes started independent daemon PIDs and control sockets sharing one executable inode without creating dedicated `releases/`; explicit `daemon update --from-cli --yes` pinned a dedicated package preserved across Wrapper update and rollback.
- Help (including `-hV`), version, queries, `--no-daemon`, `--remote`, and direct `--from-cli` skipped link creation; literal `-- -hV` and attached option values containing `-hV` still launched TUI.
- Release-binary fault injection covered link rename/dir sync failures, pre/post-state-rename termination, state-dir sync failure (exit `3` + recoverable journal), direct rollback re-entry, and non-symlink preservation/rejection at `standalone/current`.
- Workspace tests passed under umasks `0077` and `0022`. Evidence root: `$PREFIX/tmp/daemon-fix-o8w22wke/`.

---

## Historical Archives

- **2026-11-16 – 2026-11-18 Stage 1 P0 cutover, installer/journal/release-provenance faults, Issue #33 FD 35 fix, Stage 2.1 snapshot engine, and retired work orders:** [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md).
- **Pre-rewrite generation admission and legacy `rewrite/rust-core` (`0.167.x`) archives:** [docs/history/GENERATION_ADMISSION.md](docs/history/GENERATION_ADMISSION.md) · [docs/history/LEGACY_RUST_CORE.md](docs/history/LEGACY_RUST_CORE.md).
