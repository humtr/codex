# config

`config/`는 wrapper version metadata의 source file을 소유한다. 이 값은 support file 설치 때 runtime directory로 복사되고, state/registry에는 runtime을 만든 wrapper version으로 기록된다.

## 경계

- `wrapper-version.env`는 shell에서 source 가능한 `KEY=value` 형식이어야 한다.
- Runtime 설치 시점의 commit과 timestamp는 설치 스크립트가 덧붙인다.
- Upstream Codex version이나 npm package version은 이 파일의 소유가 아니다.

## 항상 지킬 것

- 값에 shell quoting이 필요한 문자를 넣지 않는다.
- Channel과 repo 이름은 registry에서 provenance를 읽는 사람이 source를 구분할 수 있게 유지한다.
- Version bump 없이 behavior를 바꿀 수는 있지만, 배포 가능한 wrapper 상태를 표시하려면 version을 함께 올린다.

## 변경 검증

`config/wrapper-version.env`를 바꾸면 `bash -n`으로 직접 검증할 수 없으므로, `bin/install-runtime.sh support` 또는 임시 runtime dir 설치에서 file이 source 가능한지 확인한다.
