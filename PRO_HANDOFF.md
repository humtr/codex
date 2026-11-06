# Pro 인계: 기존 기능을 보존하는 구조 평탄화

## 1. 이번 인계의 범위

사용자의 최신 지시는 Codex가 기본 소스·증거를 정돈한 뒤 Pro가 능동적으로
설계·구현을 인수하는 것이다. 이 준비에서는 운영 앱·서비스를 바꾸거나 새 Native
연구 작업을 생성하지 않는다. 필요한 동작을 기준으로 변경을 선별해 한 기준에
수렴시키며, 보관 브랜치를 먼저 모두 합치는 방식은 사용하지 않았다.

**이 준비안과 기준 브랜치 자체도 Pro의 검토 대상이다.** Pro는 더 단순하고
일관된 방법을 찾으면 기준 소스·통합 순서·책임 배분·구현을 재선정하고,
준비안을 수정하거나 폐기할 권한과 책임이 있다. 기존 코드·문서·이전 제안에
있다는 이유로 유지하지 않는다. 선택 근거를 짧게 알린 뒤 승인된 범위에서
진행하며, 통상적인 설계 선택마다 재승인을 기다리지 않는다.

**최우선은 아래 P0 앱/CDP 실패 후 같은 작업의 입력 복구다.** 이번 준비는
사용자 제공 장애를 기존 평탄화의 우선 실증 사례로 통합하는 문서 변경이다.
장애 원인 규명·구현·실제 제품 복구를 이미 마쳤다고 주장하지 않는다.

사용자가 말한 평탄화는 기능을 유지하면서 중복된 결정, 상태, 준비, 실행,
복구 경로를 줄이는 것이다. 실제 생산자·결정 주체·소비자·수명과 보호해야 할
계약을 확인해 책임 경계부터 판단한다. 필요한 변경을 한 사용자 흐름씩 완결하고,
대체된 코드·상태·설정·소비자·시험·문서를 같은 변경에서 정리한다.
행동 보존 리팩터링과 동작 수정은 구분한다. 파일/줄 수 자체, 새 래퍼의 이름,
예외마다 추가하는 경로는 개선의 근거가 아니다.

## 2. 저장소와 브랜치

저장소: <https://github.com/humtr/codex>

| 역할 | 브랜치 / 기준 커밋 | 사용 방법 |
| --- | --- | --- |
| **준비 기준 브랜치** | `prep/pro-baseline-20261106` | Pro 작업 브랜치의 출발 후보. 미완성 catalog 초안을 분리하고 이 문서·운영 사실을 맞췄다. Pro는 다른 출발점이 더 적절하면 재선정할 수 있다. |
| 이전 전체 인계 보존 | `fix/native-route-transition-20261105` / `eeb6f9d808cd2dc7319a3d94f07010d650f3d281` | Native 초안까지 포함했던 원본 인계. 강제 변경 없이 보존했다. |
| 마지막 기록 기준 | `c340237982102a100de81aa70aca60b0617477eb` | 현재 복구 전의 기록 기준. 이후 인계 변경을 함께 읽는다. |
| 미완성 Native 초안 | `draft/thread-model-catalog-20261106` / `8d095cd754faf9f4ee8707f176d9648431bc08da` | thread-scoped model/list 백엔드·계약 초안. 컴파일/스키마/UI/수용 검증 미완료. 준비 기준에서는 적용을 되돌렸으며 원본은 이 브랜치에 보존했다. |
| 원래 로컬 작업 보존 | `handoff/pro-local-baseline-20261106` / `b8c14252ee338a75ace4cf3bfdc463d9747108c5` | 원래 dirty0814be7b의 모든 추적 변경과 신규 소스를 보존했다. 주 개발 브랜치에 무조건 합치지 말고 필요한 차이를 검토한다. 원래 작업 폴더는 그대로다. |
| **채택하지 않은** 모바일 런처 시도 | `archive/mobile-desktop-attempt-20261106` / `8de07907b1108bdc7ee7646e1ea55faa1ae14dec` | 현재 미채택인 비교·복원 자료. Pro는 채택 여부를 재평가할 수 있다. 기존 런처를 복제한 부분과 독립적인 viewer URL 수정은 구분한다. |
| **실제 작동하는 기존 Linux 소스** | `archive/linux-app-operating-source-20261106` / `97118e775bc6e7dbcebe20fdcb7f763ae1ee7278` | 설치된 source 파일과 원본 대비 변경을 보존했다. `experiments/web-provider`와 `novnc-sidebar.patch`를 본다. 앱·계정·인증 자료는 포함하지 않는다. |
| 책임 경계 재검토 | `design/entry-architecture-20261105` / `604df76bb30f361543246a7face53004b1f3d55a` | 기존 경계도 재검토한다는 설계의 근거. 구현 완료 증거는 아니다. |

`rewrite/rust-core`는 구현/재사용 가능한 릴리스 로직의 소유자이고 `main`은
검증된 정확한 소스를 가리키는 호출자다. 이 인계는 stable 승격이나 배포가 아니다.
공개 이력과 자산을 강제 덮어쓰지 않는다.

먼저 `AGENTS.md`, `SPEC.md` 4.5–4.6, `CURRENT.md`,
`providers/chatgpt/LINUX_APP.md`, 실제 diff와 호출자를 읽는다. 원래 로컬
스냅샷의 AGENTS/SPEC에는 이전 경계 문구가 남아 있으므로 그 스냅샷을 최신
주 개발 계약으로 오인하지 않는다.

### 기본 비교 결과와 미통합 변경

준비 소스의 Native patch/SPEC에서 미완성 `8d095cd`만 되돌렸다. Native patch는
`c340237`과 동일하며, SPEC에는 이후 P0 복구 요구를 명확히 했다. 다른 제품
구현에는 변경을 넣지 않았다. 이것은 전체
검증 완료 주장이 아니며, 초안의 분리 결정도 Pro가 재검토할 수 있다.

원래 로컬 스냅샷49개 경로를 `09ff18e`와 Git 3-way로 비교했다.
26개는 병합 미리보기에서 추가 delta가 없었다. Core 업데이트·frontend·MCP·
Native claim과 관련 시험 등이다. 나머지23개에는 아래 계정/앱 수명 기능과
직접 충돌8개가 남았다. Git의 자동 병합 성공을 의미상 정합성의 증거로 보지 않았다.

| 남은 변경 묶음 | 경로 | 판단 소요 |
| --- | --- | --- |
| 계정 선택·프로필·앱 수명 | `providers/chatgpt/src/{accounts,app,app_bridge,control,host,journal,lib,main,operator,runtime,viewer}` | 계정 선택/보존, 앱 소유와 정지·전환 경계를 함께 판단해야 한다. 일부만 기계적으로 옮기지 않았다. |
| 원래 터미널 복귀 | `crates/manager/src/lib.rs`, `crates/manager/tests/window_paths.py` | 계정/로그인 제어의 실제 호출자와 함께 볼 기존 내부 명령이다. |
| 관련 시험·실증 | `providers/chatgpt/app_bridge.test.mjs`, `tests/{native_client,operator_native,owned_host,runtime_cli}.rs`, `scripts/qualify_provider_continuity.py` | 위 기능과 기대 동작을 함께 판단한다. 시험만 복사해 합격 범위를 꾸미지 않는다. |
| 계약·안내·이전 작업 문맥 | `SPEC.md`, `CURRENT.md`, `providers/chatgpt/README.md`, `goal.md` | 오래된 경계·우선순위·운영 상태를 최신 계약으로 덮어쓰지 않는다. 고유 요구는 보존한다. |

직접 충돌한 경로:

- `CURRENT.md`
- `SPEC.md`
- `providers/chatgpt/src/app.rs`
- `providers/chatgpt/src/app_bridge.js`
- `providers/chatgpt/src/operator.rs`
- `providers/chatgpt/tests/operator_native.rs`
- `providers/chatgpt/tests/runtime_cli.rs`
- `scripts/qualify_provider_continuity.py`

이 비교는 작업트리에 충돌 표시를 넣지 않는 `git merge-tree` 미리보기였다.
현재 기준에는 해결되지 않은 Git merge가 없다. Pro는 해당 묶음을 원본과
비교한 뒤 더 나은 구현을 선별·통합·대체한다. 이 준비가 선택을 구속하지 않는다.

## 3. 작업·세션 식별자와 기록

| 종류 | 실제 확인된 값 / 한계 |
| --- | --- |
| 이 작업을 수행한 Codex 세션 | `01a1a5d8-9652-7220-a958-be6349e07419` — 실행 환경의 THREAD_ID/SESSION_ID에서 확인 |
| 이전 Pro 생성 요청 | `pro-bridge-design-20261105-c554880a84cf-create` |
| 그 요청의 회수된 work_id | `81f001ccecc1753339b40098bffc5a3e60c57dd63b14a0ea` |
| 관련 Native thread | `01a1a98a-b913-7542-86a5-e9a2fdac3e68` |
| 이전 Pro 연구 결과 | 해당 할당은 과거 기록상 생성만 회수됐다. 실제 연구·start/attach 완료로 취급하지 않는다. 직전 점검에서는 실행 전 미정리 할당이며 정상 Stop도 거부됐다. 아래 소유권 항목을 참고한다. |

Pro는 자신의 실제 실행 세션 ID, 관련 Native thread/work_id, 소스 브랜치와
커밋을 연구 시작 시 기록한다. 새 세션의 ID를 과거 할당 ID로 대신하지 않는다.
기존 생성 요청의 결과를 회수할 때만 같은 call_id와 같은 인자를 유지한다.
위 ID는 출처 참고용이다. Pro가 이 할당을 자신의 실행 세션으로 사용하거나
claim/join해야 하는 선행 조건은 없다. 이미 자신의 원격 실행 환경이 있으면
불필요한 Native 할당을 추가하지 않는다. Bridge Native 도구가 소스 접근 수단인
환경에서는 본인 연구용 할당을 명시적으로 하나 만들어 정상 도구 경로를 사용한다.

직전 소유권 정리에서 예전 Codex 작업3건은 정상 Stop과 host Stop을 확인했고
각각 remaining_jobs=[]였다. 상태 목록에서도 stopped를 확인했다:
`5c43343f6434ae9f8ece2803e4b8be3d88ded513824c5858`,
`385d5be485c78d0a6cb665dfcc010311949ce91f045a55bb`,
`a65b56a18de7759ff15e35d9d3076ef284d76bdb1345871e`.
Native/Provider 활성 효과 예약과 미정산 외부 영수증은0건이었다.
이 관찰이 이후 다른 작업의 상태까지 보장하지는 않는다.

이전 Pro 할당81f001…은 옛 소유 프로세스가 종료됐고 native_turn/pending/
도구 receipt/driver가 없었다. 해당 실행 홈의 rollout을 찾지 못했고 Native의
thread/read는 -32600 `thread not loaded`로 거부됐다. Provider stop은 그보다
앞선 binding/prepare_thread 조회에서 막혀 `native_request_rejected`를 반환했다.
`reconciliation_required` 기록은 강제 수정하지 않았다. 이는 Linux 앱 로그인
문제가 아니며, 실행 전 할당과 Native 영속화/회수 계약 및 오류 전파의 개선 과제다.
현재 Codex 대화 writer는 별개로 남아 있다. 모든 세션의 소유권을 해제했다고
주장하지 않으며, Pro는 기존 작업의 제어권을 임의 취득하지 않는다.

보고서는 작업 브랜치의 `PRO_REVIEW.md`에 작성하고, 원본 증거는 소유한 작업
경로에 보존한다. 기기 측 연구 보고서는 기존 목적지
`~/.cache/codex/acceptance/bridge-design-pro-review/REPORT.md`와 연결한다.
관찰 사실/가정/추론/권고, 변경한 책임 경계, 대안 비교, 추가·삭제한 경로,
검증 입력과 결과, 미수행 항목, 남은 제약과 다음 작업을 구분한다.
원본 개인 대화, 인증정보, capability와 원시 세션 로그는 Git에 올리지 않는다.

## 4. 현재 작동하는 환경과 증거의 한계

- **Linux 앱은 정상 실행·로그인 상태라고 사용자가 확인했다.** 기존
  `desktop_lifecycle.py`/`linux_desktop_trial.py`와 저장된 프로필로 복구했다.
  빈 새 홈을 보고 기존 로그인이 없다고 판단했던 이전 진단은 잘못됐다.
- 기존 런처 선택은
  `~/.local/share/codex-web-experiment/native-provider/runtime.json`, 현재
  데스크톱은 `~/.local/share/codex-web-experiment/linux-desktop/session.json`,
  저장 홈은 그 디렉터리의 `home`이다. 구체 경로는 설정에서 읽는다.
- 기존 구현은 설치된 `source`에 있고, Git 원본은 설치 기록의
  `sourceBase=9a5436a64d16045385358c6de18ab8e5b6699a57`의
  `experiments/web-provider`에서 찾을 수 있다. 현재 체크아웃에 없다는 이유로
  모바일 화면·태스크바·Android 브라우저 로그인 코드를 다시 만들지 않는다.
- 운영 서비스는 호환 Provider `0a6518c89417c0d7c727a771ab87f32d32bb856a`의
  보존된 바이너리로, **현재 데스크톱 descriptor에 명시적으로 연결한 복구 상태**다.
  최신 Native 개발 후보를 배포한 상태가 아니다. 앱 재시작 후에도 연결을 유지하는
  정식 수명 통합은 남았다. Native/MCP 부팅을 optional 앱에 종속시키지 않는다.
- 현재 선택·복구 전 실행 명령·롤백 자료는
  `~/.cache/codex/acceptance/connectivity-app-repair-20261105`의
  `activation.json`, `info.json` 등에 있다. 비공개 제어값은 출력하지 않는다.
- 기존 `scripts/d3_refresh.mjs`로 앱 캐시 무효화, **17개 전체 입력 스키마 일치**,
  실제 MCP 조회를 완료했다. 이후 순차 Provider host 관찰에서 ready=true였다.
  이 복구 상태에서 새 Native create/start/attach/execute/finish와 실제 모델
  왕복은 아직 수행하지 않았다.
- 외부 CDP 릴레이는 한 번에 한 연결만 받는다. 직접 D3 관찰과 Provider 관찰을
  동시에 실행하여 생긴 점유 실패를 앱 고장·로그아웃으로 오진하지 않는다.
- noVNC는 기존 초기화의 `UI.toggleControlbarSide()`로 오른쪽에서 시작하도록
  **현재 비공개 설치 자산만** 변경했다. 원본 자산·매니페스트·적용 patch는 복구
  디렉터리의 `novnc-sidebar`에 있다. 설치 매니페스트는 변경된 자산과 일치한다.
  유지보수 원본/업데이트 경로에 재현 가능하게 반영하는 작업은 남았다.
  앱은 재시작하지 않았으며 오른쪽 위치의 물리 화면 확인은 수행하지 않았다.
- 이전 SSE429의 구체적 제한 주체는 확정하지 못했다. 그 오류를 TUI/Native
  실패와 같은 원인으로 단정하지 않는다.

원격 Git 소스 분석·정리·로컬 시험·커밋/푸시는 Termux Bridge 없이 진행할 수
있다. 실제 Android/Linux 앱·운영 커넥터·기기 모델 실증에는 Bridge 등 기기 접근
수단이 필요하다. Bridge가 제공되면 실제 도구 schema와 읽기 전용 상태부터 확인한다.
사용할 수 없으면 독립적인 소스 작업을 진행하고 기기 의존 결과만 미수행으로 남긴다.
정상인 앱을 다시 설치하거나 반복 모델 요청으로 연결을 시험하지 않는다.

Bridge로 새 연구 작업이 필요할 때만 자신의 현재 Chat에 새 할당을 만든다.
이미 추론 중인 Chat의 붙은 작업은 실제 schema에 따라 inference=attached를
사용하고 별도 모델 추론을 시작하지 않는다. 같은 call_id/인자의 불확실한 결과를
회수하며, start→context receipt 확인/수락→실행 순서와 원래 결과를 보존한다.
격리된 실증 세션은 필요한 사례에만 만들고 기존 work/thread를 복제·위장하지 않는다.

### P0: CDP 타임아웃과 같은 작업 재입력 복구

#### 기존 범위와 추가 소요

우선 과제3·4·6·9와 NC-27·28·29에는 연결 수명, 원본 결과·미확정 효과,
입력 초안 보존이 포함돼 있었다. 아래 실제 장애 순서, 전송 전/후 복구의
해제 조건, 승인 검토의 별도 경로 및 여섯 회귀 사례는 구체적으로 없었다.
이들을 기존 책임 경계 리팩터링의 **P0 구현·수용 범위**로 추가한다.
별도 복구 제품/실행기나 결함 후보 등록만으로 완료하지 않는다.

#### 사용자 제공 관찰과 현재 확인 범위

- 2026-11-06 운영 Codex TUI에서 ChatGPT 새 작업이
  `stream disconnected before completion: cdp_operation_timeout`으로 실패했다.
  이후 같은 작업의 새 입력은 `ChatGPT input synchronization failed:
  operator_reconciliation_required; draft retained`로 차단됐다.
- Bridge에서는 Native create→start→attach→execute와 읽기 전용 명령의 exit0,
  AppHost present/ready=true, 프로젝트 목록과 모델 목록 조회 성공이 보고됐다.
  당시 `chatgpt_desktop` 주소는 `127.0.0.1:45341`이었다. 현재 설정을 재확인하고
  이 포트·계정 식별자를 구현 상수로 넣지 않는다.
- 제공된 Provider 기록은09:51경 `app_lane_open_uncertain`, phase=opening과
  09:52경 `cdp_operation_timeout`, phase=new, pending=[], round="",
  user_messages=0, dispatches=0, host_reply_messages=0이다. 후자에는 앱 신원은
  있으나 프로젝트 바인딩은 없었다. 두 스냅샷의 Lane/요청 연관성부터 확인한다.
  로컬 필드만으로 앱에 전달되지 않았다고 확정하지 않는다.
- history.mode=project에 기존 프로젝트 ID가 있었다고 보고됐다. 준비 소스의
  `cdp.rs`는15초 요청 제한과 Runtime.evaluate awaitPromise=true를 사용하며,
  `project.rs::resolve`는 기존 ID에 project_read를 호출한다. 이 소스 사실은
  확인했으나 당시 명령별 시간/결과가 없어 project_read는 원인 가설이다.
- 새 Rust 빌드 시도는 `Automatic approval review failed: stream disconnected
  before completion: app_target_unavailable`로 **실행되지 않았고**, 후속 읽기
  전용 Native 명령은 성공했다고 보고됐다. 승인 검토 대상/모델/요청 상관관계를
  확인해 A/B와 같은 원인인지 별도 Target 선택 문제인지 판정한다. 승인을
  우회하거나 전체 Native 실행기 장애로 일반화하지 않는다.
- 이 추가 작업에서 운영 journal/실제 모델/TUI를 다시 조사·재현하지 않았다.
  운영 Provider는 보존된 호환 복구 바이너리이므로 현재 후보 소스/격리 빌드와
  같다고 가정하지 않는다. 실행 이미지·출처·선택 설정을 실제 프로세스와 대조한다.

#### 조사·구현 계약

먼저 기존 worktree·브랜치·HEAD·diff·실행 작업과 제어권을 조회한다.
`implementation`은 원래 후보이며 현재 준비 후보는 `pro-baseline`이다.
진행 중 작업의 연속성을 보존한다. 불필요한 clone/checkout/별도 구현 경로를
만들거나 reset/clean/강제 병합하지 않는다. 별도 Pro 브랜치는 깨끗하고 다른
작업이 쓰지 않는 기존 트리에서 시작할 수 있을 때 만들고, 아니면 안전한 통합
방법부터 정한다. 다른 활성 작업의 checkout/브랜치를 바꾸지 않는다.

실제 호출자와 데이터를 다음 끝까지 추적한다:
TUI 입력→Native 수락→Provider 준비→CDP 오류→원본 상태 기록→같은 작업 재입력
→synchronize/reconciliation→Native 후속 수락 또는 구체적 안전 차단.

| 책임 | 우선 조사 경로와 확인할 사실 |
| --- | --- |
| CDP/AppHost | `src/{cdp,app,host}.rs`, `src/app_bridge.js`: 큐 대기/전송/응답 매칭과 명령별 경과, 비동기 조회와 evaluate 완료의 결합, Target/Session/React Scope/Composer 선택, 홈·프로젝트·대화 화면, 단일 CDP 연결 점유와 재생성 뒤 원래 앱 작업의 지속/완료. 연결·대상·인증·프로젝트·입력 가능을 각각 관찰한다. |
| 준비·외부 요청 수명 | `src/{project,app,server}.rs`: 신원→프로젝트→Lane→모델 전송 순서와 실제 전달 경계. timeout은 취소/비전달 증거가 아니다. 앱 작업 지연과 완료 후 응답 유실을 구분하고 필요한 최소 계측으로 최초 실패를 판별한다. |
| work/효과 정산 | `src/{operator,operator_native,native_claims,mcp_bridge}.rs`: New/Opening/Idle/Dispatching/Running/Unknown과 reconciliation_required의 생산자·소비자·정확한 진입/해제 조건, controller/work 수명, Native 효과와 앱 요청 상관관계를 확인한다. `src/`는 `providers/chatgpt/src/`이며 이동했다면 실제 소스를 따른다. |
| Native/TUI | 유지보수 입력 준비·Core 진입·route/client/catalog·owner/observer 호출자: Native 수락과 TUI draft 소비, 원본 call/input ID, 후속 입력의 실패 상태 상속, 잘못된 새 work/controller 생성, 복구와 barrier 순서를 확인한다. |

상태 분석은 최소한 아래 여덟 지점을 구분한다. 단계마다 **확정 사실·미확정
사실·관찰 권한을 가진 구성요소·정상 복귀 또는 차단 조건**을 실제 소스/증거로
작성한다. 여덟 enum이나 별도 저장소를 새로 만들라는 요구는 아니다.

1. 앱 신원 조회 전 실패.
2. 프로젝트 조회·검증 중 실패.
3. Lane 생성 요청 전 실패.
4. Lane 생성 요청 전달 여부 불확실.
5. Lane 생성 확인, 모델 입력 미전송.
6. 모델 입력 전송 여부 불확실.
7. 모델 입력 수락 확인, 스트림 관찰 실패.
8. 모델 응답/도구 결과 최종 확정 실패.

CDP/대상 관리는 CDP/AppHost, 원본 앱 요청 전달·관찰·정산은 Provider,
스레드·입력·writer·승인·도구 효과는 Native가 소유한다. TUI는 확정 상태와
초안을 표현하고 Core는 Termux 실행 적응을 담당한다. 입력 동기화는 Provider의
정상 정산 결과를 소비한다. 잘못된 경계는 근거와 함께 계약부터 수정하되
두 번째 실행기/복구 조정자나 독립 transcript를 만들지 않는다.

기존 thread/work/history/route/model, 승인, 원본 call/input/Lane/round 식별자와
이미 실행한 효과를 보존한다. 원래 Lane/round를 재관찰해 안전이 입증되면
소유 상태를 갱신하여 같은 작업을 계속하게 한다. 실제 미확정 효과는 차단을
유지하고 구체 사유·가능 조치를 표시한다. 모델 입력의 비전달이 확인됐는데도
일반 reconciliation이 영구 차단하면 결함이다. pending/dispatches=0만 보고
기존 불확실성을 지우지 않는다. 원본 모델 요청을 새 ID로 자동 재전송하지 않는다.
실패 입력의 자동 처리/사용자 재제출과 draft 소비 시점은 실제 Native 수락 계약으로
결정한다. 초안 보존만으로 정상 복구를 주장하지 않는다.

단순 timeout 연장, 무제한 재시도, 오류 무시, 상태/승인/소유권 강제 초기화,
다른 계정/모델 자동 전환은 해법이 아니다. timeout 정책 변경이 필요하면 작업별
정상 실행 시간을 먼저 실측한다. 같은 책임의 중복 준비·재연결·재시도는
실제 소비자를 확인해 통합하고 대체된 코드·상태·설정·시험·문서를 함께 제거한다.

#### 필수 회귀와 실제 제품 수용

| 사례 | 주입 경계와 관찰할 결과 |
| --- | --- |
| 1. 모델 전송 전 timeout | 프로젝트 조회/준비 응답을 지연한다. 비전달 사실·작업·입력을 보존하고 연결 회복 뒤 같은 작업에서 후속 입력을 수락한다. 영구 차단/중복 제출이 없어야 한다. |
| 2. Lane Open 응답 유실 | 앱의 Lane 생성 뒤 ACK를 잃는다. 동일 Lane 식별자로 원본 상태를 회수하며 불확실한 새 Lane을 만들지 않는다. |
| 3. 모델 제출 ACK 유실 | 앱이 원본 요청을 수락한 뒤 응답을 잃는다. 모델을 재전송하지 않고 원래 round를 관찰/정산하며 완료·미확정·실패를 정확히 구분한다. |
| 4. timeout 뒤 같은 작업 재입력 | 실제 시작→오류→상태 기록→TUI 새 입력→동기화/복구→후속 수락을 재현한다. 해결 가능한 상태에서 generic reconciliation이 반복되지 않으며 실제 위험은 구체적으로 차단한다. draft 소비와 Native 수락을 함께 검증한다. |
| 5. Composer/Target 부재와 승인 검토 | Target은 있으나 Composer가 없는 화면과 Target 소실을 분리한다. 자동 승인 검토의 실제 대상 경로와 미실행을 확인하고 승인 정책을 유지한다. |
| 6. 다중 진입점 | TUI/Native/MCP가 같은 원본 work의 복구 결과를 각 책임대로 관찰하고, 별도 이력/Provider 실행 경로 없이 계속 작업한다. |

기존 격리 인프라에서 최초 실패→수정 후 성공을 남긴다. 원본 요청/모델 제출과
실제 도구 효과 횟수·최종 입력 수락을 검증한다. 오류 문자열 비교나 성공을
가정한 mock만으로 완료하지 않는다. 실질 회귀는 격리 소스의 결함 주입으로
실패/복원을 확인한다. 이미 통과한 불변 입력의 대형 CI는 다시 시작하지 않는다.

이후 담당 구성요소 빌드/연결→보존 프로필의 분리 AppHost/CDP→실제 Codex TUI
→실패/복구→동일 작업 후속 입력과 최종 모델 응답을 확인한다. 원래 기록/입력/
선택과 중복 효과 부재를 소비 경계에서 확인한다. 실제 모델 시험은 기존 승인과
유한 비용/할당량 범위를 확인하고 필요한 범위만 수행한다. 별도 Pro 추론은
시작하지 않는다. Bridge가 끊기면 안전한 소스 작업을 계속하고 기기 결과만
미수행으로 남긴다. 승인 검토 실패는 승인으로 바꾸지 않는다.

운영 교체는 기존 서명·배포·롤백 계약을 따른다. 갱신 후 현재 소유한 CDP 연결과
정확한 connector/Provider/state로 `scripts/d3_refresh.mjs`의 전체 입력 스키마
새로고침을 완료하고 새 Native create/start/attach/execute/finish를 확인한다.
단일 CDP 관찰자를 경쟁시키지 않고 기존 앱·프로필·대화를 초기화하지 않는다.
소스/대역/기기/실제 모델/TUI/서명/운영 활성화는 각각 증거를 구분한다.

`goal.md`에는 활성 범위만, `CURRENT.md`에는 확인된 사실과 미완료만 유지한다.
`PRO_REVIEW.md`와 기존 기기 REPORT.md에 다음을 보고한다: 최초 확인 실패와
근본 원인, reconciliation 진입/해제 전이, A/B/C의 공통/독립 판정, 필요한
보호와 불필요 차단, 수정 책임·통합/삭제/호환, 최초 실패/수정/결함 주입 결과,
실제 TUI 재입력·최종 모델 응답·Native 연속성, 미완료 검증·배포·롤백 상태.

## 5. 보존된 검증: 다시 시작하지 말 것

- 실제 유지 패치의 준비 메타데이터가 정한 upstream을 읽는다. 이 작업의 기록상
  공식 기준은 `6ff1e1ca3b30fc8b914720681fcf5fe62e95f856` / Codex0.166.0이다.
- [Native workflow42025671701](https://github.com/humtr/codex/actions/runs/42025671701)은
  정확한 소스83838e8에서 API/initialize/catalog/compaction/Guardian/client/config와
  세 가지 결함 주입의 실패·복원 성공 증거를 남겼다. **전체 작업은120분 제한에
  걸려 TUI 컴파일 중 취소됐다.** 뒤의 게이트나 전체 성공을 주장하지 않는다.
- 그 보존된 unsigned CLI로 Android에서 빈/기록 있는/종료 후 cold Native 세션의
  동일-thread 전환, 원래 문맥, 실제 write/patch, 한 번만 효과가 나는 재시도,
  finish와 원래 route 복귀를 확인했다. primary inference는 비활성 대역이었다.
- Native 컴팩션 기기 사례8개도 통과 기록이 있다. 앱 답변 대역을 썼으며,
  원격 Chat 교체나 실제 ChatGPT 추론을 입증하지 않는다.
- Provider 소스 gate와114 JS 검사는 이전 범위에서 통과했다. 새로 보존한
  `model/list(threadId)` 초안까지 검증한 결과가 아니다.
- 새 모바일 런처 시도는 초기 기기 시험에서 태스크바 부재로 실패했다.
  이후 수정은 완전한 수용 검증을 마치지 않았으며 현재 운영에서 쓰지 않는다.

원본 결과는 기기의 `$TMPDIR/connectivity-repair-70jt4h2u/checks`, Native
기록과 보존 실행 파일은 같은 소유 작업 트리에 있다. 전체 CI를 처음부터
재실행하지 말고 변경된 입력과 아직 완료되지 않은 경계에 맞게 검증한다.
취소된 작업·0개 실행·소스 조립 성공을 통과 증거로 바꾸지 않는다.

## 6. 우선 정리·연구·구현할 내용

위 P0를 첫 사용자 흐름으로 완결하고, 공통 원인이면 아래3·4·6·9를 함께
해결한다. 기존12개/NC/누적 범위를 대체하거나 별도 예외 경로로 분리하지 않는다.

1. **기준 통합.** 브랜치별 계약·운영 문서·소스·실행 바이너리 차이를 정리한다.
   원래 로컬 스냅샷의 필요한 변경만 판별하고 사용자의 고유 변경을 보존한다.
   보관 모바일 시도는 현재 미채택 상태이며 채택 여부도 재검토한다. 상대 WebSocket URL 수정처럼
   독립적으로 필요한 조각은 원인과 실제 소비자를 확인해 살린다.
2. **책임 경계부터 재평가.** 실제 생산자→결정 주체→소비자와 시작/종료 주체를
   추적한다. 기존 경계가 잘못됐다면 먼저 SPEC을 고친다. 유지/개선/통합/이동/삭제를
   비교하고 한 책임에 한 구현을 남긴다. 새 조정자·registry·fallback을 기본 해법으로
   추가하지 않는다. 계정명·워크스페이스·설치 경로·connector 선택은 설정/발견 값이다.
3. **앱·프로필·연결 수명 평탄화.** 기존 런처와 저장 계정을 재사용하고, 임시
   descriptor 고정과 단일 CDP 점유 문제를 실제 연결 소유 책임에서 해결한다.
   앱 관찰·모델 발견·추론·유지보수가 연결을 경쟁해서 새로 만들지 않게 한다.
   실제 존재하는 계정별 모델 권한을 보존하고, Native/Manager 프로필과 앱 로그인을
   혼동하지 않는다. 실행 전 할당이 Native를 조회할 수 없을 때 회수에 도달하지
   못하는 현재 문제도 수명 계약에서 해결한다. 없는 세션 오류를 성공으로 바꾸는
   분기만 추가하지 않는다. 기존 명령/바이너리/서비스 선택도 함께 정합하게 만든다.
4. **연결부터 원래 결과 반환까지.** 갱신된 실제 클라이언트에서 새 격리 Native
   작업을 생성→시작→attach/receipt→실제 도구 실행→원래 결과→finish까지 잇는다.
   첫 도구 이후 계속 실행, Queue/Steer/Stop, 분리·재연결·서비스 교체·정확 재시도,
   불확실한 효과와 원래 호출자에 대한 완전한 최종 답변을 확인한다. 기능을 read-only나
   합성 시험으로 축소하지 않는다. 기존 승인·writer·원본 기록의 권위를 유지한다.
5. **Native route와 모델 목록 초안 완성/재판단.** 별도 draft 브랜치의 초안은 loaded thread의
   provider/ModelsManager를 coherent snapshot으로 읽는 백엔드까지다. 실제 생성기로
   schema를 갱신하고, TUI가 표시 중인 thread를 기존 비동기 모델 조회에 전달하고,
   설정 이벤트에서 기존 cache/correlation 경로로 갱신하도록 연결해야 한다.
   형식이 같은 구버전의 unknown-field 무시를 capability 지원으로 보지 않는다.
   특히 초안이 기존 initialize.modelProviderUpdates의 범위를 모델 목록까지
   확장하므로, route update만 지원하는 이전 서버와 구분할 수 있는지 재검토한다.
   예전 `support_verbosity` 누락처럼 실제 catalog producer/schema 불일치도 확인한다.
   정책·인증·형제 세션·기본 catalog와 저장/실패 의미를 보존한다.
6. **TUI 재시작/체크포인트 중복 제거.** warm Native transition과 겹치는
   약800줄의 process reentry/checkpoint/summary 조정을 그대로 두고 우회하지 않는다.
   Native가 route·context·observer를 한 경계에서 소유하도록 검토하고, 대체된
   코드·설정·상태·테스트를 함께 없앤다. draft, 터미널, 진행 중인 job과 이벤트는 보존한다.
7. **진입점·동일 세션의 일관성.** 아래 네 경로를 실제 입력/출력으로 구분해 확인한다.
   OpenAI→ChatGPT→OpenAI, Codex 진입점의 ChatGPT→OpenAI, ChatGPT/Bridge에서
   Native 기록을 만든 작업→Codex→ChatGPT, 독립 ChatGPT 대화→Bridge로 새 Native
   작업 생성→양방향 계속 작업. coherent task는 동일 Native thread를 우선하고,
   과거 provider는 출처이지 영구 금지 조건이 아니다. 무단 fork와 전환마다 분기하는
   악순환은 피한다. 기존 context import와 원래 출처·권한·정책은 정확히 유지한다.
8. **컴팩션과 원격 Chat 수명.** Native 자동/수동 컴팩션을 유일한 정식 context
   교체로 사용한다. Provider가 별도로 의미 요약하는 경로와 public projection을
   구분한다. V2 checkpoint, 공개 요약이 없는 경우, opaque/encrypted 기록, 이미지/파일,
   중단·취소·복구·모델 복귀를 다룬다. Native 컴팩션 후 관리되는 Chat의 새 대화 생성과
   context 주입, 외부 사용자가 소유한 Chat의 처리, 진행 중 MCP 호출의 원래 반환과
   교체 사이의 교착 가능성을 연구·실증한다. 임의 환경변수/상위 지시로 문맥을 승격하지 않는다.
9. **상태·claim·복구 경로 중복 제거.** Operator Record/driver phase/AppHost lane/
   MCP broker에서 Native가 이미 소유하는 사실을 독립적으로 재진행하지 않는다.
   실제 필요한 외부 호출 상관관계·정확 재시도·효과 미확정 상태만 남긴다.
   MCP와 Function은 같은 작업 의미를 공유하되 원래 invocation/lease/결과 반환의
   차이는 보존한다. 도구 호출 권한과 renderer 추론을 다른 실행기로 분기하지 않는다.
10. **자동 Chat 진입의 실현 가능성.** MCP 등록·프롬프트·보고 호출만으로 사용자
    입력/출력/분기/원래 메시지 수신이 보장되지 않는다. 실제 호스트 event/handoff
    접점을 연구한다. 모든 적법한 지원 모델/진입점의 일반 입력에서 자연스럽게 시작하되
    사용할 수 없는 모델 권한은 정확히 알리고 모델/effort를 조용히 바꾸지 않는다.
11. **설치·갱신·화면 설정 재현성.** noVNC 오른쪽 시작을 유지보수 원본과 갱신
    경로에 반영하고 기존 모바일/로그인 기능을 보존한다. 부분 설치·선택 구성·실행 중
    구세대 유지·롤백을 기존 generation 체계에서 정합하게 만든다. 새 refresh API를
    만들지 말고 기존 full-schema refresh를 사용한다. 공개 서명·readback·승격·기기
    활성화와 물리 UI 관찰은 각각 별도 사실로 기록한다.
12. **증거와 CI 정리.** 현재 미완성/정확한 입력/이미 통과한 범위를 구분한다.
    TUI 컴파일과 결함 주입 때문에 한 작업이 시간 제한에 걸린 구성을 검토하되,
    artifact exact-source 소비자와 수용 조건을 약화하지 않는다. 중복되거나 구조를
    고정하는 시험은 합치고 실제 실패 보호는 남긴다. 새 회귀는 격리 source에서
    결함 주입 실패와 복원 성공을 확인한다. 변경 없는 대형 CI를 반복하지 않는다.

## 7. 기존 NC-01..29의 남은 작업 전체

`CURRENT.md`의 행별 원본 증거가 기준이다. 아래는 누락 방지용 작업 목록이며
모든 행이 통과했다는 뜻이 아니다. 관련 경계부터 검토하고 상태를 갱신한다.

| 사례 | 남은 작업 |
| --- | --- |
| NC-01 | 같은 workspace의 두 work, parent/child, 두 transport까지 독립 실행 |
| NC-02 | orchestrator를 포함한 실제 동시 독립 읽기 |
| NC-03 | 읽기/opaque writer 겹침의 view 표기와 transport 조합 |
| NC-04 | 일관된 snapshot/content set 또는 명확한 실패; 혼합 버전 성공 금지 |
| NC-05 | write set 강제와 실제 분리된 동시 쓰기의 전체 경로 |
| NC-06 | 병렬 준비 후 비중첩 변경의 guarded merge |
| NC-07 | 겹치는 hunk 충돌과 양쪽 원본 기록 보존 |
| NC-08 | 활성화 후 일반 모델, 재시작/정확 재시도, 동시 관찰자와 단일 효과 |
| NC-09 | create/create, rename/edit, delete/edit 경합 |
| NC-10 | 형제 경로 쓰기와 parent 교체 중 admission |
| NC-11 | 재귀/metadata 범위, rename/delete와 live descendant |
| NC-12 | hardlink/symlink/path-swap/bind-mount; Android 제약의 실제 관찰 |
| NC-13 | nested repo/submodule/worktree와 공통 Git metadata 소유권 |
| NC-14 | non-login shell/startup/env/Git config의 inert 보장과 writer 중 status/diff/log |
| NC-15 | worktree별 index/HEAD/ref를 불필요하게 직렬화하지 않음 |
| NC-16 | index/ref/lock별 조정, expected HEAD/index, 자기 변경만 stage/commit |
| NC-17 | 격리 출력과 정확한 source를 갖는 실제 동시 Native build |
| NC-18 | 공유 compiler/package cache와 개별 생성 대상의 실제 잠금 의미 |
| NC-19 | work별 TMPDIR 효과; opaque 처리가 전체 build 병목이 되지 않음 |
| NC-20 | 재귀 cleanup과 live descendant 경합; 다른/불명확 소유 출력 보존 |
| NC-21 | 서로 다른 repo의 실제 동시 writer와 실제 공통 자원 충돌 |
| NC-22 | account/config/socket/runtime 변경의 동시성과 각 권한 경계 |
| NC-23 | 외부 부작용/출판 권한; 분리된 파일 경로가 원격 권한을 만들지 않음 |
| NC-24 | 실제 사전 충돌·원래 거부 재시도·뒤늦은 효과 없음; OS/staging/env 검증 |
| NC-25 | 외부 편집기와 publish 시점 경합; 보이지 않는 writer까지 막았다고 주장하지 않음 |
| NC-26 | 일반 모델의 연속 도구/원래 final, 동일 work의 독립 동시 효과 |
| NC-27 | 실제 동시 waiting/reconciliation과 무관한 작업의 진행 |
| NC-28 | 응답 유실·owner 재시작·Chat 분리·stale PID·실제 process/Stop 정착 |
| NC-29 | 복합 충돌·부분 효과·모든 원래 결과와 실행 가능한 소유자 진단 |

## 8. 그 밖의 누적 과제: 이번 정리에서 잃지 말 것

아래는 `CURRENT.md`의 기존 개발 순서/기능 표를 잇는 후속 범위다. 이번 평탄화와
무관한 항목을 한 번에 섞지 말고, 필요한 연결만 고친 뒤 나머지 상태를 보존한다.

- update의 변경된 구성만 표시, public/selected Provider sequence와 실제 이미지 정합성.
- Standalone/Integrated의 의도 선택, context/기록 범위, standalone Function 입구와
  실제 원래 입력·진행·도구·완전한 final의 일치. 개인 Chat을 배경 crawler로 수집하지 않음.
- 계정별 앱 로그인과 모델/project/Chat/connector cache 갱신, 허용된 터널 유지와
  dual-login 전환. 관리자 키·앱 로그인·Native 프로필의 역할 분리.
- P0/NC 이후 기존 refresh의 phase-neutral 제품 통합과 정확한 activation 경계.
- 확인된 구 명칭/형식/설정/패키지와 originality inventory의 미해결27개 항목 정리.
  의존성 라이선스, 고유 사용자 자료, 현재/롤백·공개 이력은 보존.
- Core 필수, Manager/TUI/Provider/patched tunnel의 독립 선택·Full 구성과
  선택적 build/update/reuse, in-repo tunnel 획득/패치/복구.
- 선택적5시간 quota window 시작: 실제 같은 account/bucket 중복 방지와 최소 실제 활동.
- Android 이미지+텍스트 clipboard→upstream composer/model, 다중 이미지·취소·snapshot/
  import/release. 파일/share fallback은 구분. 실제 APK/키/IME/알림/tap/focus도 별도 관찰.
- `/switch`/profile 전환과 형제 계정 상태, `/copy`·Ctrl+O의 완전한 UTF-8 bytes,
  default-off 키 복사, EDITOR/VISUAL save/cancel, 기존 시간대 지원 보존.
- 호출 예산은 고정75/1000 대신 실제 소비·미실행·권한·진행과 Stop으로 판단.
- 실제 Native subagent/parent/temporary-thread lineage와 별도 결과·공개 기록.
  순차 Chat 요청을 독립 worker로 부르지 않음. BTW는 기존 발견 단서부터 검토.
- first-run, Linux host, project/history/viewer/IME/touch, repo 분리, 장시간 동작의
  남은 사용자 경로. ENFILE·termination·model/effort 변화는 실제 provenance로 진단.
- 완료 배치 뒤 소유한 생성물만 기존 cleaner로 정리하고 원본 증거·소스·current/
  rollback 실행 파일을 보존. compiler cache는 약15GiB 이내. 무관한 재검증 금지.

## 9. Pro에게 보낼 실행 프롬프트

```text
ChatGPT 진입점에서 @TermuxBridge를 사용하여 humtr/codex의 책임경계 평탄화와
미완성 연결 작업을 인수해라. 최우선은 운영 Codex의 ChatGPT CDP timeout 뒤
같은 Native 작업의 입력 동기화가 막히는 P0를 원인 규명→수정→회귀→실제 복구까지
해결하는 것이다. 분석 보고나 결함 후보 등록으로 멈추지 마라.

저장소: https://github.com/humtr/codex
출발 후보: prep/pro-baseline-20261106의 최신 원격 HEAD
먼저 PRO_HANDOFF.md 전체(특히 P0), AGENTS.md, SPEC.md4.5–4.6,
CURRENT.md, goal.md, providers/chatgpt/LINUX_APP.md를 읽어라.
복구/컴팩션/인수 뒤에는 실제 AGENTS.md를 다시 읽되 통과한 검사를 재시작하지 마라.

실제 worktree·브랜치·HEAD·diff·실행 작업·소유권과 운영 이미지/설정을 확인해라.
$TMPDIR/connectivity-repair-70jt4h2u/implementation 및
fix/native-route-transition-20261105는 이전 후보이고 pro-baseline은 준비 후보다.
현재도 유효하거나 동일한 소스라고 가정하지 마라. 진행 중 작업의 연속성을
보존하고, 다른 작업이 쓰지 않는 기존의 깨끗한 트리에서 네 작업 브랜치를 만들어라.
불필요한 clone/checkout/별도 구현 경로, reset/clean/강제 병합은 금지한다.
네 실제 세션 ID, 선택한 기준 브랜치/commit, 관련 Native thread/work ID를 기록해라.
기준 변경이 필요하면 근거와 함께 기록하고 기존 변경을 보존해라.

이전 Codex 세션: 01a1a5d8-9652-7220-a958-be6349e07419
관련 Native thread: 01a1a98a-b913-7542-86a5-e9a2fdac3e68
이전 Pro work_id: 81f001ccecc1753339b40098bffc5a3e60c57dd63b14a0ea
이는 출처 참고다. 과거 할당을 네 연구 세션으로 위장하거나 claim/join부터 하지 마라.
Bridge Native가 도구 접근 수단이면 현재 Chat에 붙는 본인 연구 작업을 하나 만들고,
실제 schema의 inference=attached, 안정적인 call_id, start/attach/context receipt
수락 절차를 따른다. 생성 결과가 불확실하면 같은 ID/인자로 회수한다. 불필요한
할당이나 별도 Pro 추론은 시작하지 않는다. 격리 실증은 필요한 새 테스트 세션을 쓴다.

P0 관찰은 2026-11-06 사용자 제공 증거다. 운영 TUI ChatGPT 요청이
cdp_operation_timeout으로 실패한 뒤 재입력이 operator_reconciliation_required;
draft retained로 막혔다. 기록은 app_lane_open_uncertain/opening과 후속
cdp_operation_timeout/new, pending=[], round="", user_messages/dispatches/
host_reply_messages=0이며 후자에는 신원만 있고 프로젝트 바인딩은 없었다.
두 기록의 원본 Lane/round/call 연관성과 실제 전달 사실부터 확인해라.
Native create/start/attach/execute·읽기 exit0, host present/ready, project/catalog
조회는 성공했다고 보고됐다. 당시 Provider 주소127.0.0.1:45341을 상수화하지 마라.
운영 Provider는 별도 호환 복구 바이너리이며 후보 소스/격리 빌드와 다르다.
history.mode=project와 기존 project ID, 소스의15초 CDP 제한/awaitPromise=true/
project_read는 단서다. 명령별 시간 증거 없이 project_read를 원인으로 확정하지 마라.
자동 승인 검토의 app_target_unavailable로 빌드가 미실행된 별도 장애도 추적해라.
같은 원인인지 Target 선택 문제인지 판별하며 승인 실패를 우회하지 마라.

cdp/app/app_bridge/host/project/server/operator/operator_native/native_claims/
mcp_bridge와 실제 TUI/Core 호출자를 추적해라. CDP 큐·응답·단일 연결 점유,
Target/Session/React Scope/Composer, 화면 전환, 비동기 작업과 응답 유실을 구분해라.
TUI 입력→Native 수락→Provider 준비→오류→상태 기록→같은 작업 재입력
→synchronize/reconciliation→후속 수락 또는 구체적 안전 차단까지 확인해라.
New/Opening/Idle/Dispatching/Running/Unknown/reconciliation_required의
진입·해제 조건을 확인하고 P0 문서의 여덟 전달 단계를 실제 증거로 구분해라.
로컬 카운터0이나 연결 종료만으로 비전달/취소를 추정하지 마라.

CDP/대상 관리는 CDP/AppHost, 외부 요청 전달·재관찰·정산은 Provider,
Native 입력·writer·승인·도구·정식 기록은 Native가 소유한다. TUI는 확정 상태와
초안을 표현하고 Core는 Termux 실행 적응을 맡는다. 복구 가능한 원본 Lane/round를
확인해 같은 작업의 차단을 정상 종료하되 실제 미확정 효과 보호는 유지해라.
thread/work/history/model/route/승인과 원본 요청 ID를 보존하고 중복 모델 제출·
도구 실행·입력 손실을 막아라. draft 소비/재제출은 Native 수락 사실로 결정해라.
단순 timeout 연장·무제한 재시도·오류 무시·강제 초기화·자동 계정/모델 전환으로
해결하지 마라. timeout 정책 수정에는 정상 실행 시간의 실측 근거가 필요하다.

준비안과 기존 경계도 독립적으로 재평가해라. 더 간결한 구조가 있으면 기준·통합
순서·책임·구현을 재조정하고 SPEC/문서를 먼저 맞춰라. 공유 Native 실행과
공급자별 얇은 추론 연결부, 지원되는 대안과 최소 Native 수정의 비용/정확성을
비교해라. Provider를 openai로 이름만 맞추는 것은 호환성 증명이 아니다.
동일 세션의 양방향 전환과 bootstrap modelProviderMismatch도 기존 필수 범위다.
중복 결정·상태·준비·연결·복구를 통합하고 대체 코드·설정·호출자·시험·문서를
함께 정리해라. 별도 실행기/기록/복구 상태머신이나 증상별 래퍼를 늘리지 마라.
통상적 선택은 근거를 짧게 알리고 승인된 범위에서 계속해라.

P0 여섯 사례를 기존 격리 인프라에서 재현/검증해라: (1) 모델 전송 전 timeout,
(2) Lane Open ACK 유실, (3) 모델 제출 ACK 유실, (4) timeout 후 같은 TUI 작업
재입력, (5) Composer 부재와 Target 소실 및 승인 검토, (6) TUI/Native/MCP 상태
일관성. 원본 요청 관찰·입력 수락·한 번만 실행되는 효과를 검증해라. 최초 실패와
수정 후 성공, 실질 회귀의 격리 결함 주입 실패/복원을 남겨라. 오류 문자열 비교나
성공 mock만으로 완료하지 마라. 동일 입력의 기존 대형 CI는 다시 돌리지 마라.

이후 담당 빌드/연결→분리 AppHost/CDP→실제 Codex TUI→실패/복구→같은 작업의
후속 입력과 최종 모델 응답을 확인해라. 실제 모델은 기존 승인·유한 비용/할당량
범위를 확인해 필요한 만큼만 사용해라. 기기/앱/모델 실증에는 @TermuxBridge를
사용하고 연결이 없으면 독립 소스 작업을 계속하되 기기 실증은 미수행으로 남겨라.
기존 로그인·계정·브라우저·대화·실행 작업을 보존하고 이미 있는 모바일 런처·
태스크바·Android 로그인 연동을 재발명하지 마라. CDP 관찰자를 경쟁시키지 마라.
운영 교체는 기존 서명/배포/롤백 절차를 따르고, 현재 소유 CDP와 정확한
connector/Provider/state로 scripts/d3_refresh.mjs의 전체 입력 스키마를 갱신한 뒤
새 Native create/start/attach/execute/finish까지 확인해라.

P0를 우선 완결하고 기존12개 과제·NC-01..29·누적 backlog를 빠짐없이 판별해라.
관련 구조 문제는 함께 해결하되 무관한 과제를 한 패치에 몰아넣지 마라.
읽기·쓰기·계속 작업·양방향 전환·컴팩션·중단·재연결의 기존 요구를 보존해라.
goal.md는 간결히, CURRENT.md는 확정 사실과 미완료로 유지해라. PRO_REVIEW.md에
최초 실패/근본 원인, reconciliation 전이, A/B/C 공통·독립 판정, 필요한 보호와
불필요 차단, 책임 변경/대안/삭제·호환, 재현·수정·결함 주입 결과, 실제 TUI
재입력·모델 완료·Native 연속성, 미완료/배포/롤백을 기록하고 커밋/푸시해라.
기기 보고서: ~/.cache/codex/acceptance/bridge-design-pro-review/REPORT.md
소스/대역/Android/실제 모델/물리 UI/서명/활성화를 구분하며 인증정보·capability·
개인 원시 대화는 공개하지 마라. 완료 기준은 같은 작업을 중복 효과 없이
정상적으로 계속할 수 있는 것이다.
```
