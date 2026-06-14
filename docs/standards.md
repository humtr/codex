# 변경 규칙

- Runtime 원본은 `@openai/codex` Linux ARM64 npm package여야 한다. 다른 fork나 local build를 runtime 원본으로 쓰는 변경은 이 repo의 목표를 바꾸는 일이므로 일반 update로 처리하면 안 된다.
- Raw binary patch는 byte length를 보존해야 한다. `/etc/resolv.conf`를 다른 길이의 문자열로 바꾸면 ELF 내부 문자열 배치가 깨질 수 있으므로, resolver rewrite는 같은 길이의 `/proc/self/fd/33`만 허용한다.
- Runtime 실행 전 fd 33은 readable resolver file로 열려 있어야 한다. fd 33 준비를 제거하거나 다른 번호로 바꾸면 patched musl binary가 resolver 설정을 찾지 못하므로 doctor의 DNS patch check도 함께 바뀌어야 한다.
- State를 바꾸는 setup, update, repair, use 작업은 lock을 거쳐야 한다. 동시에 runtime tree나 registry를 교체하면 raw/runtime tuple이 엇갈릴 수 있으므로 lock 우회는 위반이다.
- Public launcher 교체는 marker 검사와 backup을 유지해야 한다. marker 없는 launcher를 backup 없이 삭제하거나 directory launcher path를 파일로 바꾸는 변경은 허용하지 않는다.
- `CODEX_HOME` profile 실행은 profile auth를 runtime 상태와 분리해야 한다. 허용되는 profile config 변경은 `CODEX_NATIVE_PROFILE_NETWORK_ACCESS=0`이 아닐 때 workspace-write network access를 true로 보장하는 것뿐이며, auth migration이나 다른 config rewrite로 확장하면 안 된다.
- Named profile의 plugin 공유는 profile-local `plugins` 항목이 없을 때만 symlink를 만든다. 이미 존재하는 file, directory, symlink를 공유 symlink로 바꾸면 profile-local plugin 선택권을 침해한다.
- Termux `bwrap` 호환 도구는 env, cwd, argv0, `--args` 실행 계약을 보존해야 한다. namespace/mount option은 Android에서 실행 보장 대상이 아니지만, inner command 실행에 영향을 주는 option은 무시하면 안 된다.
- Termux compatibility bwrap은 runtime-private `codex-path/bwrap`에만 배치하고 runtime 실행 PATH에서 public Termux tools보다 먼저 선택해야 한다.
- Verification은 변경 범위에 맞춰 실제 소비 경로를 확인해야 한다. Shell 변경은 `bash -n`으로 syntax를 확인하고, Python 도구 변경은 해당 entrypoint의 help 또는 smoke path를 실행하며, runtime 배치 변경은 `doctor --json`의 relevant check를 통과시켜야 한다.
- 네트워크 문제 수정은 resolver file 변경만으로 완료했다고 판단하면 안 된다. sandbox network denial, external resolver 응답, profile `network_access` 설정을 분리해 확인해야 한다.
