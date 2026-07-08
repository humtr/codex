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
bash -lc '. "$1"; ! codex_termux_main notify --hooks TypoHook >/dev/null 2>&1; ! codex_termux_main notify --toast-gravity center >/dev/null 2>&1; ! codex_termux_main notify --channel invalid >/dev/null 2>&1; ! codex_termux_main notify >/dev/null 2>&1; ! codex_termux_main toast >/dev/null 2>&1; [ "$(codex_termux_cmd notify-hook --action parse-selection --value "")" = "Stop" ]; [ "$(codex_termux_cmd notify-hook --action parse-selection --value "1")" = "SessionStart" ]; ! codex_termux_cmd notify-hook --action parse-selection --value "99" >/dev/null 2>&1; ! codex_termux_cmd notify-hook --action parse-selection --value "1abc" >/dev/null 2>&1' _ "$LIB_SH"

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

TMUX_TARGET_TMP="$TMP_DIR/tmux-target"
mkdir -p "$TMUX_TARGET_TMP/bin" "$TMUX_TARGET_TMP/state/notify"
cat >"$TMUX_TARGET_TMP/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
    display-message)
        [ "$2" = "-p" ] || exit 2
        [ "$3" = "#S:#I.#P" ] || exit 3
        printf 'work:7.2\n'
        ;;
    has-session)
        [ "$2" = "-t" ] || exit 4
        [ "$3" = "work" ] || exit 5
        ;;
    list-clients)
        [ "$2" = "-t" ] || exit 7
        [ "$3" = "work" ] || exit 8
        printf '/dev/pts/9: work [120x40]\n'
        ;;
    *)
        exit 6
        ;;
esac
SH
cat >"$TMUX_TARGET_TMP/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >"$CODEX_NOTIFY_ARGS"
SH
chmod +x "$TMUX_TARGET_TMP/bin/tmux" "$TMUX_TARGET_TMP/bin/termux-notification"
env -i \
    CODEX_NOTIFY_ARGS="$TMUX_TARGET_TMP/notify.args" \
    CODEX_TERMUX_HOME="$TMUX_TARGET_TMP/home" \
    CODEX_TERMUX_STATE_DIR="$TMUX_TARGET_TMP/state" \
    CODEX_TERMUX_NOTIFY_CHANNEL=notification \
    TMUX="/tmp/tmux-123/default,999,0" \
    PATH="$TMUX_TARGET_TMP/bin:$PATH" \
    bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON' >/dev/null 2>&1
{"session_id":"pane-target-alpha","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"pane target check"}
JSON
grep -F -- "--action" "$TMUX_TARGET_TMP/notify.args" >/dev/null \
    || fail 'notification action missing'
grep -F -- "--open-tmux work:7.2" "$TMUX_TARGET_TMP/notify.args" >/dev/null \
    || fail 'notification action did not target hook pane'

TMUX_OUTSIDE_TMP="$TMP_DIR/tmux-outside"
mkdir -p "$TMUX_OUTSIDE_TMP/bin" "$TMUX_OUTSIDE_TMP/state/notify"
cat >"$TMUX_OUTSIDE_TMP/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
    display-message)
        [ "$2" = "-p" ] || exit 2
        printf 'wrong:1.0\n'
        ;;
    has-session)
        [ "$2" = "-t" ] || exit 3
        [ "$3" = "wrong" ] || exit 4
        ;;
    *)
        exit 5
        ;;
esac
SH
cat >"$TMUX_OUTSIDE_TMP/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >"$CODEX_NOTIFY_ARGS"
SH
chmod +x "$TMUX_OUTSIDE_TMP/bin/tmux" "$TMUX_OUTSIDE_TMP/bin/termux-notification"
env -i \
    CODEX_NOTIFY_ARGS="$TMUX_OUTSIDE_TMP/notify.args" \
    CODEX_TERMUX_HOME="$TMUX_OUTSIDE_TMP/home" \
    CODEX_TERMUX_STATE_DIR="$TMUX_OUTSIDE_TMP/state" \
    CODEX_TERMUX_NOTIFY_CHANNEL=notification \
    PATH="$TMUX_OUTSIDE_TMP/bin:$PATH" \
    bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON' >/dev/null 2>&1
{"session_id":"outside-alpha","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"outside tmux check"}
JSON
grep -F -- "--open-termux" "$TMUX_OUTSIDE_TMP/notify.args" >/dev/null \
    || fail 'non-tmux hook should not deep-link to tmux target'

TMUX_DETACHED_TMP="$TMP_DIR/tmux-detached"
mkdir -p "$TMUX_DETACHED_TMP/bin" "$TMUX_DETACHED_TMP/state/notify"
cat >"$TMUX_DETACHED_TMP/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
    display-message)
        [ "$2" = "-p" ] || exit 2
        [ "$3" = "#S:#I.#P" ] || exit 3
        printf 'ghost:2.1\n'
        ;;
    list-clients)
        [ "$2" = "-t" ] || exit 4
        [ "$3" = "ghost" ] || exit 5
        exit 1
        ;;
    has-session)
        [ "$2" = "-t" ] || exit 6
        [ "$3" = "ghost" ] || exit 7
        ;;
    *)
        exit 8
        ;;
esac
SH
cat >"$TMUX_DETACHED_TMP/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >"$CODEX_NOTIFY_ARGS"
SH
chmod +x "$TMUX_DETACHED_TMP/bin/tmux" "$TMUX_DETACHED_TMP/bin/termux-notification"
env -i \
    CODEX_NOTIFY_ARGS="$TMUX_DETACHED_TMP/notify.args" \
    CODEX_TERMUX_HOME="$TMUX_DETACHED_TMP/home" \
    CODEX_TERMUX_STATE_DIR="$TMUX_DETACHED_TMP/state" \
    CODEX_TERMUX_NOTIFY_CHANNEL=notification \
    TMUX="/tmp/tmux-555/default,777,0" \
    PATH="$TMUX_DETACHED_TMP/bin:$PATH" \
    bash "$ROOT_DIR/tools/codex-turn-notify.sh" <<'JSON' >/dev/null 2>&1
{"session_id":"detached-alpha","cwd":"/data/data/com.termux/files/home/prj/codex","last_assistant_message":"detached check"}
JSON
grep -F -- "--open-termux" "$TMUX_DETACHED_TMP/notify.args" >/dev/null \
    || fail 'detached tmux session should not deep-link to tmux target'
