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

display_date="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli display-runtime-date --value 20260702T030405Z
)"
[ "$display_date" = "2026-07-02" ] || fail "display date mismatch: $display_date"

package_spec="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli package-spec \
            --requested 0.142.5 \
            --default @openai/codex@latest-linux-arm64
)"
[ "$package_spec" = "@openai/codex@0.142.5-linux-arm64" ] || fail "package spec mismatch: $package_spec"

explicit_package_spec="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli package-spec \
            --requested @openai/codex@0.142.5-linux-arm64 \
            --default @openai/codex@latest-linux-arm64
)"
[ "$explicit_package_spec" = "@openai/codex@0.142.5-linux-arm64" ] || fail "explicit package spec mismatch: $explicit_package_spec"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-retention-ok --value 3 \
    || fail 'runtime retention rejected positive integer'

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-retention-ok --value 0
then
    fail 'runtime retention accepted zero'
fi

mode="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-mode --mode always
)"
[ "$mode" = "force" ] || fail "auto-update mode mismatch: $mode"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli auto-update-due \
        --enabled 1 --mode prompt --now 100 --last 10 --interval 60 \
    || fail 'auto-update due rejected due update'

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli auto-update-due \
        --enabled 1 --mode off --now 100 --last 10 --interval 60
then
    fail 'auto-update due accepted off mode'
fi

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli failed-auto-update-due \
        --record $'0.142.4\t95' --version 0.142.4 --now 100 --interval 60
then
    fail 'failed auto-update retry accepted too early'
fi

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli failed-auto-update-due \
        --record $'0.142.4\t30' --version 0.142.4 --now 100 --interval 60 \
    || fail 'failed auto-update retry rejected due retry'

printf 'runtime-date: ok\n'
