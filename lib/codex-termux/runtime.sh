# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_replace_tree_atomic() {
    local source="$1" target="$2" backup="$3"
    local existed=0
    codex_assert_managed_tree_target "$source" "replacement source" || return $?
    codex_assert_managed_tree_target "$target" "replacement target" || return $?
    codex_assert_managed_tree_target "$backup" "replacement backup" || return $?
    codex_rm_rf_managed "$backup" || return $?
    if [ -e "$target" ] || [ -L "$target" ]; then
        mv "$target" "$backup" || return 1
        existed=1
    fi
    if mv "$source" "$target"; then
        codex_rm_rf_managed "$backup" || return $?
        return 0
    fi
    if [ "$existed" -eq 1 ]; then
        mv "$backup" "$target" || return 1
    fi
    return 1
}

codex_prepare_complete_runtime_tree() {
    local payload_dir="$1" complete_dir="$2" name support_dir
    support_dir="$(codex_termux_cmd support-source-dir \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")"
    if { [ ! -r "$support_dir/bwrap-termux-compat.py" ] || [ ! -r "$support_dir/rg-termux-shim.sh" ]; } &&
        [ -r "$payload_dir/bwrap-termux-compat.py" ] &&
        [ -r "$payload_dir/rg-termux-shim.sh" ]; then
        support_dir="$payload_dir"
    fi
    [ -x "$payload_dir/codex" ] || return 1
    codex_rm_rf_managed "$complete_dir" || return $?
    mkdir -p "$complete_dir"
    for name in codex codex-resources codex-path codex-package.json runtime-build.json; do
        [ -e "$payload_dir/$name" ] || {
            codex_rm_rf_managed "$complete_dir" || return $?
            return 1
        }
        cp -R "$payload_dir/$name" "$complete_dir/$name"
    done
    [ -x "$CODEX_TERMUX_RUNTIME_BUILDER" ] &&
        [ -r "$support_dir/bwrap-termux-compat.py" ] &&
        [ -r "$support_dir/rg-termux-shim.sh" ] || {
        codex_rm_rf_managed "$complete_dir" || return $?
        return 1
    }
    cp -R "$support_dir/bwrap-termux-compat.py" "$complete_dir/codex-path/bwrap"
    cp -R "$support_dir/rg-termux-shim.sh" "$complete_dir/codex-path/rg"
    rm -f "$complete_dir/codex-resources/bwrap.real"
    chmod 755 "$complete_dir/codex" \
        "$complete_dir/codex-path/bwrap" \
        "$complete_dir/codex-path/rg" \
        "$complete_dir/codex-path/rg.real" 2>/dev/null || true
}

codex_package_spec() {
    codex_termux_cmd package-spec \
        --requested "${1:-}" \
        --default "$CODEX_TERMUX_PACKAGE_SPEC_DEFAULT"
}

codex_fetch_package() {
    local requested="${1:-}" package_spec tmp pack_json pack_env tgz
    package_spec="$(codex_package_spec "$requested")"
    tmp="$(codex_mktemp_dir codex-pack)" || return 1
    pack_json="$tmp/pack.json"
    codex_ui_step fetch_package "$package_spec"
    if ! npm pack "$package_spec" --json --pack-destination "$tmp" >"$pack_json"; then
        rm -rf "$tmp"
        codex_fail "Failed to fetch $package_spec"
        return 1
    fi
    pack_env="$(codex_termux_cmd package-fields-env --json-file "$pack_json")" || {
        rm -rf "$tmp"
        return 1
    }
    eval "$pack_env"
    tgz="$tmp/$CODEX_PACKAGE_FILENAME"
    if [ ! -f "$tgz" ]; then
        rm -rf "$tmp"
        codex_fail "Package fetch did not produce the expected tarball"
        return 1
    fi
    codex_ui_step validate_archive
    mkdir -p "$tmp/package"
    if ! codex_termux_cmd validate-tarball --path "$tgz" >/dev/null 2>&1; then
        rm -rf "$tmp"
        codex_fail "Package archive contains unsafe paths: $tgz"
        return 1
    fi
    codex_ui_step unpack_archive
    if ! tar -xzf "$tgz" -C "$tmp/package" --strip-components=1; then
        rm -rf "$tmp"
        codex_fail "Failed to extract $tgz"
        return 1
    fi
    printf '%s\t%s\t%s\t%s\n' "$tmp" "$tmp/package/vendor/aarch64-unknown-linux-musl" "$CODEX_PACKAGE_VERSION" "$package_spec"
}

codex_install_raw_vendor() {
    local src_vendor="$1" target_dir="${2:-$CODEX_TERMUX_RAW_DIR}" staged
    staged="$target_dir.new.$$"
    codex_rm_rf_managed "$staged" || return $?
    mkdir -p "$staged/vendor"
    cp -R "$src_vendor" "$staged/vendor/aarch64-unknown-linux-musl"
    chmod 755 "$staged/vendor/aarch64-unknown-linux-musl/bin/codex"
    if ! codex_replace_tree_atomic "$staged" "$target_dir" "$target_dir.old"; then
        codex_rm_rf_managed "$staged" || return $?
        return 1
    fi
}

codex_build_runtime_tree() {
    local raw_vendor="$1"
    local runtime_dir="$2"
    local log_file="$3"
    local builder="$CODEX_TERMUX_RUNTIME_BUILDER"
    local report_file="${log_file}.report.json"

    mkdir -p "$(codex_parent_dir "$runtime_dir")"
    if ! python3 "$builder" "$raw_vendor" --runtime-dir "$runtime_dir" --report-json "$report_file" >"$log_file" 2>&1; then
        return 70
    fi
    if ! codex_runtime_exec "$runtime_dir/codex" --version >/dev/null 2>&1; then
        return 72
    fi
    return 0
}

codex_runtime_build_cached_unlocked() {
    local version="${1:-unknown}" package_spec="${2:-local}" report build_stdout raw_sha runtime_sha
    local runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.build.$$" runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ] || return 1
    mkdir -p "$CODEX_TERMUX_STATE_DIR" "$CODEX_TERMUX_DOCTOR_DIR"
    report="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.json"
    build_stdout="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"
    codex_rm_rf_managed "$runtime_stage" || return $?
    if [ "${CODEX_TERMUX_BUILD_VERBOSE:-0}" = "1" ]; then
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report"; then
            codex_rm_rf_managed "$runtime_stage" || return $?
            return 1
        fi
    else
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report" >"$build_stdout" 2>&1; then
            codex_rm_rf_managed "$runtime_stage" || return $?
            return 1
        fi
    fi
    codex_ui_step assemble_runtime
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        codex_rm_rf_managed "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    raw_sha="$(codex_sha256 "$CODEX_TERMUX_RAW_VENDOR/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    codex_rm_rf_managed "$runtime_stage" || return $?
    codex_ui_step smoke_test_runtime
    if ! codex_runtime_exec "$runtime_complete/codex" --version >/dev/null 2>&1; then
        codex_rm_rf_managed "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step activate_runtime
    codex_activate_tuple_unlocked "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec"
}

codex_runtime_install_cached_unlocked() {
    local plan_env
    [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ] &&
    codex_termux_cmd raw-integrity \
        --raw-binary "$CODEX_TERMUX_RAW_VENDOR/bin/codex" \
        --state-path "$CODEX_TERMUX_STATE_FILE" || {
        codex_fail "$(codex_ui_text_get cached_raw_integrity_failed)"
        return 1
    }
    plan_env="$(codex_termux_cmd runtime-cached-build-plan-env \
        --state-file "$CODEX_TERMUX_STATE_FILE")" || return $?
    eval "$plan_env"
    codex_runtime_build_cached_unlocked \
        "$CODEX_RUNTIME_CACHED_VERSION" \
        "$CODEX_RUNTIME_CACHED_PACKAGE_SPEC" || return $?
}

codex_runtime_install_cached() {
    local status=0
    codex_with_lock codex_runtime_install_cached_unlocked || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

codex_repair_install_support() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_ui_step repair_support
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 CODEX_TERMUX_INSTALL_SURFACE=0 bash "$source" install support >/dev/null
}

codex_repair_diagnose_action() {
    local field="${1:-action}"
    codex_termux_cmd repair-diagnose \
        --managed-shell "$CODEX_TERMUX_MANAGED_SHELL" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --public-codex "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --support-dir "$(codex_termux_cmd support-source-dir --manager-dir "$CODEX_TERMUX_MANAGER_DIR" --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" \
        --manifest-path "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --raw-binary "$CODEX_TERMUX_RAW_VENDOR/bin/codex" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --field "$field"
}

codex_runtime_apply_action() {
    local action="$1" intent="${2:-readiness}" plan_env status=0
    plan_env="$(codex_termux_cmd runtime-action-plan-env \
        --action "$action" \
        --intent "$intent")" || return $?
    eval "$plan_env"
    [ -z "$CODEX_RUNTIME_ACTION_STEP" ] || codex_ui_step "$CODEX_RUNTIME_ACTION_STEP"
    case "$CODEX_RUNTIME_ACTION_KIND" in
        noop)
            return 0
            ;;
        refresh_metadata)
            codex_refresh_runtime_metadata || status=$?
            ;;
        restore_verified)
            codex_try_verified_rollback || status=$?
            ;;
        rebuild_cached)
            codex_runtime_install_cached || status=$?
            ;;
        error)
            codex_fail "$CODEX_RUNTIME_ACTION_ERROR"
            return "$CODEX_RUNTIME_ACTION_EXIT_CODE"
            ;;
        *)
            codex_fail "Unknown runtime action executor: $CODEX_RUNTIME_ACTION_KIND"
            return 1
            ;;
    esac
    [ "$status" -eq 0 ] || return "$status"
    [ "$CODEX_RUNTIME_ACTION_REFRESH_AFTER" != "1" ] || codex_refresh_runtime_metadata
}

codex_repair_apply() {
    local action support_attempted=0
    while :; do
        action="$(codex_repair_diagnose_action action)" || return $?
        case "$action" in
            refresh_support)
                if [ "$support_attempted" = "1" ]; then
                    codex_fail "Support layer repair did not complete; run bash install.sh from a wrapper checkout"
                    return 1
                fi
                support_attempted=1
                codex_repair_install_support || return $?
                ;;
            *)
                codex_runtime_apply_action "$action" repair
                return $?
                ;;
        esac
    done
}

codex_repair_core_unlocked() {
    codex_validate_runtime_retention || return $?
    codex_repair_apply
}

codex_repair_public() {
    local status=0
    codex_with_lock codex_repair_core_unlocked || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    [ "$status" -eq 0 ] || return "$status"
    codex_version || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

codex_runtime_install_upstream_unlocked() {
    local requested="${1:-}" fetched tmp vendor version spec raw_stage runtime_stage runtime_complete raw_sha runtime_sha
    fetched="$(codex_fetch_package "$requested")" || return $?
    IFS=$'\t' read -r tmp vendor version spec <<EOF
$fetched
EOF
    raw_stage="$CODEX_TERMUX_RAW_DIR.update.$$"
    runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.update.$$"
    runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    mkdir -p "$CODEX_TERMUX_DOCTOR_DIR"
    codex_ui_step stage_raw
    if ! codex_install_raw_vendor "$vendor" "$raw_stage"; then
        rm -rf "$tmp"
        return 1
    fi
    codex_ui_step build_runtime
    if ! codex_build_runtime_tree "$raw_stage/vendor/aarch64-unknown-linux-musl" "$runtime_stage" "$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step assemble_runtime
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    raw_sha="$(codex_sha256 "$raw_stage/vendor/aarch64-unknown-linux-musl/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    codex_rm_rf_managed "$runtime_stage" || return $?
    codex_ui_step smoke_test_runtime
    if ! codex_runtime_exec "$runtime_complete/codex" --version >/dev/null 2>&1; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step activate_runtime
    if ! codex_activate_tuple_unlocked "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$spec" "$raw_stage"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_complete" || return $?
        return 1
    fi
    rm -rf "$tmp"
    codex_ui_step switch_runtime "$(codex_display_version "$version")"
}

codex_runtime_install_upstream() {
    local status=0
    codex_validate_runtime_retention || return $?
    codex_with_lock codex_runtime_install_upstream_unlocked "${1:-}" || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

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


codex_refresh_runtime_metadata_unlocked() {
    local metadata_current=0 plan_env
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" && metadata_current=1
    plan_env="$(codex_termux_cmd runtime-refresh-plan-env \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --metadata-current "$metadata_current")" || return $?
    eval "$plan_env"
    [ "$CODEX_RUNTIME_REFRESH_ACTION" = "activate" ] || return 0
    codex_activate_tuple_unlocked \
        "$CODEX_TERMUX_RUNTIME_DIR" \
        "$CODEX_RUNTIME_REFRESH_VERSION" \
        "$CODEX_RUNTIME_REFRESH_RAW_SHA256" \
        "$CODEX_RUNTIME_REFRESH_RUNTIME_SHA256" \
        "$CODEX_RUNTIME_REFRESH_PACKAGE_SPEC"
}

codex_refresh_runtime_metadata() {
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" && return 0
    codex_with_lock codex_refresh_runtime_metadata_unlocked
}

codex_ensure_runtime_ready() {
    local action
    action="$(codex_repair_diagnose_action readiness-action)" || return $?
    codex_runtime_apply_action "$action" readiness
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
