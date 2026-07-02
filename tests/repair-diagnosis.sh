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
import json
import tempfile
from pathlib import Path

from codex_termux import repair, runtime_checks, schemas


def action(**checks: bool) -> str:
    return repair.action_from_checks(
        support_ok=checks.get("support_ok", True),
        runtime_ok=checks.get("runtime_ok", True),
        metadata_current=checks.get("metadata_current", True),
        verified_rollback_available=checks.get("verified_rollback_available", False),
        raw_ok=checks.get("raw_ok", True),
    )


def readiness(**checks: bool) -> str:
    return repair.readiness_action_from_checks(
        runtime_ok=checks.get("runtime_ok", True),
        metadata_current=checks.get("metadata_current", True),
        verified_rollback_available=checks.get("verified_rollback_available", False),
        raw_available=checks.get("raw_available", True),
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
assert readiness(runtime_ok=True, metadata_current=True) == repair.ACTION_READY
assert readiness(runtime_ok=True, metadata_current=False) == repair.ACTION_REFRESH_METADATA
assert (
    readiness(runtime_ok=False, verified_rollback_available=True)
    == repair.ACTION_RESTORE_VERIFIED
)
assert (
    readiness(runtime_ok=False, verified_rollback_available=False, raw_available=True, raw_ok=True)
    == repair.ACTION_REBUILD_CACHED
)
assert (
    readiness(runtime_ok=False, verified_rollback_available=False, raw_available=True, raw_ok=False)
    == repair.ACTION_RAW_CORRUPT
)
assert (
    readiness(runtime_ok=False, verified_rollback_available=False, raw_available=False, raw_ok=False)
    == repair.ACTION_MISSING_RUNTIME
)
assert set(repair.REPAIR_ACTIONS) == {
    repair.ACTION_NONE,
    repair.ACTION_REFRESH_SUPPORT,
    repair.ACTION_REFRESH_METADATA,
    repair.ACTION_RESTORE_VERIFIED,
    repair.ACTION_REBUILD_CACHED,
    repair.ACTION_UNRECOVERABLE,
}
assert set(repair.READINESS_ACTIONS) == {
    repair.ACTION_READY,
    repair.ACTION_REFRESH_METADATA,
    repair.ACTION_RESTORE_VERIFIED,
    repair.ACTION_REBUILD_CACHED,
    repair.ACTION_RAW_CORRUPT,
    repair.ACTION_MISSING_RUNTIME,
}

plan = repair.runtime_action_plan
assert plan(repair.ACTION_NONE, "repair").kind == repair.PLAN_NOOP
assert plan(repair.ACTION_READY, "readiness").kind == repair.PLAN_NOOP
metadata = plan(repair.ACTION_REFRESH_METADATA, "repair")
assert metadata.kind == repair.PLAN_REFRESH_METADATA
assert metadata.step == "repair_metadata"
assert metadata.refresh_after is False
restore = plan(repair.ACTION_RESTORE_VERIFIED, "readiness")
assert restore.kind == repair.PLAN_RESTORE_VERIFIED
assert restore.refresh_after is True
rebuild_repair = plan(repair.ACTION_REBUILD_CACHED, "repair")
assert rebuild_repair.kind == repair.PLAN_REBUILD_CACHED
assert rebuild_repair.step == "repair_runtime"
assert rebuild_repair.refresh_after is True
rebuild_ready = plan(repair.ACTION_REBUILD_CACHED, "readiness")
assert rebuild_ready.kind == repair.PLAN_REBUILD_CACHED
assert rebuild_ready.step == "rebuild_cached_runtime"
assert rebuild_ready.refresh_after is False
raw_corrupt = plan(repair.ACTION_RAW_CORRUPT, "readiness")
assert raw_corrupt.kind == repair.PLAN_ERROR
assert raw_corrupt.exit_code == 1
assert "cached raw" in raw_corrupt.error.lower()
missing = plan(repair.ACTION_MISSING_RUNTIME, "readiness")
assert missing.kind == repair.PLAN_ERROR
assert missing.exit_code == 127
unknown = plan("future_action", "readiness")
assert unknown.kind == repair.PLAN_ERROR
assert unknown.exit_code == 1

with tempfile.TemporaryDirectory() as tmp:
    state_file = Path(tmp) / "state.json"
    state = schemas.build_state_v3(
        version="0.142.4",
        raw_sha256="a" * 64,
        runtime_sha256="b" * 64,
        package_spec="@openai/codex@0.142.4-linux-arm64",
        active_tuple_id="tuple-a",
        wrapper_version="260702-54",
        wrapper_commit="abcdef123456",
        updated_at="2026-07-02T00:00:00Z",
        verified_tuple_id="tuple-a",
        verified_at="2026-07-02T00:00:00Z",
    )
    state_file.write_text(json.dumps(state), encoding="utf-8")
    cached = runtime_checks.runtime_cached_build_plan_exports(state_file)
    assert "CODEX_RUNTIME_CACHED_VERSION=0.142.4" in cached
    assert "CODEX_RUNTIME_CACHED_PACKAGE_SPEC=@openai/codex@0.142.4-linux-arm64" in cached
    refresh = runtime_checks.runtime_refresh_plan_exports(state_file, metadata_current=False)
    assert "CODEX_RUNTIME_REFRESH_ACTION=activate" in refresh
    assert "CODEX_RUNTIME_REFRESH_RAW_SHA256=" + ("a" * 64) in refresh
    current = runtime_checks.runtime_refresh_plan_exports(state_file, metadata_current=True)
    assert "CODEX_RUNTIME_REFRESH_ACTION=none" in current

with tempfile.TemporaryDirectory() as tmp:
    missing = Path(tmp) / "missing.json"
    cached = runtime_checks.runtime_cached_build_plan_exports(missing)
    assert "CODEX_RUNTIME_CACHED_VERSION=unknown" in cached
    assert "CODEX_RUNTIME_CACHED_PACKAGE_SPEC=local" in cached
    refresh = runtime_checks.runtime_refresh_plan_exports(missing, metadata_current=False)
    assert "CODEX_RUNTIME_REFRESH_ACTION=skip" in refresh
PYTHON

[ "$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-action-plan \
    --action rebuild_cached --intent repair --field step)" = "repair_runtime" ] ||
    fail 'runtime action plan CLI did not return repair step'

[ "$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-action-plan \
    --action restore_verified --intent readiness --field refresh-after)" = "1" ] ||
    fail 'runtime action plan CLI did not return refresh-after'

action_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli runtime-action-plan-env \
            --action rebuild_cached --intent repair
)"
eval "$action_env"
[ "$CODEX_RUNTIME_ACTION_KIND" = "rebuild_cached" ] ||
    fail "runtime action plan env kind mismatch: $CODEX_RUNTIME_ACTION_KIND"
[ "$CODEX_RUNTIME_ACTION_STEP" = "repair_runtime" ] ||
    fail "runtime action plan env step mismatch: $CODEX_RUNTIME_ACTION_STEP"
[ "$CODEX_RUNTIME_ACTION_REFRESH_AFTER" = "1" ] ||
    fail "runtime action plan env refresh mismatch: $CODEX_RUNTIME_ACTION_REFRESH_AFTER"

state_file="$TMP_DIR/state.json"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - "$state_file" <<'PYTHON'
import json
import sys
from pathlib import Path

from codex_termux import schemas

Path(sys.argv[1]).write_text(json.dumps(schemas.build_state_v3(
    version="0.142.4",
    raw_sha256="a" * 64,
    runtime_sha256="b" * 64,
    package_spec="@openai/codex@0.142.4-linux-arm64",
    active_tuple_id="tuple-a",
    wrapper_version="260702-54",
    wrapper_commit="abcdef123456",
    updated_at="2026-07-02T00:00:00Z",
    verified_tuple_id="tuple-a",
    verified_at="2026-07-02T00:00:00Z",
)), encoding="utf-8")
PYTHON
[ "$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-cached-build-plan-env \
    --state-file "$state_file" | sed -n '1p')" = "CODEX_RUNTIME_CACHED_VERSION=0.142.4" ] ||
    fail 'runtime cached build plan CLI did not return version'

[ "$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-refresh-plan-env \
    --state-file "$state_file" --metadata-current 0 | sed -n '1p')" = "CODEX_RUNTIME_REFRESH_ACTION=activate" ] ||
    fail 'runtime refresh plan CLI did not return activate action'

diagnosis_json="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli repair-diagnose \
            --managed-shell "$TMP_DIR/missing-managed.sh" \
            --manager-dir "$TMP_DIR/manager" \
            --public-codex "$TMP_DIR/codex" \
            --marker 'codex termux managed launcher' \
            --runtime-dir "$TMP_DIR/current" \
            --runtime "$TMP_DIR/current/codex" \
            --support-dir "$TMP_DIR/manager" \
            --manifest-path "$TMP_DIR/current/runtime-build.json" \
            --builder "$TMP_DIR/manager/build-runtime.py" \
            --state-path "$TMP_DIR/state.json" \
            --registry-path "$TMP_DIR/registry.json" \
            --current "$TMP_DIR/current" \
            --verified "$TMP_DIR/verified" \
            --raw "$TMP_DIR/raw" \
            --raw-binary "$TMP_DIR/raw/vendor/aarch64-unknown-linux-musl/bin/codex" \
            --patch-policy termux-fd-remap-v1
)" || fail 'repair-diagnose CLI failed to return diagnosis JSON'
PYTHONDONTWRITEBYTECODE=1 python3 -B - "$diagnosis_json" <<'PYTHON' || fail 'repair-diagnose CLI diagnosis JSON mismatch'
import json
import sys

data = json.loads(sys.argv[1])
assert data["support_ok"] is False
assert data["runtime_ok"] is False
assert data["raw_available"] is False
assert data["action"] == "refresh_support"
assert data["readiness_action"] == "missing_runtime"
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
steps=""
codex_repair_install_support() { support=$((support + 1)); }
codex_refresh_runtime_metadata() { metadata=$((metadata + 1)); }
codex_try_verified_rollback() { rollback=$((rollback + 1)); }
codex_runtime_install_cached() { cached=$((cached + 1)); }
codex_ui_step() { steps="${steps:+$steps }$1"; }
set_actions refresh_support none
codex_repair_apply
[ "$support" -eq 1 ] && [ "$metadata" -eq 0 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ] && [ "$steps" = "" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions refresh_metadata
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ] && [ "$steps" = "repair_metadata" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions restore_verified
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 1 ] && [ "$cached" -eq 0 ] && [ "$steps" = "" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions rebuild_cached
codex_repair_apply
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 1 ] && [ "$steps" = "repair_runtime" ]

codex_repair_diagnose_action() {
    case "$1" in
        readiness-action)
            local action
            action="$(sed -n "1p" "$action_file")"
            sed -n "2,\$p" "$action_file" >"$action_file.next"
            mv "$action_file.next" "$action_file"
            printf "%s\n" "$action"
            ;;
        *)
            return 2
            ;;
    esac
}

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions refresh_metadata
codex_ensure_runtime_ready
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ] && [ "$steps" = "" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions restore_verified
codex_ensure_runtime_ready
[ "$support" -eq 0 ] && [ "$metadata" -eq 1 ] && [ "$rollback" -eq 1 ] && [ "$cached" -eq 0 ] && [ "$steps" = "" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions rebuild_cached
codex_ensure_runtime_ready
[ "$support" -eq 0 ] && [ "$metadata" -eq 0 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 1 ] && [ "$steps" = "rebuild_cached_runtime" ]

support=0; metadata=0; rollback=0; cached=0; steps=""
set_actions ready
codex_ensure_runtime_ready
[ "$support" -eq 0 ] && [ "$metadata" -eq 0 ] && [ "$rollback" -eq 0 ] && [ "$cached" -eq 0 ] && [ "$steps" = "" ]
' _ "$LIB_SH" "$TMP_DIR" || fail 'shell repair action flow failed'

if rg -n 'CODEX_REPAIR_NEEDS_' "$ROOT_DIR/lib" >/dev/null; then
    fail 'repair diagnosis leaked shell global flags'
fi

rg -n 'repair-diagnose' "$ROOT_DIR/lib/codex-termux/runtime.sh" >/dev/null ||
    fail 'runtime repair does not delegate diagnosis to Python'

rg -n 'runtime-action-plan' "$ROOT_DIR/lib/codex-termux/runtime.sh" >/dev/null ||
    fail 'runtime repair does not delegate action planning to Python'

if rg -n 'raw_corrupt|missing_runtime|unrecoverable|Cached raw package integrity|no cached raw package|Runtime is damaged' \
    "$ROOT_DIR/lib/codex-termux/runtime.sh" >/dev/null; then
    fail 'runtime action policy leaked back into shell'
fi

printf 'repair-diagnosis: ok\n'
