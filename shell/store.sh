# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_store_id() {
    local version="$1" sha="$2" tree_sha="${3:-}" builder_sha="unknown" bwrap_sha="unknown" rg_sha="unknown" support_dir
    support_dir="$(codex_termux_cmd support-source-dir \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")"
    if [ -r "$CODEX_TERMUX_RUNTIME_BUILDER" ]; then
        builder_sha="$(codex_sha256 "$CODEX_TERMUX_RUNTIME_BUILDER")"
    fi
    if [ -r "$support_dir/bwrap-termux-compat.py" ]; then
        bwrap_sha="$(codex_sha256 "$support_dir/bwrap-termux-compat.py")"
    fi
    if [ -r "$support_dir/rg-termux-shim.sh" ]; then
        rg_sha="$(codex_sha256 "$support_dir/rg-termux-shim.sh")"
    fi
    codex_termux_cmd store-id \
        --version "$version" \
        --sha256 "$sha" \
        --builder-sha256 "$builder_sha" \
        --bwrap-sha256 "$bwrap_sha" \
        --rg-sha256 "$rg_sha" \
        --tree-sha256 "$tree_sha"
}

codex_validate_runtime_retention() {
    codex_termux_cmd runtime-retention-ok --value "$CODEX_TERMUX_RUNTIME_RETENTION" || {
        codex_fail "CODEX_TERMUX_RUNTIME_RETENTION must be an integer greater than zero"
        return 2
    }
}

codex_prune_runtime_store() {
    local protected_runtime_arg=()
    codex_validate_runtime_retention || return $?
    if [ -n "${CODEX_SELF_EXE:-}" ]; then
        protected_runtime_arg=(--protect-runtime-path "$(codex_parent_dir "$CODEX_SELF_EXE")")
    fi
    codex_termux_cmd store-prune \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --retention "$CODEX_TERMUX_RUNTIME_RETENTION" \
        --current-link "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        "${protected_runtime_arg[@]}" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" >/dev/null
}

codex_activate_tuple_unlocked() {
    local runtime_src="$1" version="$2" raw_sha="$3" runtime_sha="$4" package_spec="$5" raw_src="${6:-}"
    local raw_store_src="$CODEX_TERMUX_RAW_DIR" runtime_target raw_target
    local cleanup_raw=() runtime_tree_sha raw_tree_sha
    if [ -n "$raw_src" ]; then
        raw_store_src="$raw_src"
        cleanup_raw=(--cleanup-raw-source)
    fi
    runtime_tree_sha="$(codex_termux_cmd tree-digest --path "$(codex_termux_cmd resolve-path --path "$runtime_src")")"
    raw_tree_sha="$(codex_termux_cmd tree-digest --path "$(codex_termux_cmd resolve-path --path "$raw_store_src")")"
    runtime_target="$CODEX_TERMUX_RUNTIME_STORE_DIR/$(codex_store_id "$version" "$runtime_sha" "$runtime_tree_sha")"
    raw_target="$CODEX_TERMUX_RAW_STORE_DIR/$(codex_store_id "$version" "$raw_sha" "$raw_tree_sha")"
    codex_termux_activation_cmd activation-commit \
        --candidate-runtime "$runtime_src" \
        --candidate-raw "$raw_store_src" \
        --runtime-target "$runtime_target" \
        --raw-target "$raw_target" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --package-spec "$package_spec" \
        --cleanup-runtime-source \
        "${cleanup_raw[@]}" >/dev/null || return 1
    codex_prune_runtime_store
}

codex_activate_cached_runtime_unlocked() {
    local runtime_path="$1" raw_path="$2" version="$3" raw_sha="$4" runtime_sha="$5" package_spec="$6"
    local runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.use.$$" raw_complete="$CODEX_TERMUX_RAW_DIR.use.$$"
    codex_prepare_complete_runtime_tree "$runtime_path" "$runtime_complete" || return 1
    if ! codex_install_raw_vendor "$raw_path/vendor/aarch64-unknown-linux-musl" "$raw_complete"; then
        codex_rm_rf_managed "$runtime_complete" "$raw_complete" || return $?
        return 1
    fi
    if ! codex_activate_tuple_unlocked "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$raw_complete"; then
        codex_rm_rf_managed "$runtime_complete" "$raw_complete" || return $?
        return 1
    fi
    codex_ui_step switch_runtime "$version"
}

codex_try_verified_rollback() {
    [ -e "$CODEX_TERMUX_VERIFIED_LINK" ] || [ -L "$CODEX_TERMUX_VERIFIED_LINK" ] || return 1
    [ -x "$(codex_termux_cmd resolve-path --path "$CODEX_TERMUX_VERIFIED_LINK")/codex" ] || return 1
    [ "$(codex_termux_cmd resolve-path --path "$CODEX_TERMUX_RUNTIME_DIR")" != "$(codex_termux_cmd resolve-path --path "$CODEX_TERMUX_VERIFIED_LINK")" ] || return 1
    codex_with_lock codex_termux_activation_cmd activation-restore-verified >/dev/null || return $?
    codex_say "$(codex_ui_text_get restored_verified)"
}
