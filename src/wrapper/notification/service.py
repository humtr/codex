"""Notification input, repository context, logging, and provider orchestration."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
from typing import Any, Mapping, TextIO

from .config import NotificationConfigError, load_settings
from .model import (
    ClickAction,
    NotificationRequest,
    NotificationSettings,
    RenderedNotification,
    render_notification,
)
from .provider import ProviderResult, TermuxProvider


def environment_with_config(values: Mapping[str, str]) -> dict[str, str]:
    env = dict(values)
    home = env.get("CODEX_TERMUX_HOME") or env.get("HOME") or ""
    state_dir = env.get("CODEX_TERMUX_STATE_DIR") or str(
        Path(home) / ".local/share/codex/termux"
    )
    notify_dir = env.get("CODEX_TERMUX_NOTIFY_DIR") or str(Path(state_dir) / "notify")
    config_file = env.get("CODEX_TERMUX_NOTIFY_CONFIG") or str(
        Path(notify_dir) / "config.env"
    )
    for key, value in _read_env_file(Path(config_file)).items():
        env.setdefault(key, value)
    env.setdefault("CODEX_TERMUX_HOME", home)
    env.setdefault("CODEX_TERMUX_STATE_DIR", state_dir)
    env.setdefault("CODEX_TERMUX_NOTIFY_DIR", notify_dir)
    env.setdefault("CODEX_TERMUX_NOTIFY_CONFIG", config_file)
    return env


def hook(
    *,
    event: str,
    notify_executable: Path,
    input_stream: TextIO = sys.stdin,
    values: Mapping[str, str] = os.environ,
) -> int:
    env = environment_with_config(values)
    payload = _read_json_stream(input_stream)
    provider = _provider(notify_executable, env)
    tmux_target = _discover_tmux_target(env, provider)
    request = _codex_request(payload, event=event, tmux_target=tmux_target)
    return _deliver(request, provider=provider, env=env)


def send(
    *,
    notify_executable: Path,
    input_stream: TextIO = sys.stdin,
    values: Mapping[str, str] = os.environ,
) -> int:
    env = environment_with_config(values)
    payload = _read_json_stream(input_stream)
    provider = _provider(notify_executable, env)
    requested_target = str(payload.get("tmux_target") or payload.get("tmuxTarget") or "")
    tmux_target = requested_target if provider.tmux_target_valid(requested_target) else ""
    request = _generic_request(payload, tmux_target=tmux_target)
    return _deliver(request, provider=provider, env=env)


def open_target(
    *,
    target: str,
    tmux_target: str,
    notify_executable: Path,
    values: Mapping[str, str] = os.environ,
) -> int:
    env = environment_with_config(values)
    provider = _provider(notify_executable, env)
    try:
        action = {
            "none": ClickAction.NONE,
            "termux": ClickAction.OPEN_TERMUX,
            "tmux": ClickAction.OPEN_TMUX,
        }[target]
    except KeyError as exc:
        raise ValueError(f"unsupported notification target: {target}") from exc
    return provider.open(action, tmux_target)


def self_test() -> int:
    settings = NotificationSettings()
    request = NotificationRequest(
        source="codex",
        event="Stop",
        title=None,
        message="line 1\r\nline 2",
        cwd="",
        project_name=None,
        repository_name="self-test",
        origin_url="",
        git_root_name="",
        session_id="self-test-session",
        transcript_path="",
        tmux_target="",
        dedupe_key="",
        click_action=ClickAction.OPEN_TERMUX,
    )
    rendered = render_notification(request, settings)
    assert rendered.title == "Codex: self-test"
    assert rendered.body == "line 1\nline 2"
    assert rendered.summary == "line 1"
    assert rendered.notification_id.isdigit()
    print("notify self-test: ok")
    return 0


def _deliver(
    request: NotificationRequest,
    *,
    provider: TermuxProvider,
    env: Mapping[str, str],
) -> int:
    try:
        settings = load_settings(env)
    except NotificationConfigError as exc:
        print(f"notify: {exc}", file=sys.stderr)
        return 2
    rendered = render_notification(request, settings)
    notify_dir = Path(env["CODEX_TERMUX_NOTIFY_DIR"])
    notify_dir.mkdir(parents=True, exist_ok=True)
    _write_last_payload(notify_dir / "last-payload.json", rendered, request.source)
    result = provider.deliver(rendered, settings)
    _append_log(notify_dir / "notify.log", result, rendered)
    if not result.delivered:
        sys.stdout.write("\a")
        sys.stdout.flush()
    return 0


def _codex_request(
    payload: dict[str, Any],
    *,
    event: str,
    tmux_target: str,
) -> NotificationRequest:
    cwd = str(payload.get("cwd") or os.environ.get("PWD") or "")
    origin_url, git_root_name = _repository_context(cwd)
    session_id = str(payload.get("session_id") or payload.get("sessionId") or "")
    transcript = str(
        payload.get("transcript_path") or payload.get("transcriptPath") or ""
    )
    base_dedupe_key = str(
        payload.get("dedupe_key")
        or payload.get("dedupeKey")
        or transcript
        or session_id
        or cwd
        or "codex"
    )
    dedupe_key = _session_scoped_key(
        base_dedupe_key,
        payload=payload,
        tmux_target=tmux_target,
    )
    message = str(
        payload.get("content")
        or payload.get("last_assistant_message")
        or payload.get("lastAssistantMessage")
        or payload.get("message")
        or payload.get("event_msg")
        or "Codex turn finished"
    )
    return NotificationRequest(
        source="codex",
        event=event,
        title=str(payload["title"]) if payload.get("title") else None,
        message=message,
        cwd=cwd,
        project_name=_optional_text(payload.get("project_name") or payload.get("projectName")),
        repository_name=_optional_text(
            payload.get("repository_name") or payload.get("repositoryName")
        ),
        origin_url=origin_url,
        git_root_name=git_root_name,
        session_id=session_id,
        transcript_path=transcript,
        tmux_target=tmux_target,
        dedupe_key=dedupe_key,
        click_action=(
            ClickAction.OPEN_TMUX if tmux_target else ClickAction.OPEN_TERMUX
        ),
    )


def _generic_request(
    payload: dict[str, Any],
    *,
    tmux_target: str,
) -> NotificationRequest:
    cwd = str(payload.get("cwd") or os.environ.get("PWD") or "")
    session_id = str(payload.get("session_id") or payload.get("sessionId") or "")
    transcript = str(
        payload.get("transcript_path") or payload.get("transcriptPath") or ""
    )
    base_dedupe_key = str(
        payload.get("dedupe_key")
        or payload.get("dedupeKey")
        or session_id
        or transcript
        or cwd
        or "termux"
    )
    dedupe_key = _session_scoped_key(
        base_dedupe_key,
        payload=payload,
        tmux_target=tmux_target,
    )
    title = str(
        payload.get("title")
        or payload.get("header")
        or payload.get("name")
        or "Termux notification"
    )
    if tmux_target and "tmux:" not in title:
        title = f"{title} | tmux: {tmux_target}"
    message = str(
        payload.get("content")
        or payload.get("message")
        or payload.get("body")
        or payload.get("summary")
        or "Task finished"
    )
    return NotificationRequest(
        source="termux",
        event="",
        title=title,
        message=message,
        cwd=cwd,
        project_name=None,
        repository_name=None,
        origin_url="",
        git_root_name="",
        session_id=session_id,
        transcript_path=transcript,
        tmux_target=tmux_target,
        dedupe_key=dedupe_key,
        click_action=(
            ClickAction.OPEN_TMUX if tmux_target else ClickAction.OPEN_TERMUX
        ),
    )


def _session_scoped_key(
    base_key: str,
    *,
    payload: Mapping[str, Any],
    tmux_target: str,
) -> str:
    explicit = str(
        payload.get("termux_session_id")
        or payload.get("termuxSessionId")
        or payload.get("termux_session")
        or payload.get("termuxSession")
        or ""
    ).strip()
    if explicit:
        scope = explicit
    elif tmux_target:
        scope = tmux_target.split(":", 1)[0]
    else:
        scope = str(
            os.environ.get("TERMUX_SESSION_ID")
            or os.environ.get("TERMUX_SESSION")
            or os.environ.get("TERMUX_PANE")
            or os.environ.get("TMUX_PANE")
            or "default"
        ).strip()
    return f"termux-session:{scope}|{base_key}"


def _repository_context(cwd: str) -> tuple[str, str]:
    if not cwd or not Path(cwd).is_dir():
        return "", ""
    git = shutil.which("git")
    if git is None:
        return "", ""
    origin = _run_text([git, "-C", cwd, "remote", "get-url", "origin"])
    root = _run_text([git, "-C", cwd, "rev-parse", "--show-toplevel"])
    return origin, Path(root).name if root else ""


def _discover_tmux_target(
    env: Mapping[str, str],
    provider: TermuxProvider,
) -> str:
    if not env.get("TMUX"):
        return ""
    tmux = shutil.which("tmux", path=env.get("PATH"))
    if tmux is None:
        return ""
    target = _run_text(
        [tmux, "display-message", "-p", "#S:#I.#P"],
        env=env,
    ) or _run_text([tmux, "display-message", "-p", "#S"], env=env)
    return target if provider.tmux_target_valid(target) else ""


def _provider(notify_executable: Path, env: Mapping[str, str]) -> TermuxProvider:
    prefix = Path(env.get("PREFIX") or "/data/data/com.termux/files/usr")
    return TermuxProvider(
        notify_executable=notify_executable,
        prefix=prefix,
        env=env,
    )


def _read_json_stream(stream: TextIO) -> dict[str, Any]:
    text = stream.read()
    if not text.strip():
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


def _read_env_file(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return result
    for raw_line in lines:
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        try:
            parsed = shlex.split(raw_value, posix=True)
        except ValueError:
            continue
        result[key] = parsed[0] if len(parsed) == 1 else raw_value
    return result


def _write_last_payload(
    path: Path,
    rendered: RenderedNotification,
    source_name: str,
) -> None:
    payload = {
        "source": source_name,
        "title": rendered.title,
        "content": rendered.body,
        "cwd": rendered.cwd,
        "session_id": rendered.session_id,
        "tmux_target": rendered.tmux_target,
        "dedupe_key": rendered.dedupe_key,
    }
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(
        json.dumps(payload, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    os.replace(temporary, path)


def _append_log(
    path: Path,
    result: ProviderResult,
    rendered: RenderedNotification,
) -> None:
    from datetime import datetime

    line = (
        f"{datetime.now().astimezone().isoformat()} "
        f"provider={result.name} id={rendered.notification_id} "
        f"session={rendered.session_id or 'none'} "
        f"tmux_target={rendered.tmux_target or 'none'} "
        f"cwd={rendered.cwd or 'none'}\n"
    )
    with path.open("a", encoding="utf-8") as handle:
        handle.write(line)


def _run_text(
    args: list[str],
    *,
    env: Mapping[str, str] | None = None,
) -> str:
    try:
        result = subprocess.run(
            args,
            check=False,
            capture_output=True,
            text=True,
            env=dict(env) if env is not None else None,
            timeout=2,
        )
    except (OSError, subprocess.TimeoutExpired):
        return ""
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def _optional_text(value: object) -> str | None:
    if value is None or value == "":
        return None
    return str(value)
