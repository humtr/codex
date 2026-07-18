# shellcheck shell=bash
_codex_compat_domain_dir="$(cd "${BASH_SOURCE[0]%/*}" && pwd)"
if [ -r "$_codex_compat_domain_dir/../shell/doctor.sh" ]; then
    . "$_codex_compat_domain_dir/../shell/doctor.sh"
elif [ -r "$_codex_compat_domain_dir/../../shell/doctor.sh" ]; then
    . "$_codex_compat_domain_dir/../../shell/doctor.sh"
else
    printf 'ERROR: missing wrapper shell domain: doctor\n' >&2
    return 1
fi
unset _codex_compat_domain_dir
