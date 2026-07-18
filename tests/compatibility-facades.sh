#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 -B - "$ROOT_DIR" <<'PY'
import re,sys
from pathlib import Path
root=Path(sys.argv[1])
public=(root/'lib/codex-termux.sh').read_text()
assert len(public.splitlines()) <= 30
assert 'shell/loader.sh' in public
assert 'codex_source_domain()' not in public
for facade in sorted((root/'lib/codex-termux').glob('*.sh')):
    text=facade.read_text()
    assert len(text.splitlines()) <= 15, facade
    assert re.search(r'^\s*(?:function\s+)?[A-Za-z_][A-Za-z0-9_]*\s*\(\)', text, re.M) is None, facade
    assert f'shell/{facade.name}' in text, facade
PY
printf 'compatibility-facades: ok\n'
