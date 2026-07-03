# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_latest_linux_arm64_version() {
    if command -v timeout >/dev/null 2>&1; then
        timeout "$CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | codex_termux_cmd strip-quotes
    else
        npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | codex_termux_cmd strip-quotes
    fi
}

codex_mark_auto_update_checked() {
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    date +%s >"$CODEX_TERMUX_AUTO_UPDATE_STAMP"
}

codex_write_pending_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\n' "$version" >"$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_clear_pending_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_write_failed_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\t%s\n' "$version" "$(date +%s)" >"$CODEX_TERMUX_AUTO_UPDATE_FAILED"
}

codex_clear_failed_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_FAILED"
}

codex_prompt_update() {
    local current="$1" latest="$2" choice display_current display_latest
    [ -t 0 ] && [ -t 2 ] || return 1
    display_current="$(codex_display_version "$current")"
    display_latest="$(codex_display_version "$latest")"
    codex_confirm_menu \
        "$(codex_ui_text_get update_available_title)" \
        "$display_current -> $display_latest" \
        y "$display_latest" "$(codex_ui_badge update)" \
        N "$display_current" "$(codex_ui_badge current) $(codex_ui_badge keep)" \
        "$(codex_ui_text_get apply_update_prompt)" \
        cancel || {
        [ "$?" -eq 130 ] && return 130
        return 1
    }
    choice="$(codex_prompt_result)"
    choice="$(codex_termux_cmd update-prompt-decision --choice "$choice")" || return $?
    case "$choice" in
        apply)
            return 0
            ;;
        keep)
            codex_say "$(codex_ui_text_get current_kept "$(codex_display_version "$current")")"
            return 1
            ;;
        *)
            return 130
            ;;
    esac
}

codex_auto_update_if_needed() {
    local current latest plan_env now
    codex_termux_cmd runtime-layout-ok \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --support-dir "$(codex_termux_cmd support-source-dir --manager-dir "$CODEX_TERMUX_MANAGER_DIR" --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" &&
    [ -r "$CODEX_TERMUX_STATE_FILE" ] &&
    [ -r "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" ] &&
    [ -x "$CODEX_TERMUX_RUNTIME_BUILDER" ] &&
    codex_termux_cmd runtime-integrity \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --manifest-path "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" || return 0
    current="$(codex_termux_cmd state-read-field --state-file "$CODEX_TERMUX_STATE_FILE" --field version)"
    now="$(date +%s)"
    plan_env="$(codex_termux_cmd auto-update-check-plan-env \
        --enabled "$CODEX_TERMUX_AUTO_UPDATE" \
        --mode "${CODEX_TERMUX_AUTO_UPDATE_MODE:-prompt}" \
        --current "$current" \
        --pending-file "$CODEX_TERMUX_AUTO_UPDATE_PENDING" \
        --now "$now" \
        --last-file "$CODEX_TERMUX_AUTO_UPDATE_STAMP" \
        --interval "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS")" || return $?
    eval "$plan_env"
    [ "$CODEX_AUTO_UPDATE_CLEAR_PENDING" = "0" ] || codex_clear_pending_auto_update
    case "$CODEX_AUTO_UPDATE_ACTION" in
        skip)
            return 0
            ;;
        use_pending)
            latest="$CODEX_AUTO_UPDATE_LATEST"
            ;;
        fetch)
            codex_mark_auto_update_checked
            latest="$(codex_latest_linux_arm64_version || true)"
            if [ -z "$latest" ]; then
                [ "$CODEX_AUTO_UPDATE_CLEAR_PENDING_ON_EMPTY_LATEST" = "0" ] || codex_clear_pending_auto_update
                return 0
            fi
            ;;
        *)
            return 0
            ;;
    esac
    if [ "$latest" != "$current" ]; then
        codex_write_pending_auto_update "$latest"
        now="$(date +%s)"
        plan_env="$(codex_termux_cmd auto-update-apply-plan-env \
            --current "$current" \
            --latest "$latest" \
            --failed-record-file "$CODEX_TERMUX_AUTO_UPDATE_FAILED" \
            --mode "$CODEX_AUTO_UPDATE_MODE" \
            --now "$now" \
            --interval "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS")" || return $?
        eval "$plan_env"
        case "$CODEX_AUTO_UPDATE_ACTION" in
            install)
                codex_ui_step update_runtime "$current" "$latest"
                if codex_runtime_install_upstream "$latest"; then
                    codex_clear_pending_auto_update
                    codex_clear_failed_auto_update
                else
                    codex_write_failed_auto_update "$latest"
                    codex_say "$(codex_ui_text_get update_failed_continue "$current")"
                    return 1
                fi
                ;;
            prompt)
                codex_prompt_update "$current" "$latest"
                case "$?" in
                    0)
                        codex_ui_step update_runtime "$current" "$latest"
                        if codex_runtime_install_upstream "$latest"; then
                            codex_clear_pending_auto_update
                            codex_clear_failed_auto_update
                        else
                            codex_write_failed_auto_update "$latest"
                            codex_say "$(codex_ui_text_get update_failed_continue "$current")"
                            return 1
                        fi
                        ;;
                    130)
                        return 130
                        ;;
                esac
                ;;
        esac
    else
        codex_clear_pending_auto_update
    fi
    return 0
}

codex_upstream_release_date() {
    local version="${1:-}" release_date
    [ -n "$version" ] || return 0
    release_date="$(codex_termux_cmd upstream-release-cache-read \
        --cache "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" \
        --version "$version" 2>/dev/null || true)"
    if [ -n "$release_date" ]; then
        printf '%s\n' "$release_date"
        return 0
    fi
    if command -v timeout >/dev/null 2>&1; then
        release_date="$(timeout "$CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex time --json 2>/dev/null | \
            codex_termux_cmd upstream-release-date --version "$version" || true)"
    else
        release_date="$(npm view @openai/codex time --json 2>/dev/null | \
            codex_termux_cmd upstream-release-date --version "$version" || true)"
    fi
    if [ -n "$release_date" ]; then
        codex_termux_cmd upstream-release-cache-write \
            --cache "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" \
            --version "$version" \
            --release-date "$release_date" >/dev/null 2>&1 || true
        printf '%s\n' "$release_date"
    fi
}

codex_version() {
    local upstream upstream_version upstream_date runtime_date runtime_date_value metadata_env status=0
    codex_status_clear
    if upstream="$(codex_run_current_runtime --version 2>/dev/null)"; then
        status=0
    else
        status=$?
        upstream=""
    fi
    upstream_version="$(codex_termux_cmd upstream-version --text "$upstream")"
    upstream_date="$(codex_upstream_release_date "$upstream_version" || true)"
    runtime_date_value="$(codex_termux_cmd registry-active-runtime-date \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" || true)"
    runtime_date="$(codex_termux_cmd display-runtime-date --value "$runtime_date_value")"
    metadata_env="$(codex_termux_cmd wrapper-metadata-env \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" || return 1
    eval "$metadata_env"
    codex_termux_cmd version-report \
        --upstream "$upstream" \
        --upstream-date "$upstream_date" \
        --runtime-date "$runtime_date" \
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT" >&2
    return "$status"
}
