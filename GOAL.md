# Goal: Python Boundary Expansion for the Termux Codex Wrapper

## Objective

Complete the remaining shell/Python responsibility split for the Termux Codex
wrapper while keeping this repository product-only, thin, and free of generic
automation.

The target shape is:

- shell owns Termux process execution, file mutation, locking, fd wiring, and
  public launcher dispatch.
- Python owns diagnosis, plans, structured state interpretation, validation,
  source resolution, and policy decisions.
- no GitHub automation, handoff loop, API orchestration, or generic workflow
  material returns to this repository.

## Current Baseline

- Branch: `refactor/python-boundary-expansion`
- Base commit: `0f9b5316b1f3` (`Move repair diagnosis into Python`)
- Phase 3/4/5 have already been merged into `main`.
- Current installed wrapper baseline after Phase 5 was validated as
  `260701-9 (0f9b5316b1f3)`.
- Existing proof:
  - `validate --root .`: pass
  - `canon-audit --strict`: pass
  - `tests/run-portable.sh`: pass
  - `tests/run-termux.sh` with rebuild smoke: pass
  - `codex termux repair`: pass
  - `tests/run-all.sh`: pass

## Hard Boundaries

### Repository Boundary

- Keep `humtr/codex` product-only.
- Do not add generic automation, merge-readiness workflows, agent handoff,
  OpenAI API orchestration, GitHub API orchestration, local-server automation,
  or reusable loop framework material.
- Do not add or restore `.github/workflows/*` unless the user explicitly changes
  this goal.
- Do not add non-wrapper docs, schemas, templates, or playbooks.

### Network Boundary

This goal must be executed without using network-dependent work.

Do not run:

- `git fetch`, `git pull`, `git push`, or any remote branch operation
- `gh`, GitHub API, OpenAI API, curl/wget network calls
- upstream package downloads or network update checks
- npm registry queries

Allowed:

- local git operations only: branch, diff, status, log, add, commit, reset
  of local work when needed
- local tests and local Termux smoke
- cached raw runtime rebuilds only
- local checkout support install

When running product smoke, disable opportunistic network checks where relevant
with local environment controls such as `CODEX_TERMUX_AUTO_UPDATE=0`.

### Commit Boundary

- Use local git checkpoint commits freely.
- Do not push any branch during this goal.
- Keep checkpoint commits phase-scoped and reversible.
- At completion, leave either:
  - a clean stack of local checkpoint commits, or
  - a local squash-ready final branch, depending on user instruction.

### Product Contract Boundary

- Preserve public `codex` launcher behavior.
- Preserve `codex termux` command compatibility unless the user explicitly asks
  for a breaking change.
- Preserve runtime, install, launcher, repair, rollback, registry, and protected
  path contracts.
- Preserve default profile behavior: do not force `CODEX_HOME` for the default
  profile.
- Preserve custom profile isolation.
- Preserve managed launcher and runtime artifact paths.

## Execution Plan

### Phase 5-B: Runtime Readiness Diagnosis

Unify `codex_ensure_runtime_ready` with the Python diagnosis/action model.

Expected changes:

- Extend the repair/runtime diagnosis model so normal runtime readiness and
  manual repair share the same structured checks.
- Replace shell-side readiness branching with Python-selected actions such as:
  - `ready`
  - `refresh_metadata`
  - `restore_verified`
  - `rebuild_cached`
  - `missing_runtime`
  - `raw_corrupt`
- Keep shell responsible for executing rollback, cached rebuild, and runtime
  exec wiring.

Acceptance:

- No new shell global readiness flags.
- `codex_ensure_runtime_ready` delegates diagnosis decisions to Python.
- Regression tests cover ready, metadata refresh, rollback, rebuild, and
  unrecoverable paths.

### Phase 6: Install/Rebuild/Update Plan Model

Move install/rebuild/update decision planning into Python while preserving shell
execution.

Expected changes:

- Add an install plan module for support, rebuild, upstream, and update
  scenarios.
- Python owns mode validation, option-like version rejection, source requirement
  decisions, and action selection.
- Shell continues to copy files, invoke builders, install support, and run
  activation.

Acceptance:

- Existing install dispatch contracts still pass.
- Branch/local checkout rebuild smoke does not accidentally resolve to `main`.
- No network update or upstream install is required for validation.

### Phase 7: Wrapper Source Resolution Model

Move wrapper source config interpretation and source resolution policy further
into Python.

Expected changes:

- Python owns wrapper source config parsing, local checkout validation, fallback
  policy, and source-root validity checks.
- Shell keeps only local process/file operations needed to prepare and clean
  temporary source directories.

Acceptance:

- `tests/wrapper-source-config.sh` remains passing.
- Source resolution behavior is covered for local checkout, config reuse,
  invalid source, and branch smoke override cases.

### Phase 8: Notify Configuration and Option Parsing

Move notify option normalization and config rendering into Python.

Expected changes:

- Add or extend a notify Python module for hook normalization, channel/options
  validation, and config rendering.
- Shell remains responsible for public command dispatch and actual file write.

Acceptance:

- `tests/notify.sh` remains passing and gains coverage for Python-owned option
  normalization.
- No runtime or install code is changed for notify-only behavior.

### Phase 9: Profile and Use Boundary Tightening

Reduce shell-side profile/use state decisions without changing user behavior.

Expected changes:

- Move remaining profile selection/display/recent-state decisions into Python
  where practical.
- Keep shell responsible for interactive prompts and final runtime exec.

Acceptance:

- Default profile still does not force `CODEX_HOME`.
- Custom profile still uses explicit profile home.
- Session/profile state bleed tests remain passing or are strengthened.

### Phase 10: Manifest-Driven Shell Budget and Regression Guards

Make the thin-wrapper target measurable.

Expected changes:

- Extend `codex-wrapper.manifest.json` or `canon-audit` with product-local shell
  budget and ownership checks.
- Reject reintroduction of shell global decision flags.
- Reject shell-side JSON parsing when a Python helper should own it.
- Reject new private cross-domain calls unless explicitly justified.

Acceptance:

- `canon-audit --strict` reports `status: ok` and zero findings.
- Shell budget thresholds are realistic but stricter than the current baseline.
- The audit protects the direction of travel without blocking legitimate
  Termux execution glue.

### Phase 11: Runtime Action Vocabulary Consolidation

Use a coherent action vocabulary across runtime readiness, repair, rollback,
rebuild, and activation-adjacent flows.

Expected changes:

- Consolidate repeated action names and meanings in Python.
- Keep shell `case` blocks small and execution-only.
- Avoid duplicating the same runtime checks across shell functions.

Acceptance:

- Runtime action names are documented in code or tests.
- Repair and readiness decisions do not diverge silently.
- Existing runtime/rollback/store tests still pass.

### Phase 12: Final Shell Reduction and Product Proof

Complete the boundary expansion with quantitative proof.

Expected changes:

- Reduce shell decision density in `bin/install-runtime.sh` and
  `lib/codex-termux/runtime.sh`.
- Keep shell where it is the right tool: fd wiring, exec, chmod/cp/mv/link,
  traps, locks, and Termux process environment.
- Preserve product-only repo purity.

Acceptance:

- `bin/install-runtime.sh` line count decreases or its remaining shell logic is
  explicitly justified.
- Domain shell total does not grow without a stated reason.
- No generic automation or workflow material is introduced.
- Local installed wrapper matches the final local commit.
- Final validation passes.

## Required Validation

Run focused tests after each phase-local change, then full validation before
declaring completion.

Baseline commands:

```bash
bash -n install.sh bin/install-local.sh bin/install-runtime.sh \
  lib/codex-termux.sh lib/codex-termux/*.sh \
  tools/smoke-termux-wrapper.sh tools/package-release.sh tests/*.sh

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools \
  python3 -B -m codex_termux.cli validate --root .

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools \
  python3 -B -m codex_termux.cli canon-audit --root . --strict

PYTHONDONTWRITEBYTECODE=1 bash tests/run-portable.sh
```

Live local product proof:

```bash
bash bin/install-local.sh support
CODEX_TERMUX_AUTO_UPDATE=0 PYTHONDONTWRITEBYTECODE=1 bash tests/run-termux.sh
CODEX_TERMUX_AUTO_UPDATE=0 PYTHONDONTWRITEBYTECODE=1 bash tests/run-all.sh
```

Optional cached rebuild proof when runtime paths are touched:

```bash
CODEX_TERMUX_AUTO_UPDATE=0 CODEX_TERMUX_RUN_REBUILD_SMOKE=1 \
  PYTHONDONTWRITEBYTECODE=1 bash tests/run-termux.sh
```

## Acceptance Ledger

- [x] Phase 5-B runtime readiness diagnosis implemented.
- [x] Phase 6 install/rebuild/update plan model implemented.
- [x] Phase 7 wrapper source resolution model implemented.
- [x] Phase 8 notify option/config boundary tightened.
- [x] Phase 9 profile/use boundary tightened.
- [x] Phase 10 shell budget and regression guards implemented.
- [x] Phase 11 runtime action vocabulary consolidated.
- [x] Phase 12 final shell reduction and product proof completed.
- [x] No network-dependent command was used.
- [x] No branch was pushed.
- [x] Only local checkpoint commits were created.
- [x] Product repo remained free of generic automation.
- [x] Final working tree is clean or intentionally left with documented local
  changes.
- [x] Final validation results are recorded in this file before completion.

## Validation Ledger

Last completed local-only validation on `refactor/python-boundary-expansion`:

- `bash -n install.sh bin/install-local.sh bin/install-runtime.sh lib/codex-termux.sh lib/codex-termux/*.sh tools/smoke-termux-wrapper.sh tools/package-release.sh tests/*.sh`: pass
- `PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B -m codex_termux.cli validate --root .`: pass
- `PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B -m codex_termux.cli canon-audit --root . --strict`: pass
  - findings: `[]`
  - `install_runtime_lines`: 626
  - `domain_shell_lines`: 2621
  - `notify_shell_functions`: 20
  - `profile_shell_functions`: 16
- `PYTHONDONTWRITEBYTECODE=1 bash tests/run-portable.sh`: pass

Live local product proof on the installed Termux wrapper:

- `CODEX_TERMUX_AUTO_UPDATE=0 bash bin/install-local.sh support`: pass
  - installed wrapper matched the local checkpoint under test.
- `CODEX_TERMUX_AUTO_UPDATE=0 PYTHONDONTWRITEBYTECODE=1 bash tests/run-termux.sh`: pass
- `CODEX_TERMUX_AUTO_UPDATE=0 CODEX_TERMUX_RUN_REBUILD_SMOKE=1 PYTHONDONTWRITEBYTECODE=1 bash tests/run-termux.sh`: pass
  - cached rebuild used local wrapper source.
  - doctor `overallStatus`: `ok`
- `CODEX_TERMUX_AUTO_UPDATE=0 PYTHONDONTWRITEBYTECODE=1 bash tests/run-all.sh`: pass

## Not Proven Yet

None for this goal. The branch remains a local checkpoint stack and has not
been pushed.

## Resume Notes

The goal is complete locally. The branch was intentionally left unpushed per
the network boundary. Next work should start from the final local checkpoint
commit on `refactor/python-boundary-expansion`.
