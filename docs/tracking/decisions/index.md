# 결정 목록

- `0001-official-linux-arm64-runtime.md`: 공식 Linux ARM64 package를 원본으로 유지하고 Termux wrapper가 runtime을 관리한다.
- `0002-fd33-resolver-path.md`: musl resolver path를 fd 33으로 바꿔 Termux resolver file을 연결한다.
- `0003-termux-bwrap-compat.md`: Android에서 namespace 격리를 주장하지 않는 `bwrap` 호환 실행 경로를 둔다.
- `0004-shared-profile-plugins.md`: named profile의 plugin path가 없을 때 default plugin directory를 공유한다.
- `0005-profile-owned-network-access.md`: Termux profile 실행은 network access를 켠 상태를 기본값으로 보장하되 opt-out을 둔다.
