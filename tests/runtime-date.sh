#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'runtime-date: FAIL: %s\n' "$*" >&2
    exit 1
}

date_text="$(
    printf '{"0.142.5":"2026-07-01T02:03:04.000Z"}' |
        PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
            python3 -B -m codex_termux.cli upstream-release-date --version 0.142.5
)"
[ "$date_text" = "2026-07-01" ] || fail "release date parse mismatch: $date_text"

printf 'runtime-date: ok\n'
