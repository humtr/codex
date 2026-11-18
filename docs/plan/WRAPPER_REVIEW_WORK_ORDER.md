# Termux 패치 래퍼 개선 작업 명세서

**업스트림 경로 정렬 및 기능 개선(2026-11-18 반영).** CLI 진입점은 `$PREFIX/bin/codex`를 유지하고, 언팩·패치된 업스트림 패키지 저장소와 활성 심볼릭 링크는 업스트림 레이아웃(`~/.codex/packages/standalone/releases/<version>-<6hex>` 및 `current`)과 일치시킨다. 또한 다운로드 진행률(`downloaded / total`), 패치 영역 transient 표시, `CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` 외부 프로필 인증 분리·세션 공유 바인딩을 함께 반영한다.

2026-11-18 기준 `rewrite/wrapper`의 `f0db5f0557567499e389b010fd1e86d53dcfce5f`를 기준으로 P1 01–06, P2 07–10, R1 리팩터링 및 업스트림 경로·UI·외부 프로필 공유 정렬을 모두 구현·검증했습니다. 실제 기기 검증 결과는 [증거 원장](../../CURRENT.md)에 기록합니다.

**적용 범위.** `install.sh`가 래퍼 실행 파일과 설치 영수증을, Rust 래퍼가 업스트림 선택·후보 검증·활성화 포인터와 저널을, release-builder가 패키지와 ELF 변환을 소유한다. 인증, 세션, 데몬 수명주기는 업스트림 소유다. 공개 동작이 바뀌는 항목은 구현에 앞서 해당 계약을 정리한다. [제품 명세](../../SPEC.md) · [CLI 계약](../spec/USER_INTERFACES.md) · [배포·복구 계약](../spec/DELIVERY.md) · [구현 책임 지도](../architecture/STAGE1_BOUNDARIES.md)

## 작업 순서

| 순서 | 작업 | 의존 관계 |
| --- | --- | --- |
| 먼저 | P1 01–04. 후보 무결성, 롤백 양방향 실패, 설치 완료 판정 | 01의 실제 파일 검증을 02와 04에서 재사용한다. 03은 02와 같은 롤백 경로에 적용한다. |
| 독립적인 P1 | 05 조회·복구 잠금, 06 업데이터 명령 라우팅 | 서로 독립적이다. 05를 수정할 때 07의 카탈로그 잠금과 잠금 순서를 함께 검토한다. |
| 후속 P2 | 07 최초 설치 보호, 08 FD 35 패처, 09 진단, 10 설정 오류 격리 | 08은 독립적으로 진행할 수 있다. 10에서 생기는 경고와 실행 실패는 09의 진단 결과에서 식별할 수 있어야 한다. |
| 독립적인 정리 | R1 안정 버전 판정과 업데이트 인자 파싱 통합 | 결함 수정의 선행 조건이 아니다. 회귀 원인을 구분할 수 있도록 별도 변경으로 검증한다. |

## 세대 검증과 복구

### 01 P1 준비된 세대의 SHA-256을 실제 파일과 대조한다

- **재현.** 임시 세대의 `generation.meta`를 작성한 뒤 `bin/codex`에 동작에 영향을 주지 않는 주석을 추가했다. 버전과 기능 조회는 통과했고 `update --path <DIR>`는 digest가 달라진 세대를 활성화했다. 현재 `with_qualified_loaded_runtime`은 manifest 문자열을 그대로 `observed_digest`로 전달한다. [런타임 바인딩](../../crates/wrapper/src/main.rs) · [후보 복사와 설치](../../crates/wrapper/src/update.rs)
- **구현.** 래퍼가 최종 복사한 후보에서 런타임과 manifest에 선언된 브라우저 helper의 SHA-256을 계산해 대조한 후 실행 검증으로 넘긴다. 일반 실행의 매 시작 시 전체 패키지를 다시 해시하는 비용은 피한다. 온라인 경로의 공식 아카이브 digest 검증과 오프라인 입력의 신뢰 범위를 유지한다.
- **완료 판정.** 정상 prepared generation을 설치하고 실행한다. 런타임 변경, helper 한 개 변경, 선언된 helper 누락을 각각 거부한다. 거부된 후보의 실행 횟수는 0이고 기존 `current`와 `previous`가 가리키는 세대 및 포인터 파일 바이트가 보존된다.

### 02 P1 실행할 수 없는 이전 세대로 롤백하지 않도록 한다

- **재현.** A에서 B로 업데이트한 뒤 A의 `bin/codex` 실행 권한을 제거했다. `update --rollback`은 종료 0으로 A를 선택했지만 이어진 `--version`은 `Permission denied`로 실패했다. [롤백 구현](../../crates/wrapper/src/update.rs) · [후보 실행 검증](../../crates/wrapper/src/qualification.rs)
- **구현.** `run_rollback`이 대상의 세대 ID, manifest 호환성, 파일 무결성 및 실행 가능성을 확인한다. 격리된 홈에서 기존 버전·기능 검증을 재사용한다. 포인터 교체 직전에 기존 상태와의 일치 여부도 다시 확인한다.
- **완료 판정.** 정상 A로 복귀한 뒤 ID와 실제 버전이 A와 일치한다. 실행 비트 누락, 잘못된 버전 출력 또는 기능 조회 실패 시 종료는 실패이고 B는 계속 실행된다. 실패 전후의 포인터 바이트가 같아야 한다.

### 03 P1 현재 세대가 손상돼도 정상 이전 세대로 복구한다

- **재현.** A와 B가 설치된 상태에서 현재 B의 `bin/codex`만 이동했다. A는 온전했지만 롤백은 `activated generation runtime is missing`으로 끝났다. `run_rollback`이 출력할 버전을 얻기 위해 B 전체를 먼저 로드하기 때문이다. [롤백과 결과 출력](../../crates/wrapper/src/update.rs) · [포인터와 저널](../../crates/wrapper/src/state.rs)
- **구현.** 저널 복구 결과로 얻은 포인터에서 대상을 선택하고 02의 기준으로 A를 검증한다. B의 버전을 읽을 수 없으면 이미 확보한 세대 ID로 전환 결과를 표시한다. 커밋에는 기존 활성화 트랜잭션을 사용한다.
- **완료 판정.** B의 런타임 누락과 descriptor 손상을 각각 주입한 뒤, A로 롤백하여 `--version`을 실행한다. 이어서 손상된 B로 되돌리려는 시도는 02의 검사를 통해 거부된다. 포인터 자체가 모순되거나 저널을 복구할 수 없는 경우에는 대상을 추정하지 않고 오류를 반환한다.

### 04 P1 설치 완료와 중단 복구가 같은 Rust 호환성 판정을 사용한다

- **재현.** 격리 설치에서 선택된 세대의 `wrapper_api_identity`만 호환되지 않는 값으로 바꿨다. `wrapper status --json`은 `active`와 종료 0, 실제 `--version`은 호환성 오류와 종료 1을 반환했다. 실제 `install.sh`도 자격 확인을 건너뛰고 성공을 보고했다. [설치기의 qualified_new_activation](../../install.sh) · [상태 조회와 manifest 호환성 검사](../../crates/wrapper/src/main.rs)
- **구현.** 포인터의 선택 상태와 설치 완료 증거를 구분한다. 래퍼가 같은 세대 스냅샷에서 호환성, 예상 래퍼 SHA-256, 명시된 경우 요청 버전 및 대상 검증 결과를 제공하고 설치기가 이를 소비하게 한다. `qualified_new_activation`의 정상 설치·중단 복구 호출자를 함께 전환한다. 설치기의 실행 파일 소유권과 updater 프로세스 영수증 검사는 유지한다.
- **완료 판정.** 동일 래퍼와 정상 세대 재설치는 성공한다. API 불일치나 실행 불가 대상은 `active`여도 자격 확인을 통과할 수 없다. 정상 설치 시 복구·재검증이 성공했다면 **최종 선택된 런타임**이 실행돼야 한다. 중단 설치에서 자격을 확인할 수 없으면 영수증과 백업을 보존하고 불확실한 updater를 재실행하지 않는다. [설치 복구 테스트](../../.github/scripts/test_install_recovery.py)

## 동시 실행과 명령 라우팅

### 05 P1 정상 조회끼리 활성화 쓰기 잠금을 다투지 않게 한다

- **재현.** 업데이트와 미완료 저널이 없는 임시 설치에서 `wrapper status --json` 96회를 동시성 16으로 실행했다. 85회 성공, 11회가 `another activation transaction is in progress`로 실패했다. 일반 실행도 같은 복구 함수를 호출한다. [상태 읽기와 복구](../../crates/wrapper/src/state.rs) · [일반 실행과 status](../../crates/wrapper/src/main.rs)
- **구현.** 기존 잠금 아래에서 정상 포인터를 여러 독자가 일관되게 읽게 한다. 복구가 필요한 독자는 쓰기 잠금을 획득한 후 저널과 포인터를 다시 읽고 전환한다. 조회, 실행, 활성화, 롤백, 정리가 사용하는 잠금 순서를 일치시킨다.
- **완료 판정.** 읽기 전용 동시 조회가 전부 성공한다. 동시에 정상 실행을 시작해도 독자 간 경합으로 실패하지 않는다. 실제 활성화와 겹친 성공 조회는 전환 전후의 완전한 세대 중 하나를 가리킨다. 최초 포인터가 없는 저널, 중단된 업데이트·롤백, 충돌하는 쓰기 작업에 대한 기존 복구 테스트도 통과한다.

### 06 P1 모든 실행 경로에서 업스트림 자체 업데이터 호출을 차단한다

- **재현.** 격리 런타임에서 `codex update --help`는 래퍼가 처리했고, 전역 `-c` 옵션이 앞선 형태와 `codex wrapper --bare -- update --help`는 업스트림으로 전달됐다. 기기의 공식 0.169.0에서도 자체 `update` 도움말을 확인했다. 관찰 범위는 라우팅과 도움말 출력까지다. [공개 명령 분류와 bare 경로](../../crates/wrapper/src/main.rs) · [공개 명령 계약](../../SPEC.md)
- **구현.** 직접 호출과 `--bare`가 같은 최종 업스트림 명령 분류를 사용하게 한다. 최상위 `codex update`는 기존 래퍼 경로를 따른다. 우회 형태가 업스트림 자체 업데이트를 뜻할 때의 제안 동작은 사용법 오류(종료 2)와 공식 래퍼 명령 안내다. 공개 계약에서 처리 방식을 먼저 확정한다.
- **완료 판정.** 도움말 및 호출 기록용 fixture로 두 우회 형태가 업스트림 updater에 도달하지 않음을 확인한다. 프롬프트·옵션 값·하위 명령의 데이터로 전달된 `update`는 원래 argv 그대로 도착해야 한다. 업스트림이 소유하는 `app-server daemon update --from-cli`도 명령 분류 fixture에서 보존한다. 실제 교체 효과를 일으키지 않고 라우팅을 검사할 수 있다.

### 07 P2 최초 설치의 staging도 정리 작업으로부터 보호한다

- **재현.** 빈 HOME에서 첫 updater가 메타데이터 취득 중일 때 다른 프로세스가 카탈로그의 배타 정리 잠금을 획득했다. `generations`를 먼저 만들어 둔 대조군에서는 획득이 차단됐다. 리뷰에서 확인한 효과는 잠금 획득이다. 진행 중 파일의 삭제는 이 경합으로 가능한 후속 결과다. [카탈로그 lease와 prune](../../crates/wrapper/src/maintenance.rs) · [업데이트 진입점](../../crates/wrapper/src/update.rs)
- **구현.** 변경형 업데이트가 카탈로그 잠금 대상을 준비하고 공유 lease를 확보한 뒤 staging을 생성하게 한다. 보호 기간은 후보 준비부터 커밋 또는 실패 정리까지다. `update --check`와 `--help`에는 카탈로그 생성이 생기지 않아야 한다.
- **완료 판정.** 빈 HOME과 기존 설치에서 첫 updater를 동기화 지점에 멈춘 뒤 정리 시도를 겹친다. 진행 중인 staging은 유지되고 updater는 정상 완료할 수 있어야 한다. lease 해제 후에는 보존 대상에서 제외된 중단 staging을 기존 정책으로 정리할 수 있다.

## 패처와 진단

### 08 P2 FD 35 패처가 역방향 AArch64 BL 호출을 인식하게 한다

- **재현.** 합성 ELF에서 `strdup`을 호출 지점 앞으로 옮기자 `expected unique strdup fallback inside realpath`로 빌드가 실패했다. `extract_strdup_bl`과 `locate_realpath_and_strdup`에는 호출 명령을 판별하면서 PC에 0을 전달하는 경로가 있다. [FD 35 변환과 AArch64 fixtures](../../crates/release-builder/src/runtime_patch.rs)
- **구현.** 호출 종류만 필요할 때는 주소 계산과 분리하고, 목적지를 계산할 때는 실제 명령의 VA와 부호 있는 변위를 사용한다. 기존 유일성·ELF 범위·재배치 검사는 유지한다.
- **완료 판정.** 같은 의미의 순방향·역방향 `strdup`과 `realpath` 호출 fixture가 모두 올바른 대상을 찾아 패치한다. 후보가 없거나 둘 이상이거나 범위를 벗어나면 거부한다. 기존 AArch64 FD 35 실행 테스트와 현재 호환 패키지의 격리 데몬 소켓 검증을 통과한다. 공식 0.169.0에서 이 역방향 결함이 발생했다는 관측은 없다.

### 09 P2 doctor가 실패한 구성요소와 검사 중단 원인을 보고하게 한다

- **재현.** 격리 PREFIX의 resolver 파일을 이동하자 래퍼가 FD를 열지 못했다. 업스트림 doctor 프로세스가 시작되기 전에 반환된 JSON은 `termux_wrapper=healthy`, `upstream=unhealthy`였다. 소스에서 자식 stdout의 바이트 상한은 확인했으나 EOF 읽기와 종료 대기에 시간 상한이 없다. [doctor 실행과 출력](../../crates/wrapper/src/doctor.rs) · [진단 상태 생성](../../crates/wrapper/src/main.rs)
- **구현.** 래퍼 준비 실패, 업스트림 미실행, 실행 후 비정상 종료, 출력 초과, 시간 초과를 구분한다. 준비 결과로 래퍼 상태를 산출하고 사람이 읽는 출력과 JSON이 같은 결과를 사용하게 한다. stdout 수집부터 종료 대기까지 기한을 적용하고 생성한 자식을 회수한다. JSON 소비자에 미치는 변경은 CLI 계약에 기록한다.
- **완료 판정.** resolver 누락 시 래퍼 준비 실패와 업스트림 미실행을 확인한다. 정상·비정상 자식, 출력 없이 대기하는 자식, 과다 출력을 내는 자식으로 분류·종료 시간·회수를 검증한다. 정상 응답과 기존 출력 마스킹을 유지하고 qualification 도구의 JSON 판독도 검증한다. [qualification 소비자](../../.github/scripts/qualify_wrapper.py)

### 10 P2 부가 설정 오류가 영향을 미치는 명령 범위를 정한다

- **재현.** 잘못된 `wrapper.toml`에서 일반 `features list`, `wrapper status`, `wrapper setup --bare`가 실패했고 일회성 `wrapper --bare -- features list`는 성공했다. 별도로 계정의 데몬 `settings.json`만 손상시키면 일반 `features list`는 실행 전에 실패했다. [디스패처와 setup](../../crates/wrapper/src/main.rs) · [애드온 설정](../../crates/wrapper/src/extensions.rs) · [데몬 설정](../../crates/wrapper/src/daemon_settings.rs)
- **정책 결정.** 구현 전에 정상·문법 오류·안전하게 읽을 수 없는 파일별로 일반 실행, 일회성 bare, status/doctor, 설정 변경, 데몬 관리 명령의 결과를 CLI 계약에 적는다. 제안은 선택 모듈 설정 오류를 표시하면서 독립적인 기본 실행과 상태 조회를 제공하고, 기존 설정을 보존할 수 없는 쓰기 요청은 명시적으로 실패시키는 것이다. 데몬 자동 업데이트 억제가 필요한 경로는 그 정책을 보장할 수 있을 때만 실행하도록 구분한다.
- **완료 판정.** 두 파일을 각각 손상시킨 뒤 확정한 명령별 종료 코드, 출력과 실제 업스트림 호출 여부를 검사한다. 읽기·진단 경로가 손상 파일을 덮어쓰지 않아야 한다. 정상 설정의 빠른 도움말·버전 조회와 명시적 모듈 선택의 기존 동작도 확인한다.

## R1 동작 보존 리팩터링

release-builder의 안정 버전 판정을 래퍼에서 재사용하고, 공개 진입점에서 한 번 파싱한 `UpdateAction`을 실행 함수에 넘긴다. 임시 소스 복사본에 두 변경만 적용해 `cargo test --offline --locked --workspace`에서 Wrapper 53개와 release-builder 8개 통과를 확인했다. 이 결과는 두 정리 작업의 실현 가능성에 해당한다. [버전 판정](../../crates/release-builder/src/lib.rs) · [UpdateAction](../../crates/wrapper/src/update.rs) · [호출자](../../crates/wrapper/src/main.rs)

실제 반영 시 기존 버전 입력의 수락·거부, 중복 및 충돌 옵션, 종료 코드, `--check`·도움말의 무변경 동작을 비교한다. 01–10의 동작 변경과 분리해 검토할 수 있는 단위로 제출한다.

## 검증과 인계

재현과 수정 검증은 각 실행이 소유하는 임시 `HOME`, `CODEX_HOME`, `PREFIX`, `TMPDIR`에서 수행한다. 세대 A와 B는 버전과 ID를 다르게 두고, 각 테스트는 종료 코드에 더해 실제 실행 파일의 버전, 포인터 바이트, 자식 호출 기록 등 해당 작업의 효과를 확인한다. 같은 홈에서 동시 작업을 검증할 때는 테스트 프로세스끼리만 그 홈을 공유한다.

변경별 회귀 테스트와 워크스페이스 테스트를 함께 실행한다. 설치·복구 변경에는 기존 설치기 복구 스위트, ELF 변경에는 AArch64 실행 및 공식 패키지 격리 검증을 적용한다. 인계에는 소스 SHA, 사용한 입력, 실행 환경, 통과·실패 항목, 남은 검증을 기록한다. 실제 기기 검증을 완료한 경계는 기존 증거 원장에, 책임이나 공개 경로 변경은 구현 지도와 해당 계약에 반영한다. [설치 복구 스위트](../../.github/scripts/test_install_recovery.py) · [공식 패키지 검증 도구](../../.github/scripts/qualify_wrapper.py) · [증거 원장](../../CURRENT.md)
