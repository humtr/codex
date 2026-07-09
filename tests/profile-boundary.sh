#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"

fail() {
    printf 'profile-boundary: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$TMP_DIR/home" "$TMP_DIR/profiles/team" "$TMP_DIR/profiles/Alpha"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'use selection plan model changed'
from codex_termux import use

remote = use.selection_plan_exports({"kind": "remote", "version": "0.142.5"})
assert "CODEX_USE_PLAN_ACTION=install_upstream" in remote
assert "CODEX_USE_PLAN_VERSION=0.142.5" in remote

cached = use.selection_plan_exports({
    "kind": "cached",
    "runtime_path": "/runtime path",
    "raw_path": "/raw",
    "version": "0.142.4",
    "raw_sha256": "raw",
    "runtime_sha256": "runtime",
    "package_spec": "@openai/codex@0.142.4-linux-arm64",
})
assert "CODEX_USE_PLAN_ACTION=activate_cached" in cached
assert "CODEX_USE_PLAN_RUNTIME_PATH='/runtime path'" in cached

assert "CODEX_USE_COMMAND_ACTION=menu" in use.command_plan_exports([])
assert "CODEX_USE_COMMAND_ACTION=list" in use.command_plan_exports(["--list"])
select = use.command_plan_exports(["cached", "ignored"])
assert "CODEX_USE_COMMAND_ACTION=select" in select
assert "CODEX_USE_COMMAND_CHOICE=cached" in select
PYTHON

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_HOME="$TMP_DIR/profiles/Alpha" \
PYTHONDONTWRITEBYTECODE=1 \
PYTHONPATH="$ROOT_DIR/tools" \
python3 -B - <<'PYTHON' || fail 'profile status model changed'
import base64
import json
import os
import sqlite3
from datetime import datetime, timezone
from pathlib import Path

from codex_termux import session


def b64(data):
    raw = json.dumps(data, separators=(",", ":")).encode("utf-8")
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")


def jwt(claims):
    return f"{b64({'alg': 'none', 'typ': 'JWT'})}.{b64(claims)}.sig"


def write_auth(root, *, user, email, token_account, profile_claim, refresh, exp):
    root.mkdir(parents=True, exist_ok=True)
    data = {
        "auth_mode": "chatgpt",
        "tokens": {
            "id_token": jwt({"sub": user, "email": email, "exp": exp}),
            "access_token": jwt({"sub": user, "https://api.openai.com/profile": profile_claim, "exp": exp}),
            "refresh_token": refresh,
            "account_id": token_account,
        },
        "last_refresh": "2026-07-06T00:00:00Z",
    }
    (root / "auth.json").write_text(json.dumps(data), encoding="utf-8")


home = Path(os.environ["CODEX_TERMUX_HOME"])
profiles = Path(os.environ["CODEX_TERMUX_PROFILE_ROOT"])
exp = int(datetime(2026, 7, 16, tzinfo=timezone.utc).timestamp())
shared_account = "workspace-or-account-shared"
write_auth(home / ".codex", user="user-default", email="default@example.test", token_account=shared_account, profile_claim="profile-default", refresh="refresh-default", exp=exp)
write_auth(profiles / "team", user="user-team", email="team@example.test", token_account=shared_account, profile_claim="profile-team", refresh="refresh-team", exp=exp)
write_auth(profiles / "Alpha", user="user-alpha", email="alpha@example.test", token_account=shared_account, profile_claim="profile-alpha", refresh="refresh-alpha", exp=exp)

log_db = profiles / "team" / "logs_2.sqlite"
con = sqlite3.connect(log_db)
con.execute("CREATE TABLE logs (ts REAL, target TEXT, feedback_log_body TEXT)")
con.execute(
    "INSERT INTO logs VALUES (?, ?, ?)",
    (
        datetime(2026, 7, 5, tzinfo=timezone.utc).timestamp(),
        "codex_login::auth::manager",
        "Older token_invalidated event before the current last_refresh.",
    ),
)
con.execute(
    "INSERT INTO logs VALUES (?, ?, ?)",
    (
        datetime(2026, 7, 6, 0, 0, 1, tzinfo=timezone.utc).timestamp(),
        "codex_login::auth::manager",
        "Failed to refresh token: 401 Unauthorized: refresh_token_invalidated",
    ),
)
con.execute(
    "INSERT INTO logs VALUES (?, ?, ?)",
    (
        datetime(2026, 7, 7, tzinfo=timezone.utc).timestamp(),
        "log",
        "Assistant tool output mentioned refresh_token_invalidated but this is not an auth event.",
    ),
)
con.commit()
con.close()

session.write_recent_profile("team")
current = "\n".join(session.profile_current_lines())
assert "current: Alpha" in current, current
assert "bare: team" in current, current
assert "warning: current CODEX_HOME differs from bare launch profile" in current, current

status = "\n".join(session.profile_status_lines())
assert "profiles:" in status, status
assert "  Alpha marks=current" in status, status
assert "  team marks=recent" in status, status
assert "last_auth_error=refresh_token_invalidated@2026-07-06T00:00:01Z" in status, status

alpha_line = next(line for line in status.splitlines() if line.startswith("  Alpha "))
team_line = next(line for line in status.splitlines() if line.startswith("  team "))
assert "token_account=" in alpha_line and "token_account=" in team_line
assert alpha_line.split("token_account=", 1)[1].split()[0] == team_line.split("token_account=", 1)[1].split()[0]
assert alpha_line.split("user=", 1)[1].split()[0] != team_line.split("user=", 1)[1].split()[0]
assert "example.test" not in status
assert "refresh-alpha" not in status
assert "refresh-team" not in status
PYTHON

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles" \
PYTHONDONTWRITEBYTECODE=1 \
PYTHONPATH="$ROOT_DIR/tools" \
python3 -B - <<'PYTHON' || fail 'profile state default path changed'
from pathlib import Path
import os

from codex_termux import session

home = Path(os.environ["CODEX_TERMUX_HOME"])
state_dir = home / ".local/share/codex/termux"
state_dir.mkdir(parents=True, exist_ok=True)
(state_dir / "last-profile").write_text("Alpha\n", encoding="utf-8")
assert session.get_codex_termux_state_dir() == state_dir
assert session.read_recent_profile() == "Alpha"
PYTHON

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
bash -lc '
. "$1"
[ "$(codex_termux_cmd profile-choice-to-name --choice home)" = "default" ]
[ "$(codex_termux_cmd profile-dir --profile default)" = "$CODEX_TERMUX_HOME/.codex" ]
[ "$(codex_termux_cmd profile-dir --profile team)" = "$CODEX_TERMUX_PROFILE_ROOT/team" ]
codex_termux_cmd profile-validate --profile team
! codex_termux_cmd profile-validate --profile termux
! codex_termux_cmd profile-validate --profile "../bad"
	codex_termux_cmd profile-write-recent --profile team
	[ "$(codex_termux_cmd profile-read-recent)" = "team" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile current --argc 1 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=current" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile status --argc 1 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=status" ]
	profile_current="$(CODEX_HOME="$CODEX_TERMUX_PROFILE_ROOT/Alpha" codex_profile_run current)"
	printf "%s\n" "$profile_current" | grep -F "current: Alpha" >/dev/null
	printf "%s\n" "$profile_current" | grep -F "bare: team" >/dev/null
	profile_status="$(CODEX_HOME="$CODEX_TERMUX_PROFILE_ROOT/Alpha" codex_profile_run status)"
	printf "%s\n" "$profile_status" | grep -F "warning: current CODEX_HOME differs from bare launch profile" >/dev/null
	printf "%s\n" "$profile_status" | grep -F "last_auth_error=refresh_token_invalidated@2026-07-06T00:00:01Z" >/dev/null
	rm -rf "$CODEX_TERMUX_PROFILE_ROOT/team"
	[ "$(codex_termux_cmd profile-read-recent)" = "default" ]
profiles="$(codex_termux_cmd profile-list)"
case "$profiles" in
    Alpha) ;;
    *) printf "profiles=%s\n" "$profiles" >&2; exit 1 ;;
esac
menu="$(codex_termux_cmd profile-menu-ids | tr "\n" " ")"
[ "$menu" = "default Alpha " ]
	[ "$(codex_termux_cmd profile-menu-choice --choice 0)" = "default" ]
	[ "$(codex_termux_cmd profile-menu-choice --choice 1)" = "Alpha" ]
	[ "$(codex_termux_cmd profile-menu-choice --choice home)" = "default" ]
	[ "$(codex_termux_cmd profile-menu-choice --choice Alpha)" = "Alpha" ]
	[ "$(codex_termux_cmd profile-run-plan-env --argc 0 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=select" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile list --argc 1 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=list" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile list --argc 2 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=profile_arg_error" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile Alpha --argc 1 | sed -n "2p")" = "CODEX_PROFILE_RUN_PROFILE=Alpha" ]
	[ "$(codex_termux_cmd profile-run-plan-env --profile termux --argc 1 | sed -n "1p")" = "CODEX_PROFILE_RUN_ACTION=invalid_profile" ]
	render_count="$(codex_termux_cmd profile-menu-render --interactive 1 2>"$2/profile-menu.err")"
[ "$render_count" = "2" ]
grep -F "Choose profile" "$2/profile-menu.err" >/dev/null
grep -F "Alpha" "$2/profile-menu.err" >/dev/null
codex_termux_cmd profile-create-confirmed --choice y
codex_termux_cmd profile-create-confirmed --choice Y
! codex_termux_cmd profile-create-confirmed --choice n
! codex_termux_cmd profile-create-confirmed --choice ""
[ "$(codex_termux_cmd prompt-choice-action --reply 4 --mode digits --max-items 4 --phase final)" = "accept" ]
[ "$(codex_termux_cmd prompt-choice-action --reply 5 --mode digits --max-items 4 --phase final)" = "fail" ]
[ "$(codex_termux_cmd prompt-choice-action --reply 5 --mode digits --max-items 4 --phase tty)" = "continue" ]
[ "$(codex_termux_cmd prompt-choice-action --reply y --mode yn --max-items 0 --phase final)" = "accept" ]
[ "$(codex_termux_cmd prompt-choice-action --reply x --mode yn --max-items 0 --phase final)" = "fail" ]
	[ "$(codex_termux_cmd prompt-choice-action --reply x --mode freeform --max-items 12 --phase final)" = "read-rest" ]
	[ "$(codex_termux_cmd use-command-plan-env | sed -n "1p")" = "CODEX_USE_COMMAND_ACTION=menu" ]
	[ "$(codex_termux_cmd use-command-plan-env --arg=--list | sed -n "1p")" = "CODEX_USE_COMMAND_ACTION=list" ]
	[ "$(codex_termux_cmd use-command-plan-env --arg=cached --arg=ignored | sed -n "2p")" = "CODEX_USE_COMMAND_CHOICE=cached" ]
	codex_prompt_choice "digits> " digits 4 <<<"4" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "4" ]
! codex_prompt_choice "digits> " digits 4 <<<"5" 2>>"$2/prompt.err"
codex_prompt_choice "yn> " yn 0 <<<"y" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "y" ]
! codex_prompt_choice "yn> " yn 0 <<<"x" 2>>"$2/prompt.err"
codex_prompt_choice "freeform> " freeform 12 <<<"alpha" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "alpha" ]
' _ "$LIB_SH" "$TMP_DIR" || fail 'profile shell wrappers changed behavior'

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
bash -lc '
. "$1"
remote_arg=""
cached_args=""
version_count=0
CODEX_USE_LAST_LATEST="0.142.5"
codex_latest_linux_arm64_version() { printf "0.142.5\n"; }
codex_fail() { printf "%s\n" "$*" >&2; return 1; }
codex_version() { version_count=$((version_count + 1)); }
codex_with_lock() { local cmd="$1"; shift; "$cmd" "$@"; }
codex_runtime_install_upstream() { remote_arg="$1"; }
codex_activate_cached_runtime_unlocked() { cached_args="$*"; }
codex_termux_cmd() {
    case "$1" in
        use-select-env)
            shift
            choice=""
            while [ "$#" -gt 0 ]; do
                case "$1" in
                    --choice)
                        choice="${2:-}"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            case "$choice" in
                remote)
                    printf "%s\n" \
                        "CODEX_USE_PLAN_ACTION=install_upstream" \
                        "CODEX_USE_PLAN_RUNTIME_PATH=''" \
                        "CODEX_USE_PLAN_RAW_PATH=''" \
                        "CODEX_USE_PLAN_VERSION=0.142.5" \
                        "CODEX_USE_PLAN_RAW_SHA256=''" \
                        "CODEX_USE_PLAN_RUNTIME_SHA256=''" \
                        "CODEX_USE_PLAN_PACKAGE_SPEC=''"
                    ;;
                cached)
                    printf "%s\n" \
                        "CODEX_USE_PLAN_ACTION=activate_cached" \
                        "CODEX_USE_PLAN_RUNTIME_PATH=/runtime" \
                        "CODEX_USE_PLAN_RAW_PATH=/raw" \
                        "CODEX_USE_PLAN_VERSION=0.142.4" \
                        "CODEX_USE_PLAN_RAW_SHA256=raw" \
                        "CODEX_USE_PLAN_RUNTIME_SHA256=runtime" \
                        "CODEX_USE_PLAN_PACKAGE_SPEC=@openai/codex@0.142.4-linux-arm64"
                    ;;
                *) return 1 ;;
            esac
            ;;
        *) return 2 ;;
    esac
}
codex_use_select remote
[ "$remote_arg" = "0.142.5" ] && [ "$version_count" -eq 1 ]
	codex_use_select cached
	[ "$cached_args" = "/runtime /raw 0.142.4 raw runtime @openai/codex@0.142.4-linux-arm64" ] && [ "$version_count" -eq 2 ]
	' _ "$LIB_SH" || fail 'profile runtime selection shell executor changed behavior'

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PROFILE_ROOT="$TMP_DIR/profiles" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
bash -lc '
	. "$1"
	list_mode=""
	selected_choice=""
	codex_use_list() { list_mode="$1"; }
	codex_use_select() { selected_choice="$1"; }
	codex_termux_cmd() {
	    case "$1" in
	        use-command-plan-env)
	            shift
	            arg=""
	            while [ "$#" -gt 0 ]; do
	                case "$1" in
	                    --arg=*)
	                        arg="${1#*=}"
	                        shift
	                        ;;
	                    --arg)
	                        arg="${2:-}"
	                        shift 2
	                        ;;
	                    *)
	                        shift
	                        ;;
	                esac
	            done
	            case "$arg" in
	                "")
	                    printf "%s\n" "CODEX_USE_COMMAND_ACTION=menu" "CODEX_USE_COMMAND_CHOICE=''"
	                    ;;
	                --list)
	                    printf "%s\n" "CODEX_USE_COMMAND_ACTION=list" "CODEX_USE_COMMAND_CHOICE=''"
	                    ;;
	                *)
	                    printf "%s\n" "CODEX_USE_COMMAND_ACTION=select" "CODEX_USE_COMMAND_CHOICE=$arg"
	                    ;;
	            esac
	            ;;
	        *) return 2 ;;
	    esac
	}
	codex_use --list extra
	[ "$list_mode" = "list" ]
	codex_use cached extra
	[ "$selected_choice" = "cached" ]
	codex_use < /dev/null
	[ "$list_mode" = "menu" ]
	' _ "$LIB_SH" || fail 'profile use command plan shell dispatch changed behavior'

printf 'profile-boundary: ok\n'
