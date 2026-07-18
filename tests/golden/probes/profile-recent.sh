#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
# shellcheck disable=SC1090
. "$ROOT_DIR/lib/codex-termux.sh"

codex_termux_cmd profile-write-recent --profile work
codex_termux_cmd profile-read-recent
