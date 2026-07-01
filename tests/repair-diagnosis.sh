#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'repair-diagnosis: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'repair action model failed'
from codex_termux import repair


def action(**checks: bool) -> str:
    return repair.action_from_checks(
        support_ok=checks.get("support_ok", True),
        runtime_ok=checks.get("runtime_ok", True),
        metadata_current=checks.get("metadata_current", True),
        verified_rollback_available=checks.get("verified_rollback_available", False),
        raw_ok=checks.get("raw_ok", True),
    )


assert action(support_ok=False) == repair.ACTION_REFRESH_SUPPORT
assert action(runtime_ok=True, metadata_current=False) == repair.ACTION_REFRESH_METADATA
assert action(runtime_ok=True, metadata_current=True) == repair.ACTION_NONE
assert (
    action(runtime_ok=False, verified_rollback_available=True)
    == repair.ACTION_RESTORE_VERIFIED
)
assert (
    action(runtime_ok=False, verified_rollback_available=False, raw_ok=True)
    == repair.ACTION_REBUILD_CACHED
)
assert (
    action(runtime_ok=False, verified_rollback_available=False, raw_ok=False)
    == repair.ACTION_UNRECOVERABLE
)
PYTHON

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_PREFIX="$TMP_DIR/prefix" \
bash -lc '
. "$1"
codex_ui_step() { :; }
codex_fail() { printf "%s\n" "$*" >&2; return 1; }

action_file="$2/actions"
mkdir -p "$2"
set_actions() {
    printf "%s\n" "$@" >"$action_file"
}
codex_repair_diagnose_action() {
    local action
    action="$(sed -n "1p" "$action_file")"
    sed -n "2,\$p" "$action_file" >"$action_file.next"
    mv "$action_file.next" "$action_file"
    printf "%s\n" "$action"
}

support=0
metadata=0
rollback=0
cached=0
codex_repair_install_support() { support=$((support + 1)); }
codex_refresh_runtime_metadata() { metadata=$((metadata + 1)); }
codex_try_verified_rollback() { rollback=$((rollback + 1)); }
codex_runtime_install_cached() { cached=$((cached + 1)); }
set_actions refresh_support none
codex_repair_apply
[ "$support" -eq 1 ] && [ "$metadata" -eq 0 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ]

support=0; metadata=0; rollback=0; cached=0
set_actions refresh_metadata
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ]

support=0; metadata=0; rollback=0; cached=0
set_actions restore_verified
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 1 ] && [ "$cached" -eq 0 ]

support=0; metadata=0; rollback=0; cached=0
set_actions rebuild_cached
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 1 ]
' _ "$LIB_SH" "$TMP_DIR" || fail 'shell repair action flow failed'

if rg -n 'CODEX_REPAIR_NEEDS_' "$ROOT_DIR/lib" >/dev/null; then
    fail 'repair diagnosis leaked shell global flags'
fi

rg -n 'repair-diagnose' "$ROOT_DIR/lib/codex-termux/runtime.sh" >/dev/null ||
    fail 'runtime repair does not delegate diagnosis to Python'

printf 'repair-diagnosis: ok\n'
