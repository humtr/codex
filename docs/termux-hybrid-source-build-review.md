# Termux 하이브리드 소스 빌드 구조 개편 검토·심사서

## 문서 통제

| 항목 | 값 |
|---|---|
| 문서 상태 | `PROPOSED / IMPLEMENTATION NOT STARTED` |
| 기준일 | 2026-07-29 |
| 대상 저장소 | `humtr/codex` Termux Codex wrapper |
| 기준 브랜치 | `refactor/support-activation` |
| 기준 wrapper 버전 | `260710-6` |
| 기준 활성 runtime | Codex `0.146.0`, `aarch64-unknown-linux-musl` |
| 기준 runtime patch policy | `termux-fd-remap-v1` |
| 결정문 | 현행 공식 바이너리 패치 방식 유지 + A(정확한 upstream 소스와 patch queue) + B(작은 외부 clipboard helper adapter) + musl 소스 빌드 |
| 구현 승인 | 이 문서 자체는 구현·설치·활성화·커밋·푸시를 승인하지 않는다 |

이 문서는 다음 작업자가 이전 대화를 읽지 않아도 구조 개편을 재개할 수
있도록 결정, 현재 증거, 설계 경계, 단계별 게이트, 검증 명령, 실패 및
롤백 조건을 한곳에 고정한다.

문서에서 사용하는 규범 용어는 다음과 같다.

- `MUST`: 통과하지 않으면 다음 단계로 진행할 수 없는 조건
- `SHOULD`: 예외 사유와 대체 증거를 기록해야 생략할 수 있는 조건
- `MAY`: 구현자가 선택할 수 있는 항목
- `NOT PROVEN`: 아직 구현 또는 실기기 증거가 없어 완료로 간주할 수 없는 항목

구현을 재개할 때는 이 문서의 **재개 절차**, **단계별 실행 계획**,
**수용 기준 원장** 순서로 읽는다. 저장소 또는 upstream 상태가 바뀌었으면
이 문서의 기준 스냅샷을 먼저 갱신하고, 가장 앞에 남아 있는 `NOT PROVEN`
항목부터 작업한다.

---

## 1. 최종 구조 판단

채택할 구조는 두 runtime lane을 함께 유지하는 하이브리드 방식이다.

1. `official-patched`
   - 현행 방식이다.
   - 공식 `@openai/codex`의 `aarch64-unknown-linux-musl` 배포물을 받는다.
   - 현재의 고정 길이 경로 치환기와 Termux support overlay를 적용한다.
   - upstream 버전이 나오면 가장 빨리 따라가는 기본 업데이트 및
     cross-lane 복구 경로로 유지한다.

2. `source-musl`
   - 정확한 upstream tag/commit/archive를 입력으로 사용한다.
   - 저장소가 관리하는 작은 patch queue를 빌드 전에 적용한다.
   - `aarch64-unknown-linux-musl`을 1차 target으로 빌드한다.
   - FD/config 경로 호환과 clipboard adapter를 소스 수준에서 컴파일한다.
   - 빌드 성공만으로 활성화하지 않고, 별도 candidate 검증과 명시적
     승격을 거친다.

두 lane은 launcher, manager, immutable store, doctor, registry, rollback
정책을 공유하되, 입력 종류와 patch policy는 섞지 않는다.

```text
                         ┌─ official npm arm64-musl bundle
upstream release ────────┤    └─ fixed-length patch + support overlay
                         │         └─ official-patched candidate
                         │
                         └─ exact source tag/archive
                              └─ verified patch queue
                                   └─ arm64 musl build
                                        └─ helper-inclusive bundle
                                             └─ source-musl candidate

both candidates ──> shared validation ──> immutable store ──> explicit activation
                                              │
                                              └─ pinned known-good per lane
                                                      └─ verified rollback
```

### 1.1 A, B, musl의 고정 의미

- **A — source snapshot + patch queue**
  - GitHub fork가 아니라 exact upstream source snapshot을 입력으로 삼는다.
  - tag, commit, archive SHA-256, source tree digest, `Cargo.lock` digest를
    manifest에 남긴다.
  - patch는 번호가 붙은 독립 파일과 machine-readable series 파일로 관리한다.
  - patch가 정확히 적용되지 않으면 fuzz 또는 임의 재작성 없이 실패한다.

- **B — external clipboard helper adapter**
  - Codex 소스에는 Termux 구현을 넣지 않고, 고정된 argv/stdin/stdout/exit
    protocol을 호출하는 작은 adapter만 넣는다.
  - Android/Termux 세부 구현은 wrapper가 제공하는 helper가 소유한다.
  - helper는 runtime bundle 안에 포함되고 다른 산출물과 함께 hash·검증·설치된다.
  - 일반 Linux/macOS/Windows의 upstream clipboard 동작은 helper opt-in이
    없으면 바뀌지 않는다.

- **musl — 1차 source-build target**
  - 첫 목표는 현행 공식 runtime과 같은 `aarch64-unknown-linux-musl`이다.
  - Android/Bionic native target은 musl feasibility gate가 실패하고 별도
    심사를 통과한 경우에만 대안으로 연다.
  - target 문자열이 같다는 이유만으로 공식 bundle과 동등하다고 간주하지
    않는다. code-mode host, V8, TLS, PTY, locking, signal, TTY, FD 및 exit
    behavior를 모두 검증한다.

---

## 2. 이 결정을 택한 이유

| 요구 | official-patched만 사용 | source-musl만 사용 | 채택한 hybrid |
|---|---:|---:|---:|
| upstream 직후 업데이트 속도 | 가장 빠름 | 빌드·patch 보수 시간 필요 | 공식 lane으로 보장 |
| 긴 `/copy`의 근본 수정 | 안전한 byte patch로는 어려움 | 가능 | source lane에서 해결 |
| 이미지 clipboard 확장 가능성 | 사실상 없음 | adapter로 탐색 가능 | source lane에서 gate |
| 빌드 실패 시 사용 가능성 | 높음 | 낮아질 수 있음 | 공식 known-good 유지 |
| 기존 launcher/repair/rollback 보존 | 이미 충족 | 재검증 필요 | 공통 manager 계약으로 보존 |
| fork 없이 운영 | 가능 | 가능 | 가능 |

핵심은 “업데이트를 빨리 받는 경로”와 “소스 수준 수정이 필요한 기능”을
같은 실패 도메인으로 묶지 않는 것이다. source build가 한 버전에서
실패하더라도 공식 lane은 계속 갱신할 수 있어야 한다.

---

## 3. 공개성 및 upstream 관계 정책

이 저장소는 계속 공개한다. 다만 다음 행위는 자동으로 하지 않는다.

- OpenAI 저장소의 GitHub fork 생성
- upstream remote에 대한 push
- upstream PR, issue, discussion 생성
- DioNanos 저장소의 fork, submodule 또는 자동 동기화
- 구현 사실을 알리는 release, announcement 또는 notification
- GitHub Actions workflow 추가

upstream source archive를 다운로드하는 것은 공개적인 fork 관계를 만들지
않지만, 호스팅 서버와의 통신 자체는 발생한다. “알리지 않는다”는 요구는
공개 fork/PR/issue/announcement 같은 사회적·저장소 연결을 만들지 않는다는
의미로 적용한다.

이 저장소가 공개인 이상, 여기 커밋·푸시한 patch와 문서는 누구나 발견할
수 있다. 비공개성을 보장하는 설계가 아니며, 단지 능동적으로 알리거나
upstream 연결을 만들지 않는다.

---

## 4. 범위와 비범위

### 4.1 포함 범위

- 현행 official-patched lane을 그대로 유지하는 구조
- exact upstream source를 가져오고 검증하는 로컬 build plan
- versioned patch queue와 적용 증거
- Termux에서 실행 가능한 musl build backend
- source-built runtime bundle과 provenance manifest
- text clipboard helper adapter 및 `/copy`, `Ctrl+O` 회귀 수정
- image clipboard 접근 가능성 검증과, 가능할 때만 실제 paste 구현
- lane-aware candidate 저장, doctor, activation, rollback, retention
- on-device 수동/자동 업데이트 경로
- 실제 Termux 기기 검증과 fault injection

### 4.2 명시적 비범위

- upstream Codex 전체 소스를 이 저장소에 vendoring
- build cache, Cargo registry, target tree 또는 source archive를 Git에 추가
- 기존 `codex` 및 `codex termux` command의 breaking change
- 기존 FD 33/34 계약 변경
- 빌드 완료 즉시 자동 활성화
- 앱 시작 때마다 무거운 source build 실행
- 이미지 clipboard가 입증되기 전에 지원된다고 표기
- Termux:API APK 교체 또는 companion APK 배포
- Android/Bionic port를 musl 검증보다 먼저 추진
- DioNanos 구현을 dependency 또는 자동 patch 공급원으로 사용
- 현재 진행 중인 `support-activation` 변경과 한 변경 단위로 혼합

이미지 clipboard가 현재 Termux:API 표면으로 불가능하다고 판정되면,
Android companion/Termux:API 변경은 이 문서의 범위를 넘어서는 별도
제품 심사로 분리한다. 파일 선택기 기반 이미지 가져오기는 clipboard
paste와 다른 UX이므로 별도 기능명으로만 제안할 수 있다.

---

## 5. 2026-07-29 기준 현행 구조와 증거

이 절은 재개 시 반드시 다시 측정해야 하는 스냅샷이다.

| 관찰 대상 | 현재 증거 | 구조적 의미 |
|---|---|---|
| Git branch | `refactor/support-activation` | 새 기능은 별도 slice에서 시작해야 한다 |
| 미커밋 파일 | `M src/wrapper/support_layout.py` | 사용자 작업으로 간주하고 보존한다 |
| wrapper version | `config/wrapper-version.env`: `260710-6` | manifest/tuple에 포함되는 현행 wrapper identity |
| active runtime path | `.../store/runtime/0.146.0-linux-arm64+...` | Codex 0.146.0 musl bundle이 활성 상태 |
| runtime patch | `libexec/build-runtime.py` | fixed-length binary rewrite가 현행 owner |
| patch policy | `termux-fd-remap-v1` | source lane은 같은 이름을 재사용하면 안 된다 |
| changed bytes | `runtime-build.json`: 54 bytes | 네 경로 치환의 합이며 기능 patch 체계가 아니다 |
| overlay | `codex`, `codex-path/bwrap`, `codex-path/rg` | 여러 파일이 이미 하나의 runtime bundle로 관리된다 |
| runtime pointers | `current`, `verified`, `raw` | 활성/rollback/raw 입력 계약 |
| metadata | schema 3 `state.json`, `registry.json` | old/new manager reader compatibility가 필요하다 |
| public launcher | `$PREFIX/bin/codex` | argv, env, FD, signal, TTY, exit code를 보존해야 한다 |
| FD mapping | resolver FD 33, managed config directory FD 34 | 소스 patch에서도 동일 의미를 유지해야 한다 |
| release package | `tools/package-release.sh` allowlist | `docs/`는 배포물에 들어가지 않는 repo-only 자료 |
| full tests | `tests/run-all.sh` | portable + real-Termux 검증 entrypoint |

현행 binary rewrite는 아래 네 소스 문자열을 같은 길이의 Termux 경로로
바꾼다.

| upstream 문자열 | runtime 문자열 |
|---|---|
| `/etc/resolv.conf` | `/proc/self/fd/33` |
| `/etc/codex/config.toml` | `/dev/fd/34/config.toml` |
| `/etc/codex/requirements.toml` | `/dev/fd/34/requirements.toml` |
| `/etc/codex/managed_config.toml` | `/dev/fd/34/managed_config.toml` |

현행 source/build host의 관찰값은 다음과 같다.

- `rustc`/`cargo`: 1.96.1
- host: `aarch64-linux-android`
- 설치된 Rust std target: Android 계열
- musl std/toolchain: 아직 준비되지 않음
- 여유 공간: 약 315 GB

따라서 “Termux에서 musl을 자동 빌드할 수 있다”는 아직 `NOT PROVEN`이다.
공식 musl binary가 Termux에서 실행된다는 사실과, Android host에서 같은
target을 재현 빌드할 수 있다는 사실은 별개의 검증 항목이다.

### 5.1 현행 구조에서 특히 보존할 계약

- bare `codex`와 모든 upstream argv passthrough
- `codex termux` command namespace
- stdout, stderr, upstream exit status
- signal forwarding과 TTY/PTY behavior
- FD 33 resolver, FD 34 managed config inheritance
- no-network normal execution
- schema 3 reader/writer compatibility
- `current`, `verified`, `raw`의 원자적 전환과 복구
- managed path deletion guard
- launcher/runtime/support repair 및 cached raw rebuild
- profile/session 분리
- credential, token, clipboard payload 비노출

---

## 6. `/copy` 및 `Ctrl+O` 긴 답변 장애의 원인 판정

### 6.1 관찰된 경로

Codex 0.146.0의 TUI clipboard 경로는 대체로 다음 순서를 갖는다.

1. native clipboard backend를 시도한다.
2. local graphical clipboard를 열 수 없으면 OSC 52 terminal escape로
   fallback한다.
3. OSC 52는 payload를 Base64로 인코딩해 terminal에 보낸다.

현재 Termux 세션에는 `DISPLAY`/`WAYLAND_DISPLAY`가 없어 native graphical
clipboard backend가 성립하지 않는다. 따라서 fallback이 실제 경로가 된다.

### 6.2 경계 불일치

- Codex OSC 52 쪽 raw payload 허용량: 100,000 bytes
- Termux terminal emulator의 `MAX_OSC_STRING_LENGTH`: 8,192 chars
- OSC prefix와 Base64 4:3 팽창을 고려한 실제 raw 경계: 약 6,138 bytes

긴 답변은 Termux parser 한도를 넘어 OSC sequence가 끝까지 하나의
clipboard 명령으로 처리되지 않는다. 화면에 나타나는 Base64처럼 보이는
문자열은 별도 에러 코드가 아니라, 처리되지 못하고 terminal text로 새어
나온 OSC 52 payload 일부다.

근거가 되는 upstream source 위치:

- OpenAI Codex: `codex-rs/tui/src/clipboard_copy.rs`
- Termux terminal: `terminal-emulator/.../TerminalEmulator.java`
  (`MAX_OSC_STRING_LENGTH = 8192`)

### 6.3 왜 현행 binary patcher만으로 고치지 않는가

현재 patcher는 **이미 존재하는 고정 길이 경로 문자열**을 정확히 한 번
치환하는 도구다. clipboard 수정에는 다음 로직이 필요하다.

- helper opt-in 판정
- child process 생성
- stdin streaming
- timeout 및 exit code 해석
- payload 크기별 OSC 52 금지
- 사용자에게 안전한 실패 메시지

이를 post-link byte rewrite로 넣는 것은 검증 가능하지도, 유지 가능하지도
않다. 따라서 text clipboard의 근본 수정은 source-musl lane의 B adapter가
소유한다. official-patched lane에는 무리한 binary injection을 하지 않는다.

---

## 7. 이미지 붙여넣기 경계 판정

OpenAI Codex TUI가 일반 환경에서 interactive image input을 지원한다는
사실만으로 Termux clipboard image가 지원되는 것은 아니다.

현재 Termux:API `ClipboardAPI`의 실질 표면은 다음과 같다.

- set: plain text clip 생성
- get: 첫 `ClipData.Item`을 `coerceToText()`로 반환

즉 현재 확인된 API만으로는 image MIME, `content://` URI, raw PNG/JPEG
bytes를 안정적으로 꺼낼 수 없다. text helper 구현과 image helper 구현은
같은 난이도가 아니다.

image 지원은 아래 feasibility gate를 먼저 통과해야 한다.

1. target Android/Termux 조합에서 clipboard의 non-text item 존재 여부,
   MIME 및 URI를 payload를 노출하지 않고 식별할 수 있는가.
2. 새로운 APK 없이 해당 URI를 Termux process가 읽을 수 있는가.
3. 일회성 URI permission과 Android clipboard access restriction을
   안정적으로 처리할 수 있는가.
4. PNG/JPEG bytes를 mode `0600` 임시 파일로 materialize할 수 있는가.
5. Codex가 그 파일을 기존 image input과 동일하게 소비하고 정리할 수 있는가.

하나라도 실패하면 `paste-image`는 구현하지 않는다. 그 경우 선택지는
다음 두 개로 분리 심사한다.

- 명시적 file picker import: `termux-storage-get` 등을 이용하되
  “clipboard paste”라고 부르지 않는다.
- companion APK 또는 Termux:API 변경: 별도 저장소·배포·권한·서명·보안
  심사가 필요한 새로운 제품 범위다.

---

## 8. 설계 원칙

### 8.1 Additive, not replacement

source-musl lane은 현행 official-patched lane을 덮어쓰며 시작하지 않는다.
초기 구현은 별도 candidate store와 별도 build command로만 존재해야 한다.

### 8.2 Build is not activation

다운로드, patch, compile, package, smoke, install-inactive, activate를 서로
다른 상태로 기록한다. build 성공은 `current`, `verified`, `raw` symlink를
바꿀 권한이 없다.

### 8.3 One compatibility tuple

승격 단위는 binary 하나가 아니라 다음 전체 tuple이다.

- manager artifact 및 wrapper commit/version
- lane 및 upstream version/tag/commit
- source/archive/tree/patchset digest
- target/API/toolchain/linker
- schema reader/writer compatibility
- patch policy
- Codex binary와 code-mode host
- bwrap, rg, clipboard helper
- runtime/source build manifest
- test set과 결과
- activation 전 state/registry snapshot

### 8.4 Fail closed on drift

- tag가 다른 commit을 가리키면 실패
- archive hash가 다르면 실패
- `Cargo.lock`이 예상과 다르면 실패
- patch context가 정확히 맞지 않으면 실패
- `cargo --locked`가 lockfile 수정을 요구하면 실패
- helper 또는 code-mode host가 빠지면 bundle 생성 실패
- candidate 검증이 하나라도 실패하면 activation 금지

### 8.5 Atomic external tools

`bwrap`, `rg`, `codex-code-mode-host`, clipboard helper는 별도 executable로
남을 수 있다. 구조적으로 중요한 것은 파일 개수가 아니라 하나의 immutable
bundle로 조립·hash·검증·설치·rollback되는가이다.

### 8.6 Upstream default behavior preservation

Codex source patch는 wrapper가 명시적으로 helper mode를 제공할 때만
활성화한다. 그 외 환경에서는 upstream clipboard 경로를 유지한다.

---

## 9. 제안 runtime lane 모델

### 9.1 고정 lane ID

- `official-patched`
- `source-musl`

향후 Android/Bionic을 승인하더라도 `source-musl`의 의미를 바꾸지 않고
`source-android` 같은 새 ID를 사용한다.

### 9.2 candidate lifecycle

```text
planned
  -> fetched
  -> input_verified
  -> patches_checked
  -> patched
  -> built
  -> packaged
  -> smoke_passed
  -> installed_inactive
  -> activated
  -> soak_passed
  -> lane_verified

어느 단계에서든 -> failed (현재/verified pointer 불변)
```

각 전이는 timestamp, 입력/산출물 hash, 실행한 test ID, 실패 class를
machine-readable journal에 남긴다. clipboard 본문, auth token, 환경 전체
dump는 기록하지 않는다.

### 9.3 known-good 보존

최소한 아래 두 산출물을 동시에 prune으로부터 보호해야 한다.

- 가장 최근의 건강한 `official-patched`
- 가장 최근의 건강한 `source-musl` 또는 현재 source candidate

기존 public `current`, `verified`, `raw` 계약은 보존한다. per-lane verified
index를 registry optional extension으로 둘지, 별도 symlink로 둘지는 schema
설계 단계에서 결정한다. 어느 방식을 택해도 다음 불변식은 MUST다.

1. source lane 최초 활성화 전의 official known-good를 잃지 않는다.
2. source runtime이 손상되면 source rebuild가 불가능해도 official known-good로
   복구할 수 있다.
3. 이전 manager가 새 metadata를 읽을 수 없을 때 state snapshot으로 되돌릴
   수 있다.
4. store prune은 current, global verified, per-lane known-good, 실행 중
   runtime을 삭제하지 않는다.

초기 단계에서는 source candidate를 global `verified`로 올리지 않는다.
cross-version/device soak와 fallback 검증이 끝날 때까지 official lane이
global rollback baseline으로 남는다.

---

## 10. A: source acquisition 및 patch queue

### 10.1 입력 취득

기본 입력은 OpenAI Codex의 exact release tag source archive다.

예시 식별자:

```text
version: 0.146.0
tag: rust-v0.146.0
commit: <resolved full commit SHA>
archive_sha256: <downloaded archive SHA-256>
cargo_lock_sha256: <extracted Cargo.lock SHA-256>
```

release tag, commit 및 archive hash를 모두 기록한다. tag만 기록하거나
`main`을 빌드 입력으로 쓰지 않는다. GitHub fork나 persistent upstream
remote는 필요하지 않다.

### 10.2 제안 저장소 배치

```text
config/
  source-build-policy.json
patches/
  codex/
    series.toml
    0001-termux-fd-and-managed-config.patch
    0002-termux-clipboard-helper-adapter.patch
    versions/
      0.146.0.toml
libexec/
  build-source-runtime.py
tools/
  build-source-runtime.py
```

의미:

- `series.toml`: patch 순서, 논리 ID, 필수/선택 여부, 적용 대상 파일
- numbered patch: 사람이 검토 가능한 최소 delta
- `versions/<version>.toml`: tag, commit, archive hash, lock hash, 적용 가능한
  series revision
- `libexec/build-source-runtime.py`: canonical implementation
- `tools/build-source-runtime.py`: 설치된 manager와 repo 양쪽에서 쓰는 얇은
  compatibility facade

정확한 파일명은 첫 contract slice에서 확정한다. 전체 upstream source는
repo 밖 cache에 둔다.

### 10.3 patch 적용 규칙

- clean extracted tree에서 시작한다.
- 각 patch 전에 `git apply --check`와 동등한 strict check를 수행한다.
- offset/fuzz를 자동 허용하지 않는다.
- patch 적용 전후 tree digest를 남긴다.
- patch 파일 자체 및 ordered series의 aggregate digest를 남긴다.
- 생성된 diff에 series 밖 변경이 있으면 실패한다.
- patch refresh는 별도 review commit으로 수행한다.
- upstream version별 conditional code가 patch에 계속 누적되면 patch를
  논리 단위로 재작성하고 old version manifest를 고정한다.

### 10.4 DioNanos/mmmbuto의 위치

`DioNanos/codex-termux`는 2026-07-29 확인 시 공개·활성 상태이며 Android
ARM64/Bionic, PTY/locking, in-process V8/code-mode 등의 선행 사례다.

그러나 이 계획과는 다음이 다르다.

- DioNanos: upstream fork 전체를 Android/Bionic target으로 유지
- 본 계획: fork 없이 exact snapshot + local patch queue + musl target
- DioNanos: 자체 release/npm update channel
- 본 계획: 현행 official-patched lane을 함께 유지

따라서 reference로만 사용한다. musl feasibility 또는 code-mode parity가
막힐 때 patch inventory를 읽을 수는 있지만, 자동 cherry-pick, submodule,
fork relation은 만들지 않는다. 코드를 가져올 경우 Apache-2.0 attribution,
실제 필요성, 최소 delta를 별도 기록한다.

---

## 11. musl build backend 심사

### 11.1 우선순위

1. **M1: native Termux cross-build to musl**
   - Android host에서 musl target std/linker를 갖춘다.
   - 가장 직접적이지만 toolchain bootstrap과 일부 native dependency가
     어려울 수 있다.

2. **M2: Termux가 구동하는 격리된 musl/proot build environment**
   - 사용자는 Termux 명령 하나로 실행하지만 실제 compile root는 격리한다.
   - bootstrap 용량과 시간이 늘지만 host 차이를 줄일 수 있다.

3. **M3: Android/Bionic native build**
   - M1/M2가 객관적으로 유지 불가능할 때만 별도 심사한다.
   - 이 문서의 자동 fallback이 아니다.

M1과 M2 중 먼저 재현성·보수성·build time을 충족하는 backend를 채택한다.
backend가 달라도 출력 manifest와 runtime bundle 계약은 동일해야 한다.

### 11.2 Phase 1 feasibility에 필요한 증거

- exact upstream source가 patch 없이 `cargo --locked`로 해석된다.
- target `aarch64-unknown-linux-musl`용 Codex CLI가 link된다.
- matching `codex-code-mode-host` 및 필요한 V8 artifact가 생성된다.
- 생성 binary의 architecture/interpreter/dynamic dependency가 의도와 맞다.
- target Termux에서 `--version`이 정상 종료한다.
- code-mode host를 포함한 최소 smoke가 동작한다.
- clean cache와 warm cache 양쪽 build가 재현된다.
- build interrupt 후 orphan process/lock/partial candidate가 남지 않는다.
- build 시간, peak storage, peak memory, thermal/battery 영향을 기록한다.

이 단계에서 clipboard patch를 섞지 않는다. 먼저 “그대로 빌드 가능한가”를
입증해야 build failure와 기능 patch failure를 구분할 수 있다.

### 11.3 source-built runtime의 FD/config 처리

source-musl에서는 기존 네 경로를 post-link rewrite하지 않는다. 동일한
runtime 의미가 되도록 source constant/path resolution을 patch하고 컴파일한다.

source-built manifest는 예를 들어 다음처럼 별도 policy를 사용한다.

```text
patch_policy: termux-source-musl-v1
fd_contract:
  resolver: 33
  managed_config: 34
post_link_rewrite: false
```

`termux-fd-remap-v1`을 재사용하거나 `changed_byte_count`를 가짜 값으로
채우면 안 된다.

### 11.4 source-built bundle 최소 구성

```text
codex
codex-code-mode-host
codex-resources/...
codex-path/bwrap
codex-path/rg
codex-path/codex-termux-clipboard
codex-package.json
runtime-build.json
source-build.json
```

정확한 upstream resource tree는 해당 version의 official bundle과 비교해
결정한다. source binary만 official bundle에 덮어쓰는 방식은 code-mode
host/resource version mismatch가 없다는 증거 없이는 금지한다.

---

## 12. B: clipboard helper protocol

### 12.1 경계

Codex source adapter는 Android API나 `termux-*` command 이름을 알지 않는다.
wrapper가 다음 두 환경값을 제공할 때만 helper mode를 사용한다.

```text
CODEX_TERMUX_CLIPBOARD_MODE=helper
CODEX_TERMUX_CLIPBOARD_HELPER=<validated absolute path>
```

helper path는 active immutable runtime bundle 내부로 resolve되어야 한다.
PATH 검색, shell command string, `eval`, user-controlled arbitrary executable
fallback을 사용하지 않는다.

제안 bundle 위치:

```text
<runtime>/codex-path/codex-termux-clipboard
```

### 12.2 `copy-text` protocol

```text
argv:   codex-termux-clipboard copy-text
stdin:  exact UTF-8 bytes, EOF로 종료
stdout: 비어 있어야 함
stderr: payload를 포함하지 않는 짧은 진단
exit 0: Android clipboard에 전체 payload 저장 완료
exit 2: unsupported 또는 dependency 없음
exit 3: permission/access 거부
exit 4: timeout
exit 5: invalid input/size
exit 70: internal failure
```

MVP helper는 stdin을 `termux-clipboard-set`으로 전달할 수 있다. 향후 helper를
native executable로 바꾸더라도 protocol은 유지한다. Android clipboard
bridge 자체가 Termux:API process이므로 helper를 compile한다고 외부 API
dependency가 사라지는 것은 아니다.

### 12.3 fallback 정책

- `helper` mode에서 helper가 성공하면 OSC 52를 절대 출력하지 않는다.
- `helper` mode에서 helper가 실패하면 payload를 terminal에 fallback하지
  않고 명확한 copy failure를 반환한다.
- payload 내용이나 Base64를 stderr에 기록하지 않는다.
- helper mode가 설정되지 않은 환경은 upstream behavior를 유지한다.
- SSH에서 OSC 52로 client clipboard를 쓰려는 사용자를 위해 향후
  `terminal` mode를 둘 수 있다.
- 자동 SSH 판정은 별도 UX 검토 없이 넣지 않는다. `tmux` 존재만으로 remote
  session이라 판단하지 않는다.
- OSC 52를 유지하는 mode에서도 terminal별 안전 한도를 넘는 payload는
  sequence로 보내지 않고 실패해야 한다.

### 12.4 `paste-image` 후보 protocol

image feasibility가 통과한 경우에만 다음 protocol을 확정한다.

```text
argv:   codex-termux-clipboard paste-image
stdin:  empty
stdout: 한 줄의 strict JSON
stderr: payload/URI를 포함하지 않는 진단
```

성공 예시:

```json
{
  "schema": 1,
  "status": "ok",
  "path": "/validated/private/temp/image.png",
  "mime": "image/png",
  "size": 12345,
  "width": 800,
  "height": 600
}
```

필수 안전 조건:

- path는 wrapper가 승인한 private temp root 내부
- regular file, symlink 아님, mode `0600`
- 허용 MIME 및 magic bytes 일치
- byte/pixel/dimension 상한
- URI와 clipboard metadata를 log에 남기지 않음
- Codex가 읽은 뒤 정상/실패/interrupt 모두 정리
- TOCTOU를 막을 수 없으면 path protocol 대신 inherited FD protocol 검토

---

## 13. build 및 provenance manifest

source lane은 현재 `runtime-build.json`을 의미 없이 재사용하지 않는다.
초기에는 다음 두 파일로 책임을 분리한다.

- `source-build.json`: source, patch, toolchain, build, test provenance
- `runtime-build.json`: installed runtime integrity와 wrapper가 요구하는
  runtime contract

제안 `source-build.json` 최소 필드:

```json
{
  "schema": 1,
  "lane": "source-musl",
  "upstream": {
    "version": "<version>",
    "tag": "<exact tag>",
    "commit": "<full commit>",
    "archive_sha256": "<sha256>",
    "tree_sha256_before": "<sha256>",
    "cargo_lock_sha256": "<sha256>"
  },
  "patchset": {
    "policy": "termux-source-musl-v1",
    "series_sha256": "<sha256>",
    "tree_sha256_after": "<sha256>",
    "patches": [
      {
        "id": "<stable id>",
        "sha256": "<sha256>",
        "status": "applied"
      }
    ]
  },
  "toolchain": {
    "backend": "<M1 or M2>",
    "host": "aarch64-linux-android",
    "target": "aarch64-unknown-linux-musl",
    "rustc": "<version -v>",
    "cargo": "<version -v>",
    "linker": "<identity>",
    "flags_sha256": "<sha256>"
  },
  "artifacts": {
    "codex_sha256": "<sha256>",
    "code_mode_host_sha256": "<sha256>",
    "helper_sha256": "<sha256>",
    "bundle_tree_sha256": "<sha256>"
  },
  "tests": {
    "set_sha256": "<sha256>",
    "result": "pass",
    "completed_at": "<RFC3339>"
  }
}
```

추가 요구:

- build command의 secret-bearing 환경은 저장하지 않는다.
- timestamps 등 비결정적 필드를 제외한 reproducibility digest를 따로 둔다.
- 같은 input tuple을 clean build 두 번 했을 때 artifact가 다르면 차이의
  원인을 기록하고, 허용 가능한 비결정성인지 심사한다.
- build log는 source path를 정규화하고 token, clipboard, auth를 redact한다.
- manifest 자신과 builder의 hash를 runtime tuple에 포함한다.

### 13.1 schema 3 통합 금지선

현재 `raw_sha256`는 official raw binary를 뜻하고, cached rebuild도 그 구조를
가정한다. source archive SHA를 같은 필드에 넣어 의미를 바꾸면 안 된다.

다음이 설계되기 전에는 source candidate를 shared active registry에 넣지 않는다.

- `lane`
- `input_kind` (`official_vendor` 또는 `source_archive`)
- source manifest identity
- lane-aware rebuild command
- old manager가 optional field를 보존/무시하는 방식
- old manager로 fallback했을 때 source-active state의 처리
- state/registry snapshot 및 downgrade

schema version을 올릴지 schema 3 optional extension으로 유지할지는
compatibility test 후 결정한다. 단순히 “필드 추가니까 안전하다”고 가정하지
않는다.

---

## 14. update 및 자동화 정책

### 14.1 업데이트 우선순위

기존 명령의 기본 의미는 유지한다.

```text
codex termux update
codex termux install [VERSION]
codex termux install upstream [VERSION]
codex termux install rebuild
```

unqualified `update`는 계속 official-patched lane의 빠른 업데이트를
수행한다. source build를 암묵적으로 붙여 update를 수십 분짜리 명령으로
바꾸지 않는다.

### 14.2 단계별 source command

초기 개발 단계는 repo-local tool로만 노출한다. public CLI는 artifact와
rollback 계약이 검증된 뒤 additive command로 설계한다.

후보 public surface:

```text
codex termux source plan [VERSION]
codex termux source build [VERSION]
codex termux source status [--json]
codex termux source activate <candidate-id>
```

이는 제안이며 Phase 5 CLI 심사에서 최종 확정한다. 기존 command parser와
help output의 호환성을 먼저 측정한다.

### 14.3 자동화 mode

최종적으로 MAY인 mode:

- `manual`: 사용자가 source build를 명시 실행
- `after-official`: official update가 성공한 뒤 source candidate build를
  이어서 요청
- `source-only-explicit`: 지정 version만 build

기본값은 source build가 안정화될 때까지 `manual`이다. 백그라운드 daemon,
앱 시작 시 무조건 build, 무인 자동 activation은 허용하지 않는다.

### 14.4 새 version 처리 순서

1. update lock 획득
2. exact upstream version/tag/commit resolve
3. version policy/hash record 확인
4. official-patched update는 기존 transaction으로 독립 수행 가능
5. source archive download 및 verify
6. patch queue strict check/apply
7. musl compile
8. full bundle assembly
9. portable 및 device candidate smoke
10. immutable store에 inactive install
11. 결과 보고
12. 별도 사용자 승인 후 activate

source 단계가 실패해도 successful official update를 되돌리지 않는다.
반대로 official update가 실패하면 source build가 우연히 성공했다는 이유로
자동 활성화하지 않는다.

---

## 15. 구조 개편 slice 계획

현재 `config/refactor-boundaries.json`은 한 change unit에 한 구조 slice와
focused evidence를 요구한다. 현재 `support-activation` slice의 미커밋
변경과 새 기능을 섞지 않는다.

구현 시작 전:

1. 현재 `support-activation` 작업을 사용자가 원하는 방식으로 완료하거나
   안전한 별도 worktree/branch 상태로 분리한다.
2. clean integrated baseline에서 새 topic branch를 만든다.
3. `config/refactor-boundaries.json`에 하이브리드 계획용 slice를 명시적으로
   추가한다.
4. OpenAI upstream 또는 DioNanos 쪽 PR은 만들지 않는다.
5. 이 저장소의 public PR도 사용자가 명시하지 않으면 자동 생성하지 않는다.

제안 slice:

| 순서 | slice ID | 책임 | 대표 증거 |
|---:|---|---|---|
| 0 | `source-build-contract` | lane, manifest, policy, cache, CLI 비공개 plan | contract/schema tests |
| 1 | `source-musl-feasibility` | exact upstream 무수정 musl build | clean/warm/device build report |
| 2 | `source-runtime-artifact` | FD/config source patch, complete bundle | runtime parity tests |
| 3 | `clipboard-adapter` | B protocol 및 text copy | fake-helper + real clipboard tests |
| 4 | `source-activation` | inactive store, lane-aware registry/rollback | fault injection, old-manager fallback |
| 5 | `source-update-integration` | additive public CLI와 local automation | install/update/doctor tests |
| 6 | `image-clipboard-feasibility` | MIME/URI/raw access proof | target-device evidence |
| 7 | `image-clipboard-adapter` | gate 통과 시에만 image paste | image security/cleanup tests |

각 slice는 implementation과 regression evidence를 같은 변경 단위에 둔다.
`main`에 직접 push하지 않고, force-push하지 않으며, merge/push는 사용자의
명시적 요청을 따른다.

---

## 16. 단계별 실행 계획과 exit gate

### Phase 0 — Baseline freeze 및 contract

작업:

- current branch/dirty state 보존
- wrapper/runtime/state/registry/doctor snapshot
- 새 slice와 allowed paths 정의
- lane, manifest, candidate lifecycle schema 확정
- 새 기능 전 golden behavior capture

Exit gate:

- [ ] 기존 portable test 전체 통과
- [ ] real-Termux 기본 smoke 통과
- [ ] current/verified/raw 및 state/registry snapshot 복구 연습
- [ ] source build가 existing command 의미를 바꾸지 않는다는 contract test
- [ ] security 및 storage budget 승인

### Phase 1 — 무수정 upstream musl feasibility

작업:

- exact tag/archive/hash 고정
- M1 native 또는 M2 isolated backend bootstrap
- patch 없는 upstream CLI와 matching host/resource 빌드
- clean/warm/interrupt build 측정

Exit gate:

- [ ] `cargo --locked` 성공
- [ ] target/ELF/dependency audit 통과
- [ ] `codex --version` target-device 성공
- [ ] code-mode 최소 smoke 성공
- [ ] clean build 재현성 결과 기록
- [ ] interrupt 후 orphan/partial/lock 없음

실패 시:

- source lane 구현을 중단하고 official-patched를 유지한다.
- M1 실패는 M2 검토 사유가 되지만 Android/Bionic 자동 전환 사유는 아니다.

### Phase 2 — source runtime parity

작업:

- FD 33/34 및 managed config source patch
- existing official bundle과 resource inventory 비교
- source/runtime manifest 생성
- inactive candidate store

Exit gate:

- [ ] resolver와 managed config가 FD를 통해 동작
- [ ] argv/env/signal/TTY/exit differential test 통과
- [ ] code-mode host/resource 일치
- [ ] build만으로 pointer/state/registry 불변
- [ ] runtime bundle tree digest 검증

### Phase 3 — text clipboard adapter

작업:

- upstream-default-preserving Codex adapter patch
- wrapper-owned helper와 fixed-path validation
- copy failure UI 및 no-OSC52-long-payload guard

Exit gate:

- [ ] 짧은 ASCII copy
- [ ] 6,138-byte 경계 전후 copy
- [ ] 100 KiB 이상 copy
- [ ] 한국어/emoji/multibyte copy
- [ ] newline/NUL policy 검증
- [ ] helper missing/timeout/permission failure
- [ ] 성공/실패 모두 Base64 terminal leakage 없음
- [ ] clipboard payload가 log/stderr/manifest에 없음

### Phase 4 — activation 및 rollback

작업:

- lane-aware registry/store/repair/doctor
- source candidate 명시적 activation
- per-lane known-good retention
- old-manager/state downgrade 검증

Exit gate:

- [ ] activation 전 snapshot 생성
- [ ] switch 중 모든 fault point에서 pointer/metadata 원자 복구
- [ ] source runtime 손상 시 official known-good 복구
- [ ] source builder/helper 손상 시 official known-good 복구
- [ ] store prune 보호
- [ ] 이전 manager reader/rollback compatibility

### Phase 5 — update integration

작업:

- additive `codex termux source ...` surface 확정
- manual source build
- optional after-official mode
- pending/failed source update journal

Exit gate:

- [ ] unqualified 기존 command 출력/동작 불변
- [ ] patch drift가 structured failure로 기록
- [ ] official update 성공 + source failure 조합 검증
- [ ] source success가 무인 activation을 하지 않음
- [ ] network interruption 및 resume
- [ ] full `tests/run-all.sh`

### Phase 6 — image feasibility

작업:

- clipboard MIME/URI probe
- permission 및 byte materialization proof
- security/cleanup threat review

Exit gate:

- [ ] 실제 PNG/JPEG clipboard item 읽기
- [ ] no-image/revoked-URI/oversized cases
- [ ] private temp/cleanup/TOCTOU 검증
- [ ] Codex image input round trip

gate 실패 시 Phase 7은 생성하지 않는다.

### Phase 7 — image adapter 및 장기 soak

작업:

- 승인된 protocol 구현
- repeated paste, cancellation, crash cleanup
- 여러 upstream version에 patch queue 재적용
- 배터리/열/저장공간 포함 real-device soak

Exit gate:

- [ ] 최소 두 upstream version에서 source update 재현
- [ ] text/image clipboard 장기 반복 smoke
- [ ] clean install/update/rebuild/repair/rollback 전체 통과
- [ ] release package 및 doctor 통합
- [ ] 사용자의 활성/배포 승인

---

## 17. 수용 기준 원장

상태 값은 `NOT PROVEN`, `PASS`, `FAIL`, `DEFERRED`만 사용한다. 구현자가
evidence path 또는 명령 결과를 기록하지 않고 `PASS`로 바꾸면 안 된다.

| ID | 수용 기준 | 초기 상태 | 필수 증거 |
|---|---|---|---|
| HBR-001 | 기존 bare `codex`와 upstream argv가 불변이다 | NOT PROVEN | golden + device |
| HBR-002 | `codex termux` 기존 command가 불변이다 | NOT PROVEN | CLI contract |
| HBR-003 | official-patched update가 source build와 독립해 계속 동작한다 | NOT PROVEN | combined failure matrix |
| HBR-004 | exact source tag/commit/archive/lock hash가 기록된다 | NOT PROVEN | source manifest |
| HBR-005 | GitHub fork/PR/issue/workflow 없이 build된다 | NOT PROVEN | source policy audit |
| HBR-006 | patch drift가 fuzz 없이 fail-closed한다 | NOT PROVEN | negative test |
| HBR-007 | Termux에서 musl clean build가 재현된다 | NOT PROVEN | Phase 1 report |
| HBR-008 | matching code-mode host/resource가 bundle에 있다 | NOT PROVEN | code-mode smoke |
| HBR-009 | FD 33/34 의미가 source runtime에서도 같다 | NOT PROVEN | FD probes |
| HBR-010 | signal/TTY/exit behavior가 upstream과 같다 | NOT PROVEN | differential test |
| HBR-011 | build는 active/verified/raw를 변경하지 않는다 | NOT PROVEN | mutation snapshot |
| HBR-012 | candidate bundle이 atomic하고 hash-complete하다 | NOT PROVEN | manifest/tree audit |
| HBR-013 | source activation 전 official known-good가 pin된다 | NOT PROVEN | store/registry evidence |
| HBR-014 | source 손상 시 official known-good로 복구된다 | NOT PROVEN | fault injection |
| HBR-015 | old manager/schema fallback이 검증된다 | NOT PROVEN | downgrade test |
| HBR-016 | text helper는 exact bytes를 Android clipboard에 쓴다 | NOT PROVEN | fake + device readback |
| HBR-017 | 6,138-byte 초과 답변이 Base64로 화면에 새지 않는다 | NOT PROVEN | real smoke regression |
| HBR-018 | 한국어/emoji/대용량 copy가 손실되지 않는다 | NOT PROVEN | byte equality |
| HBR-019 | clipboard 내용이 log/stderr/manifest에 없다 | NOT PROVEN | redaction audit |
| HBR-020 | helper failure가 OSC 52 long fallback을 유발하지 않는다 | NOT PROVEN | failure injection |
| HBR-021 | helper 미설정 환경의 upstream behavior가 불변이다 | NOT PROVEN | upstream-platform tests |
| HBR-022 | image MIME/URI/raw access가 실제 기기에서 입증된다 | NOT PROVEN | Phase 6 report |
| HBR-023 | HBR-022 전에는 image paste 지원을 표기하지 않는다 | PASS | 이 문서의 gate |
| HBR-024 | image temp가 private하고 항상 정리된다 | NOT PROVEN | security/interrupt tests |
| HBR-025 | source update 실패가 healthy official runtime을 해치지 않는다 | NOT PROVEN | failure matrix |
| HBR-026 | 무인 build 성공이 자동 activation하지 않는다 | NOT PROVEN | pointer audit |
| HBR-027 | clean install/update/rebuild/repair/rollback을 통과한다 | NOT PROVEN | full device run |
| HBR-028 | public repo 외에 능동적 upstream 연결을 만들지 않는다 | NOT PROVEN | remote/release audit |
| HBR-029 | 현재 사용자 미커밋 변경을 보존한다 | PASS | 2026-07-29 status snapshot |
| HBR-030 | release 전 `tests/run-all.sh`를 통과한다 | NOT PROVEN | test log |

---

## 18. 검증 매트릭스

### 18.1 현행 baseline 명령

```sh
git status --short --branch
git rev-parse HEAD
sed -n '1,20p' config/wrapper-version.env
readlink -f "$HOME/.local/lib/codex/termux/current"
readlink -f "$HOME/.local/lib/codex/termux/verified"
readlink -f "$HOME/.local/lib/codex/termux/raw"
sed -n '1,240p' "$HOME/.local/lib/codex/termux/current/runtime-build.json"
codex termux doctor --json
codex --version
```

`codex --version`이 update/network를 시도하지 않도록 live smoke에서는
`CODEX_TERMUX_AUTO_UPDATE=0`을 명시한다.

### 18.2 기존 focused tests

```sh
bash tests/runtime-build.sh
bash tests/store-rollback.sh
bash tests/wrapper-contracts.sh
bash tests/golden.sh
bash tests/run-portable.sh
```

구조·installer/runtime 위험 변경 후:

```sh
bash tests/run-all.sh
```

실제 Termux 기본 smoke:

```sh
CODEX_TERMUX_AUTO_UPDATE=0 bash tests/run-termux.sh
```

실제 설치 runtime을 rebuild/activate하는 변이 smoke는 verified backup과
복구 명령을 확인한 뒤에만 실행한다.

```sh
CODEX_TERMUX_AUTO_UPDATE=0 \
CODEX_TERMUX_RUN_REBUILD_SMOKE=1 \
bash tests/run-termux.sh
```

### 18.3 새로 필요한 tests

제안 이름이며 첫 contract slice에서 확정한다.

```text
tests/source-policy.sh
tests/source-archive-safety.sh
tests/source-patch-queue.sh
tests/source-musl-build.sh
tests/source-runtime-bundle.sh
tests/source-build-interrupt.sh
tests/source-registry-compat.sh
tests/source-activation-rollback.sh
tests/clipboard-helper-contract.sh
tests/clipboard-copy-regression.sh
tests/image-clipboard-feasibility.sh
```

필수 fault cases:

- archive traversal, symlink escape, hash mismatch
- tag/commit mismatch
- patch hunk drift, already-applied patch, unexpected dirty tree
- cargo/network interruption, disk full, compiler killed
- code-mode host/helper 누락 또는 hash mismatch
- candidate install 도중 kill
- pointer switch 전/후 kill
- state write 후 registry write 실패 및 반대 순서
- old manager로 실행/repair/rollback
- helper missing, non-executable, symlink escape, timeout, nonzero exit
- `termux-clipboard-set` missing/permission failure
- clipboard payload 0, boundary-1, boundary, boundary+1, 100 KiB 이상
- image absent, text-only, invalid MIME, revoked URI, oversized image

### 18.4 text clipboard 실기기 판정

각 payload는 copy 후 `termux-clipboard-get` 결과와 byte-for-byte 비교한다.
terminal capture에서도 OSC 52 Base64가 출력되지 않았음을 확인한다.

| Case | 입력 |
|---|---|
| TXT-01 | `hello` |
| TXT-02 | 여러 줄 ASCII |
| TXT-03 | 한국어, emoji, combining character |
| TXT-04 | 6,137 bytes |
| TXT-05 | 6,138 bytes |
| TXT-06 | 6,139 bytes |
| TXT-07 | 8 KiB |
| TXT-08 | 100 KiB |
| TXT-09 | 1 MiB 또는 합의한 상한 |
| TXT-10 | helper timeout/kill |

NUL은 Android text clipboard와 Rust string 의미를 먼저 확인해 명시적으로
reject 또는 normalize한다. 조용히 truncate하면 실패다.

---

## 19. 보안 및 supply-chain 심사

### 19.1 source

- HTTPS fetch만 허용
- exact digest 검증 전 extract/build 금지
- archive path traversal와 special file 거부
- source cache path는 managed root 아래로 제한
- cache key에 upstream commit, archive hash, patchset hash 포함
- `cargo --locked`
- dependency 변경은 `Cargo.lock` diff review 없이 허용하지 않음
- build script가 repo 밖 임의 path를 삭제하지 않음

### 19.2 process

- shell string이나 `eval`로 helper 실행 금지
- argv array와 고정 absolute executable 사용
- timeout 시 process group 정리
- build PID/lock owner 기록
- Android가 장시간 build를 kill할 수 있으므로 resumable cache와 명확한
  partial-state cleanup 제공

### 19.3 clipboard

- clipboard 본문을 log, error, telemetry, manifest에 기록하지 않음
- helper stdout은 protocol 외 비워 둠
- image URI/path를 일반 log에 기록하지 않음
- temp root, file type, ownership, mode, size 검증
- terminal escape injection을 위해 clipboard 내용을 terminal에 재출력하지 않음

### 19.4 public repository

- patch queue는 공개 정보임을 전제로 secret을 넣지 않음
- GitHub token/PAT를 version manifest에 넣지 않음
- local build environment 전체 dump 금지
- DioNanos 또는 제3자 patch 사용 시 source/commit/license 기록

---

## 20. failure class와 처리

| Class | 예 | 처리 |
|---|---|---|
| `INPUT_UNAVAILABLE` | tag/archive 없음 | official lane 유지, 재시도 가능 |
| `INPUT_MISMATCH` | commit/hash/lock 불일치 | fail-closed, 수동 policy 갱신 |
| `PATCH_DRIFT` | hunk 불일치 | 자동 fuzz 금지, patch review |
| `TOOLCHAIN_MISSING` | musl std/linker 없음 | Phase 1 backend 조정 |
| `BUILD_FAILED` | compile/link/V8 실패 | candidate 폐기, official 유지 |
| `BUNDLE_INVALID` | host/helper/resource 누락 | install-inactive 금지 |
| `SMOKE_FAILED` | version/code-mode/FD 실패 | activation 금지 |
| `ACTIVATION_FAILED` | pointer/metadata transaction 실패 | snapshot 복구 |
| `CLIPBOARD_UNAVAILABLE` | API/permission/timeout | payload 비노출 오류 |
| `IMAGE_UNSUPPORTED` | raw clipboard image 접근 불가 | Phase 7 중단 |

`failed-source-update.json` 같은 journal을 도입할 경우 최소한 version,
lane, phase, error class, non-sensitive summary, attempt time, input/patch policy
ID만 저장한다.

---

## 21. activation 및 rollback 심사

### 21.1 activation 전 조건

- candidate ID가 immutable store object를 가리킨다.
- manifest와 실제 tree/hash가 일치한다.
- required test set이 같은 artifact에서 통과했다.
- current/verified/raw/state/registry snapshot이 있다.
- official known-good가 retention pin 상태다.
- 실행 중 runtime path가 prune protection에 포함된다.
- 명시적 사용자 activation 요청이 있다.

### 21.2 activation fault point

최소 다음 위치마다 강제 실패를 주입한다.

1. candidate store copy 전
2. runtime store commit 후
3. registry staged write 후
4. current pointer switch 직전
5. current pointer switch 직후
6. state commit 전
7. state commit 후
8. doctor smoke 전/후
9. source lane verified 표시 전/후

각 경우 process 재실행 후 state, registry, pointer가 서로 설명 가능한 한
tuple을 가리켜야 한다. 반쯤 활성화된 source runtime을 정상으로 보고하면
실패다.

### 21.3 즉시 rollback trigger

- `codex --version` 비정상 종료
- FD 33/34 probe 실패
- TUI startup crash 또는 terminal corruption
- signal/TTY/upstream exit regression
- code-mode host spawn 실패
- 긴 copy에서 Base64 leakage
- doctor `overallStatus != ok`
- active tuple과 registry/state 불일치

### 21.4 recovery 우선순위

1. 같은 lane의 이전 verified artifact
2. pinned `official-patched` known-good
3. cached official raw에서 현행 patch rebuild
4. fresh official upstream install

source rebuild가 복구의 유일한 경로가 되면 안 된다.

---

## 22. 검토·심사 체크리스트

### Architecture review

- [ ] 두 lane의 입력, builder, patch policy, artifact ID가 구분된다.
- [ ] build와 activation이 분리된다.
- [ ] external helper가 bundle transaction에 포함된다.
- [ ] current public launcher/command 계약을 보존한다.

### Source/provenance review

- [ ] tag/commit/archive/lock/tree/patch digest가 모두 있다.
- [ ] fork, PR, issue, workflow가 필요 없다.
- [ ] patch drift가 fail-closed한다.
- [ ] third-party 참고 코드의 license와 source가 기록된다.

### Compatibility review

- [ ] schema 3 old/new reader behavior가 입증된다.
- [ ] FD/signal/TTY/exit behavior가 입증된다.
- [ ] code-mode host와 resources가 일치한다.
- [ ] repair/rebuild/rollback이 lane-aware하다.

### Clipboard review

- [ ] helper opt-in 외 환경은 upstream behavior를 유지한다.
- [ ] helper path와 protocol이 고정되어 있다.
- [ ] long payload가 OSC 52로 새지 않는다.
- [ ] payload가 어디에도 기록되지 않는다.
- [ ] image는 feasibility 통과 전 지원으로 표시되지 않는다.

### Security review

- [ ] archive/cache/temp path가 managed-path policy를 따른다.
- [ ] shell injection 및 symlink escape가 차단된다.
- [ ] build/clipboard timeout과 cleanup이 있다.
- [ ] secret-bearing env와 clipboard content가 redact된다.

### Release review

- [ ] focused tests와 regression tests가 있다.
- [ ] `tests/run-all.sh`가 통과한다.
- [ ] real-Termux install/update/rebuild/repair/rollback이 통과한다.
- [ ] official known-good가 실제로 복구 가능하다.
- [ ] 활성/merge/push를 사용자가 명시적으로 승인했다.

---

## 23. 재개 절차

다음 작업자는 아래 순서를 그대로 따른다.

### 23.1 저장소 상태 확인

```sh
cd /data/data/com.termux/files/home/prj/codex
git status --short --branch
git rev-parse HEAD
git remote -v
```

2026-07-29 당시 `src/wrapper/support_layout.py`에 사용자 미커밋 변경이
있었다. 그대로 남아 있다면 내용을 덮어쓰거나 stage하지 않는다.

### 23.2 제품 기준 재측정

```sh
sed -n '1,20p' config/wrapper-version.env
sed -n '1,280p' config/refactor-boundaries.json
sed -n '1,220p' config/layout-contracts.json
readlink -f "$HOME/.local/lib/codex/termux/current"
readlink -f "$HOME/.local/lib/codex/termux/verified"
readlink -f "$HOME/.local/lib/codex/termux/raw"
CODEX_TERMUX_AUTO_UPDATE=0 codex termux doctor --json
CODEX_TERMUX_AUTO_UPDATE=0 codex --version
```

branch, wrapper, active runtime, patch policy, schema 또는 test surface가
바뀌었으면 이 문서의 **문서 통제**, **현행 구조와 증거**, **변경 기록**을
먼저 갱신한다.

### 23.3 구현 시작 조건

- 사용자가 실제 구현 시작을 명시했다.
- 현재 `support-activation` dirty work의 처리 방향이 정해졌다.
- 새 branch/slice가 기존 변경과 분리되었다.
- Phase 0 baseline tests가 통과했다.
- 수용 기준 원장의 가장 빠른 `NOT PROVEN` 항목이 선택되었다.

### 23.4 작업 규칙

1. 한 번에 한 slice만 구현한다.
2. 실제 smoke failure마다 같은 branch에 regression test를 추가한다.
3. build artifact는 active pointer와 분리한다.
4. 실기기 변경 전 verified 복구 경로를 확인한다.
5. 실패하면 다음 phase로 넘어가지 않는다.
6. evidence path와 명령을 원장에 기록한다.
7. 사용자가 요청하기 전 commit/push/activation을 임의로 넓히지 않는다.

### 23.5 최초 재개 작업

가장 먼저 할 구현은 clipboard patch가 아니다.

```text
source-build-contract
  -> source-musl-feasibility
  -> source-runtime-artifact
  -> clipboard-adapter
```

musl 무수정 build가 입증되지 않은 상태에서 기능 patch부터 작성하면
toolchain failure와 patch failure를 분리할 수 없다.

---

## 24. 미결정 사항

아래 항목은 구현 시작 시 결정하고 근거를 이 문서에 추가한다.

| ID | 질문 | 결정 시점 |
|---|---|---|
| OQ-01 | M1 native와 M2 isolated 중 어느 musl backend가 지속 가능한가 | Phase 1 |
| OQ-02 | exact upstream source archive/hash index를 어떤 파일 형식으로 고정할까 | Phase 0 |
| OQ-03 | matching code-mode host/V8를 같은 build에서 어떻게 만들까 | Phase 1 |
| OQ-04 | source input을 schema 3에 optional extension으로 넣을 수 있는가 | Phase 0/4 |
| OQ-05 | per-lane known-good를 registry index와 symlink 중 어디에 둘까 | Phase 4 |
| OQ-06 | source public CLI의 최종 command 이름은 무엇인가 | Phase 5 |
| OQ-07 | SSH에서 Android clipboard와 OSC 52 client clipboard 중 기본은 무엇인가 | Phase 3 |
| OQ-08 | text payload 상한과 timeout은 얼마인가 | Phase 3 |
| OQ-09 | 새 APK 없이 image clipboard bytes를 읽을 수 있는가 | Phase 6 |
| OQ-10 | source build를 official update 뒤 동기 실행할지 명시적 별도 실행할지 | Phase 5 |

---

## 25. 외부 기준 자료

날짜가 지나면 내용이 바뀔 수 있으므로 구현 재개 시 exact tag/commit으로
다시 고정한다.

- OpenAI Codex repository: <https://github.com/openai/codex>
- OpenAI Codex 0.146.0 clipboard source:
  <https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/tui/src/clipboard_copy.rs>
- Termux terminal OSC parser:
  <https://github.com/termux/termux-app/blob/master/terminal-emulator/src/main/java/com/termux/terminal/TerminalEmulator.java>
- Termux:API clipboard source:
  <https://github.com/termux/termux-api/blob/master/app/src/main/java/com/termux/api/ClipboardAPI.java>
- DioNanos Termux port reference:
  <https://github.com/DioNanos/codex-termux>

---

## 26. 변경 기록

| 날짜 | 상태 | 변경 |
|---|---|---|
| 2026-07-29 | PROPOSED | 현행 유지 + A + B + musl 하이브리드 결정을 최초 고정. 구현은 시작하지 않음. |
