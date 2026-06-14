#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'doctor-contract: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/doctor-contract-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

fake_runtime="$FIXTURE_ROOT/codex"
printf '#!%s\nprintf '\''upstream:%%s\\n'\'' "$*"\n' "$(command -v sh)" >"$fake_runtime"
chmod 755 "$fake_runtime"

export CODEX_NATIVE_RUNTIME="$fake_runtime"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

codex_wrapper_doctor_json() {
    cat <<'JSON'
{
  "activeTupleId": "tuple",
  "buildManifest": {"patch_policy": "dns-fd33-only-v1"},
  "checks": {
    "build_manifest": true,
    "bundled_bwrap": true,
    "bwrap_exec": true,
    "cert": true,
    "dns_only_patch": true,
    "network_boundary": true,
    "path_bwrap": true,
    "raw": true,
    "raw_hash": true,
    "registry": true,
    "registry_active_tuple": true,
    "resolv": true,
    "rg": true,
    "rg_exec": true,
    "rg_real": true,
    "runtime": true,
    "runtime_hash": true,
    "state": true,
    "support_bwrap_match": true,
    "support_rg_match": true,
    "zsh": true
  },
  "networkBoundary": {
    "checks": {
      "baseline_socket": true,
      "network_off": true,
      "network_on": true,
      "network_reset": true
    },
    "overallStatus": "ok"
  },
  "overallStatus": "ok",
  "paths": {
    "raw_vendor": "/raw",
    "runtime": "/runtime"
  },
  "raw_sha256": "rawhash",
  "runtime_sha256": "runtimehash",
  "version": "test"
}
JSON
}
wrapper_human="$(codex_wrapper_doctor)"
[[ "$wrapper_human" == "Termux Wrapper Doctor"* ]] \
    || fail "wrapper doctor did not render a title"
[[ "$wrapper_human" == *$'\nRuntime\n'* ]] \
    || fail "wrapper doctor did not render runtime section"
[[ "$wrapper_human" == *"Wrapper status: ok"* ]] \
    || fail "wrapper doctor did not render final status"
case "$wrapper_human" in
    "{"*) fail "wrapper doctor default output is still raw JSON" ;;
esac

codex_ensure_runtime_ready() { return 0; }
codex_prepare_runtime_env() { return 0; }
codex_wrapper_doctor() { printf 'wrapper\n'; }

human="$(codex_public_doctor)"
[ "$human" = $'upstream:doctor\n\nwrapper' ] \
    || fail "default doctor did not compose upstream and wrapper output"
json="$(codex_public_doctor --json)"
[ "$json" = "upstream:doctor --json" ] \
    || fail "doctor arguments were not passed directly to upstream"

printf 'doctor-contract: ok\n'
