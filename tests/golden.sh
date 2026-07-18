#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/tools/golden_capture.py" \
    --validate-contract "$ROOT_DIR/config/migration-contracts.json"

for case_file in "$ROOT_DIR"/tests/golden/cases/*.json; do
    PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/tools/golden_capture.py" \
        --case "$case_file"
done

printf 'golden tests: ok\n'
