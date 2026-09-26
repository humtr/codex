#!/bin/sh
set -eu

if [ -n "${TDEV_ENV_DIR:-}" ]; then
  CARGO_TARGET_DIR="$TDEV_ENV_DIR/cargo-target-check"
  export CARGO_TARGET_DIR
  cleanup_target=false
else
  check_tmp=$(mktemp -d)
  CARGO_TARGET_DIR="$check_tmp/target"
  export CARGO_TARGET_DIR
  cleanup_target=true
fi

cleanup() {
  if [ "${cleanup_target}" = "true" ]; then
    rm -rf "$check_tmp"
  fi
  rm -rf scripts/__pycache__
}
trap cleanup EXIT HUP INT TERM

export PYTHONDONTWRITEBYTECODE=1

cargo fmt --all -- --check
python3 -m unittest -v scripts/test_shared_state_migrate.py
cargo test --workspace

# Historical .github RALD tests intentionally pin superseded release/workflow SHAs;
# current product acceptance is carried by the workspace tests above.
git diff --check
