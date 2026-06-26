#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="${TMPDIR:-/tmp}/codex-notify-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'notify: FAIL: %s\n' "$*" >&2
    exit 1
}

run_notify() {
    CODEX_TERMUX_HOME="$TMP_DIR/home" \
    CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
    CODEX_TERMUX_NOTIFY_NO_API=1 \
    CODEX_TERMUX_NOTIFY_PRETOOLUSE=1 \
    bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON'
{"session_id":"session-alpha","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"first response"}
JSON
}

run_notify >/dev/null 2>&1
log="$TMP_DIR/state/notify/notify.log"
[ -s "$log" ] || fail 'notify log was not written'
first_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ -n "$first_id" ] || fail 'notification id was not logged'

run_notify >/dev/null 2>&1
second_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ "$first_id" = "$second_id" ] || fail 'same session did not reuse notification id'

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_NOTIFY_NO_API=1 \
CODEX_TERMUX_NOTIFY_PRETOOLUSE=1 \
bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON' >/dev/null 2>&1
{"session_id":"session-beta","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"second response"}
JSON

third_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ "$third_id" != "$first_id" ] || fail 'different session reused notification id'

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_NOTIFY_NO_API=1 \
CODEX_TERMUX_NOTIFY_PRETOOLUSE=1 \
bash "$ROOT_DIR/tools/codex-turn-notify.sh" --event PreToolUse <<'JSON' >/dev/null 2>&1
{"session_id":"session-gamma","cwd":"/data/data/com.termux/files/home/prj/codex","message":"tool is starting"}
JSON

fourth_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ -n "$fourth_id" ] || fail 'pretooluse notification id was not logged'

printf 'notify: ok\n'

CONFIG_TMP="$TMP_DIR/config"
mkdir -p "$CONFIG_TMP/home" "$CONFIG_TMP/tmp"
CODEX_TERMUX_HOME="$CONFIG_TMP/home" \
CODEX_TERMUX_STATE_DIR="$CONFIG_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$CONFIG_TMP/tmp" \
CODEX_TERMUX_NOTIFY_PRETOOLUSE=1 \
bash -lc '. /data/data/com.termux/files/home/prj/codex/lib/codex-termux.sh; codex_prepare_system_config; grep -q "hooks.PreToolUse" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.Stop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"'
