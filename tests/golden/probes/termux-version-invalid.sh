#!/usr/bin/env bash
set -u

ROOT_DIR="$1"
# shellcheck disable=SC1090
. "$ROOT_DIR/lib/codex-termux.sh"

codex_termux_main version junk
exit $?
