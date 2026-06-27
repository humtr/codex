#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for test_script in \
    "$ROOT_DIR/tests/invariants.sh" \
    "$ROOT_DIR/tests/runtime-build.sh" \
    "$ROOT_DIR/tests/package-safety.sh" \
    "$ROOT_DIR/tests/tmp-paths.sh" \
    "$ROOT_DIR/tests/lock.sh" \
    "$ROOT_DIR/tests/install-dispatch.sh" \
    "$ROOT_DIR/tests/cli-surface.sh" \
    "$ROOT_DIR/tests/notify.sh" \
    "$ROOT_DIR/tests/store-rollback.sh" \
    "$ROOT_DIR/tests/session.sh"
do
    bash "$test_script"
done

printf 'tests: ok\n'
