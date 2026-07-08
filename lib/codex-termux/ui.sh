# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_status_clear() {
    if [ "${CODEX_STATUS_ACTIVE:-0}" -eq 1 ] && [ -t 2 ]; then
        printf '\r\033[2K' >&2
        CODEX_STATUS_ACTIVE=0
    fi
}

codex_status() {
    local message
    message="$(codex_termux_cmd ui-status-text --message "$*")" || return 1
    if [ -t 2 ]; then
        printf '\r\033[2K%s' "$message" >&2
        CODEX_STATUS_ACTIVE=1
    else
        printf '%s\n' "$message" >&2
    fi
}

codex_say() {
    codex_status_clear
    printf '%s\n' "$*" >&2
}

codex_selection_cancelled() {
    codex_say "$(codex_ui_text_get selection_cancelled)"
}

codex_fail() {
    codex_status_clear
    printf 'Error: %s\n' "$*" >&2
    return 1
}

codex_ui_format() {
    local color=0 kind="$1" value="${2:-}"
    [ -t 2 ] && [ -z "${NO_COLOR:-}" ] && color=1
    codex_termux_cmd ui-format --kind "$kind" --value "$value" --color "$color" | tr -d '\n'
}

codex_ui_number() {
    codex_ui_format number "$1"
}

codex_ui_badge() {
    codex_ui_format badge "$1"
}

codex_ui_menu_header() {
    local title="$1" subtitle="${2:-}"
    codex_status_clear
    printf '%s\n' "$title" >&2
    if [ -n "$subtitle" ]; then
        printf '%s\n' "$(codex_ui_format dim "$subtitle")" >&2
    fi
}

codex_ui_menu_row() {
    local key="$1" label="$2"
    shift 2 || true
    printf '  %s %s' "$(codex_ui_number "$key")" "$label" >&2
    while [ "$#" -gt 0 ]; do
        [ -n "$1" ] && printf ' %s' "$1" >&2
        shift
    done
    printf '\n' >&2
}

codex_ui_prompt() {
    codex_ui_format prompt "$1"
}

codex_ui_text() {
    local key="$1"
    shift || true
    codex_termux_cmd ui-text --key "$key" "$@"
}

codex_ui_step_text() {
    local key="$1"
    shift || true
    codex_termux_cmd ui-step-text --key "$key" "$@"
}

codex_ui_step_mode() {
    local key="$1"
    codex_termux_cmd ui-step-mode --key "$key" | tr -d '\n'
}

codex_ui_text_get() {
    codex_ui_text "$@" | tr -d '\n'
}

codex_ui_step() {
    local key="$1" mode message
    shift || true
    mode="$(codex_ui_step_mode "$key")" || return 1
    message="$(codex_ui_step_text "$key" "$@")" || return 1
    if [ "$mode" = "committed" ]; then
        codex_say "${message%$'\n'}"
        return 0
    fi
    codex_status "${message%$'\n'}"
}

codex_display_version() {
    codex_ui_format display-version "${1:-unknown}"
}
