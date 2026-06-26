#!/data/data/com.termux/files/usr/bin/bash
set -u

CODEX_TERMUX_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_TERMUX_HOME="${CODEX_TERMUX_HOME:-$HOME}"
CODEX_TERMUX_STATE_DIR="${CODEX_TERMUX_STATE_DIR:-$CODEX_TERMUX_HOME/.local/share/codex/termux}"
CODEX_TERMUX_NOTIFY_DIR="${CODEX_TERMUX_NOTIFY_DIR:-$CODEX_TERMUX_STATE_DIR/notify}"
CODEX_TERMUX_NOTIFY_GROUP="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"

codex_notify_log() {
    mkdir -p "$CODEX_TERMUX_NOTIFY_DIR" 2>/dev/null || return 0
    printf '%s %s\n' "$(date -Is 2>/dev/null || date)" "$*" >>"$CODEX_TERMUX_NOTIFY_DIR/notify.log" 2>/dev/null || true
}

codex_notify_tmux_session() {
    command -v tmux >/dev/null 2>&1 || return 0
    tmux display-message -p '#S' 2>/dev/null || true
}

codex_notify_open_termux() {
    command -v am >/dev/null 2>&1 || return 0
    am start --user 0 -n com.termux/.app.TermuxActivity >/dev/null 2>&1 || true
}

codex_notify_open_tmux() {
    local session="${1:-}"
    [ -n "$session" ] || {
        codex_notify_open_termux
        return 0
    }
    command -v tmux >/dev/null 2>&1 && tmux has-session -t "$session" 2>/dev/null || {
        codex_notify_open_termux
        return 0
    }
    if command -v am >/dev/null 2>&1; then
        am startservice --user 0 -n com.termux/.app.RunCommandService \
            -a com.termux.RUN_COMMAND \
            --es com.termux.RUN_COMMAND_PATH "$CODEX_TERMUX_PREFIX/bin/tmux" \
            --esa com.termux.RUN_COMMAND_ARGUMENTS "attach,-t,$session" \
            --ez com.termux.RUN_COMMAND_BACKGROUND false \
            --ez com.termux.RUN_COMMAND_SESSION_ACTION 1 >/dev/null 2>&1 || true
    fi
    tmux switch-client -t "$session" >/dev/null 2>&1 || true
}

codex_notify_escape_action() {
    local out="" item
    for item in "$@"; do
        printf -v item '%q' "$item"
        out="${out:+$out }$item"
    done
    printf '%s\n' "$out"
}

codex_notify_metadata() {
    python3 -c '
import hashlib
import json
import os
import sys

home = sys.argv[1]
tmux_session = sys.argv[2]
payload = sys.stdin.read()

try:
    data = json.loads(payload) if payload.strip() else {}
except json.JSONDecodeError:
    data = {}

cwd = data.get("cwd") or os.environ.get("PWD") or ""
session_id = data.get("session_id") or data.get("sessionId") or ""
transcript = data.get("transcript_path") or data.get("transcriptPath") or ""
key = session_id or transcript or cwd or "codex"
digest = hashlib.sha256(key.encode("utf-8", "replace")).hexdigest()
notification_id = str(10000 + (int(digest[:8], 16) % 2000000000))

def compact_path(path: str) -> str:
    if not path:
        return "unknown"
    path = os.path.abspath(path)
    home_abs = os.path.abspath(home) if home else ""
    if home_abs and (path == home_abs or path.startswith(home_abs + os.sep)):
        path = "~" + path[len(home_abs):]
    if len(path) > 54:
        path = "..." + path[-51:]
    return path

message = (
    data.get("last_assistant_message")
    or data.get("lastAssistantMessage")
    or data.get("message")
    or data.get("event_msg")
    or "Codex turn finished"
)
message = " ".join(str(message).split())
if len(message) > 140:
    message = message[:137] + "..."

title = f"Codex: {compact_path(cwd)}"
if tmux_session:
    title = f"{title} | tmux: {tmux_session}"

print(notification_id)
print(title)
print(message)
print(cwd)
print(session_id)
' "$CODEX_TERMUX_HOME" "${TMUX_SESSION:-}"
}

codex_notify_payload() {
    local payload tmux_session meta notification_id title content cwd session_id action provider="fallback"
    payload="$(cat)"
    tmux_session="$(codex_notify_tmux_session)"
    meta="$(printf '%s' "$payload" | TMUX_SESSION="$tmux_session" codex_notify_metadata)" || meta=""
    notification_id="$(printf '%s\n' "$meta" | sed -n '1p')"
    title="$(printf '%s\n' "$meta" | sed -n '2p')"
    content="$(printf '%s\n' "$meta" | sed -n '3p')"
    cwd="$(printf '%s\n' "$meta" | sed -n '4p')"
    session_id="$(printf '%s\n' "$meta" | sed -n '5p')"
    [ -n "$notification_id" ] || notification_id=10000
    [ -n "$title" ] || title="Codex: unknown"
    [ -n "$content" ] || content="Codex turn finished"

    mkdir -p "$CODEX_TERMUX_NOTIFY_DIR" 2>/dev/null || true
    printf '%s' "$payload" >"$CODEX_TERMUX_NOTIFY_DIR/last-payload.json" 2>/dev/null || true

    if [ -n "$tmux_session" ]; then
        action="$(codex_notify_escape_action "$0" --open-tmux "$tmux_session")"
    else
        action="$(codex_notify_escape_action "$0" --open-termux)"
    fi

    if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" != 1 ] && command -v termux-toast >/dev/null 2>&1; then
        termux-toast "$content" >/dev/null 2>&1 || true
    fi

    if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" != 1 ] && command -v termux-notification >/dev/null 2>&1; then
        if termux-notification \
            --id "$notification_id" \
            --group "$CODEX_TERMUX_NOTIFY_GROUP" \
            --priority max \
            --sound \
            --vibrate 300,150,300 \
            --title "$title" \
            --content "$content" \
            --action "$action" >/dev/null 2>&1; then
            provider="termux-api"
        fi
    fi

    if [ "$provider" != "termux-api" ] && command -v tmux >/dev/null 2>&1; then
        tmux display-message "$title: $content" >/dev/null 2>&1 || true
        provider="tmux"
    fi
    [ "$provider" = "termux-api" ] || printf '\a' 2>/dev/null || true
    codex_notify_log "provider=$provider id=$notification_id session=${session_id:-none} tmux=${tmux_session:-none} cwd=${cwd:-none}"
}

case "${1:-}" in
    --open-termux)
        codex_notify_open_termux
        ;;
    --open-tmux)
        shift
        codex_notify_open_tmux "${1:-}"
        ;;
    *)
        codex_notify_payload
        ;;
esac
