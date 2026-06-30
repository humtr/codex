#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() {
    printf '== %s ==\n' "$*"
}

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

clean_bytecode_noise() {
    find "$ROOT_DIR" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -exec rm -rf {} + 2>/dev/null || true
}

assert_no_bytecode_noise() {
    local found
    found="$(find "$ROOT_DIR" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -print -quit 2>/dev/null || true)"
    [ -z "$found" ] || fail "Python bytecode artifact was created: $found"
}

run_static_checks() {
    say static
    cd "$ROOT_DIR"
    clean_bytecode_noise
    bash -n install.sh
    bash -n bin/install-runtime.sh
    bash -n lib/codex-termux.sh
    bash -n tools/rg-termux-shim.sh
    bash -n tools/smoke-termux-wrapper.sh
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B - <<'PYTHON'
import pathlib

for pattern in ("tools/*.py", "tools/codex_termux/*.py"):
    for path in sorted(pathlib.Path(".").glob(pattern)):
        source = path.read_text(encoding="utf-8")
        compile(source, str(path), "exec")
PYTHON
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B -m codex_termux.cli validate --root "$ROOT_DIR"
    assert_no_bytecode_noise
}


run_non_installed_check() {
    say non-installed
    cd "$ROOT_DIR"
    if grep -R -- 'smoke-termux-wrapper.sh' install.sh bin lib >/dev/null 2>&1; then
        fail 'smoke check is referenced from installed wrapper paths'
    fi
}

run_profile_contract_checks() {
    say profile-contract
    local tmp contract_script
    tmp="$(mktemp -d)"
    mkdir -p "$tmp/home" "$tmp/prefix" "$tmp/root" "$tmp/state" "$tmp/profiles"
    contract_script="$tmp/profile-contract.sh"
    cat >"$contract_script" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
TMP_DIR="$2"

fail_contract() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

. "$ROOT_DIR/lib/codex-termux.sh"

codex_ensure_runtime_ready() { return 0; }
codex_auto_update_if_needed() { return 0; }
codex_exec_current_runtime() {
    printf 'runtime CODEX_HOME=%s\n' "${CODEX_HOME-__UNSET__}"
    return 0
}

unset CODEX_HOME
default_output="$(codex_profile_exec "$(codex_profile_home_dir default)" default 2>/dev/null)"
[ "$default_output" = 'runtime CODEX_HOME=__UNSET__' ] || fail_contract "default profile changed CODEX_HOME: $default_output"
[ ! -e "$CODEX_TERMUX_HOME/.codex" ] || fail_contract 'default profile created managed home directory'

work_dir="$(codex_profile_home_dir work)"
mkdir -p "$work_dir"
unset CODEX_HOME
work_output="$(codex_profile_exec "$work_dir" work 2>/dev/null)"
[ "$work_output" = "runtime CODEX_HOME=$work_dir" ] || fail_contract "custom profile did not set CODEX_HOME: $work_output"

    mkdir -p \
        "$CODEX_TERMUX_PROFILE_ROOT/clean" \
        "$CODEX_TERMUX_PROFILE_ROOT/beta" \
        "$CODEX_TERMUX_PROFILE_ROOT/-bad" \
        "$CODEX_TERMUX_PROFILE_ROOT/bad name" \
        "$CODEX_TERMUX_PROFILE_ROOT/foo..bar" \
        "$CODEX_TERMUX_PROFILE_ROOT/.hidden" \
        "$CODEX_TERMUX_PROFILE_ROOT/termux"
list_output="$(codex_list_profiles)"
printf '%s\n' "$list_output" | grep -Fx -- 'clean' >/dev/null || fail_contract 'valid profile is missing from list'
printf '%s\n' "$list_output" | grep -Fx -- 'work' >/dev/null || fail_contract 'work profile is missing from list'
command_output="$(codex_profile_list_command)"
printf '%s\n' "$command_output" | grep -Fx -- 'default' >/dev/null || fail_contract 'profile list command omitted default'
printf '%s\n' "$command_output" | grep -Fx -- 'clean' >/dev/null || fail_contract 'profile list command omitted valid profile'
printf '%s\n' "$command_output" | grep -Fx -- 'work' >/dev/null || fail_contract 'profile list command omitted work profile'
for invalid_profile in '-bad' 'bad name' 'foo..bar' '.hidden' 'termux'; do
    if printf '%s\n' "$list_output" | grep -Fx -- "$invalid_profile" >/dev/null; then
        fail_contract "invalid profile leaked into list: $invalid_profile"
    fi
    if printf '%s\n' "$command_output" | grep -Fx -- "$invalid_profile" >/dev/null; then
        fail_contract "invalid profile leaked into profile list command: $invalid_profile"
    fi
done

    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf 'work\n' >"$CODEX_TERMUX_LAST_PROFILE_FILE"
    menu_ids="$(codex_profile_menu_items)"
    expected_menu_ids="$(printf 'default\nbeta\nclean\nwork\n')"
    [ "$menu_ids" = "$expected_menu_ids" ] || fail_contract "recent profile changed menu order: $menu_ids"

missing_dir="$(codex_profile_home_dir missing)"
set +e
codex_profile_exec "$missing_dir" missing >"$TMP_DIR/missing.out" 2>"$TMP_DIR/missing.err" </dev/null
missing_status=$?
set -e
[ "$missing_status" -eq 2 ] || fail_contract "missing profile status was $missing_status"
[ ! -e "$missing_dir" ] || fail_contract 'missing profile was created without confirmation'

trace_file="$TMP_DIR/runtime-trace"
mkdir -p "$CODEX_TERMUX_STATE_DIR"
printf 'work\n' >"$CODEX_TERMUX_LAST_PROFILE_FILE"
: >"$trace_file"
codex_exec_current_runtime() {
    printf 'runtime CODEX_HOME=%s ARGS=%s\n' "${CODEX_HOME-__UNSET__}" "$*" >>"$trace_file"
    return 0
}

unset CODEX_HOME
codex_main resume s-alpha >/dev/null 2>&1
bare_trace_output="$(cat "$trace_file")"
[ "$bare_trace_output" = "runtime CODEX_HOME=$work_dir ARGS=resume s-alpha" ] || fail_contract "bare command stopped using recent profile: $bare_trace_output"

codex_profile_runtime_exec() {
    printf 'profile-runtime-used\n' >>"$trace_file"
    codex_exec_current_runtime "$@"
}

: >"$trace_file"
CODEX_HOME="$work_dir" codex_main resume s-alpha >/dev/null 2>&1
explicit_trace_output="$(cat "$trace_file")"
[ "$explicit_trace_output" = "runtime CODEX_HOME=$work_dir ARGS=resume s-alpha" ] || fail_contract "explicit CODEX_HOME was not preserved: $explicit_trace_output"

repair_called=0
codex_install_source_command() { return 1; }
codex_repair_public() {
    repair_called=1
}
codex_termux_main repair >/dev/null 2>&1
[ "$repair_called" -eq 1 ] || fail_contract 'termux repair command did not route to codex_repair_public'

install_called=0
codex_install_source_command() { printf '%s\n' "$ROOT_DIR/bin/install-runtime.sh"; }
codex_run_install_source_command() { install_called=1; [ "$2" = install ]; }
codex_termux_main install >/dev/null 2>&1
[ "$install_called" -eq 1 ] || fail_contract 'termux install command did not route through install source command'

install_args=""
codex_run_install_source_command() {
    shift
    install_args="$*"
}
codex_termux_main install rebuild >/dev/null 2>&1
[ "$install_args" = 'install rebuild' ] || fail_contract "termux install rebuild command did not route through install source command: $install_args"

: >"$trace_file"
CODEX_HOME="$work_dir" codex_main repair >/dev/null 2>&1
top_level_repair_trace="$(cat "$trace_file")"
[ "$top_level_repair_trace" = "runtime CODEX_HOME=$work_dir ARGS=repair" ] || fail_contract "top-level repair was not passed upstream: $top_level_repair_trace"

: >"$trace_file"
CODEX_HOME="$work_dir" codex_main setup >/dev/null 2>&1
setup_trace="$(cat "$trace_file")"
[ "$setup_trace" = "runtime CODEX_HOME=$work_dir ARGS=setup" ] || fail_contract "top-level setup was not passed upstream: $setup_trace"
BASH
    chmod +x "$contract_script"
    if CODEX_TERMUX_HOME="$tmp/home" \
        CODEX_TERMUX_PREFIX="$tmp/prefix" \
        CODEX_TERMUX_ROOT="$tmp/root" \
        CODEX_TERMUX_STATE_DIR="$tmp/state" \
        CODEX_TERMUX_PROFILE_ROOT="$tmp/profiles" \
        bash "$contract_script" "$ROOT_DIR" "$tmp"; then
        rm -rf "$tmp"
    else
        local status=$?
        rm -rf "$tmp"
        return "$status"
    fi
}

run_managed_path_guard_checks() {
    say managed-path-guard
    local tmp contract_script
    tmp="$(mktemp -d)"
    mkdir -p "$tmp/home" "$tmp/prefix" "$tmp/root" "$tmp/state"
    contract_script="$tmp/managed-path-guard.sh"
    cat >"$contract_script" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$1"
TMP_DIR="$2"

fail_guard() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

. "$ROOT_DIR/lib/codex-termux.sh"

expect_guard_failure() {
    local label="$1" root_value="$2" state_value="$3" target_value="$4" status
    CODEX_TERMUX_ROOT="$root_value"
    CODEX_TERMUX_STATE_DIR="$state_value"
    set +e
    codex_assert_managed_tree_target "$target_value" "$label" >/dev/null 2>&1
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail_guard "$label unexpectedly passed"
}

codex_assert_managed_tree_target "$CODEX_TERMUX_ROOT/current.complete.$$" good-runtime-target
codex_assert_managed_tree_target "$CODEX_TERMUX_STATE_DIR/doctor/report.json" good-state-target
mkdir -p "$CODEX_TERMUX_ROOT/remove-me"
codex_rm_rf_managed "$CODEX_TERMUX_ROOT/remove-me"
[ ! -e "$CODEX_TERMUX_ROOT/remove-me" ] || fail_guard 'managed rm did not remove safe target'
expect_guard_failure root-slash / "$TMP_DIR/state" /
expect_guard_failure root-home "$CODEX_TERMUX_HOME" "$TMP_DIR/state" "$CODEX_TERMUX_HOME"
expect_guard_failure root-prefix "$CODEX_TERMUX_PREFIX" "$TMP_DIR/state" "$CODEX_TERMUX_PREFIX"
expect_guard_failure root-relative relative/root "$TMP_DIR/state" relative/root
expect_guard_failure state-slash "$TMP_DIR/root" / /
expect_guard_failure outside-managed "$TMP_DIR/root" "$TMP_DIR/state" "$TMP_DIR/outside"
BASH
    chmod +x "$contract_script"
    if PREFIX="$tmp/prefix"         CODEX_TERMUX_HOME="$tmp/home"         CODEX_TERMUX_ROOT="$tmp/root"         CODEX_TERMUX_STATE_DIR="$tmp/state"         bash "$contract_script" "$ROOT_DIR" "$tmp"; then
        rm -rf "$tmp"
    else
        local status=$?
        rm -rf "$tmp"
        return "$status"
    fi
}

usage() {
    cat <<'EOF'
Usage: tools/smoke-termux-wrapper.sh [all|static|profile-contract|managed-path-guard|non-installed]

Runs repository-local smoke checks for the Codex Termux Wrapper. The script is
not installed by the wrapper and does not require network access for its default
checks.
EOF
}

main() {
    case "${1:-all}" in
        all)
            run_static_checks
            run_non_installed_check
            run_profile_contract_checks
            run_managed_path_guard_checks
            ;;
        static)
            run_static_checks
            ;;
        profile-contract)
            run_profile_contract_checks
            ;;
        managed-path-guard)
            run_managed_path_guard_checks
            ;;
        non-installed)
            run_non_installed_check
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            usage >&2
            return 2
            ;;
    esac
    printf 'smoke: ok\n'
}

main "$@"
