"""Termux notification, toast, tmux, and click-action providers."""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
from typing import Mapping

from .model import (
    ClickAction,
    NotificationSettings,
    ProviderCapabilities,
    RenderedNotification,
)


_TMUX_TARGET = re.compile(r"^[A-Za-z0-9_.-]+:[0-9]+\.[0-9]+$")


@dataclass(frozen=True)
class ProviderResult:
    name: str
    delivered: bool


class TermuxProvider:
    capabilities = ProviderCapabilities(
        compact_summary_with_expanded_body=False,
    )

    def __init__(
        self,
        *,
        notify_executable: Path,
        prefix: Path,
        env: Mapping[str, str],
    ) -> None:
        self.notify_executable = notify_executable.resolve()
        self.prefix = prefix
        self.env = dict(env)

    def deliver(
        self,
        rendered: RenderedNotification,
        settings: NotificationSettings,
    ) -> ProviderResult:
        channel = settings.channel
        names: list[str] = []
        delivered = False
        if channel in {"notification", "both"}:
            if self._notification(rendered, settings):
                names.append("termux-api")
                delivered = True
        if channel in {"toast", "both"}:
            if self._toast(rendered, settings):
                names.append("toast")
                delivered = True
        if not delivered and channel == "notification":
            if self._toast(rendered, settings):
                names.append("toast")
                delivered = True
        if not delivered and channel == "toast":
            if self._notification(rendered, settings):
                names.append("termux-api")
                delivered = True
        if not delivered and self._tmux_fallback(rendered):
            names.append("tmux")
            delivered = True
        return ProviderResult(name="+".join(names) if names else "fallback", delivered=delivered)

    def open(self, action: ClickAction, tmux_target: str = "") -> int:
        if action is ClickAction.NONE:
            return 0
        if action is ClickAction.OPEN_TERMUX:
            self._open_termux()
            return 0
        if action is ClickAction.OPEN_TMUX:
            if self.tmux_target_valid(tmux_target):
                self._open_tmux(tmux_target)
            else:
                self._open_termux()
            return 0
        raise ValueError(f"unsupported click action: {action}")

    def tmux_target_valid(self, target: str) -> bool:
        if not _TMUX_TARGET.fullmatch(target):
            return False
        tmux = shutil.which("tmux", path=self.env.get("PATH"))
        if tmux is None:
            return False
        session = target.split(":", 1)[0]
        if self._run([tmux, "has-session", "-t", session]).returncode != 0:
            return False
        return self._run([tmux, "list-clients", "-t", session]).returncode == 0

    def _notification(
        self,
        rendered: RenderedNotification,
        settings: NotificationSettings,
    ) -> bool:
        if self.env.get("CODEX_TERMUX_NOTIFY_NO_API") == "1":
            return False
        executable = shutil.which("termux-notification", path=self.env.get("PATH"))
        if executable is None:
            return False
        args = [
            executable,
            "--id",
            rendered.notification_id,
            "--group",
            settings.group,
            "--priority",
            settings.priority,
        ]
        if settings.sound:
            args.append("--sound")
        if settings.vibrate:
            args.extend(("--vibrate", settings.vibrate))
        args.extend(
            (
                "--title",
                rendered.title,
                "--content",
                rendered.body,
                "--action",
                self._action_command(rendered),
            )
        )
        return self._run(args).returncode == 0

    def _toast(
        self,
        rendered: RenderedNotification,
        settings: NotificationSettings,
    ) -> bool:
        if self.env.get("CODEX_TERMUX_NOTIFY_NO_API") == "1":
            return False
        executable = shutil.which("termux-toast", path=self.env.get("PATH"))
        if executable is None:
            return False
        args = [executable]
        if settings.toast_gravity:
            args.extend(("-g", settings.toast_gravity))
        if settings.toast_duration == "short":
            args.append("-s")
        if settings.toast_background:
            args.extend(("-b", settings.toast_background))
        if settings.toast_color:
            args.extend(("-c", settings.toast_color))
        args.append(rendered.body)
        return self._run(args).returncode == 0

    def _tmux_fallback(self, rendered: RenderedNotification) -> bool:
        tmux = shutil.which("tmux", path=self.env.get("PATH"))
        if tmux is None:
            return False
        result = self._run([tmux, "display-message", f"{rendered.title}: {rendered.body}"])
        return result.returncode == 0

    def _action_command(self, rendered: RenderedNotification) -> str:
        args = [str(self.notify_executable), "open"]
        if rendered.click_action is ClickAction.OPEN_TMUX:
            if not _TMUX_TARGET.fullmatch(rendered.tmux_target):
                raise ValueError("invalid tmux click target")
            args.extend(("--target", "tmux", "--tmux-target", rendered.tmux_target))
        elif rendered.click_action is ClickAction.OPEN_TERMUX:
            args.extend(("--target", "termux"))
        else:
            args.extend(("--target", "none"))
        return shlex.join(args)

    def _open_termux(self) -> None:
        am = shutil.which("am", path=self.env.get("PATH"))
        if am is None:
            return
        self._run(
            [am, "start", "--user", "0", "-n", "com.termux/.app.TermuxActivity"]
        )

    def _open_tmux(self, target: str) -> None:
        am = shutil.which("am", path=self.env.get("PATH"))
        tmux = shutil.which("tmux", path=self.env.get("PATH"))
        if am is not None:
            self._run(
                [
                    am,
                    "startservice",
                    "--user",
                    "0",
                    "-n",
                    "com.termux/.app.RunCommandService",
                    "-a",
                    "com.termux.RUN_COMMAND",
                    "--es",
                    "com.termux.RUN_COMMAND_PATH",
                    str(self.prefix / "bin/tmux"),
                    "--esa",
                    "com.termux.RUN_COMMAND_ARGUMENTS",
                    f"attach,-t,{target}",
                    "--ez",
                    "com.termux.RUN_COMMAND_BACKGROUND",
                    "false",
                    "--ez",
                    "com.termux.RUN_COMMAND_SESSION_ACTION",
                    "1",
                ]
            )
        if tmux is not None:
            self._run([tmux, "switch-client", "-t", target])

    def _run(self, args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            args,
            check=False,
            capture_output=True,
            text=True,
            env=self.env,
            timeout=10,
        )
