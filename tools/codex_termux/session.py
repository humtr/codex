"""Cross-profile Codex session picker logic.

This module handles session discovery, parsing metadata, session sharing
across profiles, and generating the launch/resume plan.
"""

from __future__ import annotations

import curses
import os
import re
import sys
import unicodedata
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


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


def find_session_homes() -> list[SessionHome]:
    homes = []
    # default home
    default_path = get_codex_termux_home() / ".codex"
    homes.append(SessionHome(profile="default", home_path=str(default_path), is_default=True))
    
    # custom profiles
    profiles_dir = get_codex_termux_profile_root()
    if profiles_dir.is_dir():
        for p in sorted(profiles_dir.iterdir()):
            if p.is_dir() and not p.name.startswith("."):
                # Profile name validation (letters, numbers, underscore, dash)
                if re.match(r"^[A-Za-z0-9_-]+$", p.name):
                    homes.append(SessionHome(profile=p.name, home_path=str(p), is_default=False))
    return homes


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
    for home in homes:
        sessions_dir = Path(home.home_path) / "sessions"
        if sessions_dir.is_dir():
            for p in sessions_dir.rglob("*.jsonl"):
                if p.is_file():
                    row = parse_codex_session(p, home.profile)
                    if row:
                        rows.append(row)
    # Sort by updated_at descending (newest first)
    rows.sort(key=lambda r: r.updated_at, reverse=True)
    return rows


def share_session(session_path_str: str, session_profile: str, target_profile: str) -> None:
    if session_profile == target_profile:
        return
    
    src_path = Path(session_path_str)
    base_A = get_codex_termux_home() / ".codex" / "sessions" if session_profile == "default" else get_codex_termux_profile_root() / session_profile / "sessions"
    base_B = get_codex_termux_home() / ".codex" / "sessions" if target_profile == "default" else get_codex_termux_profile_root() / target_profile / "sessions"
    
    try:
        rel_path = src_path.relative_to(base_A)
    except ValueError:
        rel_path = Path(src_path.name)
        
    dst_path = base_B / rel_path
    if not dst_path.exists():
        dst_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            dst_path.symlink_to(src_path)
        except Exception:
            import shutil
            shutil.copy2(src_path, dst_path)


def get_session_plan(idx: int, target_profile: str) -> str:
    sessions = discover_sessions()
    if idx < 0 or idx >= len(sessions):
        sys.exit(f"ERROR: Choice out of range [0-{len(sessions)-1}]: {idx}")
        
    session = sessions[idx]
    return get_session_plan_for_row(session, target_profile)


def get_session_plan_for_row(session: SessionRow, target_profile: str) -> str:
    # Perform cross-profile session sharing
    share_session(session.source_path, session.source_profile, target_profile)
    
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
    ]
    return "\x1f".join(fields)


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

    all_sessions = discover_sessions()
    if not all_sessions:
        print("No discovered sessions found.", file=sys.stderr)
        return 1

    if not show_all:
        current_cwd = os.getcwd()
        has_matching = False
        for s in all_sessions:
            if s.workdir:
                try:
                    s_abs = os.path.abspath(os.path.expanduser(s.workdir))
                    c_abs = os.path.abspath(current_cwd)
                    if s_abs == c_abs:
                        has_matching = True
                        break
                except Exception:
                    pass
        if not has_matching:
            print("No matching sessions found in the current directory. Run with --all to show all sessions.", file=sys.stderr)
            return 1

    try:
        res = curses.wrapper(tui_main, all_sessions, show_all)
        if res:
            with open(output_file, "w", encoding="utf-8") as f:
                f.write(res)
            return 0
        return 130
    except KeyboardInterrupt:
        return 130


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
    default_profile_env = os.environ.get("CODEX_SESSION_TUI_DEFAULT_PROFILE")
    if default_profile_env in profiles:
        recent_profile = default_profile_env
    else:
        last_profile_path = os.environ.get("CODEX_TERMUX_LAST_PROFILE_FILE")
        if not last_profile_path:
            last_profile_path = get_codex_termux_home() / ".codex" / "last-profile"
        else:
            last_profile_path = Path(last_profile_path)
            
        recent_profile = "default"
        if last_profile_path and last_profile_path.is_file():
            try:
                recent_profile = last_profile_path.read_text(encoding="utf-8").strip()
            except Exception:
                pass
            
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
