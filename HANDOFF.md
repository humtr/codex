# Stage 1 Completion & Refactoring Handoff (`rewrite/wrapper`)

**Status:** Active ephemeral work order (remove or archive to [docs/history/STAGE1_MILESTONES.md](docs/history/STAGE1_MILESTONES.md) when completed).
**Branch:** `rewrite/wrapper` (baseline after WP0 `b7b48fca` + documentation consolidation `e30bc790`).
**Authority:** [AGENTS.md](AGENTS.md) · [SPEC.md](SPEC.md) · [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) · [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) · [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [CURRENT.md](CURRENT.md).

---

## 1. Execution Model & Safety Guardrails

- **Orchestration:** Supervisor (Antigravity) assigns one Work Package (WP) at a time to Codex CLI (`codex exec -m gpt-6.2-sol -c model_reasoning_effort="xhigh" --ephemeral --dangerously-bypass-approvals-and-sandbox --json`) inside an isolated `git worktree`.
- **Account Rotation:** `szob` first → on quota exhaustion switch to `janmori101` → on both exhausted stop immediately and report next quota reset times.
- **Review & Commit Gate:** The worker **must NOT run `git commit`, `git push`, `git tag`, or modify remote releases**. Leave changes clean in the worktree for Supervisor `git diff` review and independent test verification; Supervisor finalizes one coherent commit per passed WP.
- **Hard Device Guardrails (NEVER violate):**
  1. Never modify `$PREFIX/bin/codex`, `$HOME/.codex`, `$HOME/.local/share/codex`, running daemons, or `runit` services.
  2. Never touch, stage, or commit `ais.config.toml` or `chatgpt.config.toml`.
  3. All integration/qualification runs must use disposable private `$TMPDIR/codex-wp-*` roots with isolated `HOME`, `CODEX_HOME`, `PREFIX`, and `TMPDIR`.
  4. Never weaken assertions, skip failing tests, or add crates outside `Cargo.lock = ['codex', 'codex-release-builder', 'libc']`.

---

## 2. Settled Design Decisions (Locked — Do Not Reopen)

1. **Wrapper Upgrade ↔ Upstream Generation Decoupling (`1-A`):**
   - `wrapper_artifact_digest` in `generation.meta` is provenance metadata only.
   - Compatibility gate is `(wrapper_api_identity, persistent_schema_identity, patch_policy_id)`.
   - `codex update [--upstream] [--version <M.m.p>]` skips download/rebuild when active `(upstream_package_version, source_artifact_digest)` matches and compatibility holds, unless `--force-rebuild` is explicitly passed.
   - `install.sh` separates:
     - `qualified_active_generation()`: runs `"$dest_bin" wrapper status --verify --json` (NO `--wrapper-sha256`) — used **only** pre-updater when `--version` is omitted so compatible active generations stay active offline and preserve `previous`.
     - `qualified_new_activation()`: runs `"$dest_bin" wrapper status --verify --wrapper-sha256 "$candidate_sha"` (+ `--upstream-version` when set) — used **only** after updater actually ran (`needs_update=1`) AND inside `recover_install_lock` for phases `updating`/`updated` so crash recovery never mistakes a pre-existing generation for an interrupted updater's commit.
2. **External Shared Home Extra Files (`2-A`):**
   - Remove hardcoded `ais.config.toml` and `chatgpt.config.toml` from `crates/wrapper/src/shared_home.rs`.
   - Default `SHARED_OPTIONAL_FILES = &["installation_id", "config.toml"]`.
   - Opt-in `CODEX_SHARED_EXTRA_FILES`: colon-separated (`:`), empty items ignored, max 16 unique names, each 1..=128 ASCII graphic bytes (`0x21..=0x7e`), single path component (no `/`, `\0`, `.`, `..`), rejecting reserved/sensitive names (`auth.json`, `auth.json.lock`, `auth.lock`, `config.toml`, `installation_id`, `session_index.jsonl`, `history.jsonl`, `version.json`, `models_cache.json`, any `state_*` or `*.sqlite` / `*.sqlite-wal` / `*.sqlite-shm` / `*.lock`). Invalid entries fail closed with `InvalidData`.
3. **Internal Pure-Rust SHA-256 (`3-A`):**
   - Add `crates/release-builder/src/sha256.rs` (~70 LOC FIPS 180-4 streaming `Sha256`, `pub fn sha256_bytes(&[u8]) -> String`, `pub fn sha256_file(&Path) -> io::Result<String>`, verified against NIST vectors).
   - Both `codex-release-builder` and `codex` use this internal SHA-256; remove runtime `openssl-tool` requirement from Wrapper, `install.sh` (`for req in curl gzip`), and `qualify_wrapper.py`.
   - Keep `codex-release-builder build --openssl <PATH>` as an accepted optional no-op CLI flag for caller compatibility, while making `GenerationRequest.openssl` optional (or unused) internally.
   - `install.sh` retains its POSIX shell checksum fallback (`sha256sum` → `openssl dgst -sha256`) solely for verifying the downloaded Wrapper tarball itself.
4. **Companion Binary Digests in `generation.meta` (Backward-Compatible):**
   - After the `helper\t<id>\t<sha256>` lines, `release-builder` emits `0..N` sorted lines `companion\tbin/<name>\t<lowercase-64-hex>` for every companion binary in `bin/*` other than `bin/codex` (including `bin/codex-code-mode-host`, `bin/codex-exec-server`, and any extra `bin/*`).
   - `load_local_generation` parses `0..N` optional `companion\t...` lines strictly after `helper_count` helpers (rejecting duplicates, paths not of the form `bin/<single-normal-component>`, `bin/codex`, invalid hex, or unexpected trailing fields). Existing installed generations with `0` `companion` lines (e.g. `0.170.0-3f7e1a`) MUST continue loading and verifying cleanly.
   - `verify_generation_asset_digests` verifies executable regular file + SHA-256 for runtime, every declared helper, and every declared companion.
5. **Exact Edge-Case Contracts:**
   - **Read-only `codex doctor`:** Never call mutating `prepare_shared_home_from_env()` or `prepare_wrapper_config()` inside `doctor.rs`. Validate `CODEX_SHARED_HOME` read-only (`inspect_shared_home_from_env()` resolves `CODEX_SQLITE_HOME` without creating dirs/symlinks/merging logs). If `$config_dir/config.toml` exists and matches `TERMUX_WRAPPER_CONFIG_CONTENT`, open `$config_dir` read-only for FD 34; if absent or drifted, report config status in the Termux wrapper section and supply FD 34 from an ephemeral `0700` temp directory under `$TMPDIR` removed immediately after spawning/exit (zero writes to `$HOME`, `$CODEX_HOME`, `$CODEX_SHARED_HOME`, or `$PREFIX`). Capture bounded `stderr` in non-PTY mode.
   - **Leading flags before Wrapper subcommands:** `codex -c k=v update|doctor|wrapper ...` fails with exit `2` (`DispatchPlanError`) telling the user to run `codex <subcommand>` directly without leading upstream flags. Post-`--` args and option values (`codex -c model=doctor exec hi`) still pass through.
   - **Corrupt `wrapper.toml` recovery:** Keep existing behavior (status/doctor/ordinary launch warn without writing; `--with` and mutating `--enable`/`--disable`/`--signed-update` reject corrupt file), EXCEPT pure `codex wrapper setup --bare` (only `--bare`, no other flags) atomically replaces a malformed regular `wrapper.toml` with bare defaults (`signed_update = false`, all addons `false`; still rejects symlinks/unsafe ownership).
6. **Remote Promotion Boundary (`4-A`):**
   - Stop after WP6 isolated Android verification and report full evidence. No `git push`, `origin/main` promotion, or GitHub Release mutation until explicit final user approval.

---

## 3. Work Packages (WP1 – WP6)

### WP1 — `crates/release-builder`: Patcher, TAR, Companions & Pure-Rust SHA-256

- **Scope:**
  1. `[NEW] crates/release-builder/src/sha256.rs`: pure-Rust FIPS 180-4 SHA-256 (`sha256_bytes`, `sha256_file`) + NIST short/long/streaming unit tests. Export publicly from `lib.rs` (`pub mod sha256;` or `pub use sha256::{sha256_bytes, sha256_file};`) and expose `pub const PATCH_POLICY_ID: &str` for `wrapper`.
  2. `[MODIFY] crates/release-builder/src/runtime_patch.rs`:
     - Fix `is_mov_wide` (`((insn >> 23) & 0x3f) == 0b100101` only — stops misclassifying `CSEL`/`UDIV`/`SDIV`/`RBIT`/`CLZ`).
     - Fix `is_safe_interleaved`: reject `ADCS`/`SBCS`/`CCMP`/`CCMN` (`0x1a00_0000` flag-setting), reg-indexed load/store `Rm <= 1`, 3-source DP (`MADD`/`MSUB`) `Ra <= 1`, and single `LDR`/`STR` SP writeback (`Rn == 31`).
     - Add `contains_daemon_prefix_literal(&[u8]) -> bool` checking `b"codex-daemon-"`. Main `bin/codex` fails closed (`BuilderError::Archive`) if `codex-daemon-` is present without `DAEMON_FMT_NEEDLE`.
     - Pass 3 RX cave finder (`L625–681`): enumerate **all** aligned zero runs per gap (`find_zero_slots`), not just the first run; raise `REALPATH_STUB_MAX_BYTES` headroom safely while checking against actual `stub.len()` so tight 0x140 caves still work.
     - Support static-PIE (`e_type == 3`, `ET_DYN`) when `PT_INTERP` is absent (still rejecting dynamic ELF with `PT_INTERP`).
  3. `[MODIFY] crates/release-builder/src/lib.rs`:
     - Replace external `openssl` calls with internal `sha256_file` (keep `--openssl` CLI flag optional no-op).
     - Main `bin/codex` FD remap validation: require `counts[0] > 0 && counts[1] > 0` (`/etc/resolv.conf` AND `/etc/codex/config.toml`).
     - TAR parser: raise `ARCHIVE_MAX_BYTES = 512 * 1024 * 1024`, `LOGICAL_ENTRY_MAX = 1024`, `PAX_PAYLOAD_MAX_BYTES = 4096`; support USTAR `prefix` (`345..500`), GNU LongName (`'L'`), generic PAX headers (`'x'`, arbitrary header name, `path=`/`size=`/`mtime=`/ignore unknown harmless keys), and base-256 numeric uid/gid/size while preserving strict path traversal/symlink safety.
     - Static-PIE (`ET_DYN` without `PT_INTERP`) acceptance in `validate_static_aarch64_elf`.
     - Hash and emit sorted `companion\tbin/<name>\t<sha256>` lines in `generation.meta` for all companion binaries in `bin/*` (`code_mode_host`, `extra_bins`).
- **Mandatory Tests:**
  - NIST SHA-256 vectors (`""`, `"abc"`, 448-bit/multi-block, file streaming).
  - `is_safe_interleaved` rejects `CSEL`, `UDIV`, `ADCS`, `CCMP`, `MADD` (`Ra=0/1`), reg-indexed `LDR` (`Rm=0/1`), single `LDR`/`STR` SP writeback.
  - Main `bin/codex` fails if `/etc/codex/config.toml` count is `0` (even when `/etc/resolv.conf` > 0); fails closed if `b"codex-daemon-"` appears without `DAEMON_FMT_NEEDLE`.
  - Pass 3 cave finder succeeds when a gap's 2nd zero run fits; synthetic `ET_DYN` (no `PT_INTERP`) patches cleanly while `ET_DYN` with `PT_INTERP` is rejected.
  - TAR parser tests for USTAR `prefix` (>100B path), GNU `L`, PAX `path=` + non-`././@PaxHeader` name + custom PAX keys, base-256 uid/gid, and path-traversal rejection.
  - `generation.meta` emits sorted `companion\tbin/...` digests for companions.
- **Verification Gate:** `cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && (umask 0022 && cargo test --workspace --locked) && (umask 0077 && cargo test --workspace --locked)`.

---

### WP2 — Wrapper/Upstream Decoupling, Updater, HTTP Transport & `install.sh`

- **Scope:**
  1. `[MODIFY] crates/wrapper/src/main.rs` (generation loader/verifier pieces needed by WP2):
     - Parse optional `companion\tbin/<name>\t<sha256>` lines after `helper` lines (backward-compatible with `0` companions).
     - Switch `openssl_sha256` callers to `codex_release_builder::sha256_file`; verify companion digests in `verify_generation_asset_digests`; remove `OpenSslUnavailable` runtime gate from update/status paths.
  2. `[MODIFY] crates/wrapper/src/update.rs`:
     - Add `--force-rebuild` to `UpdateAction::Upstream` & update help output.
     - Gate `check_release` (`"current"`) and `run_upstream_update` (`AlreadyCurrent`) on `(upstream_package_version, source_artifact_digest)` + `generation_compatible_with_wrapper` (`wrapper_api_identity`, `persistent_schema_identity`, `patch_policy_id`) instead of `wrapper_artifact_digest == core_sha256`, honoring `force_rebuild`.
     - Allow forward `codex update` and `codex update --path` to recover even when the current generation on disk is damaged (load pointer state independently of `current` runtime integrity, like `run_rollback`).
     - Decode UTF-16 surrogate pairs in `JsonCursor::string` (GitHub release body emoji).
     - Allow `"digest": null` on non-target release assets in `parse_assets` (still reject `null`/missing on `UPSTREAM_PACKAGE_ASSET`).
     - Use `REMOTE_CONTROL_TRANSFER_TIMEOUT_SECONDS` (`30s`) for metadata fetches (`L688`).
     - Call `sync_tree_recursive(&final_dir)?` in `reuse_existing` branch (`L1207–1219`).
  3. `[MODIFY] crates/wrapper/src/remote_download.rs`:
     - Drop `command` immediately after `spawn()` so parent releases stderr write-end.
     - Replace raw byte `tail.drain(..tail.len() - 256)` with UTF-8-safe bounded suffix retention (`from_utf8_lossy` / char boundary) so `CODEX_HTTP_STATUS=` is never split/dropped.
     - Send `User-Agent: codex-termux-wrapper` (and `Accept: application/vnd.github+json` for GitHub API); surface actionable HTTP 403/429 rate-limit message suggesting `codex update --version <X.Y.Z>`.
     - Cache `known_total` once parsed from `.hdr` instead of re-reading `.hdr` every 80 ms; remove `.hdr` on completion/error; skip hashing 0-byte fresh `.part`.
  4. `[MODIFY] install.sh` & `[MODIFY] .github/scripts/test_install_recovery.py`:
     - Remove `openssl` from `for req in curl gzip` (`L673`), keeping shell checksum fallback (`sha256sum` → `openssl`).
     - Implement `qualified_active_generation()` vs `qualified_new_activation()` per Section 2.1 so Wrapper-only upgrade preserves compatible active generation and `previous` rollback slot offline, while post-updater & crash-recovery verification retains `--wrapper-sha256`.
     - Fix missing `else` in `updater_is_quiescent` (`L268–276`) so zombie (`Z`/`X`) updater with matching start ticks is quiescent instead of failing as PID reuse.
     - Guard `curl | sh`: when `[ ! -f "$0" ]`, never auto-detect local bundle/repo in cwd (`source_mode='release'`).
     - Export `TMPDIR` alongside `PREFIX` (`L679–680`) when set.
     - Reject `/` in `--version` (`validate_version_token` matching `*[!A-Za-z0-9._-]*`).
     - Expand `test_install_recovery.py` with tests for: compatible Wrapper-only upgrade preserving `previous` without calling updater, zombie (`Z`) updater recovery, stdin pipe ignoring cwd bundle, and `--version` with `/` rejection.
- **Verification Gate:** `cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && (umask 0022 && cargo test --workspace --locked) && (umask 0077 && cargo test --workspace --locked) && sh -n install.sh && shellcheck install.sh && PYTHONDONTWRITEBYTECODE=1 python3 -m unittest .github/scripts/test_install_recovery.py -v`.

---

### WP3 — Runtime Correctness: CLI Dispatch, Locks, `shared_home`, `doctor`, `qualification`, `daemon_settings`, `timezone`, `maintenance`, `fs_util`

- **Scope:**
  1. `[MODIFY] crates/wrapper/src/main.rs`:
     - Block attached `-sread-only`/`-sworkspace-write`.
     - Handle non-UTF-8 args in `first_top_level_subcommand_index` without bypassing `update`/`doctor`/`wrapper` classification.
     - Reject leading global flags before `doctor` and `wrapper` with exit `2` (matching `update` and updated `SPEC.md`).
     - Add `summary.saw_help_or_version` check to `upstream_invocation_requires_daemon_settings`.
     - Eliminate TOCTOU in `reconcile_standalone_current_and_load` (use bounded waiting `LOCK_EX` or retry on writer contention) and acquire `maintenance::launch_lease` in `run_wrapper_status`.
     - Remove duplicate `maintenance::prune(&roots)` call in `run_public_main` (`L3110–3114`; already executed by `UpdateLease::drop`).
     - Allow pure `codex wrapper setup --bare` to overwrite malformed regular `wrapper.toml` (keeping `--enable`/`--disable`/`--signed-update`/`--with` rejection).
     - Skip `timezone::configure` on fast `--version`/`--help` passthrough (`is_help_or_version_invocation`).
  2. `[MODIFY] crates/wrapper/src/state.rs`:
     - Handle absent `standalone/releases` cleanly during uncommitted initial journal recovery (`before=None, authoritative=None`, `L849–863`).
  3. `[MODIFY] crates/wrapper/src/shared_home.rs`:
     - Implement `2-A` (`SHARED_OPTIONAL_FILES = &["installation_id", "config.toml"]` + validated `CODEX_SHARED_EXTRA_FILES`).
     - Fix `session_index.jsonl` merge (`L267–288`): deduplicate by exact full line (not by `"id"`) so append-only thread renames are preserved.
     - Replace unbounded `fs::read` in `ensure_optional_shared_file` (`L353–354`) with `fs_util::read_bounded_regular(..., 4 * 1024 * 1024, false, ...)`.
     - Add read-only `inspect_shared_home(&codex_home, &shared_home)` for `doctor.rs`.
  4. `[MODIFY] crates/wrapper/src/doctor.rs`:
     - Normalize/merge overlapping redaction ranges before `doctor_styled_redactions`; tighten key-to-delimiter proximity so unrelated later `:`/`=` isn't over-redacted; fix `\x1b[0K` (erase to end of line, not whole line); add comprehensive unit tests for redaction/SGR/UTF-8/`\x1b[0K`.
     - Capture bounded `stderr` in non-PTY mode; decouple `try_wait()` from `eof` so dead children with leaked grandchild pipe fds don't stall until deadline.
     - Enforce strictly read-only `doctor` (use `inspect_shared_home`, never write `$config_dir/config.toml` — use ephemeral read-only temp dir for FD 34 if absent/drifted and report status).
  5. `[MODIFY] crates/wrapper/src/qualification.rs`:
     - Clear `CODEX_SQLITE_HOME` and `CODEX_SHARED_HOME` on probe child (`L84–90`).
     - Preserve original probe error if `fs::remove_dir_all(&home)` also fails (`L165`).
  6. `[MODIFY] crates/wrapper/src/daemon_settings.rs`:
     - Canonicalize/enforce `autoUpdateEnabled: false` across duplicate `"updater"` / `"autoUpdateEnabled"` JSON keys (`L328–371`).
     - Reject non-hex characters such as `\u+041` in `\uXXXX` escape parsing (`L565–575`).
  7. `[MODIFY] crates/wrapper/src/timezone.rs`:
     - Preserve leading `:` semantics in `selected_zone` (`L89–102`) so `:EST5EDT`, `:GMT0`, etc. are extracted as IANA zones instead of treated as POSIX rules.
     - Cache/check `tzdata` `(mtime, size)` metadata stamp alongside cached TZif so unchanged launches avoid reading 412 KiB `tzdata` repeatedly.
  8. `[MODIFY] crates/wrapper/src/maintenance.rs`:
     - Inspect `state_root` (as well as `generation_root`) in `process_references` so active `.local-update-*` staging dirs are protected.
     - Replace `root.exists()` (`L28`) with `fs::symlink_metadata(root).is_ok()` to catch dangling symlinks.
     - Prune orphaned `.hdr` files and bounded old temp residue safely.
  9. `[MODIFY] crates/wrapper/src/fs_util.rs`:
     - In `flock_with_timeout`, retry on `Interrupted` and propagate non-`WouldBlock` OS errors directly instead of masking them as timeouts; add unit tests for `fs_util.rs`.
- **Verification Gate:** `cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && (umask 0022 && cargo test --workspace --locked) && (umask 0077 && cargo test --workspace --locked)`.

---

### WP4 — Behavior-Preserving Structural Refactoring (Zero Observable Behavior Change)

- **Absolute Invariant:** **Every test passing at the end of WP3 MUST pass unmodified (except module path imports if strictly required for private helpers) with zero behavior or CLI output change.**
- **Scope:**
  1. Split `crates/wrapper/src/main.rs` into clean single-responsibility modules:
     - `[NEW] crates/wrapper/src/cli.rs`: unify the 4 argv scanners into one single-pass tokenizer (`UpstreamArgScan`) + dispatch/sandbox guard.
     - `[NEW] crates/wrapper/src/error.rs`: `LocalProductError`, `RuntimeLaunchError`, `PublicDispatchExecutionError`.
     - `[NEW] crates/wrapper/src/generation.rs`: `LocalWrapperRoots`, `GenerationManifest`, `LoadedLocalGeneration`, load/reconcile/verify/probe.
     - `[NEW] crates/wrapper/src/runtime_env.rs`: FD 33/34/35 (`RuntimeFdSources`), env plan, wrapper config, shared-home hook, `exec`.
     - `[NEW] crates/wrapper/src/wrapper_cli.rs`: `codex wrapper <status|setup|--bare|--with|--help>`.
     - `[MODIFY] crates/wrapper/src/main.rs`: slim entrypoint (~300–450 LOC) + top-level routing & integration tests.
  2. Split `crates/wrapper/src/update.rs`:
     - `[NEW] crates/wrapper/src/upstream_metadata.rs`: official GitHub/OpenAI metadata fetch, parsing, cross-check, version ordering.
     - `[NEW] crates/wrapper/src/progress.rs`: TTY detection, byte formatting, `UpdatePresentation`.
     - `[MODIFY] crates/wrapper/src/update.rs`: update/path/rollback orchestration.
  3. Unify duplicated internal primitives (zero new crates):
     - `[NEW] crates/wrapper/src/json.rs`: single bounded mini-JSON parser/serializer + `json_escape` (with UTF-16 surrogate pairs & strict hex validation) used by `daemon_settings.rs`, `upstream_metadata.rs`, `shared_home.rs`, `doctor.rs`, `wrapper_cli.rs`.
     - `[NEW] crates/wrapper/src/proc_exec.rs`: shared bounded child timeout / poll / process-group reap helpers used by `doctor.rs`, `qualification.rs`, `system_config.rs`, `remote_download.rs`.
     - `[MODIFY] crates/wrapper/src/fs_util.rs`: consolidate duplicated `flock`, owned-real-directory checks, and `rename_noreplace` across `state.rs`, `maintenance.rs`, `shared_home.rs`, `extensions.rs`, `generation.rs`.
  4. `[MODIFY] contrib/codex-account`: add `-h`/`--help`, accept suffix-free `codex-manager-profile` alongside `v1`/`v2`, ensure `sh -n` and `shellcheck` clean.
- **Verification Gate:** `cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && (umask 0022 && cargo test --workspace --locked) && (umask 0077 && cargo test --workspace --locked) && sh -n install.sh contrib/codex-account && shellcheck install.sh contrib/codex-account` + verify `Cargo.lock` still has strictly `['codex', 'codex-release-builder', 'libc']`.

---

### WP5 — CI/Release Gates, Qualification Harness & Final Doc Sync

- **Scope:**
  1. `[MODIFY] .github/scripts/publish_wrapper_release.py` & `test_publish_wrapper_release.py`: honor `--prerelease` / `--prerelease=false` when updating an existing GitHub Release (`gh release edit`) and readback-verify `isPrerelease`; add unit tests for prerelease transitions and mismatch rejection.
  2. `[MODIFY] .github/workflows/wrapper.yml`: ensure release publication requires `qualify_upstream: true` in `verify`.
  3. `[MODIFY] .github/workflows/wrapper-verify.yml`: lint `contrib/codex-account` (`sh -n` + `shellcheck`), remove dead `schedule` checks (`L581, L595`), wire `--previous-version` option where applicable.
  4. `[MODIFY] .github/scripts/qualify_wrapper.py`: remove `openssl` from required copied host tools (keep `sh`, `curl`, `gzip`, `tar`, `ps`); add `--previous-version` cross-version upgrade/rollback + Wrapper-only upgrade preserving `previous` rollback slot + `CODEX_SHARED_HOME` multi-profile rename-preservation & `auth.json`-isolation checks + read-only `doctor` assertion.
  5. `[MODIFY] .github/scripts/check_doc_links.py` & `test_check_doc_links.py`: verify every linked local target is Git-tracked (`git ls-files`) so untracked files fail locally just like CI; add negative unit test.
  6. `[MODIFY] docs/ARCHITECTURE.md` & `[MODIFY] README.md`: update module/file map to match WP4 split (`cli.rs`, `generation.rs`, `runtime_env.rs`, `wrapper_cli.rs`, `json.rs`, `proc_exec.rs`, `sha256.rs`, etc.) and update `README.md` user docs (`--force-rebuild`, `CODEX_SHARED_EXTRA_FILES`, removed `openssl-tool` runtime requirement, decoupled Wrapper upgrade preserving `previous`).
- **Verification Gate:** Full workspace lint/tests + `ruby` YAML parse of `.github/workflows/*.yml` + `sh -n` & `shellcheck` + `python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v` + `python3 .github/scripts/check_doc_links.py` + `git diff --check`.

---

### WP6 — Isolated Android API 37 Qualification & Final Checkpoint Report (Supervisor-Owned)

- **Scope (run by Supervisor in disposable `$TMPDIR/codex-stage1-qual-*`, zero live-device mutation):**
  1. Build `cargo build --release --locked -p codex -p codex-release-builder`.
  2. Run `qualify_wrapper.py --wrapper target/release/codex --root <isolated_root> --previous-version 0.169.1 --version 0.170.0`:
     - Verify `0.169.1` → `0.170.0` → `--rollback` → `0.170.0`, 16 KiB ELF geometry, daemon lifecycle, PTY, read-only `doctor`, `CODEX_SHARED_HOME` rename preservation & `auth.json` isolation.
  3. Verify offline Wrapper-only upgrade (`install.sh --binary ...` with slightly different Wrapper build) preserves `current=0.170.0` & `previous=0.169.1` without downloading, and `--force-rebuild` explicitly re-adapts.
  4. Benchmark `codex --version` cold-start latency vs baseline (~40 ms → ~21 ms target).
  5. Record exact dated evidence in [CURRENT.md](CURRENT.md), clean disposable root, confirm live `$PREFIX/bin/codex`, 3 daemons, and 5 `runit` services are untouched, and **stop before `git push` / `origin/main` promotion / GitHub Release** for final user sign-off.

---

## 4. Checkpoint Progress Tracker

| WP | Owner | Account / Model | Status | Commit | Notes |
| --- | --- | --- | --- | --- | --- |
| **WP0** | Supervisor | — | **Done** | `b7b48fca` | Pre-existing maintenance docs preserved verbatim |
| **Foundation** | Supervisor | — | **Done** | `e30bc790` + `HANDOFF.md` | Doc architecture consolidated, `SPEC.md` distilled, Phase A contracts updated |
| **WP1** | Codex + Review | `szob` (`gpt-6.2-sol` `xhigh`) | Pending | — | `release-builder`: patcher, TAR, companions, pure-Rust SHA-256 |
| **WP2** | Codex + Review | `szob` / `janmori101` | Pending | — | Wrapper/upstream decoupling, updater, HTTP, `install.sh` |
| **WP3** | Codex + Review | `szob` / `janmori101` | Pending | — | CLI, locks, `shared_home`, `doctor`, `qualification`, `daemon_settings`, `timezone`, `maintenance`, `fs_util` |
| **WP4** | Codex + Review | `szob` / `janmori101` | Pending | — | Behavior-preserving structural refactoring + `contrib/codex-account` |
| **WP5** | Codex + Review | `szob` / `janmori101` | Pending | — | CI/release gates, `qualify_wrapper.py`, `check_doc_links.py`, `ARCHITECTURE.md`/`README.md` sync |
| **WP6** | Supervisor | — | Pending | — | Isolated Android API 37 qualification + `CURRENT.md` proof → stop before push/release |
