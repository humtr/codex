# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_restore_backup() {
    local public="$1" base latest
    base="$(basename "$public")"
    latest="$(ls -t "$CODEX_TERMUX_BACKUP_DIR"/"$base".*.bak 2>/dev/null | sed -n '1p' || true)"
    if [ -n "$latest" ]; then
        cp -Pp "$latest" "$public"
        codex_say "$(codex_ui_text_get restored_backup "$public" "$latest")"
    fi
}

codex_remove() {
    if codex_termux_cmd file-has-marker \
        --path "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"; then
        rm -f "$CODEX_TERMUX_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
    codex_rm_rf_managed "$CODEX_TERMUX_ROOT" || return $?
    codex_say "$(codex_ui_text_get removed_runtime "$CODEX_TERMUX_STATE_DIR")"
}
