# Stage 2 원점 재설계 및 작업 기준서 (`stage2/phase-2.2`)

- **Branch:** `stage2/phase-2.2` (Stage 1 안정 `main` `8e7d1db5` 기반)
- **Lifecycle:** Ephemeral design/execution work order per [docs/INDEX.md](docs/INDEX.md). Stage 2 설계·구현 완료 시 [CURRENT.md](CURRENT.md) / `docs/history/`로 이관 후 정리.
- **Normative authorities:** [SPEC.md](SPEC.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md), [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md), [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/ROADMAP.md](docs/ROADMAP.md). *(참고: `docs/spec/*` 및 `docs/ROADMAP.md`의 구 `0.167.x` 결합 계약 정비는 덩어리 3에서 의도적 개정 진행)*

---

## 1. 확정된 최상위 설계 원칙 (Stage 2 원점 재설계)

1. **전면 결합도 0 수렴:** Wrapper(`codex`)는 Termux 실행(FD 33/34/35) + 공식 업스트림 `update`/`rollback`/`doctor` + 관리형 데몬 동기화 + 경량 서명 검증만 담당한다. 다계정 프로필·알림·Provider·MCP·터널은 모두 독립 컴포넌트/서비스로 분리한다.
2. **업스트림 소스 패치 0 (무수정 추종):** `native_route.patch`(14.5K줄 코어 포크)와 `frontend/tui/native.patch`(4.7K줄 TUI 포크)를 **모두 폐기**한다. 공식 업스트림 `bin/codex`와 `app-server daemon`을 바이너리 ELF 패치만으로 즉시 추종한다.
3. **자체 작업 컨트롤러/DDPE 배제:** `shared_server`, FD 36 `execution_lease`, `codex-runtime-binding`, `0.167.2` DDPE 멀티워커 오케스트레이션(별도 트랙)을 Stage 2에 끌고 오지 않는다. Codex 작업 소유권은 공식 `codex app-server daemon`에, ChatGPT 세션 소유권은 ChatGPT 앱에 둔다.
4. **인증·프로바이더 직교 모델:**
   - **OpenAI OAuth 다계정 (`CODEX_HOME` 분리, `/switch`)**: `auth.json`과 `app-server-daemon` 소켓만 계정별 격리, 대화/SQLite/설정은 `CODEX_SHARED_HOME` 공유. 동일 OpenAI 프로바이더 내 계정 전환은 같은 스레드 `resume <uuid>` 이어받기.
   - **외부 프로바이더 (`ais.config.toml`, `chatgpt.config.toml` 등)**: `requires_openai_auth = false`로 `auth.json`을 사용하지 않으므로 단일 기본 홈(`~/.codex`)에서 `-p <profile>`로 실행하며, 프로바이더 변경은 **새 스레드**로 시작(업스트림 `0.171.0` `threads.model_provider` 정책 일치).
5. **경량 서명 배포:** 레거시 ~3,500줄 CDN/`asset-map`/`Base·Full`/`release_display` 체계를 폐기하고, GitHub Release 기반 경량 **`SHA256SUMS.sig` Ed25519** 검증으로 단순화한다.
6. **라이브 기기 안전성:** 기존 `$PREFIX/bin/codex`, `0.167.2` 기반 `codex-chatgpt-*` 서비스, `~/.cache/codex-web-experiment/` (`26.1007.21434`), `~/.config/codex/providers/chatgpt.toml`, `ais.config.toml`/`chatgpt.config.toml`은 일절 건드리지 않고 새 경로에서 병렬 구축·검증 후 명시적 컷오버한다.

---

## 2. [확정 완료] 덩어리 1 — Provider · 3종 MCP · AppHost · Tunnel · Supervision

### 2.1 컴포넌트 및 단방향 의존성 지도 (순환 0 · Wrapper 결합도 0)

```text
[1. @TermuxShell]   mcp/termux-shell   (~700줄 · 추출)
        ▲             └─ 의존성 0: Codex·Provider·Linux 앱 전부 죽어도 생존 (비상/복구 평면)
        │ (runit: termux-shell-tunnel)
   Android ChatGPT
        ├── (runit: termux-chatgpt-tunnel) ──▶ [4. @TermuxChatGPT] mcp/termux-chatgpt (그린필드)
        │                                             │ (로컬 인증 HTTP /control/*)
        │                                             ▼
        │                                      [3. chatgpt-provider] providers/chatgpt (단독 CDP + API)
        │                                             ▲                        │
        │                                             │ (/v1/responses +       │ (Xvnc + PRoot)
        │                                             │  앱 도구 콜백)          ▼
        │                                      [Codex CLI -p chatgpt]   [2. chatgpt-desktop]
        │                                                               (불변 rootfs 제너레이션 +
        │                                                                분리된 영구 profile)
        │
        └── (runit: termux-codex-tunnel)   ──▶ [5. @TermuxCodex] mcp/termux-codex (그린필드)
                                                      │ (공개 JSON-RPC UDS)
                                                      ▼
                                               [공식 codex app-server daemon]
```

### 2.2 컴포넌트별 확정 사양 및 소스 전략

| 컴포넌트 | 경로 / 크레이트 | 바이너리 / 서비스 | 커넥터 | 소스 전략 & 핵심 경계 |
| --- | --- | --- | --- | --- |
| **호스트 셸 MCP (비상/독립)** | `mcp/termux-shell` | `termux-shell-mcp` · `termux-shell-tunnel` | **`@TermuxShell`** | `fa1b1713:termux_shell.rs`(683줄) 독립 추출 + `MCP_SHELL.md` 정식 편입. Codex·Provider·AppHost 의존성 0 |
| **Linux ChatGPT AppHost** | `host/chatgpt-desktop` | `~/.local/lib/chatgpt-desktop/{generations/<ver>/rootfs,current,previous,viewer}` | — | `~/.cache` 탈출, 공식 OpenAI ARM64 APT(`https://persistent.oaistatic.com/codex-app-prod/linux/deb`, GPG 검증) 기반 불변 제너레이션 + 영구 로그인 프로필(`~/.local/share/chatgpt-desktop/profile`, `flock` 리스) 분리. 기존 `26.1007.21434` 백업 유지 후 최신 `.deb` 격리 검증 |
| **유형 1 Provider (단독 CDP)** | `providers/chatgpt` | `chatgpt-provider` (`runit: chatgpt-provider`) | — | `host`/`cdp`/`auth_browser`/`viewer`/`responses`/`server`/`models`/`chats`/`project`/`registration` 코어 발라내기 + **`page_driver.js` 클린룸 그린필드 작성**(구 `app_bridge.js` 완전 폐기) + DDPE/MCP/`shared_server`/`v1~v3` ~40K줄 제거. `/v1/responses`(앱 도구 콜백 단일 경로, 내부 필드 `local_function_signatures`) + 로컬 `/control/*`만 노출. 중복 `codex-chatgpt-host` 셸 keepalive 서비스 폐지 |
| **유형 2-A ChatGPT MCP** | `mcp/termux-chatgpt` | `termux-chatgpt-mcp` · `termux-chatgpt-tunnel` | **`@TermuxChatGPT`** | 그린필드. `chatgpt-provider` `/control/*`만 중계 (로컬 코드 실행 권한 0) |
| **유형 2-B Codex MCP** | `mcp/termux-codex` | `termux-codex-mcp` · `termux-codex-tunnel` | **`@TermuxCodex`** | 그린필드. 공식 `codex app-server daemon` 공개 JSON-RPC만 중계 (자체 작업 DB 0, 향후 공식 `remote-control` 성숙 시 단독 퇴역 가능) |
| **터널 전송 계층** | `transport/tunnel-client` | `tunnel-client` (`~/.local/lib/tunnel-client/{generations/<ver>,current,previous}`) | — | `5c495538` `v0.0.20` 기준 + `build_tunnel_client.py` GitHub API 403 폴백 추가 + CI 사전빌드 경량 서명 배포 |

### 2.3 3종 MCP 최초 도구 구성 & 불변 규약 (실증 시 확장 검토)

- **공통 어노테이션 일괄 정책 (불변):** 3개 MCP 전 도구에 `readOnlyHint: true`, `destructiveHint: false`, `openWorldHint: false` 일괄 부여 + 각 도구의 실제 멱등 계약에 맞는 `idempotentHint` 선언.
- **상호 배타성(Disjoint) 원칙:** 정적 핵심 도구와 `*_catalog` / `*_call` 확장 슬롯은 완전 상호배타(중복 노출 금지). 검증 실패 시 해당 도구의 정확한 JSON 스키마를 에러 본문에 즉시 반환. 임의 스크립트 실행 금지(명시적 allowlist 레지스트리만 노출).
- **경량 `call_id` 전송 중복방지 영수증:** `termux_shell`, `chatgpt_send`, `codex_turn(start/steer)`는 모바일 HTTP/2 단절 재시도 방지를 위해 소형 bounded `call_id` 전송 영수증만 기록(자체 작업 DB/상태머신 금지; 실제 상태는 항상 라이브 `app-server` / `chatgpt-provider`에서 조회).
- **선택적 `@Termux` 스킬:** MCP에 별도 `guide` 도구를 추가하지 않으며, `@Termux` ChatGPT 스킬은 순수 opt-in 부가 편의로만 취급.

| MCP 커넥터 | 정적 핵심 도구 (1발 실행 · 제약 디코딩) | 상호배타 확장 슬롯 (무리프레시) | 확장 슬롯 초기 후보 |
| --- | --- | --- | --- |
| **`@TermuxShell`** *(1개)* | `termux_shell` (`call_id`, `command`, `cwd?`, `timeout_ms?`) | *(없음 — 영구 불변)* | — |
| **`@TermuxChatGPT`** *(6개)* | `chatgpt_status` (읽기: 앱·인증·모델)<br>`chatgpt_chats` (`list \| read`, 페이지네이션)<br>`chatgpt_projects` (`list \| read`, 순수 읽기)<br>`chatgpt_send` (`send \| observe \| stop`, `call_id` 멱등 + bounded `wait_ms`) | `chatgpt_catalog`<br>`chatgpt_call` | `project_create`, `chat_rename`, `viewer_url`, `host_restart` |
| **`@TermuxCodex`** *(5개)* | `codex_status` (읽기: 데몬·로드 스레드·프로바이더·선택 모델)<br>`codex_threads` (`list \| read \| items`, 순수 읽기·페이지네이션)<br>`codex_turn` (`start \| steer \| interrupt`, `call_id` 멱등 + bounded `wait_ms`; `thread_id` 생략 시 새 스레드+첫 턴 1발 시작) | `codex_catalog`<br>`codex_call` | `thread_fork`, `thread_archive`, `thread_unarchive`, `thread_rename`, `thread_compact`, `review_start`, `account_usage`, `goal_*` |

### 2.4 수퍼비전 계층 (`runit`) 불변식
- **opt-in 등록 원칙:** 바이너리 설치/업데이트만으로 새 `runit` 서비스를 자동 등록하지 않음.
- **안정 `current` 링크 실행:** `run` 스크립트는 버전 하드코딩 env 파일 대신 항상 검증된 `current` 심볼릭 링크를 실행.
- **3상태 분리 보고:** `status`는 ① `selected generation`, ② `running generation`(`/proc/<pid>/exe` + readiness), ③ `activation state`(`active` / `pending_activation` / `error`)를 명확히 분리.
- **안전 재바인딩 & 롤백:** 활성 리스/스트림 안전 경계 확인 → `sv restart` → PID/실행파일/readiness 검증 → 실패 시 `current → previous` 롤백 (활성 작업 중이면 강제 중단 대신 `pending_activation` 보고).

---

## 3. [확정 완료] 덩어리 2 — Wrapper 슬림화 · `codex-ext` (`cdx`) · Termux 전환/알림 UX

### 3.1 Wrapper 본체(`codex`) 경계 고정 및 Phase 2.1 잔여 코드 제거
- **`$PREFIX/bin/codex` 정체성 고정:** 온전히 Stage 1 업스트림 실행(`execve()` 즉시 퇴장) + 바이너리 ELF 패치 + `update`/`rollback`/`doctor` + 관리형 데몬 동기화 + 경량 서명 검증에 국한한다.
- **Phase 2.1 잔여 코드 제거 ([SPEC.md](SPEC.md) 개정 동반):**
  - 소비자 0인 `crates/wrapper/src/system_config.rs`(799줄 FD 34 스냅샷 엔진) 완전 제거.
  - `crates/wrapper/src/extensions.rs` 제거 및 [wrapper_cli.rs](crates/wrapper/src/wrapper_cli.rs)의 미사용 `manager`·`tui`·`provider`·`--with`·`--bare`·`setup --enable/--disable` 죽은 분기 제거 (`codex wrapper status [--verify] [--json]` 중심으로 슬림화).

### 3.2 독립 확장 바이너리 `codex-ext` (축약 심볼릭 링크 `cdx`)
- **패키징 & 독립 주기:** 독립 크레이트(`crates/codex-ext`, 자체 `[workspace]`) → 단일 바이너리 `$PREFIX/bin/codex-ext` + 축약 심볼릭 링크 `$PREFIX/bin/cdx`. 업스트림 `codex` 버전업 시 수정·빌드·배포 0회(완전 독립 릴리스).
- **기존 `contrib/codex-account` 흡수 (Slice 2에서 퇴역 완료):** `codex-ext` 자격 검증 완료 후 `contrib/codex-account` 퇴역 (`~/.local/share/codex/manager/profiles/` 기존 프로필 디렉터리 호환 유지).
- **핵심 서브커맨드:**
  - `cdx [use <acct>] [CODEX_ARGS...]` — non-PTY 상위 프로세스 런처(Termux TTY `fd 0/1/2` 100% 직결, PTY 프록시 0, 장문 OSC 52 `/copy`·`SIGWINCH` 무손실) + 원본 `termios` 백업 + `~/.local/state/codex-ext/sessions/<pid>.json`(`0600`) 세션 등록.
  - `!cdx switch <acct> [<tid>] [--wait]` & `!cdx provider <openai|chatgpt|ais|...>` — 공식 `0.171.0` TUI 내장 `!` 명령을 이용한 **세션 내 즉시 전환** (업스트림 소스 패치 0):
    - `0.171.0` 실측 반영: `!` 명령은 `app-server daemon` 자식으로 실행되어 데몬 환경(`CODEX_HOME`, `cwd`)을 상속함 → `($CODEX_HOME, cwd)` + 살아 있는 PID 검증으로 부모 `codex-ext` 특정(복수 창 시 `--session` 명시 안내, bare `codex` 실행 시 수동 전환 안내).
    - **사전 검사로 헛걸음 0%:** 데몬 A의 다른 활성 턴 존재 시 TUI를 닫지 않고 그 자리에서 경고 출력 후 종료; 유휴(95%)일 때만 `0600` 핸드오프 티켓 작성 → `SIGUSR1` 발송 → 즉시 `exit 0`(자기종료 레이스 차단).
    - **부모 `codex-ext` 전환 절차:** `codex TUI` `SIGTERM` 종료(실측 `0.02s`) → 백업된 `termios`/alt-screen 복원 → 유휴 정착 확인 후 데몬 A `daemon stop`(~0.2s) → `<tid>.lock` 해제 확인 → 같은 터미널에서 즉시 `codex resume <tid>` (또는 새 스레드 `codex -p <provider>`) 재기동.
    - 셸 직접 실행 `cdx switch <acct>` 충돌 시 에러 후 방치 대신 `[w] 대기 / [r] 원래 세션 복귀 / [q] 취소` 대화형 안전 복귀 제공.
  - `cdx profile list|current|create|delete|rename|default` — OpenAI OAuth 다계정 CRUD (외부 프로바이더 `ais`/`chatgpt`는 기본 홈 `~/.codex` 단일 실행).
  - `cdx notify show|set|test|emit <EVENT>` — **A안 확정:**
    - `Stop`(응답 완료): `~/.codex/config.toml`의 `notify = ["codex-ext", "notify", "emit", "Stop"]` (신뢰 승인 팝업 없이 전 계정·비대화형 즉시 동작; 기존 사용자 설정·주석 보존 원자적 편집).
    - 세부 이벤트(`UserInputRequest`, `PermissionRequest`, `SubagentStop` 등): `~/.codex/hooks.json` 분리 관리 + `CODEX_SHARED_EXTRA_FILES=hooks.json`으로 전 계정 공유(Wrapper 코드 수정 0줄; `0.171.0` 해시 신뢰 승인 안내).
    - 전송 수단: 표준 Termux:API (`termux-notification` / `termux-toast`).
- **업스트림 스레드 수명 불변식 (`dc41a272` 교훈 유지):**
  - `thread_unload_delay_secs`는 **업스트림 기본값 유지** (`= 0` 금지 — `queue`/단절 시 `turn_aborted` 재발 원천 차단).
- **PTY 프록시 미도입** (기본 완성 후 필요 시 `tmux` 팝업 레시피만 별도 제공).
- **검증 게이트:** 공식 `0.171.0` Termux 실기 `/copy` OSC 52 장문 상한 확인, [docs/ROADMAP.md](docs/ROADMAP.md) §7(동일 UUID `thread-writer-locks/` 해제 & `session_index.jsonl` 권위 원천 검증), `!cdx switch` 데몬 자식/자기종료 레이스/복수 창 테스트.

---

## 4. [확정 완료] 덩어리 3 — 공용 경량 서명 배포 · `docs/spec/` 정비 · Slice 1→6 실행 계획

### 4.1 공용 경량 Ed25519 서명 배포 체계 (`SHA256SUMS.sig`)
- **공통 포맷:** 레거시 CDN/`asset-map`/`update-index`를 폐기하고, 모든 자체 빌드 컴포넌트의 GitHub Release에 **`SHA256SUMS`**(상단 헤더 `# component=<name>`, `# sequence=<N>`, `# commit=<40-hex>` + 아카이브 SHA-256) + **`SHA256SUMS.sig`**(단일 Ed25519 서명)를 공통 적용한다.
- **검증 3단계:** ① 핀된 공개키(`release-public-key.pem`, pinned SHA-256)로 `SHA256SUMS.sig` 검증(`openssl pkeyutl -verify -pubin -rawin`) → ② 컴포넌트별 `sequence` 단조 증가(anti-rollback) 확인 → ③ 아카이브 SHA-256 일치 확인 후 설치/교체. 키 교체 시에만 `release-public-key.pem.sig` 로테이션 체인 지원.
- **신뢰 키:** Stage 2 전용 신규 Ed25519 키쌍 1개 (`0.167.2` 레거시 키와 완전 분리; 개인키는 GitHub Actions Secret `CODEX_SIGNING_KEY`에만 보관, 공개키는 [release-public-key.pem](release/release-public-key.pem) SHA-256 `0df170dae2d7234b4425fcdddaa4bd9b6e2104794d0541955b022631b1105fca` 핀).
- **Stage 1 bare 무영향:** 공식 OpenAI 업스트림 `codex update`는 기존대로 OpenAI SHA-256 + 로컬 ELF 패치(`LOCAL_ZERO`)를 유지하며 `openssl-tool`이나 서명 키를 요구하지 않는다.
- **릴리스·태그 현황:** `humtr/codex` 원격에는 현재 Stage 1 `wrapper` 태그/릴리스 1개만 존재(구 `0.167.2` 릴리스 없음). Slice 1 완료 시 경량 서명 `wrapper` 릴리스로 갱신하고, 이후 컴포넌트별 독립 태그로 발행한다.

### 4.2 모노레포 내 독립 워크스페이스 · 공용 내부 크레이트 · CI 격리

| 컴포넌트 | 독립 워크스페이스 / 경로 | CI 트리거 | GitHub Release 태그 |
| --- | --- | --- | --- |
| Wrapper (`codex`) | 루트 [Cargo.toml](Cargo.toml) (`libc` 단독 유지) | `crates/wrapper/**`, `crates/release-builder/**`, `install.sh` | `wrapper` |
| Extension (`codex-ext` / `cdx`) | `crates/codex-ext/Cargo.toml` (`[workspace]`) | `crates/codex-ext/**`, `crates/codex-daemon-client/**` | `codex-ext` |
| `@TermuxShell` | `mcp/termux-shell/Cargo.toml` (`[workspace]`) | `mcp/termux-shell/**`, `crates/mcp-kit/**` | `termux-shell-mcp` |
| `@TermuxCodex` | `mcp/termux-codex/Cargo.toml` (`[workspace]`) | `mcp/termux-codex/**`, `crates/mcp-kit/**`, `crates/codex-daemon-client/**` | `termux-codex-mcp` |
| `chatgpt-provider` + AppHost | `providers/chatgpt/Cargo.toml` (`[workspace]`) | `providers/chatgpt/**`, `host/chatgpt-desktop/**` | `chatgpt-provider` |
| `@TermuxChatGPT` | `mcp/termux-chatgpt/Cargo.toml` (`[workspace]`) | `mcp/termux-chatgpt/**`, `crates/mcp-kit/**` | `termux-chatgpt-mcp` |
| `tunnel-client` | `transport/tunnel-client/` (Go) | `transport/tunnel-client/**`, `scripts/build_tunnel_client.py` | `tunnel-client` |

- **초경량 공용 내부 크레이트 2개 (루트 `codex` Wrapper 무의존, path 참조 전용):**
  1. `crates/codex-daemon-client` (~250줄, Slice 2 추출 → Slice 4 재사용): 공식 `codex app-server daemon` UDS 발견 및 공개 JSON-RPC 클라이언트.
  2. `crates/mcp-kit` (~350줄, Slice 3 추출 → Slice 3·4·6 공용): MCP stdio JSON-RPC 루프, 불변 어노테이션 강제, bounded `call_id` 전송 영수증, 상호배타 `catalog`/`call` 헬퍼.

### 4.3 `docs/spec/` 점진적 슬림화 전략
- 설계 확정 직후: [SPEC.md](SPEC.md) + [docs/ROADMAP.md](docs/ROADMAP.md) + [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md)(경량 서명) 동기화.
- Slice 2 직전: [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md)를 `codex-ext`/`cdx` · `notify` A안 중심으로 슬림화.
- Slice 3~6 직전: `docs/spec/MCP_SHELL.md` 편입 + [docs/spec/BRIDGE.md](docs/spec/BRIDGE.md)를 3종 MCP · `chatgpt-provider` · `runit` 3상태 수퍼비전 중심으로 슬림화.

### 4.4 확정 실행 순서 (의존성 DAG · Slice 1 → 6)

```text
Slice 1: Wrapper 슬림화 + 공용 SHA256SUMS.sig 경량 Ed25519 서명·배포 기반
  │       ▶ crates/wrapper 영구 종결 + 공통 서명/릴리스 파이프라인
  │
  ├──▶ Slice 2: codex-ext (cdx) + [공용] codex-daemon-client + §7 / OSC 52 실증
  │      │     ▶ !cdx switch / !cdx provider / profile / notify 실사용 + contrib/codex-account 퇴역
  │      │
  └──▶ Slice 3: [공용] mcp-kit + @TermuxShell + tunnel-client (v0.0.20+, 403) + runit 3상태 수퍼비전
         │     ▶ 비상 복구 평면 @TermuxShell 독립 가동 + 수퍼비전 표준 확립
         │
         ├──▶ Slice 4: @TermuxCodex (5도구 그린필드)
         │             ▶ Linux 앱 없이 모바일 Codex 원격 제어 실사용
         │
         └──▶ [Gate 5-0: 최신 chatgpt .deb 격리 PRoot/Xvnc 부팅 · CDP 관찰]
                │
                ▼
              Slice 5: host/chatgpt-desktop (제너레이션 설치/롤백) + providers/chatgpt (page_driver.js 클린룸)
                │      ▶ 유형 1 (codex -p chatgpt) 완결
                │
                └──▶ Slice 6: @TermuxChatGPT (6도구 그린필드) + 선택적 0.167.2 컷오버
                              ▶ 유형 2-A 완결 + Stage 2 전체 마감
```

---

## 5. Slice 1 체크리스트 (완료 — `main` `3ea207cb` · 실기기 컷오버 마감)

- [x] 덩어리 1·2·3 설계 확정 및 `HANDOFF.md` 고정.
- [x] [SPEC.md](SPEC.md), [docs/ROADMAP.md](docs/ROADMAP.md), [docs/spec/DELIVERY.md](docs/spec/DELIVERY.md) 규범 동기화.
- [x] Wrapper Phase 2.1 잔여 코드 제거 (`system_config.rs`, `extensions.rs` 제거 및 [wrapper_cli.rs](crates/wrapper/src/wrapper_cli.rs) 슬림화, 관련 호출부·테스트·[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)·[wrapper-verify.yml](.github/workflows/wrapper-verify.yml) 정리).
- [x] 공용 경량 `SHA256SUMS.sig` Ed25519 서명/검증 구현 ([release_sign.py](.github/scripts/release_sign.py), [publish_wrapper_release.py](.github/scripts/publish_wrapper_release.py), [wrapper.yml](.github/workflows/wrapper.yml), [install.sh](install.sh) 사전 검증 + sequence anti-rollback + `release-public-key.pem` 핀/로테이션 검증).
- [x] 출구 게이트: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (umask `0022`/`0077`), `python3 -m unittest discover -s .github/scripts`, `check_doc_links.py`, `shellcheck install.sh`, 서명 변조·다운그레이드·로테이션·미서명 로컬 폴백 테스트 통과.
- [x] Stage 2 전용 Ed25519 키 프로비저닝(`CODEX_SIGNING_KEY`, 공개키 핀 `0df170da...`), `main` 승격 및 [Wrapper CI run 44997871190](https://github.com/humtr/codex/actions/runs/44997871190) 서명 `wrapper` 릴리스 발행(`sequence=1`, `prerelease=false`), 실기기 Termux `--require-signed --skip-upstream` 컷오버 완료 (상세 영수증 [CURRENT.md](CURRENT.md)).

> **Slice 2 완료 (`main` 머지 & 실기기 `$PREFIX/bin/codex-ext`·`cdx` 설치 완료 — 2026-11-22):**
> - [x] [docs/spec/USER_INTERFACES.md](docs/spec/USER_INTERFACES.md) 슬림화 (`codex-ext`/`cdx` · `notify` A안 · `!cdx switch`/`!cdx provider` 규범 확정).
> - [x] 공용 [crates/codex-daemon-client](crates/codex-daemon-client/src/lib.rs) (독립 `[workspace]`, RFC 6455 WebSocket-over-UDS + Termux FD 35 `dirfd` 해결 + `flock` writer-lock 프로브).
> - [x] 독립 [crates/codex-ext](crates/codex-ext/src/main.rs) (`cdx` 논-PTY 런처, `!cdx switch`/`!cdx provider`, `profile` CRUD, `notify` A안, `status`) + 독립 경로 트리거 CI ([codex-ext.yml](.github/workflows/codex-ext.yml)).
> - [x] 사후 감사·개선 (`SIGINT` 부모 보호, 데몬 프로브 fail-closed, 외부 프로바이더→계정 전환 가드, symlink 경로 정규화, 멀티라인 `notify` TOML, `codex-ext.yml` 시퀀스 단조증가·readback), §7 격리 실증, `contrib/codex-account` 퇴역, 실기기 설치(`138def40...`) 완료.
> **Slice 3 완료 (`main` 머지 & `~/.local/lib/termux-shell-mcp/current/termux-shell-mcp` + `$PREFIX/bin/termux-shell-mcp` 설치 완료 — 2026-11-22):**
> - [x] 공용 [crates/mcp-kit](crates/mcp-kit/src/lib.rs) (독립 `[workspace]`, 불변 어노테이션, 상호배타 `catalog`/`call` 레지스트리, `O_NOFOLLOW` + `fsync` `call_id` 영수증 저널·상한 프루닝, 바운디드 stdio JSON-RPC 루프).
> - [x] 독립 [mcp/termux-shell](mcp/termux-shell/src/main.rs) (`@TermuxShell` `termux-shell-mcp`, 정적 1도구 `termux_shell`, 비-tokio 경량 프로세스 그룹/바운디드 캡처, `termux-shell-v1/v2` 호환) + [docs/spec/MCP_SHELL.md](docs/spec/MCP_SHELL.md) + [termux-shell.yml](.github/workflows/termux-shell.yml).
> - [x] [transport/tunnel-client](transport/tunnel-client/recipe.json) (`v0.0.21` + `termux.patch` HTTP/2 half-open PING), [scripts/build_tunnel_client.py](scripts/build_tunnel_client.py) (GitHub API 403 리다이렉트 폴백 + NDK 크로스컴파일 지원) + [tunnel-client.yml](.github/workflows/tunnel-client.yml) (Go `1.27.4`).
> - [x] opt-in `runit` 3상태 수퍼비전 ([scripts/runit_service.py](scripts/runit_service.py) + [docs/spec/SUPERVISION.md](docs/spec/SUPERVISION.md) + 지연 활성화 재시도·롤백 시 `previous` 보존 테스트).
> **Slice 4 구현 진행 (`stage2/slice-4-termux-codex` — 2026-11-22):**
> - [x] 독립 [mcp/termux-codex](mcp/termux-codex/src/main.rs) (`@TermuxCodex` `termux-codex-mcp`, 정적 5도구 `codex_status`/`codex_threads`/`codex_turn`/`codex_catalog`/`codex_call` + 상호배타 10개 확장 액션 + `call_id` 멱등 영수증 + `codex-daemon-client`/`mcp-kit` 재사용) + [docs/spec/MCP_CODEX.md](docs/spec/MCP_CODEX.md) + [termux-codex.yml](.github/workflows/termux-codex.yml).
> - [ ] **다음 착수:** Slice 4 사후 감사·개선 → `main` 머지/푸시 → **Slice 5 (`Gate 5-0` + `host/chatgpt-desktop` + `providers/chatgpt` 클린룸 `page_driver.js`)**.
