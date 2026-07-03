# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_profile_runtime_exec() {
    local profile="$1" profile_dir="$2"
    shift 2 || true
    codex_termux_cmd profile-write-recent --profile "$profile"
    codex_ui_step open_profile "$(codex_termux_cmd profile-display-name --profile "$profile")"
    if ! codex_termux_cmd profile-is-default --profile "$profile"; then
        export CODEX_HOME="$profile_dir"
    fi
    codex_exec_current_runtime "$@"
}

codex_profile_ensure_dir() {
    local profile_dir="$1" profile="${2:-default}" display status
    if codex_termux_cmd profile-is-default --profile "$profile"; then
        return 0
    fi
    if [ -d "$profile_dir" ]; then
        return 0
    fi
    display="$(codex_termux_cmd profile-display-name --profile "$profile")"
    if [ ! -t 0 ] || [ ! -t 2 ]; then
        codex_fail "$(codex_ui_text_get missing_profile "$display")"
        return 2
    fi
    codex_prompt_interactive "$(codex_ui_text_get create_profile_prompt "$display")" yn 0 cancel "$(codex_ui_text_get profile_create_cancelled)" || {
        status=$?
        return "$status"
    }
    codex_termux_cmd profile-create-confirmed --choice "$(codex_prompt_result)" || return 130
    mkdir -p "$profile_dir"
    codex_say "$(codex_ui_text_get created_profile "$display")"
}

codex_profile_exec() {
    local profile_dir="$1" profile="${2:-default}"
    shift 2 || true
    codex_profile_ensure_dir "$profile_dir" "$profile" || return $?
    codex_ensure_runtime_ready || return $?
    codex_auto_update_if_needed || return $?
    codex_profile_runtime_exec "$profile" "$profile_dir" "$@"
}

codex_runtime_exec_with_context() {
    if [ -n "${CODEX_HOME:-}" ]; then
        codex_exec_current_runtime "$@"
        return $?
    fi
    local recent_profile recent_profile_dir
    recent_profile="$(codex_termux_cmd profile-read-recent)"
    recent_profile_dir="$(codex_termux_cmd profile-dir --profile "$recent_profile")"
    codex_profile_runtime_exec "$recent_profile" "$recent_profile_dir" "$@"
}

codex_profile_select() {
    local profile choice profile_dir interactive=0 max_items
    [ -t 0 ] && interactive=1
    max_items="$(codex_termux_cmd profile-menu-render --interactive "$interactive")" || return $?

    if [ ! -t 0 ]; then
        return 0
    fi

    codex_prompt_interactive "$(codex_ui_text_get choose_profile_prompt)" freeform "$max_items" cancel || return $?
    choice="$(codex_prompt_result)"

    profile="$(codex_termux_cmd profile-menu-choice --choice "$choice")"

    codex_termux_cmd profile-validate --profile "$profile" || {
        codex_fail "$(codex_ui_text_get invalid_profile "$profile")"
        return 2
    }
    profile_dir="$(codex_termux_cmd profile-dir --profile "$profile")"
    codex_profile_exec "$profile_dir" "$profile"
}

codex_profile_run() {
    local plan_env
    plan_env="$(codex_termux_cmd profile-run-plan-env --profile "${1:-}" --argc "$#")" || return $?
    eval "$plan_env"
    case "$CODEX_PROFILE_RUN_ACTION" in
        select)
            codex_profile_select
            return $?
            ;;
        list)
            codex_status_clear
            codex_termux_cmd profile-list --include-default
            return 0
            ;;
        profile_arg_error)
            codex_fail "$(codex_ui_text_get profile_arg_error "$CODEX_PROFILE_RUN_ERROR")"
            return 2
            ;;
        invalid_profile)
            codex_fail "$(codex_ui_text_get invalid_profile "$CODEX_PROFILE_RUN_ERROR")"
            return 2
            ;;
        exec)
            ;;
        *)
            codex_fail "Unknown profile action: $CODEX_PROFILE_RUN_ACTION"
            return 1
            ;;
    esac
    local profile="$CODEX_PROFILE_RUN_PROFILE" profile_dir
    profile_dir="$(codex_termux_cmd profile-dir --profile "$profile")"
    shift || true
    codex_profile_exec "$profile_dir" "$profile" "$@"
}
