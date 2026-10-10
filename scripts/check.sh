#!/bin/sh
# Select a gate deliberately; compiler output is reusable, test roots are not.
set -eu
scope=${1:-full}
case "$scope" in docs|python|rust|full) ;; *) echo 'usage: scripts/check.sh [docs|python|rust|full]' >&2; exit 2 ;; esac
test "$#" -le 1 || { echo 'unexpected arguments' >&2; exit 2; }
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export PYTHONDONTWRITEBYTECODE=1
if [ "$scope" = rust ] || [ "$scope" = full ]; then
  : "${CARGO_TARGET_DIR:=${XDG_CACHE_HOME:-$HOME/.cache}/codex/check-target}"
  export CARGO_TARGET_DIR
  cargo fmt --all -- --check
fi
if [ "$scope" = python ] || [ "$scope" = full ]; then
  python3 -m unittest -v scripts/test_shared_state_migrate.py scripts/test_shared_visibility_transition.py scripts/test_check.py scripts/test_ci_scope.py
  python3 -m unittest discover -s .github/scripts -p 'test_*.py'
fi
if [ "$scope" = rust ] || [ "$scope" = full ]; then
  # /proc reference tests must not observe unrelated parallel test processes.
  cargo test --workspace --locked -- --test-threads=1
  cargo clippy --workspace --all-targets --locked -- -D warnings
fi
git diff --check
