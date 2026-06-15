# 구조

이 프로젝트의 시스템 경계는 Termux 사용자 환경 안에 있다. root의 `install.sh`가 의존성 설치를 맡고, `bin/install-runtime.sh`가 public launcher와 support file을 배치하며, `lib/codex-termux-lib.sh`가 런타임 상태·프로필·업데이트·실행 정책을 관리한다. `tools/`는 공식 패키지에서 받은 raw vendor tree를 Termux에서 실행 가능한 runtime tree로 바꾸는 변환 도구다.

대표 설치 흐름은 `install.sh` -> `bin/install-runtime.sh setup` -> manager support file 설치 -> launcher 설치 -> legacy layout migration -> raw package 확보 또는 cached raw로 repair -> candidate runtime build -> smoke test -> immutable artifact store 기록 -> state/registry 기록 -> `current`/`verified`/`raw` pointer activation 순서다. 의존성 설치가 먼저 끝나야 `npm pack`, `python3`, `tar`, `curl`, `bash`를 신뢰할 수 있고, support file 설치가 먼저 끝나야 runtime rebuild가 최신 `bwrap`/`rg` 호환 도구를 runtime-private `codex-path`에 넣을 수 있다.

대표 실행 흐름은 `$PREFIX/bin/codex` -> `native/manager/managed.sh` -> `codex_main` -> legacy layout migration check -> runtime manifest/hash readiness check -> verified pointer rollback check -> auto-update check -> resolver fd 준비 -> upstream Codex exec 순서다. prompt처럼 보이는 첫 인자는 upstream에 직접 넘기지 않고 `exec` 하위 명령으로 라우팅하며, `--`는 이 라우팅을 끄는 명시적 passthrough다.

런타임 저장소는 raw, wrapper, runtime의 세 층으로 나뉜다. raw는 공식 npm package의 vendor tree이고, wrapper는 `native/manager`의 support 도구와 버전 정보이며, runtime은 raw와 wrapper support를 합쳐 만든 실행 산출물이다. `native/store/runtime/<tuple>`과 `native/store/raw/<tuple>`은 immutable artifact로 취급하고, `native/current`, `native/verified`, `native/raw` pointer가 현재 실행 runtime, last-known-good runtime, 현재 raw cache를 가리킨다. registry는 이 세 값을 tuple로 묶어 “어떤 upstream 바이너리를 어떤 wrapper 코드로 패치했는지”를 추적한다.

state는 active tuple과 last-known-good verified tuple을 함께 기록한다. runtime prune는 active tuple과 verified tuple을 우선 보호하고, verified tuple은 smoke test와 readiness check를 통과한 runtime만 가리켜야 한다.

프로필 경계는 Codex의 `CODEX_HOME` 경계와 같다. 기본 프로필은 `~/.codex`, named profile은 `~/.codex-profiles/<name>`이며, named profile에 `plugins` 항목이 없을 때만 `~/.codex/plugins`를 가리키는 symlink를 만든다. 이미 존재하는 profile-local `plugins` 파일, 디렉터리, symlink는 보존 대상이다.

외부 의존성은 Termux package manager, npm registry, OpenAI Codex upstream package, Android process environment, Termux TLS certificate store, Termux resolver file이다. 이 프로젝트는 OpenAI 계정 인증을 직접 처리하지 않으며, upstream Codex가 사용하는 `CODEX_HOME` 내부 상태를 runtime 설치 상태와 분리한다.
