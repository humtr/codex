# 공식 Linux ARM64 runtime 유지

## 상황

Termux에서 Codex를 실행하려면 Android-native fork를 쓰거나 공식 upstream Linux ARM64 package를 감싸는 방법이 있다. Android-native fork는 Termux에 더 자연스러운 바이너리를 만들 수 있지만 upstream package와 다른 제품 표면을 갖게 된다.

## 결정

이 repo는 공식 `@openai/codex` Linux ARM64 npm package를 raw 원본으로 사용하고, Termux에서 필요한 실행 호환만 wrapper와 runtime rebuild 단계에서 적용한다.

## 대안

Android-native fork를 원본으로 삼는 방법은 Termux-specific patch를 더 깊게 적용할 수 있지만, 공식 upstream package의 배포 경로와 hash provenance를 잃는다. Raw Linux binary를 그대로 실행하는 방법은 단순하지만 Termux resolver path, bwrap, tool path 차이를 처리하지 못한다.

## 결과

Runtime provenance는 npm package와 wrapper commit의 조합으로 설명된다. Android-native PTY, RUNPATH, bundled libc++ 같은 변경은 현재 repo의 일반 update 범위가 아니며, 필요하면 별도 packaging 전략으로 다뤄야 한다.
