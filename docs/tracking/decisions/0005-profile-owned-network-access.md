# Termux network와 approval 책임 경계

## 상황

Compatibility bwrap은 filesystem namespace 격리를 제공하지 않지만, upstream Codex는 network-off 실행에 seccomp socket 차단을 적용한다. Profile network와 approval 설정은 upstream Codex 사용자 상태다.

## 결정

Wrapper는 selected profile의 config를 수정하지 않는다. Network-off seccomp 경계를 보존하고, network 허용과 approval 요청 생성은 upstream Codex와 사용자가 결정한다. Wrapper doctor는 off/on/reset 실행 경로를 검증한다.

## 대안

Wrapper가 network access를 자동으로 켜면 반복 승인은 줄지만 유효한 seccomp 경계를 제거하고 사용자 설정을 변경한다. Wrapper가 approval 요청 생성을 흉내 내면 upstream agent loop와 책임이 충돌한다.

## 결과

Network-off는 제한적이지만 실제 경계다. Wrapper는 approval 생성을 보장하지 않으며 upstream approval 경로를 방해하지 않는 실행 호환성과 진단만 책임진다.
