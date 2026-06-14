# 작업 메모

## DNS 실패처럼 보이는 sandbox 차단

증상: `curl`이나 upstream Codex 명령이 `Could not resolve host` 또는 DNS 실패처럼 보이는 오류를 낸다.  
원인: profile sandbox에서 network access가 꺼져 있으면 DNS socket 자체가 막혀 resolver 장애처럼 보일 수 있다. 외부 resolver가 실제로 죽은 경우와 오류 표면이 비슷하다.  
대응: sandbox 안팎을 분리해 검증한다. sandbox 밖에서 `dig @1.1.1.1 <host> A`, `dig @8.8.8.8 <host> A`, `curl -I <url>`이 성공하면 외부 DNS가 아니라 sandbox/profile network 설정 문제다. 이 wrapper는 profile 실행 때 `[sandbox_workspace_write] network_access = true`를 보장하므로, 같은 증상이 남으면 현재 실행이 wrapper profile 경로를 탔는지와 `CODEX_NATIVE_PROFILE_NETWORK_ACCESS=0` 설정 여부를 먼저 확인한다.

## `/etc/resolv.conf` 수정과 fd 33 패치의 역할

증상: Termux에서 공식 Linux musl binary가 resolver 설정을 찾지 못한다.  
원인: binary가 Linux 기본 경로인 `/etc/resolv.conf`를 읽으려 하지만 Termux의 resolver file은 `$PREFIX/etc/resolv.conf`에 있다.  
대응: build step에서 binary 문자열을 `/proc/self/fd/33`으로 바꾸고, 실행 wrapper가 fd 33을 Termux resolver file로 열어 둔다. 이 방식은 경로 차이를 해결할 뿐이며, 외부 DNS 차단이나 Codex sandbox network denial을 해결하지 않는다.

## Termux `bwrap`는 격리 도구가 아니다

증상: upstream Codex가 bubblewrap을 찾거나 namespace 관련 warning을 낸다.  
원인: Android/Termux에서는 Linux namespace setup이 일반 Linux와 같은 방식으로 동작하지 않는다.  
대응: Runtime-private `codex-path/bwrap`은 namespace/mount setup을 수행하지 않고 inner command를 실행하는 compatibility launcher로 유지한다. Runtime PATH에서 이 경로를 public Termux tools보다 먼저 두며, 문서나 출력에서 Linux sandbox 보안 보장을 제공한다고 표현하면 안 된다.

## Auto-update prompt가 반복되는 이유

증상: interactive `codex` 실행 때 같은 update prompt가 계속 나온다.  
원인: 새 dist-tag가 발견되었고 사용자가 현재 runtime 실행을 선택하면 pending version을 남긴다. 이 상태는 “알림을 봤지만 update하지 않음”이지 “영구 무시”가 아니다.  
대응: 현재 runtime을 계속 쓰려면 prompt에서 current 선택을 반복하거나 `CODEX_NATIVE_AUTO_UPDATE=0` 또는 auto-update mode off를 사용한다. 업데이트하려면 prompt에서 update를 선택하거나 `codex update`를 직접 실행한다.

## Profile plugin 공유

증상: named profile에서 default profile에 설치된 plugin이나 skill이 보이지 않는다.  
원인: upstream Codex는 `CODEX_HOME` 기준으로 plugin path를 찾고, named profile은 기본적으로 별도 home을 사용한다.  
대응: named profile에 `plugins` 항목이 없을 때만 `~/.codex/plugins`를 가리키는 symlink를 만든다. 이미 profile-local plugins가 있으면 공유하지 않는 선택으로 보고 그대로 둔다.

## Support drift repair

증상: runtime binary는 존재하지만 `doctor`에서 `support_bwrap_match`나 `support_rg_match`가 실패한다.  
원인: wrapper support tool이 업데이트되었지만 runtime tree 안의 복사본은 이전 버전일 수 있다.  
대응: cached raw package가 있으면 runtime을 다시 rebuild해 support tool 복사본을 맞춘다. raw package가 없으면 runtime을 재구성할 근거가 없으므로 package fetch가 필요한 setup/update를 실행한다.

## Termux `rg` 우선

증상: upstream package에 bundled `rg`가 있어도 Termux 환경에서 기대한 동작과 다를 수 있다.  
원인: bundled Linux tool보다 Termux package의 `rg`가 Android path와 libc 환경에 더 잘 맞는다.  
대응: runtime path의 shim은 `/data/data/com.termux/files/usr/bin/rg`가 실행 가능하면 그것을 우선 사용하고, 없으면 bundled `rg.real`로 fallback한다.
