# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_runtime_apply_env_plan() {
    codex_termux_cmd runtime-env-plan \
        --runtime-dir "$1" \
        --runtime-exe "$2" \
        --set-home "$3" \
        --home "$4" \
        --tmpdir "$CODEX_TERMUX_TMPDIR" \
        --cert-file "$CODEX_TERMUX_CERT_FILE" \
        --cert-dir "$CODEX_TERMUX_CERT_DIR" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --path "$PATH" \
        --browser "${BROWSER:-}" \
        --ssl-cert-file "${SSL_CERT_FILE:-}" \
        --ssl-cert-dir "${SSL_CERT_DIR:-}" \
        --xdg-config-home "${XDG_CONFIG_HOME:-}" \
        --xdg-cache-home "${XDG_CACHE_HOME:-}" \
        --xdg-data-home "${XDG_DATA_HOME:-}" \
        --godebug "${GODEBUG:-}" \
        --bwrap-quiet "${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-}" \
        --termux-open-url "$5"
}

codex_runtime_exec() {
    local executable="$1"
    shift || true
    local run_home runtime_dir runtime_env
    run_home="${CODEX_TERMUX_HOME:-$HOME}"
    case "$executable" in
        */*) runtime_dir="${executable%/*}" ;;
        *) runtime_dir="$executable" ;;
    esac
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    runtime_env="$(codex_runtime_apply_env_plan "$runtime_dir" "$executable" 1 "$run_home" 0)" || return $?
    eval "$runtime_env"
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
    codex_prepare_system_config || return $?
    "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_prepare_runtime_env() {
    local runtime_dir runtime_exe termux_open_url=0 runtime_env
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    codex_prepare_system_config || return $?
    runtime_dir="$(codex_termux_cmd resolve-path --path "$CODEX_TERMUX_RUNTIME_DIR")" || return $?
    runtime_exe="$runtime_dir/codex"
    command -v termux-open-url >/dev/null 2>&1 && termux_open_url=1
    runtime_env="$(codex_runtime_apply_env_plan "$runtime_dir" "$runtime_exe" 0 "" "$termux_open_url")" || return $?
    eval "$runtime_env"
}

codex_run_current_runtime() {
    codex_status_clear
    codex_prepare_runtime_env || return $?
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
    "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_exec_current_runtime() {
    codex_status_clear
    codex_prepare_runtime_env || return $?
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
    exec "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}
