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
bash -lc '
. "$1"
[ "$(codex_termux_cmd profile-choice-to-name --choice home)" = "default" ]
[ "$(codex_profile_home_dir default)" = "$CODEX_TERMUX_HOME/.codex" ]
[ "$(codex_profile_home_dir team)" = "$CODEX_TERMUX_PROFILE_ROOT/team" ]
codex_profile_name_valid team
! codex_profile_name_valid termux
! codex_profile_name_valid "../bad"
codex_termux_cmd profile-write-recent --profile team
[ "$(codex_termux_cmd profile-read-recent)" = "team" ]
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
	codex_use
	[ "$list_mode" = "menu" ]
	' _ "$LIB_SH" || fail 'profile use command plan shell dispatch changed behavior'

printf 'profile-boundary: ok\n'
