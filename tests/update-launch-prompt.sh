#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'update-launch-prompt: FAIL: %s\n' "$*" >&2
    exit 1
}

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-interactive.sh"

launch_calls=0
launch_profile=""
launch_profile_dir=""

codex_profile_read_recent() {
    printf 'default\n'
}

codex_profile_dir() {
    printf '%s\n' "/tmp/codex-profile"
}

codex_profile_runtime_exec() {
    launch_calls=$((launch_calls + 1))
    launch_profile="$1"
    launch_profile_dir="$2"
    return 0
}

CODEX_PROMPT_CHOICE_RESULT=y
codex_update_launch_selected 0.139.0 || fail "yes choice did not reach launch path"
[ "$launch_calls" -eq 1 ] || fail "yes choice did not launch runtime"
[ "$launch_profile" = "default" ] || fail "yes choice used the wrong profile"
[ "$launch_profile_dir" = "/tmp/codex-profile" ] || fail "yes choice used the wrong profile dir"

CODEX_PROMPT_CHOICE_RESULT=n
codex_update_launch_selected 0.139.0 || fail "no choice unexpectedly failed"
[ "$launch_calls" -eq 1 ] || fail "no choice should not launch runtime"

CODEX_PROMPT_CHOICE_RESULT=""
codex_update_launch_selected 0.139.0 || fail "empty choice unexpectedly failed"
[ "$launch_calls" -eq 1 ] || fail "empty choice should not launch runtime"

printf 'update-launch-prompt: ok\n'
