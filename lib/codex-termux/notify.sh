# shellcheck shell=bash
_codex_compat_domain_dir="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
if [ -r "$_codex_compat_domain_dir/../shell/notify.sh" ]; then
    . "$_codex_compat_domain_dir/../shell/notify.sh"
elif [ -r "$_codex_compat_domain_dir/../../shell/notify.sh" ]; then
    . "$_codex_compat_domain_dir/../../shell/notify.sh"
else
    printf 'ERROR: missing wrapper shell domain: notify\n' >&2
    return 1
fi
unset _codex_compat_domain_dir
