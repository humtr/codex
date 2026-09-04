#!/data/data/com.termux/files/usr/bin/sh
set -eu
umask 077
LC_ALL=C
export LC_ALL

fail() {
  printf '%s\n' "codex install: $*" >&2
  exit 1
}

script_dir=$(
  CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null &&
    pwd -P
) || fail 'cannot resolve installer directory'

bootstrap=$script_dir/bootstrap/codex-bootstrap
if [ ! -f "$bootstrap" ] || [ -L "$bootstrap" ] || [ ! -x "$bootstrap" ]; then
  fail 'bundled bootstrap must be a regular executable'
fi

exec "$bootstrap" "$@"
