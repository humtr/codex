"""Cross-profile Codex session picker logic.

This module handles session discovery, parsing metadata, session sharing
across profiles, and generating the launch/resume plan.
"""

from __future__ import annotations

import base64
import curses
import hashlib
import json
import os
import re
import shlex
import sqlite3
import sys
import unicodedata
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from . import ui


@dataclass(frozen=True)
class SessionHome:
    profile: str
    home_path: str
    is_default: bool


@dataclass(frozen=True)
class SessionRow:
    source_profile: str
    selected_profile: str
    session_id: str
    native_session_ref: str
    title: str
    updated_at: str
    workdir: str
    source_path: str
    branch: str = ""
    messages: list[tuple[str, str]] = None


@dataclass(frozen=True)
class ProfileAuthIdentity:
    mode: str
    chatgpt_subject: str = ""
    chatgpt_account_id: str = ""
    api_key: str = ""


class SessionBoundaryError(RuntimeError):
    pass


def get_codex_termux_home() -> Path:
    val = os.environ.get("CODEX_TERMUX_HOME")
    if val:
        return Path(val)
    return Path.home()


def get_codex_termux_profile_root() -> Path:
    val = os.environ.get("CODEX_TERMUX_PROFILE_ROOT")
    if val:
        return Path(val)
    return get_codex_termux_home() / ".codex-profiles"


def get_codex_termux_state_dir() -> Path:
    val = os.environ.get("CODEX_TERMUX_STATE_DIR")
    if val:
        return Path(val)
    return get_codex_termux_home() / ".local/share/codex/termux"


def get_last_profile_file() -> Path:
    val = os.environ.get("CODEX_TERMUX_LAST_PROFILE_FILE")
    if val:
        return Path(val)
    return get_codex_termux_state_dir() / "last-profile"


def normalize_profile_choice(choice: str | None) -> str:
    if choice in (None, "", "home", "default"):
        return "default"
    return choice


def is_default_profile(profile: str | None) -> bool:
    return normalize_profile_choice(profile) == "default"


def validate_profile_name(profile: str | None) -> bool:
    name = normalize_profile_choice(profile)
    if name == "default":
        return True
    if name == "termux":
        return False
    if name.startswith(("-", ".")):
        return False
    if "/" in name or ".." in name:
        return False
    if any(ch.isspace() for ch in name):
        return False
    return True


def profile_dir(profile: str | None = "default") -> Path:
    name = normalize_profile_choice(profile)
    if name == "default":
        return get_codex_termux_home() / ".codex"
    return get_codex_termux_profile_root() / name


def profile_display_name(profile: str | None = "default") -> str:
    return normalize_profile_choice(profile)


def profile_for_home(home_path: str | None) -> str:
    if not home_path:
        return ""
    wanted = _normalize_path(Path(home_path).expanduser())
    for profile in profile_menu_ids():
        if _normalize_path(profile_dir(profile)) == wanted:
            return profile
    return ""


def list_profiles() -> list[str]:
    root = get_codex_termux_profile_root()
    if not root.is_dir():
        return []
    names = []
    for path in root.iterdir():
        if not path.is_dir():
            continue
        name = path.name
        if name == "default":
            continue
        if validate_profile_name(name):
            names.append(name)
    return sorted(names, key=str.casefold)


def profile_menu_ids() -> list[str]:
    return ["default", *list_profiles()]


def render_profile_menu(*, interactive: bool) -> int:
    profiles = profile_menu_ids()
    recent = read_recent_profile()
    display_limit = 9 if interactive else 0
    color_enabled = sys.stderr.isatty() and not os.environ.get("NO_COLOR")
    print(ui.text("choose_profile_title"), file=sys.stderr)
    print(ui.format_text("dim", ui.text("choose_profile_subtitle"), color=color_enabled), file=sys.stderr)
    truncated = False
    for index, profile in enumerate(profiles):
        if display_limit and index > display_limit:
            truncated = True
            break
        label = profile_display_name(profile)
        line = f"  {ui.format_text('number', str(index), color=color_enabled)} {label}"
        if profile == recent:
            line = f"{line} {ui.format_text('badge', 'recent', color=color_enabled)}"
        print(line, file=sys.stderr)
    if truncated:
        print(ui.format_text("dim", ui.text("choose_profile_more"), color=color_enabled), file=sys.stderr)
    print(file=sys.stderr)
    return min(len(profiles), 9)


def resolve_profile_menu_choice(choice: str | None) -> str:
    raw = choice or ""
    profiles = profile_menu_ids()
    if raw.isdigit():
        index = int(raw)
        if 0 <= index < len(profiles):
            return profiles[index]
    return normalize_profile_choice(raw)


def profile_create_confirmed(choice: str | None) -> bool:
    return (choice or "") in {"y", "Y"}


def profile_run_plan_exports(profile: str | None, argc: str | int) -> str:
    count = _int_or_zero(argc)
    raw = profile or ""
    if not raw:
        action = "select"
        name = ""
        error = ""
    elif raw in {"list", "ls"}:
        action = "list" if count <= 1 else "profile_arg_error"
        name = raw
        error = raw
    elif raw in {"current", "status"}:
        action = raw if count <= 1 else "profile_arg_error"
        name = raw
        error = raw
    else:
        name = normalize_profile_choice(raw)
        if validate_profile_name(name):
            action = "exec"
            error = ""
        else:
            action = "invalid_profile"
            error = name
    data = {
        "CODEX_PROFILE_RUN_ACTION": action,
        "CODEX_PROFILE_RUN_PROFILE": name,
        "CODEX_PROFILE_RUN_ERROR": error,
    }
    return "\n".join(f"{key}={shlex.quote(value)}" for key, value in data.items())


def prompt_choice_action(reply: str | None, *, mode: str, max_items: str | int, phase: str) -> str:
    raw = reply or ""
    limit = _int_or_zero(max_items)
    if raw == "\x1b":
        return "cancel"
    if raw in ("", "\n", "\r"):
        return "empty"
    if raw.isdigit() and len(raw) == 1:
        value = int(raw)
        if mode == "digits":
            return "accept" if value == 0 or value <= limit else _invalid_prompt_action(phase)
        if limit <= 9:
            return "accept"
        return "read-rest"
    if mode == "digits":
        return _invalid_prompt_action(phase)
    if mode == "yn":
        return "accept" if raw in {"y", "Y", "n", "N"} else _invalid_prompt_action(phase)
    return "read-rest"


def _invalid_prompt_action(phase: str) -> str:
    return "continue" if phase == "tty" else "fail"


def _int_or_zero(value: str | int) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def write_recent_profile(profile: str | None) -> None:
    name = normalize_profile_choice(profile)
    target = get_last_profile_file()
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(name + "\n", encoding="utf-8")


def read_recent_profile() -> str:
    target = get_last_profile_file()
    try:
        profile = target.read_text(encoding="utf-8").splitlines()[0]
    except (OSError, IndexError):
        return "default"
    name = normalize_profile_choice(profile)
    if not validate_profile_name(name):
        return "default"
    if name != "default" and not profile_dir(name).is_dir():
        return "default"
    return name


def profile_current_lines() -> list[str]:
    current_home = os.environ.get("CODEX_HOME", "")
    current_profile = profile_for_home(current_home)
    recent = read_recent_profile()
    bare_home = profile_dir(recent)
    if current_home:
        if current_profile:
            current = f"{current_profile} ({profile_dir(current_profile)}) source=CODEX_HOME"
        else:
            current = f"external ({current_home}) source=CODEX_HOME"
    else:
        current = "unset source=environment"
    lines = [
        f"current: {current}",
        f"bare: {recent} ({bare_home}) source=last-profile",
    ]
    if current_profile and current_profile != recent:
        lines.append("warning: current CODEX_HOME differs from bare launch profile")
    return lines


def profile_status_lines() -> list[str]:
    lines = profile_current_lines()
    lines.append("profiles:")
    recent = read_recent_profile()
    current = profile_for_home(os.environ.get("CODEX_HOME", ""))
    for profile in profile_menu_ids():
        marks = []
        if profile == current:
            marks.append("current")
        if profile == recent:
            marks.append("recent")
        mark = ",".join(marks) if marks else "-"
        auth = profile_auth_summary(profile)
        err = profile_recent_auth_error(profile)
        parts = [f"  {profile}", f"marks={mark}", f"path={profile_dir(profile)}", auth]
        if err:
            parts.append(err)
        lines.append(" ".join(parts))
    return lines


def profile_auth_summary(profile: str) -> str:
    auth_path = profile_dir(profile) / "auth.json"
    if not auth_path.is_file():
        return "auth=none"
    try:
        data = json.loads(auth_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return "auth=invalid"
    mode = str(data.get("auth_mode") or "unknown")
    if mode == "chatgpt":
        return _chatgpt_auth_summary(data)
    if mode == "apikey":
        key = str(data.get("OPENAI_API_KEY") or "")
        suffix = f" key={_hash12(key)}" if key else ""
        return f"auth=apikey{suffix}"
    return f"auth={mode}"


def profile_auth_identity(profile: str) -> ProfileAuthIdentity:
    auth_path = profile_dir(profile) / "auth.json"
    if not auth_path.is_file():
        return ProfileAuthIdentity(mode="none")
    try:
        data = json.loads(auth_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return ProfileAuthIdentity(mode="invalid")

    mode = str(data.get("auth_mode") or "unknown")
    if mode == "chatgpt":
        tokens = data.get("tokens") if isinstance(data.get("tokens"), dict) else {}
        id_claims = _decode_jwt_claims(str(tokens.get("id_token") or ""))
        access_claims = _decode_jwt_claims(str(tokens.get("access_token") or ""))
        subject = str(id_claims.get("sub") or access_claims.get("sub") or "")
        account_id = str(tokens.get("account_id") or "")
        return ProfileAuthIdentity(
            mode=mode,
            chatgpt_subject=subject,
            chatgpt_account_id=account_id,
        )
    if mode == "apikey":
        return ProfileAuthIdentity(
            mode=mode,
            api_key=str(data.get("OPENAI_API_KEY") or ""),
        )
    return ProfileAuthIdentity(mode=mode)


def session_boundary_reason(source_profile: str, target_profile: str) -> str:
    source = normalize_profile_choice(source_profile)
    target = normalize_profile_choice(target_profile)
    if source == target:
        return ""
    if os.environ.get("CODEX_SESSION_ALLOW_CROSS_AUTH", "").lower() in {"1", "true", "yes"}:
        return ""

    source_auth = profile_auth_identity(source)
    target_auth = profile_auth_identity(target)

    # Keep unauthenticated/dev profiles compatible; enforce only known auth boundaries.
    if source_auth.mode in {"none", "invalid"} or target_auth.mode in {"none", "invalid"}:
        return ""

    if source_auth.mode != target_auth.mode:
        return f"source auth mode {source_auth.mode!r} differs from target auth mode {target_auth.mode!r}"

    if source_auth.mode == "chatgpt":
        if (
            source_auth.chatgpt_subject
            and target_auth.chatgpt_subject
            and source_auth.chatgpt_subject != target_auth.chatgpt_subject
        ):
            return "source ChatGPT user differs from target profile"
        if (
            source_auth.chatgpt_account_id
            and target_auth.chatgpt_account_id
            and source_auth.chatgpt_account_id != target_auth.chatgpt_account_id
        ):
            return "source ChatGPT account/workspace differs from target profile"
        return ""

    if source_auth.mode == "apikey":
        if source_auth.api_key and target_auth.api_key and source_auth.api_key != target_auth.api_key:
            return "source API key differs from target profile"
        return ""

    return ""


def session_boundary_error_message(source_profile: str, target_profile: str, reason: str) -> str:
    return (
        "Refusing cross-profile session resume/share: "
        f"{normalize_profile_choice(source_profile)} -> {normalize_profile_choice(target_profile)}: {reason}. "
        "Set CODEX_SESSION_ALLOW_CROSS_AUTH=1 to override explicitly."
    )


def require_session_boundary(source_profile: str, target_profile: str) -> None:
    reason = session_boundary_reason(source_profile, target_profile)
    if reason:
        raise SessionBoundaryError(session_boundary_error_message(source_profile, target_profile, reason))


def profile_recent_auth_error(profile: str) -> str:
    db_path = profile_dir(profile) / "logs_2.sqlite"
    if not db_path.is_file():
        return ""
    since_ts = _profile_auth_last_refresh_epoch(profile)
    patterns = (
        ("refresh_token_invalidated", "%refresh_token_invalidated%"),
        ("token_invalidated", "%token_invalidated%"),
        ("401_unauthorized", "%401 Unauthorized%"),
        ("http_401", "%HTTP 401%"),
        ("status_401", "%status 401%"),
    )
    target_patterns = (
        "codex_login::auth::%",
        "codex_core_plugins::%",
        "codex_mcp::%",
        "codex_api::endpoint::%",
        "codex_client::%",
        "codex_analytics::%",
        "rmcp::%",
        "feedback_tags",
    )
    try:
        con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
        columns = {row[1] for row in con.execute("PRAGMA table_info(logs)").fetchall()}
        clauses = " OR ".join(["feedback_log_body LIKE ?" for _ in patterns])
        params = [pattern for _, pattern in patterns]
        target_clause = ""
        if "target" in columns:
            target_clause = " AND (" + " OR ".join(["target LIKE ?" for _ in target_patterns]) + ")"
            params.extend(target_patterns)
        since_clause = ""
        if since_ts is not None:
            since_clause = " AND ts > ?"
            params.append(since_ts)
        row = con.execute(
            f"SELECT ts, feedback_log_body FROM logs WHERE ({clauses}){target_clause}{since_clause} ORDER BY ts DESC LIMIT 1",
            params,
        ).fetchone()
    except sqlite3.Error:
        return ""
    if not row:
        return ""
    ts, body = row
    code = "auth_error"
    text = str(body or "")
    for name, pattern in patterns:
        needle = pattern.strip("%")
        if needle in text:
            code = name
            break
    return f"last_auth_error={code}@{_format_epoch(ts)}"


def _profile_auth_last_refresh_epoch(profile: str) -> float | None:
    auth_path = profile_dir(profile) / "auth.json"
    if not auth_path.is_file():
        return None
    try:
        data = json.loads(auth_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    value = data.get("last_refresh")
    if not isinstance(value, str) or not value:
        return None
    return _parse_iso_epoch(value)


def select_recent_profile(profiles: list[str]) -> str:
    default_profile_env = os.environ.get("CODEX_SESSION_TUI_DEFAULT_PROFILE")
    if default_profile_env in profiles:
        return default_profile_env
    recent_profile = read_recent_profile()
    if recent_profile in profiles:
        return recent_profile
    return "default" if "default" in profiles else (profiles[0] if profiles else "default")


def _chatgpt_auth_summary(data: dict[str, Any]) -> str:
    tokens = data.get("tokens") if isinstance(data.get("tokens"), dict) else {}
    id_claims = _decode_jwt_claims(str(tokens.get("id_token") or ""))
    access_claims = _decode_jwt_claims(str(tokens.get("access_token") or ""))
    parts = ["auth=chatgpt"]
    user = id_claims.get("sub") or access_claims.get("sub")
    email = id_claims.get("email")
    token_account = tokens.get("account_id")
    profile_claim = access_claims.get("https://api.openai.com/profile")
    access_exp = access_claims.get("exp")
    last_refresh = data.get("last_refresh")
    if user:
        parts.append(f"user={_hash12(user)}")
    if email:
        parts.append(f"email={_hash12(email)}")
    if token_account:
        parts.append(f"token_account={_hash12(token_account)}")
    if profile_claim:
        parts.append(f"profile={_hash12(profile_claim)}")
    if isinstance(access_exp, (int, float)):
        parts.append(f"access_exp={_format_epoch(access_exp)}")
    if isinstance(last_refresh, str) and last_refresh:
        parts.append(f"last_refresh={last_refresh}")
    return " ".join(parts)


def _decode_jwt_claims(token: str) -> dict[str, Any]:
    if token.count(".") < 2:
        return {}
    payload = token.split(".", 2)[1]
    payload += "=" * ((4 - len(payload) % 4) % 4)
    try:
        decoded = base64.urlsafe_b64decode(payload.encode("ascii"))
        claims = json.loads(decoded)
    except (ValueError, json.JSONDecodeError):
        return {}
    return claims if isinstance(claims, dict) else {}


def _hash12(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, (dict, list)):
        raw = json.dumps(value, sort_keys=True, separators=(",", ":"))
    else:
        raw = str(value)
    if not raw:
        return ""
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:12]


def _format_epoch(value: Any) -> str:
    try:
        return datetime.fromtimestamp(float(value), timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    except (TypeError, ValueError, OSError, OverflowError):
        return "unknown"


def _parse_iso_epoch(value: str) -> float | None:
    try:
        normalized = value
        if normalized.endswith("Z"):
            normalized = normalized[:-1] + "+00:00"
        return datetime.fromisoformat(normalized).timestamp()
    except ValueError:
        return None


def _normalize_path(path: Path) -> str:
    return os.path.normcase(os.path.abspath(os.path.expanduser(str(path))))


def find_session_homes() -> list[SessionHome]:
    homes = []
    homes.append(SessionHome(profile="default", home_path=str(profile_dir("default")), is_default=True))
    for profile in list_profiles():
        homes.append(SessionHome(profile=profile, home_path=str(profile_dir(profile)), is_default=False))
    return homes


def profile_for_session_path(path: Path) -> str:
    try:
        wanted = _normalize_path(path.resolve(strict=False))
    except OSError:
        wanted = _normalize_path(path)
    for home in find_session_homes():
        base = _normalize_path(Path(home.home_path) / "sessions")
        try:
            if os.path.commonpath([wanted, base]) == base:
                return home.profile
        except ValueError:
            continue
    return ""


def get_session_id(path: Path) -> str:
    stem = path.stem
    return re.sub(r"^rollout-[0-9TZ:-]+-", "", stem) or stem


def extract_text(content: Any) -> str:
    if not content:
        return ""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        if content:
            return extract_text(content[0])
        return ""
    if isinstance(content, dict):
        return (
            extract_text(content.get("text"))
            or extract_text(content.get("content"))
            or extract_text(content.get("message"))
            or extract_text(content.get("value"))
            or ""
        )
    return str(content)


def parse_codex_session(path: Path, profile: str) -> SessionRow | None:
    session_id = get_session_id(path)
    native_session_ref = path.stem
    workdir = ""
    updated_at = ""
    git_branch = ""
    
    first_user_msg = ""
    last_msg = ""
    messages = []
    
    try:
        mtime = path.stat().st_mtime
        updated_at = datetime.fromtimestamp(mtime, tz=timezone.utc).isoformat()
        
        # Read file contents and extract metadata
        import json
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    data = json.loads(line)
                except json.JSONDecodeError:
                    continue
                
                payload = data.get("payload")
                if not isinstance(payload, dict):
                    payload = {}
                
                ts = data.get("timestamp") or payload.get("timestamp")
                if ts:
                    updated_at = str(ts)
                
                git_info = data.get("git")
                if not git_info and isinstance(payload, dict):
                    git_info = payload.get("git")
                if isinstance(git_info, dict) and "branch" in git_info:
                    git_branch = str(git_info["branch"])
                
                msg_type = data.get("type")
                if msg_type == "session_meta":
                    if "cwd" in payload:
                        workdir = str(payload["cwd"])
                    if "id" in payload:
                        session_id = str(payload["id"])
                elif msg_type == "response_item" and payload.get("type") == "message":
                    role = payload.get("role")
                    content = extract_text(payload.get("content"))
                    if content:
                        messages.append((role, content))
                        if role == "user":
                            if not first_user_msg:
                                first_user_msg = content
                            last_msg = content
                        elif role in ("assistant", "model"):
                            last_msg = content
                elif msg_type == "event_msg":
                    event_type = payload.get("type")
                    msg = extract_text(payload.get("message"))
                    if msg:
                        if event_type == "user_message":
                            messages.append(("user", msg))
                            if not first_user_msg:
                                first_user_msg = msg
                            last_msg = msg
                        elif event_type == "agent_message":
                            messages.append(("assistant", msg))
                            last_msg = msg
    except OSError:
        return None
    
    title_raw = first_user_msg or last_msg or session_id
    title = re.sub(r"\s+", " ", title_raw).strip()
    if len(title) > 80:
        title = title[:77] + "..."
        
    return SessionRow(
        source_profile=profile,
        selected_profile="",
        session_id=session_id,
        native_session_ref=native_session_ref,
        title=title,
        updated_at=updated_at,
        workdir=workdir,
        source_path=str(path),
        branch=git_branch,
        messages=messages,
    )


def discover_sessions() -> list[SessionRow]:
    homes = find_session_homes()
    rows = []
    seen_paths = set()
    for home in homes:
        sessions_dir = Path(home.home_path) / "sessions"
        if sessions_dir.is_dir():
            for p in sessions_dir.rglob("*.jsonl"):
                if p.is_file():
                    source_path = p
                    source_profile = home.profile
                    if p.is_symlink():
                        try:
                            source_path = p.resolve(strict=True)
                        except OSError:
                            continue
                        resolved_profile = profile_for_session_path(source_path)
                        if resolved_profile:
                            source_profile = resolved_profile
                    path_key = _normalize_path(source_path)
                    if path_key in seen_paths:
                        continue
                    seen_paths.add(path_key)
                    row = parse_codex_session(source_path, source_profile)
                    if row:
                        rows.append(row)
    # Sort by updated_at descending (newest first)
    rows.sort(key=lambda r: r.updated_at, reverse=True)
    return rows


def share_session(session_path_str: str, session_profile: str, target_profile: str) -> None:
    source_profile = normalize_profile_choice(session_profile)
    target = normalize_profile_choice(target_profile)
    if source_profile == target:
        return
    require_session_boundary(source_profile, target)

    src_path = Path(session_path_str)
    source_base = profile_dir(source_profile) / "sessions"
    target_base = profile_dir(target) / "sessions"

    try:
        rel_path = src_path.relative_to(source_base)
    except ValueError:
        rel_path = Path(src_path.name)

    dst_path = target_base / rel_path
    if dst_path.exists() or dst_path.is_symlink():
        return
    dst_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        dst_path.symlink_to(src_path)
    except Exception:
        import shutil
        shutil.copy2(src_path, dst_path)


def _display_path(path: str) -> str:
    if not path:
        return ""
    home = str(Path.home())
    if path.startswith(home):
        return "~" + path[len(home):]
    return path


def _format_session_summary(row: SessionRow) -> str:
    parts = [
        row.source_profile,
        format_datetime(row.updated_at) if row.updated_at else "",
        _display_path(row.workdir),
        row.title,
    ]
    return "  ·  ".join(part for part in parts if part)


def _pick_index(stdscr: curses._CursesWindow, title: str, subtitle: str, items: list[str], selected: int = 0) -> int | None:
    curses.curs_set(0)
    h, w = stdscr.getmaxyx()
    prompt = "Enter select   Esc cancel   ↑/↓ move"
    idx = max(0, min(selected, len(items) - 1)) if items else 0
    top = 3
    while True:
        stdscr.erase()
        h, w = stdscr.getmaxyx()
        try:
            stdscr.addstr(0, 1, title[: max(1, w - 2)], curses.A_BOLD)
            if subtitle:
                stdscr.addstr(1, 1, subtitle[: max(1, w - 2)], curses.A_DIM)
        except curses.error:
            pass
        visible_rows = max(1, h - top - 2)
        if items:
            start = max(0, min(idx - visible_rows + 1, max(0, len(items) - visible_rows)))
            end = min(len(items), start + visible_rows)
            for row_no, item_idx in enumerate(range(start, end)):
                attr = curses.A_REVERSE | curses.A_BOLD if item_idx == idx else curses.A_NORMAL
                marker = "❯ " if item_idx == idx else "  "
                line = f"{marker}{items[item_idx]}"
                try:
                    stdscr.addnstr(top + row_no, 1, clip_to_display_width(line, max(1, w - 2)), max(1, w - 2), attr)
                except curses.error:
                    pass
        else:
            try:
                stdscr.addstr(top, 1, "No items found.", curses.A_DIM)
            except curses.error:
                pass
        try:
            stdscr.addnstr(h - 1, 1, prompt[: max(1, w - 2)], max(1, w - 2), curses.A_DIM)
        except curses.error:
            pass
        stdscr.refresh()
        ch = stdscr.getch()
        if ch in (27, 3):
            return None
        if ch in (10, 13, curses.KEY_ENTER):
            return idx if items else None
        if ch == curses.KEY_UP and items:
            idx = (idx - 1) % len(items)
        elif ch == curses.KEY_DOWN and items:
            idx = (idx + 1) % len(items)
        elif ch == curses.KEY_HOME and items:
            idx = 0
        elif ch == curses.KEY_END and items:
            idx = len(items) - 1
        elif ch == curses.KEY_NPAGE and items:
            idx = min(len(items) - 1, idx + max(1, visible_rows))
        elif ch == curses.KEY_PPAGE and items:
            idx = max(0, idx - max(1, visible_rows))


def _pick_session_target() -> str | None:
    homes = find_session_homes()
    profiles = [h.profile for h in homes] or ["default"]
    recent_profile = select_recent_profile(profiles)
    profile_idx = profiles.index(recent_profile) if recent_profile in profiles else 0

    def _profile_picker(stdscr: curses._CursesWindow) -> int | None:
        return _pick_index(
            stdscr,
            "Choose target profile",
            "Select the Codex home to resume the session in.",
            profiles,
            profile_idx,
        )

    try:
        selected = curses.wrapper(_profile_picker)
    except KeyboardInterrupt:
        return None
    if selected is None:
        return None
    return profiles[selected]


def _pick_session_row(show_all: bool) -> SessionRow | None:
    sessions = discover_sessions()
    if not sessions:
        return None
    if not show_all:
        current_cwd = os.getcwd()
        filtered = []
        for s in sessions:
            if not s.workdir:
                continue
            try:
                if os.path.abspath(os.path.expanduser(s.workdir)) == os.path.abspath(current_cwd):
                    filtered.append(s)
            except Exception:
                pass
        if filtered:
            sessions = filtered
        else:
            return None
    labels = [_format_session_summary(s) for s in sessions]

    def _session_picker(stdscr: curses._CursesWindow) -> int | None:
        return _pick_index(
            stdscr,
            "Choose session",
            "Select any discovered Codex session.",
            labels,
            0,
        )

    try:
        selected = curses.wrapper(_session_picker)
    except KeyboardInterrupt:
        return None
    if selected is None:
        return None
    return sessions[selected]


def get_session_plan(idx: int, target_profile: str) -> str:
    sessions = discover_sessions()
    if idx < 0 or idx >= len(sessions):
        sys.exit(f"ERROR: Choice out of range [0-{len(sessions)-1}]: {idx}")
        
    session = sessions[idx]
    return get_session_plan_for_row(session, target_profile)


def get_session_plan_for_row(session: SessionRow, target_profile: str) -> str:
    home_dir = get_codex_termux_home()
    if target_profile == "default":
        target_profile_dir = home_dir / ".codex"
        codex_home_env = ""
    else:
        target_profile_dir = get_codex_termux_profile_root() / target_profile
        codex_home_env = str(target_profile_dir)
        
    fields = [
        target_profile,
        str(target_profile_dir),
        session.native_session_ref,
        session.source_profile,
        session.workdir,
        codex_home_env,
        session.source_path,
    ]
    return "\x1f".join(fields)


def session_plan_exports(plan: str) -> str:
    fields = plan.split("\x1f")
    fields.extend([""] * (7 - len(fields)))
    names = (
        "CODEX_SESSION_TARGET_PROFILE",
        "CODEX_SESSION_TARGET_PROFILE_DIR",
        "CODEX_SESSION_NATIVE_REF",
        "CODEX_SESSION_SOURCE_PROFILE",
        "CODEX_SESSION_WORKDIR",
        "CODEX_SESSION_CODEX_HOME_ENV",
        "CODEX_SESSION_SOURCE_PATH",
    )
    return "\n".join(
        f"{name}={shlex.quote(value)}"
        for name, value in zip(names, fields[:7])
    )


def session_select(choice: str, target_profile: str) -> None:
    try:
        idx = int(choice)
    except ValueError:
        sys.exit(f"ERROR: Choice must be an integer, got '{choice}'")
    print(get_session_plan(idx, target_profile))


def format_datetime(iso_str: str) -> str:
    match = re.match(r"^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})", iso_str)
    if match:
        return f"{match.group(1)} {match.group(2)}"
    return iso_str


def get_relative_time(iso_str: str) -> str:
    try:
        if iso_str.endswith("Z"):
            iso_str = iso_str[:-1] + "+00:00"
        dt = datetime.fromisoformat(iso_str)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        now = datetime.now(timezone.utc)
        diff = now - dt
        seconds = int(diff.total_seconds())
        if seconds < 0:
            return "just now"
        if seconds < 60:
            return "just now"
        minutes = seconds // 60
        if minutes < 60:
            return f"{minutes}m ago"
        hours = minutes // 60
        if hours < 24:
            return f"{hours}h ago"
        days = hours // 24
        if days < 7:
            return f"{days}d ago"
        weeks = days // 7
        if weeks < 4:
            return f"{weeks}w ago"
        months = days // 30
        if months < 12:
            return f"{months}mo ago"
        years = days // 365
        return f"{years}y ago"
    except Exception:
        match = re.match(r"^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})", iso_str)
        if match:
            return f"{match.group(1)} {match.group(2)}"
        return iso_str


def pad_string(s: str, width: int) -> str:
    if len(s) >= width:
        return s[:width]
    return s + " " * (width - len(s))


def clip_to_display_width(s: str, max_width: int) -> str:
    current_width = 0
    clipped_chars = []
    for char in s:
        char_width = 2 if unicodedata.east_asian_width(char) in ('W', 'F') else 1
        if current_width + char_width > max_width:
            break
        clipped_chars.append(char)
        current_width += char_width
    return "".join(clipped_chars)


def make_divider_line(width: int, idx: int, total: int) -> str:
    return "─" * (width - 1)


def draw_profiles_row(stdscr: curses._CursesWindow, y: int, max_x: int, profiles: list[str], profile_idx: int, focused_section: str) -> None:
    is_focused = (focused_section == "profile")
    marker = "❯ " if is_focused else "  "
    label = f"{marker}Profile: "
    
    label_attr = curses.color_pair(4) | curses.A_BOLD if is_focused else curses.color_pair(4) | curses.A_DIM
    
    try:
        stdscr.addstr(y, 2, label, label_attr)
    except curses.error:
        pass
        
    start_x = 13
    remaining_width = max_x - start_x - 2
    if remaining_width <= 0:
        return
        
    visible_start = profile_idx
    visible_end = profile_idx + 1
    
    gap = 2
    while True:
        trial_start = max(0, visible_start - 1)
        trial_end = min(len(profiles), visible_end + 1)
        
        trial_items = profiles[trial_start:trial_end]
        trial_width = sum(len(p) + 4 for p in trial_items) + gap * (len(trial_items) - 1)
        if trial_start > 0:
            trial_width += 4
        if trial_end < len(profiles):
            trial_width += 3
            
        if trial_width > remaining_width:
            break
            
        visible_start, visible_end = trial_start, trial_end
        if visible_start == 0 and visible_end == len(profiles):
            break
            
    x = start_x
    try:
        if visible_start > 0:
            stdscr.addstr(y, x, "... ", curses.A_DIM)
            x += 4
            
        for idx in range(visible_start, visible_end):
            prof = profiles[idx]
            label_text = f"  {prof}  "
            if idx == profile_idx:
                stdscr.addstr(y, x, label_text, curses.color_pair(1) | (curses.A_BOLD if is_focused else curses.A_DIM))
            else:
                stdscr.addstr(y, x, label_text, curses.A_NORMAL if is_focused else curses.A_DIM)
            x += len(label_text) + gap
            
        if visible_end < len(profiles):
            stdscr.addstr(y, x - gap, "...", curses.A_DIM)
    except curses.error:
        pass


def session_tui_command(output_file: str, show_all: bool = False) -> int:
    mock_profile = os.environ.get("CODEX_SESSION_TUI_MOCK_PROFILE")
    mock_choice = os.environ.get("CODEX_SESSION_TUI_MOCK_CHOICE")
    if mock_profile is not None or mock_choice is not None:
        if mock_profile is None:
            mock_profile = os.environ.get("CODEX_SESSION_TUI_DEFAULT_PROFILE", "default")
        if mock_choice is None:
            mock_choice = "0"
        
        # In mock mode, write the plan to the output file directly
        plan = get_session_plan(int(mock_choice), mock_profile)
        with open(output_file, "w", encoding="utf-8") as f:
            f.write(plan)
        return 0

    if not sys.stdout.isatty() or not sys.stdin.isatty():
        print("ERROR: Curses TUI requires an interactive terminal (TTY)", file=sys.stderr)
        return 1

    target_profile = _pick_session_target()
    if target_profile is None:
        return 130
    selected_session = _pick_session_row(show_all=show_all)
    if selected_session is None:
        return 130
    plan = get_session_plan_for_row(selected_session, target_profile)
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(plan)
    return 0


def tui_main(stdscr: curses._CursesWindow, sessions: list[SessionRow], show_all: bool) -> str | None:
    # Set ESC timeout to 50ms to make Esc key responsive
    os.environ.setdefault("ESCDELAY", "50")
    
    # Hide cursor
    try:
        curses.curs_set(0)
    except Exception:
        pass
        
    # Init colors
    try:
        curses.use_default_colors()
        curses.init_pair(1, curses.COLOR_BLACK, curses.COLOR_CYAN)   # Cyan background block highlight
        curses.init_pair(2, curses.COLOR_YELLOW, -1)                  # Yellow foreground
        curses.init_pair(3, curses.COLOR_MAGENTA, -1)                 # Magenta foreground
        curses.init_pair(4, curses.COLOR_CYAN, -1)                    # Cyan foreground
    except Exception:
        pass
        
    # Discover profiles
    homes = find_session_homes()
    profiles = [h.profile for h in homes]
    if not profiles:
        profiles = ["default"]
        
    # Read the recent profile to pre-select it
    recent_profile = select_recent_profile(profiles)
    profile_idx = 0
    if recent_profile in profiles:
        profile_idx = profiles.index(recent_profile)
        
    session_idx = 0
    scroll_offset = 0
    search_query = ""
    
    # Focus and layout states
    focused_section = "session"  # "session" or "profile"
    layout_mode = "comfy"        # "comfy", "dense", "expanded"
    
    while True:
        stdscr.erase()
        max_y, max_x = stdscr.getmaxyx()
        
        # Check minimum screen requirements
        if max_y < 11 or max_x < 40:
            try:
                stdscr.addstr(0, 0, "Terminal too small. Resize to at least 80x12.")
                stdscr.refresh()
            except curses.error:
                pass
            ch = stdscr.getch()
            if ch in (27, ord('q'), ord('Q')):
                return None
            continue
            
        # Draw header (Row 0)
        try:
            stdscr.addstr(0, 1, "Resume a previous session", curses.color_pair(4) | curses.A_BOLD)
        except curses.error:
            pass
        
        # Draw Profiles Bar (Row 1)
        draw_profiles_row(stdscr, 1, max_x, profiles, profile_idx, focused_section)
            
        # Filter sessions by show_all (CWD vs All)
        if not show_all:
            current_cwd = os.getcwd()
            sessions_filtered_by_cwd = []
            for s in sessions:
                if s.workdir:
                    try:
                        s_abs = os.path.abspath(os.path.expanduser(s.workdir))
                        c_abs = os.path.abspath(current_cwd)
                        if s_abs == c_abs:
                            sessions_filtered_by_cwd.append(s)
                    except Exception:
                        pass
            tui_sessions = sessions_filtered_by_cwd
        else:
            tui_sessions = sessions
            
        # Filter sessions by search query
        filtered_sessions = []
        for s in tui_sessions:
            if not search_query:
                filtered_sessions.append(s)
            else:
                q = search_query.lower()
                if q in s.title.lower() or q in s.session_id.lower() or q in s.workdir.lower():
                    filtered_sessions.append(s)
                    
        if len(filtered_sessions) == 0:
            session_idx = 0
        else:
            session_idx = max(0, min(session_idx, len(filtered_sessions) - 1))
            
        # Draw Search/Filter row (Row 3)
        try:
            stdscr.move(3, 0)
            stdscr.clrtoeol()
            
            # Left part: Search Input
            if not search_query:
                stdscr.addstr(3, 2, "Type to search", curses.color_pair(4) | curses.A_DIM)
            else:
                stdscr.addstr(3, 2, search_query, curses.A_NORMAL)
            
            # Right part: Filter & Sort Options
            segments = []
            segments.append(("Filter:  ", curses.color_pair(4) | curses.A_DIM))
            if show_all:
                segments.append(("Cwd ", curses.color_pair(4) | curses.A_DIM))
                segments.append(("[All]", curses.color_pair(3) | curses.A_BOLD))
            else:
                segments.append(("[Cwd]", curses.color_pair(3) | curses.A_BOLD))
                segments.append(("  All", curses.color_pair(4) | curses.A_DIM))
                
            segments.append(("   Sort: ", curses.color_pair(4) | curses.A_DIM))
            segments.append(("[Updated]", curses.color_pair(3) | curses.A_BOLD))
            segments.append((" Created", curses.color_pair(4) | curses.A_DIM))
            
            total_len = sum(len(text) for text, _ in segments)
            start_x = max(20, max_x - total_len - 2)
            
            stdscr.move(3, start_x)
            for text, attr in segments:
                stdscr.addstr(text, attr)
        except curses.error:
            pass
            
        # List dimensions
        list_y_start = 5
        div_y = max_y - 3
        list_available_rows = div_y - list_y_start
        
        # Adjust scroll offset depending on layout mode
        if len(filtered_sessions) > 0:
            if layout_mode == "comfy":
                list_height = max(1, list_available_rows // 3)
                if session_idx < scroll_offset:
                    scroll_offset = session_idx
                elif session_idx >= scroll_offset + list_height:
                    scroll_offset = session_idx - list_height + 1
            elif layout_mode == "dense":
                list_height = max(1, list_available_rows // 1)
                if session_idx < scroll_offset:
                    scroll_offset = session_idx
                elif session_idx >= scroll_offset + list_height:
                    scroll_offset = session_idx - list_height + 1
            elif layout_mode == "expanded":
                if session_idx < scroll_offset:
                    scroll_offset = session_idx
                else:
                    # Make sure the selected item fits on the screen
                    while True:
                        y = list_y_start
                        for idx in range(scroll_offset, session_idx + 1):
                            if idx == session_idx:
                                y += 13
                            else:
                                y += 1
                        if y <= div_y or scroll_offset >= session_idx:
                            break
                        scroll_offset += 1
            
            curr_y = list_y_start
            for i in range(len(filtered_sessions)):
                item_idx = scroll_offset + i
                if item_idx >= len(filtered_sessions):
                    break
                    
                if curr_y >= div_y:
                    break
                    
                sess = filtered_sessions[item_idx]
                is_selected = (item_idx == session_idx)
                
                if layout_mode == "comfy":
                    prefix = "  ❯ " if is_selected else "    "
                    title_text = sess.title
                    
                    rel_time = get_relative_time(sess.updated_at)
                    rel_time_padded = pad_string(rel_time, 14)
                    
                    if show_all:
                        workdir = sess.workdir
                        if workdir:
                            home_str = str(Path.home())
                            if workdir.startswith(home_str):
                                workdir = "~" + workdir[len(home_str):]
                        else:
                            workdir = "<None>"
                        workdir_padded = pad_string(workdir, 44)
                        line2_text = f"    {rel_time_padded}⌁ {workdir_padded}"
                    else:
                        line2_text = f"    {rel_time_padded}" + " " * 46
                        
                    branch = getattr(sess, "branch", "")
                    if branch:
                        line2_text += f" {branch}"
                        
                    y1 = curr_y
                    y2 = y1 + 1
                    
                    try:
                        if is_selected:
                            stdscr.addstr(y1, 0, prefix, curses.color_pair(2) | curses.A_BOLD)
                            clipped_title = clip_to_display_width(title_text, max_x - 6)
                            stdscr.addstr(y1, len(prefix), clipped_title, curses.color_pair(2) | curses.A_BOLD)
                            
                            clipped_details = clip_to_display_width(line2_text, max_x - 2)
                            stdscr.addstr(y2, 0, clipped_details, curses.color_pair(2) | curses.A_DIM)
                        else:
                            clipped_line1 = clip_to_display_width(f"{prefix}{title_text}", max_x - 2)
                            stdscr.addstr(y1, 0, clipped_line1, curses.A_NORMAL)
                            
                            clipped_details = clip_to_display_width(line2_text, max_x - 2)
                            stdscr.addstr(y2, 0, clipped_details, curses.A_DIM)
                    except curses.error:
                        pass
                    curr_y += 3
                    
                elif layout_mode == "dense":
                    prefix = "  ❯ " if is_selected else "    "
                    rel_time = get_relative_time(sess.updated_at)
                    rel_time_padded = pad_string(rel_time, 12)
                    title_text = sess.title
                    
                    line_text = f"{prefix}{rel_time_padded}{title_text}"
                    
                    try:
                        if is_selected:
                            clipped_line = clip_to_display_width(line_text, max_x - 2)
                            stdscr.addstr(curr_y, 0, clipped_line, curses.color_pair(2) | curses.A_BOLD)
                        else:
                            clipped_line = clip_to_display_width(line_text, max_x - 2)
                            stdscr.addstr(curr_y, 0, clipped_line, curses.A_NORMAL)
                    except curses.error:
                        pass
                    curr_y += 1
                    
                elif layout_mode == "expanded":
                    if not is_selected:
                        prefix = "    "
                        rel_time = get_relative_time(sess.updated_at)
                        rel_time_padded = pad_string(rel_time, 12)
                        title_text = sess.title
                        line_text = f"{prefix}{rel_time_padded}{title_text}"
                        try:
                            clipped_line = clip_to_display_width(line_text, max_x - 2)
                            stdscr.addstr(curr_y, 0, clipped_line, curses.A_NORMAL)
                        except curses.error:
                            pass
                        curr_y += 1
                    else:
                        try:
                            stdscr.addstr(curr_y, 0, "   ⌄", curses.color_pair(2) | curses.A_BOLD)
                        except curses.error:
                            pass
                        curr_y += 1
                        
                        session_id_val = sess.session_id
                        
                        created_time_abs = ""
                        match = re.match(r"^rollout-(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})", Path(sess.source_path).name)
                        if match:
                            created_time_abs = f"{match.group(1)} {match.group(2)}:{match.group(3)}:{match.group(4)}"
                        else:
                            created_time_abs = format_datetime(sess.updated_at)
                            
                        created_time_rel = "unknown"
                        if match:
                            created_iso = f"{match.group(1)}T{match.group(2)}:{match.group(3)}:{match.group(4)}Z"
                            created_time_rel = get_relative_time(created_iso)
                        else:
                            created_time_rel = get_relative_time(sess.updated_at)
                            
                        created_val = f"{created_time_rel} · {created_time_abs}"
                        
                        updated_time_abs = format_datetime(sess.updated_at)
                        updated_time_rel = get_relative_time(sess.updated_at)
                        updated_val = f"{updated_time_rel} · {updated_time_abs}"
                        
                        workdir_val = sess.workdir
                        if workdir_val:
                            home_str = str(Path.home())
                            if workdir_val.startswith(home_str):
                                workdir_val = "~" + workdir_val[len(home_str):]
                        else:
                            workdir_val = "<None>"
                            
                        branch_val = getattr(sess, "branch", "")
                        
                        detail_lines = []
                        detail_lines.append(("Session:    ", session_id_val))
                        detail_lines.append(("Created:    ", created_val))
                        detail_lines.append(("Updated:    ", updated_val))
                        detail_lines.append(("Directory:  ", workdir_val))
                        if branch_val:
                            detail_lines.append(("Branch:     ", f" {branch_val}"))
                            
                        detail_lines.append(("", ""))
                        detail_lines.append(("Conversation:", ""))
                        
                        remaining_lines_count = 12 - len(detail_lines)
                        
                        conversation_text = ""
                        if getattr(sess, "messages", None):
                            conv_parts = []
                            for role, content in sess.messages:
                                role_label = "User" if role == "user" else "Assistant"
                                conv_parts.append(f"{role_label}: {content}")
                            conversation_text = "  ".join(conv_parts)
                        else:
                            conversation_text = sess.title
                            
                        wrap_width = max(20, max_x - 18)
                        import textwrap
                        wrapped_conv = textwrap.wrap(conversation_text, width=wrap_width)
                        
                        for line in wrapped_conv[:remaining_lines_count]:
                            detail_lines.append(("", line))
                            
                        while len(detail_lines) < 12:
                            detail_lines.append(("", ""))
                            
                        detail_lines = detail_lines[:12]
                        
                        for d_idx, (key, val) in enumerate(detail_lines):
                            line_prefix = "    └ " if d_idx == 11 else "    │ "
                            try:
                                stdscr.addstr(curr_y, 0, line_prefix, curses.color_pair(4) | curses.A_DIM)
                                x_offset = len(line_prefix)
                                
                                if key:
                                    stdscr.addstr(curr_y, x_offset, key, curses.color_pair(4) | curses.A_DIM)
                                    x_offset += len(key)
                                    
                                if val:
                                    max_val_width = max(1, max_x - x_offset - 2)
                                    clipped_val = clip_to_display_width(val, max_val_width)
                                    stdscr.addstr(curr_y, x_offset, clipped_val, curses.A_NORMAL)
                            except curses.error:
                                pass
                            curr_y += 1
        else:
            try:
                stdscr.addstr(list_y_start, 2, pad_string(" (No matching sessions. Use Left/Right to show all sessions.)", max_x - 4), curses.A_DIM)
            except curses.error:
                pass
                
        # Draw Divider Line and Help Bar
        try:
            div_line = make_divider_line(max_x, session_idx, len(filtered_sessions))
            stdscr.addstr(div_y, 0, div_line, curses.color_pair(4) | curses.A_DIM)
            
            # Re-draw the pagination text in normal white
            if len(filtered_sessions) == 0:
                label = " 0 / 0 · 0% "
            else:
                percent = int((session_idx + 1) / len(filtered_sessions) * 100)
                label = f" {session_idx + 1} / {len(filtered_sessions)} · {percent}% "
            label_len = len(label)
            start_pos = (max_x - 1) - label_len - 1
            if start_pos > 0:
                stdscr.addstr(div_y, start_pos, label, curses.A_NORMAL)
            
            help_y1 = max_y - 2
            help_y2 = max_y - 1
            
            stdscr.move(help_y1, 0)
            stdscr.clrtoeol()
            stdscr.move(help_y2, 0)
            stdscr.clrtoeol()
            
            if focused_section == "profile":
                help_line1 = " enter resume   esc new   ctrl+c quit   tab focus   ←/→ profile"
            else:
                help_line1 = " enter resume   esc new   ctrl+c quit   tab focus   ←/→ option"
                
            ctrl_o_label = "comfy" if layout_mode == "dense" else "dense"
            ctrl_e_label = "comfy" if layout_mode == "expanded" else "exp"
            help_line2 = f" ctrl+o {ctrl_o_label:<5}   ctrl+t preview   ctrl+e {ctrl_e_label:<5}   ↑/↓ browse"
            
            stdscr.addstr(help_y1, 1, help_line1, curses.color_pair(4) | curses.A_DIM)
            stdscr.addstr(help_y2, 1, help_line2, curses.color_pair(4) | curses.A_DIM)
        except curses.error:
            pass
            
        stdscr.refresh()
        
        ch = stdscr.getch()
        
        # 1. Ctrl-O toggle layout density (dense/comfy)
        if ch == 15:  # Ctrl-O
            if layout_mode == "comfy":
                layout_mode = "dense"
            else:
                layout_mode = "comfy"
            scroll_offset = 0
            
        # 2. Ctrl-E toggle layout expanded (expanded/comfy)
        elif ch == 5:  # Ctrl-E
            if layout_mode == "expanded":
                layout_mode = "comfy"
            else:
                layout_mode = "expanded"
            scroll_offset = 0
            
        # 3. Tab and Shift-Tab focus switching
        elif ch in (9, ord('\t')):  # Tab
            focused_section = "profile" if focused_section == "session" else "session"
        elif ch in (353, curses.KEY_BTAB):  # Shift-Tab
            focused_section = "profile" if focused_section == "session" else "session"
            
        # 4. Keyboard Navigation
        elif ch == curses.KEY_UP:
            if len(filtered_sessions) > 0:
                session_idx = (session_idx - 1) % len(filtered_sessions)
        elif ch == curses.KEY_DOWN:
            if len(filtered_sessions) > 0:
                session_idx = (session_idx + 1) % len(filtered_sessions)
        elif ch == curses.KEY_LEFT:
            if focused_section == "profile":
                profile_idx = (profile_idx - 1) % len(profiles)
            else:
                show_all = not show_all
        elif ch == curses.KEY_RIGHT:
            if focused_section == "profile":
                profile_idx = (profile_idx + 1) % len(profiles)
            else:
                show_all = not show_all
                
        elif ch == curses.KEY_NPAGE:  # PageDown
            if len(filtered_sessions) > 0:
                if layout_mode == "comfy":
                    session_idx = min(len(filtered_sessions) - 1, session_idx + max(1, list_available_rows // 3))
                elif layout_mode == "dense":
                    session_idx = min(len(filtered_sessions) - 1, session_idx + list_available_rows)
                elif layout_mode == "expanded":
                    session_idx = min(len(filtered_sessions) - 1, session_idx + max(1, list_available_rows - 12))
        elif ch == curses.KEY_PPAGE:  # PageUp
            if len(filtered_sessions) > 0:
                if layout_mode == "comfy":
                    session_idx = max(0, session_idx - max(1, list_available_rows // 3))
                elif layout_mode == "dense":
                    session_idx = max(0, session_idx - list_available_rows)
                elif layout_mode == "expanded":
                    session_idx = max(0, session_idx - max(1, list_available_rows - 12))
        elif ch == curses.KEY_HOME:
            if len(filtered_sessions) > 0:
                session_idx = 0
        elif ch == curses.KEY_END:
            if len(filtered_sessions) > 0:
                session_idx = len(filtered_sessions) - 1
                
        elif ch in (10, 13, curses.KEY_ENTER):
            if len(filtered_sessions) > 0:
                target_profile = profiles[profile_idx]
                selected_sess = filtered_sessions[session_idx]
                return get_session_plan_for_row(selected_sess, target_profile)
                
        elif ch in (27, 3):  # Esc or Ctrl-C
            return None
            
        elif ch in (curses.KEY_BACKSPACE, 127, 8):
            if search_query:
                search_query = search_query[:-1]
                session_idx = 0
                
        elif 32 <= ch <= 126:
            if not search_query and ch in (ord('q'), ord('Q')):
                return None
            search_query += chr(ch)
            session_idx = 0
