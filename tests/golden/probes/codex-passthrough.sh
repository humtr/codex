#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
SANDBOX="$2"
# shellcheck disable=SC1090
. "$ROOT_DIR/lib/codex-termux.sh"

codex_ensure_runtime_ready() {
    printf 'readiness\n' >>"$SANDBOX/processes.log"
}

codex_auto_update_if_needed() {
    printf 'auto-update\n' >>"$SANDBOX/processes.log"
}

codex_runtime_exec_with_context() {
    local arg
    printf 'runtime'
    for arg in "$@"; do
        printf ' <%s>' "$arg"
    done
    printf '\n'
    {
        printf 'runtime'
        for arg in "$@"; do
            printf ' %s' "$arg"
        done
        printf '\n'
    } >>"$SANDBOX/processes.log"
}

codex_main run --flag
