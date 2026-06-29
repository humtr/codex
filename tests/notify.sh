#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-notify-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"

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
bash -lc '. "$1"; codex_prepare_system_config; ! grep -q "hooks.PreToolUse" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.Stop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"' _ "$LIB_SH"

NOTIFY_TMP="$TMP_DIR/notify-cmd"
mkdir -p "$NOTIFY_TMP/home" "$NOTIFY_TMP/tmp"
CODEX_TERMUX_HOME="$NOTIFY_TMP/home" \
CODEX_TERMUX_STATE_DIR="$NOTIFY_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$NOTIFY_TMP/tmp" \
bash -lc '. "$1"; codex_termux_main notify --channel both --hooks all --toast-gravity top --content-chars 0 --pretooluse 1 >/dev/null 2>&1; grep -q "CODEX_TERMUX_NOTIFY_CHANNEL=both" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "CODEX_TERMUX_NOTIFY_HOOKS=all" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; ! grep -q "CODEX_TERMUX_NOTIFY_TOAST=" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; ! grep -q "CODEX_TERMUX_NOTIFY_NOTIFICATION=" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "hooks.SessionStart" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.SubagentStop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.Stop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"' _ "$LIB_SH"

INVALID_TMP="$TMP_DIR/notify-invalid"
mkdir -p "$INVALID_TMP/home" "$INVALID_TMP/tmp"
CODEX_TERMUX_HOME="$INVALID_TMP/home" \
CODEX_TERMUX_STATE_DIR="$INVALID_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$INVALID_TMP/tmp" \
bash -lc '. "$1"; ! codex_termux_main notify --hooks TypoHook >/dev/null 2>&1; ! codex_termux_main notify --toast-gravity center >/dev/null 2>&1; ! codex_termux_main notify --channel invalid >/dev/null 2>&1; ! codex_termux_main notify >/dev/null 2>&1; ! codex_termux_main toast >/dev/null 2>&1; [ "$(codex_notify_parse_hook_selection "")" = "Stop" ]; [ "$(codex_notify_parse_hook_selection "1")" = "SessionStart" ]; ! codex_notify_parse_hook_selection "99" >/dev/null 2>&1; ! codex_notify_parse_hook_selection "1abc" >/dev/null 2>&1' _ "$LIB_SH"

PROVIDER_TMP="$TMP_DIR/provider"
mkdir -p "$PROVIDER_TMP/bin" "$PROVIDER_TMP/state/notify"
cat >"$PROVIDER_TMP/bin/termux-notification" <<'SH'
#!/bin/sh
printf 'notification\n' >>"$CODEX_PROVIDER_CALLS"
SH
cat >"$PROVIDER_TMP/bin/termux-toast" <<'SH'
#!/bin/sh
printf 'toast\n' >>"$CODEX_PROVIDER_CALLS"
SH
chmod +x "$PROVIDER_TMP/bin/termux-notification" "$PROVIDER_TMP/bin/termux-toast"
cat >"$PROVIDER_TMP/state/notify/config.env" <<'ENV'
CODEX_TERMUX_NOTIFY_CHANNEL=both
CODEX_TERMUX_NOTIFY_HOOKS=Stop
CODEX_TERMUX_NOTIFY_CONTENT_CHARS=140
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES=0
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top
CODEX_TERMUX_NOTIFY_TOAST_SHORT=0
CODEX_TERMUX_NOTIFY_GROUP=codex-turns
ENV
env -i \
    CODEX_PROVIDER_CALLS="$PROVIDER_TMP/calls" \
    CODEX_TERMUX_HOME="$PROVIDER_TMP/home" \
    CODEX_TERMUX_STATE_DIR="$PROVIDER_TMP/state" \
    CODEX_TERMUX_NOTIFY_CONFIG="$PROVIDER_TMP/state/notify/config.env" \
    PATH="$PROVIDER_TMP/bin:$PATH" \
    bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON' >"$PROVIDER_TMP/out" 2>&1
{"session_id":"provider-alpha","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"provider response"}
JSON
grep -q '^notification$' "$PROVIDER_TMP/calls" || fail 'notification provider was not called'
grep -q '^toast$' "$PROVIDER_TMP/calls" || fail 'toast provider was not called'
if grep -q "$(printf '\a')" "$PROVIDER_TMP/out"; then
    fail 'bell fallback ran after termux-api providers succeeded'
fi
