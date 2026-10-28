#!/bin/sh
# Select a gate deliberately; compiler output is reusable, test roots are not.
set -eu
scope=${1:-full}
case "$scope" in docs|python|rust|provider|full) ;; *) echo 'usage: scripts/check.sh [docs|python|rust|provider|full]' >&2; exit 2 ;; esac
test "$#" -le 1 || { echo 'unexpected arguments' >&2; exit 2; }
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export PYTHONDONTWRITEBYTECODE=1
if [ "$scope" = rust ] || [ "$scope" = full ]; then
  : "${CARGO_TARGET_DIR:=${XDG_CACHE_HOME:-$HOME/.cache}/codex/check-target}"
  export CARGO_TARGET_DIR
  cargo fmt --all -- --check
fi
if [ "$scope" = python ] || [ "$scope" = full ]; then
  python3 -m unittest -v scripts/test_shared_state_migrate.py scripts/test_shared_visibility_transition.py scripts/test_check.py scripts/test_ci_scope.py scripts/test_build_tunnel_client.py scripts/test_verify_chat_native_text.py scripts/test_prepare_runtime_source.py
  python3 -m unittest discover -s .github/scripts -p 'test_*.py'
  python3 -m unittest discover -s frontend/manager-tui -p 'test_*.py'
fi
if [ "$scope" = rust ] || [ "$scope" = full ]; then
  # /proc reference tests must not observe unrelated parallel test processes.
  cargo test --workspace --locked -- --test-threads=1
  cargo clippy --workspace --all-targets --locked -- -D warnings
fi
if [ "$scope" = provider ]; then
  : "${CARGO_TARGET_DIR:=${XDG_CACHE_HOME:-$HOME/.cache}/codex/provider-target}"
  export CARGO_TARGET_DIR
  cargo fmt --manifest-path providers/chatgpt/Cargo.toml --all -- --check
  cargo test --manifest-path providers/chatgpt/Cargo.toml --locked
  cargo clippy --manifest-path providers/chatgpt/Cargo.toml --all-targets --locked -- -D warnings
  cargo build --manifest-path providers/chatgpt/Cargo.toml --release --locked
  : "${BUN:?set BUN to the qualified optional-provider Bun executable}"
  "$BUN" test ./providers/chatgpt/*.test.mjs ./scripts/f_cdp_relay.test.mjs ./scripts/chat_entry_client.test.mjs
  python3 -B -m unittest discover -s providers/chatgpt -p 'test_*.py'
  python3 -B -m unittest -v scripts/test_provider_qualification.py scripts/test_d3_notify.py scripts/test_provider_f.py scripts/test_provider_f_agents.py
  python3 -B -m unittest -v scripts/test_provider_g.py scripts/test_qualification_diagnostics.py
  # Import/CLI checks only; device fixtures and cleanup never run in source CI.
  python3 -B scripts/clean_development.py --help >/dev/null
  python3 -B scripts/qualify_provider_product.py --help >/dev/null
  python3 -B scripts/qualify_provider_package.py --help >/dev/null
  python3 -B scripts/qualify_provider_g.py --help >/dev/null
  python3 -B scripts/qualify_managed_native_lifecycle.py --help >/dev/null
  python3 -B scripts/release_g_observer.py --help >/dev/null
fi
git diff --check
