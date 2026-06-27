#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="$ROOT_DIR/config/wrapper-version.env"
MODE="${1:-write}"

fail() {
    printf 'update-wrapper-version: %s\n' "$*" >&2
    exit 1
}

cd "$ROOT_DIR"

git rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
    fail 'not inside a git work tree'

date_full="$(git show -s --format=%cs HEAD 2>/dev/null || date +%Y-%m-%d)"
date_short="$(git show -s --format=%cd --date=format:%y%m%d HEAD 2>/dev/null || date +%y%m%d)"
count="$(git rev-list --first-parent --count --since="$date_full 00:00:00" --until="$date_full 23:59:59" HEAD 2>/dev/null || printf '0')"

if git diff --quiet -- . && git diff --cached --quiet -- .; then
    rev="$count"
else
    rev=$((count + 1))
fi
[ "$rev" -gt 0 ] || rev=1
version="$date_short-$rev"

tmp="$VERSION_FILE.$$"
{
    printf 'CODEX_TERMUX_WRAPPER_VERSION=%s\n' "$version"
    printf 'CODEX_TERMUX_WRAPPER_CHANNEL=termux\n'
    printf 'CODEX_TERMUX_WRAPPER_REPO=humtr/codex\n'
} >"$tmp"

case "$MODE" in
    --check|check)
        if cmp -s "$tmp" "$VERSION_FILE"; then
            rm -f "$tmp"
            exit 0
        fi
        rm -f "$tmp"
        fail "wrapper version metadata is stale; run tools/update-wrapper-version.sh"
        ;;
    write|"")
        mv "$tmp" "$VERSION_FILE"
        ;;
    *)
        rm -f "$tmp"
        fail "unknown mode: $MODE"
        ;;
esac
