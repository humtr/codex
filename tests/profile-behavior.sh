#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'profile-behavior: FAIL: %s\n' "$*" >&2
    exit 1
}

assert_python_config() {
    local config_file="$1" expected="$2"
    python3 - "$config_file" "$expected" <<'PY'
import sys
import tomllib
from pathlib import Path

path = Path(sys.argv[1])
expected = sys.argv[2] == "true"
data = tomllib.loads(path.read_text())
actual = data["sandbox_workspace_write"]["network_access"]
if actual is not expected:
    raise SystemExit(f"expected network_access={expected}, got {actual}")
PY
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/profile-behavior-test.XXXXXX")"
mkdir -p "$FIXTURE_ROOT/home" "$FIXTURE_ROOT/shared-plugins"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_PROFILE_ROOT="$FIXTURE_ROOT/home/.codex-profiles"
export CODEX_NATIVE_SHARED_PLUGINS_DIR="$FIXTURE_ROOT/shared-plugins"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

network_profile="$FIXTURE_ROOT/network-default"
mkdir -p "$network_profile"
cat >"$network_profile/config.toml" <<'EOF'
[sandbox_workspace_write]
network_access = false
preserved = "yes"

[other]
value = 1
EOF
codex_profile_enable_network_access "$network_profile"
first_config="$(cat "$network_profile/config.toml")"
codex_profile_enable_network_access "$network_profile"
[ "$(cat "$network_profile/config.toml")" = "$first_config" ] \
    || fail "network config changed on repeated application"
assert_python_config "$network_profile/config.toml" true
python3 - "$network_profile/config.toml" <<'PY'
import sys
import tomllib
from pathlib import Path

data = tomllib.loads(Path(sys.argv[1]).read_text())
assert data["sandbox_workspace_write"]["preserved"] == "yes"
assert data["other"]["value"] == 1
PY

opt_out_profile="$FIXTURE_ROOT/network-opt-out"
mkdir -p "$opt_out_profile"
cat >"$opt_out_profile/config.toml" <<'EOF'
[sandbox_workspace_write]
network_access = false
EOF
CODEX_NATIVE_PROFILE_NETWORK_ACCESS=0
codex_profile_enable_network_access "$opt_out_profile"
assert_python_config "$opt_out_profile/config.toml" false
CODEX_NATIVE_PROFILE_NETWORK_ACCESS=1

missing_profile="$FIXTURE_ROOT/plugins-missing"
mkdir -p "$missing_profile"
codex_profile_share_plugins "$missing_profile"
[ -L "$missing_profile/plugins" ] || fail "missing plugins entry did not become a symlink"
[ "$(readlink "$missing_profile/plugins")" = "$CODEX_NATIVE_SHARED_PLUGINS_DIR" ] \
    || fail "plugins symlink points to the wrong shared directory"
codex_profile_share_plugins "$missing_profile"
[ "$(readlink "$missing_profile/plugins")" = "$CODEX_NATIVE_SHARED_PLUGINS_DIR" ] \
    || fail "plugins symlink changed on repeated application"

file_profile="$FIXTURE_ROOT/plugins-file"
mkdir -p "$file_profile"
printf 'local plugins\n' >"$file_profile/plugins"
codex_profile_share_plugins "$file_profile"
[ -f "$file_profile/plugins" ] && [ ! -L "$file_profile/plugins" ] \
    || fail "existing plugins file was replaced"
[ "$(cat "$file_profile/plugins")" = "local plugins" ] || fail "existing plugins file changed"

directory_profile="$FIXTURE_ROOT/plugins-directory"
mkdir -p "$directory_profile/plugins"
printf 'keep\n' >"$directory_profile/plugins/marker"
codex_profile_share_plugins "$directory_profile"
[ -d "$directory_profile/plugins" ] && [ ! -L "$directory_profile/plugins" ] \
    || fail "existing plugins directory was replaced"
[ "$(cat "$directory_profile/plugins/marker")" = "keep" ] \
    || fail "existing plugins directory changed"

symlink_profile="$FIXTURE_ROOT/plugins-symlink"
alternate_plugins="$FIXTURE_ROOT/alternate-plugins"
mkdir -p "$symlink_profile" "$alternate_plugins"
ln -s "$alternate_plugins" "$symlink_profile/plugins"
codex_profile_share_plugins "$symlink_profile"
[ -L "$symlink_profile/plugins" ] || fail "existing plugins symlink was replaced"
[ "$(readlink "$symlink_profile/plugins")" = "$alternate_plugins" ] \
    || fail "existing plugins symlink target changed"

printf 'profile-behavior: ok\n'
