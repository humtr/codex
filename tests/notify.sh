#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-notify-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"

fail() {
    printf 'notify: FAIL: %s\n' "$*" >&2
    exit 1
}

run_notify() {
    local session="$1" message="$2"
    printf '{"session_id":"%s","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"%s"}' "$session" "$message" | \
        CODEX_TERMUX_HOME="$TMP_DIR/home" \
        CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
        CODEX_TERMUX_NOTIFY_NO_API=1 \
        bash "$ROOT_DIR/tools/codex-turn-notify.sh" --event Stop >/dev/null 2>&1
}

run_notify session-alpha first
log="$TMP_DIR/state/notify/notify.log"
[ -s "$log" ] || fail 'notify log was not written'
first_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ -n "$first_id" ] || fail 'notification id was not logged'
run_notify session-alpha second
second_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ "$first_id" = "$second_id" ] || fail 'same session did not reuse notification id'
run_notify session-beta third
third_id="$(sed -n 's/.* id=\([0-9][0-9]*\) .*/\1/p' "$log" | tail -n 1)"
[ "$third_id" != "$first_id" ] || fail 'different session reused notification id'

CONFIG_TMP="$TMP_DIR/config"
mkdir -p "$CONFIG_TMP/home" "$CONFIG_TMP/tmp"
CODEX_TERMUX_HOME="$CONFIG_TMP/home" \
CODEX_TERMUX_STATE_DIR="$CONFIG_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$CONFIG_TMP/tmp" \
CODEX_TERMUX_NOTIFY_PRETOOLUSE=1 \
    bash -lc '. "$1"; codex_prepare_system_config; ! grep -q "hooks.PreToolUse" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.Stop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"' \
        _ "$LIB_SH"

NOTIFY_TMP="$TMP_DIR/notify-cmd"
mkdir -p "$NOTIFY_TMP/home" "$NOTIFY_TMP/tmp"
CODEX_TERMUX_HOME="$NOTIFY_TMP/home" \
CODEX_TERMUX_STATE_DIR="$NOTIFY_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$NOTIFY_TMP/tmp" \
    bash -lc '. "$1"; codex_termux_main notify --channel both --hooks all --toast-gravity top --content-chars 0 --pretooluse 1 >/dev/null 2>&1; grep -q "CODEX_TERMUX_NOTIFY_CHANNEL=both" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "CODEX_TERMUX_NOTIFY_HOOKS=all" "$CODEX_TERMUX_NOTIFY_DIR/config.env"; grep -q "hooks.SessionStart" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.SubagentStop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"; grep -q "hooks.Stop" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"' \
        _ "$LIB_SH"

INVALID_TMP="$TMP_DIR/notify-invalid"
mkdir -p "$INVALID_TMP/home" "$INVALID_TMP/tmp"
CODEX_TERMUX_HOME="$INVALID_TMP/home" \
CODEX_TERMUX_STATE_DIR="$INVALID_TMP/home/.local/share/codex/termux" \
CODEX_TERMUX_TMPDIR="$INVALID_TMP/tmp" \
    bash -lc '. "$1"; ! codex_termux_main notify --hooks TypoHook >/dev/null 2>&1; ! codex_termux_main notify --toast-gravity center >/dev/null 2>&1; ! codex_termux_main notify --channel invalid >/dev/null 2>&1; ! codex_termux_main notify >/dev/null 2>&1; [ "$(wrapper_cmd notify-hook --action parse-selection --value "")" = "Stop" ]; [ "$(wrapper_cmd notify-hook --action parse-selection --value "1")" = "SessionStart" ]; ! wrapper_cmd notify-hook --action parse-selection --value "99" >/dev/null 2>&1' \
        _ "$LIB_SH"

PROVIDER_TMP="$TMP_DIR/provider"
mkdir -p "$PROVIDER_TMP/bin" "$PROVIDER_TMP/state/notify" "$PROVIDER_TMP/home" "$PROVIDER_TMP/tmp"
cat >"$PROVIDER_TMP/bin/termux-notification" <<'SH'
#!/bin/sh
printf 'notification\n' >>"$CODEX_PROVIDER_CALLS"
SH
cat >"$PROVIDER_TMP/bin/termux-toast" <<'SH'
#!/bin/sh
printf 'toast\n' >>"$CODEX_PROVIDER_CALLS"
SH
chmod 755 "$PROVIDER_TMP/bin/termux-notification" "$PROVIDER_TMP/bin/termux-toast"
cat >"$PROVIDER_TMP/state/notify/config.env" <<'ENV'
CODEX_TERMUX_NOTIFY_CHANNEL=both
CODEX_TERMUX_NOTIFY_HOOKS=Stop
CODEX_TERMUX_NOTIFY_CONTENT_CHARS=140
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES=0
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY=top
CODEX_TERMUX_NOTIFY_TOAST_SHORT=0
CODEX_TERMUX_NOTIFY_GROUP=codex-turns
ENV
printf '%s' '{"session_id":"provider-alpha","last_assistant_message":"provider response"}' | \
    env -i \
        HOME="$PROVIDER_TMP/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$PROVIDER_TMP/bin:$PATH" \
        CODEX_PROVIDER_CALLS="$PROVIDER_TMP/calls" \
        CODEX_TERMUX_HOME="$PROVIDER_TMP/home" \
        CODEX_TERMUX_STATE_DIR="$PROVIDER_TMP/state" \
        CODEX_TERMUX_NOTIFY_CONFIG="$PROVIDER_TMP/state/notify/config.env" \
        bash "$ROOT_DIR/tools/codex-turn-notify.sh" --event Stop >"$PROVIDER_TMP/out" 2>&1
grep -q '^notification$' "$PROVIDER_TMP/calls" || fail 'notification provider was not called'
grep -q '^toast$' "$PROVIDER_TMP/calls" || fail 'toast provider was not called'
if grep -q "$(printf '\a')" "$PROVIDER_TMP/out"; then
    fail 'bell fallback ran after providers succeeded'
fi

make_notification_stub() {
    local root="$1"
    cat >"$root/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >"$CODEX_NOTIFY_ARGS"
SH
    chmod 755 "$root/bin/termux-notification"
}

ATTACHED="$TMP_DIR/attached"
mkdir -p "$ATTACHED/bin" "$ATTACHED/state/notify" "$ATTACHED/home" "$ATTACHED/tmp"
cat >"$ATTACHED/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
  display-message) printf 'work:7.2\n' ;;
  has-session) [ "$3" = work ] ;;
  list-clients) printf 'client\n' ;;
  switch-client) exit 0 ;;
  *) exit 1 ;;
esac
SH
make_notification_stub "$ATTACHED"
chmod 755 "$ATTACHED/bin/tmux"
printf '%s' '{"title":"Task","content":"pane target check","tmux_target":"work:7.2","session_id":"pane-target-alpha"}' | \
    env -i \
        HOME="$ATTACHED/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$ATTACHED/bin:$PATH" \
        CODEX_NOTIFY_ARGS="$ATTACHED/notify.args" \
        CODEX_TERMUX_HOME="$ATTACHED/home" \
        CODEX_TERMUX_STATE_DIR="$ATTACHED/state" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        bash "$ROOT_DIR/tools/termux-notify.sh" >/dev/null 2>&1
grep -F -- '--action' "$ATTACHED/notify.args" >/dev/null || fail 'notification action missing'
grep -F -- 'libexec/notify open --target tmux --tmux-target work:7.2' "$ATTACHED/notify.args" >/dev/null \
    || fail 'allowlisted tmux action missing'

OUTSIDE="$TMP_DIR/outside"
mkdir -p "$OUTSIDE/bin" "$OUTSIDE/state/notify" "$OUTSIDE/home" "$OUTSIDE/tmp"
make_notification_stub "$OUTSIDE"
printf '%s' '{"title":"Task","content":"outside check","session_id":"outside-alpha"}' | \
    env -i \
        HOME="$OUTSIDE/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$OUTSIDE/bin:$PATH" \
        CODEX_NOTIFY_ARGS="$OUTSIDE/notify.args" \
        CODEX_TERMUX_HOME="$OUTSIDE/home" \
        CODEX_TERMUX_STATE_DIR="$OUTSIDE/state" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        bash "$ROOT_DIR/tools/termux-notify.sh" >/dev/null 2>&1
grep -F -- 'libexec/notify open --target termux' "$OUTSIDE/notify.args" >/dev/null \
    || fail 'non-tmux notification did not use Termux action'

DETACHED="$TMP_DIR/detached"
mkdir -p "$DETACHED/bin" "$DETACHED/state/notify" "$DETACHED/home" "$DETACHED/tmp"
cat >"$DETACHED/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
  has-session) exit 0 ;;
  list-clients) exit 1 ;;
  *) exit 1 ;;
esac
SH
make_notification_stub "$DETACHED"
chmod 755 "$DETACHED/bin/tmux"
printf '%s' '{"title":"Task","content":"detached","tmux_target":"ghost:2.1","session_id":"detached-alpha"}' | \
    env -i \
        HOME="$DETACHED/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$DETACHED/bin:$PATH" \
        CODEX_NOTIFY_ARGS="$DETACHED/notify.args" \
        CODEX_TERMUX_HOME="$DETACHED/home" \
        CODEX_TERMUX_STATE_DIR="$DETACHED/state" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        bash "$ROOT_DIR/tools/termux-notify.sh" >/dev/null 2>&1
grep -F -- 'libexec/notify open --target termux' "$DETACHED/notify.args" >/dev/null \
    || fail 'detached tmux target did not fall back to Termux action'

printf 'notify: ok\n'
