#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'session tests: FAIL: %s\n' "$*" >&2
    exit 1
}

# 1. Setup mock environment
export CODEX_TERMUX_HOME="$TMP_DIR/home"
export CODEX_TERMUX_PREFIX="$TMP_DIR/prefix"
export CODEX_TERMUX_TMPDIR="$TMP_DIR/prefix/tmp"
export CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles"
export CODEX_TERMUX_STATE_DIR="$TMP_DIR/state"
export CODEX_TERMUX_LAST_PROFILE_FILE="$CODEX_TERMUX_STATE_DIR/last-profile"
export HOME="$TMP_DIR/home"

mkdir -p "$CODEX_TERMUX_HOME" "$CODEX_TERMUX_TMPDIR" "$CODEX_TERMUX_PROFILE_ROOT" "$CODEX_TERMUX_STATE_DIR"

# 2. Test Python-side discovery and launch plans
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B - <<'PYTHON'
import os
import shutil
from pathlib import Path
from codex_termux import session

home = Path(os.environ["CODEX_TERMUX_HOME"])

# Create default profile sessions directory
default_sess_dir = home / ".codex" / "sessions"
default_sess_dir.mkdir(parents=True, exist_ok=True)

# Create custom profiles
(Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"]) / "team-alpha" / "sessions").mkdir(parents=True, exist_ok=True)
(Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"]) / "team-beta" / "sessions").mkdir(parents=True, exist_ok=True)

# Check profile discovery
homes = session.find_session_homes()
profiles = [h.profile for h in homes]
assert "default" in profiles, f"Expected default in {profiles}"
assert "team-alpha" in profiles, f"Expected team-alpha in {profiles}"
assert "team-beta" in profiles, f"Expected team-beta in {profiles}"

# Check canonical recent profile ownership
session.write_recent_profile("team-beta")
legacy_recent_file = home / ".codex" / "last-profile"
legacy_recent_file.parent.mkdir(parents=True, exist_ok=True)
legacy_recent_file.write_text("team-alpha\n", encoding="utf-8")
assert session.read_recent_profile() == "team-beta"
assert session.select_recent_profile(profiles) == "team-beta"
os.environ["CODEX_SESSION_TUI_DEFAULT_PROFILE"] = "team-alpha"
assert session.select_recent_profile(profiles) == "team-alpha"
del os.environ["CODEX_SESSION_TUI_DEFAULT_PROFILE"]

# Write mock session files
sess_default = default_sess_dir / "s-default.jsonl"
sess_default.write_text('{"type": "session_meta", "payload": {"cwd": "/workspace/default", "id": "s-default"}}\n{"type": "response_item", "payload": {"type": "message", "role": "user", "content": "Hello default"}}\n', encoding="utf-8")

sess_alpha = Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"]) / "team-alpha" / "sessions" / "s-alpha.jsonl"
sess_alpha.write_text('{"type": "session_meta", "payload": {"cwd": "/workspace/alpha", "id": "s-alpha"}}\n{"type": "response_item", "payload": {"type": "message", "role": "user", "content": "Hello alpha"}}\n', encoding="utf-8")

# Check session discovery and parsing
sessions = session.discover_sessions()
session_ids = [s.session_id for s in sessions]
assert "s-default" in session_ids, f"Expected s-default in {session_ids}"
assert "s-alpha" in session_ids, f"Expected s-alpha in {session_ids}"

s_default_row = next(s for s in sessions if s.session_id == "s-default")
assert s_default_row.source_profile == "default"
assert s_default_row.workdir == "/workspace/default"
assert s_default_row.title == "Hello default"

s_alpha_row = next(s for s in sessions if s.session_id == "s-alpha")
assert s_alpha_row.source_profile == "team-alpha"
assert s_alpha_row.workdir == "/workspace/alpha"
assert s_alpha_row.title == "Hello alpha"

plan = session.get_session_plan_for_row(s_alpha_row, "team-beta")
exports = session.session_plan_exports(plan)
assert "CODEX_SESSION_TARGET_PROFILE=team-beta" in exports
assert "CODEX_SESSION_NATIVE_REF=s-alpha" in exports
assert "CODEX_SESSION_SOURCE_PROFILE=team-alpha" in exports
assert "CODEX_SESSION_WORKDIR=/workspace/alpha" in exports

# Check cross-profile sharing (default target)
dest_default = home / ".codex" / "sessions" / "s-alpha.jsonl"
session.share_session(s_alpha_row.source_path, "team-alpha", "default")
assert dest_default.exists()
assert dest_default.is_symlink() or dest_default.is_file()

# Check cross-profile sharing (custom target)
dest_beta = Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"]) / "team-beta" / "sessions" / "s-alpha.jsonl"
session.share_session(s_alpha_row.source_path, "team-alpha", "team-beta")
assert dest_beta.exists()
assert dest_beta.is_symlink() or dest_beta.is_file()
PYTHON

# 3. Test Shell Integration / Dispatch
# Source wrapper script
. "$ROOT_DIR/lib/codex-termux.sh"
unset CODEX_HOME

# Mock shell functions to capture execution details
codex_ensure_runtime_ready() { return 0; }
codex_auto_update_if_needed() { return 0; }
codex_termux_package_root() { printf '%s/tools\n' "$ROOT_DIR"; }

# Override codex_exec_current_runtime to verify execution commands
LAST_COMMAND=""
LAST_CODEX_HOME=""
codex_exec_current_runtime() {
    LAST_COMMAND="$*"
    LAST_CODEX_HOME="${CODEX_HOME-__UNSET__}"
}

# Find index of s-alpha session dynamically
S_ALPHA_IDX=0
idx=0
while IFS= read -r line; do
    if [[ "$line" == s-alpha* ]]; then
        S_ALPHA_IDX="$idx"
        break
    fi
    idx=$((idx + 1))
done < <(codex_termux_cmd session-list)

PLAN_ENV="$(codex_termux_cmd session-plan-env --plan "$(codex_termux_cmd session-select --choice "$S_ALPHA_IDX" --target-profile team-beta)")"
printf '%s\n' "$PLAN_ENV" | grep -Fx "CODEX_SESSION_TARGET_PROFILE=team-beta" >/dev/null ||
    fail "session-plan-env missing target profile: $PLAN_ENV"
printf '%s\n' "$PLAN_ENV" | grep -Fx "CODEX_SESSION_NATIVE_REF=s-alpha" >/dev/null ||
    fail "session-plan-env missing session ref: $PLAN_ENV"

# Test 3a: Select default target profile and s-alpha session
(
    # Simulate TTY and export mock values
    export TERM=xterm
    export CODEX_SESSION_TUI_MOCK_PROFILE="default"
    export CODEX_SESSION_TUI_MOCK_CHOICE="$S_ALPHA_IDX"
    unset CODEX_HOME
    
    codex_session --all
    
    [ "$LAST_COMMAND" = "resume s-alpha --all" ] || fail "Expected command 'resume s-alpha --all', got '$LAST_COMMAND'"
    [ "$LAST_CODEX_HOME" = "__UNSET__" ] || fail "Expected CODEX_HOME to be unset for default profile, got '$LAST_CODEX_HOME'"
)

# Test 3b: Select non-default target profile (team-beta) and s-alpha session
(
    export TERM=xterm
    export CODEX_SESSION_TUI_MOCK_PROFILE="team-beta"
    export CODEX_SESSION_TUI_MOCK_CHOICE="$S_ALPHA_IDX"
    unset CODEX_HOME
    
    codex_session --all
    
    [ "$LAST_COMMAND" = "resume s-alpha --all" ] || fail "Expected command 'resume s-alpha --all', got '$LAST_COMMAND'"
    [ "$LAST_CODEX_HOME" = "$CODEX_TERMUX_PROFILE_ROOT/team-beta" ] || fail "Expected CODEX_HOME to be set to team-beta path, got '$LAST_CODEX_HOME'"
)

# Test 3c: Forward extra options to codex resume
(
    export TERM=xterm
    export CODEX_SESSION_TUI_MOCK_PROFILE="team-beta"
    export CODEX_SESSION_TUI_MOCK_CHOICE="$S_ALPHA_IDX"
    unset CODEX_HOME
    
    codex_session --all --model o3-mini --sandbox danger-full-access
    
    [ "$LAST_COMMAND" = "resume s-alpha --all --model o3-mini --sandbox danger-full-access" ] || fail "Expected command 'resume s-alpha --all --model o3-mini --sandbox danger-full-access', got '$LAST_COMMAND'"
    [ "$LAST_CODEX_HOME" = "$CODEX_TERMUX_PROFILE_ROOT/team-beta" ] || fail "Expected CODEX_HOME to be set to team-beta path, got '$LAST_CODEX_HOME'"
)

# Test 3d: Target profile passed as first argument and forward options
(
    export TERM=xterm
    export CODEX_SESSION_TUI_MOCK_CHOICE="$S_ALPHA_IDX"
    unset CODEX_HOME
    
    codex_session team-beta --all --model o3-mini
    
    [ "$LAST_COMMAND" = "resume s-alpha --all --model o3-mini" ] || fail "Expected command 'resume s-alpha --all --model o3-mini', got '$LAST_COMMAND'"
    [ "$LAST_CODEX_HOME" = "$CODEX_TERMUX_PROFILE_ROOT/team-beta" ] || fail "Expected CODEX_HOME to be set to team-beta path, got '$LAST_CODEX_HOME'"
)

# 4. Test wrapper namespace contract (recognized by wrapper dispatch)
# We mock codex_session to verify that 'codex termux session' calls it
MOCK_SESSION_CALLED=0
codex_session() {
    MOCK_SESSION_CALLED=1
}

# Run dispatch
codex_termux_main session

[ "$MOCK_SESSION_CALLED" -eq 1 ] || fail "Wrapper dispatch did not route 'termux session' command to codex_session"

printf 'session tests: ok\n'
