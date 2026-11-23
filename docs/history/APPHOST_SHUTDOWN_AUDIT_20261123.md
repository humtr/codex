# Android Termux ChatGPT AppHost 종료·고아·좀비 독립 검증 기록 (2026-11-23)

- **문서 성격:** 비규범적(Non-normative) 독립 감사·설계 검증 기록 (`docs/history/*`). 규범 계약은 [docs/spec/CHATGPT_PROVIDER.md](../spec/CHATGPT_PROVIDER.md), 현재 구현은 [docs/ARCHITECTURE.md](../ARCHITECTURE.md), 라이브 증빙은 [CURRENT.md](../../CURRENT.md)를 참조.
- **조사 기준선:** `main` @ `4c6dae31`, Android 14 / Linux `6.1.162` (비루트 Termux UID), `chatgpt-provider 0.1.0+7b`, ChatGPT Desktop `26.1117.41037`, Termux PRoot `5.1.107.102`, `runit 2.3.1`.
- **최종 판정 (2차 `host-supervisor` 수정안):** **조건부 채택** — 비루트 Android Termux(PID namespace·위임 cgroup 불가, 순정 PRoot `--sysvipc`의 `subreaper` 교착 확인)에서 `Provider SIGKILL`·`PRoot SIGKILL`·동일 프로필 즉시 재시작을 모두 만족하는 최소 경계는 `chatgpt-provider` 내부 단일 스레드 `host-supervisor`(`PR_SET_CHILD_SUBREAPER` + `<profile>.lock` 소유)가 맞다. 다만 현 수정안의 `SIGQUIT` 의존 구간, FD 상속, `SingletonLock` 무조건 삭제, Provider 종료 경로 결함을 반드시 보완한 뒤 Android 수락 실험을 통과해야 운영 적용이 가능하다.
- **후속 조치** [현재 증빙](../../CURRENT.md#2026-11-23--slice-5-slice-7-and-product-completion-audit)에 구현 후보의 Android 수락 결과를 기록했다. 아래 설계와 검증 경계는 감사 당시 기준이다.

---

## 1. 1차 검증 요약: 기존 4개 판단과 PRoot 내부 Subreaper 반증

| 기존 판단 | 판정 | 핵심 근거 |
| --- | --- | --- |
| **판단 1.** 기존 종료 코드(`host.rs`, `manager.py`)의 불완전성 | **부분 지지** | [host.rs](../../providers/chatgpt/src/host.rs) `ChildGroup::stop`은 직접 자식 `wait()`만 대기하며 PRoot 선행 사망 시 `SIGTERM` 유예·`SIGKILL` 에스컬레이션 없이 즉시 반환하고, [manager.py](../../host/chatgpt-desktop/manager.py) `finally`는 `p.poll() is not None`이면 `killpg`를 건너뛴다. `PR_SET_PDEATHSIG(SIGKILL)`은 `fork()` 시 초기화되어 PRoot 단일 PID만 사살한다. 단, 정상 종료 시 외부 수동 subreaper 하네스에 관측된 좀비 10~12개는 `ptrace_reparented` 과도기 좀비(운영에서는 PID 1이 즉시 회수)이므로 최초 사고의 생존 고아 zygote 하위 영구 좀비 24개와 동일시할 수 없다. |
| **판단 2.** PRoot `PTRACE_O_EXITKILL` + `PR_SET_CHILD_SUBREAPER` 결합 | **부분 지지** (`EXITKILL` 유효 / `Subreaper` 결합 **반박**) | `PTRACE_O_EXITKILL`은 PRoot가 종료되면 추적 중인 게스트를 커널에서 종료한다. 기존 PRoot 종료 후 추적이 끊긴 zygote의 CPU 급증을 관측했다. 스핀을 일으킨 구체적 syscall은 확인하지 않았다. 반면 PRoot 자체에 `PR_SET_CHILD_SUBREAPER`를 설정하면 `--sysvipc`의 `proot --shm-helper`(비추적 double-fork, `sysvipc_shm.c:153`)가 PRoot에 재부모화되어 **PRoot(`do_wait`) ↔ `--shm-helper`(`pipe_read`) ABBA 교착**이 발생하고, `event.c:428,505`가 `last_exit_status`를 오염시킨다. |
| **판단 3.** PRoot 업스트림 선택형 기능 단독 채택 (감독기 불필요) | **반박** | PRoot는 외부 프로세스(`Xvnc`, `openbox`, `wmctrl`)·비추적 `--shm-helper`·`<profile>.lock` `flock` 수명을 제어할 수 없으며, 내부 `Subreaper`는 `--sysvipc`와 교착한다. |
| **판단 4.** 운영 적용 전 검증 불완전성 | **지지** | `flock` 조기 해제 및 동일 프로필 중복 기동, `--eager-host` 시그널 미등록, `axum` 드레인 지연, 유휴 중 `host_running` 거짓 고정, `wmctrl` `kill_on_drop` 부재 등 다수의 운영 수명주기 결함이 코드에서 직접 확인됐다. |

> **별도 테스트 서버 5개(`c1672` fixture 4개 + `slice2-proto-*` 1개):** 테스트 assertion 실패 시 정리 코드가 실행되지 않는 RAII/Drop 부재 문제로, ChatGPT AppHost / PRoot 종료 결함과 메커니즘이 완전히 분리된다.

---

## 2. 2차 검증: `host-supervisor` 수정안의 심각도순 발견 사항

### [Critical 1] `SIGQUIT` 계약의 4가지 틈새 (핸들러 설치 전 `SIG_DFL` · 비-async-signal-safe 핸들러 · 미등록 `fork` 누락 · `--shm-helper` `accept()` 영구 교착)

- **소스 위치:**
  - Termux PRoot `v5.1.107.102`: [`src/cli/cli.c:486,505`](https://github.com/termux/proot/blob/v5.1.107.102/src/cli/cli.c), [`src/tracee/event.c:109-178,325-371,412-458`](https://github.com/termux/proot/blob/v5.1.107.102/src/tracee/event.c), [`src/tracee/tracee.c:699-750,1043-1049`](https://github.com/termux/proot/blob/v5.1.107.102/src/tracee/tracee.c), [`src/extension/sysvipc/sysvipc_shm.c:188-196,859-914`](https://github.com/termux/proot/blob/v5.1.107.102/src/extension/sysvipc/sysvipc_shm.c)
- **실제 실패 경로:**
  1. **핸들러 설치 전 `SIGQUIT`:** `cli.c:486`이 첫 tracee를 `fork()`한 뒤에야 `event_loop()`(`event.c:333,368`)가 `sigaction(SIGQUIT, kill_all_tracees2)`를 등록한다. 그전 도착한 `SIGQUIT`은 `SIG_DFL`로 PRoot만 즉시 코어 종료시켜 tracee가 비추적 탈출한다.
  2. **ptrace/리스트 변경 중 `SIGQUIT`:** `kill_all_tracees2()`는 시그널 핸들러 안에서 비-async-signal-safe 함수(`note()`/`fprintf`)와 전역 `&tracees` 연결 리스트 순회(`LIST_FOREACH`)를 수행한다. `event_loop()`의 리스트 변경 중 진입하면 충돌·데드락 위험이 있으며, 아직 `&tracees`에 미등록된 `fork`/`clone` 자식(`pending_child`)은 사살에서 누락된다. `if (signum != SIGQUIT) _exit(...)` 때문에 PRoot는 종료하지 않고 `event_loop()`로 복귀해 계속 실행될 수 있다.
  3. **`--shm-helper` `accept()` 교착:** `SHMHELPER_DISTRIBUTE`(`sysvipc_shm.c:885`)는 타임아웃과 stdin `poll()` 없이 블로킹 `accept()`를 호출한다. `SIGQUIT`이 `connect()` 직전 게스트를 사살하면 PRoot 종료로 stdin 파이프가 닫혀도 `--shm-helper`가 `accept()`에서 영구 잔류한다.
  4. **34ms 최소 게스트 실험의 한계:** 유휴 `waitpid`, 극소 tracee, `read(0)` 대기 중 `--shm-helper`라는 최적 조건만 검증됐으며, 16+ 프로세스가 활발히 `clone`/`shmat`를 호출하는 실제 Electron에 일반화할 수 없다.
- **가장 작은 수정:**
  - `SIGQUIT`은 유한 타임아웃(500ms~1s)의 1차 빠른 경로로만 사용하고, 초과 시 반드시 감독기의 subreaper 전수 스윕(게스트·`--shm-helper`·`Crashpad` 우선 `pidfd_send_signal(SIGKILL)` → `proot` 최후 `SIGKILL` → `ECHILD`)으로 넘어간다.
- **검증 실험:**
  - PRoot 기동 10ms·50ms 시점(`sigaction` 설치 전후) 중지 요청 및 `SHMHELPER_DISTRIBUTE` 직후 중지 요청에서 `--shm-helper` 포함 전원 `ECHILD` 회수 확인.

---

### [Critical 2] `SingletonLock` 무조건 삭제와 `host-supervisor` `SIGKILL` 시 동일 Chromium 프로필 이중 오픈

- **소스 위치:**
  - [providers/chatgpt/src/host.rs](../../providers/chatgpt/src/host.rs) (454–485행 `flock` 및 `SingletonLock` 삭제)
  - [providers/chatgpt/src/driver.rs](../../providers/chatgpt/src/driver.rs) (332–362행 `restart_host`)
- **실제 실패 경로:**
  - 라이브 기기 직접 확인 결과 `SingletonLock`은 `localhost-29176`이며, `29176`은 실제 살아 있는 호스트 PID(`/usr/lib/chatgpt/ChatGPT`)와 일치한다.
  - `host-supervisor` 자체가 `SIGKILL`되면 커널이 `<profile>.lock`을 즉시 해제하지만 Electron(`29176`)은 PID 1 아래 생존한다. 새 `host-supervisor`가 `flock` 획득 직후 `SingletonLock`을 무조건 삭제하면 구세대와 신세대 Electron이 동일 `chromium/` 프로필을 동시에 열어 손상시킨다.
  - 또한 `Provider SIGKILL` 직후 `runit`이 즉시 재기동할 때 구 감독기가 1~3초 정리 중이면 비블로킹 `LOCK_NB` 단독은 즉시 `host_profile_busy` 오류를 낸다.
- **가장 작은 수정:**
  1. `SingletonLock` 삭제는 `flock` 획득 **후** `readlink(SingletonLock)`의 `<pid>`가 동일 UID의 살아 있는 `ChatGPT`/해당 프로필 프로세스가 아님을 확인(`ESRCH` 등)했을 때만 허용하고, 살아 있으면 `host_profile_busy_stale_electron`으로 fail-closed한다.
  2. 감독기 시작 시 `flock(LOCK_EX | LOCK_NB)`는 최대 5~6초(100ms 간격) 유한 재시도로 직렬화한다.
- **검증 실험:**
  - 격리 프로필에서 `host-supervisor`만 `SIGKILL`해 구 Electron이 살아 있는 상태에서 새 감독기 기동 → `SingletonLock` 미삭제 및 `host_profile_busy_stale_electron` 반환 확인.

---

### [High 3] FD 상속 누수 (`ctrl` 채널 · `<profile>.lock` · CDP 소켓 FD 3/4)

- **소스 위치:**
  - [providers/chatgpt/src/host.rs](../../providers/chatgpt/src/host.rs) (603–695행 `dup2(source_fd, 3/4)`)
  - [`src/extension/sysvipc/sysvipc_shm.c:142-156`](https://github.com/termux/proot/blob/v5.1.107.102/src/extension/sysvipc/sysvipc_shm.c) (추가 FD를 닫지 않고 `execl`)
- **실제 실패 경로:**
  - 라이브 `/proc/29170/fd`(`proot`)와 `/proc/29483/fd`(`proot --shm-helper`) 모두 동일한 CDP 소켓(`socket:[31925892]`)을 FD 3·4로 보유함을 직접 확인했다.
  - `ctrl_provider`에 `SOCK_CLOEXEC`이 빠지면 Provider 사망 후 감독기 EOF가 막히고, `ctrl_supervisor`(`dup2`로 `FD_CLOEXEC` 해제됨)에 감독기 진입 즉시 `FD_CLOEXEC`을 복구하지 않으면 자식들이 `ctrl_supervisor`를 상속해 감독기 사망 후 Provider EOF가 막힌다. `<profile>.lock`에 `O_CLOEXEC`이 빠지면 자식이 잠금을 영구 점유한다.
- **가장 작은 수정:**
  - Provider는 `SOCK_CLOEXEC`으로 `ctrl` 쌍 생성, 감독기 `pre_exec`에만 `ctrl_supervisor`/`cdp_child`를 `dup2`하고 spawn 직후 부모 측 `drop`.
  - 감독기는 `main()` 첫 줄에서 `ctrl_supervisor`/`cdp_child`/`<profile>.lock` 전부에 `F_SETFD(FD_CLOEXEC)` 재설정. `proot` `pre_exec`에만 `cdp_child`를 FD 3/4로 `dup2`하고 spawn 직후 감독기 측 `cdp_child` `drop`.
  - AppHost 사망 감지는 CDP EOF가 아닌 `ctrl_provider` EOF 및 감독기의 `proot` exit 통지로 단일화한다.
- **검증 실험:**
  - 기동 후 `/proc/<proot,shm-helper,xvnc>/fd` 전수 검사로 `ctrl` 소켓 inode와 `profile.lock` 누수 0건 단언.

---

### [High 4] Provider 종료·상태 경로 및 `manager.py` 결함

- **소스 위치:**
  - [providers/chatgpt/src/main.rs](../../providers/chatgpt/src/main.rs) (211–230행 `--eager-host` 시그널 미등록, `axum::serve` 무제한 드레인)
  - [providers/chatgpt/src/driver.rs](../../providers/chatgpt/src/driver.rs) (50, 128–137, 171–180, 205–243행 유휴 크래시 후 `host_running` 미갱신)
  - [providers/chatgpt/src/host.rs](../../providers/chatgpt/src/host.rs) (38–42행 `PR_SET_PDEATHSIG`, 749–770행 `wmctrl` `kill_on_drop` 부재)
  - [host/chatgpt-desktop/manager.py](../../host/chatgpt-desktop/manager.py) (776행 `if p.poll() is None:`)
- **실제 실패 경로:**
  - 열린 SSE/WebSocket 중 `sv restart`(`runit` 7초 대기) 시 `axum` 드레인에 막혀 `driver.shutdown().await` 전에 `SIGKILL`에 도달할 수 있다.
  - 유휴 중 AppHost가 죽으면 `CdpClient` 종료가 `CdpDriver`에 전파되지 않아 `/healthz`가 `host_running: true`를 계속 반환한다.
  - `wmctrl`이 감독기 바깥에 남아 `ECHILD` 보장에서 빠진다.
- **가장 작은 수정:**
  - `PR_SET_PDEATHSIG(SIGKILL)` 제거(제어 파이프 EOF로 대체), `wmctrl` 감독기 내부 이동, `open_android_url` 10초 타임아웃 추가, `main.rs`에서 `--eager-host` 전 시그널 등록 및 `axum` 3~4초 유한 드레인 후 `driver.shutdown().await` 보장, `driver.rs`에 `ctrl_provider` EOF 감시 태스크 추가, `manager.py probe`에 자체 `PR_SET_CHILD_SUBREAPER` + `ECHILD` 회수 적용.
- **검증 실험:**
  - 열린 SSE 중 `Provider SIGTERM` → 5초 내 정리 완료; 유휴 중 `PRoot SIGKILL` → 2초 내 `/healthz` `host_running == false` 자동 수렴.

---

### [Medium 5] Fallback 후손 스윕의 Android `/proc` 제약 및 TOCTOU·재fork 대응

- **실제 확인 결과:** 본 기기(Linux `6.1.162`)에서 `/proc/self/task/*/children`은 **부재(`CONFIG_PROC_CHILDREN` 비활성)**, 반면 `pidfd_open(434)` 및 `pidfd_send_signal(424)`는 **정상 동작(`errno=0`)**한다.
- **가장 작은 수정:**
  - `/proc/[0-9]*/stat`으로 `my_pid` 루트 후손 `(pid, ppid, start_ticks)` 트리를 구성 → `proot` 생존 시 비-`proot` 후손(`--shm-helper`, `Crashpad`, Electron)부터 `pidfd_open` → `(ppid_in_tree, start_ticks)` 재검증 → `pidfd_send_signal(SIGKILL)` → 이후 `proot`·`openbox`·`Xvnc`·`wmctrl` 사살 → `waitpid(-1, &st, WNOHANG | __WALL)`이 `ECHILD`를 반환할 때까지 30ms 간격 반복.
  - **충분 조건:** 첫 `fork()` 전 `PR_SET_CHILD_SUBREAPER(1)` 설정 + AppHost 전 프로세스가 감독기 후손 + `waitpid(-1, ..., __WALL)`의 `-1 / ECHILD` 반환.

---

## 3. 가장 단순한 최종안 (책임·대기·잠금 해제 시점)

1. **기동 주체:**
   - `chatgpt-provider`: `chatgpt-provider host-supervisor`만 기동(기존 `runit` 서비스·순정 PRoot 유지). `AuthBrowserRelay`(`termux-am-socket`, 10초 타임아웃)만 Provider가 유지.
   - `host-supervisor` (단일 스레드, `PR_SET_CHILD_SUBREAPER(1)`, `PDEATHSIG` 미사용): `FD_CLOEXEC` 복구 → `<profile>.lock` `flock` 획득(최대 6초 유한 재시도) → `SingletonLock` 대상 PID 사망 확인 후 정리 → `Xvnc`, `openbox`, `proot`(FD 3/4 `cdp_child` 전달 후 즉시 `drop`), `wmctrl` 기동 → 제어 소켓으로 1줄 준비 JSON 송신.
2. **단일 종료 계약 (Provider → 감독기 EOF = 종료):**
   - 정상 종료 및 `restart_host()`: Provider가 CDP `Browser.close`(최대 3초) 호출 → `ctrl_fd` 닫기(EOF) → `supervisor_child.wait()` 완료 대기 → 이후에야 새 세대 시작.
   - `Provider SIGKILL` 또는 `proot` 선행 사망: 감독기가 `poll()`로 `ctrl_fd` EOF 또는 `SIGCHLD` 즉시 감지 → 정상 종료와 **동일한 단일 정리 루프** 실행(`proot` PGID `SIGTERM` 최대 1.5s → `proot` PID `SIGQUIT` 최대 0.8s → `pidfd_send_signal(SIGKILL)` 비-`proot` 후손 우선 사살 후 `proot`/X11 사살 → `waitpid(-1, ..., __WALL) == -1 && errno == ECHILD`까지 반복).
3. **프로필 잠금(`flock`) 해제 시점:**
   - 반드시 **`ECHILD` 확인 → 검증된 `(dev, ino)` X11 `.X<N>-lock`/`X<N>` 및 `cgp-*` 삭제 → 맨 마지막에 `<profile>.lock` close 및 `_exit(0)`** 순서를 준수한다.
4. **별도 선택형 업스트림 개선:**
   - Termux PRoot 업스트림에는 `PR_SET_CHILD_SUBREAPER`를 제외하고 **순수 선택형 `PTRACE_O_EXITKILL`만** 별도 제안한다(감독기 자체 `SIGKILL` 극단 케이스의 2차 방어).

---

## 4. Android 수락 실험 매트릭스 (계정 없는 격리 임시 프로필 전용)

외부 관측용 `subreaper` 하네스가 `waitpid` 호출 전에 스냅샷한 **일시적 좀비(`Z_transient`)**와, 감독기 종료 및 하네스 `waitpid` 회수 후에도 기기에 남아 있는 **잔류 좀비(`Z_residual`)**를 반드시 분리 기록한다.

| 시나리오 | 절차 | 합격 기준 |
| --- | --- | --- |
| **1. 정상 종료** | CDP 준비 + `--shm-helper` 생성 확인 + 열린 SSE 1개 상태에서 `Provider SIGTERM` | ≤ 5s 종료, `t_end+1s`/`+5s` 생존 PID(PRoot/Electron/Crashpad/D-Bus/Xvnc/openbox/`wmctrl`/`--shm-helper`) **0개**, `Z_residual == 0`, X11/`cgp-*` 0개 |
| **2. 부팅 중단** | PRoot 기동 10~50ms(`sigaction(SIGQUIT)` 전후), D-Bus 직후, `Browser.getVersion` 전 각 시점에 `ctrl_fd` 닫기 및 `Provider SIGKILL` | 매 단계 감독기 `ECHILD` 수렴, 생존 PID **0개**, `Z_residual == 0` |
| **3. `PRoot SIGKILL`** | CDP 준비 + `--shm-helper` 확인 후 `PRoot` PID만 `SIGKILL` | ≤ 2s 내 입양 후손 전원 사살·회수, `Z_residual == 0`, 유휴 Provider `/healthz` `host_running == false` 자동 전환 |
| **4. `Provider SIGKILL`** | CDP 준비 + `--shm-helper` 확인 후 `Provider` PID만 `SIGKILL` | 감독기 `ctrl_fd` EOF 감지 → ≤ 5s 내 전체 후손 사살·`ECHILD`·자원 정리 후 마지막에 `flock` 해제, 생존 PID **0개**, `Z_residual == 0` |
| **5. 동일 프로필 즉시 재시작** | ① 정상 `restart_host()`, ② `PRoot SIGKILL` 50ms 후, ③ `Provider SIGKILL` 50ms 후, ④ `host-supervisor SIGKILL` 직후(구 Electron 생존) 새 기동 | ①~③: `t_last_dead <= t_unlock`, 2세대 `Browser.getVersion` 성공, 활성 Electron ≤ 1개.<br>④: `SingletonLock` 미삭제 및 `host_profile_busy_stale_electron` fail-closed |
| **6. FD 비누수 및 운영 보존** | `/proc/<proot,shm-helper,xvnc>/fd` 전수 검사; 실험 전후 운영 `runit` 6개 서비스·라이브 `chatgpt-provider`·Codex 데몬 `(PID, start_ticks)` 대조 | `ctrl`/`profile.lock` 누수 **0건**, 운영 프로세스 `(PID, start_ticks)` **100% 불변** |

---

## 5. 근거 출처 및 검증 경계

| 구분 | 내용 |
| --- | --- |
| **제공된 실험 기록 의존** | 최초 사고 고아 zygote 8개·D-Bus 4개·좀비 24개, 임시 `PTRACE_O_EXITKILL` 빌드 결과, 최소 SHM 게스트 PRoot `subreaper` 교착(`do_wait`/`pipe_read`) 및 종료 코드 `42`, 순정 PRoot 최소 게스트 `SIGQUIT` 34ms 결과 |
| **직접 확인한 코드** | [host.rs](../../providers/chatgpt/src/host.rs), [main.rs](../../providers/chatgpt/src/main.rs), [driver.rs](../../providers/chatgpt/src/driver.rs), [cdp.rs](../../providers/chatgpt/src/cdp.rs), [server.rs](../../providers/chatgpt/src/server.rs), [manager.py](../../host/chatgpt-desktop/manager.py) (`4c6dae31`); Termux PRoot `v5.1.107.102` `cli.c`, `event.c`, `tracee.c`, `ptrace.c`, `sysvipc_shm.c`, Termux `packages/proot/build.sh` |
| **독립 재현 · 읽기 전용 라이브 관측** | Android 14 / Linux `6.1.162`: `CLONE_NEWPID`=`EPERM`, `CLONE_NEWUSER`=`EINVAL`, cgroup 쓰기 불가, `PR_SET_CHILD_SUBREAPER(1)`=`OK`, `/proc/self/task/*/children` 부재, `pidfd_open(434)`/`pidfd_send_signal(424)`=`OK`; 라이브 `chatgpt-provider`(PID `28994`) 트리에서 `browser_crashpad_handler`(별도 SID/PGID, `ppid=1, tracer=29170`), `proot --shm-helper`(`pid=29483, ppid=1, tracer=0`, FD 3/4에 `proot`와 동일 CDP 소켓 `socket:[31925892]` 보유), `SingletonLock -> localhost-29176` 직접 확인 |
| **추론 및 미검증 경계** | `host-supervisor` 구현체 실기기 수락 실험, 실제 Electron 고부하 중 `SIGQUIT` async-signal-safety 충돌 빈도, `SHMHELPER_DISTRIBUTE` `accept()` 경합 실측, 다기종 Android LMK 동작은 미실험 상태이며 §4 수락 실험 통과 전 운영 배포를 승인하지 않는다 |
