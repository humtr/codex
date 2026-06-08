# 프로필 plugin 공유

## 상황

Codex profile을 `CODEX_HOME`으로 분리하면 named profile마다 plugin directory도 분리된다. 사용자는 기본 profile에 설치한 plugin을 여러 profile에서 그대로 쓰고 싶어 한다.

## 결정

Named profile에 `plugins` 항목이 없으면 `~/.codex/plugins`를 가리키는 symlink를 만든다. 기존 `plugins` file, directory, symlink가 있으면 profile-local 선택으로 보고 그대로 둔다.

## 대안

모든 profile에 plugin을 복사하는 방법은 update와 제거가 profile마다 갈라져 drift를 만든다. 기존 profile-local plugin directory를 symlink로 교체하는 방법은 사용자가 의도적으로 분리한 plugin 상태를 손상시킨다. 아무것도 하지 않는 방법은 기본 plugin 설치를 named profile에서 다시 반복하게 만든다.

## 결과

새 named profile은 기본 plugin cache와 skill을 공유한다. Profile-local plugin을 원하는 사용자는 `plugins` 항목을 먼저 만들어 symlink 생성을 막을 수 있다.
