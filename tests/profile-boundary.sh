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
[ "$(codex_profile_choice_name home)" = "default" ]
[ "$(codex_profile_home_dir default)" = "$CODEX_TERMUX_HOME/.codex" ]
[ "$(codex_profile_home_dir team)" = "$CODEX_TERMUX_PROFILE_ROOT/team" ]
codex_profile_name_valid team
! codex_profile_name_valid termux
! codex_profile_name_valid "../bad"
codex_profile_recent_write team
[ "$(codex_profile_recent_read)" = "team" ]
rm -rf "$CODEX_TERMUX_PROFILE_ROOT/team"
[ "$(codex_profile_recent_read)" = "default" ]
profiles="$(codex_list_profiles)"
case "$profiles" in
    Alpha) ;;
    *) printf "profiles=%s\n" "$profiles" >&2; exit 1 ;;
esac
menu="$(codex_profile_menu_items | tr "\n" " ")"
[ "$menu" = "default Alpha " ]
[ "$(codex_termux_cmd profile-menu-choice --choice 0)" = "default" ]
[ "$(codex_termux_cmd profile-menu-choice --choice 1)" = "Alpha" ]
[ "$(codex_termux_cmd profile-menu-choice --choice home)" = "default" ]
[ "$(codex_termux_cmd profile-menu-choice --choice Alpha)" = "Alpha" ]
' _ "$LIB_SH" || fail 'profile shell wrappers changed behavior'

printf 'profile-boundary: ok\n'
