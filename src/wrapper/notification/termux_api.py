"""Best-effort adapter for the Termux:API notification commands."""

from __future__ import annotations

from pathlib import Path
import shutil
import subprocess
from typing import Mapping, Sequence


class TermuxApiAdapter:
    """Run Termux:API commands without making notification hooks fail."""

    def __init__(self, env: Mapping[str, str]) -> None:
        self.env = dict(env)
        self.prefix = Path(
            self.env.get("PREFIX", "/data/data/com.termux/files/usr")
        )
        try:
            self.timeout = max(0.1, float(self.env.get("CODEX_TERMUX_API_TIMEOUT", "2")))
        except ValueError:
            self.timeout = 2.0

    def notification(self, args: Sequence[str]) -> bool:
        return self._run("termux-notification", args)

    def toast(self, args: Sequence[str]) -> bool:
        return self._run("termux-toast", args)

    def _run(self, command: str, args: Sequence[str]) -> bool:
        executable = self._executable(command)
        if executable is None:
            return False
        try:
            result = subprocess.run(
                [executable, *args],
                check=False,
                capture_output=True,
                text=True,
                env=self.env,
                timeout=self.timeout,
            )
        except (OSError, subprocess.TimeoutExpired):
            return False
        return result.returncode == 0

    def _executable(self, command: str) -> str | None:
        executable = shutil.which(command, path=self.env.get("PATH"))
        if executable is not None:
            return executable
        bundled = self.prefix / "bin" / command
        if bundled.is_file() and bundled.stat().st_mode & 0o111:
            return str(bundled)
        return None
