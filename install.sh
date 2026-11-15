#!/bin/sh
set -eu
umask 077
LC_ALL=C
export LC_ALL

DEFAULT_PREFIX='/data/data/com.termux/files/usr'
DEFAULT_REPO='humtr/codex'
DEFAULT_RELEASE_TAG='minimal-wrapper'
ASSET_NAME='codex-minimal-wrapper-aarch64-linux-android.tar.gz'
MAX_TARBALL_BYTES=20971520
MAX_SHA256_BYTES=4096

fail() {
  printf '%s\n' "codex install: $*" >&2
  exit 1
}

note() {
  printf '%s\n' "codex install: $*"
}

usage() {
  cat <<'EOF'
Usage: install.sh [OPTIONS]

Install the Stage 1 Termux minimal codex wrapper into $PREFIX/bin/codex.

Options:
  --prefix <DIR>     Install root prefix (default: $PREFIX or /data/data/com.termux/files/usr)
  --binary <PATH>    Install a specific local codex wrapper binary
  --build            Build codex locally with `cargo build --release --locked -p codex`
  --release <TAG>    Download tarball from GitHub Releases tag (default: minimal-wrapper)
  --repo <OWNER/REPO>
                     GitHub repository for --release (default: humtr/codex)
  --version <X.Y.Z>  Install a specific upstream Codex version after wrapper install
  --skip-upstream    Install wrapper only without running `codex update --upstream`
  -h, --help         Show this help text

Source resolution when --binary, --build, and --release are omitted:
  1. ./bin/codex or ./codex beside install.sh (unpacked release bundle)
  2. ./target/release/codex in a built git checkout
  3. GitHub Release tag 'minimal-wrapper' from humtr/codex
EOF
}

require_value() {
  flag=$1
  if [ "$#" -lt 2 ] || [ -z "$2" ]; then
    fail "$flag requires a non-empty value"
  fi
}

require_regular_file() {
  path=$1
  label=$2
  if [ ! -f "$path" ] || [ -L "$path" ]; then
    fail "$label must be a regular non-symlink file: $path"
  fi
}

validate_simple_token() {
  label=$1
  value=$2
  [ -n "$value" ] || fail "$label must not be empty"
  case $value in
    *[!A-Za-z0-9._/-]* | /* | */ | *//* | *..* )
      fail "$label contains unsupported characters: $value"
      ;;
  esac
}

find_tool() {
  tool=$1
  if command -v "$tool" >/dev/null 2>&1; then
    command -v "$tool"
    return 0
  fi
  if [ -n "${prefix:-}" ] && [ -x "$prefix/bin/$tool" ]; then
    printf '%s\n' "$prefix/bin/$tool"
    return 0
  fi
  return 1
}

sha256_hex() {
  file=$1
  if command -v sha256sum >/dev/null 2>&1; then
    digest=$(sha256sum "$file" | awk '{print $1}')
  else
    openssl_bin=$(find_tool openssl) || fail 'neither sha256sum nor openssl is available'
    digest=$("$openssl_bin" dgst -sha256 -r "$file" 2>/dev/null | awk '{print $1}') ||
      fail "OpenSSL SHA-256 failed for $file"
  fi
  [ "${#digest}" -eq 64 ] || fail "invalid SHA-256 output for $file"
  case $digest in *[!0-9a-f]*) fail "malformed SHA-256 digest for $file" ;; esac
  printf '%s\n' "$digest"
}

verify_sha256_file() {
  target_file=$1
  sum_file=$2
  expected_entry=${3:-}
  require_regular_file "$sum_file" 'checksum file'
  if [ -n "$expected_entry" ]; then
    expected=$(awk -v name="$expected_entry" '
      $2 == name || $2 == "./" name || $2 == "*" name { print $1; count++ }
      END { if (count != 1) exit 1 }
    ' "$sum_file" 2>/dev/null) || fail "missing unique SHA-256 entry for $expected_entry in $sum_file"
  else
    expected=$(awk 'NR == 1 { print $1; count++ } END { if (count != 1) exit 1 }' "$sum_file" 2>/dev/null) ||
      fail "malformed checksum file: $sum_file"
  fi
  [ "${#expected}" -eq 64 ] || fail "invalid expected SHA-256 length in $sum_file"
  case $expected in *[!0-9a-f]*) fail "malformed expected SHA-256 hex in $sum_file" ;; esac
  actual=$(sha256_hex "$target_file")
  [ "$actual" = "$expected" ] || fail "SHA-256 mismatch for $target_file (expected $expected, got $actual)"
}

fetch_https() {
  url=$1
  out=$2
  max_bytes=$3
  curl_bin=$(find_tool curl) || fail 'curl is required to download release assets'
  "$curl_bin" --fail --silent --show-error --location \
    --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --connect-timeout 15 --max-time 300 --max-filesize "$max_bytes" \
    --output "$out" "$url" || fail "HTTPS download failed: $url"
  require_regular_file "$out" 'downloaded asset'
  size=$(wc -c < "$out" | tr -d '[:space:]')
  case $size in ''|*[!0-9]*) fail "invalid downloaded byte size for $out" ;; esac
  if [ "$size" -le 0 ] || [ "$size" -gt "$max_bytes" ]; then
    fail "downloaded file out of bounds: $out"
  fi
}

prefix=${PREFIX:-$DEFAULT_PREFIX}
repo=$DEFAULT_REPO
release_tag=''
source_mode=''
binary_path=''
upstream_version=''
skip_upstream=0

while [ "$#" -gt 0 ]; do
  case $1 in
    --prefix)
      require_value "$1" "${2:-}"
      prefix=$2
      shift 2
      ;;
    --binary)
      require_value "$1" "${2:-}"
      [ -z "$source_mode" ] || fail 'specify at most one of --binary, --build, or --release'
      source_mode='binary'
      binary_path=$2
      shift 2
      ;;
    --build)
      [ -z "$source_mode" ] || fail 'specify at most one of --binary, --build, or --release'
      source_mode='build'
      shift
      ;;
    --release)
      require_value "$1" "${2:-}"
      [ -z "$source_mode" ] || fail 'specify at most one of --binary, --build, or --release'
      source_mode='release'
      release_tag=$2
      shift 2
      ;;
    --repo)
      require_value "$1" "${2:-}"
      repo=$2
      shift 2
      ;;
    --version)
      require_value "$1" "${2:-}"
      upstream_version=$2
      shift 2
      ;;
    --skip-upstream)
      skip_upstream=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown option: $1 (try --help)"
      ;;
  esac
done

if [ -z "${HOME:-}" ] || [ ! -d "$HOME" ]; then
  fail 'HOME must point to an existing directory'
fi
validate_simple_token 'repository' "$repo"
case $repo in
  */*) ;;
  *) fail "repository must look like owner/repo: $repo" ;;
esac

if [ -n "$upstream_version" ]; then
  validate_simple_token 'upstream version' "$upstream_version"
fi

script_dir=$(
  CDPATH='' cd -- "$(dirname -- "$0")" 2>/dev/null &&
    pwd -P
) || fail 'cannot resolve script directory'

work_parent=${TMPDIR:-"$prefix/tmp"}
if [ ! -d "$work_parent" ]; then
  if [ -d "$HOME/.cache" ]; then
    work_parent="$HOME/.cache"
  elif [ -d /tmp ]; then
    work_parent=/tmp
  else
    work_parent="$HOME"
  fi
fi
work="$work_parent/.codex-minimal-install.$$"
mkdir -p "$work"
chmod 0700 "$work"

staged_tmp=''
cleanup() {
  if [ -n "$staged_tmp" ] && [ -e "$staged_tmp" ]; then
    rm -f "$staged_tmp"
  fi
  rm -rf "$work"
}
trap cleanup EXIT HUP INT TERM

if [ -z "$source_mode" ]; then
  if [ -f "$script_dir/bin/codex" ] && [ ! -L "$script_dir/bin/codex" ] && [ -f "$script_dir/SHA256SUMS" ]; then
    source_mode='bundle'
    binary_path="$script_dir/bin/codex"
  elif [ -f "$script_dir/codex" ] && [ ! -L "$script_dir/codex" ] && [ -f "$script_dir/SHA256SUMS" ]; then
    source_mode='bundle_root'
    binary_path="$script_dir/codex"
  elif [ -f "$script_dir/Cargo.toml" ] && [ -f "$script_dir/target/release/codex" ] && [ ! -L "$script_dir/target/release/codex" ]; then
    source_mode='binary'
    binary_path="$script_dir/target/release/codex"
  else
    source_mode='release'
    release_tag=$DEFAULT_RELEASE_TAG
  fi
fi

case $source_mode in
  bundle)
    if [ -f "$script_dir/SHA256SUMS" ]; then
      verify_sha256_file "$binary_path" "$script_dir/SHA256SUMS" 'bin/codex'
      note 'verified bundled bin/codex against SHA256SUMS'
    fi
    ;;
  bundle_root)
    if [ -f "$script_dir/SHA256SUMS" ]; then
      verify_sha256_file "$binary_path" "$script_dir/SHA256SUMS" 'codex'
      note 'verified bundled codex against SHA256SUMS'
    fi
    ;;
  binary)
    require_regular_file "$binary_path" '--binary'
    if [ -f "$binary_path.sha256" ]; then
      verify_sha256_file "$binary_path" "$binary_path.sha256"
      note "verified $(basename -- "$binary_path") against .sha256"
    fi
    ;;
  build)
    [ -f "$script_dir/Cargo.toml" ] || fail '--build must be run from the codex repository root'
    cargo_bin=$(find_tool cargo) || fail 'cargo is required for --build'
    note 'building minimal codex wrapper in release mode'
    "$cargo_bin" build --release --locked --manifest-path "$script_dir/Cargo.toml" -p codex
    binary_path="$script_dir/target/release/codex"
    require_regular_file "$binary_path" 'cargo build output'
    ;;
  release)
    validate_simple_token 'release tag' "$release_tag"
    tar_bin=$(find_tool tar) || fail 'tar is required to unpack release assets'
    find_tool gzip >/dev/null 2>&1 || fail 'gzip is required to unpack release assets'
    base_url="https://github.com/$repo/releases/download/$release_tag"
    tarball="$work/$ASSET_NAME"
    tarball_sha="$work/$ASSET_NAME.sha256"
    note "downloading $ASSET_NAME from tag $release_tag ($repo)"
    fetch_https "$base_url/$ASSET_NAME" "$tarball" "$MAX_TARBALL_BYTES"
    fetch_https "$base_url/$ASSET_NAME.sha256" "$tarball_sha" "$MAX_SHA256_BYTES"
    verify_sha256_file "$tarball" "$tarball_sha"
    note 'verified release archive SHA-256'

    if "$tar_bin" -tzf "$tarball" | grep -E '(^/|(^|/)\.\.(/|$))' >/dev/null 2>&1; then
      fail 'release tarball contains unsafe path entries'
    fi
    extract_dir="$work/extract"
    mkdir -p "$extract_dir"
    "$tar_bin" -xzf "$tarball" -C "$extract_dir"
    binary_path="$extract_dir/bin/codex"
    require_regular_file "$binary_path" 'release archive bin/codex'
    if [ -f "$extract_dir/SHA256SUMS" ]; then
      verify_sha256_file "$binary_path" "$extract_dir/SHA256SUMS" 'bin/codex'
      note 'verified extracted bin/codex against SHA256SUMS'
    fi
    ;;
  *)
    fail "internal error: unexpected source mode $source_mode"
    ;;
esac

if [ "$skip_upstream" -eq 0 ]; then
  for req in curl openssl gzip; do
    find_tool "$req" >/dev/null 2>&1 || fail "runtime dependency missing from PATH/PREFIX: $req"
  done
fi

dest_dir="$prefix/bin"
dest_bin="$dest_dir/codex"
mkdir -p "$dest_dir"
if [ -d "$dest_bin" ]; then
  fail "install destination is a directory: $dest_bin"
fi

staged_tmp="$dest_dir/.codex.tmp.$$"
cp "$binary_path" "$staged_tmp"
chmod 0755 "$staged_tmp"

if [ -x /system/bin/linker64 ]; then
  "$staged_tmp" update --help >/dev/null 2>&1 ||
    fail 'staged codex wrapper failed execution check under /system/bin/linker64'
fi

mv -f "$staged_tmp" "$dest_bin"
staged_tmp=''
note "installed Stage 1 wrapper to $dest_bin"

if [ "$skip_upstream" -eq 1 ]; then
  note "skipped upstream bootstrap (--skip-upstream); run '$dest_bin update --upstream' when ready"
  exit 0
fi

if [ -x /system/bin/linker64 ]; then
  if [ -n "$upstream_version" ]; then
    note "fetching upstream Codex v$upstream_version"
    "$dest_bin" update --upstream --version "$upstream_version"
  elif "$dest_bin" wrapper status >/dev/null 2>&1; then
    note "upstream generation already active; run '$dest_bin update --upstream' anytime to check updates"
  else
    note 'bootstrapping initial upstream Codex generation'
    "$dest_bin" update --upstream
  fi
  note "installation complete; try '$dest_bin doctor' or '$dest_bin wrapper status'"
else
  note "non-Android host detected; skipping on-device upstream bootstrap"
fi
