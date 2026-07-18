#!/usr/bin/env bash
set -u

_codex_compat_dir="${BASH_SOURCE[0]%/*}"
[ "$_codex_compat_dir" = "${BASH_SOURCE[0]}" ] && _codex_compat_dir="."
_codex_compat_dir="$(cd "$_codex_compat_dir" && pwd)"

for _codex_loader in     "$_codex_compat_dir/../shell/loader.sh"     "${CODEX_TERMUX_SOURCE_DIR:-}/shell/loader.sh"     "${CODEX_TERMUX_MANAGER_DIR:-}/source/shell/loader.sh"
do
    [ -n "$_codex_loader" ] || continue
    if [ -r "$_codex_loader" ]; then
        CODEX_TERMUX_WRAPPER_ROOT="${CODEX_TERMUX_WRAPPER_ROOT:-$(cd "$(dirname "$_codex_loader")/.." && pwd)}"
        . "$_codex_loader"
        unset _codex_compat_dir _codex_loader
        return 0 2>/dev/null || exit 0
    fi
done

printf 'ERROR: missing Codex Termux shell loader
' >&2
unset _codex_compat_dir _codex_loader
return 1 2>/dev/null || exit 1
