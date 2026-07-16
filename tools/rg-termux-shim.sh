#!/bin/sh
ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
exec "$ROOT_DIR/libexec/rg-termux-shim.sh" "$@"
