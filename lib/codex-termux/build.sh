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
    [ -x "$payload_dir/codex" ] && [ -x "$payload_dir/codex-code-mode-host" ] || return 1
    codex_rm_rf_managed "$complete_dir" || return $?
    mkdir -p "$complete_dir"
    for name in codex codex-code-mode-host codex-resources codex-path codex-package.json runtime-build.json; do
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
        "$complete_dir/codex-code-mode-host" \
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
