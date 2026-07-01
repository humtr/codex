# Agent Rules for humtr/codex

This repository can be edited by human maintainers and assisted coding agents. Keep this file small: it is a repo-local safety anchor, not an iteration log.

## Branch policy

- Do not push directly to `main` unless the user explicitly asks for a final merge or hotfix.
- Do not merge or copy from `handoff/*` branches into product branches. Handoff branches are communication artifacts only.
- Do not force-push shared branches unless the user explicitly authorizes it.
- Do not modify `automation/canon-snapshot-index` as part of Termux wrapper work.

## Wrapper contracts

- Preserve the public `codex` launcher behavior.
- Preserve `codex termux` command compatibility unless a breaking change is explicitly requested.
- Preserve Termux runtime, install, launcher, repair, rollback, registry, and protected path contracts.
- Keep `bin/install-runtime.sh` and `lib/codex-termux*.sh` changes narrow and test-backed.

## Validation tiers

Use validation proportional to risk.

- Chat loop: static review, targeted file checks, and focused tests. Do not require `tests/run-all.sh` for every small edit.
- Checkpoint: shell syntax, canon audit, and focused wrapper tests.
- Final gate: `tests/run-all.sh`, merge-readiness, and Termux smoke when runtime/install/launcher paths are touched.

## Agent roles

- ChatGPT owns planning, code review, small patches, and loop bookkeeping.
- Codex or local execution should be used for heavier shell/runtime validation.
- The human maintainer owns final merge approval and real Termux smoke output.

## Operating notes

- Keep transient loop state in PR comments, workflow artifacts, or handoff-only branches, not in product code.
- Add regression tests for every real smoke failure that reaches a product branch.
