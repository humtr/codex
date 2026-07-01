"""Notification hook and config helpers for the Termux wrapper."""

from __future__ import annotations

import shlex
from dataclasses import dataclass


HOOKS = (
    "SessionStart",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "UserPromptSubmit",
    "SubagentStart",
    "SubagentStop",
    "Stop",
)

_CANONICAL = {hook.lower(): hook for hook in HOOKS}
_CANONICAL["all"] = "all"

_STATUS_MESSAGES = {
    "SessionStart": "Notify session start",
    "PreToolUse": "Notify tool start",
    "PermissionRequest": "Notify permission request",
    "PostToolUse": "Notify tool finish",
    "PreCompact": "Notify before compact",
    "PostCompact": "Notify after compact",
    "UserPromptSubmit": "Notify prompt submit",
    "SubagentStart": "Notify subagent start",
    "SubagentStop": "Notify subagent stop",
    "Stop": "Notify turn completion",
}


class NotifyConfigError(ValueError):
    """Raised when notification options are invalid."""


@dataclass(frozen=True)
class NotifySettings:
    content_chars: str
    preserve_newlines: str
    toast_gravity: str
    toast_short: str
    toast_background: str
    toast_color: str
    group: str
    channel: str
    hooks: str
    pretooluse: str


def canonical_hook(value: str) -> str:
    return _CANONICAL.get(value, _CANONICAL.get(value.lower(), value))


def hook_valid(value: str) -> bool:
    return canonical_hook(value) in {*HOOKS, "all"}


def normalize_hooks(value: str = "Stop") -> str:
    hooks = value or "Stop"
    raw_items = hooks.split(",")
    if any(item in {"all", "ALL"} for item in raw_items):
        return "all"
    seen: set[str] = set()
    normalized: list[str] = []
    for item in raw_items:
        event = canonical_hook(item)
        if not event or not hook_valid(event):
            continue
        if event == "all":
            return "all"
        if event not in seen:
            seen.add(event)
            normalized.append(event)
    return ",".join(normalized) if normalized else "Stop"


def hook_list(value: str = "Stop") -> list[str]:
    hooks = normalize_hooks(value)
    if hooks == "all":
        return list(HOOKS)
    return [canonical_hook(item) for item in hooks.split(",") if canonical_hook(item)]


def status_message(value: str) -> str:
    event = canonical_hook(value)
    return _STATUS_MESSAGES.get(event, f"Notify {value}")


def validate_settings(settings: NotifySettings) -> NotifySettings:
    _validate_hooks(settings.hooks)
    _validate_bool("--pretooluse", settings.pretooluse)
    _validate_content_chars(settings.content_chars)
    _validate_bool("--preserve-newlines", settings.preserve_newlines)
    _validate_bool("--toast-short", settings.toast_short)
    if settings.toast_gravity not in {"", "top", "middle", "bottom"}:
        raise NotifyConfigError("--toast-gravity must be top, middle, or bottom")
    if settings.channel not in {"toast", "notification", "both"}:
        raise NotifyConfigError("--channel must be notification, toast, or both")
    return NotifySettings(
        content_chars=settings.content_chars,
        preserve_newlines=settings.preserve_newlines,
        toast_gravity=settings.toast_gravity,
        toast_short=settings.toast_short,
        toast_background=settings.toast_background,
        toast_color=settings.toast_color,
        group=settings.group,
        channel=settings.channel,
        hooks=normalize_hooks(settings.hooks),
        pretooluse=settings.pretooluse,
    )


def render_config_env(settings: NotifySettings) -> str:
    settings = validate_settings(settings)
    entries = (
        ("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", settings.content_chars),
        ("CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", settings.preserve_newlines),
        ("CODEX_TERMUX_NOTIFY_TOAST_GRAVITY", settings.toast_gravity),
        ("CODEX_TERMUX_NOTIFY_TOAST_SHORT", settings.toast_short),
        ("CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND", settings.toast_background),
        ("CODEX_TERMUX_NOTIFY_TOAST_COLOR", settings.toast_color),
        ("CODEX_TERMUX_NOTIFY_GROUP", settings.group),
        ("CODEX_TERMUX_NOTIFY_CHANNEL", settings.channel),
        ("CODEX_TERMUX_NOTIFY_HOOKS", settings.hooks),
        ("CODEX_TERMUX_NOTIFY_PRETOOLUSE", settings.pretooluse),
    )
    return "".join(f"{key}={shlex.quote(value)}\n" for key, value in entries)


def render_system_config(*, hooks: str, turn_notify: str) -> str:
    lines = [
        "[sandbox_workspace_write]",
        "exclude_slash_tmp = true",
        "exclude_tmpdir_env_var = false",
    ]
    for event in hook_list(hooks):
        lines.extend(_hook_block(event, f"{turn_notify} --event {event}"))
    return "\n".join(lines) + "\n"


def _hook_block(event: str, command: str) -> list[str]:
    return [
        f"[[hooks.{event}]]",
        "",
        f"[[hooks.{event}.hooks]]",
        'type = "command"',
        f'command = "{_toml_string(command)}"',
        "timeout = 10",
        f'statusMessage = "{_toml_string(status_message(event))}"',
    ]


def _validate_hooks(value: str) -> None:
    hooks = value or "Stop"
    raw_items = hooks.split(",")
    if any(item in {"all", "ALL"} for item in raw_items):
        return
    for item in raw_items:
        event = canonical_hook(item)
        if event and not hook_valid(event):
            raise NotifyConfigError(f"Unknown notification hook: {item}")


def _validate_bool(label: str, value: str) -> None:
    if value not in {"0", "1"}:
        raise NotifyConfigError(f"{label} must be 0 or 1")


def _validate_content_chars(value: str) -> None:
    if value in {"0", "full", "none", "unlimited"}:
        return
    if value.isdigit() and int(value) > 0:
        return
    raise NotifyConfigError("--content-chars must be a positive integer, 0, full, none, or unlimited")


def _toml_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')
