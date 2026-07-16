# shellcheck shell=bash
_codex_compat_domain_dir="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
if [ -r "$_codex_compat_domain_dir/../shell/exec.sh" ]; then
    . "$_codex_compat_domain_dir/../shell/exec.sh"
elif [ -r "$_codex_compat_domain_dir/../../shell/exec.sh" ]; then
    . "$_codex_compat_domain_dir/../../shell/exec.sh"
else
    printf 'ERROR: missing wrapper shell domain: exec\n' >&2
    return 1
fi
unset _codex_compat_domain_dir
