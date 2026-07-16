"""Pure notification data and rendering policy."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import hashlib
import os
from urllib.parse import urlparse


class ClickAction(str, Enum):
    NONE = "none"
    OPEN_TERMUX = "open_termux"
    OPEN_TMUX = "open_tmux"


@dataclass(frozen=True)
class ProviderCapabilities:
    compact_summary_with_expanded_body: bool = False


@dataclass(frozen=True)
class NotificationSettings:
    channel: str = "notification"
    priority: str = "max"
    max_lines: int = 10
    max_chars: int | None = None
    preserve_newlines: bool = True
    toast_duration: str = "long"
    toast_gravity: str = "top"
    toast_background: str = ""
    toast_color: str = ""
    group: str = "codex-turns"
    sound: bool = True
    vibrate: str = "300,150,300"


@dataclass(frozen=True)
class NotificationRequest:
    source: str
    event: str
    title: str | None
    message: str
    cwd: str
    project_name: str | None
    repository_name: str | None
    origin_url: str
    git_root_name: str
    session_id: str
    transcript_path: str
    tmux_target: str
    dedupe_key: str
    click_action: ClickAction


@dataclass(frozen=True)
class RenderedNotification:
    notification_id: str
    title: str
    summary: str
    body: str
    click_action: ClickAction
    cwd: str
    session_id: str
    tmux_target: str
    dedupe_key: str

    def payload(self) -> dict[str, str]:
        return {
            "source": "codex" if self.title.startswith("Codex") else "termux",
            "title": self.title,
            "content": self.body,
            "summary": self.summary,
            "cwd": self.cwd,
            "session_id": self.session_id,
            "tmux_target": self.tmux_target,
            "dedupe_key": self.dedupe_key,
            "notification_id": self.notification_id,
            "click_action": self.click_action.value,
        }


_EVENT_LABELS = {
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
}


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


def repository_label(request: NotificationRequest) -> str:
    for candidate in (
        request.project_name,
        request.repository_name,
        remote_repository_name(request.origin_url),
        request.git_root_name,
    ):
        if candidate:
            return str(candidate)
    return "General"


def normalize_body(message: str, settings: NotificationSettings) -> str:
    text = str(message)
    if settings.preserve_newlines:
        text = text.replace("\r\n", "\n").replace("\r", "\n")
        text = "\n".join(line.rstrip() for line in text.split("\n")[: settings.max_lines]).strip()
        if "\n" not in text:
            text += "\n"
    else:
        text = " ".join(text.split())
    if settings.max_chars is not None and len(text) > settings.max_chars:
        limit = settings.max_chars
        text = text[: limit - 3] + "..." if limit > 3 else text[:limit]
    return text


def body_summary(body: str) -> str:
    for line in body.splitlines():
        if line.strip():
            return line.strip()
    return body.strip()


def notification_id(key: str) -> str:
    digest = hashlib.sha256(key.encode("utf-8", "replace")).hexdigest()
    return str(10000 + (int(digest[:8], 16) % 2000000000))


def render_notification(
    request: NotificationRequest,
    settings: NotificationSettings,
) -> RenderedNotification:
    title = request.title or f"Codex: {repository_label(request)}"
    event_label = _EVENT_LABELS.get(request.event, request.event)
    if event_label:
        title = f"{title} · {event_label}"
    body = normalize_body(request.message, settings)
    key = (
        request.dedupe_key
        or request.session_id
        or request.transcript_path
        or request.cwd
        or "codex"
    )
    return RenderedNotification(
        notification_id=notification_id(key),
        title=title,
        summary=body_summary(body),
        body=body,
        click_action=request.click_action,
        cwd=request.cwd,
        session_id=request.session_id,
        tmux_target=request.tmux_target,
        dedupe_key=key,
    )
