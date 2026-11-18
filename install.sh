#!/bin/sh
set -eu
umask 077
LC_ALL=C
export LC_ALL

DEFAULT_PREFIX='/data/data/com.termux/files/usr'
DEFAULT_REPO='humtr/codex'
DEFAULT_RELEASE_TAG='wrapper'
ASSET_NAME='codex-wrapper-aarch64-linux-android.tar.gz'
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

Install the Termux codex wrapper into $PREFIX/bin/codex.

Options:
  --prefix <DIR>     Install root prefix (default: $PREFIX or /data/data/com.termux/files/usr)
  --binary <PATH>    Install a specific local codex wrapper binary
  --build            Build codex locally with `cargo build --release --locked -p codex`
  --release <TAG>    Download tarball from GitHub Releases tag (default: wrapper)
  --repo <OWNER/REPO>
                     GitHub repository for --release (default: humtr/codex)
  --version <X.Y.Z>  Install a specific upstream Codex version after wrapper install
  --skip-upstream    Install wrapper only without running `codex update --upstream`
  --recover          Reconcile a verified interrupted install (no updater replay)
  -h, --help         Show this help text

Source resolution when --binary, --build, and --release are omitted:
  1. ./bin/codex or ./codex beside install.sh (unpacked release bundle)
  2. ./target/release/codex in a built git checkout
  3. GitHub Release tag 'wrapper' from humtr/codex
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


# The installer has a separate filesystem transaction from the Wrapper's
# activation journal. Only a verified, quiescent transaction may be reclaimed.
install_sync() {
  sync "$1" || fail "cannot durably sync $1"
}

digest_or_none() {
  if [ -e "$1" ] || [ -L "$1" ]; then
    require_regular_file "$1" 'transaction asset'
    sha256_hex "$1"
  else
    printf 'none\n'
  fi
}

record_value() {
  awk -v key="$1" '
    index($0, key "=") == 1 { value=substr($0, length(key)+2); count++ }
    END { if (count != 1 || value == "") exit 1; print value }
  ' "$2"
}

proc_start_ticks() {
  [ -r "/proc/$1/stat" ] || return 1
  sed 's/^.*) //' "/proc/$1/stat" | awk 'NF >= 20 { print $20 }'
}

boot_identity() {
  [ -r /proc/sys/kernel/random/boot_id ] || return 1
  cat /proc/sys/kernel/random/boot_id
}

activation_state_file() {
  printf '%s\n' "$HOME/.local/share/codex/wrapper/activation-state"
}

activation_snapshot() {
  root="$HOME/.local/share/codex/wrapper"
  for name in activation-journal activation-journal.tmp activation-state.tmp; do
    if [ -e "$root/$name" ] || [ -L "$root/$name" ]; then
      fail "activation transaction is pending ($name); resolve it before installing"
    fi
  done
  digest_or_none "$(activation_state_file)"
}

valid_sha_or_none() {
  [ "$1" = none ] && return 0
  [ "${#1}" -eq 64 ] || return 1
  case $1 in *[!0-9a-f]*) return 1 ;; esac
}

write_install_phase() {
  phase=$1
  printf 'format=codex-install-v1\nphase=%s\nold_sha=%s\nnew_sha=%s\nstate_before=%s\nskip_upstream=%s\nrequested_version=%s\n' \
    "$phase" "$before_sha" "$candidate_sha" "$state_before_sha" "$skip_upstream" "${upstream_version:-none}" \
    > "$install_lock/transaction.next"
  install_sync "$install_lock/transaction.next"
  mv -f "$install_lock/transaction.next" "$install_lock/transaction"
  install_sync "$install_lock"
}

install_record_owner() {
  start=$(proc_start_ticks "$$") || fail 'cannot identify installer process start time'
  boot=$(boot_identity) || fail 'cannot identify kernel boot for install ownership'
  if [ -z "$start" ] || [ -z "$boot" ]; then
    fail 'incomplete installer process identity'
  fi
  printf 'pid=%s\nstart=%s\nboot=%s\n' "$$" "$start" "$boot" > "$install_lock/owner.next"
  install_sync "$install_lock/owner.next"
  mv -f "$install_lock/owner.next" "$install_lock/owner"
  install_sync "$install_lock"
}

# Only a fully qualified active generation can prove that an uncertain
# updater already committed. A process exit alone is not a commit receipt.
qualified_new_activation() {
  state=$(activation_state_file)
  [ -f "$state" ] && [ ! -L "$state" ] || return 1
  root="$HOME/.local/share/codex/wrapper"
  for name in activation-journal activation-journal.tmp activation-state.tmp; do
    [ ! -e "$root/$name" ] && [ ! -L "$root/$name" ] || return 1
  done
  [ "$(sed -n '1p' "$state")" = 'format=codex-activation-state' ] || return 1
  gen=$(record_value current "$state") || return 1
  case $gen in ''|.|..|*/*) return 1 ;; esac
  meta="$HOME/.codex/packages/standalone/releases/$gen/generation.meta"
  [ -f "$meta" ] && [ ! -L "$meta" ] || return 1
  if [ "${upstream_version:-none}" != none ]; then
    "$dest_bin" wrapper status --verify --wrapper-sha256 "$candidate_sha" --upstream-version "$upstream_version" --json >/dev/null 2>&1
  else
    "$dest_bin" wrapper status --verify --wrapper-sha256 "$candidate_sha" --json >/dev/null 2>&1
  fi
}

# Do not infer a dead owner solely from kill -0; same-boot PID reuse, unreadable
# /proc and a still-running updater are independent reasons to refuse.
owner_is_quiescent() {
  owner_pid=$(record_value pid "$install_lock/owner") || fail 'invalid install lock owner PID'
  owner_start=$(record_value start "$install_lock/owner") || fail 'invalid install lock start'
  owner_boot=$(record_value boot "$install_lock/owner") || fail 'invalid install lock boot'
  case $owner_pid in ''|*[!0-9]*) fail 'malformed owner PID' ;; esac
  case $owner_start in ''|*[!0-9]*) fail 'malformed owner start time' ;; esac
  now_boot=$(boot_identity) || fail 'cannot establish current kernel boot'
  if [ "$owner_boot" = "$now_boot" ]; then
    if [ -d "/proc/$owner_pid" ]; then
      now_start=$(proc_start_ticks "$owner_pid") ||
        fail "cannot read install lock owner $owner_pid; refusing recovery"
      if [ "$owner_start" = "$now_start" ]; then
        fail "install lock belongs to live process $owner_pid; refusing recovery"
      fi
      fail "install lock PID $owner_pid was reused; manual verification required"
    fi
    [ -d /proc/self ] || fail 'procfs unavailable; refusing lock reclamation'
  fi
}

updater_is_quiescent() {
  updater_file="$install_lock/updater"
  if [ ! -f "$updater_file" ] || [ -L "$updater_file" ]; then
    fail 'updater launch receipt missing; cannot safely settle this install'
  fi
  update_pid=$(record_value pid "$updater_file") || fail 'invalid updater PID receipt'
  update_start=$(record_value start "$updater_file") || fail 'invalid updater start receipt'
  update_group=$(record_value pgid "$updater_file") || fail 'invalid updater process-group receipt'
  case $update_pid:$update_start:$update_group in
    *[!0-9:]*|:*|*::*) fail 'malformed updater process identity' ;;
  esac
  if [ "$owner_boot" = "$now_boot" ]; then
    if [ -d "/proc/$update_pid" ]; then
      actual=$(proc_start_ticks "$update_pid") || fail 'updater PID is unreadable'
      if [ "$actual" = "$update_start" ]; then
        proc_state=$(sed 's/^.*) //' "/proc/$update_pid/stat" | awk '{print $1}') ||
          fail 'cannot inspect updater process state'
        case $proc_state in Z|X) ;; *) fail "updater process $update_pid is still alive; refusing recovery" ;; esac
      fi
      fail 'updater PID was reused; refusing automatic settlement'
    fi
    [ "$update_group" != 0 ] || fail 'updater group was not independently tracked'
    groups=$(ps -eo stat=,pgid= 2>/dev/null) ||
      fail 'cannot verify updater process-group quiescence'
    if printf '%s\n' "$groups" | awk -v pgid="$update_group" '
        $2 == pgid && substr($1,1,1) != "Z" && substr($1,1,1) != "X" { found=1 }
        END { exit !found }
      '; then
      fail "updater process group $update_group still has live members"
    fi
  fi
}

remove_owned_install_file() {
  if [ -e "$1" ] || [ -L "$1" ]; then
    require_regular_file "$1" 'install temporary/backup'
    rm -f "$1" || fail "cannot clean owned installer asset $1"
  fi
}

# Recovery is serialized separately so two stale-lock reclaimers cannot both
# modify the same executable. An unrecognized or unowned lock remains closed.
recover_install_lock() {
  if ! mkdir "$recovery_gate" 2>/dev/null; then
    fail "another install recovery is active or left an uncertain gate: $recovery_gate"
  fi
  recovery_gate_owned=1
  if [ ! -d "$install_lock" ] || [ -L "$install_lock" ]; then
    fail 'install lock is not a real directory'
  fi
  for name in owner transaction; do
    if [ ! -f "$install_lock/$name" ] || [ -L "$install_lock/$name" ]; then
      fail "install lock missing valid $name; manual inspection required"
    fi
  done
  owner_is_quiescent
  [ "$(record_value format "$install_lock/transaction")" = codex-install-v1 ] ||
    fail 'unknown install transaction format'
  recovered_phase=$(record_value phase "$install_lock/transaction")
  before_sha=$(record_value old_sha "$install_lock/transaction")
  candidate_sha=$(record_value new_sha "$install_lock/transaction")
  state_before_sha=$(record_value state_before "$install_lock/transaction")
  skip_upstream=$(record_value skip_upstream "$install_lock/transaction")
  upstream_version=$(record_value requested_version "$install_lock/transaction")
  if ! { valid_sha_or_none "$before_sha" && valid_sha_or_none "$candidate_sha" &&
         valid_sha_or_none "$state_before_sha" && [ "$candidate_sha" != none ]; }; then
    fail 'invalid digest in install transaction'
  fi
  case $skip_upstream in 0|1) ;; *) fail 'invalid install transaction mode' ;; esac
  case $upstream_version in
    *[!A-Za-z0-9._-]*|'') fail 'invalid requested version in transaction' ;;
  esac
  # Every recovery path checks the installed bytes before any destructive step.
  recovered_tmp="$dest_dir/.codex.tmp.$owner_pid"
  recovered_backup="$dest_dir/.codex.previous.$owner_pid"
  actual_sha=$(digest_or_none "$dest_bin")
  case $actual_sha in
    "$before_sha"|"$candidate_sha") ;;
    *) fail 'installed executable differs from both recorded SHA-256 values' ;;
  esac
  case $recovered_phase in
    preparing|ready|swapped)
      [ "$(activation_snapshot)" = "$state_before_sha" ] ||
        fail 'activation state changed before updater admission; manual reconciliation required'
      if [ "$recovered_phase" = preparing ] && [ "$actual_sha" != "$before_sha" ]; then
        fail 'executable changed during uncommitted preparation'
      fi
      if [ "$recovered_phase" = swapped ] && [ "$skip_upstream" -eq 1 ] &&
         [ "$actual_sha" = "$candidate_sha" ]; then
        note 'recovered committed --skip-upstream executable without rerunning updater'
      elif [ "$actual_sha" = "$candidate_sha" ] && [ "$before_sha" != "$candidate_sha" ]; then
        if [ "$before_sha" = none ]; then
          rm -f "$dest_bin"
        else
          [ "$(digest_or_none "$recovered_backup")" = "$before_sha" ] ||
            fail 'previous executable backup is missing or corrupt'
          mv -f "$recovered_backup" "$dest_bin"
        fi
        install_sync "$dest_dir"
        note 'restored verified prior executable after interrupted pre-updater install'
      fi
      ;;
    updating)
      updater_is_quiescent
      if ! { [ "$actual_sha" = "$candidate_sha" ] && qualified_new_activation; }; then
        fail 'updater outcome remains uncertain; preserving executable, backup and lock'
      fi
      note 'reconciled already activated generation; updater was not replayed'
      ;;
    updated)
      if ! { [ "$actual_sha" = "$candidate_sha" ] && qualified_new_activation; }; then
        fail 'completed updater receipt contradicts activation metadata'
      fi
      note 'recovered completed update without replay'
      ;;
    *)
      fail "unknown install phase $recovered_phase"
      ;;
  esac
  remove_owned_install_file "$recovered_tmp"
  remove_owned_install_file "$recovered_backup"
  for name in owner owner.next transaction transaction.next updater updater.next; do
    remove_owned_install_file "$install_lock/$name"
  done
  rmdir "$install_lock" || fail 'install lock contains unknown assets; refusing to remove'
  install_sync "$dest_dir"
  rmdir "$recovery_gate" || fail 'cannot release installer recovery gate'
  recovery_gate_owned=0
}

prefix=${PREFIX:-$DEFAULT_PREFIX}
repo=$DEFAULT_REPO
release_tag=''
source_mode=''
binary_path=''
upstream_version=''
skip_upstream=0
recover_only=0

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
    --recover)
      recover_only=1
      shift
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
work="$work_parent/.codex-wrapper-install.$$"
if ! mkdir "$work" 2>/dev/null; then
  fail "installer temporary directory already exists: $work"
fi
chmod 0700 "$work"

# The installed executable and activation pointer cannot be renamed atomically
# together. This transaction records both expected byte states before swap and
# never repeats an updater whose completion is unknown.
staged_tmp=''
backup_bin=''
install_lock=''
recovery_gate=''
lock_owned=0
recovery_gate_owned=0
swap_pending=0
install_committed=0
before_sha=none
candidate_sha=none
state_before_sha=none

finish_install() {
  result=$1
  trap - EXIT HUP INT TERM
  set +e

  if [ "$swap_pending" -eq 1 ] && [ "$install_committed" -eq 0 ]; then
    # A failed updater may already have committed the new generation. Never
    # restore an old Wrapper over changed authoritative activation state.
    installed_now=$(digest_or_none "$dest_bin")
    state_now=$(activation_snapshot)
    if [ "$state_now" = "$state_before_sha" ]; then
      if [ "$installed_now" = "$candidate_sha" ]; then
        if [ "$before_sha" = none ]; then
          rm -f "$dest_bin" && install_sync "$dest_dir"
          restored=$?
        elif [ "$(digest_or_none "$backup_bin")" = "$before_sha" ]; then
          mv -f "$backup_bin" "$dest_bin" && install_sync "$dest_dir"
          restored=$?
        else
          restored=1
        fi
      elif [ "$installed_now" = "$before_sha" ]; then
        restored=0
      else
        restored=1
      fi
      if [ "$restored" -eq 0 ]; then
        swap_pending=0
        note 'restored verified previous executable after failed install'
      fi
    fi
    if [ "$swap_pending" -eq 1 ]; then
      printf '%s\n' "codex install: unsafe to restore executable; preserving $install_lock and backup for explicit --recover" >&2
      result=1
    fi
  fi

  if [ "$swap_pending" -eq 0 ] || [ "$install_committed" -eq 1 ]; then
    if [ -n "$backup_bin" ]; then
      remove_owned_install_file "$backup_bin"
    fi
    if [ -n "$staged_tmp" ]; then
      remove_owned_install_file "$staged_tmp"
    fi
    if [ "$lock_owned" -eq 1 ]; then
      for name in owner owner.next transaction transaction.next updater updater.next; do
        remove_owned_install_file "$install_lock/$name"
      done
      if ! rmdir "$install_lock" || ! install_sync "$dest_dir"; then
        printf '%s\n' "codex install: install lock cleanup failed: $install_lock" >&2
        result=1
      fi
    fi
  fi
  if [ "$recovery_gate_owned" -eq 1 ]; then
    if ! rmdir "$recovery_gate"; then
      printf '%s\n' "codex install: recovery gate cleanup failed: $recovery_gate" >&2
      result=1
    fi
  fi
  rm -rf "$work"
  exit "$result"
}
trap 'finish_install "$?"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

dest_dir="$prefix/bin"
dest_bin="$dest_dir/codex"
install_lock="$dest_dir/.codex-install.lock"
recovery_gate="$dest_dir/.codex-install.recovering"
if [ "$recover_only" -eq 1 ]; then
  if [ -n "$source_mode" ] || [ -n "$upstream_version" ] || [ "$skip_upstream" -ne 0 ]; then
    fail '--recover may only be combined with --prefix'
  fi
  [ -d "$dest_dir" ] || fail "install directory does not exist: $dest_dir"
  if [ -e "$install_lock" ] || [ -L "$install_lock" ]; then
    recover_install_lock
  else
    [ ! -e "$recovery_gate" ] || fail 'unsettled recovery gate; manual review required'
    note 'no unfinished installer transaction'
  fi
  exit 0
fi

if [ -e "$recovery_gate" ] || [ -L "$recovery_gate" ]; then
  fail "unsettled installer recovery: $recovery_gate"
fi
# An existing lock may be safely reconciled only if its durable receipt
# proves it never launched an updater, or proves activation finished.
if [ -d "$install_lock" ] && [ ! -L "$install_lock" ]; then
  wanted_skip=$skip_upstream
  wanted_version=$upstream_version
  recover_install_lock
  skip_upstream=$wanted_skip
  upstream_version=$wanted_version
fi

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
    note 'building codex wrapper in release mode'
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

# --prefix must also govern the child Wrapper's actual runtime roots.
PREFIX=$prefix
export PREFIX
mkdir -p "$dest_dir"
if [ -e "$recovery_gate" ] || [ -L "$recovery_gate" ]; then
  fail "unsettled installer recovery: $recovery_gate"
fi
if ! mkdir "$install_lock" 2>/dev/null; then
  fail "installation already in progress or unowned lock: $install_lock (run --recover after inspection)"
fi
lock_owned=1
install_record_owner

if [ -L "$dest_bin" ] || { [ -e "$dest_bin" ] && [ ! -f "$dest_bin" ]; }; then
  fail "install destination must be absent or a regular non-symlink file: $dest_bin"
fi

before_sha=$(digest_or_none "$dest_bin")
candidate_sha=$(sha256_hex "$binary_path")
state_before_sha=$(activation_snapshot)
write_install_phase preparing

staged_tmp="$dest_dir/.codex.tmp.$$"
if [ -e "$staged_tmp" ] || [ -L "$staged_tmp" ]; then
  fail "staging collision: $staged_tmp"
fi
cp "$binary_path" "$staged_tmp"
chmod 0755 "$staged_tmp"
[ "$(sha256_hex "$staged_tmp")" = "$candidate_sha" ] ||
  fail 'staged executable checksum changed during copy'
install_sync "$staged_tmp"
install_sync "$dest_dir"

if [ -x /system/bin/linker64 ]; then
  "$staged_tmp" update --help >/dev/null 2>&1 ||
    fail 'staged codex wrapper failed execution check under /system/bin/linker64'
fi

if [ "$before_sha" != none ]; then
  backup_bin="$dest_dir/.codex.previous.$$"
  if [ -e "$backup_bin" ] || [ -L "$backup_bin" ]; then
    fail "backup collision: $backup_bin"
  fi
  cp -p "$dest_bin" "$backup_bin" || fail "cannot preserve previous codex binary in $dest_dir"
  [ "$(sha256_hex "$backup_bin")" = "$before_sha" ] ||
    fail 'previous executable backup failed integrity verification'
  install_sync "$backup_bin"
  install_sync "$dest_dir"
fi

[ "$(activation_snapshot)" = "$state_before_sha" ] ||
  fail 'activation changed while preparing installer transaction'
write_install_phase ready

# The backup and transaction are durable before the executable rename.
# If killed in this gap, recovery compares actual bytes to both digests.
swap_pending=1
mv -f "$staged_tmp" "$dest_bin"
staged_tmp=''
install_sync "$dest_dir"
write_install_phase swapped
note "installed codex wrapper to $dest_bin"

if [ "$skip_upstream" -eq 1 ]; then
  note "skipped upstream bootstrap (--skip-upstream); run '$dest_bin update --upstream' when ready"
  install_committed=1
  exit 0
fi

if [ -x /system/bin/linker64 ]; then
  if [ -n "$upstream_version" ]; then
    note "fetching upstream Codex v$upstream_version"
    needs_update=1
  elif qualified_new_activation; then
    note "qualified active generation already matches new Wrapper SHA-256"
    needs_update=0
  else
    note 'bootstrapping or requalifying upstream generation for the new Wrapper'
    needs_update=1
  fi
  if [ "$needs_update" -eq 1 ]; then
    [ "$(activation_snapshot)" = "$state_before_sha" ] ||
      fail 'activation changed before updater admission'
    write_install_phase updating
    update_group=0
    if command -v setsid >/dev/null 2>&1; then
      if [ -n "$upstream_version" ]; then
        setsid "$dest_bin" update --upstream --version "$upstream_version" &
      else
        setsid "$dest_bin" update --upstream &
      fi
      updater_pid=$!
      observed_group=0
      retries=0
      while [ "$retries" -lt 30 ]; do
        observed_group=$(sed 's/^.*) //' "/proc/$updater_pid/stat" 2>/dev/null | awk '{print $3}')
        [ "$observed_group" != "$updater_pid" ] || break
        retries=$((retries+1))
        sleep 0.01
      done
      if [ "$observed_group" = "$updater_pid" ]; then update_group=$updater_pid; fi
    else
      if [ -n "$upstream_version" ]; then
        "$dest_bin" update --upstream --version "$upstream_version" &
      else
        "$dest_bin" update --upstream &
      fi
      updater_pid=$!
    fi
    updater_start=$(proc_start_ticks "$updater_pid" || :)
    if [ -z "$updater_start" ]; then updater_start=0; fi
    printf 'pid=%s\nstart=%s\npgid=%s\n' "$updater_pid" "$updater_start" "$update_group" > "$install_lock/updater.next"
    install_sync "$install_lock/updater.next"
    mv -f "$install_lock/updater.next" "$install_lock/updater"
    install_sync "$install_lock"
    if wait "$updater_pid"; then
      write_install_phase updated
    else
      result=$?
      fail "upstream updater exited with status $result; activation must be reconciled"
    fi
  fi
  if ! qualified_new_activation; then
    fail 'updater returned but active generation is not qualified against this Wrapper SHA-256'
  fi
  note "installation complete; try '$dest_bin doctor' or '$dest_bin wrapper status'"
else
  note "non-Android host detected; skipping on-device upstream bootstrap"
fi
install_committed=1
