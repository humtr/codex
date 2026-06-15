# Ideal Wrapper Refactor Directive

이 문서는 Codex Termux wrapper를 이상적 구조로 리팩터링하기 위한 실행 지시서다. 구현자가 초보여도 방향을 임의로 바꾸지 않도록, 저장 위치, 모듈 경계, 함수 축출 근거, 파편화 방지 규칙, 검증 게이트, 금지사항을 모두 고정한다.

이 문서는 제안서가 아니다. 아래 규칙을 어기면 리팩터링 실패로 본다.

## 0. 최우선 원칙

1. 사용자가 아는 public command는 계속 `codex`, `codex setup`, `codex update`, `codex use`, `codex doctor`, `codex version`, `codex remove`뿐이다.
2. Wrapper 내부 구현을 Python package로 옮겨도 사용자에게 새 wrapper subcommand를 노출하지 않는다.
3. Public `$PREFIX/bin/codex`에는 runtime 본체를 두지 않는다. public launcher는 manager shell로 넘기는 얇은 진입점이다.
4. Upstream raw binary는 공식 `@openai/codex` Linux ARM64 npm package에서 온 원본이어야 한다.
5. Runtime binary는 raw binary에 DNS fd33 same-length patch를 적용하고 support tool을 결합한 산출물이어야 한다.
6. Raw artifact와 runtime artifact는 publish 후 수정하지 않는다. 새 상태는 새 artifact와 pointer 교체로 표현한다.
7. `state.json`, `registry.json`, migration report, doctor JSON은 단일 schema 출처에서 생성하고 검증한다.
8. 오류는 기본적으로 fail-fast다. Doctor, migration report, user-facing warning 같은 경계에서만 degraded/warn으로 변환한다.
9. Shell은 process boundary만 맡는다. JSON shape, store pruning, pointer transaction, migration 판단은 shell에 남기지 않는다.
10. 기존 동작을 통과시키려고 방어 코드를 무작정 추가하지 않는다. 원인을 schema, transaction, boundary에서 해결한다.

## 1. 현재 문제 요약

현재 구조는 기능적으로 많이 안정화되어 있지만 이상적 구조는 아니다.

- `lib/codex-termux-lib.sh`가 2,878줄, `codex_*` 함수 105개를 가진 God module이다.
- JSON state/registry/doctor/migration shape가 embedded Python과 tests에 분산되어 있다.
- malformed registry를 빈 registry로 취급해 덮어쓸 수 있다.
- rollback helper가 실패를 반환해도 일부 호출부가 확인하지 않는다.
- `tree_digest`, `sha256`, `managed_runtime_path`, `managed_raw_path`가 여러 곳에 중복되어 있다.
- shell tests가 내부 함수를 override하는 방식에 많이 의존한다.
- lint, import boundary, max function size, dependency direction을 기계적으로 막는 CI가 없다.

리팩터링 목표는 단순히 파일을 쪼개는 것이 아니다. 목표는 아래 위험을 제거하는 것이다.

- 손상된 registry/state가 조용히 정상 값처럼 취급되는 위험
- active runtime, verified runtime, raw cache pointer가 서로 다른 tuple을 가리키는 위험
- store prune가 current/verified/raw pointer target을 삭제하는 위험
- raw/runtime provenance를 나중에 설명할 수 없는 위험
- doctor가 실제 wrapper 건강 상태와 다른 summary를 출력하는 위험
- 새 helper 추가로 같은 shape와 판단이 다시 중복되는 위험

## 2. 바이너리와 파일 보관 정책

리팩터링 중 바이너리 저장 위치를 바꾸면 안 된다. 저장 위치 변경은 별도 architectural decision 없이는 금지한다.

### 2.1 Raw upstream artifact

위치:

```text
$CODEX_NATIVE_STORE_DIR/raw/<raw-id>/vendor/aarch64-unknown-linux-musl/bin/codex
```

의미:

- 공식 npm package에서 받은 원본 binary다.
- DNS patch, chmod 보정, support file 주입을 하면 안 된다.
- raw artifact는 immutable이다.
- 동일 `<raw-id>`가 이미 있으면 전체 tree digest와 permission까지 같을 때만 재사용한다.
- 같은 `<raw-id>`인데 내용이 다르면 collision으로 실패한다.

필수 metadata:

- raw sha256
- upstream version
- package spec
- raw path
- updated_at

### 2.2 Runtime artifact

위치:

```text
$CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>/codex
$CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>/runtime-build.json
$CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>/codex-resources/
$CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>/codex-path/
$CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>/codex-package.json
```

의미:

- raw binary를 DNS fd33 same-length patch한 실행 산출물이다.
- runtime-private `codex-path/bwrap`, `codex-path/rg`, `codex-path/rg.real`을 포함한다.
- `runtime-build.json`은 patch policy, builder hash, raw hash, runtime hash를 기록한다.
- runtime artifact도 immutable이다.

금지:

- runtime artifact 안에 manager `managed.sh`를 넣지 않는다.
- runtime artifact를 active 상태로 만든 뒤 내부 파일을 수정하지 않는다.
- `native/current` 밑에 직접 새 파일을 쓴 뒤 그것을 store로 나중에 복사하는 흐름을 만들지 않는다.

### 2.3 Active pointers

위치:

```text
$CODEX_NATIVE_NATIVE_ROOT/current  -> $CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>
$CODEX_NATIVE_NATIVE_ROOT/verified -> $CODEX_NATIVE_STORE_DIR/runtime/<tuple-id>
$CODEX_NATIVE_NATIVE_ROOT/raw      -> $CODEX_NATIVE_STORE_DIR/raw/<raw-id>
```

의미:

- `current`는 현재 실행 runtime이다.
- `verified`는 last-known-good runtime이다.
- `raw`는 현재 active raw cache다.
- 세 pointer와 registry/state가 같은 tuple 계열을 가리키는지 항상 검증한다.

pointer 변경 규칙:

- pointer, state, registry 변경은 하나의 transaction으로 처리한다.
- 실패 시 이전 pointer와 이전 metadata를 복원한다.
- rollback 실패는 절대 성공으로 숨기지 않는다.
- cleanup 실패도 최소한 warning report에 남긴다. active 상태가 불명확하면 실패로 끝낸다.

### 2.4 Manager/support files

위치:

```text
$CODEX_NATIVE_NATIVE_ROOT/manager/
  managed.sh
  lib.sh
  wrapper-version.env
  build-runtime.py
  bwrap-termux-compat.py
  rg-termux-shim.sh
  codex_native/
```

의미:

- manager는 wrapper code와 support tool의 설치 위치다.
- Python package `codex_native`는 source tree의 `tools/codex_native/`를 manager에 복사한 것이다.
- runtime artifact는 manager code를 소유하지 않는다.

Manager copy 규칙:

- `bin/install-runtime.sh support`는 `codex_native/` 전체를 manager로 복사해야 한다.
- 복사 후 `python3 -m py_compile`에 해당하는 최소 syntax validation을 source와 installed copy 모두에서 수행한다.
- manager의 `wrapper-version.env`는 wrapper version/commit/provenance를 기록한다.

### 2.5 Public launcher

위치:

```text
$PREFIX/bin/codex
```

의미:

- public launcher는 manager `managed.sh`를 실행한다.
- compiled launcher와 shell launcher 모두 기본 target은 `native/manager/managed.sh`다.
- marker 없는 기존 user launcher는 backup 없이 덮어쓰지 않는다.

## 3. 목표 디렉터리 구조

최종 구조는 아래를 목표로 한다.

```text
bin/
  install-runtime.sh
lib/
  codex-termux-lib.sh
tools/
  codex_native/
    __init__.py
    cli.py
    errors.py
    paths.py
    schemas.py
    hashing.py
    atomic.py
    state.py
    registry.py
    store.py
    builder.py
    activation.py
    migration.py
    doctor_report.py
    doctor_render.py
  build-runtime.py
  bwrap-termux-compat.py
  rg-termux-shim.sh
  codex-launcher.c
tests/
  ...
ci/
  check-structure.sh
  check-python-imports.py
```

설명:

- `lib/codex-termux-lib.sh`는 당장 삭제하지 않는다. 기존 shell function name은 compatibility facade로 남긴다.
- 새 domain logic은 `tools/codex_native/`에 둔다.
- installed manager에서는 `$CODEX_NATIVE_MANAGER_DIR/codex_native/`가 같은 package 역할을 한다.
- shell에서 Python package를 호출할 때는 아래 helper 하나만 사용한다.

```bash
codex_native_cmd() {
    PYTHONPATH="$CODEX_NATIVE_MANAGER_DIR${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -m codex_native.cli "$@"
}
```

source tree test에서는 아래처럼 호출한다.

```bash
PYTHONPATH="$ROOT_DIR/tools" python3 -m codex_native.cli ...
```

금지:

- Python package를 `pip install`해야만 동작하게 만들지 않는다.
- 사용자가 직접 알아야 하는 `codex native ...` public command를 만들지 않는다.
- shell에서 `python3 - <<'PY'` embedded block을 새로 추가하지 않는다. 기존 block은 단계적으로 제거한다.

## 4. 모듈 의존 방향

아래 방향만 허용한다.

```text
errors
paths
schemas
hashing
atomic
state      -> schemas, atomic
registry   -> schemas, hashing, atomic
store      -> schemas, hashing, atomic, registry
builder    -> schemas, hashing, atomic
activation -> schemas, hashing, atomic, state, registry, store
migration  -> schemas, hashing, atomic, registry, store
doctor_report -> schemas, hashing, state, registry, store, migration
doctor_render -> schemas
cli -> all domain modules
```

금지 의존:

- `schemas`가 다른 domain module을 import하면 실패다.
- `hashing`이 state/registry/store를 import하면 실패다.
- `state`와 `registry`가 activation/migration/doctor를 import하면 실패다.
- `store`가 doctor_render를 import하면 실패다.
- `doctor_render`가 filesystem mutation module을 import하면 실패다.
- domain module이 shell path나 global env를 직접 추측하면 실패다. env 해석은 `paths.py`에서만 한다.

기계적 강제:

- `ci/check-python-imports.py`를 작성해 위 import direction 위반 시 실패하게 한다.
- CI가 없어도 local `bash ci/check-structure.sh`에서 반드시 호출한다.

## 5. Public/internal interface 정책

### 5.1 User-facing public interface

유지:

- `codex`
- `codex setup [version]`
- `codex update [version]`
- `codex use [--list|selection]`
- `codex doctor`
- `codex doctor --json|--summary|--all` upstream passthrough
- `codex version`
- `codex remove`
- `codex profile ...`

금지:

- 사용자를 대상으로 `codex doctor wrapper`, `codex native`, `codex internal`, `codex repair-store` 같은 새 command를 추가하지 않는다.
- Wrapper doctor machine JSON은 기존처럼 개발자/installer 경로에서만 접근한다.

### 5.2 Internal Python CLI

허용:

```text
python3 -m codex_native.cli state-read-field
python3 -m codex_native.cli state-write
python3 -m codex_native.cli registry-record
python3 -m codex_native.cli store-publish-runtime
python3 -m codex_native.cli store-publish-raw
python3 -m codex_native.cli store-prune
python3 -m codex_native.cli activation-commit
python3 -m codex_native.cli migration-legacy-store
python3 -m codex_native.cli doctor-json
python3 -m codex_native.cli doctor-render
python3 -m codex_native.cli validate
```

조건:

- 이것은 user-facing command가 아니다.
- `bin/install-runtime.sh` usage에 무분별하게 노출하지 않는다.
- shell facade가 호출하기 위한 stable internal interface다.
- 각 subcommand는 JSON input/output 또는 명확한 argv contract를 가져야 한다.

## 6. Error policy

### 6.1 기본 규칙

- JSON parse failure는 fail-fast다.
- state/registry schema mismatch는 fail-fast다.
- immutable artifact collision은 fail-fast다.
- rollback failure는 fail-fast다.
- cleanup failure는 무시하지 않는다. cleanup 실패가 active 상태를 불명확하게 만들면 fail-fast다.
- doctor rendering failure는 wrapper doctor failure다.

### 6.2 degraded로 변환 가능한 경계

아래만 degraded/warn으로 변환할 수 있다.

- doctor report에서 legacy migration report에 skipped entry가 있는 경우
- doctor network boundary baseline이 outer sandbox 때문에 inconclusive인 경우
- migration best-effort import에서 특정 legacy tuple이 invalid라 skipped report에 기록된 경우

주의:

- best-effort migration에서 특정 tuple skip은 허용하지만, migration engine 자체가 report도 못 쓰고 죽으면 성공으로 반환하면 안 된다.
- registry parse 실패를 migration warning으로 낮추면 안 된다. registry parse 실패는 wrapper health failure다.

### 6.3 broad exception 규칙

Python에서 `except Exception`은 아래 경우에만 허용한다.

1. doctor report가 개별 check를 false로 바꾸는 경계
2. migration이 개별 legacy tuple을 skipped로 기록하는 경계
3. CLI main이 user-facing error message로 변환하는 최상위 경계

그 외 `except Exception: pass`, `except Exception: data = {}`, `except Exception: return False`는 금지한다.

## 7. Schema SSOT

`tools/codex_native/schemas.py`는 아래 type과 validator의 유일한 출처다.

필수 type:

- `StateV3`
- `RegistryV3`
- `RawEntry`
- `WrapperEntry`
- `RuntimeEntry`
- `InstallEntry`
- `BuildManifestV2`
- `MigrationReportV1`
- `DoctorReportV4`
- `NetworkBoundaryReport`
- `ActivationPlan`
- `ActivationResult`
- `PrunePlan`
- `PruneResult`

구현 방식:

- Python `dataclass(frozen=True)` 또는 `TypedDict`를 사용한다.
- JSON boundary에서 받은 `dict[str, object]`는 즉시 validator를 통과시킨다.
- validator 이후 domain module은 raw dict 대신 typed object 또는 validated dict helper만 사용한다.
- schema number는 magic number로 흩뿌리지 않는다.

금지:

- 각 module이 `data.get("schema") == 3` 같은 검사를 따로 구현하지 않는다.
- tests가 전체 schema fixture를 여러 번 복붙하지 않는다. fixture builder를 사용한다.
- malformed JSON을 빈 default로 대체하지 않는다.

## 8. 함수 축출 기준

함수를 Python package로 옮기는 기준은 아래다.

Python으로 옮겨야 하는 함수:

- JSON state/registry/doctor/migration을 읽거나 쓴다.
- artifact hash, tree digest, immutable collision을 판단한다.
- current/verified/raw pointer를 바꾸거나 rollback한다.
- migration/prune처럼 파일 삭제나 store 재배치를 한다.
- doctor machine report나 human rendering을 만든다.
- embedded Python block을 포함한다.
- 같은 판단을 다른 함수와 중복한다.

Shell에 남길 수 있는 함수:

- process `exec`가 필요하다.
- fd 33, env sanitization, `PATH`, `CODEX_HOME`을 직접 다룬다.
- public CLI routing만 담당한다.
- Termux package install이나 launcher install처럼 shell이 더 명확한 작업이다.

## 9. 함수별 축출 매핑

아래 표의 target을 임의로 바꾸지 않는다. target이 부적절하다고 판단되면 먼저 이 문서를 수정하는 별도 commit을 만들고 근거를 적는다.

| 현재 함수/영역 | 현재 문제 | 이동 대상 | 완료 기준 |
| --- | --- | --- | --- |
| `codex_sha256` | shell helper와 embedded Python sha256 중복 | `hashing.py` | 모든 Python sha256은 `hashing.sha256_file` 사용 |
| `codex_tree_digest` | migration에도 tree digest 중복 | `hashing.py` | tree digest 구현은 1개만 존재 |
| `codex_publish_immutable_tree` | store collision policy가 shell에 있음 | `store.py` | identical reuse, collision reject, permission collision test 통과 |
| `codex_write_json_state` | state schema가 shell embedded Python에 있음 | `state.py`, `schemas.py` | state write는 atomic write와 schema validator 사용 |
| `codex_read_state_field` | JSON parse failure handling이 흩어짐 | `state.py` | malformed state는 nonzero |
| `codex_record_registry` | registry parse 실패를 빈 registry로 덮어씀 | `registry.py`, `schemas.py` | malformed registry는 nonzero, 기존 파일 보존 |
| `codex_registry_tuple_for_runtime_path` | registry traversal logic 중복 | `registry.py` | path resolve rule이 registry module 한 곳에 있음 |
| `codex_registry_tuple_state_fields` | tuple field extraction shape 중복 | `registry.py` | typed tuple lookup으로 대체 |
| `codex_runtime_metadata_current` | state/registry/pointer alignment가 shell embedded Python | `registry.py` 또는 `activation.py` | alignment validator 1개 |
| `codex_prune_runtime_store` | destructive delete와 registry rewrite가 섞임 | `store.py` | `PrunePlan` 생성 후 apply, malformed registry면 삭제 없음 |
| `codex_store_runtime_payload` | immutable publish와 runtime validation 결합 | `store.py` | runtime artifact validation function 사용 |
| `codex_store_raw_payload` | raw publish와 validation 결합 | `store.py` | raw artifact validation function 사용 |
| `codex_restore_file_snapshot` | rollback helper failure 전파 불안정 | `activation.py` | rollback result가 aggregate되어 반환 |
| `codex_rollback_path_replacement` | 호출부에서 반환값 누락 | `activation.py` | rollback 실패 test 추가 |
| `codex_finish_path_replacement` | cleanup failure 무시 가능 | `activation.py` | cleanup failure test 추가 |
| `codex_activate_tuple_unlocked` | 96줄 shell transaction, pointer 3개와 metadata 결합 | `activation.py` | `ActivationPlan -> apply -> verify -> commit/rollback` |
| `codex_commit_runtime_candidate` | thin wrapper | shell facade 또는 제거 | Python activation 호출로 대체 |
| `codex_rebuild_runtime_unlocked` | build orchestration 일부는 shell, metadata는 Python | shell + `builder.py` | build step과 activation step 분리 |
| `codex_update_unlocked` | fetch/build/activate가 긴 shell 흐름 | shell orchestrator + `activation.py` | shell은 단계 호출만 수행 |
| `codex_runtime_integrity_ok` | manifest/state/hash 검증 embedded Python | `builder.py` 또는 `store.py` | manifest validator 1개 |
| `codex_raw_integrity_ok` | raw hash 검증 embedded Python | `store.py` | raw validator 1개 |
| `codex_refresh_runtime_metadata_unlocked` | metadata repair와 publish 결합 | `activation.py` | plan/apply 방식으로 변경 |
| `codex_try_verified_rollback_unlocked` | rollback path가 activation과 중복 | `activation.py` | verified rollback도 same transaction engine 사용 |
| `codex_migrate_legacy_store_cache_unlocked` | 191줄 embedded Python, report shape 혼재 | `migration.py` | report schema와 migration logic 분리 |
| `codex_migrate_legacy_runtime_layout_unlocked` | legacy activation path가 shell에 남음 | `migration.py` + `activation.py` | legacy runtime도 activation engine 사용 |
| `codex_network_boundary_json` | runtime sandbox probe orchestration | shell 유지 또는 `doctor_report.py` 보조 | runtime exec 필요 부분만 shell 유지 |
| `codex_wrapper_doctor_json` | 240줄 embedded Python | `doctor_report.py` | JSON schema 4 generator 1개 |
| `codex_wrapper_doctor` | human renderer embedded Python | `doctor_render.py` | renderer는 filesystem mutation 없음 |
| `codex_public_doctor` | upstream + wrapper composition | shell 유지 | 공백행/구분선/exit aggregation 유지 |
| `codex_use_render` | registry filtering 중복 | `registry.py` + shell facade | managed_runtime/raw filtering 1개 |
| `codex_use_select` | `codex_use_render`와 중복 | `registry.py` | selection resolver 1개 |
| `codex_bootstrap_store_unlocked` | state/registry bootstrap shell | `activation.py` 또는 `registry.py` | lock은 shell, mutation은 Python |

## 10. 파편화 방지 규칙

새 파일을 만들기 전에 아래 질문에 모두 답한다.

1. 이 파일의 domain owner는 무엇인가.
2. 이 파일보다 기존 module에 넣는 것이 더 자연스럽지 않은가.
3. 이 파일이 80줄 미만이면 독립 파일이어야 하는 강한 이유가 있는가.
4. 이 파일이 300줄을 넘으면 두 domain이 섞인 것은 아닌가.
5. 이 파일이 import direction을 어기지 않는가.
6. 이 파일의 public function이 7개를 넘으면 facade가 필요한 것은 아닌가.

금지:

- 함수 하나당 파일 하나로 쪼개지 않는다.
- `utils.py`, `helpers.py`, `common.py` 같은 무소유 파일을 만들지 않는다.
- `schemas.py` 밖에 shape constant를 만들지 않는다.
- `doctor.py` 하나에 report 생성과 rendering과 filesystem mutation을 모두 넣지 않는다.
- test fixture JSON을 여러 파일에 복붙하지 않는다.

허용 예외:

- `__init__.py`
- `errors.py`
- `cli.py`
- import boundary checker 같은 작은 CI tool

## 11. 리팩터링 순서

순서를 바꾸지 않는다. 앞 단계가 green이 아니면 다음 단계로 가지 않는다.

### Phase 0: Baseline과 guardrail

목표:

- 기존 동작을 고정한다.
- 구조 규칙을 먼저 만든다.
- 아직 production behavior를 바꾸지 않는다.

작업:

1. `tools/codex_native/` package skeleton을 만든다.
2. `ci/check-structure.sh`를 만든다.
3. `ci/check-python-imports.py`를 만든다.
4. shell syntax, Python compile, C syntax, diff check를 한 번에 실행하는 `ci/check-structure.sh`를 만든다.
5. function length와 file length 제한을 경고가 아니라 실패로 만든다. 초기 threshold는 현실적으로 둔다.

초기 threshold:

```text
lib/codex-termux-lib.sh max lines: 3000
any shell function max lines: 250
new Python module max lines: 350
new Python function max lines: 80
```

단계별로 threshold를 낮춘다. 최종 목표:

```text
lib/codex-termux-lib.sh max lines: 900
any shell function max lines: 80
Python module max lines: 350
Python function max lines: 70
```

검증:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
git diff --check
```

완료 조건:

- production behavior change 없음.
- 모든 기존 test 통과.
- CI script가 local에서 동작.

### Phase 1: Schema SSOT 도입

목표:

- state/registry/doctor/migration shape를 `schemas.py`로 고정한다.
- malformed JSON을 빈 default로 대체하는 경로를 제거할 준비를 한다.

작업:

1. `schemas.py`에 schema type과 validator를 만든다.
2. `errors.py`에 `CodexNativeError`, `SchemaError`, `IntegrityError`, `TransactionError`, `CollisionError`를 만든다.
3. `atomic.py`에 atomic write helper를 만든다.
4. `state.py`에 state read/write/validate를 만든다.
5. `registry.py`에 registry read/write/validate와 tuple lookup을 만든다.
6. shell의 `codex_read_state_field`, `codex_write_json_state`, `codex_record_registry`는 Python CLI를 호출하는 facade로 바꾼다.

금지:

- `except Exception: data = {}`를 유지하지 않는다.
- malformed registry를 repair한다는 명목으로 자동 초기화하지 않는다.

필수 tests:

- valid state read/write
- malformed state read failure
- malformed registry record failure
- malformed registry가 기존 파일을 보존하는지
- missing optional field가 validator에서 허용되는지, 필수 field 누락은 실패하는지

검증:

```bash
bash ci/check-structure.sh
bash tests/runtime-integrity.sh
bash tests/pointer-activation.sh
bash tests/transactional-update.sh
bash tests/doctor-contract.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- malformed registry 재현 시 파일이 덮어써지지 않는다.
- registry/state read/write의 schema number가 `schemas.py` 밖에서 새로 정의되지 않는다.

### Phase 2: Hashing과 immutable store 이동

목표:

- sha256, tree digest, immutable publish, collision 판단을 Python SSOT로 만든다.

작업:

1. `hashing.py`에 `sha256_file`, `tree_digest`를 만든다.
2. `store.py`에 `publish_immutable_tree`, `validate_runtime_artifact`, `validate_raw_artifact`를 만든다.
3. shell의 `codex_tree_digest`, `codex_publish_immutable_tree`, `codex_store_runtime_payload`, `codex_store_raw_payload`는 Python CLI facade로 바꾼다.
4. migration의 tree digest 중복을 제거한다.

필수 tests:

- identical artifact reuse
- same id different content collision
- same id different permission collision
- symlink/special file 처리
- target이 symlink면 collision
- concurrent publish race에서 동일 content는 성공, 다른 content는 실패

검증:

```bash
bash ci/check-structure.sh
bash tests/immutable-store.sh
bash tests/pointer-activation.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- `rg -n '^def tree_digest|def sha256' lib tools/codex_native` 결과에서 구현은 `hashing.py` 한 곳이어야 한다.
- shell은 hashing 구현을 직접 갖지 않는다.

### Phase 3: Activation transaction 이동

목표:

- current/verified/raw pointer, state, registry 변경을 하나의 transaction engine으로 통합한다.
- rollback 실패와 cleanup 실패를 호출부가 놓치지 않게 한다.

작업:

1. `activation.py`에 `ActivationPlan`, `ActivationSnapshot`, `ActivationResult`를 구현한다.
2. `activation.py`는 아래 순서로 동작한다.
   - candidate runtime smoke test 확인
   - runtime/raw immutable store publish
   - state/registry snapshot 생성
   - registry candidate write
   - state candidate write
   - current pointer replace
   - verified pointer replace
   - raw pointer replace
   - runtime readiness verify
   - backup cleanup
   - prune trigger는 activation 성공 후 별도 단계
3. 실패 시 역순 rollback한다.
4. 모든 rollback action 결과를 aggregate한다.
5. rollback 실패가 하나라도 있으면 `TransactionError`로 끝낸다.
6. shell의 `codex_activate_tuple_unlocked`, `codex_commit_runtime_candidate`, `codex_try_verified_rollback_unlocked`, `codex_refresh_runtime_metadata_unlocked`, `codex_bootstrap_store_unlocked`는 Python activation facade로 축소한다.

금지:

- pointer 하나를 바꾼 뒤 state/registry를 나중에 맞추는 구조를 만들지 않는다.
- cleanup 실패를 `|| true`로 숨기지 않는다.
- rollback helper를 호출하고 반환값을 무시하지 않는다.

필수 tests:

- current pointer replace failure
- verified pointer replace failure
- raw pointer replace failure
- state write failure
- registry write failure
- readiness failure after pointers moved
- rollback cleanup failure
- snapshot restore failure
- verified rollback success
- verified rollback failure leaves old current intact
- activation lock contention

검증:

```bash
bash ci/check-structure.sh
bash tests/pointer-activation.sh
bash tests/pointer-rollback.sh
bash tests/verified-rollback.sh
bash tests/transactional-update.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- `rg -n 'codex_rollback_path_replacement|codex_restore_file_snapshot|codex_finish_path_replacement' lib/codex-termux-lib.sh` 결과가 없거나 facade만 남는다.
- activation 실패 재현에서 old current, verified, raw, state, registry가 모두 유지된다.

### Phase 4: Prune plan/apply 분리

목표:

- destructive delete를 plan과 apply로 분리한다.
- malformed registry/state에서는 삭제하지 않는다.

작업:

1. `store.py`에 `build_prune_plan`을 만든다.
2. `build_prune_plan`은 삭제 대상, 보존 대상, registry rewrite 내용을 JSON plan으로 반환한다.
3. `apply_prune_plan`은 plan에 있는 path만 삭제한다.
4. current/verified/raw pointer target은 registry/state와 별개로 항상 보호한다.
5. registry parse failure, state parse failure, path outside managed store는 prune failure다.

필수 tests:

- current pointer target protected
- verified pointer target protected
- raw pointer target protected
- malformed registry does not delete and does not rewrite
- malformed state does not delete and does not rewrite
- retention count respected
- incompatible unprotected runtime deleted
- registry rewrite removes pruned entries only

검증:

```bash
bash ci/check-structure.sh
bash tests/prune-pointer-protection.sh
bash tests/immutable-store.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- 손상 registry 재현 시 파일과 store가 보존되고 command가 nonzero로 끝난다.
- prune에서 `except Exception: data = {}` 형태가 없다.

### Phase 5: Legacy migration 이동

목표:

- legacy store migration과 legacy runtime layout migration을 Python module로 분리한다.
- best-effort tuple skip과 engine failure를 구분한다.

작업:

1. `migration.py`에 `migrate_legacy_store_cache`를 만든다.
2. invalid legacy tuple은 `MigrationReportV1.skipped`에 기록한다.
3. registry/state parse failure는 migration engine failure다.
4. report write failure는 command failure다.
5. 이미 report가 있으면 idempotent no-op이다.
6. legacy runtime layout migration은 activation engine을 사용해 current/verified/raw pointer layout으로 승격한다.

필수 tests:

- valid legacy tuple imported
- invalid tuple skipped with reason
- missing runtime/raw entries skipped
- raw outside legacy raw store skipped
- malformed registry fails and writes no false success report
- report write failure returns nonzero
- repeated migration does not change report
- legacy runtime layout migration uses store pointers

검증:

```bash
bash ci/check-structure.sh
bash tests/legacy-store-migration.sh
bash tests/legacy-migration.sh
bash tests/installer-layout.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- migration engine 자체 실패가 `return 0`으로 숨겨지지 않는다.
- migration report shape는 `schemas.py`에서만 정의된다.

### Phase 6: Doctor report/render 분리

목표:

- wrapper doctor JSON 생성과 human rendering을 분리한다.
- upstream doctor와 wrapper doctor 사이의 public contract는 유지한다.

작업:

1. `doctor_report.py`에 `build_doctor_report`를 만든다.
2. `doctor_render.py`에 `render_human_doctor`를 만든다.
3. renderer는 filesystem mutation을 하지 않는다.
4. report builder는 schema 4를 생성한다.
5. migration warning은 human summary를 degraded로 만들 수 있지만 `overallStatus == ok`이면 exit code 0을 유지한다.
6. `codex_public_doctor` shell function은 upstream doctor 실행, 구분선 출력, wrapper doctor 실행만 담당한다.

유지해야 할 출력 계약:

- `codex doctor`는 upstream human doctor 후 빈 줄, 흰색 구분선, 빈 줄, wrapper human doctor를 출력한다.
- `codex doctor --json`, `--summary`, `--all` 등 인자가 있으면 upstream에 그대로 전달한다.
- wrapper header 아래에 별도 구분선을 추가하지 않는다.
- wrapper summary 형식은 `17 ok · 1 idle · 0 warn · 0 fail ok` 계열이다.

필수 tests:

- healthy not-needed summary
- healthy completed summary
- migration issues degraded summary with exit 0
- broken current pointer fail summary with nonzero
- upstream passthrough
- separator exact contract
- renderer does not output raw JSON in default mode

검증:

```bash
bash ci/check-structure.sh
bash tests/doctor-contract.sh
TERM=xterm-256color codex doctor
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- `codex_wrapper_doctor_json`는 shell embedded Python을 갖지 않는다.
- `codex_wrapper_doctor`는 Python renderer facade로 축소된다.

### Phase 7: `codex use` registry selection 이동

목표:

- cached runtime filtering과 selection을 registry/store validator로 통합한다.

작업:

1. `registry.py`에 `list_usable_runtimes`를 만든다.
2. `registry.py`에 `resolve_runtime_selection`을 만든다.
3. active/latest/remote row rendering은 shell 또는 Python CLI 중 하나로 고정한다.
4. `managed_runtime_path`, `managed_raw_path` 중복을 제거한다.

필수 tests:

- list cached runtimes
- invalid runtime path excluded
- invalid raw path excluded
- active badge preserved
- selection by index
- selection by version
- selection by runtime hash prefix
- unknown selection fails

검증:

```bash
bash ci/check-structure.sh
bash tests/use-cache-activation.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- `rg -n 'def managed_runtime_path|def managed_raw_path' lib tools/codex_native` 결과에서 구현은 한 곳이다.

### Phase 8: Shell facade 축소

목표:

- `lib/codex-termux-lib.sh`를 process boundary 중심으로 줄인다.

Shell에 남길 것:

- `codex_say`, `codex_fail`
- env/path default 정의
- `codex_with_lock`
- `codex_runtime_exec`
- `codex_smoke_test_runtime`
- `codex_prepare_runtime_env`
- `codex_open_fd33_and_exec`
- `codex_public_doctor`
- `codex_profile_*`
- `codex_main`
- thin facade functions that call `codex_native_cmd`

Shell에서 제거할 것:

- embedded Python JSON mutation
- embedded Python doctor rendering
- tree digest implementation
- registry filtering implementation
- destructive prune logic
- pointer transaction logic
- migration engine

검증:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
TERM=xterm-256color codex doctor
```

완료 조건:

- `lib/codex-termux-lib.sh`가 900줄 이하로 줄어든다.
- shell function 80줄 초과가 없다.
- 새 embedded Python block이 없다.

### Phase 9: Test refactor

목표:

- 내부 function override 중심 test를 public or near-public command/CLI fixture 중심으로 옮긴다.

작업:

1. `tests/fixtures/`에 fixture builder를 둔다.
2. doctor JSON fixture는 builder를 사용한다.
3. activation failure injection은 Python CLI의 explicit test hook 또는 fixture permission으로 구현한다.
4. internal shell function override를 줄인다.
5. tests는 문자열 snapshot보다 side effect를 우선 검증한다.

허용되는 문자열 검증:

- CLI public output contract
- doctor section/header/summary
- exact separator

금지되는 문자열 검증:

- source code 내부 문자열 grep만으로 동작 검증을 대체
- 구현 내부 함수명 존재 여부만으로 성공 판정

검증:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
```

완료 조건:

- tests가 내부 shell function override에 의존하는 횟수가 절반 이하로 줄어든다.
- 모든 destructive behavior는 filesystem side effect로 검증한다.

## 12. CI와 lint 강제

`ci/check-structure.sh`는 아래를 순서대로 실행한다.

```bash
#!/usr/bin/env bash
set -euo pipefail

for f in install.sh bin/*.sh lib/*.sh tools/*.sh tests/*.sh ci/*.sh; do
    [ -e "$f" ] || continue
    bash -n "$f"
done

python3 -m py_compile tools/*.py tools/codex_native/*.py ci/*.py

if command -v clang >/dev/null 2>&1; then
    clang -fsyntax-only -Wall -Wextra -Werror tools/codex-launcher.c
fi

python3 ci/check-python-imports.py

git diff --check
```

추가 구조 check:

- `lib/codex-termux-lib.sh` line count threshold
- shell function line threshold
- Python module line threshold
- Python function line threshold
- embedded Python block 신규 추가 금지
- `except Exception: pass` 금지
- `except Exception: data = {}` 금지
- `|| true` 신규 추가 금지. 필요한 경우 주석으로 이유와 boundary를 적고 review에서 확인한다.

ShellCheck:

- Termux에 `shellcheck`가 있으면 CI에서 실행한다.
- 없으면 CI가 실패하지 않게 하되, local warning을 출력한다.
- `.shellcheckrc`를 추가하고 disable은 line-local로만 허용한다.

Ruff/mypy:

- 외부 dependency 설치가 필요하면 첫 단계에서는 필수로 만들지 않는다.
- 대신 stdlib-only `py_compile`, import boundary checker, custom grep lint를 필수로 한다.
- 나중에 dependency 설치가 안정화되면 ruff/mypy를 추가한다.

## 13. Test policy

테스트는 아래 우선순위를 따른다.

1. 빈 입력, malformed JSON, missing file, permission failure, collision, rollback failure, concurrency부터 작성한다.
2. 그 다음 정상 happy path를 작성한다.
3. 내부 구현 mocking은 외부 boundary mocking보다 우선하지 않는다.
4. filesystem side effect를 검증한다.
5. public CLI contract만 문자열을 exact로 검증한다.

필수 edge cases:

- malformed state
- malformed registry
- registry schema mismatch
- state schema mismatch
- missing active tuple
- missing raw entry
- raw hash mismatch
- runtime hash mismatch
- runtime-build manifest mismatch
- DNS patch policy mismatch
- store collision by content
- store collision by permission
- target symlink collision
- concurrent publish same content
- concurrent publish different content
- current pointer replace failure
- verified pointer replace failure
- raw pointer replace failure
- state write failure
- registry write failure
- rollback restore failure
- cleanup failure
- prune with malformed registry
- prune with protected current/verified/raw
- migration invalid tuple skip
- migration engine failure
- doctor network inconclusive
- doctor wrapper fail

## 14. Commit discipline

각 phase는 별도 commit으로 끝낸다.

Commit 규칙:

- Phase 0 commit: guardrail only, behavior change 없음
- Phase 1 commit: schema/state/registry
- Phase 2 commit: hashing/store
- Phase 3 commit: activation transaction
- Phase 4 commit: prune plan/apply
- Phase 5 commit: migration
- Phase 6 commit: doctor
- Phase 7 commit: use selection
- Phase 8 commit: shell facade shrink
- Phase 9 commit: test refactor

각 commit 전에 실행:

```bash
bash ci/check-structure.sh
for f in tests/*.sh; do bash "$f"; done
git status --short
```

금지:

- phase 여러 개를 한 commit에 섞지 않는다.
- failing test를 남기고 commit하지 않는다.
- unrelated formatting churn을 섞지 않는다.
- `git reset --hard`, `git clean -fd`로 사용자의 변경을 지우지 않는다.

## 15. Done definition

전체 리팩터링 완료 조건:

- `lib/codex-termux-lib.sh`가 900줄 이하다.
- shell function 80줄 초과가 없다.
- Python module 350줄 초과가 없다. 예외는 명시적 review 필요.
- `schemas.py`가 state/registry/doctor/migration shape의 유일한 출처다.
- `tree_digest`, `sha256_file`, `managed_runtime_path`, `managed_raw_path` 구현이 중복되지 않는다.
- malformed registry/state가 자동 초기화되지 않는다.
- activation rollback failure가 test로 고정되어 있다.
- prune는 malformed registry/state에서 삭제하지 않는다.
- migration engine failure는 nonzero다.
- doctor output contract가 유지된다.
- public command surface가 늘지 않았다.
- `ci/check-structure.sh`와 전체 tests가 통과한다.
- live Termux에서 `TERM=xterm-256color codex doctor`가 wrapper와 upstream doctor를 정상 합성한다.

## 16. 구현자가 판단하면 안 되는 것

아래는 구현자가 임의로 바꾸면 안 된다.

- 바이너리 저장 위치
- public command set
- raw/runtime immutable 정책
- current/verified/raw pointer 의미
- DNS fd33 same-length patch 정책
- manager 소유권
- profile config 비변경 정책
- upstream doctor passthrough 정책
- wrapper doctor separator 정책
- malformed registry fail-fast 정책
- schema SSOT 원칙

바꿔야 한다고 생각하면 먼저 별도 문서 변경으로 이유, 대안, migration path, 검증 계획을 작성한다. 코드 변경은 그 다음이다.

## 17. 빠른 체크리스트

작업 시작 전:

- [ ] 현재 branch와 dirty files 확인
- [ ] 사용자 변경을 reset하지 않음
- [ ] 해당 phase 범위 확인
- [ ] 새 module이 import direction을 지키는지 확인
- [ ] 기존 helper/shape가 있는지 `rg`로 확인

코드 작성 중:

- [ ] JSON parse failure를 숨기지 않음
- [ ] broad `except Exception`을 boundary 밖에서 쓰지 않음
- [ ] 새 embedded Python block을 추가하지 않음
- [ ] shell에서 rollback return value를 무시하지 않음
- [ ] destructive operation은 plan/apply 또는 transaction에 있음
- [ ] test는 edge case부터 작성

리뷰 전:

- [ ] 중복 helper 없음
- [ ] 죽은 코드 없음
- [ ] commented-out code 없음
- [ ] hidden global contract 없음
- [ ] module fan-in/fan-out 과도하지 않음
- [ ] public command surface 증가 없음
- [ ] `bash ci/check-structure.sh` 통과
- [ ] `for f in tests/*.sh; do bash "$f"; done` 통과

## 18. 첫 구현자에게 주는 시작 지시

처음 작업자는 Phase 0만 수행한다. Phase 0에서 behavior를 바꾸지 않는다.

정확한 첫 작업:

1. `tools/codex_native/__init__.py` 생성
2. `tools/codex_native/errors.py` 생성
3. `tools/codex_native/cli.py` 생성. 아직 no-op `validate` command만 둔다.
4. `ci/check-python-imports.py` 생성. 처음에는 허용 import table만 검사한다.
5. `ci/check-structure.sh` 생성.
6. `bin/install-runtime.sh`의 support copy가 나중에 `codex_native/`를 복사할 수 있도록 TODO가 아니라 실제 copy path를 준비한다. 단, 아직 runtime behavior는 바꾸지 않는다.
7. 모든 tests를 실행한다.

Phase 0이 통과하면 그 다음 구현자만 Phase 1로 넘어간다.
