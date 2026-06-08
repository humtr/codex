# lib

`lib/`는 wrapper의 상태 관리와 실행 정책을 소유한다. Runtime readiness, raw/runtime registry, auto-update, profile selection, resolver fd 준비, public command routing은 이 경계 안에서 결정된다.

## 경계

- `lib/codex-termux-lib.sh`는 함수형 shell library이며 직접 실행 entrypoint가 아니다.
- Public launcher 작성과 support file 복사는 `bin/`이 호출하되, runtime mutation 정책은 이 파일의 함수가 소유한다.
- Binary patching과 compatibility tool 구현은 `tools/`가 소유한다.
- 사용자의 upstream Codex auth/config 내용은 읽기·쓰기 정책 대상이 아니며, profile 실행을 위해 `CODEX_HOME`만 설정한다.

## 항상 지킬 것

- State mutation은 `codex_with_lock` 경로를 거친다.
- Raw, wrapper, runtime tuple은 registry와 state에 함께 기록한다.
- Runtime readiness는 binary 존재만 보지 않고 support tool copy match와 state file도 확인한다.
- Runtime 실행 전 `SSL_CERT_FILE`, optional `SSL_CERT_DIR`, `CODEX_SELF_EXE`, sanitized library env, PATH, resolver fd를 준비한다.
- Prompt-like 첫 인자는 upstream command가 아닐 때만 `exec`로 라우팅한다.
- Profile name은 path traversal, hidden name, option-like name, whitespace를 허용하지 않는다.
- Named profile plugin symlink는 `plugins` 항목이 없을 때만 만든다.
- Termux profile 실행은 `CODEX_NATIVE_PROFILE_NETWORK_ACCESS=0`이 아닌 한 selected profile의 workspace-write network access를 true로 보장한다.

## 변경 검증

`lib/codex-termux-lib.sh`를 바꾸면 `bash -n lib/codex-termux-lib.sh`를 실행한다. State나 runtime promotion을 바꾸면 `codex doctor --json`에서 state, registry, active tuple, support match가 정상인지 확인한다. Profile 경로를 바꾸면 `default`와 named profile 모두에서 `CODEX_HOME`과 plugin symlink 보존 조건을 확인한다.
