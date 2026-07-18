"""Notification settings with CODEX_TERMUX compatibility parsing."""

from __future__ import annotations

from collections.abc import Mapping

from .model import NotificationSettings


class NotificationConfigError(ValueError):
    """Raised when notification configuration is invalid."""


def _bool(values: Mapping[str, str], key: str, default: bool) -> bool:
    raw = values.get(key)
    if raw is None or raw == "":
        return default
    if raw not in {"0", "1"}:
        raise NotificationConfigError(f"{key} must be 0 or 1")
    return raw == "1"


def _max_chars(values: Mapping[str, str]) -> int | None:
    raw = values.get("CODEX_TERMUX_NOTIFY_CONTENT_CHARS", "0")
    if raw in {"", "0", "full", "none", "unlimited"}:
        return None
    if raw.isdigit() and int(raw) > 0:
        return int(raw)
    raise NotificationConfigError(
        "CODEX_TERMUX_NOTIFY_CONTENT_CHARS must be a positive integer, 0, full, none, or unlimited"
    )


def _toast_duration(values: Mapping[str, str]) -> str:
    explicit = values.get("CODEX_TERMUX_NOTIFY_TOAST_DURATION", "")
    if explicit:
        if explicit not in {"short", "long"}:
            raise NotificationConfigError(
                "CODEX_TERMUX_NOTIFY_TOAST_DURATION must be short or long"
            )
        return explicit
    legacy = values.get("CODEX_TERMUX_NOTIFY_TOAST_SHORT", "0")
    if legacy not in {"0", "1", ""}:
        raise NotificationConfigError("CODEX_TERMUX_NOTIFY_TOAST_SHORT must be 0 or 1")
    return "short" if legacy == "1" else "long"


def load_settings(values: Mapping[str, str]) -> NotificationSettings:
    channel = values.get("CODEX_TERMUX_NOTIFY_CHANNEL", "notification") or "notification"
    if channel not in {"notification", "toast", "both"}:
        raise NotificationConfigError(
            "CODEX_TERMUX_NOTIFY_CHANNEL must be notification, toast, or both"
        )
    gravity = values.get("CODEX_TERMUX_NOTIFY_TOAST_GRAVITY", "top")
    if gravity not in {"", "top", "middle", "bottom"}:
        raise NotificationConfigError(
            "CODEX_TERMUX_NOTIFY_TOAST_GRAVITY must be top, middle, or bottom"
        )
    return NotificationSettings(
        channel=channel,
        priority=values.get("CODEX_TERMUX_NOTIFY_PRIORITY", "max") or "max",
        max_lines=10,
        max_chars=_max_chars(values),
        preserve_newlines=_bool(
            values, "CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES", True
        ),
        toast_duration=_toast_duration(values),
        toast_gravity=gravity,
        toast_background=values.get("CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND", ""),
        toast_color=values.get("CODEX_TERMUX_NOTIFY_TOAST_COLOR", ""),
        group=values.get("CODEX_TERMUX_NOTIFY_GROUP", "codex-turns")
        or "codex-turns",
        sound=_bool(values, "CODEX_TERMUX_NOTIFY_SOUND", True),
        vibrate=values.get("CODEX_TERMUX_NOTIFY_VIBRATE", "300,150,300")
        or "300,150,300",
    )
