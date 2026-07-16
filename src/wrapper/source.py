"""Wrapper source validation and role-oriented support installation."""

from __future__ import annotations

from ._legacy_source import *  # noqa: F401,F403
from .support_layout import (
    SupportActivation,
    commit_support_install,
    prepare_support_install,
    rollback_support_install,
)
