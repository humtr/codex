# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_use() {
    local plan_env arg=()
    [ "$#" -eq 0 ] || arg=("--arg=$1")
    plan_env="$(codex_termux_cmd use-command-plan-env "${arg[@]}")" || return $?
    eval "$plan_env"
    case "$CODEX_USE_COMMAND_ACTION" in
        list)
            codex_use_list list; return $?
            ;;
        select)
            codex_use_select "$CODEX_USE_COMMAND_CHOICE"; return $?
            ;;
        menu)
            ;;
        *)
            codex_fail "Unknown runtime use action: $CODEX_USE_COMMAND_ACTION"
            return 1
            ;;
    esac
    codex_use_list menu
    if [ ! -t 0 ]; then
        return 0
    fi
    codex_prompt_interactive "$(codex_ui_text_get choose_runtime_prompt)" digits "${CODEX_USE_MENU_COUNT:-0}" cancel || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    codex_use_select "$choice"
}

codex_use_list() {
    local mode="${1:-list}" latest interactive_limit=0
    latest="$(codex_latest_linux_arm64_version || true)"
    CODEX_USE_LAST_LATEST="$latest"
    if [ "$mode" = "menu" ] && [ -t 0 ]; then
        interactive_limit=9
    fi
    if [ "$mode" = "list" ]; then
        CODEX_USE_MENU_COUNT=0
        codex_use_render "$latest" "$interactive_limit" "$mode"
    else
        CODEX_USE_MENU_COUNT="$(codex_use_render "$latest" "$interactive_limit" "$mode")"
    fi
}

codex_use_render() {
    local latest="$1" interactive_limit="$2" mode="$3"
    codex_status_clear
    codex_termux_cmd use-render \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --interactive-limit "$interactive_limit" \
        --mode "$mode"
}

codex_use_select() {
    local choice="$1" selected_env
    local latest="${CODEX_USE_LAST_LATEST:-}"
    if [ -z "$latest" ]; then
        latest="$(codex_latest_linux_arm64_version || true)"
    fi
    selected_env="$(codex_termux_cmd use-select-env \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --choice "$choice" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY")" || {
        codex_fail "Unknown runtime selection: $choice"
        return 1
    }
    eval "$selected_env"
    if [ "$CODEX_USE_PLAN_ACTION" = "install_upstream" ]; then
        codex_runtime_install_upstream "$CODEX_USE_PLAN_VERSION" || return $?
    elif [ "$CODEX_USE_PLAN_ACTION" = "activate_cached" ]; then
        codex_with_lock codex_activate_cached_runtime_unlocked \
            "$CODEX_USE_PLAN_RUNTIME_PATH" \
            "$CODEX_USE_PLAN_RAW_PATH" \
            "$CODEX_USE_PLAN_VERSION" \
            "$CODEX_USE_PLAN_RAW_SHA256" \
            "$CODEX_USE_PLAN_RUNTIME_SHA256" \
            "$CODEX_USE_PLAN_PACKAGE_SPEC" || return $?
    else
        codex_fail "Unknown runtime selection action: $CODEX_USE_PLAN_ACTION"
        return 1
    fi
    codex_version || return $?
}
