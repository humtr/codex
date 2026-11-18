# 공유 데몬 패키지 전환 · 정리 · 세션 인계 검증 계획

**상태:** **1~3장 대체됨(Superseded for Stage 1)** — Stage 1 래퍼가 새 실행 홈의 관리 데몬 첫 기동 시 비파괴 방식의 `$CODEX_HOME/packages/app-server-daemon/current -> $HOME/.codex/packages/standalone/current` 기본 링크를 구현함으로써, 중립 공용 저장소(`$HOME/.local/share/codex/daemon-packages/`), `daemon-maintenance` 전용 홈, 별도 3세대 데몬 프루너를 두지 않기로 확정·구현했습니다.
기존에 전용 `packages/app-server-daemon/releases/` 사본을 보유한 라이브 홈을 정리하려면 별도 승인된 일회성 정리 작업이 필요하며(래퍼 변경만으로는 기존 점유 용량이 자동 회수되지 않음), **4장(`/switch` UUID 인계 및 `session_index.jsonl` 정합성 검증안)은 유효한 후속 과제로 그대로 유지됩니다.**
(참고: 이 문서의 1~3장은 2026-11-18 초기 비교 당시 임시 검증 기록 `daemon-location.SFQRwC/location.txt`, `swR.ryJJ8I/verification.txt`, `srvR.PgsDPG/verification.txt`를 바탕으로 작성했던 **구 공용 저장소안 기록**으로만 보존합니다.)
이번 단계에서는 실사용 계정 홈, `packages/app-server-daemon`, S27 `runit` 서비스, 기존 백업을 변경하지 않습니다.

---

## 1. (대체됨) 구 공용 저장소 배치 구조 (참고용)

| 구성 요소 | 경로 | 역할 및 소유권 |
| --- | --- | --- |
| 공용 데몬 패키지 실경로 | `$HOME/.local/share/codex/daemon-packages/` | 업스트림 관리 패키지 루트 전체(`releases/`, `current`, `install.lock`, `selection-history`, `auto-update-version`). `releases/`만 단독 링크하지 않고 루트 전체를 단일 실경로로 유지 |
| 기본 계정 데몬 패키지 링크 | `$HOME/.codex/packages/app-server-daemon` → 공용 실경로 | 심볼릭 링크만 연결; `auth.json`, `app-server-control/`, `app-server-daemon/`, `packages/standalone/`은 `$HOME/.codex` 소유 유지 |
| 등록 계정 데몬 패키지 링크 | `$HOME/.local/share/codex/manager/profiles/<ACCOUNT>/home/packages/app-server-daemon` → 공용 실경로 | 심볼릭 링크만 연결; 각 계정의 `auth.json`, `app-server-control/`, `app-server-daemon/`(`settings.json`, `daemon.pid`, `daemon.stderr.log`)은 프로필별 격리 유지 |
| 유지보수 전용 `CODEX_HOME` | `$HOME/.local/state/codex/daemon-maintenance/` | `packages/app-server-daemon`만 공용 실경로로 링크. `HOME`은 유지하고 `CODEX_SHARED_HOME`·`CODEX_SQLITE_HOME`은 상속하지 않음. 계정 레지스트리 미등록, 로그인·데몬 기동 없이 최초 선택 이후 `app-server daemon update --from-cli` 실행 전용 |

### 배치 불변식
1. **실물 저장소 직접 사용 금지:** 어느 사용자 계정의 `CODEX_HOME`에도 공용 실물 루트를 직접 두지 않고, `$HOME/.local/share/codex/daemon-packages` 자체를 `CODEX_HOME`으로 설정하지도 않습니다(업스트림 별칭 프루닝 우회 조건이 기본 계정 실디렉터리에는 적용되지 않기 때문).
2. **새 계정 참여 전 선행 조건:** 새 계정을 공유 데몬 패키지 방식에 참여시킬 때는 **첫 관리 데몬 기동 전에** `<ACCOUNT>/home/packages` 실디렉터리(`0700`)와 `app-server-daemon -> $HOME/.local/share/codex/daemon-packages` 심볼릭 링크를 먼저 준비하는 명시적 단계를 둡니다.
3. **`installation_id` 및 기존 공유 링크 보존:** 기존 공유 `installation_id`와 [USER_INTERFACES.md](../spec/USER_INTERFACES.md)의 `CODEX_SHARED_HOME` 심볼릭 링크 토폴로지를 그대로 유지합니다. 완전히 미초기화된 환경에서 검증할 때는 기본 홈 정식 초기화로 공유 `installation_id`를 먼저 생성·연결한 뒤 다중 홈을 시작하며, 이미 충돌하는 `installation_id`를 자동 삭제하거나 덮어쓰지 않습니다.
4. **소유자·권한 검증:** `$HOME/.local/share/codex/daemon-packages`와 `$HOME/.local/state/codex/daemon-maintenance`는 각각 본인 소유 실제 `0700` 디렉터리여야 하며, 대상 경로가 이미 존재하면 유형·소유·내용을 먼저 검증한 뒤 진행합니다.

---

## 2. (대체됨) 구 공용 저장소 일회성 이관 및 복구 절차 (참고용)

### 2.1 사전 점검 (변경 없음)
1. **대상 홈 및 실행 주체 파악:** `default` (`$HOME/.codex`)와 모든 참여 프로필(`janmori101`, `szob` 등)에 대해 현재 선택 세대(`packages/app-server-daemon/current`), 직전 세대(`releases/` 하위), `daemon.pid`, 실행 중인 `/proc/<pid>/exe` 및 열린 `/proc/<pid>/fd` 참조, 각 홈의 `packages/app-server-daemon` 디스크 할당량을 기록합니다.
2. **목표 경로·파일시스템 검증:** `$HOME/.local/share/codex/daemon-packages`와 `$HOME/.local/state/codex/daemon-maintenance`의 부재 또는 안전성(상위 디렉터리 `0700`·심볼릭 링크 없음·동일 파일시스템)을 확인하고, 모든 프로필의 `installation_id`가 기본 홈의 공유 `installation_id`와 동일 inode인지 확인합니다.

### 2.2 정비 창 확보
- S27 `runit` 서비스(`~/.p167`, `~/.local/state/codex/providers/chatgpt`)와 `codex167`(`~/.codex-167`) 등 **무관한 서비스는 중단하지 않습니다.**
- 대상 홈(`default` 및 참여 프로필)에 대해서만 신규 데몬 기동과 `daemon update` 진입을 막고, 이미 실행 중인 턴/작업이 자연스럽게 완료된 뒤 `codex app-server daemon stop`으로 정리해 대상 홈의 데몬 프로세스가 모두 비활성 상태인 정비 창을 확보합니다.

### 2.3 단계별 이관 순서
1. **사전 백업:** 각 대상 홈의 `packages/app-server-daemon`, `app-server-daemon/settings.json`, `selection-history` 메타데이터를 기존 cutover 백업 정책에 준하는 타임스탬프 백업 루트(`0700`)에 보존하거나 이동 대상을 분리 보관합니다.
2. **공용 루트 및 유지보수 홈 초기화:**
   - `$HOME/.local/share/codex/daemon-packages` (`0700`)와 `$HOME/.local/state/codex/daemon-maintenance` (`0700`, `packages/app-server-daemon -> $HOME/.local/share/codex/daemon-packages`)를 생성합니다.
   - **주의:** 기존 프로필 홈에서 복사한 절대 경로 `current` 심볼릭 링크나 오래된 `selection-history`를 그대로 신뢰하지 않습니다. 업스트림 `app-server daemon update --from-cli`는 `current` 선택이 전혀 없는 빈 저장소에서는 거절되므로(`run codex app-server daemon start first`), 유지보수 홈(`daemon-maintenance`) 자체는 로그인·데몬 기동을 하지 않은 채 **사전에 참여 홈 한 곳(예: `default`)에서 정식으로 초기화·정지했거나 검증된 최초 선택**을 기반으로 `CODEX_HOME="$HOME/.local/state/codex/daemon-maintenance"` (`CODEX_SHARED_HOME`/`CODEX_SQLITE_HOME` unset)에서 `codex app-server daemon update --from-cli --yes`를 실행해 현재 `standalone` 세대와 일치하는 정식 패키지 선택·검증을 완료합니다. (필요 시 직전 안정 세대도 동일 절차를 통해 선행 스테이징해 롤백 대상을 확보합니다.)
3. **계정 홈 심볼릭 링크 전환:**
   - 각 대상 홈의 기존 `packages/app-server-daemon`을 백업 경로로 이동한 뒤(중간 실패 시 즉시 원복 가능하도록 삭제 대신 이동), `packages/app-server-daemon -> $HOME/.local/share/codex/daemon-packages` 심볼릭 링크를 생성합니다.
4. **사후 분리·실행 검증:**
   - `default`, `janmori101`, `szob` 순으로 `app-server daemon start` → `version` → `stop`을 수행해 각 계정의 PID 파일, 제어 소켓(`app-server-control.sock`), `settings.json`이 서로 독립돼 있고 실행 바이너리 inode가 공용 실경로를 가리키는지 확인합니다.
   - 별도로 `codex doctor --json`의 `state.paths` 소요 시간을 이관 전/후로 비교 측정합니다(예상: `~/.codex` 내부 `packages/app-server-daemon` 이동분만큼 디렉터리 스캔 시간 감소).

### 2.4 중단·재개·롤백 및 용량 회수 구분
- **중단 및 재개:** 1~3단계는 홈 단위로 멱등하게 기록하며, 어느 홈에서 중단돼도 미전환 홈은 기존 로컬 `packages/app-server-daemon` 상태로 유지되고 전환 완료 홈은 공용 링크 상태를 유지합니다.
- **롤백:** 문제 발견 시 모든 대상 데몬을 정지한 뒤 각 홈의 `packages/app-server-daemon` 심볼릭 링크를 제거하고 백업 이동해 둔 원본 디렉터리를 제자리로 복원합니다.
- **회수 용량 구분:**
  - **복구 사본(백업) 유지 중 실제 회수량:** 백업을 동일 파일시스템에 보관하는 동안에는 **디스크 순회수량은 0 B** (논리 경로만 단일화).
  - **복구 사본 최종 정리 후 예상 절감량:** 3개 홈 각각 `~0.92 GiB`(`0.169.1`+`0.170.0` 기준 총 `~2.75 GiB`)에서 공용 저장소 1벌(`~0.92 GiB`)만 남아 **약 `1.83 GiB` 순절감** (단, 유지보수 홈 전환 검증과 3세대 롤백 확인 완료 후 사본을 정리).

---

## 3. (대체됨) 구 공용 저장소 3세대 정리(Pruning) 및 롤백 검증안 (참고용)

공용 저장소(`$HOME/.local/share/codex/daemon-packages`)는 별칭 경로로 연결되므로 업스트림 `0.170.0` 프루너(`release_pruning.rs`)가 자동 정리를 건너뜁니다. 반면 유지보수 홈에서 비별칭 경로로 프루닝을 단순 호출하면 타 프로필에서 실행 중인 구세대를 인지하지 못할 위험이 있으므로 아래 조건을 만족하는 별도 정리/검증 절차가 필요합니다. (`crates/wrapper/src/maintenance.rs`는 `standalone` 전용이므로 참조 잠금·`/proc` 검사 패턴만 참고합니다.)

### 3.1 보존 집합 (Keep Set)
공용 `releases/` 삭제 전 아래 중 하나라도 해당하는 세대는 무조건 보존합니다:
1. 공용 `current` 및 `selection-history`가 가리키는 현재·직전 세대
2. 이관 전 개별 프로필이 마지막으로 선택했던 세대와 롤백 대상
3. **모든 참여 홈(`default` 및 등록 프로필 전체)의 `daemon.pid`와 `/proc/<pid>/exe`·`/proc/<pid>/fd`에서 실행 중이거나 열린 파일로 참조되는 세대** (데몬 본체뿐 아니라 `codex-exec-server`, `codex-code-mode-host`, `codex-resources/` 등 도우미 포함)

### 3.2 잠금·경합 및 최소 3세대 임시 검증 시나리오
- 정리기는 공용 루트의 `install.lock` 및 기동·업데이트와 경합을 고려해야 하며, 프로세스 참조 또는 배타 잠금 여부를 확인할 수 없으면 **삭제를 보류(fail-closed)**합니다.
- **필수 임시 검증안 (일회용 `HOME`):**
  1. 공용 저장소에 1세대(V1)를 설치하고 프로필 B의 데몬을 V1으로 기동해 장시간 작업/열린 핸들을 유지합니다.
  2. B가 V1으로 계속 작업하는 동안, 유지보수 홈에서 V2를 거쳐 **V3까지 두 번 업데이트**합니다(최소 3세대 누적).
  3. 3세대 정리 수행 후에도 **B의 V1 실행 파일과 도우미(`codex-exec-server`, `codex-code-mode-host`)가 보존**되어 B의 작업이 정상 완료되는지 확인합니다.
  4. 공용 저장소에서 **직전 세대(V2)로 롤백** 후 신규 기동 프로필 A가 선택한 V2로 정상 기동·버전 보고하는지 확인합니다.
  5. 정리·업데이트 중간 지점 강제 중단(SIGKILL 등) 시 `current`·실행 중 세대가 훼손되지 않고 재개 또는 보류되는지 검증합니다.

---

## 4. 세션 인계(`/switch`) 및 `session_index.jsonl` 후속 검증안

이 두 과제는 `contrib/codex-account` 구현과 독립적으로 진행합니다.
Stage 1의 FD 34는 스레드 수명을 업스트림 정책에 맡깁니다. 마지막 클라이언트가
연결을 닫은 뒤에도 실제 writer 해제와 업스트림 재개 조건을 확인한 다음 인계합니다.

### 4.1 동일 UUID 세션 인계(`/switch`) 검증 기준
- **합격 조건:**
  1. 계정 A가 스레드 X의 턴을 완료하고, 단순 `thread/unsubscribe` ACK가 아니라 **`thread-writer-locks/`의 실제 writer 해제가 완료된 뒤** 계정 B가 동일 UUID X를 재개합니다.
  2. B가 보낸 다음 추론 요청에 A의 직전 응답 문맥이 포함되고 B의 턴이 완료됩니다.
  3. 같은 시간 동안 A의 다른 스레드 Y 작업이 중단 없이 지속·완료됩니다.
- **실계정 온라인 시험 계획:** 별도 승인된 새 테스트 스레드에서만 A 기준 턴 → writer 해제 → B 후속 턴을 검증합니다.
- **계정 일치 검사 정책:** `0.170.0`의 `cli_resume_account_check` 등 업스트림 계정 일치 검사가 켜진 환경에서 타 계정 재개가 거절되면, **계정 태그나 정책을 변조해 억지로 통과시키지 말고** 해당 버전/설정의 호환성 결과로 그대로 기록합니다.

### 4.2 `session_index.jsonl` 권위 원천 및 삭제/이름변경 검증안
- **분리 측정 항목:** A/B 동시 기동 상태에서 이름 변경(`rename`) 및 삭제(`delete`) 수행 후 아래 4개 경로의 **UUID 조회·신규 이름 조회·구 이름(alias) 조회**를 각각 측정하고 실제 권위 원천(`state_5.sqlite` vs `session_index.jsonl`)을 기록합니다:
  1. `app-server` `thread/list`
  2. `app-server` `thread/read`
  3. TUI 목록 및 `/resume`
  4. `codex exec resume <NAME|UUID>`
- **제약:** 각 조회의 권위 원천과 동시 쓰기 정합성이 입증되기 전에는 `state_5.sqlite`나 파일 타임스탬프를 역산해 `session_index.jsonl`을 임의 재생성하는 임시 우회를 적용하지 않습니다.
