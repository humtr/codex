"""Notification model, configuration, hooks, and Termux providers."""

from .config import load_settings
from .model import (
    ClickAction,
    NotificationRequest,
    NotificationSettings,
    ProviderCapabilities,
    RenderedNotification,
    render_notification,
)

__all__ = (
    "ClickAction",
    "NotificationRequest",
    "NotificationSettings",
    "ProviderCapabilities",
    "RenderedNotification",
    "load_settings",
    "render_notification",
)
