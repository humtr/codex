# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_repair_install_support() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_ui_step repair_support
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 CODEX_TERMUX_INSTALL_SURFACE=0 bash "$source" install support >/dev/null
}

codex_repair_diagnose_action() {
    local field="${1:-action}"
    codex_termux_cmd repair-diagnose \
        --managed-shell "$CODEX_TERMUX_MANAGED_SHELL" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --public-codex "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --support-dir "$(codex_termux_cmd support-source-dir --manager-dir "$CODEX_TERMUX_MANAGER_DIR" --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" \
        --manifest-path "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --raw-binary "$CODEX_TERMUX_RAW_VENDOR/bin/codex" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --field "$field"
}

codex_runtime_apply_action() {
    local action="$1" intent="${2:-readiness}" plan_env status=0
    plan_env="$(codex_termux_cmd runtime-action-plan-env \
        --action "$action" \
        --intent "$intent")" || return $?
    eval "$plan_env"
    [ -z "$CODEX_RUNTIME_ACTION_STEP" ] || codex_ui_step "$CODEX_RUNTIME_ACTION_STEP"
    case "$CODEX_RUNTIME_ACTION_KIND" in
        noop)
            return 0
            ;;
        refresh_metadata)
            codex_refresh_runtime_metadata || status=$?
            ;;
        restore_verified)
            codex_try_verified_rollback || status=$?
            ;;
        rebuild_cached)
            codex_runtime_install_cached || status=$?
            ;;
        error)
            codex_fail "$CODEX_RUNTIME_ACTION_ERROR"
            return "$CODEX_RUNTIME_ACTION_EXIT_CODE"
            ;;
        *)
            codex_fail "Unknown runtime action executor: $CODEX_RUNTIME_ACTION_KIND"
            return 1
            ;;
    esac
    [ "$status" -eq 0 ] || return "$status"
    [ "$CODEX_RUNTIME_ACTION_REFRESH_AFTER" != "1" ] || codex_refresh_runtime_metadata
}

codex_repair_apply() {
    local action support_attempted=0
    while :; do
        action="$(codex_repair_diagnose_action action)" || return $?
        case "$action" in
            refresh_support)
                if [ "$support_attempted" = "1" ]; then
                    codex_fail "Support layer repair did not complete; run bash install.sh from a wrapper checkout"
                    return 1
                fi
                support_attempted=1
                codex_repair_install_support || return $?
                ;;
            *)
                codex_runtime_apply_action "$action" repair
                return $?
                ;;
        esac
    done
}

codex_repair_core_unlocked() {
    codex_validate_runtime_retention || return $?
    codex_repair_apply
}

codex_repair_public() {
    local status=0
    codex_with_lock codex_repair_core_unlocked || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    [ "$status" -eq 0 ] || return "$status"
    codex_version || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

codex_refresh_runtime_metadata_unlocked() {
    local metadata_current=0 plan_env
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" && metadata_current=1
    plan_env="$(codex_termux_cmd runtime-refresh-plan-env \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --metadata-current "$metadata_current")" || return $?
    eval "$plan_env"
    [ "$CODEX_RUNTIME_REFRESH_ACTION" = "activate" ] || return 0
    codex_activate_tuple_unlocked \
        "$CODEX_TERMUX_RUNTIME_DIR" \
        "$CODEX_RUNTIME_REFRESH_VERSION" \
        "$CODEX_RUNTIME_REFRESH_RAW_SHA256" \
        "$CODEX_RUNTIME_REFRESH_RUNTIME_SHA256" \
        "$CODEX_RUNTIME_REFRESH_PACKAGE_SPEC"
}

codex_refresh_runtime_metadata() {
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" && return 0
    codex_with_lock codex_refresh_runtime_metadata_unlocked
}

codex_ensure_runtime_ready() {
    local action
    action="$(codex_repair_diagnose_action readiness-action)" || return $?
    codex_runtime_apply_action "$action" readiness
}
