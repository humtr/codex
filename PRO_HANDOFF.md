# Pro 인계: 기존 기능을 보존하는 구조 평탄화

## 0. 현행 기준점과 첫 Pro 검토 단계 (2026-11-08 KST)

이 문서는 현재 작업의 인계와 기존 결정·실패 증거를 함께 보존한다. **실행
기준점은 아래 확인된 값과 `CURRENT.md` 상단이며, 아래 절의 예전 세션·릴리스
기록은 별도로 표시된 역사/대안 자료**다. 시작 시 Git HEAD, 프로세스 이미지,
운영 바인딩을 다시 읽고 원본 커밋과 실제 실행 바이너리를 혼동하지 않는다.

`SPEC.md`는 버전과 무관한 규범 사양이며, 그 상세 계약은
`docs/spec/USER_INTERFACES.md`, `docs/spec/BRIDGE.md`,
`docs/spec/DELIVERY.md`에 있다. 세대별 과거 패치·수용 기록은
`docs/history/GENERATION_ADMISSION.md`로 분리됐다. `AGENTS.md`는 모델과
무관한 작업 태도 규칙이다. **`goal.md`가 비어 있다면 현재 별도 장기 목표가
설정되지 않은 것**이며, 이 문서의 보존된 과제를 자동 장기 목표로 바꾸지 않는다.

- 문서 선작업 기준: `fix/attached-native-development-loop-20261106`의
  `92a33c4821e836dd465c296c6015310d7c76ffec` 이후 변경. 실제 현재
  커밋과 작업 트리 상태는 `git rev-parse HEAD`와 `git status`로 확인한다.
  원격 `rewrite/rust-core`가 이 문서 기준을 가리키더라도 사용자의 로컬
  `~/prj/codex` dirty 작업 트리를 자동 병합하거나 작업 공간으로 대체하지 않는다.
- 실제 활성 Core/Manager/TUI: **Codex 0.167.0 Full153/Base152**,
  제품 소스 `d3becae26b076a2e29697e24914f223493c7e084`, 별도 검증된
  Frontend `45533d6a7e5bf50dad3aa9545717f36a6d52d9a9`, Full151 롤백 보존.
  원본 업스트림 소스는 `release/upstream.json`의 `086ccc833bc9cf01024ad2dd6af7dbd421b6bf9a`.
  0.167 **로컬 소스 컴파일은 미실시**이고, 서명 공개/Android 실증은 검증된
  hosted runtime 산출물을 재사용해 통과했다. 기존 세션의 구 세대 app-server
  프로세스는 독자적인 살아 있는 이미지일 수 있으므로 종료·정리를 추정하지 않는다.
- 실제 Provider/MCP: 소스 `7471ed71a0efc97dbb6d97cb092f3d5ab01d10d8`의
  **unsigned 개발 후보**가 사용된 운영 바인딩과 별도 서명 Provider25를 구분한다.
  17개 커넥터 스키마 갱신은 특정 실제 바인딩의 증거로만 취급한다.
- 추가 감사의 원본: `~/.cache/codex/acceptance/upstream-167-pr-audit-20261108/`
  아래 `REPORT.md`, `FOLLOWUP.md`, `CLOSURE.md`, `AUTO_REVIEW_AUDIT.md`,
  `GAP_AUDIT.md`. 가장 최근 감사에서 `--approve-for-me`의 Termux 시작 실패가
  확인됐지만 수정·서명 배포되지 않았다. 실제 Native+합성 Guardian
  allow/deny는 2/2 통과했어도 **실모델 Guardian의 명시적 금지 범위 거부는 과거
  실패했으며 0.167에서 해결됐다는 증거가 없다.** 명시적 제한 프로필과
  자동승인 프리셋의 설정 우선순위도 수정 시 보안 회귀 시험 대상이다.
- Native NC-01~29는 **Partial 22 / Open 7 / 전체 완료 0**. 한정된 Provider
  병렬 실행·fixture 증거는 모든 도구·Git·외부 효과의 동시성 보호가 아니다.
  최신 실제 모델 후속 입력·steering, ChatGPT↔OpenAI 전환, compaction,
  app/CDP 전달 불확실성은 미완료 조건으로 유지한다.

**첫 Pro 단계는 독립적인 읽기 전용 재검증이다.** 실제 사용 가능한 도구
카탈로그, 기존 source/work 소유권과 위 원본을 확인하고, 보고서의 주장과
코드를 대조해 결함·잘못된 결론·수정 책임·우선순위를 먼저 보고한다.
현재 Codex TUI에서 타임아웃이 없었다는 관찰을 ChatGPT 진입점의 복구
증명으로 확대하지 않는다. 새로운 Native work나 모델 추론은 필요한
이유·소유권·중복 방지 방법을 확인하기 전 자동 생성하지 않는다.
기존 work의 후속 요청이면 지원되는 목록/join/attach를 먼저 확인하고,
새 독립 연구가 명시적으로 필요한 경우에만 **현재 선언된 실제 스키마**로
생성한다. 과거 `create/start/attach` 예시를 고정 API로 간주하지 않는다.
이 첫 검토로 서명 릴리스·운영 정책·계정·사용자 대화·사용 중인
runtime을 수정하지 않는다. 이후 구현은 6.2의 소유 순서에 연결한다.

## 1. 이번 인계의 범위

사용자의 최신 지시는 Codex가 기본 소스·증거를 정돈한 뒤 Pro가 능동적으로
설계·구현을 인수하는 것이다. 초기 준비 뒤 Codex가 attached 개발 루프의 P0를
구현·기기 실증했다. 실제 소스에 연결된 개발 후보, 현재 서비스/터널 바인딩과
검증 한계는 CURRENT.md가 소유한다. 필요한 동작을 기준으로 선별했으며,
보관 브랜치를 먼저 모두 합치는 방식은 사용하지 않았다.

**이 준비안과 기준 브랜치 자체도 Pro의 검토 대상이다.** Pro는 더 단순하고
일관된 방법을 찾으면 기준 소스·통합 순서·책임 배분·구현을 재선정하고,
준비안을 수정하거나 폐기할 권한과 책임이 있다. 기존 코드·문서·이전 제안에
있다는 이유로 유지하지 않는다. 선택 근거를 짧게 알린 뒤 승인된 범위에서
진행하며, 통상적인 설계 선택마다 재승인을 기다리지 않는다.

**선행 개발 루프는 실제 실행 경로에서 검증됐다.** Full Access의 실제 ChatGPT
커넥터 API→Bridge→Native 파일 작성·컴파일·같은 작업 계속 실행, 실제 GPT-6
Guardian 검토 후 명령 실행·후속 명령, 대역 검토자의 거부/불능 시 미실행을
구분해 보존했다. **실제 모델의 금지 범위 거부는5절의 부정 실증에서 실패했다.**
검토자 호출 성공을 보호 효과로 계산하지 않는다. 기본 정책은 바꾸지 않았으며,
실제 Target 소실 후 복구·모든 모델/진입점까지 검증됐다는 의미는 아니다. 충분한 근거 전
기본값 변경 제언은 보류한다. 전역 설정 변경·검토 실패 시 자동 never 전환·
별도 shell 실행기는 해법이 아니다.

이 개발 루프와 역방향 Codex TUI→ChatGPT의 아래 P0 CDP/입력 장애는 구분한다.
공유 검토자/AppHost/provider 경계는 실제 요청 상관관계로 확인한다. 역방향 TUI
복구를 attached 실행의 선행 조건으로 두지 않는다. 초기 개발 루프 수정은
Native 정책 전달과 앱의 불필요한 Composer 의존 제거였으며
별도 실행기를 만들지 않았다. 이후 Core의 Native 소유자 연결 선택도 수정했다.
개발 Provider 후보의 실제 앱 검토와 Bridge 왕복은 통과했다. 앞선0.166
Full151/Base150에서 서명 배포·기기 활성화와 같은 작업의 일반 TUI 표시를
확인했다. 이후 업그레이드의 현재 승인 소스·공개 세대·기기 활성화·커넥터 검증은
CURRENT.md 첫 두 절에서 확인한다. 물리 화면 관찰과 실제 모델 장애 복구는 별도 미완료다.
선작업 범위는6절 계획에 따라 TUI 연속 작업과 동일
Provider의 두 진입점 인계까지 확대한다. Pro는 검증된 범위를 재사용해 모델
전환·컴팩션과 나머지 평탄화를 인수하되,
준비한 구조도 독립적으로 재조정한다.

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
| **현재 선작업 소스** | `fix/attached-native-development-loop-20261106` / 최신 선작업·검증 상태는 `CURRENT.md` | 기존 `pro-baseline` 작업공간을 재사용한다. 승인 제품 소스, 별도로 검증된 frontend 소스, 문서 HEAD와 실제 운영 Provider는 서로 구분한다. 서명 공개·기기 활성화·커넥터 검증의 현재 결과는 CURRENT에서 확인한다. 별도 Native 전환 후보와 혼동하지 않는다. |
| **준비 기준 브랜치** | `prep/pro-baseline-20261106` | Pro 작업 브랜치의 출발 후보. 미완성 catalog 초안을 분리하고 이 문서·운영 사실을 맞췄다. Pro는 다른 출발점이 더 적절하면 재선정할 수 있다. |
| 이전 전체 인계 보존 | `fix/native-route-transition-20261105` / `eeb6f9d808cd2dc7319a3d94f07010d650f3d281` | Native 초안까지 포함했던 원본 인계. 강제 변경 없이 보존했다. |
| 마지막 기록 기준 | `c340237982102a100de81aa70aca60b0617477eb` | 현재 복구 전의 기록 기준. 이후 인계 변경을 함께 읽는다. |
| 미완성 Native 초안 | `draft/thread-model-catalog-20261106` / `8d095cd754faf9f4ee8707f176d9648431bc08da` | thread-scoped model/list 백엔드·계약 초안. 컴파일/스키마/UI/수용 검증 미완료. 준비 기준에서는 적용을 되돌렸으며 원본은 이 브랜치에 보존했다. |
| 원래 로컬 작업 보존 | `handoff/pro-local-baseline-20261106` / `b8c14252ee338a75ace4cf3bfdc463d9747108c5` | 원래 dirty0814be7b의 모든 추적 변경과 신규 소스를 보존했다. 주 개발 브랜치에 무조건 합치지 말고 필요한 차이를 검토한다. 원래 작업 폴더는 그대로다. |
| **채택하지 않은** 모바일 런처 시도 | `archive/mobile-desktop-attempt-20261106` / `8de07907b1108bdc7ee7646e1ea55faa1ae14dec` | 현재 미채택인 비교·복원 자료. Pro는 채택 여부를 재평가할 수 있다. 기존 런처를 복제한 부분과 독립적인 viewer URL 수정은 구분한다. |
| **실제 작동하는 기존 Linux 소스** | `archive/linux-app-operating-source-20261106` / `97118e775bc6e7dbcebe20fdcb7f763ae1ee7278` | 설치된 source 파일과 원본 대비 변경을 보존했다. `experiments/web-provider`와 `novnc-sidebar.patch`를 본다. 앱·계정·인증 자료는 포함하지 않는다. |
| 책임 경계 재검토 | `design/entry-architecture-20261105` / `604df76bb30f361543246a7face53004b1f3d55a` | 기존 경계도 재검토한다는 설계의 근거. 구현 완료 증거는 아니다. |

`rewrite/rust-core`는 구현/재사용 가능한 릴리스 로직의 소유자이고 `main`은
검증된 정확한 소스를 가리키는 호출자다. CURRENT의 서명 공개와 기기 활성화
결과를 각각 확인한다. 이 문서 자체를 다른 후보의 배포 승인으로 해석하지 않는다.
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
| 이 작업을 수행한 Codex 세션 | `01a1a5d8-9652-7220-a958-be6349e07419` — 실행 환경의 CODEX_THREAD_ID/CODEX_SESSION_ID에서 확인 |
| 실제 Full Access 개발 시험 | work `86264e68556ad46253e83972d74dd3607c0928e80388ba04`, thread `01a1af11-681f-7453-afa6-2bfb361cff34` |
| 일반 Auto Review·직접 Bridge 계속 작업 | work `ac4f1b1fbb10317d089859b8f2a7fc9b824a8f26484aae0e`, thread `01a1af15-4b6c-74b2-aee4-78c8b7639fa0` |
| 실제 GPT-6 Guardian 시험 | work `26e22c84347e258c4071309b4488b0c68d5b371792210bc0`, thread `01a1af1d-9644-7bd1-b066-f9404f55310d`, reviewer child `01a1af1d-9663-7653-a68a-f2861b141f64` |
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
- 현재 연결된 계정의 서비스와 MCP 명령은 clean `7471ed7`에서 빌드·inventory 검증한
  **개발 복구 후보**다. 최신 Native 전환 후보나 서명된 정식 배포가 아니다. 기존
  런처의 현재 데스크톱 descriptor로 정상 재연결했으며 정식 앱 수명 통합은 남았다.
  다른 계정 인스턴스를
  이 바인딩과 혼동하지 않고 Native/MCP 부팅을 optional 앱에 종속시키지 않는다.
- 현재 후보·바인딩·이전 서비스/터널 백업과 증거는
  `~/.cache/codex/acceptance/request-continuity-7471ed71a0ef/`에 있다. 이전763c24a와 그 전
  `connectivity-app-repair-20261105`는 이전 복구 기록/롤백 근거다. 설치된 패키지와
  실제 실행 이미지를 구분하며 비공개 제어값은 출력하지 않는다.
- 기존 `scripts/refresh_chatgpt_connector.mjs`로 앱 캐시 무효화, **17개 전체 입력 스키마 일치**,
  실제 MCP 조회를 완료했다. 이후 새 Native create/start/attach/execute/finish,
  파일 수정/빌드/계속 작업과 실제 GPT-6 Guardian 검토가 통과했다. 해당 work/thread
  ID와 범위는 CURRENT에 있다. 실제 TUI 정상화·모든 모델 성공으로 확대하지 않는다.
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
- 이후 읽기 전용 조사에서 세션01a1af43-26f0-71c0-ad89-5012b14c52f6의
  app_round_identity_unconfirmed와 세션01a1af61-c9a5-7253-b0f3-59517f65a1d2의
  empty_app_round를 확인했다. Native에 원래 도구 결과가 있는데도 앱 Lane에는
  미정산 호출이 남았다. 뒤 세션에는8개 호출/결과가 있어 전체 작업이 빈 것은
  아니다. 최초 취소/반환/완료 순서, 두 실패의 공통 원인과 fork의 직접 오류는
  아직 미확정이다. CURRENT의 사실과6절 계획을 기준으로 조사·수정한다.
- 현재 실행본은 소스가 확인된7471ed7 개발 후보이며 예전 복구 바이너리·설치된
  signed 패키지·Native 전환 초안과 다르다. 매 작업의 실제 바인딩을 확인한다.

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

여기에6.2의 steering·늦은 callback·도구 인계 사이 빈 관찰·진짜 빈 완료를
결합한다. 오류 이름마다 별도 복구기를 추가하지 않고 원본 수락/결과와 후속
입력이라는 같은 소비 경계를 검증한다.

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
정확한 connector/Provider/state로 `scripts/refresh_chatgpt_connector.mjs`의 전체 입력 스키마
새로고침을 완료하고 새 Native create/start/attach/execute/finish를 확인한다.
단일 CDP 관찰자를 경쟁시키지 않고 기존 앱·프로필·대화를 초기화하지 않는다.
소스/대역/기기/실제 모델/TUI/서명/운영 활성화는 각각 증거를 구분한다.

사용자가 장기 목표를 명시한 경우에만 `goal.md`를 작성하고, 그 외에는 비워둔다.
`CURRENT.md`에는 확인된 사실과 미완료만 유지한다.
`PRO_REVIEW.md`와 기존 기기 REPORT.md에 다음을 보고한다: 최초 확인 실패와
근본 원인, reconciliation 진입/해제 전이, A/B/C의 공통/독립 판정, 필요한
보호와 불필요 차단, 수정 책임·통합/삭제/호환, 최초 실패/수정/결함 주입 결과,
실제 TUI 재입력·최종 모델 응답·Native 연속성, 미완료 검증·배포·롤백 상태.

## 5. 보존된 검증: 다시 시작하지 말 것

**후속 부정 실증은 실패다.** 세션01a1af61-c9a5-7253-b0f3-59517f65a1d2와
Guardian 자식01a1af64-3973-7823-902e-df2e8644cd56에서 허용/금지 삭제 모두
allow를 받았다. 금지 파일에 대한 설명이 검토자 입력에 있고도 금지 명령의
Native 결과가 exit0다. 사용자는 실제 삭제와 이후 명령 지속도 확인했다.
현재 응답의 읽기 전용 대조는 자식·입력·두 allow·해당 명령 exit0를 확인했다.
호출 성공, 판정 집행, 범위 위반 거부 능력을 구분한다. 기존 허용 실증을
보호 효과 증거로 확대하지 말고 입력 구성·활성 지시·판정·upstream 승인 경계를
조사한다. 원인은 아직 미확정이며 기본 정책을 임의 전환하지 않는다.
사용자가 보고한 다른 work의 opaque/pending_result 효과와 재연결 차단도 보존해
실제 소유자/원래 결과로 확인한다. 앱 오류를 근거로 그 효과를 해제하지 않는다.
현재 등록 Bridge의 읽기 전용 재관찰에서 원본 work7cf78d…는 아직
reconciliation_required/Native notLoaded이고 현재 host 관찰도 없다. 다만 같은
Provider 저널 전체의 active/unconfirmed effect는0건이다. 예전 전역 자원 차단과
개별 work의 미정산을 구분하며, 이것으로 전체 동시성 검증 완료를 주장하지 않는다.

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

현재 선작업 검증은 CURRENT.md의 Request settlement implementation과
Same-Provider TUI owner selection 및 Active operating deployment를 함께 읽는다.
초기 일반 resume의 다른 backend 선택 실패는 Core/shared runtime-binding의
실제 Native 소유자 관찰로 수정했고, 양쪽 시작 방향·외부 홈·명시적 effort·
선택적 Manager 없는 계속 실행까지 실제 Android 대역 앱 경로에서 통과했다.
151/150 운영 활성화 뒤 새 등록 Bridge의 Rust 수정/빌드/재실행·한 번만 효과가
나는 재시도와 일반 TUI의 같은 작업 기록 표시도 확인했다. 원본 실패·raw --remote의
명시적 effort 누락·실제 모델/물리 화면/미확정 결과의 한계는 보존한다.
다른 작업의 process56351 예약은 이전 관찰에서 exit-1만으로 종료를 입증하지
못했으며, 현재 실제 소유자/결과 없이 임의 해제하지 않는다.

## 6. 우선 정리·연구·구현할 내용

**업스트림 정식 버전을 계속 흡수하는 것은 이 래퍼의 상시 의무다.**0.167.0은 이번
대상일 뿐이며 이후 버전에도 같은 원칙을 적용한다. SPEC의 Continuous upstream
upgrades 계약을 따른다. Pro가 결정할 것은 업그레이드 여부가
아니라 새 업스트림 기능을 존중하면서 기존 수정의 마찰을 줄이는 구현 방법이다.
기존 발견·소스 적응·검증·서명·승격·기기 업데이트 자동화를 개선한다. 업스트림이
대체한 자체 코드는 정리하고 필요한 Termux 적응만 유지한다. 새 버전을 승인할
때까지 운영/롤백 세대를 보존하며, 이를 개발 버전 동결 방침으로 해석하지 않는다.
공통 소스 적응·산출물 재사용은 CURRENT의 최신 실증을 재사용한다. 실제 모델의
TUI 연속 작업·steering 실패 두 건은 별도 미완료 입력이며 버전업만으로 해결됐다고
가정하지 않는다. 준비한 업그레이드 구조와 검증 경계도 Pro가 재평가한다.

### 6.1 재조직 원칙과 선작업 경계

목표는 양쪽 진입점에서 같은 Native 작업의 입력·실행·결과를 유지하고,
취소/재연결/부분 응답이 있어도 확정 가능한 상태로 계속 작업하는 것이다.
판단 단위는 원본 작업/입력/호출이며, 연결과 화면은 이를 관찰·조작하는 수단이다.
물리 스트림 종료, 도구 인계용 Responses 라운드 종료, 앱 추론 완료, Native 작업
완료를 구분한다. 빈 출력과 관찰 유실은 그 자체로 작업 결과가 아니다.
실제 거부·잘못된 신원·미확정 효과의 보호와 기존 시간/비용 한계는 유지한다.

먼저 생산자·판단 주체·소비자·수명을 기존 호출자와 대조해 SPEC4.5–4.6에 맞춘다.
Native는 입력 수락/turn/call/승인/효과/기록, Provider는 외부 추론 요청의 전달·
정산, App/CDP 어댑터는 연결·실제 관찰을 소유한다. 한 사실에 한 소유자를 두며
이것을 하나의 거대 관리자에 모으라는 뜻은 아니다. MCP/Function은 원래 호출과
반환의 차이를 보존하고 같은 수명 결정을 따르며 TUI는 Native 수락과 상태를
표현한다. 현재 배치가 이 경계와 다르면 책임을 이동한다. 새 상위 복구 관리자를
추가하거나 파일 수·줄 수를 목표로 삼지 않는다.

각 묶음은 사용할 기존 경로, 옮길 책임, 지울 중복과 실제 호환 소비자를 먼저
정한다. 새 경로를 검증한 변경에서 대체 함수·분기·설정·호출자·시험·문서를 함께
제거한다. 네트워크/프로세스/권한 경계의 재검증과 미확정 효과 보호는 남긴다.
Native phase의 독립 복제, 여러 층의 완료 추정, 전달만 하는 wrapper와 원인이
확정되지 않은 fallback은 그 목적/소비자를 확인해 통합 또는 삭제한다.

**확대된 선작업은 아래1·2다.** 기존 ChatGPT→Bridge 실행 증거를 재사용하고,
Codex TUI 연속 작업과 같은 Provider에서 양쪽 진입점의 동일 작업 인계까지
완결한다. 원래 두 실패 세션을 지원된 복구 경로로 계속할 수 있는지도 포함한다.
OpenAI↔ChatGPT 모델 전환과 컴팩션은3에서 별도 인터페이스 검증을 진행한다.
Pro는1·2의 정상 개발 경로를 기반으로3~6을 인수하며 필요하면 앞선 설계도
재조정한다. 지금 별도 Pro 추론을 시작하는 지시는 아니다.
1·2에서 지원 API의 공백이 드러나면 임시 실행 경로로 선작업을 늘리지 않는다.
확인한 공백과 실증 결과를3의 Native 설계 검토로 연결하고 인계 시 미완료 범위를
명시한다. 원래 기능 요구는 남긴다.

### 6.2 실행할 여섯 묶음

| 순서 | 범위와 주요 책임 | 사용자가 확인할 완료 조건 | 원래 과제 연결 |
| --- | --- | --- | --- |
| 1. 요청 수명·정산 통합 | AppHost/Operator/MCP의 중복 완료·진행 판단을 한 소유 책임으로 모은다. 원본 요청과 물리/가상 라운드 관계를 명확히 하고 steering·empty·timeout·응답 유실을 같은 계약으로 처리한다. | 원래 실패 세션의 Native 결과를 재실행 없이 정산하고, 안전이 확인되면 같은 작업에서 후속 입력을 수락한다. 차단이 필요하면 미확정 대상과 해제 조건이 구체적이다. | 2·4·9, NC-08·26~29 |
| 2. 두 진입점의 동일 작업 | Native의 입력 수락·현재 turn·writer·checkpoint를 사용한다. TUI steering/취소/초안 소비, 명시적 fork, TUI↔ChatGPT의 인계와 원래 결과 관찰을 연결한다. | 양쪽에서 시작한 작업을 반대편으로 넘겼다가 같은 thread/history로 계속한다. 일반 입력·추론 중 추가 입력·도구 반환 중 입력·중단·재접속에서 입력 손실/중복 실행이 없다. | 4·7·10, 관련 NC-01·03·04·08·22·26~29 |
| 3. 모델 경로·문맥 통합 | 기존 Native route/catalog 후보를 지원 인터페이스와 비교한다. Native가 선택을 검증·저장하고 컴팩션을 소유하며 Provider는 원격 문맥 전달·Chat 갱신을 맡는다. | OpenAI↔ChatGPT warm/cold 전환, bootstrap mismatch 해소, 실제 모델 목록/effort, 자동·수동 컴팩션과 복귀가 같은 작업에서 이어진다. 자동 fork 누적이 없다. | 5~8, 관련 NC-08·22·26·28·29 |
| 4. 앱·설정·설치 수명 | 기존 모바일 런처/로그인을 재사용하고 CDP 연결 소유를 통합한다. 계정·모델 권한·실행 홈·터널·설치 세대·활성 이미지의 선택/갱신을 기존 설정·supervisor·generation 경로에서 연결한다. | 정상 앱을 초기화하지 않고 재시작/갱신/롤백한다. source→artifact→서비스/MCP→실제 client가 일치하며 noVNC 오른쪽 설정도 유지보수 원본에서 재현된다. | 1·3·11·12, NC-20·22·23·28 |
| 5. 실제 자원과 동시성 | NC-01~29를 아래 묶음별로 완결한다. work 상태와 실제 파일/프로세스/Git/빌드/원격 효과 예약을 구분하고 MCP/Function이 Native의 같은 실행 권위를 사용하게 한다. | 무관한 작업은 진행하고 실제 충돌만 원래 소유자/결과와 함께 정산한다. 일반화된 workspace 잠금이나 미확정 결과 삭제로 통과하지 않는다. | 4·9·12, NC-01~29 전체 |
| 6. 남은 제품 기능 | 독립 Chat의 최초 Native 참여, 호스트 입력/출력 이벤트, 선택 설치/갱신, UI/이미지/clipboard, 계정/할당량, 예산·독립 worker 등 누적 기능을 관련 선행 경계 뒤에서 연결한다. | 각 실제 생산자→소비자와 물리 UI/권한/원래 결과가 확인된다. 순차 호출을 독립 worker로 표기하거나 실제 호스트 지원 없이 자동 참여를 보장하지 않는다. | 7·10~12 및8절 누적 과제 전체 |

위 순서는 기능 묶음의 의존성이다. 문서 정리만 끝낸 뒤 멈추거나 모든 NC 검증을
단순 입력 복구의 선행 조건으로 삼지 않는다. 1·2에 필요한 NC 경계를 먼저 확인하고
나머지는5에서 완결한다. 정식 배포에는 해당 릴리스 계약이 요구하는 전체 수용
검증을 그대로 적용한다. 독립적이고 승인된 작업은 관련 경계가 준비되면 진행한다.

1의 첫 실증은 다음 순서를 포함한다: 공개 답변 전 Steer, 도구 결과 확정 직후
Steer, 이전 stream의 늦은 callback, 도구 인계 뒤 다음 stream 시작 전 빈 snapshot,
응답/ACK 유실, 확정 실패/취소, 실제로 완료됐지만 사용 가능한 출력이 없는 경우.
각 경우에 원본 입력 수락·실제 dispatch·Native 효과/원래 결과·후속 입력 가능
여부를 확인한다. 확정된 결과가 없어지는지, 한 번 실행한 도구가 다시 실행되는지,
이전 callback이 후속 요청을 오염시키는지를 검출하는 회귀를 만든다.
수명을 오류 코드별 새 상태/재시도로 확장하지 않고 기존 소유자의 관찰·정산을
통합한다. 완료 판단에 필요한 최초 오류/전달 사실은 최소한으로 보존하되 별도
대화 저장소·매 단계 영수증·추적만을 위한 hash를 추가하지 않는다.

2에서는 다음 입력이 이미 Native에 수락됐는지 확인한 뒤 draft를 소비/복구한다.
새 세션만 성공시키는 것으로 끝내지 않는다. 명시적 fork는 Native가 확인한
commit/checkpoint에서 수행하고 부모의 불확실한 효과나 실행 capability를 복제하지
않는다. 실패 세션의 실제 fork 오류는 아직 확인되지 않았으므로 원인부터 판별한다.

3은 현재 서명 Native의 capability와 미배포 초안을 구분한다. 지원되는 upstream
구성으로 요구를 충족할 수 있는지 먼저 입증한다. 부족하면 정확한 인터페이스
공백, Termux 필요성, 최소 호환 변경과 대안을 비교하고 SPEC을 갱신한다.
Provider 이름 변경이나 TUI 재시작을 호환성 증거로 삼지 않는다. 대체 Native
경로가 실제로 수용된 때 겹치는 reentry/checkpoint/summary 조정을 함께 제거한다.

4는 source 정합성 확인을 매 묶음에 적용하되 정식 수명 통합/서명 승격은 해당
통합 후보에서 수행한다. 현재7471ed7 개발 실행본·원본 rollback과 원래 dirty 소스를
보존하고, prep/draft/archive는 비교 입력으로 다룬다. 필요한 변경만 의미 단위로
통합한다. 폐기 전 실제 소비자와 복원 필요를 확인한다. live 교체 후 기존
scripts/refresh_chatgpt_connector.mjs로 전체 schema와 새 Native 왕복을 확인한다.

5의 NC 배치는 읽기/snapshot(01~04), 파일/경로 효과(05~12), Git 자원(13~16),
빌드·cache·임시 출력/정리(17~20), repo/계정/외부 권한(21~25), 진입점·복구·
복합 효과(26~29)로 나눈다. 번호는 시험 범위이며 새 관리자/실행기를 만들라는
요구가 아니다. 실제 writer/외부 변경이 관찰되지 않는 범위는 한계로 남긴다.

각 묶음은 계약·수정·기존 경로 삭제·영향 시험·실제 소비자 확인을 같이 끝낸다.
빠른 영향 시험 뒤 기능 통합점에서 필요한 빌드와 전체 수용 검증을 수행한다.
변하지 않은 입력의 green 증거는 재사용하고 substantive 회귀는 격리 결함 주입
실패/복원을 확인한다. source/대역/Native/앱/실제 TUI/서명/활성화를 구분한다.
Auto Review와 Full Access를 유지하며 기본값 결정은 실제 비교 증거와 사용자
검토 뒤에 둔다. 새 Pro 호출은 하지 않는다. 매 배치 완료 뒤 기존 cleaner로
확인된 생성물만 정리하고 원본 결과/Native 기록/current·rollback을 보존한다.

소요는 예비 추정이다.1의 원인 재현·정산 수정·기기 복구는 반나절~2일,
2의 기존 Native API를 이용한 양방향 인계는 추가 반나절~1일을 기준으로 본다.
최초 재현 뒤 수정 범위와 실제 빌드/기기 시험을 근거로 갱신한다.3은 Native
인터페이스/컴팩션 검증 결과 뒤 별도 산정하며 완료 시점을 미리 확정하지 않는다.

### 6.3 보존할 원래12개 요구

아래 번호는 범위 추적용이다. 실제 실행 순서는6.2를 따른다. 각 항목의 기능과
검증 조건은 유지하며 Pro는 실제 증거로 묶음/책임을 재조정할 수 있다.

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

아래는 현재 개발 루프 실증 뒤의 인수용 프롬프트다. **첫 단계는 위 0절의
읽기 전용 독립 감사**다. 검토에 없는 새로운 모델 추론·Native 작업·운영 변경을
묵시적으로 승인하지 않는다. 실제 인수 시 CURRENT와 원격 HEAD를 확인한다.

```text
ChatGPT의 현재 Pro 추론과 @TermuxBridge로 humtr/codex의 책임 경계 평탄화와
미완성 연결 작업을 인수해라. 기존 구현·준비안·경계도 독립적으로 평가하고,
더 단순하고 올바른 구조가 있으면 재선정·재조정한 뒤 구현과 실제 실증까지
수행해라. 단, **첫 단계에서는 읽기 전용으로 소스·증거·실제 바인딩을 확인해
결함/수정 소유권/우선순위부터 보고하라.** 사용자에게 후속 구현을 위임받은 뒤
승인된 범위에서 계속하고 보고서 작성만으로 전체 개발이 끝났다고 하지 마라.
6절의 재조직 계획을 따르되 더 간결한 책임 배분이 입증되면 갱신해라. Codex의
확대 선작업은 공통 요청 수명·TUI 연속 작업·동일 Provider의 진입점 인계다.
현재 완료됐다고 가정하지 말고 최신 CURRENT를 확인한다.

이 프로젝트는 업스트림 Codex를 계속 따라가는 Termux 래퍼다. SPEC의 Continuous
upstream upgrades는 상시 계약이며 이번0.167.0뿐 아니라 이후 정식 버전도 계속
반영해야 한다. 업그레이드 채택 여부를 다시 결정하지 마라. 새 기능을
흡수하고 업스트림이 대체한 자체 구현을 제거하면서 필요한 Termux 적응을 유지해라.
기존 upstream_discovery.py/accepted-release-inputs.yml과 release-builder의 경로를
개선해 다음 버전도 같은 절차로 반영되게 해라. 별도 릴리스 관리자나 버전별
우회표를 만들지 마라. Codex의 공통 소스 적응·Git 병합·현재 업스트림 생성기와
완료 산출물 재사용 구현은 독립 검토 대상이다. 과거 cli_main 오류를 현재 결함으로
다시 작업하지 말고 CURRENT에서 최신 승인 소스·실행 증거·미완료 경계를 확인해라.
CURRENT가 확인한 업그레이드·배포·기기·커넥터 실증은 그대로 재사용해라. 새 수정이
그 입력을 바꾸거나 미완료 경계가 남을 때 해당 검증·서명·공개 readback·승격·기기
활성화·커넥터 갱신을 수행해라. 배포 입력과 결과가 다시 섞이거나 불필요하게 재빌드되는 소비자,
새 범위 기반 시험 검증도 원본 PASS·선택 범위·실패 보호가 일치하는지 검토해라.
옛 숫자 고정 검사를 되살리거나 새 시험 추가만으로 정상 산출물을 거부하지 마라.

업그레이드 전 보고된 실제 TUI 실패도 포함해라. thread01a1b508-c8ce-7fb2-9831-028cc8c892f4는 첫
Rust 루프 뒤 후속 턴 모델이 chatgpt-desktop/gpt-6에서 gpt-6.1-sol로 달라진 기록과
unsupported_desktop_route가 있다. 전환을 유발한 입력은 미확인이다. 사용자에게
원인을 돌리거나 세션 최초 provider 값으로 현재 route를 추정하지 마라.
thread01a1b511-dfd5-7db3-9977-57582422267e는 steering 수락 뒤
native_context_prefix_changed로 종료됐고 실제 도구 결과가 없다. 두 경우를
하나의 원인으로 단정하지 말고 Native 입력/설정·Provider 문맥·TUI 수락 계약을
추적해라. 문맥 검사 삭제나 모델명 강제 고정 대신 같은 작업의 실제 재입력과
steering을 통과시켜라. 아래 대역 앱 통과를 실제 모델 통과로 확대하지 마라.

현재 선작업은 실제 Android에서 양쪽 시작 방향의 같은 작업 계속 실행, 외부
실행 홈에서 원래 소유 홈 복귀, OpenAI 인증·선택적 Manager 없이 재개, 명시적
effort 유지, 추론 중 steering/queue 편집/취소, 빈 최종 응답·준비 timeout 뒤
후속 입력, 명시적 fork의 부모 기록 보존까지 통과했다. 이 시험은 통제한 앱
응답이며 실제 모델 요청은0회다. 앞서 검증한 실제 모델·등록 Bridge 증거와
구분해라. 앞선0.166 Full151/Base150의 실제 등록 Bridge 새 작업에서 Rust 수정 /
컴파일/재실행·중복 없는 정확 재시도와 일반 TUI의 같은 기록 표시가 통과했다.
이후 업그레이드의 전체 Native gate·공개 배포·기기 활성화·클라이언트 증거의
정확한 입력과 완료 범위는 CURRENT 첫 두 절을 확인해라. 문서 HEAD와 승인 제품
소스, 동일 생산 입력으로 재사용한 frontend 소스를 혼동하지 마라. 통과한 같은
대형 CI를 다시 시작하거나 이미 끝난 기기 활성화를 반복하지 마라.

저장소: https://github.com/humtr/codex
작동하는 코드의 출발 후보: fix/attached-native-development-loop-20261106 최신 원격 HEAD
실제 개발 Provider/MCP 후보의 소스: 7471ed71a0efc97dbb6d97cb092f3d5ab01d10d8
별도 Native 전환 후보: prep/pro-baseline-20261106 (현재 서명 Native에는 미적용)
먼저 이 문서 0절과 PRO_HANDOFF.md 전체, AGENTS.md, SPEC.md4.5–4.6,
CURRENT.md와 SPEC의 관련 상세 계약, 비어 있을 수 있는 goal.md,
운영 근거인 `~/.cache/codex/acceptance/upstream-167-pr-audit-20261108/`
아래 `GAP_AUDIT.md`·`AUTO_REVIEW_AUDIT.md`, 필요할 때
providers/chatgpt/LINUX_APP.md를 읽어라. 복구/컴팩션/인수 뒤에는 실제 AGENTS.md를
다시 읽고 현재 diff·새 경로·미해결 사항을 확인하되 통과한 검사를 재시작하지 마라.

실제 worktree·branch·HEAD·diff·작업/소유권과 서비스/MCP 명령/커넥터/설정의
현재 바인딩부터 확인해라. 기존 pro-baseline 작업공간은 현재 P0 브랜치에 있다.
implementation/fix/native-route-transition-20261105와 준비 브랜치는 대안 자료다.
같은 코드나 활성 환경이라고 가정하지 마라. 소유권과 사용자 변경을 보존하고
사용하지 않는 기존 깨끗한 트리에서 본인 브랜치를 만들어라. 불필요한 clone /
중복 checkout, reset/clean, 보관 브랜치 일괄 병합·강제 변경은 금지한다.
현재 P0 기준보다 다른 출발점이 더 낫다면 증거와 전환 방법을 밝히고 재선정해라.
Native 작업의 workspace에는 선택한 실제 기기 worktree의 절대 경로를 넣어라.
기본 ~/prj/codex의 원래 dirty 소스는 보존 중이며 새 기준과 자동 통합되지 않았다.

이전 Codex 세션: 01a1a5d8-9652-7220-a958-be6349e07419
이전 Pro 관련 thread: 01a1a98a-b913-7542-86a5-e9a2fdac3e68
이전 Pro work_id: 81f001ccecc1753339b40098bffc5a3e60c57dd63b14a0ea
이 ID와 CURRENT의 시험 ID는 출처 참고다. 네 연구 세션으로 위장하거나 과거
할당 claim/join을 선행 조건으로 삼지 마라. 본인의 실제 세션/Native thread/work
ID, 선택한 기준 commit과 브랜치를 기록해라. **먼저 Bridge의 실제 최신
native_catalog 및 등록 세션 목록/상태를 읽고, 이 단계의 읽기 전용 작업에
새 Native 할당이 필요한지 판별한다.** 이미 현재 응답에 붙은 작업이나
사용자가 지정한 동일 work가 있으면 지원되는 join/attach/도구 경로를 따른다.
새 작업이 필요한 경우에만 그때 확인한 생성·시작·연결 스키마와 실제
사용 가능한 입력을 사용한다. 예전 create/start/attach와 inference=attached
계약은 역사적 증거이지 새로운 호출 인자 선언이 아니다. 불확실한 요청은
원래 call_id/인자로 회수하고 중복 작업을 만들지 마라. 현재 Pro를 유지하고
별도 Pro 추론을 제출하지 마라.

개발 루프의 검증 범위를 재사용해라. Native 정책은 create 때만 선택 가능하다:
Auto Review는 approval_policy=on-request, approvals_reviewer=auto_review;
Full Access는 approval_policy=never다. 생략은 상속이며 기존 작업·전역 기본값을
바꾸지 않는다. require_escalated는 세션 정책 전환 기능이 아니다. Native가
권한·실행·결과·history를 소유하고 두 정책은 같은 정식 도구 경로를 쓴다.
실제 Full Access 파일 작성/컴파일/수정/재컴파일/같은 작업 계속 실행, 실제 GPT-6
Guardian 검토 후 실행과 후속 명령, 대역 검토자 거부/불능 시 미실행이 검증됐다.
일반 명령은 검토 없이 허용될 수 있으므로 명령 성공을 리뷰어 호출로 세지 마라.
실제 모델의 금지 범위 거부는5절의 부정 실증에서 실패했다. 그 원인과 실제
Target 소실 후 복구·모든 모델/진입점은 아직 미확정/미검증이다.
기본 정책 변경 제언은 충분한 증거 전 보류됐으며 사용자가 결정하기 전 변경하지 마라.
검토 실패를 승인이나 자동 never 전환으로 처리하지 마라.

현재 Provider/MCP 바인딩은 소스·inventory가 확인된7471ed7 unsigned 개발 복구
후보이고 서명 Provider 패키지는 그대로다. Core/Native/Manager/TUI의 현재 공개
stable·기기 활성화·보존된 롤백은 CURRENT에서 확인한다. Provider와 Core의 서로
다른 출처를 혼동하지 마라. CURRENT의 정확한 소스, 서비스/터널 백업과 기존 바이너리를
보존하고 운영 서비스를 임의 새 빌드로 바꾸지 마라. 기존 런처의 현재 데스크톱에
Provider를 재연결하고 d3_refresh로17개 전체 스키마를 검증했다. 데스크톱 임시
descriptor의 서비스 결합과 Android 브라우저 로그인 전달 실패는 여전히 남았다.
사용자의 로그인 완료를 그 결함 수정으로 계산하지 마라. 같은 커넥터의
옛 터널 세션이 옛 스키마를 노출했던 문제는 기존 터널 재연결과 d3_refresh로
검증했다. 현재 계정/터널을 재발견하고 다른 인스턴스로 바꾸지 마라.

다음 P0는 남은 실제 앱·전달 불확실성·모델 전환 경계와 원래 실패 작업의
미완료 복구다. steering/empty_app_round/cdp_operation_timeout은 요청 수명·
정산 계약으로 검증하되 같은 최초 원인이라고 단정하지 마라. CURRENT의 통과한
같은 작업 재입력·steering·fork 증거를 재사용하고, 미확정 전달·효과를 보존한
복구와 실제 앱 대상 소실을 추가로 검증해라. 물리 stream과 도구 인계 라운드의
done을 Native 작업 완료로 혼동하지 말고 원래 요청/호출의 authoritative 결과로
판단해라. Guardian의 실제 금지 범위 승인 실패와 해당 원본 작업의 재연결은
여전히 미완료다. 검토자 호출 성공을 보호 효과로 계산하지 마라.
보고된 app_lane_open_uncertain/opening과 timeout/new 기록, 빈 pending/round /
user_messages/dispatches, identity만 있고 project가 없는 상태를 현재 소스와
운영 증거에 대조해라. 당시 port45341을 상수로 만들지 마라. 15초 CDP 제한,
awaitPromise=true, history.mode=project와 project_read는 단서일 뿐 최초 원인으로
확정되지 않았다. Guardian의 app-target 준비 실패는 별도로 수정/검증됐지만
모든 timeout이나 기존 reconciliation 차단이 해결됐다는 의미는 아니다.

실제 CDP 큐/응답, Target/Session/React Scope, 비동기 앱 작업과 연결 수명,
Provider 요청/라운드 전달, Native 수락·효과·writer, TUI 초안 소비까지 추적해라.
P0 문서의 여덟 전달 단계를 각각 무엇이 확정됐고 어떤 원본을 관찰해야 하는지로
정의해라. 연결 종료나 로컬 카운터0은 취소/비전달 증거가 아니다. 원본 Lane /
round/call을 재관찰·정산하고 안전이 확인되면 같은 작업의 차단을 정상 종료해라.
미확정 모델 입력·도구를 자동 재전송하지 말고 thread/work/history/model/route /
승인·초안을 보존해라. timeout 연장·무제한 retry·오류 무시·강제 초기화·자동
계정/모델 전환은 해법이 아니다. 시간 정책 변경은 정상 실행의 실측이 먼저다.

간결한 책임과 데이터 흐름을 먼저 제시하고 SPEC/문서를 구현 전에 맞춰라.
Native는 정식 실행·승인·writer·기록·컴팩션의 단일 권위다. Provider는 앱/모델
연결과 외부 요청 전달·재관찰을 소유하고 TUI는 상태와 사용자 의도를 표현한다.
Core는 Termux 적응을 맡는다. 경계 자체가 잘못됐다면 옮기되 두 번째 실행기 /
기록/복구 상태머신은 만들지 마라. 실제 소비자를 확인해 중복 결정·준비·상태·
경로를 통합/삭제하고 대체 코드·설정·호출자·시험·문서를 함께 정리해라.

동일 Native 세션의 OpenAI↔ChatGPT 양방향 전환과 bootstrap modelProviderMismatch도
필수다. 공유 Native 로직과 얇은 추론 연결부, 지원되는 대안과 필요한 최소
Native 호환 수정의 정확성/비용을 비교해라. Provider 이름만 openai로 맞추는 것은
입증이 아니다. 현재 Native 전환 후보를 옛 서명 런타임에 무리하게 우회 적용하지
말고 부족한 인터페이스와 Termux 필요성을 증명해라. 최초 진입점에 관계없이
문맥/기록이 이어져야 하며 자동 fork 누적을 막아라. Native 컴팩션 결과의 소유와
Chat 갱신/주입을 구분하고 독립적인 Provider 요약기를 관성으로 추가하지 마라.

새 격리 시험 세션에서 P0 여섯 사례를 검증해라: 모델 전송 전 timeout, Lane Open
응답 유실, 모델 제출 ACK 유실, 같은 TUI 작업 재입력, Composer 부재와 Target 소실,
TUI/Native/MCP 상태 일관성. 읽기·쓰기·계속 작업·양방향 전환·자동/수동 컴팩션·
중단·재연결도 실제 결과와 세션/work ID를 남겨라. 실패 전후와 실질적인 결함
주입 실패/복원을 검증하고 문자열 비교·성공 mock으로 완료하지 마라. 입력이
같은 기존 대형 CI는 반복하지 마라. source/대역/Android/앱/실제 TUI를 구분해라.

기기/앱/모델 실증에는 @TermuxBridge를 쓰고 없으면 가능한 소스 작업을 계속하되
기기 성공을 주장하지 마라. 실제 모델은 승인된 범위와 유한 비용/할당량 안에서만
사용해라. 기존 모바일 런처·저장 프로필·태스크바·Android 로그인·noVNC를
재발명/초기화하지 마라. CDP 관찰자를 경쟁시키지 마라. 실제 runtime 교체는
기존 서명/배포/롤백 계약을 따르고, 현재 소유 CDP/정확한 connector/Provider/state로
기존 scripts/refresh_chatgpt_connector.mjs의 전체 스키마와 새 Native 왕복을 확인해라.

우선 과제12개·NC-01..29·누적 backlog를 빠짐없이 판별하고 관련 작업부터 순서대로
수행해라. 무관한 정리를 한 패치에 몰아넣지 마라. 사용자가 별도 활성 장기
목표를 지정하지 않았다면 goal.md는 비워두고, CURRENT.md는
확정 사실과 남은 작업으로 유지해라. PRO_REVIEW.md에 진단/대안/책임 변경/삭제·
호환 경로/실제 증거/미완료/다음 작업을 기록하고 논리적 단위로 커밋/푸시해라.
기기 보고서: ~/.cache/codex/acceptance/bridge-design-pro-review/REPORT.md
최종 보고에는 timeout 최초 실패/근본 원인, reconciliation 전이와 보호/불필요 차단,
A/B/C의 관계, 수정 책임·회귀/결함 주입, 실제 TUI 재입력·모델 완료·Native 연속성,
미완료·서명·배포·스키마 갱신·롤백 상태를 구분해라. 인증정보·capability·개인 원시
대화는 공개하지 마라. 최종 기준은 같은 작업을 중복 효과 없이 계속할 수 있는 것이다.
```
