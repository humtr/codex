#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_DIR="$ROOT_DIR/.git/hooks"
HOOK="$HOOK_DIR/pre-commit"

mkdir -p "$HOOK_DIR"
cat >"$HOOK" <<'HOOK'
#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
if git diff --cached --quiet -- .; then
    exit 0
fi

"$ROOT_DIR/tools/update-wrapper-version.sh"
git add "$ROOT_DIR/config/wrapper-version.env"
HOOK
chmod 755 "$HOOK"
printf 'Installed %s\n' "$HOOK"
