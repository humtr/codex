# ChatGPT 단독 로컬 작업과 Codex 작업의 경계 검토

2026-10-31. 상태: 사용자 요청에 따른 조사·권고안이다. 제품 계약 변경이나 구현 승인,
새 실행 경로의 배포를 뜻하지 않는다. 기존 SPEC은 채택·수정 전까지 현행 계약이다.

## 1. 판정

핵심 문제는 중복 추론이나 알림 개수가 아니다. 현재 로컬 도구 사용 경로가 Native
Codex work/thread/turn을 전제로 해서, ChatGPT에서 독립적으로 처리하려던 작업도
Codex 세션 기록으로 남는 것이다. 알림은 이 결합에서 파생된 현상이다.

`direct/provider`는 현재 응답과 Provider가 시작하는 응답 중 추론 주체를 선택한다.
둘 다 Native work를 사용하는 현행 모델이다. ChatGPT만의 로컬 작업과 Codex에
기록·연결하는 작업의 구분으로는 부족하다. MCP/local_function도 도구 요청·결과의
운반 방식이므로 이 구분을 대신하지 않는다.

권고는 실행 의미를 먼저 분리하는 것이다.

1. **로컬 도구 사용:** ChatGPT가 자기 대화에서 판단하고 파일·명령 도구를 사용한다.
   Native thread/turn을 만들지 않고, 해당 사용자 입력과 답변을 Codex 대화에 쓰지 않는다.
2. **Codex 작업:** 사용자가 Codex 작업을 만들거나 기존 작업과 연결했을 때 Native
   thread/turn, 이력, 승인, 중단·복구 및 CLI 표시를 사용한다. 여기에서 기존
   `direct/provider` 선택이 유효하다.
3. **위임·오케스트레이션:** 실제 하위 작업들의 관계다. 모든 파일 열람을 Codex
   하위 작업으로 바꾸는 근거가 아니다. 각 노드의 실행 형태와 결과 반환을 명시한다.

업스트림0.166의 기존 `fs/*`, `command/exec`를 재사용하는 1번의 기반은 실제 Android에서
입증됐다. 새 명령 실행기, 별도 모델 또는 또 하나의 대화 저장소를 만들 필요가 없다.
현재 공개 MCP에는 이 로컬 도구 제품 경로가 아직 없다. 기반 API의 실행 증거와
완성된 ChatGPT 앱/MCP 제품의 검증은 구분한다.

## 2. 제공된 사건의 원본 대조

사용자가 식별한 Native 세션 `01a166e2-3f02-7831-867a-bbf84a874e9e`를 읽기 전용으로
확인했다. 원본 rollout, Native 상태 DB, Provider work 및 보존된 이전 controller
기록을 대조했다. 대화 본문·인증·controller 값을 이 보고서에 복사하지 않았다.

- Native 원본은0.163.0, `originator=termux-chatgpt-operator`,
  `model_provider=chatgpt_desktop`, `source=vscode`를 기록한다.
- 최초 controller 보존본은 `origin=chatgpt`, `reasoning=direct`, `return_path=mcp`다.
  assignment의 parent는 없고 독립적인 저장소 열람 역할이다.
- 첫 작업은 KST2026-10-23 12:51:13에 시작해12:57:07에 완료됐다.
  10월26일의 별도 후속 turn은 시작 뒤 중단됐다. 현재 work의 stopped 상태를 첫 작업의
 실패로 해석하면 안 된다.
- 이 세션은 실제 Native 세션 파일과 DB에 남아 있다. 단순한 CLI 화면 복제만이 아니다.
  현재 확인된 development binding은 없다. 그것을 과거 모든 시점의 부재로 소급하지 않는다.
- 현재 TUI의 기본 resume 목록은 Cli와VsCode source를 포함한다. Native의
  `source=vscode`는 이 경우 실제 VS Code 사용의 증거가 아니다. 설치본으로 만든
  이번 app-server 시험도 같은 source를 남겼다.
- 당시 Android 알림의 실제 전달 로그와 대응하는 ChatGPT conversation ID/원문 전체는
  이번 사건 자료에서 확보하지 못했다. 당시 의도는 사용자의 설명을 기준으로 한다.
  아래 시험은 현행 구조의 재현이며 과거 기기의 물리적 알림 전달 증명은 아니다.

사용자가 재개할 때 본 `modelProviderMismatch`는 별개의 기동 설정 문제다. 저장된
provider는 chatgpt_desktop인데 현재 backend가 openai여서 upstream이 거절했다.
기록 손상의 증거가 아니며, 이 세션에 작업을 재실행하거나 provider/계정을 변경하지 않았다.
앞서 검증한 invocation/profile 전달 수정은 Codex 작업을 올바르게 재개하는 데 필요하지만,
ChatGPT 단독 도구 사용의 세션 기록 문제를 해결하지는 않는다.

## 3. 코드에서 확인한 결합

| 책임 | 확인 위치 | 의미 |
| --- | --- | --- |
| 추론 선택 | `providers/chatgpt/src/operator.rs:103`, `Selection`, `Reasoning` | direct/provider와 반환 경로를 보유하며, 세션 생성 여부를 나타내지 않는다. |
| ChatGPT 작업 생성 | `operator.rs:3350`, create/resume/fork 분기 | `thread/start` 등 Native API를 호출하고 `register_in`에 기록한다. |
| 입력 시작 | `operator.rs:3609`, start/join 분기 | Direct도 Native `turn/start`를 쓴다. join의 inference_submitted=false는 Native 세션 부재를 뜻하지 않는다. |
| 실행 도구 | `operator.rs:6088`, codex_tools/codex_execute | work/controller와 Native의 전체 도구 카탈로그를 전제로 한다. 현재 로컬 파일 도구의 비세션 경로가 없다. |
| 문맥 회수 | `providers/chatgpt/src/operator_native.rs:225`, public_context | Native rollout 경로를 필수로 읽는다. ephemeral 플래그만 바꾸면 이 계약이 깨진다. |
| 알림 준비 | `crates/manager/src/runtime_config.rs:28`, prepare/render | 저장된 전역 구독에 따라 Native completion callback을 주입한다. 단독/연결 작업을 구분하지 않는다. |
| 알림 전달 | `crates/manager/src/lib.rs:1189`, emit_notification | 이벤트 구독과 thread별 표시를 적용한다. 작업의 기록 소유권 정책은 없다. |
| MCP/local 공통점 | `providers/chatgpt/src/mcp_bridge.rs:1` | 같은 lane 프로토콜에 요청·반환을 맞춘다. transport를 바꾸는 것만으로 Native 실행 계약은 바뀌지 않는다. |

원인은 “direct라는 문자열이 잘못됐다”에 그치지 않는다. 현재 제공된 도구 계약에서
Native 실행 문맥, 로컬 도구, durable Codex 기록이 묶여 있다. 모델에게 standalone이라고
지시하거나 MCP 주소만 나누고 같은 codex_sessions/create를 호출하면 기록은 계속 생긴다.

## 4. 실행 시험과 범위

증거는 `~/.cache/codex/acceptance/work-scope-review-20261031`에 있다. 설치된 signed
Full131/Core/Manager/runtime 바이트를 복사·검증한 disposable fixture를 사용했다.
제품 코드를 수정·빌드하거나 운영 설정·사용자 세션을 바꾸지 않았다. Native 서버 종료는
각 시험에서 소유 PID/실행 파일과 kernel 종료 관찰로 확인했다.

### 4.1 Native 기록·알림 관계

`scope-boundary.json`, 원본 실행 로그 및 fixture의 Native 기록:

| 조건 | Native durable 기록 | completion 알림 명령 | 전체 이력 읽기 |
| --- | --- | --- | --- |
| 일반 Native thread | 있음 |1회 | 성공 |
| threadSource를 standalone으로 분류 | 있음 |1회 | 성공 |
| ephemeral=true | 없음 |1회 | `ephemeral threads do not support includeTurns` |
| thread별 notify=[] | 있음 |0회 관찰 | 성공 |

열린 TUI를 만들지 않았다. 각 turn의 모델 응답은 로컬 시험 서버가1회씩 생성했고 도구
선언은 동일했다. 알림 실행 파일을 시험용 수신기로 교체해 실제 Manager의 호출을
포착했으며, 휴대폰에 실제 알림은 보내지 않았다.

이 결과는 분류 이름·알림 끄기·ephemeral 중 하나를 고르는 것만으로 요구가 충족되지
않음을 보여준다. 특히 ephemeral은 알림도 막지 못하고 현재 Provider의 저장 문맥/
전체 이력·재시작 복구 계약을 잃는다. 로컬 시험의 동일 도구 선언은 임의의 개발 작업
품질이 같다는 증명이 아니다.

### 4.2 Native 세션 없는 읽기·쓰기·실행

`threadless-boundary.json`, `threadless-clean-home.json`과 각각의 실행 로그:

- Core를 통한 실제 app-server에서 `fs/writeFile`, `fs/readFile`, `command/exec` 성공.
- 한글·개행·공백을 포함한 파일 바이트가 정확히 일치하고 명령의 쓰기 효과가1회 발생.
- `thread/start`, `turn/start` 호출0회, loaded thread0개.
- Native DB의 thread 목록과 rollout 파일 목록에 추가 없음.
- 모델 요청0회, 추가 completion 알림 명령0회. 서버 종료 뒤에도 추가 알림 호출 없음.
- 별도 깨끗한 home에서 Codex 인증, Provider 정의, 모델 카탈로그 파일 없이도 동일하게 통과.
  모델 접속은 시험용 수신기로 한정했고 요청이 없음을 확인했다.

따라서 **Codex 실행 파일을 재사용하는 것과 Codex 대화를 만드는 것은 다르다.**
기존 Core의 설치·실행 환경과 업스트림 I/O를 사용하면서 대화 세션을 만들지 않을 수 있다.
아직 ChatGPT Android 앱의 새 MCP 도구를 통해 이 경로를 실행한 것은 아니다.

### 4.3 업스트림 근거와 주의할 차이

조사한 업스트림은 저장소가 채택한 정확한0.166 source
`6ff1e1ca3b30fc8b914720681fcf5fe62e95f856`이다. 미래 버전 지원을 보장하지 않는다.

- `codex-rs/app-server-protocol/src/protocol/v2/command_exec.rs:21`은 command/exec가
  thread/turn 없이 실행됨을 명시한다. 입출력 스트림, stdin, PTY, timeout, terminate와
  permission profile도 이 API 계열에 있다. 이번 실제 시험은 buffered command까지다.
- `.../v2/fs.rs`에 파일 읽기·쓰기·디렉터리·메타데이터·복사·제거 API가 있다.
- `app-server/src/request_processors/command_exec_processor.rs`는 server의 permissions와
  환경으로 실행한다. Native turn의 도구 승인/독립 reviewer 전체를 자동으로 재현하지는 않는다.
- `app-server/src/request_processors/fs_processor.rs:65`의 fs API는 filesystem 객체를
  직접 호출하며 sandbox 인자는None이다. 기존 Native 도구의 승인·경로 정책과 같다고
  가정해 그대로 원격 공개해서는 안 된다. 사용자에게 허용된 읽기·쓰기를 유지하는
  명시적 workspace/접근 권한과 원래 호출의 승인 경계를 연결해야 한다.
- experimental `process/spawn`은 Codex sandbox 없는 프로세스 API다. 이미 sandbox 정책을
  적용하는 command/exec가 있으므로 파일 작업의 기본 executor로 이를 택할 이유가 없다.

## 5. 세 가지 구분을 혼합하지 않는 설계

| 구분 | 답하는 질문 | 권고 |
| --- | --- | --- |
| 실행 형태 | 로컬 도구만 사용할지, Codex 작업을 만들지 | local tools / Codex work를 먼저 분리 |
| 추론 주체 | 현재 ChatGPT가 판단할지, Provider가 모델을 시작할지 | 기존 Native direct/provider의 뜻을 유지하되 적용 범위를 분명히 설명 |
| 연결 방식 | 도구 요청과 결과를 어떻게 운반할지 | MCP / local function은 동일한 실행 의미에 연결되는 adapter |

“범용”은 다양한 작업을 처리할 수 있다는 능력이다. “독립”은 다른 작업과의 실행·기록
관계다. 범용 기능이 곧 모든 일을 Codex에 기록해야 한다는 뜻은 아니다. 독립 작업과
독립 모델 작업자도 다르다. 후자는 추가 추론/별도 문맥과 실제 결과 반환을 입증해야 한다.

권고 사용자 표현:

- **ChatGPT + 로컬 도구:** 이 Chat의 추론과 이력에 도구 결과를 반환. Native 작업 없음.
- **ChatGPT로 Codex 작업:** 현재의 native direct. Codex 기록·작업 제어가 의도된 사용.
- **Provider로 Codex 작업:** native provider. Provider가 모델을 시작하고 Native 작업을 수행.
- **별도 작업자에 위임:** 독립 assignment와 부모/결과 관계를 명시. 실제 새 추론 여부를 표시.

Provider가 별도 ChatGPT 모델을 시작하면서 로컬 도구만 쓰는 조합도 개념적으로 가능하다.
하지만 현재 Native 모델 어댑터가 그것을 완성해 준다고 주장하지 않는다. 별도 작업자의
문맥·도구·승인·Stop·원래 호출 반환을 갖춘 후속 구현이다. 이번 핵심 해결에 필요한
최초 범위는 현재 ChatGPT의 비세션 로컬 도구와 기존 Codex 작업 경로의 분리다.

## 6. 진입점에서의 선택: 매번 추측하지 않고 작업 문맥을 유지

사용자의 추가 핵심 질문은 실행 경로가 분리됐을 때 그 선택 부담이 다시 사용자에게
전가되지 않느냐는 것이다. Codex 진입에는 Native 작업이라는 문맥이 이미 있다.
ChatGPT 진입에는 단독/연결 두 가지 의도가 있어 같은 자연어라도 기록 기대가 달라질 수 있다.
이 차이를 MCP 개수나 모델의 분류 정확도만으로 해결할 수는 없다.

권고는 **진입 기본값 + 작업별 선택 유지 + 명시적 전환**이다.

| 현재 문맥 | 사용자 요청 예 | 적용할 기본 동작 |
| --- | --- | --- |
| 새 Chat, 기존 연결 없음 | 이 파일을 읽고 고쳐줘 | local tools. Native 기록을 만들지 않는다. |
| Codex에서 시작해 연결된 작업 | 이어서 고쳐줘 | 검증된 기존 Native work를 유지한다. |
| ChatGPT에서 이번 작업을 Codex와 연결하도록 선택 | 다음 단계 진행해 | 그 작업의 Native work를 유지한다. 매번 MCP 이름을 요구하지 않는다. |
| 사용자가 별도 단독 작업으로 전환 | 이건 ChatGPT에서만 해 | 별도 local context. 진행 중인 Native 작업을 임의 중단/삭제하지 않는다. |
| 여러 작업 중 어느 것인지 모호하고 기록/실행 범위가 바뀜 | 그것도 같이 해줘 | 작업 식별 또는 모드 선택이 실제로 필요한 경우만 짧게 확인한다. |

명시적 요청이 없으면 실패하도록 만드는 것이 아니다. 사용자가 정한 기본값이나 현재
작업의 의미에 따라 정상 동작하게 한다. 한 번의 자연어 선택, 설정 또는 UI 선택이
후속 도구 호출과 대화 왕복에서 유지돼야 한다. “대화 전체를 영구히 한 모드로 고정”도
아니다. 대화가 여러 작업을 다룰 때는 선택한 작업 context와 그 관계를 유지한다.
같은 폴더/대화 제목/프롬프트 유사성만으로 새 작업을 Native에 묶지 않는다.

모델은 현재 범위 안에서 도구와 개발 방법을 선택한다. Native 기록 생성·기존 work 선택은
저장된 작업 정책과 검증된 연결이 결정해야 한다. 모델이 작업 전환을 제안할 수는 있지만,
`record_in_codex=true` 같은 모델 입력만으로 사용자가 실제 전환을 원했다는 보장을 만들지는 않는다.
기본 local context에서 사소한 파일 도구 호출이 Native work 생성으로 자동 승격되면 안 된다.

보장 수준도 구분한다. 편의 모드는 자연어에 따른 모델의 최초 작업 선택을 시험해 개선한다.
**단독 고정**을 원하면 설정/사용자가 조작하는 선택 UI/고정 연결 프로필이 서버에서
Native 생성·전환을 막아야 한다. 자연어 자동 분류가 항상 정확하다는 약속과 이 강제된
기록 금지 계약을 혼합하지 않는다. UI 선택은 강한 보장의 수단이지 모든 명시적 자연어
요청에 다시 확인을 요구해야 한다는 의무가 아니다.

### 6.1 MCP 한 개와 두 개

초기 검토의 “두 연결을 일반 사용의 기본으로 권고”는 선택 문맥 유지 가능성을
검증하기 전으로서는 이르다. 최종 권고 우선순위를 수정한다. **공통 서비스 하나에서
위 문맥 유지가 실제로 성립하는지 먼저 검증하고, 두 고정 MCP 프로필은 명시적인
진입 선택/단독 고정을 제공하는 수단으로 둔다.** 매 요청마다 둘 중 하나를 모델이
다시 고르게 하는 구조는 어느 경우에도 기본안이 아니다.

| 형태 | 장점 | 한계 / 판정 |
| --- | --- | --- |
| 한 MCP, 지금처럼 Native 도구만 제공 | 기존 코드 유지 | 로컬 도구를 쓰려면 계속 Native 기록이 생김. 요구 미충족 |
| 한 MCP + 유지되는 작업 context | 연결 추가가 적고 후속 입력에 반복 지정 불필요 | 호스트/터널을 통과하는 context 식별과 서버 정책의 실제 검증을 먼저 수행. 우선 검토할 기본 UX |
| 공통 서비스의 고정 local-tools/Codex-work 프로필 | 각 프로필의 도구와 기록 의미가 명확함 | 연결/등록 선택이 추가됨. 단독 고정·명시 진입 수단; 모델이 매번 양쪽 중 하나를 고르면 근본 해결은 아님 |
| 서로 다른 provider/executor 코드·서비스로 복제 | 표면적 구분 | 수정·권한·재시도·배포 규칙이 분기. 불필요한 중복이므로 기각 |

**로컬 도구 연결**과 **Codex 작업 연결**을 별도로 고를 수 있게 할 수는 있다.
실제 ChatGPT에는 필요하면 별도 앱/MCP 연결로 제시하되 내부 구현을 복제하지 않는다.
단독 프로필의 요청이 Native work API로 자동 승격되면 안 된다. 단독 고정 모드는
서버의 프로필 계약으로 보장하고, 모델이 넘기는 `standalone=true` 같은 주장에만 맡기지 않는다.

두 연결을 같은 Chat에서 모두 사용할 수 있게 하면 모델의 잘못된 선택 가능성은 다시
생긴다. 연결을 둘로 만들었다는 사실만으로 사용자 의도를 완전히 보장한다고 말하지 않는다.
단독 고정 모드, 고급 겸용 모드, 명시적 기존-work 선택을 구분해야 한다. 호스트가
실제로 제공하는 대화별 연결 선택/명시 사용의 신뢰 범위를 시험해야 한다.

도구 목록을 턴마다 바꾸는 방식은 기본안으로 삼지 않는다. 기존 queue 스키마 불일치처럼
등록·캐시·운영 도구 목록이 다를 수 있다. 고정 프로필별 local/registered/host 목록을
대조한다. MCP 연결 두 개가 tunnel-client 한 프로필/연결을 공유할 수 있는지는 별도
검증 대상이다. 한 서비스/두 프로필과 실제 tunnel 수를 같은 개념으로 취급하지 않는다.

### 6.2 호스트 식별 정보와 구현 가능성

공식 [OpenAI Plugins Reference](https://developers.openai.com/plugins/reference)는
호스트가 도구 호출에 제공하는 `_meta["openai/session"]`을 대화 내 호출 상관관계용 익명
식별자로 문서화한다. `_meta.ui.visibility`는 UI와 모델에 보이는 도구를 구분하는 기능이다.
이것은 사용자 원문/메시지 ID/branch, 동의 또는 특정 Native work 선택을 증명하지 않는다.

현재 Rust MCP의 `mcp.rs::http`와 `operator.rs::http`는 도구 이름/arguments를 처리하며
이 session metadata를 작업 선택에 사용하지 않는다. 따라서 “MCP에는 대화를 식별할
정보가 전혀 없어서 모델이 매번 work_id를 만들어 전달해야만 한다”는 결론도 성급하다.

제안의 선행 시험은 다음과 같다. 새 제품 빌드/등록 없이 문서만으로 완료 처리하지 않는다.

1. 실제 Android/웹 → tunnel-client → mcp-stdio → Rust에서 해당 metadata가 도달하는지
   제한적으로 관찰한다. 원문/인증 값은 수집하지 않는다.
2. 같은 대화의 다음 턴, Android/웹 왕복, 새 대화, 계정/조직 변경, branch/fork, 연결 재시작
   시 식별 범위를 검증한다. 문서의 표현을 모든 클라이언트/재접속에 대한 영구 보장으로 확대하지 않는다.
3. 실제 인증 주체·연결 범위와 대화 상관관계를 구분해 저장한 모드/선택 work를 조회한다.
   익명 ID 단독을 권한으로 사용하지 않고, URL의 conversation UUID와 동일하다고 가정하지 않는다.
4. metadata 부재/충돌은 공유 서비스의 “마지막 작업”이나 같은 폴더로 채우지 않는다.
   검증된 명시 context 또는 고정 프로필을 사용하고 Native로 조용히 승격하지 않는다.
5. local function은 해당 렌더러가 검증한 lane/대화와 같은 논리 context를 연결한다.

보통의 도구 자동 선택은 모델의 판단이다. 공식
[metadata 가이드](https://developers.openai.com/plugins/guides/optimize-metadata)도
명시·간접·오호출 방지 프롬프트를 구분해 실제 선택 결과를 평가하도록 안내한다.
이를 개선 가능한 동작으로 취급하되, 세션 생성 금지를 그 정확도 하나에 의존시키지 않는다.
사용자가 모드를 선택하는 UI를 도입한다면 UI 전용 변경 도구와 실제 사용자 조작의
경계를 검증한다. 문서에 UI visibility가 있다는 사실만으로 동의가 입증된다고 하지 않는다.

## 7. MCP와 local function

두 방식은 **같은 로컬 실행 계약 또는 같은 Native 작업 계약**을 호출해야 한다.
MCP로 쓰면 기록되고 local function으로 쓰면 기록되지 않는 식의 차이는 두지 않는다.

- local-tools MCP → 공통 local operation → Core app-server fs/command → 원래 도구 호출 반환.
- local-tools local function → 같은 local operation → 같은 fs/command → 원래 함수 호출 반환.
- Codex-work MCP/local function → 기존 Native work/turn/도구·승인/이력 → 원래 호출 반환.

현재 local function은 Provider 소유 Linux 앱 렌더러에 주입하는 경로다. 일반 Android/
웹 ChatGPT에 같은 방식으로 주입할 수 있다고 가정하지 않는다. MCP의 앱/웹 경로와
Provider 렌더러의 local function 경로를 별도로 검증한다. Rust가 I/O, 실행 제어와
권한을 소유하고 JavaScript는 실제 렌더러 함수 등록·요청·반환만 맡는다.

## 8. 기록과 전환 규칙

로컬 도구 사용은 Codex thread/turn, 사용자 프롬프트/답변의 Native 투영 및 Native
completion 알림을 만들지 않는다. 실행 프로세스나 안전한 도구 재시도용 최소 상태가
존재하는 것과 Codex 대화를 저장하는 것은 구분한다. 명령 handle, mutation intent와
결과 반환 상태가 필요할 수 있으나 ChatGPT 전체 대화를 복사하는 새 DB는 만들지 않는다.

사용자가 “Codex로 계속해”라고 하면 그 시점부터 명시적으로 새 작업 또는 선택한 기존
작업에 연결한다. 이전 로컬 명령을 재실행하거나 Chat 전체를 소급 전사하지 않는다.
인계에는 선택한 결과·변경 상태·필요 지침을 반환 결과로 제공하고 원래 호출과 연결한다.
사용자가 Native 기록을 원하는 로컬 개발에는 처음부터 Codex-work를 선택할 수 있게 한다.

기존에 연결된 Chat에서도 새로 요청한 독립 작업은 별도 local context를 선택할 수 있어야
한다. 같은 repository/Chat/title이라는 이유로 기존 Codex 작업을 자동 재사용하지 않는다.
이미 진행 중인 Native 작업의 제어권·대기 승인·큐는 별도 경계로 보호한다.

독립 기록은 파일 격리나 독립 추론을 뜻하지 않는다. 같은 파일을 동시에 수정한다면
권한·기존 writer·작업 폴더 충돌을 확인하고, 필요하면 별도 checkout을 사용하거나
명시적으로 같은 Native 작업에 연결한다. Native 세션을 만들지 않는다고 잠금을 우회하지 않는다.

Codex-work와 오케스트레이션은 원래의 durable 기록/재개 기능을 유지한다. 자식 결과는
부모의 원래 반환 경로로 전달하되 원본 하위 기록을 잃지 않는다. 표시/알림 합치기는
실행 결과·진행·도구 기록의 삭제나 응답 품질 저하로 구현하지 않는다.

과거 Native 기록은 삭제·재분류하지 않는다. 소급 처리는 요구되지 않았고, 생성 당시
사실을 보존한다. 새 프로필 도입은 기존 활성 작업이나 캐시된 MCP 도구를 중간에 바꾸지 않는다.

## 9. 구현 단위와 재사용

공개 동작은 먼저 SPEC에 반영한 뒤 구현한다. 현재의 CLI/profile 및 Provider 시작 설정
수정은 Codex-work 경로에 계속 필요하며 폐기하지 않는다. 새 모델/LLM planner를 만들지 않는다.

1. **프로필/계약:** `config.rs`, `server.rs`, MCP tool-list 생산부, 등록/문서에서 local-tools와
   Codex-work의 계약과 기본값/선택 유지/전환을 정한다. 기존 execution.mode와 reasoning 이름을 standalone 뜻으로
   몰래 재해석하지 않는다. 설치·활성 작업·호스트 캐시 마이그레이션을 명시한다.
2. **Core 접속 재사용:** `operator_native.rs`의 검증된 owner/socket/RPC 수명주기에서 필요한
   공통 책임을 재사용한다. 로컬 작업의 준비는 모델 routing/catalog나 `for_thread`를 요구하지
   않아야 한다. 현재 Pool::prepare를 그대로 쓰면 모델 준비에 다시 결합되므로 그대로 재사용하지 않는다.
3. **로컬 operation:** 기존 `operation.rs` 등의 실제 재시도/불확실 결과 처리 규칙을
   이용해 파일·command API와 원래 호출을 연결한다. Native Broker/controller 토큰이나
   work_id를 만들기 위해 가짜 thread를 생성하지 않는다. 도구만을 위한 두 번째 executor를 쓰지 않는다.
4. **Native 경로 유지:** `operator.rs`의 create/join/start는 명시적인 Codex 작업에 남긴다.
   기존 문맥 수락·승인·계정·모델·writer·복구·이력 및 진행/최종 표시 계약을 보존한다.
5. **운반부:** MCP와 app local function이 같은 operation을 호출하도록 한다. 실제로
   공유하는 책임만 추출하고 discovery/lease/원래 caller 반환의 차이는 유지한다.
6. **알림/UI:** 로컬 도구는 Native 완료를 만들지 않으므로 그 원인의 Codex 알림도 없다.
   Native 작업은 기존 알림을 유지하고, 명시된 오케스트레이션 관계에서만 부모/자식
   표시 정책을 적용한다. ChatGPT 앱 push의 설정·전달은 우리 래퍼가 보장할 수 있는 경계가 아니다.

## 10. 채택 전·구현 후 수용 기준

새 빌드는 이 보고서 작성 때문에 시작하지 않는다. 다음 항목을 실제 변경 묶음의
합격 시험으로 준비한다. 아래 항목 전체가 이미 통과했다는 뜻은 아니다.

1. 일반 Android ChatGPT에서 로컬 파일 읽기·쓰기·Git 확인·테스트 실행 후 Native
   thread/turn/rollout이 추가되지 않고 원래 도구 결과와 최종 답변은 ChatGPT에 온전히 남는다.
2. 같은 작업을 명시적으로 Codex-work로 요청하면 의도한 Native 기록·승인·CLI 표시·알림이 생긴다.
3. 같은 모델/노력도와 작업 자료로 두 경로의 변경 결과·실행 시험·필요 도구·문맥 준수 수준을
   비교한다. 길이/말투나 “성공했다”는 모델 주장만으로 품질 동등성을 판정하지 않는다.
4. 로컬 작업은 Codex 로그인·Provider 모델 설정·Linux 앱 없이 읽기와 쓰기까지 가능해야 한다.
   읽기 전용으로 바꿔 요구를 축소하지 않는다. 첫 cold-start/model discovery도 관찰한다.
5. 실제 접근 범위·쓰기 승인·사용자 정의 정책·비밀값 전달·큰 출력/첨부의 경계를 검증한다.
   fs API나 command API를 Native의 전체 승인/reviewer와 동등하다고 가정하지 않는다.
6. shell stdin/stream/PTY, 취소·disconnect·Stop, 유지 중인 process handle 및 재접속 후
   불확실 효과를 검증한다. mutation의 응답 유실은 자동 재실행을 허용하지 않는다.
7. 단독 고정 MCP 프로필에서 모델이 Native create/join을 호출하려 해도 실행 전에 구분된
   오류가 나며, Native 작업이 자동 생성되지 않는다. 단순 tool description만으로 통과시키지 않는다.
8. 두 MCP 연결이 함께 활성화됐을 때의 선택 오류와 앱의 실제 명시 사용 경계를 시험한다.
   한 MCP에서의 metadata/context 유지도 비교한다. 명시 지시 없는 일반 요청, 기존 작업의
   단순 후속 지시, 무관한 새 요청, Codex를 언급만 한 설명/인용을 별도 평가한다.
   연결명/도구 스키마의 서버·등록·캐시·운영 일치와 tunnel 매핑을 확인한다.
9. local function에서도 동일한 기록 의미·파일/명령 결과를 확인한다. Android MCP 성공을
   렌더러 local function 성공으로 간주하지 않는다.
10. local→Codex 및 반대 전환은 원래 효과를 반복하지 않고 선택한 문맥만 인계한다.
    같은 Chat의 다른 작업과 기존 활성 작업·큐·승인·writer는 손상되지 않는다.
11. 오케스트레이션 부모/자식의 실제 실행·기록 관계와 원래 결과 반환을 검증한다.
    Native 도구 호출 하나를 독립 모델 작업자로 표시하지 않는다.
12. 코드·패키지·기기 활성화·Android 앱에서의 실제 동작·물리적 표시/알림을 각각 판정하고,
    실패를 Native 기록 경로로 조용히 우회해 성공 처리하지 않는다.

## 11. 이번 검토로 바뀌는 우선순위

기존 순서2의 “진입점 동등성”에는 **동일한 Codex 작업을 선택했을 때**라는 전제가 필요하다.
모든 ChatGPT 로컬 작업을 Codex로 들여보내는 것을 동등성으로 구현해서는 안 된다.
이 실행/기록 경계를 먼저 채택한 뒤 순서2의 양방향 연결을 계속한다.

이번 산출물은 원본 대조, Native/알림4조건 시험, 비세션 I/O의 기존·깨끗한 home 시험,
설계 대안 비교 및 이 보고서다. 제품 구현/새 MCP 등록/배포/빌드는 하지 않았다.
시험 driver와 원본 결과·Native 기록은 보존했고, 종료가 확인된 두 fixture의 payload는
clean_development.py로 manifest 대조·보관 후 정리했다. 복원 기록은 같은 증거 루트의
scope-fixture-payload-cleanup.jsonl과 clean-fixture-payload-cleanup.jsonl이다.
남은 핵심 불확실성은 호스트 metadata의 실제 전달/지속 범위, 선택 context의 강제성,
두 MCP 프로필의 tunnel 등록 방식, 전체 권한·도구/문맥 품질 동등성과 장기
process/재접속 수명주기다. 연결 수보다 선택 책임과 선택 유지의 검증이 선행한다.
