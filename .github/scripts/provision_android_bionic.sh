#!/usr/bin/env bash
# CI-only isolated Android execution substrate; never a device installer.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "$(uname -m)" = aarch64
test -n "${RUNNER_TEMP:-}"
for tool in curl base64 unzip debugfs file readelf sha256sum; do command -v "$tool" >/dev/null; done
sudo test ! -e /system
root="$RUNNER_TEMP/component-bionic"
mkdir -p "$root/bin" "$root/lib64/bionic"
commit=8aeb37cca394ce39c1311744c60960cbd466aa77
checksum=83bf0dce249728dae48149b80d28b48115c54adad95a352120d58a6ac669d1fc
url="https://android.googlesource.com/platform/prebuilts/runtime/+/$commit/mainline/runtime/apex/com.android.runtime-arm64.apex?format=TEXT"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  --connect-timeout 15 --max-time 120 --max-filesize 20971520 --output "$root/runtime.apex.b64" "$url"
base64 --decode "$root/runtime.apex.b64" > "$root/runtime.apex"
test "$(wc -c < "$root/runtime.apex")" -eq 13881344
test "$(sha256sum "$root/runtime.apex" | awk '{print $1}')" = "$checksum"
unzip -p "$root/runtime.apex" apex_payload.img > "$root/apex_payload.img"
test "$(wc -c < "$root/apex_payload.img")" -eq 13783040
debugfs -R "dump -p /bin/linker64 $root/bin/linker64" "$root/apex_payload.img" >/dev/null 2>&1
for library in libc.so libdl.so libm.so; do
  debugfs -R "dump -p /lib64/bionic/$library $root/lib64/bionic/$library" "$root/apex_payload.img" >/dev/null 2>&1
done
sudo mkdir -p /system/bin /system/lib64
sudo install -m 0755 "$root/bin/linker64" /system/bin/linker64
for library in libc.so libdl.so libm.so; do
  sudo install -m 0644 "$root/lib64/bionic/$library" "/system/lib64/$library"
done
