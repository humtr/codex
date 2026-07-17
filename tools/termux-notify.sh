#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NOTIFY_ENTRY=""
for candidate in \
    "$SCRIPT_DIR/../libexec/notify" \
    "$SCRIPT_DIR/libexec/notify" \
    "$SCRIPT_DIR/source/libexec/notify"
do
    if [ -f "$candidate" ]; then
        NOTIFY_ENTRY="$candidate"
        break
    fi
done
[ -n "$NOTIFY_ENTRY" ] || {
    printf 'termux-notify: unified notify executable is unavailable\n' >&2
    exit 127
}

case "${1:-}" in
    --open-termux)
        exec /data/data/com.termux/files/usr/bin/python3 -B "$NOTIFY_ENTRY" open --target termux
        ;;
    --open-tmux)
        [ "$#" -ge 2 ] || {
            printf 'termux-notify: --open-tmux requires a target\n' >&2
            exit 2
        }
        exec /data/data/com.termux/files/usr/bin/python3 -B "$NOTIFY_ENTRY" open --target tmux --tmux-target "$2"
        ;;
    "")
        exec /data/data/com.termux/files/usr/bin/python3 -B "$NOTIFY_ENTRY" send
        ;;
    *)
        printf 'termux-notify: unknown option: %s\n' "$1" >&2
        exit 2
        ;;
esac
