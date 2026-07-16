#!/data/data/com.termux/files/usr/bin/bash
set -u

CODEX_TERMUX_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_TERMUX_HOME="${CODEX_TERMUX_HOME:-$HOME}"
CODEX_TERMUX_STATE_DIR="${CODEX_TERMUX_STATE_DIR:-$CODEX_TERMUX_HOME/.local/share/codex/termux}"
CODEX_TERMUX_NOTIFY_DIR="${CODEX_TERMUX_NOTIFY_DIR:-$CODEX_TERMUX_STATE_DIR/notify}"
CODEX_TERMUX_NOTIFY_GROUP="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"
CODEX_TERMUX_NOTIFY_CONFIG="${CODEX_TERMUX_NOTIFY_CONFIG:-$CODEX_TERMUX_NOTIFY_DIR/config.env}"
[ ! -r "$CODEX_TERMUX_NOTIFY_CONFIG" ] || . "$CODEX_TERMUX_NOTIFY_CONFIG"
CODEX_TERMUX_NOTIFY_CONTENT_CHARS="${CODEX_TERMUX_NOTIFY_CONTENT_CHARS:-0}"
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES="${CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES:-1}"
CODEX_TERMUX_NOTIFY_CHANNEL="${CODEX_TERMUX_NOTIFY_CHANNEL:-notification}"
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY="${CODEX_TERMUX_NOTIFY_TOAST_GRAVITY:-top}"
CODEX_TERMUX_NOTIFY_TOAST_SHORT="${CODEX_TERMUX_NOTIFY_TOAST_SHORT:-0}"
CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND="${CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND:-}"
CODEX_TERMUX_NOTIFY_TOAST_COLOR="${CODEX_TERMUX_NOTIFY_TOAST_COLOR:-}"

termux_notify_log() {
    mkdir -p "$CODEX_TERMUX_NOTIFY_DIR" 2>/dev/null || return 0
    printf '%s %s\n' "$(date -Is 2>/dev/null || date)" "$*" >>"$CODEX_TERMUX_NOTIFY_DIR/notify.log" 2>/dev/null || true
}

termux_notify_open_termux() {
    command -v am >/dev/null 2>&1 || return 0
    am start --user 0 -n com.termux/.app.TermuxActivity >/dev/null 2>&1 || true
}

termux_notify_tmux_target_valid() {
    local target="${1:-}" session
    [ -n "$target" ] || return 1
    command -v tmux >/dev/null 2>&1 || return 1
    session="${target%%:*}"
    [ -n "$session" ] || return 1
    tmux has-session -t "$session" >/dev/null 2>&1 || return 1
    tmux list-clients -t "$session" >/dev/null 2>&1
}

termux_notify_open_tmux() {
    local target="${1:-}"
    [ -n "$target" ] || {
        termux_notify_open_termux
        return 0
    }
    termux_notify_tmux_target_valid "$target" || {
        termux_notify_open_termux
        return 0
    }
    if command -v am >/dev/null 2>&1; then
        am startservice --user 0 -n com.termux/.app.RunCommandService \
            -a com.termux.RUN_COMMAND \
            --es com.termux.RUN_COMMAND_PATH "$CODEX_TERMUX_PREFIX/bin/tmux" \
            --esa com.termux.RUN_COMMAND_ARGUMENTS "attach,-t,$target" \
            --ez com.termux.RUN_COMMAND_BACKGROUND false \
            --ez com.termux.RUN_COMMAND_SESSION_ACTION 1 >/dev/null 2>&1 || true
    fi
    tmux switch-client -t "$target" >/dev/null 2>&1 || true
}

termux_notify_escape_action() {
    local out="" item
    for item in "$@"; do
        printf -v item '%q' "$item"
        out="${out:+$out }$item"
    done
    printf '%s\n' "$out"
}

termux_notify_metadata() {
    python3 -c '
import base64
import hashlib
import json
import os
import sys

home = sys.argv[1]
payload_path = sys.argv[2] if len(sys.argv) > 2 else ""
limit = os.environ.get("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", "0")
preserve_newlines = os.environ.get("CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", "1") == "1"
if payload_path:
    try:
        with open(payload_path, "r", encoding="utf-8", errors="replace") as handle:
            payload = handle.read()
    except OSError:
        payload = ""
else:
    payload = sys.stdin.read()

try:
    data = json.loads(payload) if payload.strip() else {}
except json.JSONDecodeError:
    data = {}

cwd = data.get("cwd") or os.environ.get("PWD") or ""
session_id = data.get("session_id") or data.get("sessionId") or ""
transcript = data.get("transcript_path") or data.get("transcriptPath") or ""
dedupe_key = data.get("dedupe_key") or data.get("dedupeKey") or ""
key = dedupe_key or session_id or transcript or cwd or "termux"
digest = hashlib.sha256(key.encode("utf-8", "replace")).hexdigest()
notification_id = str(10000 + (int(digest[:8], 16) % 2000000000))

title = data.get("title") or data.get("header") or data.get("name") or "Termux notification"
message = data.get("content") or data.get("message") or data.get("body") or data.get("summary") or "Task finished"
title = str(title)
message = str(message)

if preserve_newlines:
    message = message.replace("\r\n", "\n").replace("\r", "\n")
    message = "\n".join(line.rstrip() for line in message.split("\n")[:10]).strip()
    if "\n" not in message:
        message += "\n"
else:
    message = " ".join(message.split())
if limit not in ("0", "full", "none", "unlimited"):
    try:
        max_chars = max(int(limit), 1)
    except ValueError:
        max_chars = 140
    if len(message) > max_chars:
        message = message[: max_chars - 3] + "..." if max_chars > 3 else message[:max_chars]

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

if "{cwd}" in title or title == "Termux notification":
    title = title.replace("{cwd}", compact_path(cwd))

tmux_target = data.get("tmux_target") or data.get("tmuxTarget") or ""
tmux_target = str(tmux_target)
if tmux_target and title == "Termux notification":
    title = f"{title} | tmux: {tmux_target}"
elif tmux_target and "tmux:" not in title:
    title = f"{title} | tmux: {tmux_target}"

def b64(value: str) -> str:
    return base64.b64encode(value.encode("utf-8", "replace")).decode("ascii")

print(notification_id)
print(b64(title))
print(b64(message))
print(b64(cwd))
print(b64(str(session_id)))
' "$CODEX_TERMUX_HOME" "${1:-}"
}

termux_notify_b64_decode() {
    printf '%s' "$1" | base64 -d 2>/dev/null || true
}

termux_notify_channel_has_toast() {
    case "$CODEX_TERMUX_NOTIFY_CHANNEL" in
        toast|both) return 0 ;;
        *) return 1 ;;
    esac
}

termux_notify_channel_has_notification() {
    case "$CODEX_TERMUX_NOTIFY_CHANNEL" in
        notification|both) return 0 ;;
        *) return 1 ;;
    esac
}

termux_notify_payload() {
    local payload tmux_target meta notification_id title content cwd session_id action provider="fallback"
    local toast_args=()
    payload="$(cat)"
    mkdir -p "$CODEX_TERMUX_NOTIFY_DIR" 2>/dev/null || true
    printf '%s' "$payload" >"$CODEX_TERMUX_NOTIFY_DIR/last-payload.json" 2>/dev/null || true
    meta="$(termux_notify_metadata "$CODEX_TERMUX_NOTIFY_DIR/last-payload.json")" || meta=""
    notification_id="$(printf '%s\n' "$meta" | sed -n '1p')"
    title="$(termux_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '2p')")"
    content="$(termux_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '3p')")"
    cwd="$(termux_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '4p')")"
    session_id="$(termux_notify_b64_decode "$(printf '%s\n' "$meta" | sed -n '5p')")"
    tmux_target="$(python3 -c 'import json,sys; print((json.load(open(sys.argv[1], encoding="utf-8", errors="replace"))).get("tmux_target", ""))' "$CODEX_TERMUX_NOTIFY_DIR/last-payload.json" 2>/dev/null || true)"
    [ -n "$notification_id" ] || notification_id=10000
    [ -n "$title" ] || title="Termux notification"
    [ -n "$content" ] || content="Task finished"

    if termux_notify_tmux_target_valid "$tmux_target"; then
        action="$(termux_notify_escape_action "$0" --open-tmux "$tmux_target")"
    else
        action="$(termux_notify_escape_action "$0" --open-termux)"
    fi

    termux_notify_show_toast() {
        if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" = 1 ] ||
            ! termux_notify_channel_has_toast ||
            ! command -v termux-toast >/dev/null 2>&1; then
            return 1
        fi
        toast_args=()
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_GRAVITY" ] || toast_args+=(-g "$CODEX_TERMUX_NOTIFY_TOAST_GRAVITY")
        [ "$CODEX_TERMUX_NOTIFY_TOAST_SHORT" != "1" ] || toast_args+=(-s)
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND" ] || toast_args+=(-b "$CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND")
        [ -z "$CODEX_TERMUX_NOTIFY_TOAST_COLOR" ] || toast_args+=(-c "$CODEX_TERMUX_NOTIFY_TOAST_COLOR")
        termux-toast "${toast_args[@]}" "$content" >/dev/null 2>&1
    }

    termux_notify_show_notification() {
        if [ "${CODEX_TERMUX_NOTIFY_NO_API:-0}" = 1 ] ||
            ! termux_notify_channel_has_notification ||
            ! command -v termux-notification >/dev/null 2>&1; then
            return 1
        fi
        termux-notification \
            --id "$notification_id" \
            --group "$CODEX_TERMUX_NOTIFY_GROUP" \
            --priority max \
            --sound \
            --vibrate 300,150,300 \
            --title "$title" \
            --content "$content" \
            --action "$action" >/dev/null 2>&1
    }

    case "$CODEX_TERMUX_NOTIFY_CHANNEL" in
        toast)
            if termux_notify_show_toast; then
                provider="toast"
            elif termux_notify_show_notification; then
                provider="termux-api"
            fi
            ;;
        notification)
            if termux_notify_show_notification; then
                provider="termux-api"
            elif termux_notify_show_toast; then
                provider="toast"
            fi
            ;;
        both)
            if termux_notify_show_notification; then
                provider="termux-api"
            fi
            if termux_notify_show_toast; then
                provider="${provider:+$provider+}toast"
            fi
            ;;
        *)
            if termux_notify_show_notification; then
                provider="termux-api"
            elif termux_notify_show_toast; then
                provider="toast"
            fi
            ;;
    esac

    if [ "$provider" = "fallback" ] && command -v tmux >/dev/null 2>&1; then
        tmux display-message "$title: $content" >/dev/null 2>&1 || true
        provider="tmux"
    fi
    case "$provider" in
        termux-api*) ;;
        *) printf '\a' 2>/dev/null || true ;;
    esac
    termux_notify_log "provider=$provider id=$notification_id session=${session_id:-none} tmux_target=${tmux_target:-none} cwd=${cwd:-none}"
}

case "${1:-}" in
    --open-termux)
        termux_notify_open_termux
        ;;
    --open-tmux)
        shift
        termux_notify_open_tmux "${1:-}"
        ;;
    *)
        termux_notify_payload
        ;;
esac
