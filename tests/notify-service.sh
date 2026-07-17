#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-notify-service.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'notify-service: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/libexec/notify" self-test >/dev/null

mkdir -p "$TMP_DIR/repository" "$TMP_DIR/home" "$TMP_DIR/state/notify" "$TMP_DIR/tmp"
git -C "$TMP_DIR/repository" init -q
git -C "$TMP_DIR/repository" remote add origin https://github.com/example/unified-notify.git

printf '%s' "{\"cwd\":\"$TMP_DIR/repository\",\"message\":\"line 1\\nline 2\"}" | \
    HOME="$TMP_DIR/home" \
    CODEX_TERMUX_HOME="$TMP_DIR/home" \
    CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
    CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
    CODEX_TERMUX_NOTIFY_NO_API=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    python3 -B "$ROOT_DIR/libexec/notify" hook --event Stop >/dev/null

python3 -B - "$TMP_DIR/state/notify/last-payload.json" <<'PYTHON'
import json
import sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["title"] == "Codex: unified-notify"
assert payload["content"] == "line 1\nline 2"
assert payload["tmux_target"] == ""
PYTHON

provider="$TMP_DIR/provider"
mkdir -p "$provider/bin" "$provider/home" "$provider/state/notify" "$provider/tmp"
cat >"$provider/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$@" >"$NOTIFY_ARGS"
SH
cat >"$provider/bin/termux-toast" <<'SH'
#!/bin/sh
printf '%s\n' "$@" >"$TOAST_ARGS"
SH
chmod 755 "$provider/bin/termux-notification" "$provider/bin/termux-toast"

printf '%s' '{"title":"Provider","message":"one line","session_id":"provider-session"}' | \
    env -i \
        HOME="$provider/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$provider/bin:$PATH" \
        NOTIFY_ARGS="$provider/notify.args" \
        TOAST_ARGS="$provider/toast.args" \
        CODEX_TERMUX_HOME="$provider/home" \
        CODEX_TERMUX_STATE_DIR="$provider/state" \
        CODEX_TERMUX_TMPDIR="$provider/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=both \
        CODEX_TERMUX_NOTIFY_TOAST_DURATION=short \
        CODEX_TERMUX_NOTIFY_TOAST_SHORT=0 \
        PYTHONDONTWRITEBYTECODE=1 \
        python3 -B "$ROOT_DIR/libexec/notify" hook --event Stop >/dev/null

grep -Fx -- '--priority' "$provider/notify.args" >/dev/null || fail 'priority option missing'
grep -Fx -- 'max' "$provider/notify.args" >/dev/null || fail 'max priority missing'
grep -Fx -- '--sound' "$provider/notify.args" >/dev/null || fail 'sound option missing'
grep -Fx -- '300,150,300' "$provider/notify.args" >/dev/null || fail 'vibration pattern missing'
grep -F -- "$ROOT_DIR/libexec/notify open --target termux" "$provider/notify.args" >/dev/null \
    || fail 'allowlisted Termux action missing'
grep -Fx -- '-s' "$provider/toast.args" >/dev/null || fail 'new toast duration did not override legacy setting'

printf '%s' '{"title":"Compact","message":"first line\nsecond line","session_id":"compact-session"}' | \
    env -i \
        HOME="$provider/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$provider/bin:$PATH" \
        NOTIFY_ARGS="$provider/notify.args" \
        CODEX_TERMUX_HOME="$provider/home" \
        CODEX_TERMUX_STATE_DIR="$provider/state" \
        CODEX_TERMUX_TMPDIR="$provider/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        PYTHONDONTWRITEBYTECODE=1 \
        python3 -B "$ROOT_DIR/libexec/notify" hook --event Stop >/dev/null

python3 -B - "$provider/notify.args" <<'PYTHON'
import sys

args = open(sys.argv[1], encoding="utf-8").read().splitlines()
content_index = args.index("--content")
assert args[content_index + 1] == "first line second line", args
PYTHON

PYTHONPATH="$ROOT_DIR/src" python3 -B - <<'PYTHON'
from wrapper.notification.model import NotificationSettings, render_notification
from wrapper.notification.service import _codex_request

settings = NotificationSettings()


def notification_id(tmux_target: str) -> str:
    request = _codex_request(
        {"session_id": "same-codex-session", "message": "message"},
        event="Stop",
        tmux_target=tmux_target,
    )
    return render_notification(request, settings).notification_id


same_session_a = notification_id("shared:1.0")
same_session_b = notification_id("shared:2.1")
different_session = notification_id("other:1.0")
assert same_session_a == same_session_b
assert same_session_a != different_session
PYTHON

broken="$TMP_DIR/broken"
mkdir -p "$broken/bin" "$broken/home" "$broken/state/notify" "$broken/tmp"
cat >"$broken/bin/termux-notification" <<'SH'
#!/bin/sh
exit 127
SH
chmod 755 "$broken/bin/termux-notification"

printf '%s' '{"title":"Unavailable API","message":"fallback"}' | \
    env -i \
        HOME="$broken/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$broken/bin:$PATH" \
        CODEX_TERMUX_HOME="$broken/home" \
        CODEX_TERMUX_STATE_DIR="$broken/state" \
        CODEX_TERMUX_TMPDIR="$broken/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        PYTHONDONTWRITEBYTECODE=1 \
        python3 -B "$ROOT_DIR/libexec/notify" hook --event Stop >/dev/null

attached="$TMP_DIR/attached"
mkdir -p "$attached/bin" "$attached/home" "$attached/state/notify" "$attached/tmp"
cat >"$attached/bin/tmux" <<'SH'
#!/bin/sh
case "$1" in
    display-message) printf 'work:1.0\n' ;;
    has-session) [ "$3" = work ] ;;
    list-clients) printf 'client\n' ;;
    switch-client) exit 0 ;;
    *) exit 1 ;;
esac
SH
cat >"$attached/bin/termux-notification" <<'SH'
#!/bin/sh
printf '%s\n' "$@" >"$NOTIFY_ARGS"
SH
chmod 755 "$attached/bin/tmux" "$attached/bin/termux-notification"

printf '%s' '{"message":"attached"}' | \
    env -i \
        HOME="$attached/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$attached/bin:$PATH" \
        TMUX=/tmp/tmux-test/default,1,0 \
        NOTIFY_ARGS="$attached/notify.args" \
        CODEX_TERMUX_HOME="$attached/home" \
        CODEX_TERMUX_STATE_DIR="$attached/state" \
        CODEX_TERMUX_TMPDIR="$attached/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=notification \
        PYTHONDONTWRITEBYTECODE=1 \
        python3 -B "$ROOT_DIR/libexec/notify" hook --event Stop >/dev/null

grep -F -- "$ROOT_DIR/libexec/notify open --target tmux --tmux-target work:1.0" "$attached/notify.args" >/dev/null \
    || fail 'allowlisted tmux action missing'

if PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/libexec/notify" open --target teamviewer >/dev/null 2>&1; then
    fail 'unsupported click target was accepted'
fi

if grep -R -E 'shell[[:space:]]*=[[:space:]]*True|os\.system\(|subprocess\.(call|run|Popen)\([^\n]*shell[[:space:]]*=' \
    "$ROOT_DIR/src/wrapper/notification" >/dev/null; then
    fail 'notification subsystem contains shell execution'
fi

printf 'notify-service: ok\n'
