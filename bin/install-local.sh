#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${BASH_SOURCE[0]%/*}"
[ "$ROOT_DIR" = "${BASH_SOURCE[0]}" ] && ROOT_DIR="."
ROOT_DIR="$(cd "$ROOT_DIR/.." && pwd)"

CODEX_TERMUX_LOCAL_INSTALL=1 exec bash "$ROOT_DIR/install.sh" "$@"
