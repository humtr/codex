# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_session_share_source() {
    local source_path="$1" source_profile="${2:-default}" target_profile="${3:-default}"
    [ -n "$source_path" ] || return 0
    [ "$source_profile" = "$target_profile" ] && return 0
    [ -f "$source_path" ] || return 0
    codex_termux_cmd session-share \
        --source-path "$source_path" \
        --source-profile "$source_profile" \
        --target-profile "$target_profile"
}

codex_session() {
    local target_profile="" target_profile_dir=""
    if [ "$#" -gt 0 ] && [[ ! "$1" =~ ^- ]]; then
        target_profile="$1"
        shift
        codex_profile_name_valid "$target_profile" || {
            codex_fail "$(codex_ui_text_get invalid_profile "$target_profile")"
            return 2
        }
        target_profile_dir="$(codex_profile_home_dir "$target_profile")"
    fi

    local show_all="false"
    local arg
    for arg in "$@"; do
        if [ "$arg" = "--all" ]; then
            show_all="true"
            break
        fi
    done

    local tui_args=()
    if [ "$show_all" = "true" ]; then
        tui_args+=("--all")
    fi

    # Run Python session-tui with a temporary file for output to preserve TTY
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

    local selected_plan
    selected_plan="$(cat "$temp_file")"
    rm -f "$temp_file"

    local native_session_ref source_profile workdir codex_home_env source_path
    IFS=$'\037' read -r target_profile target_profile_dir native_session_ref source_profile workdir codex_home_env source_path <<EOF
$selected_plan
EOF

    codex_session_share_source "$source_path" "$source_profile" "$target_profile"

    # Switch directory if workdir is specified and is a valid directory
    if [ -n "$workdir" ] && [ -d "$workdir" ]; then
        cd "$workdir" || true
    fi

    # Resume the session via wrapper's runtime execution path, forwarding any extra options
    codex_profile_exec "$target_profile_dir" "$target_profile" resume "$native_session_ref" "$@"
}
