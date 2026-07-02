# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_profile_name_valid() {
    codex_termux_cmd profile-validate --profile "${1:-}"
}

codex_profile_home_dir() {
    codex_termux_cmd profile-dir --profile "${1:-default}"
}

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

CODEX_PROMPT_CHOICE_RESULT=""

codex_prompt_choice_action() {
    codex_termux_cmd prompt-choice-action \
        --reply "${1:-}" \
        --mode "$2" \
        --max-items "$3" \
        --phase "$4"
}

codex_prompt_choice() {
    local prompt="${1:-choose> }" mode="${2:-freeform}" max_items="${3:-9}" reply rest old_tty status action
    CODEX_PROMPT_CHOICE_RESULT=""
    codex_status_clear
    printf '%s' "$prompt" >&2
    if [ -t 0 ]; then
        old_tty="$(stty -g 2>/dev/null || true)"
        [ -z "$old_tty" ] || stty -echo -icanon min 1 time 0 2>/dev/null || true
        while :; do
            IFS= read -r -N 1 reply
            status=$?
            if [ "$status" -ne 0 ]; then
                [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                printf '\n' >&2
                return 1
            fi
            action="$(codex_prompt_choice_action "$reply" "$mode" "$max_items" tty)" || {
                [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                printf '\n' >&2
                return 1
            }
            case "$action" in
                cancel)
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 130
                    ;;
                empty)
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 0
                    ;;
                accept)
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '%s\n' "$reply" >&2
                    CODEX_PROMPT_CHOICE_RESULT="$reply"
                    return 0
                    ;;
                continue)
                    continue
                    ;;
                read-rest)
                    break
                    ;;
                *)
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 1
                    ;;
            esac
        done
        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
    elif ! IFS= read -r -N 1 reply; then
        printf '\n' >&2
        return 1
    fi
    action="$(codex_prompt_choice_action "$reply" "$mode" "$max_items" final)" || return $?
    case "$action" in
        cancel)
            printf '\n' >&2
            return 130
            ;;
        empty)
            printf '\n' >&2
            return 0
            ;;
        accept)
            printf '%s\n' "$reply" >&2
            CODEX_PROMPT_CHOICE_RESULT="$reply"
            return 0
            ;;
        fail)
            return 1
            ;;
    esac
    rest=""
    printf '%s' "$reply" >&2
    IFS= read -r rest || true
    printf '%s%s\n' "$reply" "$rest" >&2
    CODEX_PROMPT_CHOICE_RESULT="$reply$rest"
    return 0
}

codex_prompt_interactive() {
    local prompt="$1" mode="${2:-freeform}" max_items="${3:-9}" empty_policy="${4:-keep}"
    local cancel_message="${5:-$(codex_ui_text_get selection_cancelled)}" status
    codex_prompt_choice "$(codex_ui_prompt "$prompt")" "$mode" "$max_items"
    status=$?
    case "$status" in
        130)
            [ -n "$cancel_message" ] && codex_say "$cancel_message"
            return 130
            ;;
        0)
            ;;
        *)
            return 1
            ;;
    esac
    if [ -z "${CODEX_PROMPT_CHOICE_RESULT:-}" ] && [ "$empty_policy" = "cancel" ]; then
        [ -n "$cancel_message" ] && codex_say "$cancel_message"
        return 130
    fi
    return 0
}

codex_confirm_menu() {
    local title="$1" subtitle="$2" yes_key="$3" yes_label="$4" yes_badges="$5"
    local no_key="$6" no_label="$7" no_badges="$8" prompt="$9" empty_policy="${10:-cancel}"
    codex_ui_menu_header "$title" "$subtitle"
    codex_ui_menu_row "$yes_key" "$yes_label" "$yes_badges"
    codex_ui_menu_row "$no_key" "$no_label" "$no_badges"
    printf '\n' >&2
    codex_prompt_interactive "$prompt" yn 0 "$empty_policy"
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
    codex_termux_cmd profile-create-confirmed --choice "${CODEX_PROMPT_CHOICE_RESULT:-}" || return 130
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
    recent_profile_dir="$(codex_profile_home_dir "$recent_profile")"
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
    choice="$CODEX_PROMPT_CHOICE_RESULT"

    profile="$(codex_termux_cmd profile-menu-choice --choice "$choice")"

    codex_profile_name_valid "$profile" || {
        codex_fail "$(codex_ui_text_get invalid_profile "$profile")"
        return 2
    }
    profile_dir="$(codex_profile_home_dir "$profile")"
    codex_profile_exec "$profile_dir" "$profile"
}

codex_profile_run() {
    local profile="${1:-}"
    if [ -z "$profile" ]; then
        codex_profile_select
        return $?
    fi
    case "$profile" in
        list|ls)
            shift || true
            [ "$#" -eq 0 ] || {
                codex_fail "$(codex_ui_text_get profile_arg_error "$profile")"
                return 2
            }
            codex_status_clear
            codex_termux_cmd profile-list --include-default
            return 0
            ;;
    esac
    codex_profile_name_valid "$profile" || {
        codex_fail "$(codex_ui_text_get invalid_profile "$profile")"
        return 2
    }
    local profile_dir
    profile_dir="$(codex_profile_home_dir "$profile")"
    shift || true
    codex_profile_exec "$profile_dir" "$profile" "$@"
}

codex_restore_backup() {
    local public="$1" base latest
    base="$(basename "$public")"
    latest="$(ls -t "$CODEX_TERMUX_BACKUP_DIR"/"$base".*.bak 2>/dev/null | sed -n '1p' || true)"
    if [ -n "$latest" ]; then
        cp -Pp "$latest" "$public"
        codex_say "$(codex_ui_text_get restored_backup "$public" "$latest")"
    fi
}

codex_remove() {
    if codex_termux_cmd file-has-marker \
        --path "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"; then
        rm -f "$CODEX_TERMUX_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
    codex_rm_rf_managed "$CODEX_TERMUX_ROOT" || return $?
    codex_say "$(codex_ui_text_get removed_runtime "$CODEX_TERMUX_STATE_DIR")"
}

codex_use() {
    local choice="${1:-}"
    if [ "$choice" = "--list" ]; then
        codex_use_list list
        return $?
    fi
    if [ -n "$choice" ]; then
        codex_use_select "$choice"
        return $?
    fi
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
