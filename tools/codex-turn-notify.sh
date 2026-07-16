#!/data/data/com.termux/files/usr/bin/bash
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for notify_entry in \
    "$SCRIPT_DIR/../libexec/notify" \
    "$SCRIPT_DIR/libexec/notify" \
    "$SCRIPT_DIR/source/libexec/notify"
do
    [ -f "$notify_entry" ] || continue
    case "${1:-}" in
        --event)
            exec python3 -B "$notify_entry" hook --event "${2:-}"
            ;;
        --open-termux)
            exec python3 -B "$notify_entry" open --target termux
            ;;
        --open-tmux)
            exec python3 -B "$notify_entry" open --target tmux --tmux-target "${2:-}"
            ;;
        *)
            exec python3 -B "$notify_entry" hook --event "${CODEX_TERMUX_NOTIFY_EVENT:-}"
            ;;
    esac
done
unset notify_entry

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
import subprocess
import sys
from urllib.parse import urlparse

tmux_target = sys.argv[1]
payload_path = sys.argv[2]
limit = os.environ.get("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", "0")
preserve_newlines = os.environ.get("CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", "1") == "1"
event = os.environ.get("CODEX_TERMUX_NOTIFY_EVENT", "")

try:
    payload = json.loads(open(payload_path, encoding="utf-8", errors="replace").read()) if payload_path else {}
except json.JSONDecodeError:
    payload = {}

cwd = payload.get("cwd") or os.environ.get("PWD") or ""
session_id = payload.get("session_id") or payload.get("sessionId") or ""
transcript = payload.get("transcript_path") or payload.get("transcriptPath") or ""
dedupe_key = payload.get("dedupe_key") or payload.get("dedupeKey") or transcript or session_id or cwd or "codex"

def remote_repository_name(url: str) -> str:
    value = url.strip().rstrip("/")
    if not value:
        return ""
    if "://" in value:
        path = urlparse(value).path
    elif ":" in value and not value.startswith("/"):
        path = value.split(":", 1)[1]
    else:
        path = value
    name = os.path.basename(path.rstrip("/"))
    return name[:-4] if name.endswith(".git") else name

def repository_name(data: dict, path: str) -> str:
    explicit = (
        data.get("project_name")
        or data.get("projectName")
        or data.get("repository_name")
        or data.get("repositoryName")
    )
    if explicit:
        return str(explicit)
    if path and os.path.isdir(path):
        try:
            remote = subprocess.run(
                ["git", "-C", path, "remote", "get-url", "origin"],
                check=True,
                capture_output=True,
                text=True,
                timeout=2,
            ).stdout
            name = remote_repository_name(remote)
            if name:
                return name
        except (OSError, subprocess.SubprocessError):
            pass
        try:
            root = subprocess.run(
                ["git", "-C", path, "rev-parse", "--show-toplevel"],
                check=True,
                capture_output=True,
                text=True,
                timeout=2,
            ).stdout.strip()
            if root:
                return os.path.basename(root)
        except (OSError, subprocess.SubprocessError):
            pass
    return "General"

title = payload.get("title") or f"Codex: {repository_name(payload, cwd)}"
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
        "Stop": "",
    }.get(event, event)
    if event_label:
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
