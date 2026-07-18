#!/data/data/com.termux/files/usr/bin/sh
set -eu

termux_rg="/data/data/com.termux/files/usr/bin/rg"
real_rg="${0}.real"

if [ -x "$termux_rg" ]; then
    exec "$termux_rg" "$@"
fi

exec "$real_rg" "$@"
