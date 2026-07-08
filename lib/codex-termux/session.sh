# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_session_validate_boundary() {
    local source_profile="${1:-default}" target_profile="${2:-default}"
    [ "$source_profile" = "$target_profile" ] && return 0
    codex_termux_cmd session-boundary-check \
        --source-profile "$source_profile" \
        --target-profile "$target_profile"
}

codex_session_share_source() {
    local source_path="$1" source_profile="${2:-default}" target_profile="${3:-default}"
    [ -n "$source_path" ] || return 0
    [ "$source_profile" = "$target_profile" ] && return 0
    [ -f "$source_path" ] || return 0
    codex_session_validate_boundary "$source_profile" "$target_profile" || return $?
    codex_termux_cmd session-share \
        --source-path "$source_path" \
        --source-profile "$source_profile" \
        --target-profile "$target_profile"
}

codex_session() {
    local target_profile=""
    if [ "$#" -gt 0 ] && [[ ! "$1" =~ ^- ]]; then
        target_profile="$1"
        shift
        codex_termux_cmd profile-validate --profile "$target_profile" || {
            codex_fail "$(codex_ui_text_get invalid_profile "$target_profile")"
            return 2
        }
    fi

    local show_all="false" arg
    for arg in "$@"; do
        if [ "$arg" = "--all" ]; then
            show_all="true"
            break
        fi
    done

    local tui_args=()
    [ "$show_all" != "true" ] || tui_args+=("--all")

    local temp_file
    temp_file="$(codex_mktemp_file codex-session)" || return $?

    CODEX_SESSION_TUI_DEFAULT_PROFILE="$target_profile" codex_termux_cmd session-tui --output "$temp_file" "${tui_args[@]}" || {
        local code=$?
        rm -f "$temp_file"
        if [ "$code" -eq 130 ]; then
            return 130
        fi
        return "$code"
    }

    if [ ! -s "$temp_file" ]; then
        rm -f "$temp_file"
        return 0
    fi

    local selected_plan selected_env
    selected_plan="$(cat "$temp_file")"
    rm -f "$temp_file"

    selected_env="$(codex_termux_cmd session-plan-env --plan "$selected_plan")" || return $?
    eval "$selected_env"

    codex_session_validate_boundary "${CODEX_SESSION_SOURCE_PROFILE:-default}" \
        "${CODEX_SESSION_TARGET_PROFILE:-default}" || return $?

    codex_session_share_source "${CODEX_SESSION_SOURCE_PATH:-}" \
        "${CODEX_SESSION_SOURCE_PROFILE:-default}" "${CODEX_SESSION_TARGET_PROFILE:-default}" || return $?

    if [ -n "${CODEX_SESSION_WORKDIR:-}" ] && [ -d "$CODEX_SESSION_WORKDIR" ]; then
        cd "$CODEX_SESSION_WORKDIR" || true
    fi

    codex_profile_exec "$CODEX_SESSION_TARGET_PROFILE_DIR" "$CODEX_SESSION_TARGET_PROFILE" \
        resume "$CODEX_SESSION_NATIVE_REF" "$@"
}
