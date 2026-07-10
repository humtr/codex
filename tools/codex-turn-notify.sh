#!/data/data/com.termux/files/usr/bin/bash
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_NOTIFY="$SCRIPT_DIR/termux-notify.sh"

codex_notify_tmux_target() {
    local target session
    [ -n "${TMUX:-}" ] || return 0
    command -v tmux >/dev/null 2>&1 || return 0
    target="$(tmux display-message -p '#S:#I.#P' 2>/dev/null || tmux display-message -p '#S' 2>/dev/null || true)"
    [ -n "$target" ] || return 0
    session="${target%%:*}"
    [ -n "$session" ] || return 0
    tmux list-clients -t "$session" >/dev/null 2>&1 || return 0
    printf '%s\n' "$target"
}

codex_notify_render_payload() {
    local tmux_target="$1" payload_file="$2"
    python3 - "$tmux_target" "$payload_file" <<'PY'
import json
import os
import sys

tmux_target = sys.argv[1]
payload_path = sys.argv[2]
limit = os.environ.get("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", "140")
preserve_newlines = os.environ.get("CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", "0") == "1"
event = os.environ.get("CODEX_TERMUX_NOTIFY_EVENT", "")

try:
    payload = json.loads(open(payload_path, encoding="utf-8", errors="replace").read()) if payload_path else {}
except json.JSONDecodeError:
    payload = {}

cwd = payload.get("cwd") or os.environ.get("PWD") or ""
session_id = payload.get("session_id") or payload.get("sessionId") or ""
transcript = payload.get("transcript_path") or payload.get("transcriptPath") or ""
dedupe_key = payload.get("dedupe_key") or payload.get("dedupeKey") or transcript or session_id or cwd or "codex"

def compact_path(path: str) -> str:
    if not path:
        return "unknown"
    path = os.path.abspath(path)
    home = os.path.abspath(os.environ.get("HOME") or "")
    if home and (path == home or path.startswith(home + os.sep)):
        path = "~" + path[len(home):]
    if len(path) > 54:
        path = "..." + path[-51:]
    return path

title = payload.get("title") or f"Codex: {compact_path(cwd)}"
message = (
    payload.get("content")
    or payload.get("last_assistant_message")
    or payload.get("lastAssistantMessage")
    or payload.get("message")
    or payload.get("event_msg")
    or "Codex turn finished"
)
title = str(title)
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
if event:
    event_label = {
        "SessionStart": "session start",
        "PreToolUse": "tool start",
        "PermissionRequest": "permission request",
        "PostToolUse": "tool finished",
        "PreCompact": "before compact",
        "PostCompact": "after compact",
        "UserPromptSubmit": "prompt submitted",
        "SubagentStart": "subagent start",
        "SubagentStop": "subagent finished",
        "Stop": "turn complete",
    }.get(event, event)
    title = f"{title} · {event_label}"

result = {
    "source": "codex",
    "title": title,
    "content": message,
    "cwd": cwd,
    "session_id": session_id,
    "tmux_target": tmux_target,
    "dedupe_key": dedupe_key,
}
print(json.dumps(result, ensure_ascii=False))
PY
}

case "${1:-}" in
    --event)
        shift
        export CODEX_TERMUX_NOTIFY_EVENT="${1:-}"
        shift || true
        exec "${BASH:-bash}" "$0" "$@"
        ;;
    --open-termux)
        exec "${BASH:-bash}" "$COMMON_NOTIFY" --open-termux
        ;;
    --open-tmux)
        shift
        exec "${BASH:-bash}" "$COMMON_NOTIFY" --open-tmux "${1:-}"
        ;;
    *)
        ;;
esac

tmp_root="${CODEX_TERMUX_TMPDIR:-${TMPDIR:-/data/data/com.termux/files/usr/tmp}}"
mkdir -p "$tmp_root"
tmp_dir="$(mktemp -d "$tmp_root/codex-turn-notify.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT
payload_file="$tmp_dir/payload.json"
cat >"$payload_file"
tmux_target="$(codex_notify_tmux_target)"
payload_json="$(codex_notify_render_payload "$tmux_target" "$payload_file")" || exit $?
printf '%s\n' "$payload_json" | "${BASH:-bash}" "$COMMON_NOTIFY"
