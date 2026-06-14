# Codex Termux Standalone Upstream Wrapper

Termux에서 공식 upstream Codex Linux ARM64 npm 패키지를 사용자 홈 아래의 관리형 런타임으로 설치하고 실행하는 래퍼다. Android/Termux의 제약을 숨기지 않고, 공식 바이너리의 리소스 배치를 유지한 채 실행 가능한 최소 변경만 적용한다.

```
project-root/
├── CLAUDE.md -> Claude Code 진입 지침
├── AGENTS.md -> Codex 진입 지침
├── docs/
│   ├── architecture.md -> 설치, 패치, 실행 흐름
│   ├── business-rules.md -> 래퍼가 지켜야 하는 동작 규칙
│   ├── security.md -> 보호 대상과 네트워크·격리 정책
│   ├── standards.md -> 변경 시 반드시 지킬 규칙
│   ├── engineering-notes.md -> 디버깅으로 확인된 함정과 대응
│   ├── operations.md -> 설치, 업데이트, 검증 절차
│   ├── contracts.md -> 사용자가 호출하는 CLI 계약
│   └── tracking/
│       ├── status.md -> 현재 구현 상태와 남은 일
│       ├── decisions/
│       │   ├── index.md -> 주요 결정 목록
│       │   ├── 0001-official-linux-arm64-runtime.md -> 공식 런타임 래핑 결정
│       │   ├── 0002-fd33-resolver-path.md -> DNS resolver 경로 패치 결정
│       │   ├── 0003-termux-bwrap-compat.md -> Termux bwrap 호환 경로 결정
│       │   ├── 0004-shared-profile-plugins.md -> 프로필 플러그인 공유 결정
│       │   └── 0005-profile-owned-network-access.md -> Termux profile 네트워크 기본값 결정
│       └── findings.md -> 현재 해결하지 못한 문제
├── bin/
│   └── AGENTS.md -> 설치 런타임 명령 경계
├── config/
│   └── AGENTS.md -> 래퍼 버전 메타데이터 경계
├── lib/
│   └── AGENTS.md -> 상태 관리와 실행 정책 경계
└── tools/
    └── AGENTS.md -> 런타임 빌드와 호환 도구 경계
```

## 절대 조건

- 공식 `@openai/codex` Linux ARM64 패키지가 원본이어야 하며 Android-native fork의 동작을 이 repo의 현재 목표로 섞지 않는다.
- 기존 Codex 인증·설정 상태와 비관리형 public launcher는 덮어쓰기 전에 보존한다.
- Termux용 `bwrap` 호환 경로는 Linux namespace 격리를 제공한다고 말하거나 문서화하지 않는다.
- Termux profile 실행은 사용자의 network access 설정을 보존한다. Wrapper가 profile config를 자동 변경하지 않는다.
- DNS 문제를 고칠 때는 resolver 파일, upstream sandbox 네트워크 정책, Android 실행 제한을 분리해 검증한다.

## 작업 전 확인

- 런타임 설치·업데이트·제거를 바꾸기 전에는 `lib/AGENTS.md`와 `bin/AGENTS.md`의 상태 파일, 백업, lock 규칙을 확인한다.
- 바이너리 패치나 `bwrap`/`rg` 지원 도구를 바꾸기 전에는 `tools/AGENTS.md`의 byte-length 패치, 실행 권한, smoke check 규칙을 확인한다.
- 네트워크나 DNS 증상을 다룰 때는 `docs/engineering-notes.md`의 DNS 항목을 먼저 확인하고, 샌드박스 차단과 외부 resolver 장애를 따로 증명한다.
- 프로필 동작을 바꿀 때는 기본 프로필 `~/.codex`, named profile `~/.codex-profiles/<name>`, 공유 플러그인 symlink의 기존 파일 보존 조건을 함께 확인한다.

## 문제 처리

사용자 인증·설정 손상, 비관리형 launcher 백업 누락, raw/runtime tuple 불일치, namespace 격리 제공에 대한 잘못된 주장, DNS 원인 오판은 즉시 사용자에게 보고한다. 그 외 반복 가능한 문제는 원인, 영향 범위, 지금 해결하지 못하는 이유를 `docs/tracking/findings.md`에 기록한다.
