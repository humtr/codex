# shellcheck shell=bash
_codex_compat_domain_dir="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
if [ -r "$_codex_compat_domain_dir/../shell/ui.sh" ]; then
    . "$_codex_compat_domain_dir/../shell/ui.sh"
elif [ -r "$_codex_compat_domain_dir/../../shell/ui.sh" ]; then
    . "$_codex_compat_domain_dir/../../shell/ui.sh"
else
    printf 'ERROR: missing wrapper shell domain: ui\n' >&2
    return 1
fi
unset _codex_compat_domain_dir
