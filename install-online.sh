#!/data/data/com.termux/files/usr/bin/sh
set -eu
umask 077
LC_ALL=C
export LC_ALL

fail() {
  printf '%s\n' "codex online install: $*" >&2
  exit 1
}

if [ "$#" -ne 0 ]; then
  fail 'usage: install-online.sh'
fi

case ${HOME-} in /*) ;; *) fail 'HOME must be an absolute path' ;; esac
case ${PREFIX-} in /*) ;; *) fail 'PREFIX must be an absolute path' ;; esac
case ${TMPDIR-} in /*) ;; *) fail 'TMPDIR must be an absolute path' ;; esac
[ -d "$TMPDIR" ] && [ ! -L "$TMPDIR" ] || fail 'TMPDIR must be a real directory'

curl=$PREFIX/bin/curl
openssl=$PREFIX/bin/openssl
[ -x "$curl" ] || fail 'Termux curl is unavailable'
[ -x "$openssl" ] || fail 'Termux OpenSSL is unavailable'

LOCAL_INSTALL_SOURCE_SHA='b58f2432c6a9aa9b018f3afc9502d16680e1b043'
BOOTSTRAP_KEY_SHA256='62ab1640b6b4e63afbd5952d11a0bd0a9f1cb78ddde2472e003a42c4db2b832c'
BOOTSTRAP_KEY_URL='https://github.com/humtr/codex/releases/download/local-hosted-0-154-0-37fbbd8033b8-rald45-transition/update-public-key.pem'
STABLE_INDEX_URL='https://raw.githubusercontent.com/humtr/codex/main/update-index-v1'
LOCAL_INSTALL_RAW_BASE="https://raw.githubusercontent.com/humtr/codex/$LOCAL_INSTALL_SOURCE_SHA"

unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY all_proxy NO_PROXY no_proxy || :

work=$TMPDIR/.codex-online-install.$$
[ ! -e "$work" ] && [ ! -L "$work" ] || fail 'private acquisition workspace already exists'
mkdir "$work"
cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT HUP INT TERM

fetch_bounded() {
  url=$1
  output=$2
  maximum=$3
  [ ! -e "$output" ] && [ ! -L "$output" ] || fail 'download target already exists'
  "$curl" -q --fail --silent --show-error --location     --proto '=https' --proto-redir '=https' --tlsv1.2     --connect-timeout 15 --max-time 300 --max-filesize "$maximum"     --output "$output" "$url" || fail 'HTTPS acquisition failed'
  [ -f "$output" ] && [ ! -L "$output" ] || fail 'download did not create a regular file'
  size=$(wc -c < "$output" | tr -d '[:space:]') || fail 'could not inspect downloaded file'
  case $size in ''|*[!0-9]*) fail 'downloaded file size is invalid' ;; esac
  [ "$size" -le "$maximum" ] || fail 'downloaded file exceeds its byte bound'
}

sha256_hex() {
  file=$1
  digest=$("$openssl" dgst -sha256 -r "$file" 2>/dev/null | awk '{print $1}') ||
    fail 'OpenSSL SHA-256 failed'
  [ "${#digest}" -eq 64 ] || fail 'OpenSSL SHA-256 output is malformed'
  case $digest in *[!0-9a-f]*) fail 'OpenSSL SHA-256 output is malformed' ;; esac
  printf '%s\n' "$digest"
}

verify_ed25519() {
  payload=$1
  signature=$2
  key=$3
  [ "$(wc -c < "$signature" | tr -d '[:space:]')" -eq 64 ] ||
    fail 'signature size is invalid'
  "$openssl" pkeyutl -verify -rawin -pubin -inkey "$key"     -in "$payload" -sigfile "$signature" >/dev/null 2>&1 ||
    fail 'signature verification failed'
}

key=$work/update-public-key.pem
fetch_bounded "$BOOTSTRAP_KEY_URL" "$key" 16384
[ "$(sha256_hex "$key")" = "$BOOTSTRAP_KEY_SHA256" ] ||
  fail 'bootstrap public key does not match the pinned authority'

index=$work/update-index-v1
index_sig=$work/update-index-v1.sig
fetch_bounded "$STABLE_INDEX_URL" "$index" 16384
fetch_bounded "$STABLE_INDEX_URL.sig" "$index_sig" 1024
verify_ed25519 "$index" "$index_sig" "$key"

[ "$(wc -l < "$index" | tr -d '[:space:]')" -eq 4 ] ||
  fail 'stable index record count is invalid'
[ "$(sed -n '1p' "$index")" = 'codex-update-index-v1' ] ||
  fail 'stable index format is invalid'
[ "$(sed -n '2p' "$index")" = "$(printf 'channel\tstable')" ] ||
  fail 'stable index channel is invalid'
generation=$(sed -n '3s/^generation_id	//p' "$index")
release_base=$(sed -n '4s/^release_base	//p' "$index")
[ -n "$generation" ] && [ "${#generation}" -le 512 ] ||
  fail 'stable generation identity is invalid'
first=$(printf '%s' "$generation" | cut -c 1)
case $first in [A-Za-z0-9]) ;; *) fail 'stable generation identity is invalid' ;; esac
case $generation in *[!A-Za-z0-9._~-]*) fail 'stable generation identity is invalid' ;; esac
case $release_base in
  https://*/"$generation"/) ;;
  *) fail 'stable release base is invalid' ;;
esac
case $release_base in *[[:space:]?#\\]*) fail 'stable release base is invalid' ;; esac

expected_index=$work/expected-update-index-v1
printf 'codex-update-index-v1\nchannel\tstable\ngeneration_id\t%s\nrelease_base\t%s\n'   "$generation" "$release_base" > "$expected_index"
cmp "$index" "$expected_index" >/dev/null 2>&1 ||
  fail 'stable index is not canonical'

release_dir=$work/release
mkdir "$release_dir"
manifest=$release_dir/release.manifest
release_sig=$release_dir/release.sig
fetch_bounded "${release_base}release.manifest" "$manifest" 131072
fetch_bounded "${release_base}release.sig" "$release_sig" 1024
verify_ed25519 "$manifest" "$release_sig" "$key"

[ "$(sed -n '1p' "$manifest")" = 'codex-release-v4' ] ||
  fail 'fresh-install release manifest must be v4'
manifest_generation=$(awk -F '\t' '$1=="generation_id"{count++; value=$2} END {if(count==1) print value}' "$manifest")
[ "$manifest_generation" = "$generation" ] ||
  fail 'release manifest generation does not match the signed index'
file_count=$(awk -F '\t' '$1=="file_count"{count++; value=$2} END {if(count==1) print value}' "$manifest")
case $file_count in ''|*[!0-9]*) fail 'release manifest file count is invalid' ;; esac
[ "$file_count" -ge 6 ] && [ "$file_count" -le 7 ] ||
  fail 'release manifest inventory size is unsupported'

tab=$(printf '\t')
downloaded=0
total=0
while IFS="$tab" read -r kind rel digest mode extra; do
  [ "$kind" = file ] || continue
  [ -z "$extra" ] || fail 'release manifest file record is malformed'
  case $rel in
    generation.meta|core|runtime|codex-code-mode-host|manager|helpers/0|helpers/1|browser/open/curl|browser/manual/curl) ;;
    *) fail 'release manifest contains an unsupported bootstrap path' ;;
  esac
  [ "${#digest}" -eq 64 ] || fail 'release manifest digest is invalid'
  case $digest in *[!0-9a-f]*) fail 'release manifest digest is invalid' ;; esac
  case $rel in
    generation.meta) [ "$mode" = 0644 ] || fail 'generation descriptor mode is invalid' ;;
    *) [ "$mode" = 0755 ] || fail 'bootstrap executable mode is invalid' ;;
  esac
  target=$release_dir/$rel
  [ ! -e "$target" ] && [ ! -L "$target" ] ||
    fail 'release manifest contains a duplicate bootstrap path'
  parent=${target%/*}
  [ "$parent" = "$target" ] || mkdir -p "$parent"
  fetch_bounded "${release_base}$rel" "$target" 536870912
  chmod "$mode" "$target"
  size=$(wc -c < "$target" | tr -d '[:space:]')
  total=$((total + size))
  [ "$total" -le 1073741824 ] || fail 'release inventory exceeds its total byte bound'
  downloaded=$((downloaded + 1))
done < "$manifest"
[ "$downloaded" -eq "$file_count" ] ||
  fail 'release manifest inventory count does not match downloaded files'

for required in generation.meta core runtime codex-code-mode-host; do
  [ -f "$release_dir/$required" ] && [ ! -L "$release_dir/$required" ] ||
    fail 'release manifest is missing a required bootstrap file'
done
if [ -f "$release_dir/helpers/0" ] || [ -f "$release_dir/helpers/1" ]; then
  [ -f "$release_dir/helpers/0" ] && [ -f "$release_dir/helpers/1" ] ||
    fail 'R10 helper layout is incomplete'
  [ ! -e "$release_dir/browser/open/curl" ] && [ ! -e "$release_dir/browser/manual/curl" ] ||
    fail 'release manifest mixes helper layouts'
else
  [ -f "$release_dir/browser/open/curl" ] && [ -f "$release_dir/browser/manual/curl" ] ||
    fail 'canonical browser helper layout is incomplete'
fi

bundle=$work/local-installer
mkdir -p "$bundle/bootstrap"
fetch_bounded "$LOCAL_INSTALL_RAW_BASE/install.sh" "$bundle/install.sh" 16384
fetch_bounded "$LOCAL_INSTALL_RAW_BASE/bootstrap/codex-bootstrap"   "$bundle/bootstrap/codex-bootstrap" 131072
chmod 0755 "$bundle/install.sh" "$bundle/bootstrap/codex-bootstrap"

"$bundle/install.sh" "$release_dir/core" "$release_dir" "$key"
printf 'codex online install: installed signed stable generation %s\n' "$generation"
