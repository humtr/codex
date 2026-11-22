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

## Current Live Operating Snapshot

Observed on 2026-11-22.

| Boundary | Observed State | Notes |
| --- | --- | --- |
| **Published and installed Wrapper (`$PREFIX/bin/codex`)** | Stable Ed25519-signed `wrapper` tag at `3ea207cb` (`sequence=1`, key SHA-256 `0df170da...`). Published archive SHA-256 `68033d9c...`. Installed `bin/codex` SHA-256 `f83899d6...` | CI artifact ([run 44997871190](https://github.com/humtr/codex/actions/runs/44997871190)), signed `SHA256SUMS.sig` readback, and live Termux `--require-signed` install matched on 2026-11-22 |
| **Active / rollback upstream generation** | Active `0.171.0-d8c43b-2` (`ino=9741792`, SHA-256 `26d9f90b...`). Rollback `0.170.0-3f7e1a-2` | Preserved across Slice 1 signed Wrapper cutover; `wrapper status --verify --json` (`qualified=true`) and `codex doctor --json` (`summary=healthy`) agreed |
| **Live managed daemons (`default` / `janmori101` / `szob`)** | All three homes aligned to `$HOME/.codex/packages/standalone/current`; currently stopped | Verified `0.171.0 -> 0.170.0` rollback restart (PIDs `21082`, `21219`, `21352`) and post-upgrade stop cleanly |
| **Live doctor and external services** | `codex doctor --json` `termux_wrapper=healthy`, `summary=healthy`; live `0.167.2` `runit` services (`codex-chatgpt-*`, `tdev*`) undisturbed | Verified unchanged service PIDs across Slice 1 Wrapper cutover |
| **Stage 2 progress (Slices 1–6)** | **Slices 1–2 complete** (Slice 1 signed Wrapper live; Slice 2 `$PREFIX/bin/codex-ext` + `cdx` installed, SHA-256 `138def40...`, 1.22 MiB, zero `crates/wrapper` coupling). **Slices 3–6 pending** | Independent Stage 2 workspaces (3 MCPs, `chatgpt-desktop` + `chatgpt-provider`, `tunnel-client`) not yet built |
| **Stage 1 graduation status** | **Complete** (WP0–WP6 archived in [STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md)) | Stage 2 architecture & slice plan tracked in [docs/ROADMAP.md](docs/ROADMAP.md) and active [HANDOFF.md](HANDOFF.md) |

### Unverified / Remaining Boundaries

- Physical power removal (as distinct from userspace `SIGKILL` and injected I/O faults).
- Execution on an actual 16 KiB kernel page-size device or other Android API/hardware (only 16 KiB ELF geometry on Android API 37 / 4 KiB pages is verified).
- External Remote Control client pairing and post-cutover interactive model inference across all accounts.
- Deferred Stage 2 `/switch` cross-account UUID handoff and `session_index.jsonl` authority matrix (see [docs/ROADMAP.md](docs/ROADMAP.md#7-follow-up-verification-session-handoff-and-session_indexjsonl)).
- Post-retention disk reclamation of legacy per-home `packages/app-server-daemon/releases/`.

---

## Recent Verified Evidence (Newest-First)

### 2026-11-22 — Stage 2 Slice 2: `codex-daemon-client`, standalone `codex-ext` (`cdx`), §7 proof & live Termux install

- **Scope & independent workspaces:** Added reusable [crates/codex-daemon-client](crates/codex-daemon-client/src/lib.rs) (RFC 6455 WebSocket-over-UDS with `/proc/self/fd/<dirfd>/...` Termux FD 35 resolution and advisory `flock` writer-lock probe) and standalone [crates/codex-ext](crates/codex-ext/src/main.rs) (non-PTY launcher with `SIGINT` parent-ignore/child-restore & `termios` restoration, in-TUI `!cdx switch`/`!cdx provider` with `0600` session registry + fail-closed daemon busy check, `profile` CRUD compatible with existing `~/.local/share/codex/manager/profiles/`, `notify` Option A with comment/multiline-preserving `config.toml` `Stop` + `hooks.json` extras), path-triggered [codex-ext.yml](.github/workflows/codex-ext.yml), slimmed [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md), and retired `contrib/codex-account`. Root `Cargo.toml`/`Cargo.lock` (`['codex', 'codex-release-builder', 'libc']`) and `crates/wrapper` were untouched.
- **Post-audit hardening:** Fixed `SIGINT` parent survival (`SIG_IGN` in parent, `SIG_DFL` in child `pre_exec`), fail-closed `DaemonProbe::Error` handling, external-provider -> OpenAI account switch thread-provider guard, symlink `cwd`/`CODEX_HOME` canonicalization, multiline `notify = [...]` TOML replacement, and `codex-ext.yml` publisher sequence monotonicity + post-upload Ed25519 readback.
- **Verification (Android API 37 Termux):**
  - `cargo fmt --check`, `clippy --all-targets --locked -- -D warnings`, and tests under both `umask 0022` and `0077`: `codex-daemon-client` 3/3 + `codex-ext` 6 unit + 4 E2E integration tests passed; `check_doc_links.py` (14/14), YAML parse, and `shellcheck -S info install.sh` passed.
  - **Empirical §7 proof (disposable `$TMPDIR` against live `0.171.0`):** Confirmed `thread-writer-locks/<tid>.lock` `flock` held by idle Daemon A (`B` resume rejected: `already has an active writer`), released immediately after `daemon stop` (~869 ms), followed by Account B resuming the exact same UUID and completing turn 2 with Account A's context preserved; confirmed `thread/name/set` appends to `session_index.jsonl` and updates `state_5.sqlite` `threads.name` (`thread/list` & `codex exec resume <NAME>` immediate visibility) while `thread/archive` authoritatively updates `state_5.sqlite` `archived=1`.
  - **Live Termux install:** Installed release binary `$PREFIX/bin/codex-ext` (1,227,416 B, SHA-256 `138def40cfacbda605c6e592b77d645dbd87ca77185db7cd31ba6a5b2e9def5c`) + symlink `$PREFIX/bin/cdx -> codex-ext`; verified `cdx --version` (`0.1.0`), `cdx -- --version` (`codex-cli 0.171.0`), `cdx status` (read-only detection of all 8 profiles and running `janmori101`/`szob` daemons), and `$PREFIX/bin/codex wrapper status --verify --json` (`qualified=true`) unchanged. Interactive Termux UI `/copy` clipboard paste-back remains a physical-UI observation boundary.

### 2026-11-22 — Stage 2 Slice 1: Wrapper slimming, Ed25519 `SHA256SUMS.sig` signed release & live Termux cutover

- **Scope & commits (`3a87150e..3ea207cb` on `main`):** Finalized zero-coupling Stage 2 architecture across [SPEC.md](SPEC.md), [docs/ROADMAP.md](docs/ROADMAP.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), and [HANDOFF.md](HANDOFF.md); removed unused Phase 2.1 `system_config.rs` and `extensions.rs` hooks (`-1,905` lines in `258fe62b`, leaving `$PREFIX/bin/codex` as a pure Stage 1 runtime + `update`/`rollback`/`doctor`/`wrapper status`); implemented universal lightweight Ed25519 `SHA256SUMS.sig` signing, anti-rollback `sequence`, and key-rotation verification across [release_sign.py](.github/scripts/release_sign.py), [publish_wrapper_release.py](.github/scripts/publish_wrapper_release.py), [wrapper.yml](.github/workflows/wrapper.yml), and [install.sh](install.sh); provisioned isolated Stage 2 secret `CODEX_SIGNING_KEY` and pinned [release-public-key.pem](release/release-public-key.pem) (`0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca`).
- **Signed CI & public release publication:** [Wrapper CI run 44997871190](https://github.com/humtr/codex/actions/runs/44997871190) on `main` (`3ea207cb`) passed all 4 jobs (`checks`, `android-package`, `android-smoke` including 23/23 `test_install_recovery.py` signed/tamper/rollback/rotation/SIGKILL tests and full upstream Bionic qualification, and `release`). `publish_wrapper_release.py` verified `RELEASE_VERIFIED repo=humtr/codex tag=wrapper source=3ea207cb77e42bd9c22a92425d42c110bc891a17 archive_sha256=68033d9cf26f83d08919c287e8827c22f58272935e68a634831def16d9eadb8f prerelease=False signed=True sequence=1 key_sha256=0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca` across all 5 assets (`codex-wrapper-aarch64-linux-android.tar.gz`, `.sha256`, `install.sh`, `SHA256SUMS`, `SHA256SUMS.sig` 64 B).
- **Live Termux API 37 signed cutover:** Ran `sh ./install.sh --release wrapper --require-signed --skip-upstream`. Verified real Ed25519 signature (`sequence 1`), archive SHA-256, and `bin/codex` SHA-256 (`f83899d654a23baf83a5bb995c3395e3a74c07f7fca8a77b0810a7069c7d2811`); persisted `~/.local/share/codex/wrapper/release-sequence` (`1`) and `~/.local/lib/codex/wrapper/release-public-key.pem` (`0df170da...`); confirmed `codex --version` (`0.171.0`), `codex wrapper status --verify --json` (`active`, `0.171.0-d8c43b-2`, rollback `0.170.0-3f7e1a-2`, `qualified=true`), `codex doctor --json` (`termux_wrapper=healthy`, `summary=healthy`), retired `codex wrapper setup` returning `exit 2`, and zero disruption to live `0.167.2` `runit` services (`codex-chatgpt-host` PID `2605`, `codex-chatgpt-provider` PID `19984`, `codex-chatgpt-tunnel` PID `2554`, `tdev` PID `2557`, `tdev-tunnel` PID `4678`, `tdev-tunnel-conn_*` PID `3868`).

### 2026-11-22 — End-of-row atomic progress redraws and per-profile daemon tree summary

- [Wrapper CI run 44921454220](https://github.com/humtr/codex/actions/runs/44921454220)
  qualified source `ce439293` (building on `c1aeb52f` end-of-row atomic progress
  redraws and `751b863d` rate-limit fallback / ancestor deferral) with official
  `0.170.0` and `0.171.0` on the Android Bionic runner (`checks`,
  `android-package`, `android-smoke`, `release`). Published stable `wrapper` tag
  (`prerelease=false`), archive SHA-256 `2cc5410a...`, and `bin/codex` SHA-256
  `56b4adfa...` matched independent readback.
- Installed the published Wrapper (`56b4adfa...`) via
  `install.sh --skip-upstream`, ran `codex update --rollback` to leave the live
  runtime on `0.170.0-3f7e1a-2` (`previous=0.171.0-d8c43b-3`), and cleared
  `update-check.cache`. All three live managed daemons (`default` PID `21082`,
  `janmori101` PID `21219`, `szob` PID `21352`) reported `running 0.170.0` with
  matching executable `ino=650222`.

### 2026-11-21 — Flicker-free progress, concise daemon summary, ancestor deferral & GitHub API rate-limit fallback

- [Wrapper CI run 44911991173](https://github.com/humtr/codex/actions/runs/44911991173)
  qualified source `751b863d` with official `0.170.0` and `0.171.0` on the
  Android Bionic runner (`checks`, `android-package`, `android-smoke`, and
  `release`). Published stable `wrapper` tag (`prerelease=false`), archive
  SHA-256 `4e502398...`, and `bin/codex` SHA-256 `288e21d5...` matched
  independent readback.
- Isolated Android API 37 verification passed all 38/38 `qualify_wrapper.py`
  steps (`input_id: 037b592f...`), 113 Wrapper + 19 release-builder unit tests
  (umasks `0022`/`0077`), 47 `.github/scripts` Python tests, `strace` single-write
  and 4-byte in-place spinner checks (0 `\r\x1b[2K` clears during animation),
  42-column PTY prompt/resize regressions, WS-over-UDS self-daemon deferral and
  external rerun, and live GitHub API `403` rate-limit redirect fallback.
- Installed the published Wrapper binary (`288e21d5...`) via
  `install.sh --skip-upstream`, then ran `codex update --rollback` to leave the
  live runtime on `0.170.0-3f7e1a-2` (`previous=0.171.0-d8c43b-2`) with
  `update-check.cache` cleared for interactive upgrade testing. All three live
  daemons (`default` PID `30780`, `janmori101` PID `30937`, `szob` PID `31042`)
  reported `appServerVersion=0.170.0` with matching executable `ino=650222`, and
  `codex update --check --json` succeeded under live GitHub API `403` rate
  limiting (`status=available`, `upstream_version=0.171.0`).

### 2026-11-21 — Daemon synchronization release and live Wrapper cutover

- [Wrapper CI run 44871873250](https://github.com/humtr/codex/actions/runs/44871873250)
  qualified source `0cdb8eda` with official `0.171.0` and `0.170.0` on
  the Android Bionic runner. The public stable `wrapper` tag, release
  notes and four assets matched the verified CI output.
- The API 37 device qualified the existing runtime with the released
  Wrapper, installed the verified binary with `install.sh --skip-upstream`,
  then ran `codex update --upstream --version 0.171.0`. The updater returned
  already current with zero warnings. The activation-state SHA-256,
  generation slots, daemon links, PIDs and runtime inode matched the
  pre-install snapshot. Each of the three daemon control endpoints
  independently reported `backend=pid`, `status=running`, and
  `appServerVersion=0.171.0`.

### 2026-11-21 — Independent review of `c64d3e31..5994956a` (isolated Android API 37 Termux)

- **Reproduced on the baseline:** An actual `0.170.0` legacy daemon remained
  running after `update --path` activated `0.171.0`. Creating the dedicated
  link first hid its `app-server.pid` from upstream lifecycle commands.
- **Review candidate based on `5994956a`:** Qualification passed 38 steps with
  the official `0.170.0` and `0.171.0` archives on a 4 KiB kernel
  (input `885bf9e0...`, Wrapper binary SHA-256 `a789c3de...`). Both legacy
  and modern daemons changed PID and executable across rollback and restore.
  The replacements retained the SQLite binding and feature override. A stale
  PID received link alignment only, and a directory at the daemon package
  `current` path was preserved and reported while other homes completed.
  Read-only doctor checks reported a healthy Wrapper and app-server. The
  account-free upstream auth check returned exit 1. The same 38 steps passed
  with `0.169.1` as the rollback target (input `88c686b1...`).
- **Additional isolated checks:** Official already-current updates aligned a
  dedicated pin, preserved a matching live PID and reported a busy upstream
  daemon lock. After upstream `daemon update --from-cli` copied a package,
  the Wrapper re-aligned the stopped home's link and retained that copy.
  A concurrent Wrapper update was excluded while status succeeded.
  Native `command/exec` in stdio and through a restarted daemon,
  and the TUI `!` shell resolved bare `codex` to the isolated Wrapper.
  The daemon test also resolved packaged `rg` and the arg0 `apply_patch`
  alias, and both companion binaries passed qualification.
- **Startup and workspace gates:** PTY tests covered all three TTY gates,
  opt-out, a shared five-second discovery budget with delayed transport,
  invalid and future-dated cache entries, decline, EOF, update failure, and
  exec of `0.171.0` after confirmation. The cached repeat launches made zero
  discovery requests. Rust fmt, Clippy with `-D warnings`, doc links, and
  108 Wrapper plus 19 builder tests passed under both requested umasks.

### 2026-11-19 — Stage 1 Graduation: `origin/main` Promotion, Stable `wrapper` Release & Live Device Cutover (CI + actual Termux API 37)

- **Branch promotion & archival:** Archived legacy `0.167.2` `origin/main` (`fcefb8b3`) to `origin/archive/main-legacy-0.167.2`; retired ephemeral `HANDOFF.md` after archiving its WP0–WP6 summary into [STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md); fixed Ubuntu CI `shellcheck` SC2015 (`1e714e8b`) and Linux CI test portability (`e76a3ae4`, `104` Wrapper + `19` builder tests = `123` passed); promoted `rewrite/wrapper` cleanly to `origin/main` at `135fccd1` (zero force-push, exact tree parity).
- **CI & stable public release publication:** [Wrapper CI run 44464940477](https://github.com/humtr/codex/actions/runs/44464940477) on `main` (`135fccd1`) passed all 4 jobs: `checks`, `android-package`, `android-smoke` (including full `qualify_wrapper.py` cross-version `0.169.1`↔`0.170.0` upstream qualification on Android Bionic), and `release` (`publish_wrapper_release.py` verified peeled tag, notes, `prerelease=False`, and exact asset readback). Published stable GitHub Release `wrapper` (`isPrerelease: false`, `codex-wrapper-aarch64-linux-android.tar.gz` 966,831 B SHA-256 `636c0a5a0cc3...`, `bin/codex` SHA-256 `dfdcbbc25be2...`, `install.sh` SHA-256 `40c57c2f2df6...`).
- **Live device cutover (actual Termux API 37):** Saved pre-cutover backup (`8701483e...`) and state snapshot to `0700` `$HOME/.local/share/codex/wrapper-recovery/20261119-stage1-graduation/`. Executed the published `https://github.com/humtr/codex/releases/download/wrapper/install.sh` cleanly: verified release archive & `bin/codex` SHA-256, installed `$PREFIX/bin/codex` (`dfdcbbc25be2...`), and retained the compatible active generation offline (`retaining the qualified active upstream generation`). Verified byte-identical `activation-state` (`35540262...`), unchanged `current=0.170.0-3f7e1a` (`ino=9717389`) & `previous=0.169.1-3de502`, undisturbed live daemons (`PID 20823, 21394, 21500`) and 5 `runit` services, and `codex doctor --json` returning `exit 0` (`termux_wrapper=healthy`, `app_server=ok`, `summary=healthy`, parity `0/0/0/3`) across `default`, `janmori101`, and `szob`.

### 2026-11-19 — Stage 1 Completion Candidate WP1–WP6 (isolated Android API 37 Termux)

Qualified on `rewrite/wrapper` (`e9d323ba` Rust/installer/CI/docs sources + `a1ee6590` `qualify_wrapper.py` read-only boundary fix; release binaries `target/release/codex` 1.6 MiB SHA-256 `dd0a9ce4ca24...` and `target/release/codex-release-builder` 824 KiB SHA-256 `68bee9bea5d5...`; `Cargo.lock` locked to `['codex', 'codex-release-builder', 'libc']`). Live `$PREFIX/bin/codex` (`addffcba`), live generations, 3 daemons, and 5 `runit` services were **not modified**:

- **Workspace, installer & CI gates:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` under both umask `0022` and `0077` (**`103` Wrapper + `19` release-builder = `122` tests passed**), `sh -n` and `shellcheck` on `install.sh` and `contrib/codex-account`, Ruby YAML parse of `.github/workflows/*.yml`, `python3 -m unittest discover -s .github/scripts -p 'test_*.py'` (**`47` tests passed**, including real kernel zombie `Z` installer recovery, prerelease transitions/readback mismatch, and untracked doc-link rejection), and `check_doc_links.py` (`14/14` tracked docs).
- **Full isolated Android API 37 qualification (`qualify_wrapper.py`, kernel `6.1.162-android14-11-34343818-abS928NKSU6EZJB`, `page_size: 4096`, `android_api: 37`, no `openssl` in isolated `$PREFIX/bin`):** Passed all **29/29 steps** (`status: passed`, `input_id: fe3c859be271...`) across official `openai/codex` `0.169.1` (`3de502ad...`) and `0.170.0` (`3f7e1ae3...`):
  - Pure-Rust SHA-256 verification and `generation.meta` companion digest verification (`bin/codex-code-mode-host`, `bin/codex-exec-server`); 16 KiB ELF alignment `true` across Wrapper, `codex`, `codex-code-mode-host`, and `codex-exec-server`.
  - Managed daemon `start`/`version`/`stop` sharing standalone `current`, native `app-server` stdio RPC (`initialize`, `config/read`, `command/exec`), and PTY startup (`rendered: true`, 8 s, 12,348 bytes).
  - Read-only `codex doctor --json` (`doctor` and `shared-doctor`, both `termux_wrapper = healthy`, `app_server = ok`): verified zero writes to `$HOME`, `$PREFIX`, `$CODEX_SHARED_HOME`, Wrapper-managed daemon files, and non-`tmp/arg0-linux` profile entries; separate check with `wrapper/config/` deleted confirmed `termux_wrapper = healthy` + absent-config warning without recreating `wrapper/config/`.
  - `CODEX_SHARED_HOME` multi-profile (`CODEX_SHARED_EXTRA_FILES=custom.config.toml`): fast `--version` and `doctor` left profiles/shared home untouched; `features list` linked `custom.config.toml` and merged `session_index.jsonl` preserving both same-thread rename rows (`session_index_rows: 2`) while keeping `auth.json` isolated per profile.
  - Offline Wrapper-only upgrade (`install.sh --binary` with distinct-SHA ELF overlay `ad6ca8df...` and poisoned `curl`): retained `current=0.170.0-3f7e1a` and `previous=0.169.1-3de502` offline without invoking `curl`, followed by cross-version `--rollback` (`0.169.1-3de502` ↔ `0.170.0-3f7e1a`).
- **Explicit `--force-rebuild` & `--version` latency (same disposable root, cleaned afterward):** `codex update --version 0.170.0` skipped rebuild after Wrapper SHA change (`Codex 0.170.0 is already up to date.`), `codex update --version 0.170.0 --force-rebuild` rebuilt and activated `0.170.0-3f7e1a-2` (`previous=0.170.0-3f7e1a`), and `--rollback` restored `0.170.0-3f7e1a`. Ten-run `codex --version` latency measured `33.82 ms` median (`30.59–45.04 ms`) vs `14.25 ms` median (`12.13–26.18 ms`) raw patched `bin/codex --version` (down from ~40.2 ms pre-WP3 baseline).

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
