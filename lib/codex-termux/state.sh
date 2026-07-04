codex_load_wrapper_source_config() {
    [ -r "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" ] || return 0
    # shellcheck disable=SC1090
    . "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
    codex_normalize_wrapper_source_config
}

codex_normalize_wrapper_source_config() {
    local exports
    exports="$(codex_termux_cmd wrapper-source-env \
        --repo "${CODEX_TERMUX_WRAPPER_REPO:-}" \
        --ref "${CODEX_TERMUX_WRAPPER_REF:-}" \
        --token "${CODEX_TERMUX_WRAPPER_TOKEN:-}" \
        --git-repo "${CODEX_TERMUX_WRAPPER_GIT_REPO:-}" \
        --git-ref "${CODEX_TERMUX_WRAPPER_GIT_REF:-}" \
        --git-token "${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" \
        --release-token "${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-}")" || return $?
    [ -z "$exports" ] || eval "$exports"
}

codex_wrapper_auth_token() {
    codex_termux_cmd wrapper-auth-token \
        --token "${CODEX_TERMUX_WRAPPER_TOKEN:-}" \
        --git-token "${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" \
        --release-token "${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-}" \
        --github-token "${GITHUB_TOKEN:-}" \
        --allow-gh 1
}

codex_parent_dir() {
    codex_termux_cmd parent-dir --path "$1"
}

codex_tmp_dir() {
    local candidate
    for candidate in "${CODEX_TERMUX_TMPDIR:-}" "${TMPDIR:-}" "$CODEX_TERMUX_PREFIX/tmp"; do
        [ -n "$candidate" ] || continue
        case "$candidate" in
            /tmp) continue ;;
            /*) ;;
            *) continue ;;
        esac
        if mkdir -p "$candidate" 2>/dev/null && [ -d "$candidate" ] && [ -w "$candidate" ]; then
            codex_termux_cmd strip-trailing-slashes --path "$candidate"
            return 0
        fi
    done
    codex_fail "No writable Termux temporary directory is available"
    return 1
}

codex_mktemp_dir() {
    local prefix="${1:-codex-tmp}" tmpdir
    tmpdir="$(codex_tmp_dir)" || return $?
    mktemp -d "$tmpdir/$prefix.XXXXXX"
}

codex_mktemp_file() {
    local prefix="${1:-codex-tmp}" tmpdir
    tmpdir="$(codex_tmp_dir)" || return $?
    mktemp "$tmpdir/$prefix.XXXXXX"
}

codex_assert_managed_tree_target() {
    local path="$1" label="${2:-managed tree target}" tmpdir
    tmpdir="$(codex_tmp_dir 2>/dev/null || printf '%s\n' "$CODEX_TERMUX_PREFIX/tmp")"
    codex_termux_cmd managed-tree-target-ok \
        --path "$path" \
        --label "$label" \
        --home "${CODEX_TERMUX_HOME:-${HOME:-}}" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --tmpdir "$tmpdir" \
        --root "$CODEX_TERMUX_ROOT" \
        --state "$CODEX_TERMUX_STATE_DIR"
}

codex_rm_rf_managed() {
    local path
    for path in "$@"; do
        codex_assert_managed_tree_target "$path" "destructive target" || return $?
    done
    rm -rf "$@"
}

codex_sha256() {
    codex_termux_cmd hash-file --path "$1"
}

codex_now() {
    date -Is
}

codex_termux_package_root() {
    local source_root="$CODEX_TERMUX_WRAPPER_ROOT" root_dir="${ROOT_DIR:-}" python_path
    python_path="$source_root/tools${root_dir:+:$root_dir/tools}"
    python_path="$python_path:$CODEX_TERMUX_MANAGER_DIR"
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$python_path${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -B -m codex_termux.cli helper-package-root --source-root "$source_root" --root-dir "$root_dir" \
            --manager-dir "$CODEX_TERMUX_MANAGER_DIR" || {
        codex_fail "Internal helper package is unavailable"
        return 1
    }
}

codex_termux_cmd() {
    local package_root
    package_root="$(codex_termux_package_root)" || return 1
    CODEX_TERMUX_HOME="$CODEX_TERMUX_HOME" \
    CODEX_TERMUX_PROFILE_ROOT="$CODEX_TERMUX_PROFILE_ROOT" \
    CODEX_TERMUX_STATE_DIR="$CODEX_TERMUX_STATE_DIR" \
    CODEX_TERMUX_LAST_PROFILE_FILE="$CODEX_TERMUX_LAST_PROFILE_FILE" \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -B -m codex_termux.cli "$@"
}

codex_termux_activation_cmd() {
    local action="$1" shell_lib="$CODEX_TERMUX_SHELL_LIB" metadata_env
    shift
    metadata_env="$(codex_termux_cmd wrapper-metadata-env \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" || return 1
    eval "$metadata_env"
    codex_termux_cmd "$action" \
        --current-link "$CODEX_TERMUX_CURRENT_LINK" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT" \
        --updated-at "$(codex_now)" \
        --shell-bin "${BASH:-bash}" \
        --shell-lib "$shell_lib" \
        --home "$CODEX_TERMUX_HOME" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --resolv-conf "$CODEX_TERMUX_RESOLV_CONF" \
        --cert-file "$CODEX_TERMUX_CERT_FILE" \
        --cert-dir "$CODEX_TERMUX_CERT_DIR" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        "$@"
}

codex_lock_is_held() {
    local pid="${BASHPID:-$$}"
    [ "${CODEX_TERMUX_LOCK_HELD:-0}" = "1" ] || return 1
    [ "$(readlink "/proc/$pid/fd/9" 2>/dev/null || true)" = "$CODEX_TERMUX_LOCK_FILE" ]
}

codex_with_lock() {
    local cmd="$1"
    shift
    if codex_lock_is_held; then
        "$cmd" "$@"
        return $?
    fi
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    if command -v flock >/dev/null 2>&1; then
        (
            if ! flock -w "$CODEX_TERMUX_LOCK_WAIT_SECONDS" -x 9; then
                codex_fail "Another mutation operation is already in progress: $CODEX_TERMUX_LOCK_FILE"
                exit 75
            fi
            export CODEX_TERMUX_LOCK_HELD=1
            "$cmd" "$@"
        ) 9>"$CODEX_TERMUX_LOCK_FILE"
    else
        local lock_dir="${CODEX_TERMUX_LOCK_FILE}.d" waited=0
        while ! mkdir "$lock_dir" 2>/dev/null; do
            sleep 1
            waited=$((waited + 1))
            if [ "$waited" -ge "$CODEX_TERMUX_LOCK_WAIT_SECONDS" ]; then
                codex_fail "Another mutation operation is already in progress: $CODEX_TERMUX_LOCK_FILE"
                return 75
            fi
        done
        (
            trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT
            export CODEX_TERMUX_LOCK_HELD=1
            "$cmd" "$@"
        )
    fi
}
