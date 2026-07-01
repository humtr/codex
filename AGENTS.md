# Agent Rules for humtr/codex

This repository is the Termux Codex wrapper product. Keep this file small and
product-local.

## Branch policy

- Do not push directly to `main` unless the user explicitly asks for a final merge or hotfix.
- Do not force-push shared branches unless the user explicitly authorizes it.
- Do not add non-wrapper orchestration material to this repository.

## Wrapper contracts

- Preserve the public `codex` launcher behavior.
- Preserve `codex termux` command compatibility unless a breaking change is explicitly requested.
- Preserve Termux runtime, install, launcher, repair, rollback, registry, and protected path contracts.
- Keep `bin/install-runtime.sh` and `lib/codex-termux*.sh` changes narrow and test-backed.

## Product Validation

- Add regression tests for every real smoke failure that reaches a product branch.
- Use focused tests for narrow wrapper changes and `tests/run-all.sh` before
  release or risky installer/runtime changes.
