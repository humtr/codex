# Retained signed migration regression fixture; not current installation instructions.
(
  set -eu
  migration="$(mktemp -d "${TMPDIR:?}/codex-migration.XXXXXX")"
  trap 'rm -rf -- "$migration"' EXIT
  gh release download local-hosted-0-161-0-ab644771ac89-manager-tui-bridge \
    --repo humtr/codex --dir "$migration" \
    --pattern core --pattern generation.meta --pattern manager --pattern runtime \
    --pattern codex-code-mode-host --pattern helper-0 --pattern helper-1 \
    --pattern release.manifest --pattern release.sig
  chmod 0644 "$migration/generation.meta" "$migration/release.manifest" "$migration/release.sig"
  mkdir "$migration/helpers"
  mv "$migration/helper-0" "$migration/helpers/0"
  mv "$migration/helper-1" "$migration/helpers/1"
  chmod 0755 "$migration/core" "$migration/manager" "$migration/runtime" \
    "$migration/codex-code-mode-host" "$migration/helpers/0" "$migration/helpers/1"
  codex update --local "$migration"
  codex update
)
