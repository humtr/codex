#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
OUT_DIR="${CODEX_MERGE_READINESS_OUT_DIR:-$ROOT_DIR/out/merge-readiness}"
mkdir -p "$OUT_DIR/test-logs"
REPORT="$OUT_DIR/merge_readiness_report.json"
AUDIT_JSON="$OUT_DIR/canon-audit.json"
TEST_STATUS="$OUT_DIR/test-status.txt"
PATCH_FILE="$OUT_DIR/branch.patch"
REPO_ZIP="$OUT_DIR/repository-snapshot.zip"
head_sha="$(git rev-parse HEAD 2>/dev/null || printf unknown)"
base_sha="$(git rev-parse origin/main 2>/dev/null || git rev-parse main 2>/dev/null || printf unknown)"
branch="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || printf detached)"
created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
run_url="${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-local/codex}/actions/runs/${GITHUB_RUN_ID:-local}"
syntax_status=0
bash -n install.sh bin/install-local.sh bin/install-runtime.sh lib/codex-termux.sh lib/codex-termux/*.sh || syntax_status=$?
audit_status=0
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B -m codex_termux.cli canon-audit --root . --strict >"$AUDIT_JSON" || audit_status=$?
audit_findings_count="$(python3 - "$AUDIT_JSON" <<'PY'
import json,sys
try: print(len(json.load(open(sys.argv[1], encoding='utf-8')).get('findings', [])))
except Exception: print(-1)
PY
)"
test_exit_code=0
CODEX_TERMUX_TEST_LOG_DIR="$OUT_DIR/test-logs" CODEX_TERMUX_TEST_STATUS_FILE="$TEST_STATUS" PYTHONDONTWRITEBYTECODE=1 bash tests/run-all.sh || test_exit_code=$?
contract_status=ok
grep -q '^wrapper-contracts.sh 0$' "$TEST_STATUS" 2>/dev/null || contract_status=fail
test_status=ok; [ "$test_exit_code" -eq 0 ] || test_status=fail
audit_result=ok; [ "$audit_status" -eq 0 ] || audit_result=fail
protected_path_status=ok; [ "$audit_result" = ok ] || protected_path_status=check-audit
main_delta=unknown
if git rev-parse origin/main >/dev/null 2>&1; then main_delta="$(git diff --shortstat origin/main...HEAD 2>/dev/null || printf unknown)"; fi
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then git diff --binary "${base_sha}"...HEAD >"$PATCH_FILE" 2>/dev/null || git diff --binary >"$PATCH_FILE" || true; else : >"$PATCH_FILE"; fi
python3 - "$ROOT_DIR" "$REPO_ZIP" <<'PY'
import zipfile,sys
from pathlib import Path
root=Path(sys.argv[1]).resolve(); out=Path(sys.argv[2]).resolve(); skip={'.git','__pycache__','.pytest_cache'}
if out.exists(): out.unlink()
with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED) as zf:
    for p in sorted(root.rglob('*')):
        if p.is_dir() or any(part in skip for part in p.parts): continue
        zf.write(p, p.relative_to(root).as_posix())
PY
structural_score=0
[ -f codex-wrapper.manifest.json ] && structural_score=$((structural_score+20)); [ -d lib/codex-termux ] && structural_score=$((structural_score+20)); [ "$audit_result" = ok ] && structural_score=$((structural_score+20)); [ "$test_status" = ok ] && structural_score=$((structural_score+20)); [ "$contract_status" = ok ] && structural_score=$((structural_score+20))
blockers=""; next_actions=""
add_blocker(){ blockers="${blockers}${blockers:+;}$1"; }
add_next_action(){ next_actions="${next_actions}${next_actions:+;}$1"; }
[ "$syntax_status" -eq 0 ] || { add_blocker 'syntax checks failed'; add_next_action 'fix shell syntax errors'; }
[ "$audit_result" = ok ] || { add_blocker 'canon-audit strict failed'; add_next_action 'inspect canon-audit.json and fix blockers'; }
[ "$test_status" = ok ] || { add_blocker 'tests failed'; add_next_action 'inspect test-logs and fix code/test gaps'; }
[ "$contract_status" = ok ] || { add_blocker 'compatibility contracts failed'; add_next_action 'fix wrapper-contracts.sh failures before refactoring further'; }
merge_readiness=not_ready
if [ -z "$blockers" ]; then merge_readiness=ready; add_next_action 'open or update PR'; fi
python3 - "$REPORT" "$head_sha" "$base_sha" "$branch" "$audit_result" "$audit_findings_count" "$test_status" "$test_exit_code" "$contract_status" "$protected_path_status" "$structural_score" "$main_delta" "$merge_readiness" "$created_at" "$run_url" "$blockers" "$next_actions" <<'PY'
import json,sys
keys=['report_path','head_sha','base_sha','branch','audit_status','audit_findings_count','test_status','test_exit_code','contract_status','protected_path_status','structural_score','main_delta','merge_readiness','created_at','run_url','blockers','next_actions']
v=dict(zip(keys,sys.argv[1:]))
report={k:v[k] for k in keys[1:-2]}
for k in ['audit_findings_count','test_exit_code','structural_score']: report[k]=int(report[k])
report['blockers']=[x for x in v['blockers'].split(';') if x]
report['next_actions']=[x for x in v['next_actions'].split(';') if x]
from pathlib import Path
Path(v['report_path']).write_text(json.dumps(report, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(json.dumps(report, sort_keys=True))
PY
cp "$REPORT" "$ROOT_DIR/merge_readiness_report.json"
[ "$merge_readiness" = ready ]
