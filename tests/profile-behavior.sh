#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'profile-behavior: FAIL: %s\n' "$*" >&2
    exit 1
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

profile_config="$FIXTURE_ROOT/profile-config"
mkdir -p "$profile_config"
cat >"$profile_config/config.toml" <<'EOF'
sandbox_mode = "workspace-write"
approval_policy = "on-request"
approvals_reviewer = "user"

[sandbox_workspace_write]
network_access = false
preserved = "yes"

[other]
value = 1
EOF
config_before="$(sha256sum "$profile_config/config.toml")"
fake_runtime="$FIXTURE_ROOT/fake-runtime"
printf '#!%s\nprintf '\''%%s\\n'\'' "$CODEX_HOME"\n' "$(command -v sh)" >"$fake_runtime"
chmod 755 "$fake_runtime"
profile_output="$(
    codex_ensure_runtime_ready() { return 0; }
    codex_auto_update_if_needed() { return 0; }
    codex_prepare_runtime_env() { return 0; }
    CODEX_NATIVE_RUNTIME="$fake_runtime"
    codex_profile_exec "$profile_config"
)"
[ "$profile_output" = "$profile_config" ] || fail "profile execution selected the wrong CODEX_HOME"
[ "$(sha256sum "$profile_config/config.toml")" = "$config_before" ] \
    || fail "profile execution changed config.toml"
if declare -F codex_profile_enable_network_access >/dev/null; then
    fail "legacy profile network mutation function still exists"
fi

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
