# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_notify_load_config() {
    local config_file="$CODEX_TERMUX_NOTIFY_CONFIG"
    [ -r "$config_file" ] || return 0
    # shellcheck disable=SC1090
    . "$config_file"
    CODEX_TERMUX_NOTIFY_HOOKS="$(codex_notify_hooks_normalize "${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}")"
}

codex_notify_domain_tool() {
    local source_dir source_root candidate
    candidate="${CODEX_TERMUX_NOTIFY_DOMAIN_TOOL:-}"
    if [ -n "$candidate" ] && [ -r "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
    fi
    if [ -r "$CODEX_TERMUX_TURN_NOTIFY" ]; then
        printf '%s\n' "$CODEX_TERMUX_TURN_NOTIFY"
        return 0
    fi
    source_root="$CODEX_TERMUX_WRAPPER_ROOT"
    candidate="$source_root/tools/codex-turn-notify.sh"
    if [ -n "$source_root" ] && [ -r "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
    fi
    codex_fail "Notification domain tool is unavailable: $CODEX_TERMUX_TURN_NOTIFY"
    return 1
}

codex_notify_domain_call() {
    local tool
    tool="$(codex_notify_domain_tool)" || return $?
    "${BASH:-bash}" "$tool" "$@"
}

codex_notify_all_hooks() {
    codex_termux_cmd notify-hook --action all
}

codex_notify_hook_canonical() {
    codex_termux_cmd notify-hook --action canonical --value "${1:-}"
}

codex_notify_hook_valid() {
    codex_termux_cmd notify-hook --action valid --value "${1:-}"
}

codex_notify_hooks_normalize() {
    codex_termux_cmd notify-hook --action normalize --value "${1:-Stop}"
}

codex_notify_event_label() {
    codex_notify_domain_call --event-label "${1:-}"
}

codex_notify_hook_list() {
    local hooks event seen="," event_list=()
    hooks="$(codex_notify_hooks_normalize "${1:-${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}}")"
    case ",$hooks," in
        *,all,*)
            codex_notify_all_hooks
            return 0
            ;;
    esac
    [ -n "$hooks" ] || hooks="Stop"
    IFS=, read -r -a event_list <<<"$hooks"
    for event in "${event_list[@]}"; do
        event="$(codex_notify_hook_canonical "$event")"
        [ -n "$event" ] || continue
        case "$seen" in
            *,"$event",*) continue ;;
        esac
        seen="$seen$event,"
        printf '%s\n' "$event"
    done
}

codex_notify_hook_status_message() {
    codex_termux_cmd notify-hook --action status-message --value "${1:-}"
}

codex_notify_write_system_config() {
    local config_file="$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"
    mkdir -p "$CODEX_TERMUX_TMPDIR" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR" || return $?
    codex_notify_load_config
    codex_termux_cmd notify-system-config \
        --hooks "${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}" \
        --turn-notify "$CODEX_TERMUX_TURN_NOTIFY" >"$config_file"
    [ -e "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/requirements.toml" ] ||
        : >"$CODEX_TERMUX_SYSTEM_CONFIG_DIR/requirements.toml"
    [ -e "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/managed_config.toml" ] ||
        : >"$CODEX_TERMUX_SYSTEM_CONFIG_DIR/managed_config.toml"
}

codex_prepare_system_config() {
    codex_notify_write_system_config
}

codex_notify_usage() {
    cat <<'USAGE'
Usage: codex termux notify [options]

Without options, opens an interactive notification setup prompt.

Options:
  --channel NAME          notification, toast, both
  --hooks LIST            Comma-separated hook list or "all"
  --hook NAME             Append a single hook name
  --all-hooks             Enable every supported hook position
  --pretooluse 0|1        Store legacy PreToolUse flag; use --hooks PreToolUse to enable the hook
  --content-chars N       Limit notification body to N characters
  --preserve-newlines 0|1 Keep notification body line breaks
  --toast-gravity VALUE   top, middle, or bottom
  --toast-short 0|1      Use short toast duration
  --toast-background HEX  Toast background color
  --toast-color HEX       Toast text color
  --group NAME            Notification group key
USAGE
}

codex_notify_interactive_usage() {
    cat <<'USAGE'
Usage: codex termux notify

Without options, opens an interactive notification setup prompt.
USAGE
}

codex_notify_need_arg() {
    [ $# -ge 2 ] || {
        codex_fail "Missing value for $1"
        return 64
    }
}

codex_notify_render_config_env() {
    codex_termux_cmd notify-config-env \
        --content-chars "$1" \
        --preserve-newlines "$2" \
        --toast-gravity "$3" \
        --toast-short "$4" \
        --toast-background "$5" \
        --toast-color "$6" \
        --group "$7" \
        --channel "$8" \
        --hooks "$9" \
        --pretooluse "${10}"
}

codex_notify_hook_ids() {
    codex_notify_all_hooks
}

codex_notify_render_hooks() {
    local idx=1 hook
    codex_ui_menu_header "Choose notify hooks" "Space-separated numbers or names, then Enter"
    while IFS= read -r hook; do
        printf '  %s %s\n' "$(codex_ui_number "$idx")" "$hook" >&2
        idx=$((idx + 1))
    done <<EOF
$(codex_notify_hook_ids)
EOF
    printf '  %s all\n' "$(codex_ui_number "0")" >&2
    printf '\n' >&2
}

codex_notify_parse_hook_selection() {
    local selection="${1:-}" token hook idx=0 hooks=() all_hooks=() found=0
    mapfile -t all_hooks < <(codex_notify_hook_ids)
    case "$selection" in
        "")
            printf 'Stop\n'
            return 0
            ;;
        all|ALL)
            printf 'all\n'
            return 0
            ;;
    esac
    for token in $selection; do
        case "$token" in
            0|all|ALL)
                printf 'all\n'
                return 0
                ;;
            *[!0-9]*)
                hook="$(codex_notify_hook_canonical "$token")"
                if ! codex_notify_hook_valid "$hook"; then
                    codex_fail "Unknown notification hook: $token"
                    return 64
                fi
                case "$hook" in
                    all)
                        printf 'all\n'
                        return 0
                        ;;
                esac
                case ",${hooks[*]:-}," in
                    *,"$hook",*) ;;
                    *) hooks+=("$hook") ;;
                esac
                found=1
                ;;
            [0-9]*)
                if [ "$token" -ge 1 ] && [ "$token" -le "${#all_hooks[@]}" ]; then
                    hook="${all_hooks[$((token - 1))]}"
                    case ",${hooks[*]:-}," in
                        *,"$hook",*) ;;
                        *) hooks+=("$hook") ;;
                    esac
                    found=1
                else
                    codex_fail "Notification hook number out of range: $token"
                    return 64
                fi
                ;;
            *)
                hook="$(codex_notify_hook_canonical "$token")"
                if ! codex_notify_hook_valid "$hook"; then
                    codex_fail "Unknown notification hook: $token"
                    return 64
                fi
                case "$hook" in
                    all)
                        printf 'all\n'
                        return 0
                        ;;
                esac
                case ",${hooks[*]:-}," in
                    *,"$hook",*) ;;
                    *) hooks+=("$hook") ;;
                esac
                found=1
                ;;
        esac
    done
    if [ "$found" -eq 0 ]; then
        printf 'Stop\n'
        return 0
    fi
    (IFS=,; printf '%s\n' "${hooks[*]}")
}

codex_notify_public() {
    local config_file="$CODEX_TERMUX_NOTIFY_CONFIG"
    local channel="${CODEX_TERMUX_NOTIFY_CHANNEL:-notification}"
    local hooks="${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}"
    local pretooluse="${CODEX_TERMUX_NOTIFY_PRETOOLUSE:-0}"
    local content_chars="${CODEX_TERMUX_NOTIFY_CONTENT_CHARS:-140}"
    local preserve_newlines="${CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES:-0}"
    local toast_gravity="${CODEX_TERMUX_NOTIFY_TOAST_GRAVITY:-top}"
    local toast_short="${CODEX_TERMUX_NOTIFY_TOAST_SHORT:-0}"
    local toast_background="${CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND:-}"
    local toast_color="${CODEX_TERMUX_NOTIFY_TOAST_COLOR:-}"
    local group="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"
    if [ $# -eq 0 ]; then
        if [ -t 0 ] && [ -t 2 ]; then
            codex_notify_interactive_public
            return $?
        fi
        codex_fail "codex termux notify requires options or an interactive terminal"
        return 2
    fi
    while [ $# -gt 0 ]; do
        case "$1" in
            --help|-h)
                codex_notify_usage
                return 0
                ;;
            --config-file)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                config_file="${2:-}"
                shift 2
                ;;
            --channel)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                channel="${2:-}"
                shift 2
                ;;
            --hooks)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                hooks="${2:-}"
                shift 2
                ;;
            --hook)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                hooks="${hooks:+$hooks,}${2:-}"
                shift 2
                ;;
            --all-hooks)
                hooks="all"
                shift
                ;;
            --pretooluse)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                pretooluse="${2:-}"
                shift 2
                ;;
            --content-chars)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                content_chars="${2:-}"
                shift 2
                ;;
            --preserve-newlines)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                preserve_newlines="${2:-}"
                shift 2
                ;;
            --toast-gravity)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_gravity="${2:-}"
                shift 2
                ;;
            --toast-short)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_short="${2:-}"
                shift 2
                ;;
            --toast-background)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_background="${2:-}"
                shift 2
                ;;
            --toast-color)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_color="${2:-}"
                shift 2
                ;;
            --group)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                group="${2:-}"
                shift 2
                ;;
            *)
                codex_fail "Unknown notify option: $1"
                return 64
                ;;
        esac
    done
    [ -n "$config_file" ] || {
        codex_fail "Notification config file is unavailable"
        return 66
    }
    mkdir -p "${config_file%/*}"
    codex_notify_render_config_env \
        "$content_chars" \
        "$preserve_newlines" \
        "$toast_gravity" \
        "$toast_short" \
        "$toast_background" \
        "$toast_color" \
        "$group" \
        "$channel" \
        "$hooks" \
        "$pretooluse" >"$config_file" || return $?
    codex_prepare_system_config || return $?
    codex_say "Saved notification settings to $config_file"
}

codex_notify_interactive_public() {
    local channel_choice hooks_choice gravity_choice channel hooks gravity
    codex_ui_menu_header "Configure notifications" "Choose channel, hooks, and toast position"
    printf '  %s notification\n' "$(codex_ui_number 1)" >&2
    printf '  %s toast\n' "$(codex_ui_number 2)" >&2
    printf '  %s both\n' "$(codex_ui_number 3)" >&2
    printf '\nChannel [3]> ' >&2
    IFS= read -r channel_choice || {
        codex_selection_cancelled
        return 130
    }
    case "${channel_choice:-3}" in
        1|notification) channel="notification" ;;
        2|toast) channel="toast" ;;
        3|both) channel="both" ;;
        *)
            codex_fail "Unknown notification channel selection: $channel_choice"
            return 64
            ;;
    esac

    codex_notify_render_hooks
    printf 'Hooks [Stop]> ' >&2
    IFS= read -r hooks_choice || {
        codex_selection_cancelled
        return 130
    }
    hooks="$(codex_notify_parse_hook_selection "$hooks_choice")" || return $?

    gravity="top"
    case "$channel" in
        toast|both)
            printf 'Toast gravity [top]> ' >&2
            IFS= read -r gravity_choice || {
                codex_selection_cancelled
                return 130
            }
            gravity="${gravity_choice:-top}"
            ;;
    esac

    codex_notify_public --channel "$channel" --hooks "$hooks" --toast-gravity "$gravity"
}
