# fd 33 resolver 경로

## 상황

공식 Linux musl binary는 resolver 설정을 `/etc/resolv.conf`에서 찾는다. Termux에서는 resolver file이 `$PREFIX/etc/resolv.conf`에 있으므로 raw binary가 기대하는 경로와 실제 경로가 다르다.

## 결정

Runtime build 단계에서 raw binary 안의 `/etc/resolv.conf` 문자열을 같은 길이의 `/proc/self/fd/33`으로 바꾸고, 실행 wrapper가 fd 33을 Termux resolver file로 연다.

## 대안

Termux filesystem에 `/etc/resolv.conf`를 맞추는 방법은 Android app sandbox와 prefix 경계를 흐린다. Binary를 재빌드하는 방법은 공식 package 원본을 유지한다는 목표와 맞지 않는다. Resolver path를 더 긴 경로로 직접 patch하는 방법은 binary 내부 문자열 길이를 깨뜨린다.

## 결과

Resolver path 문제는 runtime 실행 전에 fd 33을 열어야 하는 규칙으로 바뀐다. 이 결정은 외부 DNS 장애나 Codex sandbox network denial을 해결하지 않으므로, 네트워크 오류 진단 때는 resolver path와 sandbox policy를 분리해야 한다.
