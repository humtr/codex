#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Run portable checks before Termux-specific checks.
bash "$ROOT_DIR/tests/run-portable.sh"
bash "$ROOT_DIR/tests/run-termux.sh"

printf 'tests: all ok\n'
