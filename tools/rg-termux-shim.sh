#!/bin/sh
set -eu
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
for candidate in "$HERE/../libexec/rg-termux-shim.sh" "$HERE/source/libexec/rg-termux-shim.sh"; do
    if [ -f "$candidate" ]; then
        exec "$candidate" "$@"
    fi
done
printf '%s
' 'runtime artifact implementation is unavailable' >&2
exit 1
