# Refactor Subagent Orchestration Directive

이 문서는 `docs/tracking/ideal-wrapper-refactor-directive.md`를 실제로 수행할 때 사용할 subagent orchestration 지시서다. 사용자가 이 문서를 기준으로 작업 시작을 지시하면, main agent는 이 문서의 역할 배치와 phase gate에 따라 전체 작업을 총괄한다.

이 문서는 리팩터링 설계를 대체하지 않는다. 구현 기준은 항상 `ideal-wrapper-refactor-directive.md`가 우선이고, 이 문서는 누가 어떤 순서와 검증 방식으로 수행할지를 정한다.

## 0. 운영 원칙

1. Main agent는 항상 전체 책임자다. Subagent 산출물은 자동 승인하지 않는다.
2. 같은 phase 안에서도 destructive path, schema, transaction, public command surface는 main agent가 직접 검토한다.
3. Subagent는 독립적으로 큰 설계를 바꾸지 않는다. 설계 변경이 필요하면 main agent에게 근거를 보고하고 대기한다.
4. 여러 agent가 같은 파일을 동시에 수정하지 않는다. 파일 소유권은 task 단위로 main agent가 배정한다.
5. `lib/codex-termux-lib.sh`, `tools/codex_native/schemas.py`, `tools/codex_native/activation.py`, `tools/codex_native/store.py`, `tools/codex_native/registry.py`는 high-risk 파일이다. 이 파일의 최종 patch는 main agent가 직접 리뷰한다.
6. Implementer 작업은 Verification Agent의 독립 검증을 거친 뒤에만 main agent final review로 넘어간다.
7. Phase gate를 통과하지 못하면 다음 phase로 넘어가지 않는다.
8. Subagent의 작업 완료 기준은 "코드 작성"이 아니라 "지정된 검증 명령 통과 + 변경 요약 + residual risk 보고"다.
9. Verification Agent는 검증 중 production code를 수정하지 않는다. 결함을 발견하면 재현 절차와 수정 방향을 보고한다.
10. 사용자의 dirty worktree를 reset, clean, checkout으로 지우지 않는다.

## 1. 기본 Agent 배치

### 1.1 Main Agent

역할:

- 전체 phase 총괄
- phase 시작/종료 판단
- subagent task 분해와 파일 소유권 배정
- architecture, schema, transaction, destructive operation 최종 승인
- 테스트 실패 triage
- commit 단위 결정
- 사용자 보고

권장 모델:

- `GPT-5.5`

권장 reasoning:

- `xhigh`

Main agent가 직접 해야 하는 작업:

- Phase 0 planning과 guardrail 기준 확정
- Phase 1 schema fail-fast 정책 확정
- Phase 3 activation transaction 설계와 핵심 구현
- Phase 4 prune plan/apply 삭제 정책 확정
- 각 phase 최종 review
- 모든 commit 전 최종 검증

### 1.2 Main Implementer 1

기본 배치:

- 모델: `GPT-5.5`
- reasoning: `high` 또는 `xhigh`

적합 작업:

- schema, registry, state, store, activation, prune 같은 고위험 domain 구현
- rollback/fail-fast/atomic write 테스트
- 중복 embedded Python 제거
- CI guardrail 설계

부적합 작업:

- 대량 문서 정리만 하는 작업
- 단순 fixture copy-edit
- 이미 정해진 출력 문자열만 맞추는 저위험 작업

### 1.3 Main Implementer 2

기본 배치:

- 모델: `GPT-5.4`
- reasoning: `medium-high` 또는 `high`

적합 작업:

- migration module 구현
- doctor report/render 분리
- `codex use` selection 이동
- shell facade 축소
- integration test 보강
- Phase 5 이후의 구조 추종형 구현

부적합 작업:

- Phase 1의 schema policy 최종 결정
- Phase 3의 transaction core 단독 구현
- malformed registry/state fail-fast 정책 변경
- destructive prune policy 변경

### 1.4 Documentation Organizer

기본 배치:

- 모델: `GPT-5.4 mini`
- reasoning: `low-medium` 또는 `medium`

적합 작업:

- docs 업데이트
- checklist 동기화
- test fixture builder 정리
- `rg` 기반 중복/잔여 embedded Python 검색
- phase 완료 보고서 초안 작성
- changelog, status 문서 정리

부적합 작업:

- production transaction code 작성
- schema validator 작성
- destructive delete logic 작성
- public CLI contract 변경
- rollback/fail-fast 정책 결정

### 1.5 Verification Agent

기본 배치:

- 모델: `GPT-5.4`
- reasoning: `high`

고위험 phase 배치:

- 모델: `GPT-5.5`
- reasoning: `high` 또는 `xhigh`

저위험 문서/grep 검증 배치:

- 모델: `GPT-5.4 mini`
- reasoning: `medium`

역할:

- 구현자가 만든 diff를 독립적으로 리뷰한다.
- phase directive 위반을 찾는다.
- malformed JSON, rollback failure, prune delete, transaction ordering 같은 실패 경로를 재현한다.
- phase gate command를 독립 실행한다.
- 테스트가 내부 mocking이나 문자열 snapshot에 과하게 기대는지 확인한다.
- public command surface가 늘었는지 확인한다.
- shell embedded Python block, broad fallback, 신규 `|| true` 같은 구조 후퇴를 검색한다.
- 검증 보고서를 main agent에게 제출한다.

필수 투입 기준:

- 모든 phase gate 전에 반드시 별도 Verification Agent를 배정한다.
- production code가 바뀐 phase는 docs-only 검증으로 대체하지 않는다.
- schema, transaction, activation, prune, registry, state, public command surface가 포함되면 `GPT-5.5 high` 이상을 배정한다.
- 문서만 바뀐 phase라도 문서가 구현 계약을 주장하면 실제 command 또는 `rg` 기반 확인을 포함한다.
- 구현자가 실행한 테스트와 같은 명령을 재실행하는 데 그치지 않고, 최소 하나 이상의 failure path 또는 구조 위반 검색을 추가한다.
- Verification Agent가 `fail`을 보고하면 main agent는 phase gate를 통과시킬 수 없다. 단, 명확한 false positive인 경우 main agent가 근거와 재현 결과를 phase completion report에 남겨야 한다.
- Verification Agent가 `inconclusive`를 보고하면 환경 제약, 생략된 검증, 남은 위험을 phase completion report에 명시하고 사용자가 승인한 경우에만 다음 phase로 넘어간다.

독립성 규칙:

- Verification Agent는 구현자와 같은 task packet을 재사용하지 않는다. 별도 verification packet을 받는다.
- Verification Agent의 allowed files는 원칙적으로 read-only다. 예외적으로 임시 fixture를 만들 수 있지만 repo production file은 수정하지 않는다.
- Verification Agent는 구현자 보고를 사실로 받아들이지 않고 `git diff`, source, tests, directive를 직접 대조한다.
- Verification Agent는 "통과 여부"와 "수정 방향"만 보고한다. patch 작성, cleanup, formatting 정리는 구현자 또는 main agent가 맡는다.

금지:

- production code 직접 수정
- failing test를 고치기 위한 hotfix
- directive 변경
- phase scope 확대
- main agent 승인 없는 commit

적합 작업:

- diff audit
- failure-path test 설계
- 기존 tests 독립 실행
- `rg` 기반 구조 위반 검색
- edge-case 재현 script 실행
- phase completion 전 residual risk 정리

부적합 작업:

- schema validator 구현
- activation transaction 구현
- prune logic 수정
- migration/doctor production code 수정
- shell facade 직접 축소

## 2. 작업 적합도별 모델/Reasoning 선택표

| 작업 유형 | 기본 담당 | 모델 | Reasoning | Main 직접 검수 |
| --- | --- | --- | --- | --- |
| phase decomposition | Main | 5.5 | xhigh | 해당 없음 |
| import boundary/CI skeleton | Implementer 1 | 5.5 | high | 예 |
| schema SSOT 설계 | Main + Implementer 1 | 5.5 | xhigh | 예 |
| state/registry validator 구현 | Implementer 1 | 5.5 | high | 예 |
| malformed JSON fail-fast test | Implementer 1 | 5.5 | high | 예 |
| hashing/tree digest 이동 | Implementer 1 | 5.5 | high | 예 |
| immutable store publish | Implementer 1 | 5.5 | high | 예 |
| activation transaction core | Main | 5.5 | xhigh | 해당 없음 |
| activation failure tests | Implementer 1 | 5.5 | high | 예 |
| prune plan/apply | Main + Implementer 1 | 5.5 | xhigh | 예 |
| legacy migration module | Implementer 2 | 5.4 | high | 예 |
| doctor report/render | Implementer 2 | 5.4 | medium-high | 예 |
| `codex use` selection | Implementer 2 | 5.4 | medium-high | 예 |
| shell facade shrink | Implementer 2 | 5.4 | high | 예 |
| docs/status/contracts sync | Documentation Organizer | 5.4 mini | medium | 예 |
| fixture cleanup | Documentation Organizer | 5.4 mini | medium | 선택 |
| grep-based audit | Documentation Organizer | 5.4 mini | low-medium | 예 |
| phase verification, low risk | Verification Agent | 5.4 | high | 예 |
| phase verification, schema/transaction/prune | Verification Agent | 5.5 | high/xhigh | 예 |
| docs-only verification | Verification Agent | 5.4 mini | medium | 예 |
| final phase review | Main | 5.5 | xhigh | 해당 없음 |

Rule:

- destructive operation, transaction, schema, public contract가 포함되면 `5.5 high` 이상을 배정한다.
- 문서, grep audit, fixture 정리만 있으면 `5.4 mini medium`까지 낮출 수 있다.
- 5.4가 맡은 작업이 schema/transaction policy를 건드리게 되면 즉시 main agent에게 escalate한다.
- Verification Agent는 구현하지 않는다. 검증 중 즉시 고칠 수 있는 작은 문제를 발견해도 main agent에게 보고하고, 수정은 owner에게 되돌린다.

## 3. Phase별 Agent 배정

### Phase 0: Baseline과 Guardrail

Primary:

- Main Implementer 1

Reviewer:

- Main Agent

Verifier:

- Verification Agent: `GPT-5.4 high`

Support:

- Documentation Organizer

작업:

- `tools/codex_native/` skeleton
- `ci/check-structure.sh`
- `ci/check-python-imports.py`
- line/function/import boundary checks
- manager support copy 준비

Main Agent 직접 판단:

- threshold 값
- import direction table
- behavior change가 없는지

Verification Agent 확인:

- guardrail이 실제로 실패를 잡는지
- import boundary checker가 잘못된 import fixture를 거부하는지
- behavior change가 diff에 섞이지 않았는지
- phase gate가 local에서 재현되는지

Documentation Organizer 작업:

- 새 CI 사용법을 docs/status 또는 operations에 반영하는 초안
- phase 0 완료 체크리스트 작성

Gate:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
git diff --check
```

### Phase 1: Schema SSOT

Primary:

- Main Agent + Main Implementer 1

Support:

- Documentation Organizer

Verifier:

- Verification Agent: `GPT-5.5 high`

작업:

- `schemas.py`, `errors.py`, `atomic.py`, `state.py`, `registry.py`
- malformed state/registry fail-fast
- shell facade 전환

Main Agent 직접 구현 또는 pair:

- validator policy
- malformed registry 기존 파일 보존 정책
- schema migration 없음/있음 판단

Verification Agent 확인:

- malformed state/registry가 nonzero인지
- malformed registry가 덮어써지지 않는지
- schema number와 field validation이 `schemas.py` 밖에서 중복되지 않는지
- tests가 fixture 복붙으로 schema drift를 만들지 않는지

Documentation Organizer 작업:

- schema field table 문서화
- duplicate fixture JSON audit

Gate:

```bash
bash ci/check-structure.sh
bash tests/runtime-integrity.sh
bash tests/pointer-activation.sh
bash tests/transactional-update.sh
bash tests/doctor-contract.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 2: Hashing과 Immutable Store

Primary:

- Main Implementer 1

Reviewer:

- Main Agent

Verifier:

- Verification Agent: `GPT-5.4 high`

작업:

- `hashing.py`
- `store.py` publish/validate
- shell hash/store facade 축소
- duplicate `tree_digest` 제거

Main Agent 직접 확인:

- same id different content collision
- permission collision
- concurrent publish race

Verification Agent 확인:

- identical content concurrent publish가 안전한지
- different content concurrent publish가 collision으로 실패하는지
- symlink target collision이 거부되는지
- tree digest 구현이 한 곳만 남았는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/immutable-store.sh
bash tests/pointer-activation.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 3: Activation Transaction

Primary:

- Main Agent

Support:

- Main Implementer 1

Documentation:

- Documentation Organizer

Verifier:

- Verification Agent: `GPT-5.5 xhigh`

작업:

- `activation.py`
- `ActivationPlan`
- pointer/state/registry snapshot
- rollback aggregate result
- verified rollback 통합
- metadata refresh/bootstrap activation engine 사용

Main Agent 직접 구현:

- transaction order
- rollback failure handling
- cleanup failure policy
- readiness failure after pointer move

Implementer 1 지원:

- failure injection tests
- facade wiring
- repeated regression runs

Documentation Organizer:

- transaction invariants 문서 갱신

Verification Agent 확인:

- current/verified/raw pointer 변경 순서가 directive와 맞는지
- state/registry snapshot 복원이 실패할 때 nonzero인지
- rollback failure가 성공으로 숨겨지지 않는지
- cleanup failure 정책이 구현과 test에 반영됐는지
- old current, verified, raw, state, registry가 failure path에서 보존되는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/pointer-activation.sh
bash tests/pointer-rollback.sh
bash tests/verified-rollback.sh
bash tests/transactional-update.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 4: Prune Plan/Apply

Primary:

- Main Agent + Main Implementer 1

Verifier:

- Verification Agent: `GPT-5.5 high`

작업:

- prune dry-run plan
- apply step
- malformed registry/state no-delete
- pointer target protection

Main Agent 직접 확인:

- delete 대상 계산
- path outside managed store rejection
- registry rewrite order

Verification Agent 확인:

- malformed registry/state에서 삭제가 일어나지 않는지
- current/verified/raw pointer target이 registry와 무관하게 보호되는지
- plan에 없는 path가 삭제되지 않는지
- path outside managed store가 실패하는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/prune-pointer-protection.sh
bash tests/immutable-store.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 5: Legacy Migration

Primary:

- Main Implementer 2

Reviewer:

- Main Agent

Support:

- Documentation Organizer

Verifier:

- Verification Agent: `GPT-5.4 high`

작업:

- `migration.py`
- legacy store migration
- legacy runtime layout migration using activation engine
- migration report schema usage

Escalation 조건:

- registry parse failure 처리 변경 필요
- report write failure 정책 변경 필요
- activation engine 변경 필요

Verification Agent 확인:

- migration engine failure가 nonzero인지
- invalid legacy tuple skip과 engine failure가 구분되는지
- report write failure가 성공으로 보이지 않는지
- repeated migration idempotency가 유지되는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/legacy-store-migration.sh
bash tests/legacy-migration.sh
bash tests/installer-layout.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 6: Doctor Report/Render

Primary:

- Main Implementer 2

Reviewer:

- Main Agent

Support:

- Documentation Organizer

Verifier:

- Verification Agent: `GPT-5.4 high`

작업:

- `doctor_report.py`
- `doctor_render.py`
- shell doctor facade 축소
- upstream separator contract 유지

Documentation Organizer:

- operations/contracts doctor section 동기화

Verification Agent 확인:

- `codex doctor` 기본 출력 separator contract가 유지되는지
- `codex doctor --json` passthrough가 wrapper JSON으로 바뀌지 않았는지
- migration warning이 exit code를 잘못 실패로 만들지 않는지
- renderer가 filesystem mutation을 하지 않는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/doctor-contract.sh
TERM=xterm-256color codex doctor
for f in tests/*.sh; do bash "$f"; done
```

### Phase 7: `codex use` Selection

Primary:

- Main Implementer 2

Reviewer:

- Main Agent

Verifier:

- Verification Agent: `GPT-5.4 high`

작업:

- `registry.py` runtime listing/selection
- duplicate managed path validators 제거
- shell `codex_use_render`, `codex_use_select` facade 축소

Verification Agent 확인:

- invalid runtime/raw path가 list/selection에서 제외되는지
- active/latest row behavior가 유지되는지
- `managed_runtime_path`/`managed_raw_path` 구현 중복이 제거됐는지

Gate:

```bash
bash ci/check-structure.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

### Phase 8: Shell Facade Shrink

Primary:

- Main Implementer 2

Reviewer:

- Main Agent

Support:

- Documentation Organizer

Verifier:

- Verification Agent: `GPT-5.4 high`

작업:

- embedded Python block 제거
- shell function size 축소
- process/env/fd33/profile/CLI routing만 shell에 유지

Main Agent 직접 확인:

- public command surface 불변
- upstream exec behavior 불변
- profile config 비변경

Verification Agent 확인:

- shell에 embedded Python block이 새로 남아 있지 않은지
- shell function size threshold를 통과하는지
- fd33/env/profile execution behavior가 유지되는지
- wrapper internal CLI가 user-facing으로 노출되지 않았는지

Gate:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
TERM=xterm-256color codex doctor
```

### Phase 9: Test Refactor와 Documentation Sync

Primary:

- Documentation Organizer

Reviewer:

- Main Agent

Implementation Support:

- Main Implementer 2

Verifier:

- Verification Agent: `GPT-5.4 mini medium` for docs-only audit, `GPT-5.4 high` if tests changed

작업:

- fixture builder 정리
- internal shell function override 감소
- docs/status/contracts/operations/security/architecture 동기화
- final audit

Main Agent 직접 확인:

- test가 behavior/side-effect 중심인지
- 문서가 구현과 어긋나지 않는지
- residual risk가 명확한지

Verification Agent 확인:

- docs와 contracts가 실제 command behavior와 일치하는지
- internal mocking 의존도가 줄었는지
- final grep audit 결과가 directive와 일치하는지

Gate:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
TERM=xterm-256color codex doctor
git diff --check
```

## 4. Subagent Task Packet 형식

Main agent는 subagent에게 작업을 맡길 때 아래 형식을 사용한다.

```text
Task ID:
Phase:
Assigned role:
Model:
Reasoning:

Read first:
- docs/tracking/ideal-wrapper-refactor-directive.md
- docs/tracking/refactor-subagent-orchestration.md
- relevant AGENTS.md files
- relevant source/test files

Allowed files:
- ...

Do not edit:
- ...

Objective:
- ...

Required invariants:
- ...

Implementation steps:
1. ...
2. ...

Required tests:
- ...

Report back with:
- changed files
- exact behavior changed
- tests run and result
- risks/open questions
- any deviation from directive
```

Subagent에게 절대 주면 안 되는 지시:

- "전체 리팩터링을 알아서 해"
- "테스트 깨지면 적당히 고쳐"
- "문서 보고 필요한 것 아무거나 해"
- "파일 구조 마음대로 정리해"

Verification Agent에게는 추가로 아래 지시를 주면 안 된다.

- "발견한 문제를 직접 고쳐"
- "검증하면서 production도 같이 정리해"
- "테스트를 통과하도록 임시 fallback을 넣어"
- "구현자 보고 없이 작은 수정은 알아서 해"

## 5. Subagent Report 형식

Subagent는 완료 시 아래 형식으로 보고해야 한다.

```text
Task ID:
Status: complete | blocked | partial

Changed files:
- ...

Behavior changes:
- ...

Directive compliance:
- followed:
- deviations:

Tests run:
- command: result

Residual risks:
- ...

Needs main-agent decision:
- ...
```

Main agent는 이 보고만 믿지 않고 `git diff`, targeted tests, relevant source를 직접 확인한다.

## 6. Verification Agent Report 형식

Verification Agent는 구현 보고서와 별도로 아래 형식으로 보고한다.

```text
Verification Task ID:
Phase:
Status: pass | fail | inconclusive
Model:
Reasoning:

Scope reviewed:
- files:
- commands:
- directive sections:

Findings:
- severity:
  file/line:
  issue:
  reproduction:
  directive violated:

Tests run:
- command: result

Structural checks:
- public command surface:
- import boundary:
- embedded Python:
- broad fallback:
- duplicate schema/helper:
- destructive operation policy:

Residual risks:
- ...

Recommendation:
- pass phase gate | block phase gate | requires main decision
```

Rules:

- `pass`는 required tests와 structural checks가 모두 만족될 때만 사용한다.
- `inconclusive`는 환경 문제나 tool 부재 때문에 검증을 끝내지 못했을 때만 사용한다.
- severity가 high인 finding이 있으면 phase gate는 block이다.

## 7. File Ownership Lock

한 task 동안 파일 소유권을 명시한다.

High-risk ownership examples:

| File | Default owner | Concurrent edits |
| --- | --- | --- |
| `lib/codex-termux-lib.sh` | Main Agent | 금지 |
| `tools/codex_native/schemas.py` | Main Agent / Implementer 1 | 금지 |
| `tools/codex_native/registry.py` | Implementer 1 | 금지 |
| `tools/codex_native/store.py` | Implementer 1 | 금지 |
| `tools/codex_native/activation.py` | Main Agent | 금지 |
| `tools/codex_native/migration.py` | Implementer 2 | schema/activation 동시 변경 금지 |
| `tools/codex_native/doctor_report.py` | Implementer 2 | schema 동시 변경 금지 |
| `tools/codex_native/doctor_render.py` | Implementer 2 | 허용, report schema 고정 |
| `docs/*` | Documentation Organizer | production file 동시 변경 없음 |
| `tests/*` | assigned by phase | 같은 test file 동시 변경 금지 |

충돌 처리:

1. Main agent가 현재 diff를 확인한다.
2. 둘 중 더 작은 patch를 rebase-like 수동 반영한다.
3. test를 다시 실행한다.
4. 충돌 원인을 phase report에 남긴다.

## 8. Main Agent Review Checklist

Subagent patch를 받은 뒤 main agent는 반드시 확인한다.

Architecture:

- [ ] import direction 위반 없음
- [ ] 새 public command 없음
- [ ] shell에 새 embedded Python block 없음
- [ ] `utils.py`, `helpers.py`, `common.py` 같은 무소유 module 없음
- [ ] schema constant가 `schemas.py` 밖에 새로 생기지 않음

Error policy:

- [ ] malformed JSON 자동 초기화 없음
- [ ] broad exception이 허용 경계 안에 있음
- [ ] rollback/cleanup 실패 무시 없음
- [ ] destructive operation 전 검증 있음

Tests:

- [ ] edge case가 happy path보다 먼저 고정됨
- [ ] side effect 검증 있음
- [ ] 내부 mocking이 줄거나 정당화됨
- [ ] required phase tests 통과

Diff hygiene:

- [ ] unrelated formatting churn 없음
- [ ] dead/commented-out code 없음
- [ ] user changes reset 없음
- [ ] docs가 실제 구현과 맞음

Verification:

- [ ] Verification Agent report가 있음
- [ ] report status가 pass 또는 main-approved inconclusive임
- [ ] high severity finding이 없음
- [ ] verification commands를 main이 spot-check함

## 9. Escalation Rules

Subagent는 아래 상황에서 작업을 멈추고 main agent 결정을 요청한다.

- directive와 기존 구현이 충돌한다.
- public command surface 변경이 필요해 보인다.
- malformed registry/state를 자동 repair하고 싶어진다.
- activation transaction order를 바꿔야 한다.
- prune가 protected pointer target을 삭제해야만 test가 통과한다.
- doctor exit code 규칙을 바꿔야 할 것 같다.
- `codex doctor --json` passthrough를 바꿔야 할 것 같다.
- 같은 파일을 다른 agent가 수정 중이다.
- test 통과를 위해 production에 무의미한 fallback을 넣어야 할 것 같다.

Verification Agent는 아래 상황에서 phase gate block을 권고한다.

- malformed registry/state가 자동 초기화된다.
- rollback/cleanup failure가 성공으로 숨겨진다.
- prune가 malformed registry/state에서 삭제한다.
- protected pointer target이 삭제된다.
- public command surface가 증가했다.
- `codex doctor --json` passthrough가 깨졌다.
- schema/helper 중복이 새로 생겼다.
- tests가 directive 필수 edge case를 검증하지 않는다.

## 10. Phase Completion Report

각 phase 종료 시 main agent가 작성한다.

```text
Phase:
Status:

Implemented:
- ...

Files changed:
- ...

Invariants verified:
- ...

Tests:
- ...

Verification:
- agent:
- status:
- findings:

Directive deviations:
- none | ...

Residual risks:
- ...

Next phase readiness:
- ready | blocked
```

## 11. Commit/Push 운영

Main agent만 commit을 만든다.

Commit 전:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
git diff --check
git status --short
```

Commit message format:

```text
Refactor native wrapper <phase subject>
```

Examples:

```text
Refactor native wrapper guardrails
Refactor native wrapper schemas
Refactor native wrapper activation transaction
```

Push는 사용자가 요청했거나 현재 작업 흐름에서 명시적으로 요구된 경우에만 한다.

## 12. 작업 시작 Procedure

사용자가 "이 문서대로 시작" 또는 같은 의미의 지시를 하면 main agent는 아래 순서로 진행한다.

1. `git status --short --branch` 확인
2. `docs/tracking/ideal-wrapper-refactor-directive.md`와 이 문서 재확인
3. 현재 dirty worktree가 이전 의도된 변경인지 확인
4. Phase 0 task packet 생성
5. 필요하면 Main Implementer 1 subagent 배정
6. Verification Agent task packet도 함께 준비
7. Main agent가 Phase 0 파일 ownership lock 선언
8. Phase 0 구현
9. Implementer report 수집
10. Verification Agent 독립 검증
11. Main agent final review
12. Phase 0 gate 실행
13. Phase completion report 작성
14. 사용자에게 요약 보고

Phase 0이 완료되기 전에는 Phase 1 구현을 시작하지 않는다.
