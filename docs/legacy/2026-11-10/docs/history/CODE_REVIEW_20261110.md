# LEGACY SNAPSHOT — 2026-11-10; NOT CURRENT AUTHORITY

Original location: docs/history/CODE_REVIEW_20261110.md; source revision: 49cfab806cb99d47a6de0d918ed13777db8e11a5.
This file is preserved evidence; its embedded present-tense language refers only to that revision.
Current rules: see AGENTS.md and docs/DOCUMENTATION.md at repository root.

---

# 첨부 리뷰 74건 재판정 — 통합 9b5ddc9

**비규범적 검토 기록.** 이 문서는 명시된 9b5ddc9 소스 스냅샷의 재판정이다.
현재 제품 요구사항은 SPEC.md와 그 계약, 최신 실행·배포 상태는 CURRENT.md가
담당한다. 이후 수정이나 새 시험 결과를 이 과거 검토에 소급 적용하지 않는다.

## 범위와 읽는 방법

- 검토 기준: `9b5ddc9302b58dade64bd67c84e4cbe591186e18`의 소스. 이후 루트의 테스트 인자 수정은 이 표에 소급 포함하지 않았다.
- 입력: `upload/REPORT.md` 167행과 `upload/insights.md` 524행 전체. 원 리뷰의 대상은 publication `53176ed7` 및 제품 `681ecce`였으므로 현재 코드·계약·통합 이력과 다시 대조했다.
- U1의 공개 main 상태는 새 원격 조회가 아니라 로컬 `origin/main=6f61f8eb29fc5d4adc92d64c6699d71606f9d7f9` 스냅샷으로만 확인했다. 통합 브랜치의 생성기 수정이 공개 main에 이미 배포됐다는 뜻은 아니다.
- 이번 작업은 제품에 대해 읽기 전용이다. 새 시험·기기 실행·서비스 변경·제품 파일 수정은 하지 않았다. 아래 시험은 실제 존재하는 회귀와 이전 실행 증거를 구분해 기재했다.
- **반영**은 타당한 하위 주장이 실제 소스 수정으로 반영됐다는 뜻이다. 한 항목 안의 잘못된 처방까지 수용했다는 뜻은 아니다. **부분 반영**은 별도의 유효한 잔여가 있다. **이미 충족**은 기존 경계가 주장한 문제를 막거나 제안이 그 경계를 훼손한다. **근거 불충분**은 패턴은 있어도 주장한 오동작이 증명되지 않았다. **미해결**은 구체적인 잔여 또는 명시된 기능 제한이 남아 있다.

## 먼저 전달할 판단

원 리뷰의 Critical 6건은 타당한 부분에 대해 모두 통합 커밋과 회귀를 확인했다. 그러나 **74건 전부 해결**, **오탐 0**, **High 23건 전부 재현**이라고 보고할 수 없다. FD36 상속 제거, 파일 상태만으로 writer 부재 판정, zombie leader만 보고 전체 프로세스 종료 선언, Provider에 정적 ELF 강제 등은 현재 계약과 맞지 않는다.

항목별 주 판정은 **반영 26 / 부분 반영 3 / 이미 충족 7 / 근거 불충분 14 / 미해결 24 = 74**다. 미해결 24에는 낮은 우선순위 중복·진단·문서·publication main 미반영도 포함되며, 새로운 심각한 운영 장애 24건이라는 뜻이 아니다. 부분 반영 3건은 U1 H-5, U2 M-01, U2 L-04다.

유효한 잔여 중 우선순위가 높은 것은 publication CAS 뒤 동일 불변 릴리스의 재진입, already-pinned 자동 게시 복구, 시작 업데이트 안내의 Ctrl-Z 이후 snooze 처리, Manager의 ESC/긴 입력 처리다. 실제 운영 P0가 새로 재현된 것은 아니며, 이 표에서는 위험·기능 제한과 실기 장애를 구분한다.

Core 활성화의 stable 바이너리 교체→pointer commit 사이 중단 자체는 DELIVERY가 허용한다. 그 뒤 정상 재시도와 rollback snapshot 보존이 깨지던 실제 결함은 수정됐다. 반대로 rollback guard를 무조건 지우자는 처방은 보존 계약을 깨므로 거절한다.

## U1 — publication / 17건

| ID | 재판정 | 현재 코드·계약 근거와 반영 상태 |
|---|---|---|
| U1 C-1 | 반영 | `scripts/pin_release.py::workflow`의 Full 조건은 both일 때 Base success 필수, 명시적 full-only만 Base skipped 허용. `a5063a4`. `test_release_workflow.test_full_lane_runs_only_after_success_or_an_explicit_full_only_selection`이 조건 행렬을 검증한다. 원 제안처럼 모든 skipped를 허용하지 않았다. 공개 main caller 교체 완료와는 별개다. |
| U1 C-2 | 반영 | `install-online.sh`는 `core-v*`/`local-base-*`의 Base·Full 구분, `release_attention.py::generation`은 새 이름을 처리한다. `a5063a4`, 온라인 lane/서명 ID 회귀. `release_control.py::release_lane`에는 이미 인증된 Base와 production fingerprint를 쓰는 새 경로가 있어 중복 수정하지 않았다. 남은 legacy 정규식은 보수적인 both fallback이며 신규 Base 설치 차단 경로가 아니다. |
| U1 H-1 | 근거 불충분 | v1 index의 필드 수만으로 교차 replay·downgrade 수락을 입증하지 못한다. 현재 index 서명→manifest/descriptor→variant와 sequence/floor 검증이 실제 권위다(`release_heads.py`, Core signed admission). 실제 lane 불일치는 C-2에서 수정했다. 배포된 signed v1을 임의로 확장하는 처방은 거절한다. |
| U1 H-2 | 미해결 | `.github/workflows/auto-release-termux.yml::stage_release`는 기존 release를 재사용하지만 불변 tag SHA를 새 `expected_main`과 다시 비교한다. CAS 전에 main이 바뀌고 같은 generation을 재시도하면 이 경계에서 거절될 수 있다. 실제 운영 재현·수정은 이번 범위에 없다. 이미 게시된 tag 삭제나 실패 카운터로 새 이름을 만드는 처방은 채택하지 않았다. |
| U1 H-3 | 미해결 | `release_control.py::run`은 previous==product/동일 binding/명시 upstream 요청 없음이면 `already-pinned`로 조기 반환한다. pin commit 뒤 dispatch가 실행되지 못한 경우의 자동 복구는 아직 별도 검증·수정되지 않았다. 다만 명시 `requested_upstream`에 의한 rebuild/dispatch 경로는 이미 있으므로 ‘재실행 경로 전무’는 과장이다. |
| U1 H-4 | 이미 충족 | `same_product_inputs`/`no-product-change` 및 `accepted-release-inputs.yml`의 docs-only discovery는 제품 입력 불변이면 불필요한 publication을 하지 않는 계약이다. Source acceptance 실패나 문서 변경을 무조건 publish로 우회시키는 제안은 수용하지 않는다. |
| U1 H-5 | 부분 반영 | caller 최상위 concurrency는 `a5063a4`로 추가됐다. 기존 `caller_compatible`은 실제 immutable caller pin과 네 protocol 입력의 Git blob identity를 비교한다. 반면 caller 내부의 더 넓은 바인딩 선행 검증을 모두 완료한 것은 아니다. 단순히 source SHA와 caller SHA를 항상 같게 하는 것은 검증된 입력 재사용 계약과 구분해야 한다. |
| U1 M-1 | 미해결 | `accepted-release-inputs.yml`은 top-level contents/actions/issues write와 단일 advance job을 유지한다. 권한·job 분리 보강은 미구현이다. 모든 cargo 자식에 쓰기 토큰이 명시 주입된다는 주장은 현재 step별 env와 같지 않으며, 실제 credential 유출은 확인하지 않았다. |
| U1 M-2 | 미해결 | `release_attention.py::reconcile`은 최근 실행 중 최신 완료 하나만 확인한다. `accepted-release-inputs.yml`의 completed_run 조회도 `set -e` 아래 transient API 오류면 중단된다. 여러 완료 신호의 보존·재조정 범위는 보강되지 않았다. 제품 inference 재시도와는 별개다. |
| U1 M-3 | 미해결 | caller 및 accepted-source 일부 run 조회는 `per_page=100`에 제한되고 pagination이 없다. 같은 SHA의 실행이 충분히 쌓이면 과거 성공 proof를 못 찾는 보수적 false block 가능성이 남는다. 실패 run을 성공으로 수락하는 문제는 아니다. |
| U1 M-4 | 미해결 | 로컬 main 스냅샷 `6f61f8e`에는 `rald7-full-acceptance.yml`이 있고 `source-acceptance.yml` registration이 없다. 통합 source에는 실제 Source Acceptance가 존재한다. 공개 main 등록 정리를 수행했다고 보고할 수 없다. 새 원격 상태는 이번 읽기에서 조회하지 않았다. |
| U1 M-5 | 미해결 | 통합 README는 private tempfile→실행이지만 다운로드한 installer 자체 SHA 확인은 없다. 로컬 main README에는 아직 고정 commit URL의 curl→sh 안내가 남는다. HTTPS/시간·크기 bound 및 installer 내부 signed payload 검증은 있으므로 ‘배포 payload도 무검증’은 사실이 아니다. |
| U1 M-6 | 근거 불충분 | `caller_compatible`의 네 파일 목록은 그대로다. 누락된 파일 이름만으로 실제 다른 publication protocol이 승인되는 사례를 확인하지 못했다. 더 넓은 protocol 입력 검토는 남았지만 임의 파일 추가를 보안 수정 완료로 집계하지 않는다. |
| U1 L-1 | 반영 | `a5063a4`: generated caller의 죽은 schedule 조건을 제거하고 `upstream_version`이 표시용이며 accepted source가 권위임을 명시했다. 입력을 새 버전 선택 권한으로 바꾸지 않았다. |
| U1 L-2 | 미해결 | 통합 README에는 six-hour 문구가 없지만 로컬 main `6f61f8e` README에는 남아 있다. 실제 accepted workflow cron은 2시간이다. public main 문서 수정 완료로 표시하지 않는다. |
| U1 L-3 | 미해결 | generated caller의 frontend binding 선행 검사는 top-level shape/source 위주다. 상세 frontend 구조·자격은 downstream 정식 verifier가 거절한다. 조기 진단 개선은 아직 없으며 invalid artifact가 최종 admission을 통과한다는 증거는 없다. |
| U1 L-4 | 미해결 | source publication curl의 per-call 크기 bound가 균일하지 않고 `--proto-redir` 명시는 없다. 다만 이미 `--proto '=https'`가 있어 옵션 문자열 부재만으로 redirect 취약성을 단정할 수 없다. 다운로드 bound 일관성 보강은 미완료다. |

## U2 — Core / 20건

| ID | 재판정 | 현재 코드·계약 근거와 반영 상태 |
|---|---|---|
| U2 C-01 | 반영 | `main.rs::sanitize_doctor_terminal_controls`가 UTF-8 문자 경계를 지키고 redaction은 원래 문자열의 범위로 수행한다. `c4604db`, 형식 후속 `81049ca`. 잘못된 escape/한글/원본 범위/토큰 회귀가 있다. |
| U2 C-02 | 반영 | `4bfe684`의 실제 root-command 파싱과 `03d51e6`의 upstream approval aliases로 실제 workspace-write/daemon 우회를 막는다. exact 086 upstream SharedCliOptions fixture와 non-UTF8/옵션 위치 회귀가 있다. 현 upstream에 없는 `--full-auto`, TOML key escape가 실제 key를 바꾼다는 미입증 처방까지 수용하지 않았다. |
| U2 H-01 | 이미 충족 | `execution_lease.rs` FD36 상속은 Native/shared-server 및 살아 있는 자손이 profile inode lease를 보유하는 USER_INTERFACES 계약이다. `lease_tracks_inode_and_descendants_without_registry_policy`, descriptor-pressure 회귀가 있다. 백그라운드라는 이유로 FD36을 닫으면 수명 보호를 깨므로 ‘영구 deadlock’ 결론과 close 처방을 거절한다. |
| U2 H-02 | 반영 | `94a3b91`/`fdad6a8`/`356bf3e`: 허용된 Core 교체→pointer 사이 SIGKILL 뒤 동일 candidate 재시도, 원래 rollback cache 보존, 정상 legacy rollback 후 새 build를 구분한다. 실제 SIGKILL/잘못된 snapshot matrix/정상 post-rollback 회귀가 있다. DELIVERY가 허용한 중간 상태 자체를 새 journal로 바꾸지 않았다. hold 기록 실패 때 guard를 무조건 삭제하는 처방도 거절한다. |
| U2 H-03 | 반영 | `67f9c27`, delimiter fixture `f004bf0`: `JsonCursor::string`이 올바른 surrogate pair를 합성하고 lone/reversed pair를 거절한다. release notes JSON 회귀가 있다. |
| U2 H-04 | 반영 | `35d6c87`: `run_doctor_child`를 probe/capture가 공유하며 stdin null, 시간·출력 bound, nonblocking drain, 실패 시 process-group 종료와 reap을 보장한다. stdin/status, deadline/reap, byte-cap 회귀가 있다. |
| U2 H-05 | 근거 불충분 | `shared_server::retire_family`는 `/proc/PID/exe`와 `bound_connection`의 kernel SO_PEERCRED PID/UID·runtime identity를 검증한 뒤 재확인하고 signal을 보낸다. u32→i32 cast만으로 kill(-1) 도달은 입증되지 않았다. 접근 거절/다른 exe를 무조건 죽은 소유자로 간주하는 정리도 안전하지 않다. stale entry 보존과 실제 잘못된 signal은 구분해야 한다. |
| U2 H-06 | 반영 | `4bfe684`: `shared_server::eligible` 및 `launch_qualified_runtime`이 profile/config/global option의 명시 설정을 보존하고 기본 owner fallback을 피한다. 실제 routing fixture와 non-UTF8/attached option 회귀가 있다. |
| U2 M-01 | 부분 반영 | manual updater는 `0ab4002`/`8193608`로 동일 receipt·deadline을 보존해 SIGTSTP/CONT 뒤 복귀하고 inherited SIG_IGN을 보존한다(실제 PTY 회귀). 그러나 `main.rs::startup_update_prompt`의 VSUSP→raise→기본 Keep 반환과 호출자의 snooze 기록은 아직 남는다. 시작 안내까지 전체 해결했다고 집계하면 안 된다. |
| U2 M-02 | 반영 | `c4604db`: original-range redaction, key 경계, 단·쌍따옴표, GitHub token prefix를 처리한다. 기존 정상 문장 오마스킹/비밀 일부 노출 회귀가 있다. 원래 값은 보고서에 노출하지 않는다. |
| U2 M-03 | 미해결 | `provider.rs::run`의 Core activation lock이 원격 acquisition·probe·commit까지 유지되는 범위는 남아 있다. 반면 root 부재 시 file=None guard가 공개 쓰기를 우회한다는 부분은 미입증이다. Provider는 Core pointer 부재를 거절하고 activation 준비는 root를 먼저 만든다. |
| U2 M-04 | 미해결 | `timezone.rs`의 parent create→canonicalize는 원래 경로의 alias를 잃고, `local_source.rs`의 sibling Cargo/target 및 env override는 일부 절대경로 검사만 한다. 명시 ownership/type 검증 공백은 남는다. 공유 cache를 무조건 버리거나 매 버전 복제하는 것은 필요한 해결책으로 입증되지 않았다. |
| U2 M-05 | 미해결 | `shared_server::ensure`의 profile logical-link 갱신은 서로 다른 namespace writer의 공통 logical-dir lock이 없다. 별도 동시성 재현·수정은 안 했다. 그러나 PID temp는 namespace lock 아래이고 immutable_record는 counter가 있으며, with_extension은 temp 이름만 바꿔 final 파일 확장자를 지우지 않는다. |
| U2 M-06 | 근거 불충분 | `main.rs`의 helper_count replacen/File::create 패턴은 남지만 대상은 unique private unsigned staging이다. 이후 common publish 및 local admission이 descriptor/inventory/signatures를 검증한다. 운영 descriptor를 비원자적으로 덮거나 malformed descriptor를 활성화한다는 주장은 과장이다. 선행 오류 진단 개선은 별도 후보다. |
| U2 M-07 | 근거 불충분 | 매 실행 prune/installed signed-file 재검증 비용은 존재하지만 실제 지연은 측정되지 않았다. 두 reference scan은 다른 root를 다룬다. 과거 admission/inode만으로 현재 file 검증을 대체하거나 추가 전역 타이머 상태를 만드는 제안은 동등한 보존 근거 없이 수용하지 않는다. |
| U2 L-01 | 반영 | `d1278c4`: `UpdatePresentation`의 mojibake spinner를 실제 `⠙`로 교정했다. |
| U2 L-02 | 반영 | `c4604db`: lowercase view를 원본에 대해 한 번 계산하고 범위를 모아 마스킹한다. 반복 치환마다 재계산하는 구조가 사라졌다. |
| U2 L-03 | 반영 | `67f9c27`: `release_display::fields`가 빈 object를 올바르게 파싱한다. 필요한 필드 부재는 상위 decode에서 별도로 판단한다. |
| U2 L-04 | 부분 반영 | `8193608`에서 manual updater는 libc 신호 선언을 사용한다. startup 안내의 수동 ioctl 등 중복은 남는다. 모든 Android/musl ABI에서 해당 선언이 실제 오동작한다는 증거는 없으므로 임의 c_int 일괄 변경은 하지 않았다. |
| U2 L-05 | 미해결 | `valid_sha256_hex`/`is_canonical_sha256`, rollback forwarding wrapper, local source의 반복 metadata 조회는 낮은 우선순위 정리로 남는다. 현재 장애나 hashing 일관성 결함을 입증하는 근거는 아니다. |

## U3 — TUI / transport / 13건

| ID | 재판정 | 현재 코드·계약 근거와 반영 상태 |
|---|---|---|
| U3 H-1 | 근거 불충분 | `frontend/tui/src/chatwidget/input_preparation.rs::requeue_input_submission`은 원래 UserMessage와 history/source를 보존한다. exact upstream의 `input_submission`은 prepared images/prepare/local-images를 서로 배타적인 분기로 처리한다. 재준비 비용은 가능하지만 이미지 중복·붙여넣기 유실은 현 소스로 입증되지 않았다. 원래 text/images/elements/mentions 보존 회귀도 존재한다. |
| U3 H-2 | 반영 | `aea6188`: `with_progress_folded`가 question_titles/copy source/셀 메타데이터를 보존한다. exact upstream의 provider_presentation 시험 2/2 실제 PASS(`codex-upstream-tui-final-proof/tui-provider_presentation.log`). |
| U3 H-3 | 근거 불충분 | `termux_chat_models::synchronize_input`에는 총 waiting deadline/외부 retry 값 clamp가 없다. 그러나 각 자식은 60초 bound와 kill_on_drop을 갖고 input task는 취소 가능하며 Provider는 현재 고정 reconcile interval을 반환한다. 접수된 Native 작업을 기다리는 상태를 임의 타임아웃으로 바꾸는 것은 연속성 계약과 다르다. 영구 UI deadlock은 증명되지 않았다. |
| U3 H-4 | 반영 | `25e60b7`: 선택적 Manager identity 등록을 worker로 이동하고 foreground slot 갱신은 즉시 유지한다. termux_terminal actual upstream 시험 3/3 PASS(`tui-termux_terminal.log`). |
| U3 M-5 | 이미 충족 | `termux_clipboard::copy`의 동기 wait는 exact upstream `clipboard_copy/worker.rs`가 생성하는 clipboard-copy 전용 스레드에서 실행된다. 원 리뷰의 ‘TUI 이벤트 스레드에서 8초 차단’ 호출 경로는 성립하지 않는다. 새 구현을 추가하지 않았다. |
| U3 M-6 | 반영 | `ea4bd5c` + `d45a833`: patch와 owned TUI payload를 공통 Rust producer의 단일 private Git index transaction으로 합친다. `af45685`는 merged-tree inventory에 새 파일도 포함한다. 충돌 시 원래 checkout/index 불변 회귀와 전체 source assembly가 실제 통과했다. |
| U3 M-7 | 근거 불충분 | `quota_switch::observe`의 JSON projection 비용은 남지만 측정은 없다. uncertain/pending은 새 turnStarted에서 초기화되고 불확실한 tool effect 때만 자동 재진입을 막고 사용자에게 알린다. failed event를 무조건 settled로 바꾸면 effect 소유권을 잃는다. ‘비표준 이벤트로 영구 lock’은 입증되지 않았다. |
| U3 M-8 | 근거 불충분 | `profile_transport`가 상세 stderr를 숨기는 것은 맞다. 그러나 edit/pause는 일반 오류를 표시하고 refresh completion은 generation guard를 거친다. 실제 Manager mutation에도 lock이 있다. 현재 코드만으로 out-of-order IPC가 잘못된 설정을 승인한다는 race는 확인하지 못했다. |
| U3 M-9 | 반영 | `bf44d83`은 실패를 사용자에게 보이면서 원래 thread 상태를 보존한다. `e983f0e`는 동일 terminal history 재확인, `12c3bef`는 actual initial route와 회귀를 맞춘다. 오류를 삼키거나 새 대화를 만들어 성공 처리하지 않았다. |
| U3 L-10 | 근거 불충분 | `termux.patch`의 `wait - wait/2`는 양의 duration이면 양수다. 정확한 upstream `PollTimeoutOrDefault()`에서 0이 도달한다는 증거가 첨부에도 없으며 이번 읽기에서도 확보하지 않았다. arbitrary ping 최솟값을 추가한 것으로 해결 처리하지 않는다. |
| U3 L-11 | 이미 충족 | `termux_backend_catalog::reenter_owner`는 pre-config 실행 home/owner 선택이고 `bind`는 최종 config의 현재 owner를 다시 검증한다. 별도 경계의 재검증을 하나로 cache하면 오히려 stale binding을 만든다. 실제 잘못된 owner 연결 재현은 없다. |
| U3 L-12 | 근거 불충분 | `check_command_boundary.py`의 regex 기반 inventory 제한은 남고 지원하지 않는 strum 문법은 fail closed 한다. 현재 qualified 65-command 경계를 우회한 사례는 없다. `65cca2f`는 /provider 추가를 검증했을 뿐 일반 Rust parser를 구현한 커밋이 아니다. |
| U3 L-13 | 미해결 | `termux_chat_models::augment`의 CODEX_PROFILE_CORE 조건은 `termux_permissions::enabled`와 일부 중복된다. 낮은 우선순위 정리로 남는다. 구체화되지 않은 모든 clone/dead-code 주장을 수용한 것은 아니다. |

## U4 — Manager / 10건

| ID | 재판정 | 현재 코드·계약 근거와 반영 상태 |
|---|---|---|
| U4 H-1 | 미해결 | `launch.rs::choose`는 ESC 뒤 poll의 EINTR/revents를 정확히 구분하지 않고 두 바이트만 읽는다. 긴 CSI/알 수 없는 입력의 나머지가 남는 경로가 있다. 현재 통합에 Manager 소스 수정은 없으며 입력 잔여/실제 PTY 회귀도 새로 수행하지 않았다. |
| U4 H-2 | 미해결 | `quota.rs::refresh`는 다른 refresh의 lock 해제 또는 20초 경과 후 요청 IDs의 실제 갱신을 확인하지 않고 Ok를 반환한다. 진단 정확성 잔여다. 다만 소비자는 observed/status를 기준으로 fresh/stale/unavailable을 구분하므로 오래된 cache를 무조건 fresh로 인정한다는 주장은 성립하지 않는다. |
| U4 H-3 | 미해결 | `launch.rs`/`window.rs::start`는 exec 전 전역 env/cwd를 바꾼다. CLI exec 성공 경로에서는 의도된 전환이지만 lib::run 반환/실패 시 호출 프로세스 환경 오염 가능성이 남는다. 실제 다중 스레드 손상은 이번 읽기만으로 재현하지 않았다. |
| U4 M-4 | 미해결 | `launch.rs::choose`는 winsize를 한 번만 읽고 label 폭을 제한하지 않으며 selected 기반 viewport를 다시 계산한다. 긴 label/wrap/resize의 표시 결함은 미수정이다. |
| U4 M-5 | 미해결 | `window::launch`는 route ACK의 중복 존재 확인과 timeout 뒤 ‘이미 consumed’ 상태를 구분하지 않아 실행됨/호출 실패가 엇갈릴 수 있다. 다만 `start`가 rename한 consumed 파일과 원래 launch 파일은 다르므로 timeout cleanup이 consumed 요청 자체를 삭제한다는 부분은 틀리다. |
| U4 M-6 | 미해결 | `quota::refresh`는 publish 실패를 `let _`로 버리고 Ok를 반환한다. cache 상태 판단이 따로 있어 false-fresh는 미입증이지만 갱신 실패 진단이 사라지는 부분은 유효하다. |
| U4 M-7 | 미해결 | `task.rs`의 한 바이트 read는 EINTR 재시도와 127byte 초과 줄의 drain이 없다. 반면 profile_resume의 BufRead 계층은 interrupted read를 처리하고 ‘그 외 입력 취소’는 현재 prompt 의미다. 모든 공백을 선택값으로 해석하는 처방까지 채택하지 않는다. |
| U4 M-8 | 미해결 | `shared_profiles`는 suffix filtering보다 먼저 take(64), terminal/window는 전체 entry bound를 먼저 적용한다. 무관한 항목이 많으면 유효 항목이 관측되지 않을 수 있다. 원래 DoS bound를 무조건 제거하는 해결책은 적절하지 않다. |
| U4 L-9 | 근거 불충분 | getuid/geteuid 혼용은 존재하지만 지원되는 Termux app UID 모델에서 둘이 달라지는 실행 경로는 제시되지 않았다. 일반 setuid 프로그램의 문제를 이 제품의 재현된 결함으로 집계하지 않는다. |
| U4 L-10 | 미해결 | manual extern 선언, notify status mapping 중복, `let _ = id`가 남는다. 낮은 우선순위 정리이며 ABI 오작동이 실제 확인됐다는 뜻은 아니다. |

## U5 — Release Builder / Runtime binding / 14건

| ID | 재판정 | 현재 코드·계약 근거와 반영 상태 |
|---|---|---|
| U5 C-1 | 반영 | `a4ec9e5`: `validate_publish_file_mode`가 group/world write 및 위험 mode를 거절하고 `set_mode`는 O_NOFOLLOW 파일 descriptor로 fchmod한다. `publication_permissions_reject_shared_writes_and_never_follow_aliases` 회귀. canonicalize만 하고 path chmod하는 우회는 넣지 않았다. |
| U5 C-2 | 반영 | `a4ec9e5`: Provider canonical input/closed inventory와 Android PIE·정확한 `/system/bin/linker64`, raw runtime의 real-dir/inventory/mode/static ELF를 검증한다. `provider::producer_admits_android_pie_and_rejects_malformed_or_unsafe_inputs`, `runtime::raw_transfer_inventory_identity_and_alias_faults_are_rejected`. Provider를 PT_INTERP 없는 static으로 만들자는 처방은 계약이 달라 거절했다. |
| U5 H-3 | 이미 충족 | `runtime-binding::native::writer_free`는 Native와 같은 coordination/writer OS lock으로 부재를 증명한다. 짧은 경쟁의 conservative busy는 있을 수 있으나 fstat/무잠금으로 바꾸면 실제 writer 존재를 알 수 없다. persistent deadlock 재현은 없으며 그 처방은 거절한다. |
| U5 H-4 | 근거 불충분 | `Server::stream`에 EINPROGRESS/EALREADY/EISCONN 분기는 없지만 대상은 AF_UNIX이며 현재 EAGAIN/EINTR를 처리한다. 해당 OS/UDS에서 원 리뷰의 pending 실패를 재현하지 않았다. 이후 discovery 소켓 장애 수정과 이 connect errno 주장은 같은 원인으로 단정하지 않는다. |
| U5 H-5 | 반영 | `ea4bd5c`: `source::patch::communicate`가 scoped stdin writer와 동시에 stdout/stderr를 drain한다. `source::patch::tests::large_input_and_output_are_pumped_together`는 실제 pipe backpressure를 검증한다. |
| U5 H-6 | 반영 | `a4ec9e5`: `snapshot_core_artifact`/`snapshot_manager_artifact`가 bounded copy 후 copied==opened metadata.len을 검사한다. 다른 archive/SDK 경로는 이미 pinned digest로 검증하므로 모든 input이 무검증이었다는 주장은 과장이다. 동일 크기의 임의 외부 write까지 크기 비교 하나로 방지한다고 주장하지 않는다. |
| U5 H-7 | 반영 | `7d14042`: 실패한 build/prepare가 자신이 생성한 output 및 소유한 build-dir만 정리한다. preexisting/unknown output이나 원래 실패 증거를 무작정 지우는 stale auto-cleanup은 추가하지 않았다. 명시 오류 경계의 회귀가 있다. |
| U5 M-8 | 이미 충족 | `parse_pax_payload`는 의도적으로 단일 mtime만 지원하고 path/size 등 다른 의미를 거절한다. 일반 tar 전체 지원을 주장하지 않는다. unknown key를 무조건 무시하라는 처방은 closed archive admission을 약화하므로 거절한다. |
| U5 M-9 | 반영 | `7d14042`: `finish_staging`/source patch/Provider cleanup은 원래 오류를 보존하고 추가 cleanup 실패를 함께 진단한다. 성공 후 cleanup 실패도 조용히 성공으로 바꾸지 않는다. |
| U5 M-10 | 근거 불충분 | `source::syntax::test`의 normalized token cfg(test) 및 특정 import pattern 제한은 남는다. exact qualified upstream에서 잘못된 product rewrite를 승인한 사례는 확인하지 않았다. 문법 확장·유지보수 후보를 현재 장애로 단정하지 않는다. |
| U5 M-11 | 반영 | `7d14042`: `native_dependencies::literal`은 trim_start 후 exact literal/단일 값/버전·digest를 검증한다. `dependency_recipe_comes_from_literal_upstream_values_and_rejects_changed_contracts` 회귀. 임의 shell expansion을 해석하지 않는다. |
| U5 M-12 | 이미 충족 | `process_exited`의 Z/X + num_threads==1은 leader zombie만으로 살아 있는 다른 스레드를 종료로 오인하지 않기 위한 조건이다. `/proc` 부재는 별도로 종료다. threads 검사를 지우자는 처방은 전체 process 종료 증거를 약화하므로 거절한다. |
| U5 L-13 | 반영 | `f6dff75`: USAGE는 정식 `adapt-tui`와 역사적 `adapt-manager-tui = adapt-tui`를 명시한다. 동일 bytes/인자 충돌 회귀로 alias를 유지한다. |
| U5 L-14 | 반영 | `a4ec9e5`: Core/Manager Android ELF 검증기를 공통화했다. `let _ = checked_*`는 overflow/range 오류를 이미 검사하는 부분이므로 사용하지 않는 산술 값만 보고 검증을 삭제하지 않았다. |

## 시험 증거의 한계와 재사용 가능한 근거

1. U1/U2/U5 및 공통 source producer의 위 수정들은 기존 통합 이력에 있으며 Source121(`12129a5ed37efda26119b79e1e25350d91a39808`)의 전체 Source Acceptance가 성공했다. Run `42703090391`, attempt 1, 완료 `2026-11-09T21:36:27Z`는 루트가 실제 API로 확인한 값이다. 이후 9b의 새 starting-native 테스트를 그 CI가 검사했다고 주장하지 않는다.
2. 앞선 e0 Source run `42700060784`에는 workspace 442 pass/0 fail/1 explicit Termux smoke ignored, 별도 Android fixture 3개 각각 pass가 있었다. 단순 합산 중복을 피하고 구체 scope를 우선 인용한다. Release Builder 60개 및 Core 관련 회귀도 이 범위에 포함된다.
3. 실제 upstream TUI proof는 `codex-upstream-tui-final-proof/`의 로그와 담당자의 실행 보고를 교차 확인했다. 이 증거는 UI unit/source assembly이며 실제 `/provider → GPT-6` 수락과 같지 않다.
4. 새 Manager PTY, publication CAS 고장 주입, quota write-failure 등 미해결 항목의 시험은 이번 읽기 작업에서 실행하지 않았다. 코드상 확인과 실제 재현을 구분했다.
5. 원 첨부는 ChatGPT Provider 자체를 리뷰하지 않았다. 현재 P0 CDP 준비·Native 결과 POST 손실·MCP ID admission·compiler claim·TUI 실호출의 수정/수락은 이 74건과 별도 증거로 보고해야 한다.

## 후속 우선순위

- 운영 호출 수락이 진행 중인 현재 작업에 임의 publication 재설계나 새 daemon을 추가하지 않는다. 남은 Manager 입력·startup prompt 문제는 작은 독립 기능 수정과 실제 PTY 회귀가 적합하다.
- U1 H-2/H-3는 signed immutable release/CAS/자동 dispatch의 정확한 재진입 정책부터 정하고 명시 고장 주입으로 검증해야 한다. 불변 tag 삭제, 임의 실패 sequence, 무조건 retry는 해결책으로 수용하지 않는다.
- 경로·락·lease 보강은 실제 ownership 근거를 재사용해야 한다. review 문구만 따라 신뢰 경계를 제거하지 않는다.

이 문서는 현재 소스의 재판정 기록이다. 미해결을 숨겨 통합 완료로 표시하거나, 근거 불충분을 모두 버그 없음으로 단정하는 용도가 아니다.
