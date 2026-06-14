# tools

`tools/`는 공식 raw vendor tree를 Termux runtime tree로 바꾸는 변환 도구와 runtime path에 들어가는 compatibility tools를 소유한다.

## 경계

- `build-runtime.py`는 raw vendor tree를 읽어 patched runtime tree를 만든다.
- `bwrap-termux-compat.py`는 Codex가 호출하는 bubblewrap command line에서 execution-relevant option만 적용하고 inner command를 실행한다.
- `codex-launcher.c`는 public launcher가 shell library를 빠르게 exec하게 하는 작은 C entrypoint다.
- `rg-termux-shim.sh`는 Termux `rg`가 있으면 그것을 쓰고 없으면 bundled `rg.real`로 fallback한다.
- State/registry JSON 작성과 profile policy는 `lib/`의 소유다.

## 항상 지킬 것

- Resolver rewrite는 같은 byte length의 문자열만 허용한다.
- Raw binary에 이미 target resolver path가 있으면 중복 patch로 취급하고 실패한다.
- Required upstream paths가 없으면 runtime을 부분 생성하지 않는다.
- Runtime tree 교체는 temporary build directory에서 완성한 뒤 원자적으로 교체한다.
- `bwrap-termux-compat.py`는 `--args`, `--clearenv`, `--setenv`, `--unsetenv`, `--chdir`, `--argv0`의 실행 의미를 보존한다.
- Compatibility bwrap은 namespace 격리 성공을 주장하지 않는다.
- Compatibility bwrap은 runtime-private `codex-path/bwrap`에만 배치하고 public `$PREFIX/bin/bwrap`을 관리하지 않는다.

## 변경 검증

`build-runtime.py`를 바꾸면 raw vendor fixture나 live raw package로 runtime build를 실행하고 report JSON에서 resolver source/target count와 resource paths를 확인한다. `bwrap-termux-compat.py`를 바꾸면 `--version`, `--help`, `--setenv`, `--chdir`, `--args`, missing `--` error를 최소 smoke test한다. `codex-launcher.c`를 바꾸면 compiled binary에 marker 문자열이 남아 있는지 확인한다.
