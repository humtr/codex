# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_notify_load_config() {
    local config_file="$CODEX_TERMUX_NOTIFY_CONFIG"
    [ -r "$config_file" ] || return 0
    # shellcheck disable=SC1090
    . "$config_file"
    CODEX_TERMUX_NOTIFY_HOOKS="$(
        codex_termux_cmd notify-hook --action normalize --value "${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}"
    )"
}

codex_notify_all_hooks() {
    codex_termux_cmd notify-hook --action all
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

codex_notify_render_hooks() {
    local idx=1 hook
    codex_ui_menu_header "Choose notify hooks" "Space-separated numbers or names, then Enter"
    while IFS= read -r hook; do
        printf '  %s %s\n' "$(codex_ui_number "$idx")" "$hook" >&2
        idx=$((idx + 1))
    done <<EOF
$(codex_notify_all_hooks)
EOF
    printf '  %s all\n' "$(codex_ui_number "0")" >&2
    printf '\n' >&2
}

codex_notify_command_config() {
    (
        export CODEX_TERMUX_NOTIFY_CONFIG
        export CODEX_TERMUX_NOTIFY_GROUP
        export CODEX_TERMUX_NOTIFY_CONTENT_CHARS
        export CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES
        export CODEX_TERMUX_NOTIFY_TOAST_GRAVITY
        export CODEX_TERMUX_NOTIFY_TOAST_SHORT
        export CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND
        export CODEX_TERMUX_NOTIFY_TOAST_COLOR
        export CODEX_TERMUX_NOTIFY_PRETOOLUSE
        export CODEX_TERMUX_NOTIFY_HOOKS
        CODEX_TERMUX_NOTIFY_CHANNEL="${CODEX_TERMUX_NOTIFY_CHANNEL:-notification}" \
            codex_termux_cmd notify-command-config "$@"
    )
}

codex_notify_public() {
    local config_file
    if [ $# -eq 0 ]; then
        if [ -t 0 ] && [ -t 2 ]; then
            codex_notify_interactive_public
            return $?
        fi
        codex_fail "codex termux notify requires options or an interactive terminal"
        return 2
    fi
    case "${1:-}" in
        --help|-h)
            codex_notify_usage
            return 0
            ;;
    esac
    config_file="$(codex_notify_command_config --field config-file -- "$@")" || return $?
    mkdir -p "${config_file%/*}"
    codex_notify_command_config --field config-env -- "$@" >"$config_file" || return $?
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
    hooks="$(codex_termux_cmd notify-hook --action parse-selection --value "$hooks_choice")" || return $?

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
