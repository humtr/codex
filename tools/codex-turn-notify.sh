#!/data/data/com.termux/files/usr/bin/bash
set -u

CODEX_TERMUX_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_TERMUX_HOME="${CODEX_TERMUX_HOME:-$HOME}"
CODEX_TERMUX_STATE_DIR="${CODEX_TERMUX_STATE_DIR:-$CODEX_TERMUX_HOME/.local/share/codex/termux}"
CODEX_TERMUX_NOTIFY_DIR="${CODEX_TERMUX_NOTIFY_DIR:-$CODEX_TERMUX_STATE_DIR/notify}"
CODEX_TERMUX_NOTIFY_GROUP="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"
CODEX_TERMUX_NOTIFY_CONFIG="${CODEX_TERMUX_NOTIFY_CONFIG:-$CODEX_TERMUX_NOTIFY_DIR/config.env}"
[ ! -r "$CODEX_TERMUX_NOTIFY_CONFIG" ] || . "$CODEX_TERMUX_NOTIFY_CONFIG"
CODEX_TERMUX_NOTIFY_CONTENT_CHARS="${CODEX_TERMUX_NOTIFY_CONTENT_CHARS:-140}"
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES="${CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES:-0}"
CODEX_TERMUX_NOTIFY_TOAST="${CODEX_TERMUX_NOTIFY_TOAST:-1}"
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY="${CODEX_TERMUX_NOTIFY_TOAST_GRAVITY:-top}"
CODEX_TERMUX_NOTIFY_TOAST_SHORT="${CODEX_TERMUX_NOTIFY_TOAST_SHORT:-0}"
CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND="${CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND:-}"
CODEX_TERMUX_NOTIFY_TOAST_COLOR="${CODEX_TERMUX_NOTIFY_TOAST_COLOR:-}"
CODEX_TERMUX_NOTIFY_NOTIFICATION="${CODEX_TERMUX_NOTIFY_NOTIFICATION:-1}"

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
import base64
import hashlib
import json
import os
import sys

home = sys.argv[1]
tmux_session = sys.argv[2]
limit = os.environ.get("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", "140")
preserve_newlines = os.environ.get("CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", "0") == "1"
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
message = str(message)
if preserve_newlines:
    message = "\n".join(line.rstrip() for line in message.splitlines()).strip()
else:
    message = " ".join(message.split())
if limit not in ("0", "full", "none", "unlimited"):
    try:
        max_chars = max(int(limit), 1)
    except ValueError:
        max_chars = 140
    if len(message) > max_chars:
        message = message[: max_chars - 3] + "..." if max_chars > 3 else message[:max_chars]

title = f"Codex: {compact_path(cwd)}"
if tmux_session:
    title = f"{title} | tmux: {tmux_session}"

def b64(value: str) -> str:
    return base64.b64encode(value.encode("utf-8", "replace")).decode("ascii")

print(notification_id)
print(b64(title))
print(b64(message))
print(b64(cwd))
print(b64(session_id))
' "$CODEX_TERMUX_HOME" "${TMUX_SESSION:-}"
}

codex_notify_b64_decode() {
    printf '%s' "$1" | base64 -d 2>/dev/null || true
}

codex_notify_event_label() {
    case "${1:-}" in
        SessionStart) printf 'session start' ;;
        PreToolUse) printf 'tool start' ;;
        PermissionRequest) printf 'permission request' ;;
        PostToolUse) printf 'tool finished' ;;
        PreCompact) printf 'before compact' ;;
        PostCompact) printf 'after compact' ;;
        UserPromptSubmit) printf 'prompt submitted' ;;
        SubagentStart) printf 'subagent start' ;;
        SubagentStop) printf 'subagent finished' ;;
        Stop) printf 'turn complete' ;;
        *) printf '%s' "${1:-}" ;;
    esac
}

codex_notify_payload() {
    local payload tmux_session meta notification_id title content cwd session_id action provider="fallback"
    local toast_args=()
    payload="$(cat)"
    tmux_session="$(codex_notify_tmux_session)"
    meta="$(printf '%s' "$payload" | TMUX_SESSION="$tmux_session" codex_notify_metadata)" || meta=""
    notification_id="$(printf '%s\n' "$meta" | sed -n '1p')"
    title="$(codex_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '2p')")"
    content="$(codex_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '3p')")"
    cwd="$(codex_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '4p')")"
    session_id="$(codex_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '5p')")"
    [ -n "$notification_id" ] || notification_id=10000
    [ -n "$title" ] || title="Codex: unknown"
    [ -n "$content" ] || content="Codex turn finished"
    if [ -n "${CODEX_TERMUX_NOTIFY_EVENT:-}" ]; then
        title="$title · $(codex_notify_event_label "$CODEX_TERMUX_NOTIFY_EVENT")"
    fi

    mkdir -p "$CODEX_TERMUX_NOTIFY_DIR" 2>/dev/null || true
    printf '%s' "$payload" >"$CODEX_TERMUX_NOTIFY_DIR/last-payload.json" 2>/dev/null || true

    if [ -n "$tmux_session" ]; then
        action="$(codex_notify_escape_action "$0" --open-tmux "$tmux_session")"
    else
        action="$(codex_notify_escape_action "$0" --open-termux)"
    fi

    if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" != 1 ] &&
        [ "$CODEX_TERMUX_NOTIFY_TOAST" = "1" ] &&
        command -v termux-toast >/dev/null 2>&1; then
        toast_args=()
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_GRAVITY" ] || toast_args+=(-g "$CODEX_TERMUX_NOTIFY_TOAST_GRAVITY")
        [ "$CODEX_TERMUX_NOTIFY_TOAST_SHORT" != "1" ] || toast_args+=(-s)
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND" ] || toast_args+=(-b "$CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND")
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_COLOR" ] || toast_args+=(-c "$CODEX_TERMUX_NOTIFY_TOAST_COLOR")
        termux-toast "${toast_args[@]}" "$content" >/dev/null 2>&1 || true
    fi

    if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" != 1 ] &&
        [ "$CODEX_TERMUX_NOTIFY_NOTIFICATION" = "1" ] &&
        command -v termux-notification >/dev/null 2>&1; then
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
    --event)
        shift
        export CODEX_TERMUX_NOTIFY_EVENT="${1:-}"
        shift || true
        exec "$0" "$@"
        ;;
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
