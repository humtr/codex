# Pro 인계: 기존 기능을 보존하는 구조 평탄화

## 1. 이번 인계의 범위

사용자는 추가 구현을 멈추고 **최소 정리 후 소스 전체를 원격에 보존하여 Pro가
연구·정리·개선을 이어가도록** 지시했다. 이 인계에서는 앱/서비스를 바꾸거나
새 검증을 실행하지 않는다. 이전의 “일차 작업 완료 후 Pro” 순서는 이 지시로
대체됐다. 아래에는 현재 미완성 작업과 기존 누적 과제를 함께 적었다.

사용자가 말한 평탄화는 기능을 유지하면서 중복된 결정, 상태, 준비, 실행,
복구 경로를 줄이는 것이다. 기존 코드와 책임 경계도 검토 대상이다.
예외마다 새 경로를 붙이거나, 이름만 바꾼 래퍼를 만들거나, 필요한 기능을 없애
단순해졌다고 주장하지 않는다. 필요한 경계 검증은 실제 실패 가능성과 보호하는
계약으로 설명한다. 파일 수나 줄 수 자체를 목표로 삼지 않는다.

## 2. 저장소와 브랜치

저장소: <https://github.com/humtr/codex>

| 역할 | 브랜치 / 기준 커밋 | 사용 방법 |
| --- | --- | --- |
| **주 개발·인계 브랜치** | `fix/native-route-transition-20261105` | 여기서 Pro 작업 브랜치를 만든다. 이 문서와 최신 Linux 앱 운영 문서를 포함한다. |
| 마지막 기록 기준 | `c340237982102a100de81aa70aca60b0617477eb` | 현재 복구 전의 기록 기준. 이후 인계 변경을 함께 읽는다. |
| 미완성 Native 초안 | `8d095cd754faf9f4ee8707f176d9648431bc08da` | thread-scoped model/list 백엔드·계약 초안. 컴파일/스키마/UI/수용 검증 미완료. |
| 원래 로컬 작업 보존 | `handoff/pro-local-baseline-20261106` / `b8c14252ee338a75ace4cf3bfdc463d9747108c5` | 원래 dirty0814be7b의 모든 추적 변경과 신규 소스를 보존했다. 주 개발 브랜치에 무조건 합치지 말고 필요한 차이를 검토한다. 원래 작업 폴더는 그대로다. |
| **채택하지 않은** 모바일 런처 시도 | `archive/mobile-desktop-attempt-20261106` / `8de07907b1108bdc7ee7646e1ea55faa1ae14dec` | 비교·복원용. 통째로 채택하지 않는다. 기존 런처를 다시 복제한 부분과 독립적인 viewer URL 수정은 구분한다. |
| **실제 작동하는 기존 Linux 소스** | `archive/linux-app-operating-source-20261106` / `97118e775bc6e7dbcebe20fdcb7f763ae1ee7278` | 설치된 source 파일과 원본 대비 변경을 보존했다. `experiments/web-provider`와 `novnc-sidebar.patch`를 본다. 앱·계정·인증 자료는 포함하지 않는다. |
| 책임 경계 재검토 | `design/entry-architecture-20261105` / `604df76bb30f361543246a7face53004b1f3d55a` | 기존 경계도 재검토한다는 설계의 근거. 구현 완료 증거는 아니다. |

`rewrite/rust-core`는 구현/재사용 가능한 릴리스 로직의 소유자이고 `main`은
검증된 정확한 소스를 가리키는 호출자다. 이 인계는 stable 승격이나 배포가 아니다.
공개 이력과 자산을 강제 덮어쓰지 않는다.

먼저 `AGENTS.md`, `SPEC.md` 4.5–4.6, `CURRENT.md`,
`providers/chatgpt/LINUX_APP.md`, 실제 diff와 호출자를 읽는다. 원래 로컬
스냅샷의 AGENTS/SPEC에는 이전 경계 문구가 남아 있으므로 그 스냅샷을 최신
주 개발 계약으로 오인하지 않는다.

## 3. 작업·세션 식별자와 기록

| 종류 | 실제 확인된 값 / 한계 |
| --- | --- |
| 이 작업을 수행한 Codex 세션 | `01a1a5d8-9652-7220-a958-be6349e07419` — 실행 환경의 THREAD_ID/SESSION_ID에서 확인 |
| 이전 Pro 생성 요청 | `pro-bridge-design-20261105-c554880a84cf-create` |
| 그 요청의 회수된 work_id | `81f001ccecc1753339b40098bffc5a3e60c57dd63b14a0ea` |
| 관련 Native thread | `01a1a98a-b913-7542-86a5-e9a2fdac3e68` |
| 이전 Pro 연구 결과 | 해당 할당은 과거 기록상 생성만 회수됐다. 실제 연구·start/attach 완료로 취급하지 않는다. 현재 상태는 Bridge에서 조회한다. |

Pro는 자신의 실제 실행 세션 ID, 관련 Native thread/work_id, 소스 브랜치와
커밋을 연구 시작 시 기록한다. 새 세션의 ID를 과거 할당 ID로 대신하지 않는다.
기존 생성 요청을 회수할 때만 같은 call_id와 같은 인자를 유지한다.

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

실제 기기·앱·커넥터 작업에는 **Termux Bridge를 사용해야 한다.** 원격 Git
체크아웃만으로 Android 실행이나 앱 관찰을 수행했다고 주장할 수 없다.
Bridge가 없거나 실패하면 독립적인 소스 연구를 계속하고, 기기 의존 작업만
정확히 미완료로 남긴다. 연결을 확인한다며 모델 요청을 반복하지 않는다.

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

1. **기준 통합.** 브랜치별 계약·운영 문서·소스·실행 바이너리 차이를 정리한다.
   원래 로컬 스냅샷의 필요한 변경만 판별하고 사용자의 고유 변경을 보존한다.
   보관 모바일 시도는 주 경로에 합치지 않는다. 상대 WebSocket URL 수정처럼
   독립적으로 필요한 조각은 원인과 실제 소비자를 확인해 살린다.
2. **책임 경계부터 재평가.** 실제 생산자→결정 주체→소비자와 시작/종료 주체를
   추적한다. 기존 경계가 잘못됐다면 먼저 SPEC을 고친다. 유지/개선/통합/이동/삭제를
   비교하고 한 책임에 한 구현을 남긴다. 새 조정자·registry·fallback을 기본 해법으로
   추가하지 않는다. 계정명·워크스페이스·설치 경로·connector 선택은 설정/발견 값이다.
3. **앱·프로필·연결 수명 평탄화.** 기존 런처와 저장 계정을 재사용하고, 임시
   descriptor 고정과 단일 CDP 점유 문제를 실제 연결 소유 책임에서 해결한다.
   앱 관찰·모델 발견·추론·유지보수가 연결을 경쟁해서 새로 만들지 않게 한다.
   실제 존재하는 계정별 모델 권한을 보존하고, Native/Manager 프로필과 앱 로그인을
   혼동하지 않는다. 기존 명령/바이너리/서비스 선택도 함께 정합하게 만든다.
4. **연결부터 원래 결과 반환까지.** 갱신된 실제 클라이언트에서 새 격리 Native
   작업을 생성→시작→attach/receipt→실제 도구 실행→원래 결과→finish까지 잇는다.
   첫 도구 이후 계속 실행, Queue/Steer/Stop, 분리·재연결·서비스 교체·정확 재시도,
   불확실한 효과와 원래 호출자에 대한 완전한 최종 답변을 확인한다. 기능을 read-only나
   합성 시험으로 축소하지 않는다. 기존 승인·writer·원본 기록의 권위를 유지한다.
5. **Native route와 모델 목록 초안 완성/재판단.** 현재 초안은 loaded thread의
   provider/ModelsManager를 coherent snapshot으로 읽는 백엔드까지다. 실제 생성기로
   schema를 갱신하고, TUI가 표시 중인 thread를 기존 비동기 모델 조회에 전달하고,
   설정 이벤트에서 기존 cache/correlation 경로로 갱신하도록 연결해야 한다.
   형식이 같은 구버전의 unknown-field 무시를 capability 지원으로 보지 않는다.
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
humtr/codex의 기존 기능을 보존하는 구조 평탄화와 미완성 연결 작업을 인수해라.
저장소 https://github.com/humtr/codex 의 fix/native-route-transition-20261105를
받고, 먼저 PRO_HANDOFF.md 전체와 AGENTS.md, SPEC4.5–4.6, CURRENT.md,
providers/chatgpt/LINUX_APP.md를 읽어라. 네 작업 브랜치를 따로 만들고 실제
Codex/Native/work 세션 ID와 기준 commit을 기록해라.

위 문서의 내 연구는 참고 자료다. 원본 소스·실제 호출자·기록으로 독립 검토하고,
필요하면 책임 경계부터 다시 설계해라. 사용자 기능을 유지하면서 중복된 결정·상태·
준비·실행·복구 경로를 합치거나 없애라. 새 예외/래퍼/감시기를 기본 해법으로
만들지 말고 대체된 코드·설정·시험·문서를 함께 제거해라.

기기/앱/실모델 실증에는 Termux Bridge를 사용해야 한다. 기존 Linux 앱은 실행·
로그인되어 있고 모바일 화면·태스크바·Android 로그인 코드가 이미 있다.
원래 dirty 작업 폴더와 사용자 Chat/계정/작업을 보존하고 새 홈이나 새 런처로
초기화하지 마라. PRO_HANDOFF.md의 보관 브랜치는 원본 비교용이며 일괄 병합하지 마라.

문서의 우선 과제12개, NC-01..29와 누적 과제 전체를 빠짐없이 판별하되 관련
변경부터 구현해라. 새 격리 테스트 세션으로 실제 읽기/쓰기/계속 작업/전환/
컴팩션/중단/재연결을 필요한 경계에서 실증해라. 이미 통과한 동일 입력 검사는
반복하지 말고, 대역·실제 모델·Android·물리 화면·배포 결과를 구분해라.

먼저 독립 진단과 선택한 구조/대안/삭제 범위를 짧게 보고한 뒤 승인된 범위에서
개선을 이어가라. PRO_REVIEW.md에 연구·구현·실제 증거·미완료·다음 작업을
기록하고 브랜치에 커밋/푸시해라. 기기 원본 보고서는 인계 문서의 REPORT.md
목적지에도 보존하되 인증정보·capability·개인 원시 대화는 공개하지 마라.
```
