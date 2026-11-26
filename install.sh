#!/bin/sh
set -eu
umask 077
LC_ALL=C
export LC_ALL

DEFAULT_PREFIX='/data/data/com.termux/files/usr'
DEFAULT_REPO='humtr/codex'
DEFAULT_RELEASE_TAG='wrapper'
ASSET_NAME='codex-wrapper-aarch64-linux-android.tar.gz'
PINNED_RELEASE_PUBKEY_SHA256='0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca'
MAX_TARBALL_BYTES=20971520
MAX_COMPONENT_TARBALL_BYTES=67108864
MAX_SHA256_BYTES=4096
MAX_SUMS_BYTES=65536
MAX_PUBKEY_BYTES=4096
ED25519_SIG_BYTES=64
ALL_COMPONENTS='codex-ext tunnel-client termux-shell-mcp termux-codex-mcp chatgpt-provider termux-chatgpt-mcp termux-orch'

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

Install the Termux codex wrapper into $PREFIX/bin/codex and optionally
install standalone companion components, services, and account setup.

Options:
  --prefix <DIR>     Install root prefix (default: $PREFIX or /data/data/com.termux/files/usr)
  --binary <PATH>    Install a specific local codex wrapper binary
  --build            Build codex locally with `cargo build --release --locked -p codex`
  --release <TAG>    Download tarball from GitHub Releases tag (default: wrapper)
  --repo <OWNER/REPO>
                     GitHub repository for --release (default: humtr/codex)
  --verify-key <PEM> Ed25519 public key PEM used to verify release SHA256SUMS.sig
  --key-sha256 <HEX> Pin expected 64-hex SHA-256 digest of the verifier public key
  --require-signed   Fail closed if no verifier key or SHA256SUMS.sig is available
  --version <X.Y.Z>  Install a specific upstream Codex version after wrapper install
  --skip-upstream    Install wrapper only without running `codex update --upstream`
  --all              Install all signed companion components & runit services too
  --wrapper-only     Install only the core Wrapper (+ upstream Codex unless --skip-upstream)
  --components <CSV> Install selected signed components (comma-separated list)
  --no-services      Stage companion binaries/generations without enrolling runit services
  --setup            Run interactive account, profile, ChatGPT & tunnel setup wizard
  --setup-only       Skip binary installation and run only the setup wizard
  --no-setup         Skip interactive setup prompts after installation
  --login-method <device|browser|api-key|skip>
                     Select Codex login method during setup (device = --device-auth)
  --recover          Reconcile a verified interrupted install (no updater replay)
  -h, --help         Show this help text

Source resolution when --binary, --build, and --release are omitted:
  1. ./bin/codex or ./codex beside install.sh (unpacked release bundle)
  2. ./target/release/codex in a built git checkout
  3. GitHub Release tag 'wrapper' from humtr/codex (default for stdin; on Android
     also installs all signed companion components and launches guided setup when TTY)
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
      NF == 2 && !/^#/ && ($2 == name || $2 == "./" name || $2 == "*" name) { print $1; count++ }
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

fetch_optional_https() {
  url=$1
  out=$2
  max_bytes=$3
  curl_bin=$(find_tool curl) || fail 'curl is required to download release assets'
  if ! "$curl_bin" --fail --silent --location \
    --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --connect-timeout 15 --max-time 60 --max-filesize "$max_bytes" \
    --output "$out" "$url" 2>/dev/null; then
    rm -f "$out"
    return 1
  fi
  require_regular_file "$out" 'downloaded optional asset'
  size=$(wc -c < "$out" | tr -d '[:space:]')
  case $size in ''|*[!0-9]*) rm -f "$out"; return 1 ;; esac
  if [ "$size" -le 0 ] || [ "$size" -gt "$max_bytes" ]; then
    rm -f "$out"
    fail "downloaded optional file out of bounds: $out"
  fi
  return 0
}

write_embedded_release_pubkey() {
  cat > "$1" <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAgx0w1LvXUvnWBt+0eGnCrd2XrkfFZZmFuBdOR8H/2kM=
-----END PUBLIC KEY-----
EOF
  chmod 0600 "$1"
}

verify_ed25519_sig() {
  v_openssl=$1
  v_pubkey=$2
  v_data=$3
  v_sig=$4
  require_regular_file "$v_pubkey" 'Ed25519 public key'
  require_regular_file "$v_data" 'signed payload'
  require_regular_file "$v_sig" 'Ed25519 signature'
  v_sig_size=$(wc -c < "$v_sig" | tr -d '[:space:]')
  [ "$v_sig_size" = "$ED25519_SIG_BYTES" ] ||
    fail "Ed25519 signature must be $ED25519_SIG_BYTES bytes (got $v_sig_size)"
  "$v_openssl" pkeyutl -verify -pubin -inkey "$v_pubkey" -rawin \
    -in "$v_data" -sigfile "$v_sig" >/dev/null 2>&1 ||
    fail "Ed25519 signature verification failed for $(basename -- "$v_data")"
}

verify_component_sums_headers() {
  sums_file=$1
  expected_component=${2:-wrapper}
  h_fmt=$(sed -n '1p' "$sums_file")
  h_cmp=$(sed -n '2p' "$sums_file")
  h_seq=$(sed -n '3p' "$sums_file")
  h_cmt=$(sed -n '4p' "$sums_file")
  [ "$h_fmt" = '# format=codex-release-sums-v1' ] ||
    fail "unexpected SHA256SUMS format header: $h_fmt"
  [ "$h_cmp" = "# component=$expected_component" ] ||
    fail "unexpected SHA256SUMS component header: $h_cmp (expected $expected_component)"
  case $h_seq in
    '# sequence='[1-9]*)
      seq_val=${h_seq#'# sequence='}
      validate_release_sequence "$seq_val" 'SHA256SUMS sequence'
      ;;
    *) fail "missing or invalid SHA256SUMS sequence header: $h_seq" ;;
  esac
  case $h_cmt in
    '# commit='*)
      cmt_val=${h_cmt#'# commit='}
      [ "${#cmt_val}" -eq 40 ] || fail "malformed SHA256SUMS commit header"
      case $cmt_val in *[!0-9a-f]*) fail "malformed SHA256SUMS commit hex" ;; esac
      ;;
    *) fail "missing SHA256SUMS commit header" ;;
  esac
  printf '%s\n' "$seq_val"
}

verify_signed_sums_headers() {
  verify_component_sums_headers "$1" 'wrapper'
}

parse_signed_sums_commit() {
  h_cmt=$(sed -n '4p' "$1")
  cmt_val=${h_cmt#'# commit='}
  printf '%s\n' "$cmt_val"
}

verify_bundle_checksums() {
  b_dir=$1
  b_sums=$2
  b_entries=$(awk 'NF == 2 && !/^#/ { print $2 }' "$b_sums") ||
    fail "cannot parse bundle SHA256SUMS: $b_sums"
  [ -n "$b_entries" ] || fail "empty bundle SHA256SUMS: $b_sums"
  for b_entry in $b_entries; do
    b_clean=${b_entry#./}
    b_clean=${b_clean#\*}
    require_regular_file "$b_dir/$b_clean" "extracted asset $b_clean"
    verify_sha256_file "$b_dir/$b_clean" "$b_sums" "$b_clean"
  done
}

validate_release_sequence() {
  v_seq=$1
  v_label=$2
  case $v_seq in ''|*[!0-9]*) fail "invalid $v_label" ;; esac
  case $v_seq in 0|[1-9]*) ;; *) fail "non-canonical $v_label" ;; esac
  [ "${#v_seq}" -le 10 ] || fail "$v_label exceeds supported integer range"
  [ "${#v_seq}" -lt 10 ] || [ "$v_seq" -le 2147483647 ] ||
    fail "$v_label exceeds supported integer range"
}

read_recorded_release_sequence() {
  seq_file=$1
  if [ ! -e "$seq_file" ] && [ ! -L "$seq_file" ]; then
    printf '0\n'
    return 0
  fi
  require_regular_file "$seq_file" 'release sequence file'
  rec=$(tr -d '\r\n' < "$seq_file")
  validate_release_sequence "$rec" "recorded release sequence $seq_file"
  printf '%s\n' "$rec"
}

# Preserve the verifier that admitted existing generations before replacing it.
# Retired keys are local verification evidence, not download trust roots.
retain_release_key() {
  retired_dir="$key_dir/retired-keys"
  mkdir -p "$retired_dir"
  [ ! -L "$retired_dir" ] && [ -d "$retired_dir" ] ||
    fail 'retired release key directory is unsafe'
  chmod 0700 "$retired_dir"
  retired_key="$retired_dir/$(sha256_hex "$key_dest").pem"
  if [ -e "$retired_key" ] || [ -L "$retired_key" ]; then
    require_regular_file "$retired_key" 'retired release public key'
    cmp -s "$key_dest" "$retired_key" || fail 'retired release key digest conflict'
  else
    retired_tmp="$retired_dir/.key.tmp.$$"
    (umask 077; set -C; cat "$key_dest" > "$retired_tmp")
    install_sync "$retired_tmp"
    mv -f "$retired_tmp" "$retired_key"
  fi
  install_sync "$retired_dir"
  install_sync "$key_dir"
}

persist_signed_release_state() {
  state_dir="$HOME/.local/share/codex/wrapper"
  key_dir="$HOME/.local/lib/codex/wrapper"
  write_install_phase committing
  signed_commit_started=1
  mkdir -p "$state_dir" "$key_dir"
  if [ -L "$state_dir" ] || [ ! -d "$state_dir" ] || [ -L "$key_dir" ] || [ ! -d "$key_dir" ]; then
    fail 'signed release state directories must be real non-symlink directories'
  fi
  chmod 0700 "$state_dir" "$key_dir"
  seq_dest="$state_dir/release-sequence"
  if [ -L "$seq_dest" ] || { [ -e "$seq_dest" ] && [ ! -f "$seq_dest" ]; }; then
    fail "release sequence destination must be absent or a regular non-symlink file: $seq_dest"
  fi
  key_dest="$key_dir/release-public-key.pem"
  if [ -L "$key_dest" ] || { [ -e "$key_dest" ] && [ ! -f "$key_dest" ]; }; then
    fail "release public key destination must be absent or a regular non-symlink file: $key_dest"
  fi
  if [ -f "$key_dest" ] && ! cmp -s "$key_dest" "$verified_active_pubkey"; then
    retain_release_key
  fi
  seq_tmp="$state_dir/.release-sequence.tmp.$$"
  printf '%s\n' "$verified_release_sequence" > "$seq_tmp"
  chmod 0600 "$seq_tmp"
  install_sync "$seq_tmp"
  mv -f "$seq_tmp" "$seq_dest"
  install_sync "$state_dir"
  key_tmp="$key_dir/.release-public-key.pem.tmp.$$"
  cp "$verified_active_pubkey" "$key_tmp"
  chmod 0600 "$key_tmp"
  install_sync "$key_tmp"
  mv -f "$key_tmp" "$key_dest"
  install_sync "$key_dir"
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
  signed_key_sha=none
  if [ -n "$verified_active_pubkey" ]; then signed_key_sha=$(sha256_hex "$verified_active_pubkey"); fi
  printf 'format=codex-install-v1\nphase=%s\nold_sha=%s\nnew_sha=%s\nstate_before=%s\nskip_upstream=%s\nrequested_version=%s\nsigned_sequence=%s\nsigned_key_sha=%s\n' \
    "$phase" "$before_sha" "$candidate_sha" "$state_before_sha" "$skip_upstream" "${upstream_version:-none}" "${verified_release_sequence:-none}" "$signed_key_sha" \
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

qualified_generation_status() {
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
  if [ -n "$upstream_version" ]; then
    set -- "$@" --upstream-version "$upstream_version"
  fi
  "$dest_bin" wrapper status --verify "$@" --json >/dev/null 2>&1
}

qualified_active_generation() {
  qualified_generation_status
}

# Recovery uses the candidate binding as evidence of the updater's activation.
qualified_new_activation() {
  qualified_generation_status --wrapper-sha256 "$candidate_sha"
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
      else
        fail 'updater PID was reused; refusing automatic settlement'
      fi
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


verify_recovered_signed_state() {
  [ "$signed_sequence" = none ] && return 0
  validate_release_sequence "$signed_sequence" 'journal signed sequence'
  [ "${#signed_key_sha}" -eq 64 ] || fail 'invalid signed key digest in install journal'
  case $signed_key_sha in *[!0-9a-f]*) fail 'invalid signed key digest in install journal' ;; esac
  [ "$(read_recorded_release_sequence "$HOME/.local/share/codex/wrapper/release-sequence")" = "$signed_sequence" ] ||
    fail 'signed release sequence was not durably committed; preserving recovery journal'
  if ! { [ -f "$HOME/.local/lib/codex/wrapper/release-public-key.pem" ] &&
         [ "$(sha256_hex "$HOME/.local/lib/codex/wrapper/release-public-key.pem")" = "$signed_key_sha" ]; }; then
    fail 'signed release key was not durably committed; preserving recovery journal'
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
  signed_sequence=$(record_value signed_sequence "$install_lock/transaction" 2>/dev/null || printf 'none')
  signed_key_sha=$(record_value signed_key_sha "$install_lock/transaction" 2>/dev/null || printf 'none')
  if ! { valid_sha_or_none "$before_sha" && valid_sha_or_none "$candidate_sha" &&
         valid_sha_or_none "$state_before_sha" && [ "$candidate_sha" != none ]; }; then
    fail 'invalid digest in install transaction'
  fi
  case $skip_upstream in 0|1) ;; *) fail 'invalid install transaction mode' ;; esac
  case $upstream_version in
    none) upstream_version='' ;;
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
        verify_recovered_signed_state
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
      verify_recovered_signed_state
      note 'reconciled already activated generation; updater was not replayed'
      ;;
    committing|committed)
      [ "$actual_sha" = "$candidate_sha" ] || fail 'committed Wrapper executable mismatch'
      verify_recovered_signed_state
      note 'recovered committed Wrapper without replaying components or updater'
      ;;
    updated)
      if ! { [ "$actual_sha" = "$candidate_sha" ] && qualified_new_activation; }; then
        fail 'completed updater receipt contradicts activation metadata'
      fi
      verify_recovered_signed_state
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
verify_key=''
key_sha256=''
require_signed=0
verified_release_sequence=''
verified_active_pubkey=''
upstream_version=''
skip_upstream=0
recover_only=0
all_components=0
wrapper_only=0
selected_components=''
no_services=0
setup_mode='auto'
setup_only=0
login_method=''

validate_component_list() {
  raw_csv=$1
  parsed=''
  old_ifs=$IFS
  IFS=','
  for item in $raw_csv; do
    [ -n "$item" ] || fail '--components must not contain empty entries'
    case $item in
      *[!A-Za-z0-9-]*) fail "unknown component in --components: $item (supported: $ALL_COMPONENTS)" ;;
    esac
    case " $ALL_COMPONENTS " in
      *" $item "*) ;;
      *) fail "unknown component in --components: $item (supported: $ALL_COMPONENTS)" ;;
    esac
    case " $parsed " in
      *" $item "*) ;;
      *) parsed="${parsed:+$parsed }$item" ;;
    esac
  done
  IFS=$old_ifs
  [ -n "$parsed" ] || fail '--components requires at least one component name'
  printf '%s\n' "$parsed"
}

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
    --verify-key)
      require_value "$1" "${2:-}"
      verify_key=$2
      shift 2
      ;;
    --key-sha256)
      require_value "$1" "${2:-}"
      [ "${#2}" -eq 64 ] || fail '--key-sha256 must be 64 lowercase hex characters'
      case $2 in *[!0-9a-f]*) fail '--key-sha256 must be 64 lowercase hex characters' ;; esac
      key_sha256=$2
      shift 2
      ;;
    --require-signed)
      require_signed=1
      shift
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
    --all)
      all_components=1
      shift
      ;;
    --wrapper-only)
      wrapper_only=1
      shift
      ;;
    --components)
      require_value "$1" "${2:-}"
      selected_components=$(validate_component_list "$2")
      shift 2
      ;;
    --no-services)
      no_services=1
      shift
      ;;
    --setup)
      setup_mode='always'
      shift
      ;;
    --setup-only)
      setup_only=1
      setup_mode='always'
      shift
      ;;
    --no-setup)
      setup_mode='never'
      shift
      ;;
    --login-method)
      require_value "$1" "${2:-}"
      case $2 in
        device|browser|api-key|skip) login_method=$2 ;;
        *) fail "--login-method must be one of: device, browser, api-key, skip (got $2)" ;;
      esac
      setup_mode='always'
      shift 2
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

if [ "$wrapper_only" -eq 1 ] && { [ "$all_components" -eq 1 ] || [ -n "$selected_components" ]; }; then
  fail '--wrapper-only cannot be combined with --all or --components'
fi
if [ "$setup_only" -eq 1 ] && { [ -n "$source_mode" ] || [ "$recover_only" -eq 1 ] || [ "$all_components" -eq 1 ] || [ -n "$selected_components" ]; }; then
  fail '--setup-only cannot be combined with install or recovery flags'
fi

if [ -z "${HOME:-}" ] || [ ! -d "$HOME" ]; then
  fail 'HOME must point to an existing directory'
fi
validate_simple_token 'repository' "$repo"
case $repo in
  */*) ;;
  *) fail "repository must look like owner/repo: $repo" ;;
esac

if [ -n "$upstream_version" ]; then
  case $upstream_version in
    *[!A-Za-z0-9._-]*) fail "upstream version contains unsupported characters: $upstream_version" ;;
  esac
fi

script_dir=''
if [ -f "$0" ]; then
  script_dir=$(
    CDPATH='' cd -- "$(dirname -- "$0")" 2>/dev/null &&
      pwd -P
  ) || fail 'cannot resolve script directory'
fi

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
signed_commit_started=0
before_sha=none
candidate_sha=none
state_before_sha=none

finish_install() {
  result=$1
  trap - EXIT HUP INT TERM
  set +e

  if [ "$swap_pending" -eq 1 ] && [ "$install_committed" -eq 0 ] && [ "$signed_commit_started" -eq 0 ]; then
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

  if [ "$signed_commit_started" -eq 1 ] && [ "$install_committed" -eq 0 ]; then
    printf '%s\n' "codex install: signed commit incomplete; preserving $install_lock for explicit --recover" >&2
    result=1
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

commit_wrapper_transaction() {
  # Retain the installer lock for component serialization, but never undo
  # the qualified Wrapper if independent component installation fails.
  write_install_phase committed
  install_committed=1
  swap_pending=0
}

# The child Wrapper must use the installer's runtime roots, including in recovery.
PREFIX=$prefix
export PREFIX
if [ -n "${TMPDIR:-}" ]; then
  export TMPDIR
fi

dest_dir="$prefix/bin"
dest_bin="$dest_dir/codex"
install_lock="$dest_dir/.codex-install.lock"
recovery_gate="$dest_dir/.codex-install.recovering"
if [ "$recover_only" -eq 1 ]; then
  if [ -n "$source_mode" ] || [ -n "$upstream_version" ] || [ "$skip_upstream" -ne 0 ] ||
     [ -n "$verify_key" ] || [ -n "$key_sha256" ] || [ "$require_signed" -ne 0 ] ||
     [ "$all_components" -ne 0 ] || [ "$wrapper_only" -ne 0 ] || [ -n "$selected_components" ] ||
     [ "$no_services" -ne 0 ] || [ "$setup_mode" != 'auto' ] || [ "$setup_only" -ne 0 ] ||
     [ -n "$login_method" ]; then
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

if [ "$require_signed" -eq 1 ] && [ "$source_mode" != release ]; then
  fail '--require-signed is supported only with --release; local bundle/binary/build sources have no signature proof'
fi

component_verify_key_path=''

ensure_component_verify_key() {
  if [ -n "$component_verify_key_path" ]; then
    return 0
  fi
  c_key=''
  if [ -n "$verified_active_pubkey" ]; then
    c_key=$verified_active_pubkey
  elif [ -n "$verify_key" ]; then
    c_key=$verify_key
  else
    c_default="$HOME/.local/lib/codex/wrapper/release-public-key.pem"
    if [ -e "$c_default" ] || [ -L "$c_default" ]; then
      c_key=$c_default
    elif [ -n "$PINNED_RELEASE_PUBKEY_SHA256" ]; then
      c_key="$work/embedded-component-public-key.pem"
      write_embedded_release_pubkey "$c_key"
      if [ -z "$key_sha256" ]; then
        key_sha256=$PINNED_RELEASE_PUBKEY_SHA256
      fi
    fi
  fi
  [ -n "$c_key" ] ||
    fail 'signed component verification requires --verify-key or installed release-public-key.pem'
  require_regular_file "$c_key" 'component verifier public key'
  if [ -n "$key_sha256" ] && [ -z "$verified_active_pubkey" ]; then
    c_actual_sha=$(sha256_hex "$c_key")
    [ "$c_actual_sha" = "$key_sha256" ] ||
      fail "component verifier public key SHA-256 mismatch (expected $key_sha256, got $c_actual_sha)"
  fi
  component_verify_key_path=$c_key
}

update_generation_links() {
  u_root=$1
  u_gen=$2
  mkdir -p "$u_root/generations"
  chmod 0700 "$u_root" "$u_root/generations"
  u_prev=''
  if [ -L "$u_root/current" ]; then
    u_prev=$(readlink "$u_root/current" 2>/dev/null || :)
  fi
  if [ -n "$u_prev" ] && [ "$u_prev" != "generations/$u_gen" ]; then
    u_prev_tmp="$u_root/.previous.tmp.$$"
    rm -f "$u_prev_tmp"
    ln -s "$u_prev" "$u_prev_tmp"
    mv -Tf "$u_prev_tmp" "$u_root/previous" 2>/dev/null || {
      rm -f "$u_prev_tmp"
      ln -sfn "$u_prev" "$u_root/previous"
    }
  fi
  u_cur_tmp="$u_root/.current.tmp.$$"
  rm -f "$u_cur_tmp"
  ln -s "generations/$u_gen" "$u_cur_tmp"
  mv -Tf "$u_cur_tmp" "$u_root/current" 2>/dev/null || {
    rm -f "$u_cur_tmp"
    ln -sfn "generations/$u_gen" "$u_root/current"
  }
}

find_runit_service_script() {
  # An explicitly invoked checkout ships the matching supervisor source.
  # Remote/stdin installers use the helper from the selected tunnel bundle.
  if [ -n "$script_dir" ] && [ -f "$script_dir/scripts/runit_service.py" ] &&
     [ ! -L "$script_dir/scripts/runit_service.py" ]; then
    printf '%s\n' "$script_dir/scripts/runit_service.py"
    return 0
  fi
  r_installed="$HOME/.local/lib/codex/transport/tunnel-client/current/scripts/runit_service.py"
  if [ -f "$r_installed" ] && [ ! -L "$r_installed" ]; then
    printf '%s\n' "$r_installed"
    return 0
  fi
  return 1
}

component_specs() {
  case $1 in
    codex-ext)
      printf '%s:%s:%s:%s\n' 'ext' 'codex-ext' 'codex-ext-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    tunnel-client)
      printf '%s:%s:%s:%s\n' 'transport/tunnel-client' 'tunnel-client' 'tunnel-client-aarch64-linux-android.tar.gz' 'v0.0.21'
      ;;
    termux-shell-mcp)
      printf '%s:%s:%s:%s\n' 'mcp/termux-shell' 'termux-shell-mcp' 'termux-shell-mcp-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    termux-codex-mcp)
      printf '%s:%s:%s:%s\n' 'mcp/termux-codex' 'termux-codex-mcp' 'termux-codex-mcp-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    chatgpt-provider)
      printf '%s:%s:%s:%s\n' 'providers/chatgpt' 'chatgpt-provider' 'chatgpt-provider-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    termux-chatgpt-mcp)
      printf '%s:%s:%s:%s\n' 'mcp/termux-chatgpt' 'termux-chatgpt-mcp' 'termux-chatgpt-mcp-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    termux-orch)
      printf '%s:%s:%s:%s\n' 'mcp/termux-orch' 'termux-orch' 'termux-orch-aarch64-linux-android.tar.gz' '0.1.0'
      ;;
    *)
      fail "unsupported component: $1"
      ;;
  esac
}

tunnel_client_updated=0

install_atomic_prefix_binary() {
  i_src=$1
  i_name=$2
  i_tmp="$dest_dir/.${i_name}.tmp.$$"
  rm -f "$i_tmp"
  cp "$i_src" "$i_tmp"
  chmod 0755 "$i_tmp"
  install_sync "$i_tmp"
  mv -f "$i_tmp" "$dest_dir/$i_name"
  install_sync "$dest_dir"
}

run_runit_service_action() {
  a_py=$1
  a_script=$2
  shift 2
  a_out=$("$a_py" "$a_script" --home "$HOME" --prefix "$prefix" "$@") || return 1
  # A successful CLI invocation is not proof that the requested image is active.
  printf '%s\n' "$a_out" | "$a_py" -c '
import json, sys
result = json.load(sys.stdin)
state = result.get("activation_state")
if result.get("enrolled") is True:
    sys.exit(0)
if state == "pending_activation":
    print("codex install: activation pending for " + result["service"] + ": "
          + result.get("deferred_reason", "selected and running images differ"), file=sys.stderr)
elif state in ("stopped", "not_enrolled"):
    print("codex install: generation selected; service " + result["service"]
          + " is " + state, file=sys.stderr)
elif state != "active":
    print("codex install: service activation could not be verified", file=sys.stderr)
    sys.exit(1)
'
}

service_matches_target() {
  m_py=$1
  m_script=$2
  m_comp=$3
  m_bin=$4
  m_svc=$5
  m_gen=$6
  m_runner_comp=${7:-}
  m_runner_bin=${8:-}
  [ -d "$prefix/var/service/$m_svc" ] || return 1
  if [ -n "$m_runner_comp" ]; then
    m_out=$("$m_py" "$m_script" --home "$HOME" --prefix "$prefix" \
      --component "$m_comp" --binary "$m_bin" --service "$m_svc" \
      --runner-component "$m_runner_comp" --runner-binary "$m_runner_bin" status 2>/dev/null) || return 1
  else
    m_out=$("$m_py" "$m_script" --home "$HOME" --prefix "$prefix" \
      --component "$m_comp" --binary "$m_bin" --service "$m_svc" status 2>/dev/null) || return 1
  fi
  printf '%s\n' "$m_out" | "$m_py" -c '
import json, sys
target = sys.argv[1]
d = json.load(sys.stdin)
ok = (
    d.get("activation_state") == "active"
    and (d.get("running_generation") or {}).get("version") == target
    and (d.get("selected_generation") or {}).get("version") == target
    and not (d.get("running_generation") or {}).get("deleted_image")
)
raise SystemExit(0 if ok else 1)
' "$m_gen" >/dev/null 2>&1
}

ensure_chatgpt_profile_config() {
  e_bin=$1
  e_cfg="$HOME/.codex/chatgpt.config.toml"
  if [ -e "$e_cfg" ] || [ -L "$e_cfg" ]; then
    return 0
  fi
  [ -x /system/bin/linker64 ] || return 0
  mkdir -p "$HOME/.codex"
  chmod 0700 "$HOME/.codex"
  e_tmp="$HOME/.codex/.chatgpt.config.toml.tmp.$$"
  rm -f "$e_tmp"
  if "$e_bin" print-codex-profile --standalone --model gpt-6-2 > "$e_tmp" 2>/dev/null; then
    chmod 0600 "$e_tmp"
    install_sync "$e_tmp"
    mv -f "$e_tmp" "$e_cfg"
    note "generated default $e_cfg"
  else
    rm -f "$e_tmp"
  fi
}

activate_component_service() {
  ac_tag=$1
  ac_comp=$2
  ac_bin=$3
  ac_root=$4
  ac_gen=$5
  if [ "$no_services" -eq 1 ]; then
    update_generation_links "$ac_root" "$ac_gen"
    return 0
  fi
  if [ ! -x /system/bin/linker64 ]; then
    update_generation_links "$ac_root" "$ac_gen"
    return 0
  fi
  ac_py=$(find_tool python3 || :)
  ac_sv=$(find_tool sv || :)
  ac_script=$(find_runit_service_script || :)
  if [ -z "$ac_py" ] || [ -z "$ac_sv" ] || [ -z "$ac_script" ]; then
    update_generation_links "$ac_root" "$ac_gen"
    if [ "$ac_tag" != 'tunnel-client' ]; then
      note "staged $ac_tag ($ac_gen); install python and termux-services to supervise runit service"
    fi
    return 0
  fi

  case $ac_tag in
    tunnel-client)
      update_generation_links "$ac_root" "$ac_gen"
      return 0
      ;;
    chatgpt-provider)
      if [ ! -d "$prefix/var/service/chatgpt-provider" ]; then
        update_generation_links "$ac_root" "$ac_gen"
        run_runit_service_action "$ac_py" "$ac_script" \
          --component "$ac_comp" --binary "$ac_bin" --service chatgpt-provider \
          enroll --arg serve >/dev/null
      fi
      if ! service_matches_target "$ac_py" "$ac_script" "$ac_comp" "$ac_bin" chatgpt-provider "$ac_gen"; then
        run_runit_service_action "$ac_py" "$ac_script" \
          --component "$ac_comp" --binary "$ac_bin" --service chatgpt-provider \
          activate "$ac_gen" --readiness-cmd 'curl -fsS http://127.0.0.1:4319/healthz' >/dev/null
      fi
      return 0
      ;;
    termux-shell-mcp|termux-codex-mcp|termux-chatgpt-mcp)
      ac_short=${ac_tag%-mcp}
      ac_yaml="$HOME/.config/codex/tunnels/${ac_short}.yaml"
      ac_svc="${ac_short}-tunnel"
      if [ -f "$ac_yaml" ]; then
        if [ ! -d "$prefix/var/service/$ac_svc" ]; then
          update_generation_links "$ac_root" "$ac_gen"
          run_runit_service_action "$ac_py" "$ac_script" \
            --component "$ac_comp" --binary "$ac_bin" --service "$ac_svc" \
            --runner-component transport/tunnel-client --runner-binary tunnel-client \
            enroll --arg run --arg --config --arg "$ac_yaml" --arg --mcp.stdio-send-initialized-notification >/dev/null
        fi
        if [ "$tunnel_client_updated" -eq 1 ] || \
           ! service_matches_target "$ac_py" "$ac_script" "$ac_comp" "$ac_bin" "$ac_svc" "$ac_gen" transport/tunnel-client tunnel-client; then
          run_runit_service_action "$ac_py" "$ac_script" \
            --component "$ac_comp" --binary "$ac_bin" --service "$ac_svc" \
            --runner-component transport/tunnel-client --runner-binary tunnel-client \
            activate "$ac_gen" >/dev/null
        fi
      else
        update_generation_links "$ac_root" "$ac_gen"
        note "staged $ac_tag ($ac_gen); tunnel config missing ($ac_yaml)"
      fi
      return 0
      ;;
    termux-orch)
      ac_yaml="$HOME/.config/codex/tunnels/termux-orch.yaml"
      ac_relay_health="$HOME/.local/state/codex/services/termux-orch-relay/health.url"
      if [ ! -d "$prefix/var/service/termux-orch-relay" ]; then
        update_generation_links "$ac_root" "$ac_gen"
        run_runit_service_action "$ac_py" "$ac_script" \
          --component "$ac_comp" --binary "$ac_bin" --service termux-orch-relay \
          enroll --arg relay --arg --bind --arg 127.0.0.1:0 \
          --arg --health-url-file --arg "$ac_relay_health" --arg --poll-ms --arg 2000 >/dev/null
      fi
      if [ -f "$ac_yaml" ]; then
        if [ ! -d "$prefix/var/service/termux-orch-tunnel" ]; then
          run_runit_service_action "$ac_py" "$ac_script" \
            --component "$ac_comp" --binary "$ac_bin" --service termux-orch-tunnel \
            --runner-component transport/tunnel-client --runner-binary tunnel-client \
            enroll --arg run --arg --config --arg "$ac_yaml" --arg --mcp.stdio-send-initialized-notification >/dev/null
        fi
        if [ "$tunnel_client_updated" -eq 1 ] || \
           ! service_matches_target "$ac_py" "$ac_script" "$ac_comp" "$ac_bin" termux-orch-tunnel "$ac_gen" transport/tunnel-client tunnel-client || \
           ! service_matches_target "$ac_py" "$ac_script" "$ac_comp" "$ac_bin" termux-orch-relay "$ac_gen"; then
          run_runit_service_action "$ac_py" "$ac_script" \
            --component "$ac_comp" --binary "$ac_bin" --service termux-orch-tunnel \
            --runner-component transport/tunnel-client --runner-binary tunnel-client \
            --companion-service termux-orch-relay \
            activate "$ac_gen" >/dev/null
        fi
      else
        if ! service_matches_target "$ac_py" "$ac_script" "$ac_comp" "$ac_bin" termux-orch-relay "$ac_gen"; then
          run_runit_service_action "$ac_py" "$ac_script" \
            --component "$ac_comp" --binary "$ac_bin" --service termux-orch-relay \
            activate "$ac_gen" >/dev/null
        fi
        note "staged termux-orch-tunnel ($ac_gen); tunnel config missing ($ac_yaml)"
      fi
      return 0
      ;;
  esac
}

install_signed_component() {
  ic_tag=$1
  ic_spec=$(component_specs "$ic_tag")
  ic_comp=${ic_spec%%:*}
  ic_rest=${ic_spec#*:}
  ic_bin=${ic_rest%%:*}
  ic_rest=${ic_rest#*:}
  ic_asset=${ic_rest%%:*}
  ic_default_ver=${ic_rest#*:}

  ensure_component_verify_key
  ic_openssl=$(find_tool openssl) || fail 'openssl is required for Ed25519 signed component verification'
  ic_tar=$(find_tool tar) || fail 'tar is required to unpack component release assets'
  find_tool gzip >/dev/null 2>&1 || fail 'gzip is required to unpack component release assets'

  ic_work="$work/comp-$ic_tag"
  mkdir -p "$ic_work/extract"
  chmod 0700 "$ic_work" "$ic_work/extract"
  ic_url="https://github.com/$repo/releases/download/$ic_tag"
  ic_sums="$ic_work/SHA256SUMS"
  ic_sig="$ic_work/SHA256SUMS.sig"
  ic_tb="$ic_work/$ic_asset"
  ic_tb_sha="$ic_work/$ic_asset.sha256"

  note "downloading signed $ic_tag ($repo)"
  fetch_https "$ic_url/SHA256SUMS" "$ic_sums" "$MAX_SUMS_BYTES"
  fetch_https "$ic_url/SHA256SUMS.sig" "$ic_sig" "$ED25519_SIG_BYTES"
  verify_ed25519_sig "$ic_openssl" "$component_verify_key_path" "$ic_sums" "$ic_sig"
  ic_seq=$(verify_component_sums_headers "$ic_sums" "$ic_tag")
  ic_cmt=$(parse_signed_sums_commit "$ic_sums")
  ic_cmt8=$(printf '%.8s\n' "$ic_cmt")

  ic_seq_dir="$HOME/.local/share/codex/components"
  ic_seq_file="$ic_seq_dir/${ic_tag}.sequence"
  ic_prev_seq=$(read_recorded_release_sequence "$ic_seq_file")
  if [ "$ic_seq" -lt "$ic_prev_seq" ]; then
    fail "$ic_tag sequence rollback rejected (got $ic_seq, installed $ic_prev_seq)"
  fi

  fetch_https "$ic_url/$ic_asset" "$ic_tb" "$MAX_COMPONENT_TARBALL_BYTES"
  verify_sha256_file "$ic_tb" "$ic_sums" "$ic_asset"
  if fetch_optional_https "$ic_url/$ic_asset.sha256" "$ic_tb_sha" "$MAX_SHA256_BYTES"; then
    verify_sha256_file "$ic_tb" "$ic_tb_sha" "$ic_asset"
  fi

  if "$ic_tar" -tzf "$ic_tb" | grep -E '(^/|(^|/)\.\.(/|$))' >/dev/null 2>&1; then
    fail "$ic_tag tarball contains unsafe path entries"
  fi
  "$ic_tar" -xzf "$ic_tb" -C "$ic_work/extract"
  require_regular_file "$ic_work/extract/bin/$ic_bin" "$ic_tag bin/$ic_bin"
  require_regular_file "$ic_work/extract/SHA256SUMS" "$ic_tag bundle checksums"
  verify_bundle_checksums "$ic_work/extract" "$ic_work/extract/SHA256SUMS"
  verify_sha256_file "$ic_work/extract/bin/$ic_bin" "$ic_work/extract/SHA256SUMS" "bin/$ic_bin"
  if [ "$ic_tag" = 'tunnel-client' ]; then
    require_regular_file "$ic_work/extract/scripts/runit_service.py" 'tunnel supervisor'
    verify_sha256_file "$ic_work/extract/scripts/runit_service.py" "$ic_work/extract/SHA256SUMS" 'scripts/runit_service.py'
  fi

  ic_ver=$ic_default_ver
  if [ "$ic_tag" = 'tunnel-client' ] && [ -f "$ic_work/extract/build.json" ]; then
    ic_parsed_ver=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$ic_work/extract/build.json" | head -n 1)
    case $ic_parsed_ver in
      ''|*[!A-Za-z0-9._-]*) ;;
      *) ic_ver=$ic_parsed_ver ;;
    esac
  fi
  ic_gen="${ic_ver}-${ic_cmt8}"
  ic_root="$HOME/.local/lib/codex/$ic_comp"
  ic_gen_dir="$ic_root/generations/$ic_gen"
  mkdir -p "$ic_root/generations"
  chmod 0700 "$ic_root" "$ic_root/generations"

  ic_new_bin_sha=$(sha256_hex "$ic_work/extract/bin/$ic_bin")
  ic_cur_bin_sha=$(digest_or_none "$ic_root/current/$ic_bin")
  if [ "$ic_tag" = 'tunnel-client' ] && [ "$ic_new_bin_sha" != "$ic_cur_bin_sha" ]; then
    tunnel_client_updated=1
  fi

  ic_reinstall=1
  if [ -e "$ic_gen_dir" ] || [ -L "$ic_gen_dir" ]; then
    if ! { [ -d "$ic_gen_dir" ] && [ ! -L "$ic_gen_dir" ]; }; then
      fail "generation path is not a real directory: $ic_gen_dir"
    fi
    if ! { [ -d "$ic_gen_dir/signed" ] && [ ! -L "$ic_gen_dir/signed" ] &&
      [ -f "$ic_gen_dir/bin/$ic_bin" ] && [ ! -L "$ic_gen_dir/bin/$ic_bin" ] &&
      [ "$(sha256_hex "$ic_gen_dir/bin/$ic_bin")" = "$ic_new_bin_sha" ] &&
      [ -f "$ic_gen_dir/$ic_bin" ] && [ ! -L "$ic_gen_dir/$ic_bin" ] &&
      [ -x "$ic_gen_dir/$ic_bin" ] &&
      [ "$(sha256_hex "$ic_gen_dir/$ic_bin")" = "$ic_new_bin_sha" ] &&
      [ -f "$ic_gen_dir/SHA256SUMS" ] && [ ! -L "$ic_gen_dir/SHA256SUMS" ] &&
      cmp -s "$ic_gen_dir/SHA256SUMS" "$ic_work/extract/SHA256SUMS" &&
      [ -f "$ic_gen_dir/signed/SHA256SUMS" ] && [ ! -L "$ic_gen_dir/signed/SHA256SUMS" ] &&
      cmp -s "$ic_gen_dir/signed/SHA256SUMS" "$ic_sums" &&
      [ -f "$ic_gen_dir/signed/SHA256SUMS.sig" ] && [ ! -L "$ic_gen_dir/signed/SHA256SUMS.sig" ] &&
      cmp -s "$ic_gen_dir/signed/SHA256SUMS.sig" "$ic_sig"; }; then
      fail "immutable generation identity conflict; refusing to replace $ic_gen_dir"
    fi
    verify_bundle_checksums "$ic_gen_dir" "$ic_work/extract/SHA256SUMS"
    ic_key="$ic_gen_dir/signed/release-public-key.pem"
    if [ -e "$ic_key" ] || [ -L "$ic_key" ]; then
      require_regular_file "$ic_key" 'generation release public key'
      cmp -s "$ic_key" "$component_verify_key_path" ||
        fail "immutable generation public key conflict; refusing to reuse $ic_gen_dir"
    else
      ic_key_tmp="$ic_gen_dir/signed/.release-public-key.pem.tmp.$$"
      if [ -e "$ic_key_tmp" ] || [ -L "$ic_key_tmp" ]; then
        fail "generation public key staging collision: $ic_key_tmp"
      fi
      cp "$component_verify_key_path" "$ic_key_tmp"
      chmod 0600 "$ic_key_tmp"
      install_sync "$ic_key_tmp"
      mv "$ic_key_tmp" "$ic_key" || fail "cannot backfill generation public key: $ic_gen_dir"
      install_sync "$ic_gen_dir/signed"
    fi
    ic_reinstall=0
  fi

  if [ "$ic_reinstall" -eq 1 ]; then
    ic_stage="$ic_root/generations/.${ic_gen}.stage.$$"
    if [ -e "$ic_stage" ] || [ -L "$ic_stage" ]; then
      fail "generation staging collision: $ic_stage"
    fi
    mkdir -p "$ic_stage/signed"
    cp -R "$ic_work/extract/." "$ic_stage/"
    cp "$ic_work/extract/bin/$ic_bin" "$ic_stage/$ic_bin"
    cp "$ic_sums" "$ic_sig" "$ic_stage/signed/"
    cp "$component_verify_key_path" "$ic_stage/signed/release-public-key.pem"
    if [ -f "$ic_tb_sha" ]; then
      cp "$ic_tb_sha" "$ic_stage/signed/"
    fi
    chmod 0700 "$ic_stage" "$ic_stage/bin" "$ic_stage/signed" "$ic_stage/bin/$ic_bin" "$ic_stage/$ic_bin"
    chmod 0600 "$ic_stage/signed/"*
    if [ -f "$ic_stage/SHA256SUMS" ]; then
      chmod 0600 "$ic_stage/SHA256SUMS"
    fi
    if [ -d "$ic_stage/scripts" ]; then
      chmod 0700 "$ic_stage/scripts"
      chmod 0755 "$ic_stage/scripts/"* 2>/dev/null || :
    fi
    if [ -d "$ic_stage/host" ]; then
      chmod 0700 "$ic_stage/host"
      chmod 0600 "$ic_stage/host/"* 2>/dev/null || :
      if [ -f "$ic_stage/host/manager.py" ]; then
        chmod 0700 "$ic_stage/host/manager.py"
      fi
    fi
    install_sync "$ic_stage/$ic_bin"
    install_sync "$ic_stage"
    mv "$ic_stage" "$ic_gen_dir" || fail "cannot publish immutable generation $ic_gen_dir"
    install_sync "$ic_root/generations"
  fi

  if [ "$ic_tag" = 'codex-ext' ]; then
    install_atomic_prefix_binary "$ic_gen_dir/bin/codex-ext" codex-ext
    rm -f "$dest_dir/.cdx.tmp.$$"
    ln -s codex-ext "$dest_dir/.cdx.tmp.$$"
    mv -Tf "$dest_dir/.cdx.tmp.$$" "$dest_dir/cdx" 2>/dev/null || {
      rm -f "$dest_dir/.cdx.tmp.$$"
      ln -sfn codex-ext "$dest_dir/cdx"
    }
    update_generation_links "$ic_root" "$ic_gen"
  else
    activate_component_service "$ic_tag" "$ic_comp" "$ic_bin" "$ic_root" "$ic_gen"
    if [ "$ic_tag" = 'chatgpt-provider' ]; then
      ensure_chatgpt_profile_config "$ic_gen_dir/bin/chatgpt-provider"
    fi
  fi

  mkdir -p "$ic_seq_dir"
  chmod 0700 "$ic_seq_dir"
  ic_seq_tmp="$ic_seq_dir/.${ic_tag}.sequence.tmp.$$"
  printf '%s\n' "$ic_seq" > "$ic_seq_tmp"
  chmod 0600 "$ic_seq_tmp"
  install_sync "$ic_seq_tmp"
  mv -f "$ic_seq_tmp" "$ic_seq_file"
  install_sync "$ic_seq_dir"
  note "installed $ic_tag generation $ic_gen (sequence $ic_seq)"
}

tty_available() {
  [ -t 1 ] && [ -r /dev/tty ] && [ -w /dev/tty ]
}

tty_prompt() {
  tp_msg=$1
  tp_def=${2:-}
  if tty_available; then
    printf '%s' "$tp_msg" > /dev/tty
    IFS= read -r tp_ans < /dev/tty || tp_ans=''
    if [ -z "$tp_ans" ] && [ -n "$tp_def" ]; then
      tp_ans=$tp_def
    fi
    printf '%s\n' "$tp_ans"
  else
    printf '%s\n' "$tp_def"
  fi
}

tty_prompt_secret() {
  ts_msg=$1
  tty_available || return 1
  printf '%s' "$ts_msg" > /dev/tty
  stty -echo < /dev/tty 2>/dev/null || :
  IFS= read -r ts_val < /dev/tty || ts_val=''
  stty echo < /dev/tty 2>/dev/null || :
  printf '\n' > /dev/tty
  printf '%s' "$ts_val"
}

perform_codex_login() {
  pl_method=$1
  shift
  case $pl_method in
    device|1)
      note "starting Device Code login (follow URL and one-time code on screen)..."
      if [ "$#" -gt 0 ]; then
        "$@" login --device-auth
      else
        "$dest_bin" login --device-auth
      fi
      ;;
    browser|2)
      if [ "$#" -gt 0 ]; then
        "$@" login
      else
        "$dest_bin" login
      fi
      ;;
    api-key|3)
      if [ -n "${OPENAI_API_KEY:-}" ]; then
        pl_key=$OPENAI_API_KEY
      else
        pl_key=$(tty_prompt_secret 'OpenAI API key (input hidden, empty skips): ') || return 0
      fi
      [ -n "$pl_key" ] || { note 'skipped API key login'; return 0; }
      if [ "$#" -gt 0 ]; then
        printf '%s\n' "$pl_key" | "$@" login --with-api-key
      else
        printf '%s\n' "$pl_key" | "$dest_bin" login --with-api-key
      fi
      pl_key=''
      ;;
    skip|4|'')
      return 0
      ;;
    *)
      fail "unknown login method: $pl_method"
      ;;
  esac
}

setup_codex_accounts() {
  [ -x "$dest_bin" ] || fail "codex binary not found at $dest_bin; install wrapper first"
  if [ -n "$login_method" ]; then
    perform_codex_login "$login_method"
    return 0
  fi
  tty_available || return 0

  printf '\n=== Codex Account Setup ===\n' > /dev/tty
  if "$dest_bin" login status >/dev/tty 2>&1; then
    sca_relogin=$(tty_prompt 'Default account already configured. Re-authenticate default account? [y/N]: ' 'n')
    case $sca_relogin in
      y|Y|yes|YES)
        printf 'Select login method:\n  1) Device Code (codex login --device-auth) [default]\n  2) Browser OAuth (codex login)\n  3) API Key (hidden input)\n  4) Skip\n' > /dev/tty
        sca_choice=$(tty_prompt 'Choice [1]: ' '1')
        perform_codex_login "$sca_choice"
        ;;
    esac
  else
    printf 'Default account is not logged in. Select login method:\n  1) Device Code (codex login --device-auth) [default]\n  2) Browser OAuth (codex login)\n  3) API Key (hidden input)\n  4) Skip\n' > /dev/tty
    sca_choice=$(tty_prompt 'Choice [1]: ' '1')
    perform_codex_login "$sca_choice"
  fi

  if [ -x "$dest_dir/cdx" ]; then
    printf '\nConfigured cdx profiles:\n' > /dev/tty
    "$dest_dir/cdx" profile list --no-daemon-check > /dev/tty 2>&1 || :
    while true; do
      sca_add=$(tty_prompt 'Create/login an additional cdx account profile? [y/N]: ' 'n')
      case $sca_add in
        y|Y|yes|YES)
          sca_name=$(tty_prompt 'Profile name (e.g. work, alt): ' '')
          [ -n "$sca_name" ] || break
          if ! "$dest_dir/cdx" profile create "$sca_name" >/dev/tty 2>&1; then
            note "using existing or failed profile '$sca_name'"
          fi
          printf 'Login "%s" using:\n  1) Device Code (cdx use %s login --device-auth) [default]\n  2) Browser OAuth\n  3) API Key (hidden input)\n  4) Skip\n' \
            "$sca_name" "$sca_name" > /dev/tty
          sca_pchoice=$(tty_prompt 'Choice [1]: ' '1')
          perform_codex_login "$sca_pchoice" "$dest_dir/cdx" use "$sca_name"
          ;;
        *)
          break
          ;;
      esac
    done
  fi
}

setup_chatgpt_provider() {
  tty_available || return 0
  scp_bin="$HOME/.local/lib/codex/providers/chatgpt/current/chatgpt-provider"
  [ -x "$scp_bin" ] || return 0
  scp_py=$(find_tool python3 || :)
  scp_tok="$HOME/.local/state/codex/providers/chatgpt/control.token"
  scp_auth='unknown'
  if [ -n "$scp_py" ] && [ -f "$scp_tok" ]; then
    scp_auth=$("$scp_py" - "$scp_tok" <<'PYEOF' 2>/dev/null || printf 'stopped'
import json, sys, urllib.request
tok = open(sys.argv[1]).read().strip()
req = urllib.request.Request("http://127.0.0.1:4319/control/status", headers={"Authorization": f"Bearer {tok}"})
with urllib.request.urlopen(req, timeout=15) as r:
    d = json.loads(r.read().decode())
print(d.get("auth_state") or "unknown")
PYEOF
)
  fi
  printf '\n=== ChatGPT Desktop Provider ===\n' > /dev/tty
  printf 'Current status: %s\n' "$scp_auth" > /dev/tty
  if [ "$scp_auth" = 'authenticated' ]; then
    return 0
  fi
  scp_host="$HOME/.local/lib/codex/host/chatgpt-desktop/current"
  scp_mgr="$HOME/.local/lib/codex/providers/chatgpt/current/host/manager.py"
  if [ ! -e "$scp_host" ] && [ -f "$scp_mgr" ] && [ -n "$scp_py" ]; then
    scp_inst=$(tty_prompt 'ChatGPT Desktop AppHost is not installed yet. Install it now (~400 MB)? [y/N]: ' 'n')
    case $scp_inst in
      y|Y|yes|YES)
        "$scp_py" "$scp_mgr" install --provider-bin "$scp_bin" >/dev/tty 2>&1 ||
          note 'AppHost install failed; run manager.py install later'
        ;;
    esac
  fi
  if [ -e "$scp_host" ] && [ -n "$scp_py" ] && [ -f "$scp_tok" ]; then
    scp_open=$(tty_prompt 'Open ChatGPT Desktop viewer in browser to sign in (if needed)? [y/N]: ' 'n')
    case $scp_open in
      y|Y|yes|YES)
        "$scp_py" - "$scp_tok" <<'PYEOF' >/dev/tty 2>&1 || :
import json, sys, urllib.request
tok = open(sys.argv[1]).read().strip()
for path in ("/control/host/restart", "/control/viewer/open"):
    req = urllib.request.Request(f"http://127.0.0.1:4319{path}", data=b"", method="POST", headers={"Authorization": f"Bearer {tok}"})
    with urllib.request.urlopen(req, timeout=90) as r:
        print(path, r.read().decode())
PYEOF
        ;;
    esac
  fi
}

setup_tunnels() {
  tty_available || return 0
  st_dir="$HOME/.config/codex/tunnels"
  st_missing=0
  printf '\n=== OpenAI MCP Tunnels ===\n' > /dev/tty
  for st_name in termux-shell termux-codex termux-chatgpt termux-orch; do
    if [ -f "$st_dir/${st_name}.yaml" ]; then
      printf '  * %-15s configured\n' "$st_name" > /dev/tty
    else
      printf '  * %-15s not configured\n' "$st_name" > /dev/tty
      st_missing=1
    fi
  done
  [ "$st_missing" -eq 1 ] || return 0
  st_cfg=$(tty_prompt 'Configure missing MCP tunnel YAML files now (optional)? [y/N]: ' 'n')
  case $st_cfg in
    y|Y|yes|YES) ;;
    *) return 0 ;;
  esac
  if [ -n "${CONTROL_PLANE_API_KEY:-}" ]; then
    st_api_key=$CONTROL_PLANE_API_KEY
  else
    st_api_key=$(tty_prompt_secret 'CONTROL_PLANE_API_KEY (hidden, empty cancels): ') || return 0
  fi
  [ -n "$st_api_key" ] || return 0
  mkdir -p "$st_dir"
  chmod 0700 "$st_dir"
  for st_pair in \
    'termux-shell:termux-shell-mcp:mcp/termux-shell:termux-shell-mcp:termux-shell-tunnel' \
    'termux-codex:termux-codex-mcp:mcp/termux-codex:termux-codex-mcp:termux-codex-tunnel' \
    'termux-chatgpt:termux-chatgpt-mcp:mcp/termux-chatgpt:termux-chatgpt-mcp:termux-chatgpt-tunnel' \
    'termux-orch:termux-orch:mcp/termux-orch:termux-orch:termux-orch-tunnel'; do
    st_n=${st_pair%%:*}
    st_r=${st_pair#*:}
    st_tag=${st_r%%:*}
    st_r=${st_r#*:}
    st_comp=${st_r%%:*}
    st_r=${st_r#*:}
    st_bin=${st_r%%:*}
    st_svc=${st_r#*:}
    [ ! -f "$st_dir/${st_n}.yaml" ] || continue
    st_tid=$(tty_prompt "Tunnel ID for $st_n (e.g. tunnel_..., blank skips): " '')
    [ -n "$st_tid" ] || continue
    validate_simple_token 'tunnel_id' "$st_tid"
    st_yaml="$st_dir/${st_n}.yaml"
    st_ytmp="$st_dir/.${st_n}.yaml.tmp.$$"
    cat > "$st_ytmp" <<EOF
config_version: 1
control_plane:
  base_url: "https://api.openai.com"
  tunnel_id: "$st_tid"
  api_key: "$st_api_key"
health:
  listen_addr: "127.0.0.1:0"
  url_file: "$HOME/.local/state/codex/services/$st_svc/health.url"
admin_ui:
  open_browser: false
log:
  level: info
  format: json
mcp:
  commands:
    - channel: main
      command: "$HOME/.local/lib/codex/$st_comp/current/$st_bin"
EOF
    chmod 0600 "$st_ytmp"
    install_sync "$st_ytmp"
    mv -f "$st_ytmp" "$st_yaml"
    if [ "$no_services" -eq 0 ] && [ -L "$HOME/.local/lib/codex/$st_comp/current" ]; then
      st_cur_ver=$(basename -- "$(readlink "$HOME/.local/lib/codex/$st_comp/current")")
      activate_component_service "$st_tag" "$st_comp" "$st_bin" "$HOME/.local/lib/codex/$st_comp" "$st_cur_ver" || :
    fi
  done
  st_api_key=''
}

run_setup_wizard() {
  setup_codex_accounts
  setup_chatgpt_provider
  setup_tunnels
  note "setup complete (run 'codex', 'cdx', 'cdx -p chatgpt', or 'codex login --device-auth' anytime)"
}

maybe_run_components_and_setup() {
  mr_list=''
  if [ "$all_components" -eq 1 ]; then
    mr_list=$ALL_COMPONENTS
  elif [ -n "$selected_components" ]; then
    mr_list=$selected_components
  elif [ "$wrapper_only" -eq 0 ] && [ "$skip_upstream" -eq 0 ] && \
       [ "$source_mode" = 'release' ] && [ "$release_tag" = 'wrapper' ] && \
       [ -x /system/bin/linker64 ]; then
    mr_list=$ALL_COMPONENTS
  fi

  if [ -n "$mr_list" ]; then
    for mr_tag in $mr_list; do
      install_signed_component "$mr_tag"
    done
  fi

  mr_setup=0
  case $setup_mode in
    always) mr_setup=1 ;;
    never) mr_setup=0 ;;
    auto)
      if [ "$wrapper_only" -eq 0 ] && [ "$skip_upstream" -eq 0 ] && \
         [ "$source_mode" = 'release' ] && [ "$release_tag" = 'wrapper' ] && \
         [ -t 1 ] && [ -x /system/bin/linker64 ] && tty_available && [ -z "${CI:-}" ]; then
        mr_setup=1
      fi
      ;;
  esac
  if [ "$mr_setup" -eq 1 ]; then
    run_setup_wizard
  fi
}

if [ "$setup_only" -eq 1 ]; then
  run_setup_wizard
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
  if [ -z "$script_dir" ]; then
    source_mode='release'
    release_tag=$DEFAULT_RELEASE_TAG
  elif [ -f "$script_dir/bin/codex" ] && [ ! -L "$script_dir/bin/codex" ] && [ -f "$script_dir/SHA256SUMS" ]; then
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
    if [ -z "$script_dir" ] || [ ! -f "$script_dir/Cargo.toml" ]; then
      fail '--build must be run from the codex repository root'
    fi
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

    default_key="$HOME/.local/lib/codex/wrapper/release-public-key.pem"
    initial_key_sha=$(digest_or_none "$default_key")
    if [ -z "$verify_key" ]; then
      if [ -e "$default_key" ] || [ -L "$default_key" ]; then
        verify_key=$default_key
      elif [ -n "$PINNED_RELEASE_PUBKEY_SHA256" ]; then
        verify_key="$work/embedded-release-public-key.pem"
        write_embedded_release_pubkey "$verify_key"
        if [ -z "$key_sha256" ]; then
          key_sha256=$PINNED_RELEASE_PUBKEY_SHA256
        fi
      fi
    fi
    if [ -n "$verify_key" ] || [ "$require_signed" -eq 1 ] || [ -n "$key_sha256" ]; then
      [ -n "$verify_key" ] ||
        fail 'signed release verification requested, but no --verify-key or installed release-public-key.pem was found'
      require_regular_file "$verify_key" 'verifier public key'
      cp "$verify_key" "$work/verifier-public-key.pem"
      if [ "$verify_key" = "$default_key" ]; then
        [ "$(sha256_hex "$work/verifier-public-key.pem")" = "$initial_key_sha" ] ||
          fail 'release trust key changed during verification'
      fi
      verify_key="$work/verifier-public-key.pem"
      key_size=$(wc -c < "$verify_key" | tr -d '[:space:]')
      if [ "$key_size" -le 0 ] || [ "$key_size" -gt "$MAX_PUBKEY_BYTES" ]; then
        fail "verifier public key out of bounds: $verify_key"
      fi
      if [ -n "$key_sha256" ]; then
        actual_key_sha=$(sha256_hex "$verify_key")
        [ "$actual_key_sha" = "$key_sha256" ] ||
          fail "verifier public key SHA-256 mismatch (expected $key_sha256, got $actual_key_sha)"
      fi
      openssl_bin=$(find_tool openssl) ||
        fail 'openssl is required for Ed25519 signed release verification'
      sums_file="$work/SHA256SUMS"
      sig_file="$work/SHA256SUMS.sig"
      note "downloading signed SHA256SUMS from tag $release_tag ($repo)"
      fetch_https "$base_url/SHA256SUMS" "$sums_file" "$MAX_SUMS_BYTES"
      fetch_https "$base_url/SHA256SUMS.sig" "$sig_file" "$ED25519_SIG_BYTES"

      active_key="$verify_key"
      rot_sig="$work/release-public-key.pem.sig"
      rot_key="$work/release-public-key.pem"
      if fetch_optional_https "$base_url/release-public-key.pem.sig" "$rot_sig" "$ED25519_SIG_BYTES"; then
        fetch_https "$base_url/release-public-key.pem" "$rot_key" "$MAX_PUBKEY_BYTES"
        require_regular_file "$rot_key" 'rotated public key'
        grep -F 'BEGIN PUBLIC KEY' "$rot_key" >/dev/null 2>&1 ||
          fail 'rotated public key is not a PEM public key'
        if [ "$(sha256_hex "$rot_key")" = "$(sha256_hex "$verify_key")" ]; then
          active_key="$verify_key"
        else
          verify_ed25519_sig "$openssl_bin" "$verify_key" "$rot_key" "$rot_sig"
          active_key="$rot_key"
          note 'verified Ed25519 key rotation'
        fi
      fi
      verify_ed25519_sig "$openssl_bin" "$active_key" "$sums_file" "$sig_file"
      seq_val=$(verify_signed_sums_headers "$sums_file")
      prev_seq=$(read_recorded_release_sequence "$HOME/.local/share/codex/wrapper/release-sequence")
      if [ "$seq_val" -lt "$prev_seq" ]; then
        fail "release sequence rollback rejected (got $seq_val, installed $prev_seq)"
      fi
      verified_release_sequence=$seq_val
      verified_active_pubkey=$active_key
      note "verified Ed25519 release signature (sequence $seq_val)"

      note "downloading $ASSET_NAME from tag $release_tag ($repo)"
      fetch_https "$base_url/$ASSET_NAME" "$tarball" "$MAX_TARBALL_BYTES"
      verify_sha256_file "$tarball" "$sums_file" "$ASSET_NAME"
      note 'verified release archive SHA-256 against signed SHA256SUMS'
    else
      note "downloading $ASSET_NAME from tag $release_tag ($repo)"
      fetch_https "$base_url/$ASSET_NAME" "$tarball" "$MAX_TARBALL_BYTES"
      fetch_https "$base_url/$ASSET_NAME.sha256" "$tarball_sha" "$MAX_SHA256_BYTES"
      verify_sha256_file "$tarball" "$tarball_sha"
      note 'verified release archive SHA-256'
    fi

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
  for req in curl gzip; do
    find_tool "$req" >/dev/null 2>&1 || fail "runtime dependency missing from PATH/PREFIX: $req"
  done
fi

mkdir -p "$dest_dir"
if [ -e "$recovery_gate" ] || [ -L "$recovery_gate" ]; then
  fail "unsettled installer recovery: $recovery_gate"
fi
if ! mkdir "$install_lock" 2>/dev/null; then
  fail "installation already in progress or unowned lock: $install_lock (run --recover after inspection)"
fi
lock_owned=1
install_record_owner

if [ -n "$verified_release_sequence" ]; then
  [ "$(digest_or_none "$default_key")" = "$initial_key_sha" ] ||
    fail 'release trust key changed during download; verify again before installing'
  locked_seq=$(read_recorded_release_sequence "$HOME/.local/share/codex/wrapper/release-sequence")
  [ "$verified_release_sequence" -ge "$locked_seq" ] ||
    fail "release sequence rollback rejected under install lock (got $verified_release_sequence, installed $locked_seq)"
fi

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
  if [ -n "$verified_release_sequence" ]; then
    persist_signed_release_state
  fi
  note "skipped upstream bootstrap (--skip-upstream); run '$dest_bin update --upstream' when ready"
  commit_wrapper_transaction
  maybe_run_components_and_setup
  exit 0
fi

if [ -x /system/bin/linker64 ]; then
  needs_update=1
  if qualified_active_generation; then
    note 'retaining the qualified active upstream generation'
    needs_update=0
  elif [ -n "$upstream_version" ]; then
    note "fetching upstream Codex v$upstream_version"
  else
    note 'bootstrapping or requalifying upstream generation for the new Wrapper'
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
    qualified_new_activation ||
      fail 'updater returned but active generation is not qualified against this Wrapper SHA-256'
  else
    qualified_active_generation ||
      fail 'retained active generation is no longer qualified'
  fi
  note "installation complete; try '$dest_bin doctor' or '$dest_bin wrapper status'"
else
  note "non-Android host detected; skipping on-device upstream bootstrap"
fi
if [ -n "$verified_release_sequence" ]; then
  persist_signed_release_state
fi
commit_wrapper_transaction
maybe_run_components_and_setup
