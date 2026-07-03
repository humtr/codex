# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

CODEX_PROMPT_CHOICE_RESULT=""

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
            action="$(codex_termux_cmd prompt-choice-action \
                --reply "$reply" \
                --mode "$mode" \
                --max-items "$max_items" \
                --phase tty)" || {
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
    action="$(codex_termux_cmd prompt-choice-action \
        --reply "$reply" \
        --mode "$mode" \
        --max-items "$max_items" \
        --phase final)" || return $?
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
