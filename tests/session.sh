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
import base64
import json
import os
import shutil
from pathlib import Path
from codex_termux import session

home = Path(os.environ["CODEX_TERMUX_HOME"])
profile_root = Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"])


def b64(data):
    raw = json.dumps(data, separators=(",", ":")).encode("utf-8")
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def jwt(claims):
    return f"{b64({'alg': 'none', 'typ': 'JWT'})}.{b64(claims)}.sig"


def write_chatgpt_auth(root, *, user, email, account):
    root.mkdir(parents=True, exist_ok=True)
    data = {
        "auth_mode": "chatgpt",
        "tokens": {
            "id_token": jwt({"sub": user, "email": email, "exp": 1999999999}),
            "access_token": jwt({"sub": user, "https://api.openai.com/profile": user, "exp": 1999999999}),
            "refresh_token": f"refresh-{user}-{account}",
            "account_id": account,
        },
        "last_refresh": "2026-07-06T00:00:00Z",
    }
    (root / "auth.json").write_text(json.dumps(data), encoding="utf-8")

# Create default profile sessions directory
default_sess_dir = home / ".codex" / "sessions"
default_sess_dir.mkdir(parents=True, exist_ok=True)

# Create custom profiles
(profile_root / "team-alpha" / "sessions").mkdir(parents=True, exist_ok=True)
(profile_root / "team-beta" / "sessions").mkdir(parents=True, exist_ok=True)
(profile_root / "team-gamma" / "sessions").mkdir(parents=True, exist_ok=True)

write_chatgpt_auth(home / ".codex", user="user-shared", email="shared@example.test", account="account-shared")
write_chatgpt_auth(profile_root / "team-alpha", user="user-shared", email="shared@example.test", account="account-shared")
write_chatgpt_auth(profile_root / "team-beta", user="user-shared", email="shared@example.test", account="account-shared")
write_chatgpt_auth(profile_root / "team-gamma", user="user-other", email="other@example.test", account="account-other")

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

sess_alpha = profile_root / "team-alpha" / "sessions" / "s-alpha.jsonl"
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
assert session.session_boundary_reason("team-alpha", "team-beta") == ""
blocked_reason = session.session_boundary_reason("team-alpha", "team-gamma")
assert "differs" in blocked_reason, blocked_reason
try:
    session.share_session(s_alpha_row.source_path, "team-alpha", "team-gamma")
except session.SessionBoundaryError:
    pass
else:
    raise AssertionError("cross-auth session share should fail")

# Check cross-profile sharing (default target)
dest_default = home / ".codex" / "sessions" / "s-alpha.jsonl"
session.share_session(s_alpha_row.source_path, "team-alpha", "default")
assert dest_default.exists()
assert dest_default.is_symlink() or dest_default.is_file()

# Check cross-profile sharing (custom target)
dest_beta = profile_root / "team-beta" / "sessions" / "s-alpha.jsonl"
session.share_session(s_alpha_row.source_path, "team-alpha", "team-beta")
assert dest_beta.exists()
assert dest_beta.is_symlink() or dest_beta.is_file()

deduped_alpha = [s for s in session.discover_sessions() if s.session_id == "s-alpha"]
assert len(deduped_alpha) == 1, [s.source_path for s in deduped_alpha]
assert deduped_alpha[0].source_profile == "team-alpha", deduped_alpha[0]
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

# Test 3e: Cross-auth session resume is blocked before native codex resume
(
    export TERM=xterm
    export CODEX_SESSION_TUI_MOCK_PROFILE="team-gamma"
    export CODEX_SESSION_TUI_MOCK_CHOICE="$S_ALPHA_IDX"
    unset CODEX_HOME
    LAST_COMMAND=""
    LAST_CODEX_HOME=""

    if codex_session --all 2>"$TMP_DIR/cross-auth.err"; then
        fail "Expected cross-auth session resume to fail"
    fi
    grep -F "Refusing cross-profile session resume/share" "$TMP_DIR/cross-auth.err" >/dev/null ||
        fail "Expected cross-auth refusal, got: $(cat "$TMP_DIR/cross-auth.err")"
    [ -z "$LAST_COMMAND" ] || fail "Native codex resume ran despite cross-auth boundary: $LAST_COMMAND"
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
