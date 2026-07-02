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
codex_prompt_choice "digits> " digits 4 <<<"4" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "4" ]
! codex_prompt_choice "digits> " digits 4 <<<"5" 2>>"$2/prompt.err"
codex_prompt_choice "yn> " yn 0 <<<"y" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "y" ]
! codex_prompt_choice "yn> " yn 0 <<<"x" 2>>"$2/prompt.err"
codex_prompt_choice "freeform> " freeform 12 <<<"alpha" 2>>"$2/prompt.err"
[ "$CODEX_PROMPT_CHOICE_RESULT" = "alpha" ]
' _ "$LIB_SH" "$TMP_DIR" || fail 'profile shell wrappers changed behavior'

printf 'profile-boundary: ok\n'
