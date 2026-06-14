# 현재 상태

## 구현됨

- 공식 `@openai/codex` Linux ARM64 package를 npm에서 받아 raw vendor tree로 보관하고, Termux용 runtime tree로 rebuild하는 경로가 구현되어 있다.
- Runtime rebuild는 resolver path rewrite, support `bwrap`/`rg` tool 설치, package metadata 복사, executable permission 보정을 수행한다.
- Public `$PREFIX/bin/codex` launcher만 설치하며, bwrap compatibility launcher는 runtime-private `codex-path/bwrap`에 둔다.
- `state.json`과 `registry.json`은 raw package, wrapper version/commit, runtime hash, active tuple을 기록한다.
- `codex setup`, `update`, `doctor`, `version`, `help`, `use`, `profile`, `remove`, `--` passthrough가 구현되어 있다.
- Auto-update check는 interactive 실행에서 prompt를 띄우고, non-interactive 실행에서는 prompt 없이 현재 runtime을 계속 실행한다.
- Runtime drift repair는 cached raw package가 있을 때 support tool mismatch를 runtime rebuild로 복구한다.
- Named profile의 `plugins` 항목이 없으면 default `~/.codex/plugins`를 공유 symlink로 연결한다.
- Termux profile 실행은 profile config와 upstream network·approval policy를 변경하지 않는다.
- Network-off seccomp와 명시적 network-on/reset 경로를 wrapper doctor와 회귀 테스트로 검증한다.
- Runtime binary는 DNS-only patch manifest와 실제 hash를 매 실행 전 검증하고 compatible cache는 최신 세 개만 보존한다.

## 검증됨

- 최근 작업 범위에서 `bash -n lib/codex-termux-lib.sh`가 통과했다.
- `bash tests/profile-behavior.sh`가 profile config 비변경과 plugin 공유를 검증한다.
- `bash tests/network-boundary.sh`, `bash tests/runtime-integrity.sh`, `bash tests/doctor-contract.sh`가 network, runtime, public doctor 계약을 검증한다.
- 최근 작업 범위에서 `git diff --check`가 통과했다.
- 최근 live 설치에서 `codex doctor`의 overall status가 ok로 확인됐다.
- DNS 문제는 sandbox 밖 resolver query와 curl이 성공했고, sandbox 안에서는 network denial이 DNS 실패처럼 보이는 것으로 분리 확인됐다.

## 남은 일

- Profile 동작과 runtime-private bwrap 경로에는 자동화된 regression test가 있다. 나머지 검증은 shell syntax, targeted smoke test, `doctor`, 수동 DNS 분리 확인에 의존한다.
- `install.sh`와 `tools/build-runtime.py`의 전체 설치/rebuild regression test는 파일로 고정되어 있지 않다.
- 일반 Linux에서 real bubblewrap 격리를 지원하는 별도 mode는 이 repo의 현재 범위가 아니다. 필요하면 Termux compat 경로와 분리된 새 정책 결정이 먼저 필요하다.
- Approval 요청 생성 여부는 upstream Codex agent 동작이며 wrapper 회귀 테스트 범위 밖이다.

## 현재 차이

Wrapper source version은 `1.1.0`이다. Profile config 비변경, runtime-private bwrap, DNS-only runtime 무결성, cache retention, network 경계, public doctor 합성 계약을 각각 회귀 테스트한다.
