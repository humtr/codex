codex_profile_validate_name() {
    local profile="${1:-}"
    case "$profile" in
        ""|default)
            return 0
            ;;
        native|-*|.*|*/*|*..*|*[[:space:]]*)
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

codex_profile_dir() {
    local profile="${1:-default}"
    if [ -z "$profile" ] || [ "$profile" = "default" ]; then
        printf '%s\n' "$CODEX_NATIVE_HOME/.codex"
    else
        printf '%s/%s\n' "$CODEX_NATIVE_PROFILE_ROOT" "$profile"
    fi
}

codex_profile_display_name() {
    local profile="${1:-default}"
    if [ -z "$profile" ] || [ "$profile" = "default" ]; then
        printf 'default\n'
    else
        printf '%s\n' "$profile"
    fi
}

codex_profile_choice_to_name() {
    local choice="${1:-}"
    case "$choice" in
        ""|home|default)
            printf 'default\n'
            ;;
        *)
            printf '%s\n' "$choice"
            ;;
    esac
}

codex_profile_write_recent() {
    local profile="${1:-default}"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    printf '%s\n' "$profile" >"$CODEX_NATIVE_LAST_PROFILE_FILE"
}

codex_profile_read_recent() {
    local profile
    profile="$(cat "$CODEX_NATIVE_LAST_PROFILE_FILE" 2>/dev/null || true)"
    profile="$(codex_profile_choice_to_name "$profile")"
    codex_profile_validate_name "$profile" || {
        printf 'default\n'
        return 0
    }
    if [ "$profile" != "default" ] && [ ! -d "$(codex_profile_dir "$profile")" ]; then
        printf 'default\n'
        return 0
    fi
    printf '%s\n' "$profile"
}

codex_profile_note() {
    local profile="${1:-default}"
    codex_say "entering profile $(codex_profile_display_name "$profile")"
}

codex_profile_runtime_exec() {
    local profile="$1" profile_dir="$2"
    shift 2 || true
    codex_profile_share_plugins "$profile_dir"
    codex_profile_write_recent "$profile"
    codex_profile_note "$profile"
    codex_prepare_runtime_env
    CODEX_HOME="$profile_dir" exec "$CODEX_NATIVE_RUNTIME" "$@"
}

codex_profile_menu_ids() {
    local recent profile
    recent="$(codex_profile_read_recent)"
    printf 'default\n'
    if [ "$recent" != "default" ]; then
        printf '%s\n' "$recent"
    fi
    while IFS= read -r profile; do
        [ "$profile" = "default" ] && continue
        [ "$profile" = "$recent" ] && continue
        printf '%s\n' "$profile"
    done < <(codex_list_profiles)
}

codex_profile_share_plugins() {
    local profile_dir="$1" shared_plugins_dir="$CODEX_NATIVE_SHARED_PLUGINS_DIR" plugins_dir
    plugins_dir="$profile_dir/plugins"
    mkdir -p "$profile_dir" "$shared_plugins_dir"
    if [ -e "$plugins_dir" ] || [ -L "$plugins_dir" ]; then
        return 0
    fi
    ln -s "$shared_plugins_dir" "$plugins_dir"
}

codex_list_profiles() {
    local root="$CODEX_NATIVE_PROFILE_ROOT"
    [ -d "$root" ] || return 0
    find "$root" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null \
        | grep -Ev '^(default|native)$' \
        | grep -Ev '^[.]' \
        | LC_ALL=C sort -f
}

CODEX_PROMPT_CHOICE_RESULT=""

codex_prompt_choice() {
    local prompt="${1:-choose> }" mode="${2:-freeform}" max_items="${3:-9}" reply rest old_tty status
    CODEX_PROMPT_CHOICE_RESULT=""
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
            case "$reply" in
                $'\e')
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 130
                    ;;
                $'\n'|$'\r'|'')
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 0
                    ;;
                [0-9])
                    if [ "$mode" = "digits" ]; then
                        if [ "$reply" = "0" ] || [ "$reply" -le "$max_items" ]; then
                            [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                            printf '%s\n' "$reply" >&2
                            CODEX_PROMPT_CHOICE_RESULT="$reply"
                            return 0
                        fi
                        continue
                    fi
                    if [ "$max_items" -le 9 ]; then
                        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                        printf '%s\n' "$reply" >&2
                        CODEX_PROMPT_CHOICE_RESULT="$reply"
                        return 0
                    fi
                    ;;
                [yYnN])
                    if [ "$mode" = "yn" ]; then
                        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                        printf '%s\n' "$reply" >&2
                        CODEX_PROMPT_CHOICE_RESULT="$reply"
                        return 0
                    fi
                    ;;
                *)
                    case "$mode" in
                        digits|yn) continue ;;
                    esac
                    break
                    ;;
            esac
            break
        done
        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
    elif ! IFS= read -r -N 1 reply; then
        printf '\n' >&2
        return 1
    fi
    case "$reply" in
        $'\e')
            printf '\n' >&2
            return 130
            ;;
        $'\n'|$'\r'|'')
            printf '\n' >&2
            return 0
            ;;
        [0-9])
            if [ "$mode" = "digits" ]; then
                if [ "$reply" = "0" ] || [ "$reply" -le "$max_items" ]; then
                    printf '%s\n' "$reply" >&2
                    CODEX_PROMPT_CHOICE_RESULT="$reply"
                    return 0
                fi
                return 1
            fi
            if [ "$max_items" -le 9 ]; then
                printf '%s\n' "$reply" >&2
                CODEX_PROMPT_CHOICE_RESULT="$reply"
                return 0
            fi
            ;;
        *)
        [ "$mode" = "digits" ] && return 1
            ;;
    esac
    if [ "$mode" = "yn" ]; then
        case "$reply" in
            [yYnN])
                printf '%s\n' "$reply" >&2
                CODEX_PROMPT_CHOICE_RESULT="$reply"
                return 0
                ;;
            '')
                return 0
                ;;
            *)
                return 1
                ;;
        esac
    fi
    rest=""
    printf '%s' "$reply" >&2
    IFS= read -r rest || true
    printf '%s%s\n' "$reply" "$rest" >&2
    CODEX_PROMPT_CHOICE_RESULT="$reply$rest"
    return 0
}

codex_profile_exec() {
    local profile_dir="$1" profile="${2:-default}"
    shift 2 || true
    if [ ! -d "$profile_dir" ]; then
        codex_fail "profile directory not found: $profile_dir"
        return 2
    fi
    codex_ensure_runtime_ready || return $?
    codex_auto_update_if_needed || return $?
    codex_profile_runtime_exec "$profile" "$profile_dir" "$@"
}

codex_profile_select() {
    local profiles=() profile choice idx profile_dir display_limit=0 truncated=0 recent
    recent="$(codex_profile_read_recent)"
    mapfile -t profiles < <(codex_profile_menu_ids)
    if [ -t 0 ]; then
        display_limit=9
    fi

    printf '%s\n' "Choose profile" >&2
    printf '%s\n' "$(codex_ui_dim 'Select CODEX_HOME target')" >&2
    idx=0
    for profile in "${profiles[@]}"; do
        if [ "$display_limit" -gt 0 ] && [ "$idx" -gt "$display_limit" ]; then
            truncated=1
            break
        fi
        if [ "$profile" = "$recent" ]; then
            printf '  %s %s %s\n' "$(codex_ui_number "$idx")" "$(codex_profile_display_name "$profile")" "$(codex_ui_badge recent)" >&2
        else
            printf '  %s %s\n' "$(codex_ui_number "$idx")" "$(codex_profile_display_name "$profile")" >&2
        fi
        idx=$((idx + 1))
    done
    if [ "$truncated" -eq 1 ]; then
        printf '%s\n' "$(codex_ui_dim '  (More options: codex profile NAME)')" >&2
    fi
    printf '\n' >&2

    if [ ! -t 0 ]; then
        return 0
    fi

    codex_prompt_choice "$(codex_ui_prompt 'choose profile > ')" freeform "$(( ${#profiles[@]} < 9 ? ${#profiles[@]} : 9 ))" || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    [ -n "$choice" ] || return 130

    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 0 ] && [ "$choice" -lt "${#profiles[@]}" ]; then
        profile="${profiles[$choice]}"
    else
        profile="$(codex_profile_choice_to_name "$choice")"
    fi

    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
        return 2
    }
    profile_dir="$(codex_profile_dir "$profile")"
    codex_profile_exec "$profile_dir" "$profile"
}

codex_profile_run() {
    local profile="${1:-}"
    if [ -z "$profile" ]; then
        codex_profile_select
        return $?
    fi
    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
        return 2
    }
    local profile_dir
    profile_dir="$(codex_profile_dir "$profile")"
    shift || true
    codex_profile_exec "$profile_dir" "$profile" "$@"
}

codex_restore_backup() {
    local public="$1" base latest
    base="$(basename "$public")"
    latest="$(ls -t "$CODEX_NATIVE_BACKUP_DIR"/"$base".*.bak 2>/dev/null | sed -n '1p' || true)"
    if [ -n "$latest" ]; then
        cp -Pp "$latest" "$public"
        codex_say "restored $public from $latest"
    fi
}

codex_remove() {
    if codex_file_has_marker "$CODEX_NATIVE_PUBLIC_CODEX"; then
        rm -f "$CODEX_NATIVE_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_NATIVE_PUBLIC_CODEX"
    fi
    rm -rf "$CODEX_NATIVE_NATIVE_ROOT"
    codex_say "removed managed runtime; state kept at $CODEX_NATIVE_STATE_DIR for backups"
}

codex_use() {
    local choice="${1:-}" runtime_path version raw_sha runtime_sha package_spec
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
    codex_prompt_choice "$(codex_ui_prompt 'choose runtime > ')" digits "${CODEX_USE_MENU_COUNT:-0}" || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    if [ -z "$choice" ]; then
        printf 'codex use: cancelled.\n' >&2
        return 1
    fi
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
    codex_native_cmd use-render \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
        --interactive-limit "$interactive_limit" \
        --mode "$mode"
}

codex_use_select() {
    local choice="$1" selected
    local latest="${CODEX_USE_LAST_LATEST:-}"
    if [ -z "$latest" ]; then
        latest="$(codex_latest_linux_arm64_version || true)"
    fi
    selected="$(codex_native_cmd use-select \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --choice "$choice" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY")" || {
        codex_fail "unknown cached runtime selection: $choice"
        return 1
    }
    local kind runtime_path raw_path version raw_sha runtime_sha package_spec
    IFS=$'\037' read -r kind runtime_path raw_path version raw_sha runtime_sha package_spec <<EOF
$selected
EOF
    if [ "$kind" = "remote" ]; then
        codex_update "$version" || return $?
    else
        codex_with_lock codex_activate_cached_runtime_unlocked \
            "$runtime_path" "$raw_path" "$version" "$raw_sha" "$runtime_sha" "$package_spec" || return $?
    fi
    codex_version || return $?
}

codex_main() {
    local recent_profile recent_profile_dir
    case "${1:-}" in
        setup)
            shift
            codex_setup_public "$@"
            ;;
        update)
            shift
            codex_update_public "${1:-}"
            ;;
        doctor)
            shift
            codex_public_doctor "$@"
            ;;
        version)
            shift
            codex_version
            ;;
        help|--help|-h)
            shift || true
            codex_help "$@"
            ;;
        use)
            shift
            codex_use "$@"
            ;;
        profile)
            shift
            codex_profile_run "$@"
            ;;
        remove)
            shift
            codex_remove
            ;;
        *)
            codex_ensure_runtime_ready || return $?
            codex_auto_update_if_needed || return $?
            recent_profile="$(codex_profile_read_recent)"
            recent_profile_dir="$(codex_profile_dir "$recent_profile")"
            codex_profile_runtime_exec "$recent_profile" "$recent_profile_dir" "$@"
            ;;
    esac
}
