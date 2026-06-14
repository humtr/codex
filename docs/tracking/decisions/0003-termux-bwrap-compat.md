# Termux bwrap 호환 실행

## 상황

Upstream Codex는 Linux sandbox 실행을 위해 bubblewrap을 찾는다. Android/Termux에서는 Linux namespace setup이 일반적으로 허용되지 않아 upstream bubblewrap 실행이 실패하거나 warning을 낸다.

## 결정

Runtime-private `codex-path/bwrap`에만 Termux compatibility launcher를 둔다. Codex runtime 실행 시 이 directory를 PATH 앞에 두며, launcher는 namespace/mount option을 보안 경계로 구현하지 않고 command 실행에 필요한 env/cwd/argv option을 적용한 뒤 inner command를 실행한다.

## 대안

Real bubblewrap만 허용하면 Termux에서 기본 command execution이 깨진다. Warning을 그대로 노출하면 사용자는 매 실행마다 해결할 수 없는 Linux namespace 문제를 보게 된다. Namespace 격리를 제공한다고 흉내 내는 방법은 실제 보안 보장을 속이는 결과가 된다.

## 결과

Termux에서 Codex command execution은 유지되고 `$PREFIX/bin`에는 bwrap을 추가하지 않는다. 대신 이 repo는 Linux namespace isolation을 제공하지 않는다는 점을 명시해야 한다.
