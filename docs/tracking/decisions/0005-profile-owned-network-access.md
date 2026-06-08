# Termux profile network access 기본값

## 상황

Sandbox에서 network access가 꺼져 있으면 DNS 실패처럼 보이는 오류가 발생하고, `approve for me` 상태에서도 명령 승인이 반복될 수 있다. Termux의 compatibility `bwrap` 경로는 namespace 기반 네트워크 격리를 제공하지 않으므로, network-off 기본값은 보안 경계라기보다 실행 실패와 승인 반복의 원인이 되기 쉽다.

## 결정

Termux profile 실행은 selected profile의 workspace-write network access를 true로 보장한다. 이 기본값은 `CODEX_NATIVE_PROFILE_NETWORK_ACCESS=0`으로 끌 수 있다.

## 대안

Wrapper가 network access를 항상 profile 소유로 남기면 upstream Codex의 보수적 기본값을 존중하지만, Termux compatibility path에서는 DNS-like 실패와 반복 승인을 계속 만든다. Wrapper가 setup 때 모든 profile을 일괄 수정하면 사용성은 더 즉시 좋아지지만, 사용하지 않는 profile까지 바꾸는 부작용이 있다. Wrapper가 network access를 항상 끄면 일반 Linux sandbox의 보수적 기본값과 맞지만, Termux compatibility path가 namespace 격리를 제공하지 않는 상황에서는 DNS 차단을 보안 경계로 오해하게 만들 수 있다.

## 결과

DNS 진단은 외부 resolver 장애, sandbox network policy, Termux resolver path를 분리해야 한다. Wrapper로 실행한 profile은 network access가 켜져 있어야 하므로, 같은 오류가 반복되면 wrapper profile 경로를 타지 않았거나 opt-out 환경 변수가 설정된 상태부터 확인한다.
