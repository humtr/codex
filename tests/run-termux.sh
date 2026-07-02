#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'run-termux: FAIL: %s\n' "$*" >&2
    exit 1
}

is_termux() {
    case "${PREFIX:-}" in
        /data/data/com.termux/files/usr)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

doctor_check() {
    local label="$1" json_file="$2"
    local require_tuple_match="${3:-0}"
    codex termux doctor --json >"$json_file"
    cat "$json_file"
    PYTHONDONTWRITEBYTECODE=1 python3 -B - \
        "$label" "$json_file" \
        "$CODEX_TERMUX_EXPECT_WRAPPER_VERSION" \
        "$CODEX_TERMUX_EXPECT_WRAPPER_COMMIT" \
        "$CODEX_TERMUX_REQUIRE_CHECKOUT_MATCH" \
        "$require_tuple_match" <<'PYTHON'
import json
import sys
from pathlib import Path

label, json_path, expected_version, expected_commit, require_match, require_tuple_match = sys.argv[1:7]
data = json.loads(Path(json_path).read_text(encoding="utf-8"))
status = data.get("overallStatus")
if status != "ok":
    raise SystemExit(f"{label}: expected overallStatus ok, got {status!r}")
wrapper = data.get("wrapper")
if not isinstance(wrapper, dict):
    raise SystemExit(f"{label}: doctor report missing wrapper object")
for field in ("version", "commit"):
    if not wrapper.get(field):
        raise SystemExit(f"{label}: doctor report missing wrapper.{field}")
for field in ("activeTupleId", "verifiedTupleId"):
    if not data.get(field):
        raise SystemExit(f"{label}: doctor report missing {field}")
checks = data.get("checks")
if not isinstance(checks, dict):
    raise SystemExit(f"{label}: doctor report missing checks object")
for field in ("raw_hash", "runtime_hash", "registry_active_tuple"):
    if checks.get(field) is not True:
        raise SystemExit(f"{label}: doctor check {field} is not true")
if require_match == "1":
    actual_version = wrapper.get("version")
    actual_commit = wrapper.get("commit")
    if expected_version and actual_version != expected_version:
        raise SystemExit(
            f"{label}: installed wrapper version {actual_version!r} "
            f"does not match checkout version {expected_version!r}"
        )
    if expected_commit and actual_commit != expected_commit:
        raise SystemExit(
            f"{label}: installed wrapper commit {actual_commit!r} "
            f"does not match checkout commit {expected_commit!r}; "
            "run 'bash bin/install-local.sh support' from this checkout"
        )
if require_tuple_match == "1":
    expected_parts = [item for item in (expected_version, expected_commit) if item]
    for field in ("activeTupleId", "verifiedTupleId"):
        value = str(data.get(field) or "")
        for expected in expected_parts:
            if expected not in value:
                raise SystemExit(
                    f"{label}: {field} {value!r} does not include checkout wrapper marker {expected!r}; "
                    "re-run local-source rebuild smoke"
                )
PYTHON
}

bool_env() {
    local name="$1" value="$2"
    case "$value" in
        1|true|TRUE|yes|YES)
            printf '1\n'
            ;;
        0|false|FALSE|no|NO|"")
            printf '0\n'
            ;;
        *)
            fail "invalid $name value: $value"
            ;;
    esac
}

if ! is_termux; then
    cat >&2 <<'EOF'
run-termux: Termux live validation requires a real Termux environment.
Run this command on the target device:

  bash tests/run-termux.sh

For portable GitHub/desktop validation, run:

  bash tests/run-portable.sh
EOF
    exit 2
fi

command -v codex >/dev/null 2>&1 || fail 'codex command is not installed on PATH'

CODEX_TERMUX_EXPECT_WRAPPER_VERSION="${CODEX_TERMUX_EXPECT_WRAPPER_VERSION:-$(sed -n 's/^CODEX_TERMUX_WRAPPER_VERSION=//p' "$ROOT_DIR/config/wrapper-version.env" | head -n 1)}"
CODEX_TERMUX_EXPECT_WRAPPER_COMMIT="${CODEX_TERMUX_EXPECT_WRAPPER_COMMIT:-$(git -C "$ROOT_DIR" rev-parse --short=12 HEAD 2>/dev/null || true)}"
CODEX_TERMUX_REQUIRE_CHECKOUT_MATCH="$(bool_env CODEX_TERMUX_REQUIRE_CHECKOUT_MATCH "${CODEX_TERMUX_REQUIRE_CHECKOUT_MATCH:-1}")"
CODEX_TERMUX_AUTO_UPDATE="$(bool_env CODEX_TERMUX_AUTO_UPDATE "${CODEX_TERMUX_AUTO_UPDATE:-0}")"
[ "$CODEX_TERMUX_AUTO_UPDATE" = "0" ] || fail 'run-termux requires CODEX_TERMUX_AUTO_UPDATE=0 for network-disabled smoke'
export CODEX_TERMUX_AUTO_UPDATE

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

printf '== termux-version ==\n'
codex termux version

printf '== termux-doctor ==\n'
doctor_check initial "$tmp_dir/doctor-initial.json"

printf '== codex-version ==\n'
codex --version

case "$(bool_env CODEX_TERMUX_RUN_REBUILD_SMOKE "${CODEX_TERMUX_RUN_REBUILD_SMOKE:-0}")" in
    1)
        printf '== termux-install-rebuild ==\n'
        CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="$ROOT_DIR/bin/install-runtime.sh" \
            CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$tmp_dir/wrapper-source.env" \
            CODEX_TERMUX_WRAPPER_SOURCE_DIR="$ROOT_DIR" \
            codex termux install rebuild
        printf '== termux-doctor-after-rebuild ==\n'
        doctor_check rebuild "$tmp_dir/doctor-rebuild.json" 1
        ;;
    0)
        ;;
esac

printf 'termux tests: ok\n'
