# 동작 규칙

공식 upstream package가 runtime의 유일한 원본이다. 사용자가 버전을 생략하거나 `latest`, `stable`을 요청하면 `@openai/codex@linux-arm64`를 사용하고, 숫자 버전만 요청하면 `@openai/codex@<version>-linux-arm64`로 정규화한다. Android-native fork나 별도 Rust rebuild 결과물은 이 repo의 runtime 원본이 아니다.

관리형 runtime은 raw package, wrapper support version, runtime 산출물의 조합으로 식별된다. 새 runtime을 만들면 raw 바이너리 hash, runtime 바이너리 hash, wrapper version, wrapper commit, package spec을 state와 registry에 기록해야 한다. 기록되지 않은 runtime은 나중에 `use`, repair, doctor가 원인을 추적할 수 없으므로 promoted runtime으로 취급하지 않는다.

runtime을 promoted runtime으로 취급하려면 candidate build가 실제 smoke test를 통과하고, verified tuple이 state에 기록되어야 한다. smoke test가 통과하지 않은 runtime은 store에 남더라도 last-known-good로 취급하지 않는다.

기존 Codex 사용자 상태는 runtime 설치 상태와 섞지 않는다. `~/.codex`와 `~/.codex-profiles/*`는 upstream Codex의 auth/config/profile 상태이며, runtime 설치·업데이트는 이 상태를 새로 만들거나 덮어쓰는 작업이 아니다. 프로필 실행은 `CODEX_HOME`만 바꾸고 runtime binary와 support tools는 공유한다.

public launcher 교체는 관리형 marker가 있는 파일과 없는 파일을 다르게 다룬다. marker가 있는 launcher는 같은 관리 주체의 파일이므로 교체할 수 있고, marker가 없는 launcher는 먼저 backup directory에 복사한 뒤 제거해야 한다. directory인 launcher path는 실행 파일로 바꿀 수 없으므로 실패해야 한다.

auto-update는 사용자의 실행 흐름을 강제로 끊지 않는다. interactive 실행에서 새 Linux ARM64 dist-tag가 발견되면 pending version을 기록하고 선택지를 보여준다. 사용자가 현재 runtime 실행을 고르면 pending 상태를 유지해 다음 interactive 실행에서도 같은 update 가능성을 알린다. non-interactive 실행은 prompt를 띄우지 않는다.

auto-update가 실패해도 현재 runtime이 건강하면 실행은 계속되어야 한다. update 실패는 pending 기록과 실패 로그만 남기고, 기존 runtime을 훼손하거나 기본 실행을 막으면 안 된다.

runtime drift는 cached raw package가 있을 때만 자동 repair 대상이다. support file이 바뀌었거나 runtime tree의 support tool 복사본이 어긋났다면 cached raw에서 runtime을 다시 만들어야 한다. cached raw가 없으면 wrapper는 package를 추측하거나 빈 runtime을 만들지 않고 `codex setup`을 요구해야 한다.

DNS resolver 경로는 공식 musl binary가 Termux의 resolver file을 읽을 수 있게 하는 호환 규칙이다. raw binary 안의 `/etc/resolv.conf` 문자열은 같은 byte length의 `/proc/self/fd/33`으로 바뀌어야 하고, 실행 전 fd 33은 Termux resolver file로 열려 있어야 한다. 이 규칙은 외부 DNS 장애를 해결하는 정책이 아니라 Android 파일 경로 차이를 해결하는 실행 호환 규칙이다.

Wrapper는 default 또는 named profile의 `config.toml`을 수정하지 않는다. Network-off에서 upstream Codex가 적용하는 seccomp socket 차단은 Termux에서도 유효한 제한 경계이며, network 허용과 approval 요청 생성은 upstream Codex와 사용자 설정의 책임이다.

Runtime binary는 raw binary에 resolver path 치환만 적용한 결과여야 한다. Build manifest가 없거나 현재 patch policy·builder·실제 hash와 맞지 않는 runtime은 실행하거나 cache에서 승격할 수 없다. Compatible runtime store는 활성 runtime을 포함해 최신 세 개를 기본으로 보존한다.

raw/runtime promotion은 candidate staging과 rollback-safe swap으로 처리해야 한다. 중간 실패가 active raw/runtime를 엇갈리게 만들면 안 된다.
