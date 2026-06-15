# 보안 정책

보호 대상은 사용자의 Codex 인증·설정 상태, 기존 public launcher, raw/runtime registry, TLS trust path, resolver file 참조다. 설치와 업데이트는 runtime 산출물을 바꾸는 작업이지 사용자의 upstream Codex 계정 상태를 관리하는 작업이 아니다. 인증 파일이나 profile config가 손상되면 runtime update 성공 여부와 무관하게 실패로 취급한다.

이 프로젝트에는 별도 로그인 흐름이 없다. OpenAI 인증과 세션 처리는 upstream Codex가 `CODEX_HOME` 안에서 수행한다. 래퍼가 하는 일은 실행 전 환경을 준비하는 것이며, credential을 읽거나 저장하거나 rotation하지 않는다. 따라서 credential lifetime, revocation, storage format은 upstream Codex의 소유이고 이 repo의 코드가 재정의하면 안 된다.

권한 모델은 단일 Termux 사용자 모델이다. 같은 Android app sandbox 안의 사용자 파일은 그 사용자 권한으로 읽고 쓴다. 관리형 경로는 `~/.local/lib/codex/native`, `~/.local/share/codex/native`, `$PREFIX/bin/codex`이며, 그 밖의 Codex auth/config/profile 내용은 명시적으로 profile 실행에 필요한 경우를 제외하고 mutation 대상이 아니다.

Public launcher는 marker 기반으로만 소유권을 판단한다. Marker가 없는 `$PREFIX/bin/codex`는 외부 도구일 수 있으므로 backup 없이 제거하면 안 된다. Bwrap compatibility launcher는 runtime-private path에만 두며 public `$PREFIX/bin/bwrap`은 생성하거나 변경하지 않는다.

Immutable store는 기존 tuple artifact를 rewrite하지 않는다. 새 publish candidate의 내용이나 실행 권한이 기존 tuple과 다르면 collision로 취급해 activation을 거부하고, 기존 artifact는 그대로 보존한다.

Termux용 `bwrap` 호환 도구는 Linux namespace isolation을 제공하지 않는다. Android에서 bubblewrap namespace setup이 막히는 경우가 일반적이므로, 호환 도구는 Codex가 넘긴 env/cwd/argv 실행 계약을 보존한 뒤 inner command를 실행한다. 이 경로를 sandbox 보안 경계로 설명하면 안 되며, 격리가 필요한 일반 Linux 환경의 보안 모델과 동일하게 취급하면 안 된다.

Termux profile 실행은 사용자의 network와 approval 설정을 그대로 보존한다. Compatibility bwrap은 filesystem namespace 격리를 제공하지 않지만, upstream Codex의 network-off seccomp는 socket 생성을 실제로 차단한다. Wrapper는 이 제한 경계를 자동으로 끄지 않으며, 필요한 명령의 approval 요청 생성과 승인 후 외부 실행은 upstream Codex의 책임이다.

Runtime binary patch는 resolver path 치환만 허용한다. 매 실행 전 활성 runtime hash와 build manifest를 검증하며, drift가 있으면 hash가 확인된 cached raw로만 repair한다. Raw hash도 어긋나면 fail-closed하고 update를 요구한다.

Pointer activation과 state/registry metadata 갱신은 rollback 가능해야 한다. current/verified/raw pointer, state, registry 중 하나라도 교체에 실패하면 이전 active installation을 복구해야 하며, prune는 metadata drift와 별개로 실제 pointer target을 보호해야 한다.

Upstream package fetch는 tarball safety 검사를 통과한 member만 추출해야 한다. Absolute path, traversal, symlink, hardlink, special file은 설치 전에 거부한다.

실행 전 환경에서는 `LD_PRELOAD`, `LD_LIBRARY_PATH`, npm/bun 관리 marker를 제거한다. Termux와 upstream Linux binary의 library 경로가 섞이면 의도하지 않은 shared library가 runtime에 주입될 수 있으므로, Codex runtime 실행에는 관리형 runtime path, Termux prefix, TLS certificate path, resolver fd만 남긴다.

감사 가능한 기록은 state, registry, backup filename, doctor output이다. 민감한 auth token은 이 기록에 들어가면 안 된다. registry는 hash와 경로, wrapper version, package spec을 남겨 runtime provenance를 설명하고, backup directory는 외부 launcher를 복구할 수 있는 최소 증거를 남긴다.

Legacy store migration report는 imported/skipped/error 같은 provenance만 남기며 auth/config/token/cookie/OAuth material을 기록하지 않는다.

Failure case 기록은 update/build/repair/install 경로에만 제한하고, auth/config/token/cookie/OAuth material을 수집하지 않는다.
