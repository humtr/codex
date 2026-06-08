# bin

`bin/`은 repo 안에서 설치 runtime 작업을 직접 실행하는 명령 표면을 소유한다. Public `$PREFIX/bin/codex`의 실제 runtime 정책은 `lib/`가 소유하고, `bin/`은 support file 배치, launcher 작성, setup/update/remove/doctor command dispatch를 맡는다.

## 경계

- `bin/install-runtime.sh`는 runtime support file을 복사하고 public launcher를 만든다.
- Upstream package fetch, state/registry record, profile execution의 세부 정책은 shell library 함수로 위임한다.
- Termux dependency 설치는 root `install.sh`의 소유이며, `bin/`은 `apt-get`을 직접 호출하지 않는다.
- Python patch logic이나 bwrap argument parsing은 `tools/`의 소유다.

## 항상 지킬 것

- Public launcher slot을 준비할 때 directory를 파일로 교체하지 않는다.
- Marker 없는 launcher는 backup을 만든 뒤 제거한다.
- Compiled launcher를 만들 때 marker 문자열이 실제 binary에 있는지 확인한다.
- Support file을 갱신하면 `wrapper-version.env`에 wrapper commit과 installed timestamp를 기록한다.
- `support` command는 upstream runtime을 새로 받지 않는다.

## 변경 검증

`bin/install-runtime.sh`를 바꾸면 최소한 `bash -n bin/install-runtime.sh`를 실행한다. Launcher 작성 경로를 바꾸면 임시 prefix나 live support refresh에서 `$PREFIX/bin/codex`와 `$PREFIX/bin/bwrap`의 marker, executable bit, target command를 확인한다.
