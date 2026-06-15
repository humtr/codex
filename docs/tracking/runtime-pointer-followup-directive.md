# Runtime Pointer Follow-up Directive

이 문서는 5.4 medium이 현재 5.5 작업을 이어 받아 마무리할 실행 지시서다. 방향을 다시 설계하는 문서가 아니다. 아래 순서와 완료 기준대로 구현하고, 기존 5.5 핵심 transaction을 단순화하거나 재작성하지 않는다.

## 시작 상태와 작업 경계

재진단 시점의 기준은 다음과 같다.

- Branch: `main`
- Base commit: `852fc97 Introduce runtime pointer activation`
- `main`과 `origin/main`은 일치한다.
- 아래 변경은 5.5가 완료한 의도된 미커밋 작업이다. 절대 reset, checkout, clean, 삭제하지 않는다.
  - 수정: `lib/codex-termux-lib.sh`
  - 수정: `tests/pointer-activation.sh`
  - 신규: `tests/immutable-store.sh`
  - 신규: `tests/legacy-store-migration.sh`
  - 신규: `tests/pointer-rollback.sh`
  - 신규: `tests/prune-pointer-protection.sh`
  - 수정: 이 지시서
- 5.5 완료 시점에 아래 검증은 통과했다.
  - `bash -n lib/codex-termux-lib.sh bin/install-runtime.sh install.sh tests/*.sh`
  - `git diff --check`
  - `for f in tests/*.sh; do bash "$f"; done`

5.4가 production code에서 수정해도 되는 핵심 범위는 다음 두 곳뿐이다.

1. `tools/codex-launcher.c`의 잘못된 compiled launcher 기본 target 수정
2. `lib/codex-termux-lib.sh`의 wrapper doctor human renderer 보강

그 밖의 production transaction, store, migration, activation, rollback, prune 함수는 새 테스트가 실제 결함을 증명하지 않는 한 수정하지 않는다.

## 5.5가 이미 완성한 기틀

- Support file 소유자는 `native/manager`다.
- `native/store/runtime/<tuple>`과 `native/store/raw/<tuple>`은 immutable artifact다.
- `native/current`, `native/verified`, `native/raw`는 각각 active runtime, last-known-good runtime, active raw cache pointer다.
- 동일 store tuple은 전체 tree 내용과 실행 권한이 동일할 때만 재사용한다. 충돌 시 기존 artifact를 보존하고 activation을 거부한다.
- Prune는 registry/state와 별개로 실제 `current`, `verified`, `raw` pointer target을 직접 보호한다.
- Verified/raw pointer 교체 실패 시 current, verified, raw, state, registry를 rollback한다.
- 기존 `$CODEX_NATIVE_STATE_DIR/store` cache는 현재 builder, patch policy, support shim, raw/runtime hash, DNS-only patch가 모두 맞는 tuple만 새 store로 이관한다.
- Legacy runtime/cache migration, verified rollback, metadata refresh, bootstrap mutation은 공통 lock 경계를 사용한다.
- Wrapper doctor JSON schema 4는 manager/current/verified/raw/store 경로, pointer/store/registry 일치 check, migration report 계약을 이미 제공한다.

## 반드시 유지할 불변식

- 사용자가 알아야 하는 명령은 계속 `codex setup`, `codex update`, `codex use`, `codex doctor`, `codex version`, `codex remove`뿐이다.
- `bin/install-runtime.sh support|setup|update|doctor|remove`는 설치·개발용 표면이다. 새 public subcommand나 wrapper 전용 doctor option을 추가하지 않는다.
- Runtime tuple에는 upstream payload와 runtime-private `codex-path/bwrap`, `codex-path/rg`만 둔다. Manager script를 runtime tuple에 다시 넣지 않는다.
- Store artifact는 publish 후 수정하지 않는다. 새 activation은 artifact publish 후 pointer와 metadata를 바꾸는 방식이어야 한다.
- State/registry와 current/verified/raw pointer 변경은 실패 시 rollback 가능해야 한다.
- Upstream raw binary 원본성과 DNS-only same-length patch 정책을 유지한다.
- Profile config, auth, approval policy, network policy를 wrapper가 수정하지 않는다.
- Public doctor의 upstream passthrough와 upstream/wrapper 사이의 흰색 구분선은 바꾸지 않는다.

## 재진단으로 확인한 실제 결함

`tools/codex-launcher.c`의 compiled launcher 기본 target이 아직 아래 legacy 경로다.

```text
.local/lib/codex/native/runtime/managed.sh
```

현재 manager 소유권 모델의 올바른 기본 target은 아래다.

```text
.local/lib/codex/native/manager/managed.sh
```

Shell launcher는 이미 `CODEX_NATIVE_MANAGED_SHELL`을 사용하므로 정상이다. Compiled launcher가 환경 변수 `CODEX_NATIVE_MANAGED_SHELL` 없이 실행될 때도 manager shell로 들어가도록 반드시 수정하고 회귀 테스트한다.

## 작업 순서

아래 순서를 지킨다. 각 단계가 끝날 때 targeted test를 실행하고, 마지막에 전체 test를 실행한다.

### 1. Compiled launcher 기본 target 수정

수정 파일:

- `tools/codex-launcher.c`
- 새 installer E2E test 파일. 권장 이름은 `tests/installer-layout.sh`다.

구현 지시:

- `tools/codex-launcher.c`의 `default_managed` 상대 경로를 정확히 `.local/lib/codex/native/manager/managed.sh`로 바꾼다.
- `CODEX_NATIVE_MANAGED_SHELL` 환경 변수가 명시되면 이를 우선하는 기존 계약은 유지한다.
- Marker 문자열, bash 선택, argv 전달, exit behavior는 바꾸지 않는다.

필수 회귀 검증:

- Source에 새 manager 경로가 있고 legacy `native/runtime/managed.sh` 경로가 없는지 항상 검사한다.
- `clang`을 사용할 수 있으면 compiled launcher를 실제로 빌드하고 실행한다.
  - 임시 `HOME` 아래 `native/manager/managed.sh`는 성공 marker를 출력하게 만든다.
  - legacy `native/runtime/managed.sh`는 실행되면 실패하도록 만든다.
  - `CODEX_NATIVE_MANAGED_SHELL`은 unset하고 `CODEX_NATIVE_BASH`만 실제 bash 경로로 지정한다.
  - 실행 결과가 manager shell의 marker와 전달된 인자를 포함하는지 확인한다.
- 기존 `tests/launcher-transaction.sh`의 build failure, marker validation failure, final rename rollback 계약은 그대로 통과해야 한다.

### 2. Wrapper doctor human output에 schema 4 반영

수정 파일:

- `lib/codex-termux-lib.sh`
- `tests/doctor-contract.sh`

수정 경계:

- `codex_wrapper_doctor_json`의 schema, key, check 계산, `overallStatus` 의미는 바꾸지 않는다.
- `codex_wrapper_doctor()` 안의 human renderer만 schema 4의 기존 값을 소비하도록 보강한다.
- `codex_public_doctor()`의 option passthrough, exit aggregation, 공백행, 흰색 구분선은 바꾸지 않는다.
- Wrapper header 아래에 새 구분선을 추가하지 않는다.
- 기존 color helper와 색상 의미를 재사용한다. 새 임의 색상 체계를 만들지 않는다.

추가할 human section과 row는 아래처럼 고정한다.

```text
Storage
  manager
  current
  verified
  raw cache
  stores
  alignment

Migration
  legacy store
```

각 row의 판정은 아래 schema 4 check를 집계한다.

| Human row | `ok` 조건 |
| --- | --- |
| `manager` | `checks.manager` |
| `current` | `checks.current_pointer && checks.current_in_store && checks.registry_current_match` |
| `verified` | `checks.verified_pointer && checks.verified_in_store && checks.registry_verified_match` |
| `raw cache` | `checks.raw_pointer && checks.raw_in_store` |
| `stores` | `checks.runtime_store && checks.raw_store` |
| `alignment` | `checks.current_verified_match` |

Path detail은 아래 값을 보여 준다.

- `manager`: `paths.manager`
- `current`: link=`paths.current`, target=`paths.current_target`
- `verified`: link=`paths.verified`, target=`paths.verified_target`
- `raw cache`: link=`paths.raw`, target=`paths.raw_target`
- `stores`: runtime=`paths.runtime_store`, raw=`paths.raw_store`
- `alignment`: active tuple=`activeTupleId`, verified tuple=`verifiedTupleId`

Migration row는 `migration.status`를 다음처럼 표현한다.

| `migration.status` | Human status | Wrapper exit 영향 |
| --- | --- | --- |
| `not-needed` | `idle` | 없음 |
| `completed`이고 skipped/error 없음 | `ok` | 없음 |
| `pending` | `warn` | 없음 |
| `issues` | `warn` | 없음 |
| 알 수 없는 값 | `warn` | 없음 |

Migration detail은 최소한 아래를 보여 준다.

- report=`migration.report`
- legacy store=`migration.legacyStore`
- imported count=`len(migration.imported)`
- skipped count=`len(migration.skipped)`
- error가 있으면 한 줄의 간결한 error summary

`pending`, `issues`, 알 수 없는 migration status는 `Notes`에도 migration warning 한 줄을 추가한다. 기본 human 출력에 전체 skipped object나 전체 report JSON을 dump하지 않는다.

중요한 종료 코드 규칙:

- Human summary가 migration warning 때문에 `degraded`여도 `data.overallStatus == "ok"`이면 `codex_wrapper_doctor` exit code는 0이다.
- 필수 check 실패로 `data.overallStatus == "fail"`이면 exit code는 non-zero다.
- 이 규칙을 바꾸기 위해 JSON `overallStatus` 계산을 수정하지 않는다.

Summary row count는 위 고정 row 구성을 기준으로 테스트한다.

- schema 4 healthy + migration `not-needed`:
  - `23 ok · 1 idle · 0 warn · 0 fail ok`
- schema 4 healthy + migration `completed`:
  - `24 ok · 0 idle · 0 warn · 0 fail ok`
- schema 4 healthy + migration `issues` 또는 `pending`:
  - `23 ok · 0 idle · 1 warn · 0 fail degraded`
  - exit code는 0

`tests/doctor-contract.sh` 필수 case:

1. 완전한 schema 4 healthy fixture
   - Storage와 Migration section이 렌더링된다.
   - manager/current/verified/raw/store path detail이 보인다.
   - migration `not-needed` exact summary가 맞다.
2. migration `issues` fixture
   - Notes에 migration warning이 보인다.
   - Migration row가 warn이다.
   - exact degraded summary가 맞다.
   - wrapper doctor exit code는 0이다.
3. broken current pointer fixture
   - `current` row가 fail이다.
   - `overallStatus=fail`이면 wrapper doctor exit code가 non-zero다.
4. 기존 public doctor contract
   - 인자 없는 호출은 upstream human output, 정확한 공백행/구분선, wrapper output 순서다.
   - `codex doctor --json` 등 인자가 있는 호출은 upstream에 그대로 전달된다.

Command substitution에서는 TTY color가 비활성화되므로 test가 ANSI escape에 의존하지 않게 한다. Live 확인에서는 기존 upstream-style 색상과 정렬이 유지되는지 직접 확인한다.

### 3. Installer layout E2E 추가

권장 파일:

- `tests/installer-layout.sh`

기존 `tests/runtime-bwrap-path.sh`, `tests/legacy-migration.sh`, `tests/install-verification.sh`, `tests/launcher-transaction.sh`의 역할을 중복 구현하거나 약화하지 않는다. 새 test는 command orchestration과 최종 layout을 검증한다.

Fixture 공통 규칙:

- 모든 test는 임시 `HOME`, `PREFIX`, native root, state root를 사용한다.
- 환경 변수는 `bin/install-runtime.sh`를 source하기 전에 설정한다.
- Source 후 필요한 함수를 override하고 `main support` 또는 `main setup`을 호출해 command dispatch까지 검증한다.
- 외부 live 설치를 건드리지 않는다.
- 실패 sentinel을 사용해 fetch/update/repair 경로가 호출되지 않았음을 증명한다.

#### Case A: `support`는 manager와 launcher만 갱신

- `codex_launcher_available() { return 1; }`로 shell launcher를 강제해 내용을 검사한다.
- `codex_update`, `codex_fetch_package`, `codex_repair_runtime_from_raw`, `npm`, `curl`은 호출 시 sentinel을 기록하고 실패하도록 override한다.
- `main support` 실행 후 다음을 assert한다.
  - `native/manager/managed.sh`, `lib.sh`, `build-runtime.py`, `bwrap-termux-compat.py`, `rg-termux-shim.sh`, `wrapper-version.env`가 존재한다.
  - 실행 파일과 읽기 파일의 permission 계약이 맞다.
  - `managed.sh`가 `native/manager/lib.sh`를 source한다.
  - Public shell launcher에 managed marker가 있고 `native/manager/managed.sh`를 exec한다.
  - `native/current`, `native/verified`, `native/raw`, `native/store`는 생성되지 않는다.
  - 외부 public `$PREFIX/bin/bwrap`이 fixture에 있었다면 hash가 변하지 않는다.
  - network/fetch/update/repair sentinel은 생성되지 않는다.

#### Case B: `setup`은 유효한 legacy runtime을 network 없이 이관

- `tests/legacy-migration.sh`의 valid legacy runtime/raw fixture 원리를 재사용하되, 새 helper abstraction을 만들기 위해 production code를 바꾸지 않는다.
- `codex_launcher_available() { return 1; }`로 shell launcher를 사용한다.
- `codex_update`, `codex_fetch_package`, `codex_repair_runtime_from_raw`, `npm`, `curl`은 호출 시 sentinel을 기록하고 실패하도록 override한다.
- `main setup` 실행 후 다음을 assert한다.
  - update/fetch/repair/network sentinel이 없다.
  - `native/manager` support file과 public launcher가 정상이다.
  - `native/current`, `native/verified`, `native/raw`는 symlink다.
  - 각 symlink target은 각각 `native/store/runtime` 또는 `native/store/raw`의 직접 자식이다.
  - `current`와 `verified` target은 일치한다.
  - state와 registry의 active/verified tuple 및 runtime/raw path가 실제 pointer target과 일치한다.
  - `codex_runtime_ok`가 성공한다.
  - legacy runtime directory는 보존된다.

#### Case C: compiled launcher 기본 manager target

- 1단계의 compiled launcher 회귀 검증을 같은 test 파일에 포함해도 된다.
- 최소한 source-level old/new path assertion은 모든 환경에서 실행한다.
- `clang` 사용 가능 환경에서는 실제 executable behavior까지 검증한다.

### 4. 문서 정합성 sweep

문서 변경은 현재 구현을 설명해야 하며 미래 설계를 추가하지 않는다.

#### `docs/operations.md`

- `support`가 runtime support script가 아니라 `native/manager` support file과 public launcher만 갱신하며 network fetch를 하지 않는다고 명시한다.
- `update`를 “atomic promotion” 한 문장으로 뭉개지 말고 immutable artifact publish, smoke test, current/verified/raw pointer activation, metadata 기록, 실패 시 rollback 순서로 설명한다.
- Runtime 선택은 registry path가 각각의 store 직접 자식이고 검증 가능한 artifact여야 한다고 설명한다.
- 검증 명령에 새 5.5 test와 `tests/installer-layout.sh`를 포함한다.
- Doctor 설명에 schema 4 manager/store/pointer/registry/migration check를 포함한다.
- `remove`가 managed native root의 manager/store/pointer를 제거하고 state directory를 보존한다고 명시한다.
- 주요 환경 변수에 manager/store/current/verified/legacy store migration 관련 변수를 추가한다.

#### `docs/security.md`

- Immutable store의 기존 tuple은 rewrite하지 않으며 collision을 거부한다고 명시한다.
- Prune가 metadata drift와 별개로 실제 pointer target을 보호한다고 명시한다.
- Pointer와 metadata activation 실패 시 rollback한다고 명시한다.
- Legacy store migration report에는 auth/config/token을 기록하지 않는다고 명시한다.
- Termux bwrap compat가 namespace security boundary가 아니라는 기존 설명은 유지한다.

#### `docs/contracts.md`

- `codex setup`은 manager support 설치, launcher 설치, legacy migration, healthy/migratable runtime이 없을 때만 fetch/repair한다는 계약을 명시한다.
- `codex update`는 immutable publish와 current/verified/raw pointer 및 metadata의 rollback-safe activation으로 설명한다.
- `codex doctor`는 public option passthrough를 유지하면서 개발자용 wrapper JSON schema 4와 human migration warning 의미를 설명한다.
- Migration `pending/issues`만으로 wrapper doctor가 실패하지 않는다고 명시한다.
- `codex use`는 cached artifact pointer activation으로 설명한다.
- `codex remove`는 manager/store/pointer를 포함한 managed native root 제거로 설명한다.

#### `docs/business-rules.md`

- 기존 “candidate staging and rollback-safe swap” 문장을 immutable artifact publish + rollback-safe pointer/metadata activation 규칙으로 교체한다.
- 기존 tuple은 rewrite하지 않고 content/permission collision을 거부한다고 명시한다.
- Prune는 state/registry뿐 아니라 실제 pointer target을 보호해야 한다고 명시한다.
- Legacy cache migration은 best-effort이며 invalid/missing raw cache skip이 active runtime을 막지 않는다고 명시한다.

#### `docs/engineering-notes.md`

아래 troubleshooting 항목을 추가한다.

- Pointer는 건강하지만 registry path가 drift한 경우 readiness가 metadata refresh로 복구하는 방식
- Legacy store migration report가 `issues`가 되는 대표 원인과 active runtime 성공 여부와의 관계
- Immutable store tuple collision이 발생했을 때 기존 artifact를 보존하고 새 activation을 거부하는 이유

#### `docs/tracking/status.md`

- Immutable store collision, pointer rollback, pointer-target prune protection, legacy store migration, installer layout test를 구현/검증 목록에 반영한다.
- Doctor human detail과 문서 정합성 TODO가 완료되면 “남은 일”에서 제거한다.
- 실제로 남은 범위만 유지한다.

#### `docs/architecture.md`

- 이미 pointer/store 구조를 대체로 정확히 설명한다. 위 문서와 충돌하는 표현만 최소 수정한다.
- 전면 rewrite하지 않는다.

### 5. Test readability 판단

기존 5.5 test의 중복 fixture 정리는 완료 필수 작업이 아니다. 기본적으로 건드리지 않는다.

- 새 installer E2E를 작성하면서 아주 작은 test-local helper가 명확히 중복을 줄이는 경우에만 같은 test 파일 안에서 사용한다.
- 공용 test framework, production helper, transaction wrapper를 새로 만들지 않는다.
- `tests/immutable-store.sh`, `tests/prune-pointer-protection.sh`, `tests/pointer-rollback.sh`, `tests/legacy-store-migration.sh`, `tests/pointer-activation.sh`의 assertion 의미와 failure injection을 보존한다.

## 예상 변경 파일

필수:

- `tools/codex-launcher.c`
- `lib/codex-termux-lib.sh`
- `tests/doctor-contract.sh`
- `tests/installer-layout.sh` 또는 동등한 단일 installer E2E test
- `docs/operations.md`
- `docs/security.md`
- `docs/contracts.md`
- `docs/business-rules.md`
- `docs/engineering-notes.md`
- `docs/tracking/status.md`

조건부:

- `docs/architecture.md`: 실제 충돌 표현이 있을 때만 최소 수정
- 기존 launcher/installer test: 새 test와 명백히 겹치지 않는 보강이 필요할 때만 수정

수정 금지에 가까운 파일:

- 5.5가 완성한 immutable publish, activation/rollback, prune, migration production 함수
- 5.5 신규 핵심 tests
- Profile/auth/network approval 관련 코드

## 금지 사항

- Upstream Codex binary에 새 patch를 추가하지 않는다.
- `codex doctor --wrapper-*` 같은 public option이나 subcommand를 추가하지 않는다.
- Wrapper doctor JSON schema 4 key를 rename/remove하거나 check 의미를 약화하지 않는다.
- Migration warning을 숨기기 위해 migration report를 삭제하거나 JSON overall status를 조작하지 않는다.
- Compiled launcher 문제를 환경 변수 강제 설정으로 우회하지 않는다. 기본 경로 자체를 고친다.
- `codex_publish_immutable_tree`, pointer activation/rollback, prune 보호, migration 검증 기준을 단순화하지 않는다.
- 비관리형 `$PREFIX/bin/codex` backup 정책과 public `$PREFIX/bin/bwrap` 비소유권 규칙을 약화하지 않는다.
- 기존 dirty 5.5 변경을 되돌리거나 별도 임시 stash로 숨기지 않는다.

## 구현 중 검증 순서

각 단계 후:

```bash
bash -n lib/codex-termux-lib.sh bin/install-runtime.sh tests/doctor-contract.sh tests/installer-layout.sh
bash tests/doctor-contract.sh
bash tests/installer-layout.sh
bash tests/launcher-transaction.sh
git diff --check
```

전체 구현 후:

```bash
bash -n lib/codex-termux-lib.sh bin/install-runtime.sh install.sh tests/*.sh
git diff --check
for f in tests/*.sh; do bash "$f"; done
```

Live Termux 검증은 repository test가 모두 통과한 뒤 수행한다.

```bash
bash bin/install-runtime.sh setup
codex version
codex doctor
bash bin/install-runtime.sh doctor --json
bash tests/network-boundary.sh
```

Live 검증 판정:

- Public compiled launcher가 환경 변수 강제 없이 `native/manager/managed.sh`를 통해 동작해야 한다.
- Wrapper doctor JSON은 schema 4이고 필수 pointer/store/registry check가 true여야 한다.
- Legacy store migration이 실제 환경에서 `issues`여도 active runtime과 필수 check가 건강하면 wrapper human doctor는 degraded warning과 exit 0을 반환해야 한다.
- Upstream doctor의 WebSocket warning은 wrapper 결함과 분리해 보고한다.
- Sandbox가 live 설치 쓰기나 network를 막으면 승인 요청으로 재실행하고, sandbox 실패와 wrapper 실패를 구분한다.

## 완료 전 리뷰 체크리스트

1. `git status --short`에서 기존 5.5 변경이 모두 남아 있고 예상 파일만 추가 수정됐는지 확인한다.
2. `git diff -- tools/codex-launcher.c lib/codex-termux-lib.sh tests/doctor-contract.sh tests/installer-layout.sh`를 직접 읽는다.
3. Doctor JSON schema 4가 production diff에서 불필요하게 바뀌지 않았는지 확인한다.
4. Public doctor separator와 argument passthrough exact test가 유지되는지 확인한다.
5. Installer E2E가 단순 함수 존재 검사가 아니라 `main support`와 `main setup` orchestration을 실제로 실행하는지 확인한다.
6. Compiled launcher test가 legacy path를 확실히 잡는지 확인한다.
7. 문서가 `native/runtime`을 active layout으로 설명하지 않는지 `rg`로 확인한다.
8. 전체 tests와 live 검증 결과를 기록한다.

## 커밋과 보고

모든 완료 기준을 통과한 뒤에만 기존 5.5 변경과 이번 5.4 마무리 변경을 함께 검토하고 커밋한다. 사용자 변경이나 예상 밖 파일이 보이면 임의로 포함하거나 되돌리지 말고 먼저 보고한다.

최종 보고에는 아래를 포함한다.

- Compiled launcher legacy target 결함 수정 여부
- Doctor human schema 4 반영 및 migration warning/exit semantics
- 새 installer E2E가 증명하는 `support`와 `setup` 무네트워크 경로
- 문서 정합성 변경 요약
- 전체 test 결과와 live Termux 결과
- 남은 위험 또는 실행하지 못한 검증
