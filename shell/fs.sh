# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

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
