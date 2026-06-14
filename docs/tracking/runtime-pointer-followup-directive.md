# Runtime Pointer Follow-up Directive

이 문서는 5.4 medium이 이어서 처리할 보조 작업 지시다. 현재 핵심 기틀은 `native/manager`, immutable `native/store/{runtime,raw}`, `native/current`, `native/verified`, `native/raw` pointer로 구현되어 있으므로, public command surface를 늘리거나 activation 모델을 다시 active directory mutation으로 되돌리면 안 된다.

## 유지해야 할 불변식

- 사용자가 알아야 하는 명령은 계속 `codex setup`, `codex update`, `codex use`, `codex doctor`, `codex version`, `codex remove`뿐이다.
- `bin/install-runtime.sh support|setup|update|doctor|remove`는 개발자/설치용 표면으로 유지하되, 새 public subcommand를 추가하지 않는다.
- `native/manager`는 wrapper support file의 소유자다. Runtime tuple에는 upstream payload와 runtime-private `codex-path/bwrap`, `codex-path/rg`만 들어가야 한다.
- `native/store/runtime/<tuple>`과 `native/store/raw/<tuple>`은 activation 후 수정하지 않는다. 새 activation은 새 tuple을 만들고 pointer를 바꿔야 한다.
- State/registry 변경, current/verified/raw pointer 변경은 실패 시 rollback 가능해야 한다.
- Upstream raw binary 원본성과 DNS-only byte-length patch 정책은 유지한다.

## 우선 작업

1. Doctor detail 보강
   - `codex doctor`의 Termux Wrapper section에 `manager`, `current`, `verified`, `runtime store`, `raw store` path detail을 추가한다.
   - 기존 upstream 스타일의 표와 색상은 유지한다.
   - `--json`에는 path를 추가하되, 사용자-facing subcommand는 늘리지 않는다.
   - `17 ok · 0 idle · 0 warn · 0 fail ok` 양식은 유지한다. Check 수가 늘어나면 human summary와 `tests/doctor-contract.sh`를 같이 갱신한다.

2. 문서 sweep
   - `docs/operations.md`, `docs/security.md`, `docs/contracts.md`, `docs/business-rules.md`, `docs/engineering-notes.md`에서 legacy `native/runtime` active tree 전제를 찾아 새 pointer layout으로 고친다.
   - `native/runtime`은 legacy migration input으로만 설명하고, active runtime은 `native/current`라고 명시한다.
   - Termux bwrap compat가 namespace security boundary가 아니라는 기존 표현은 유지한다.

3. Migration 보강
   - 현재 migration은 live legacy runtime/raw를 current/verified/raw pointer로 흡수한다.
   - 추가로 기존 `$CODEX_NATIVE_STATE_DIR/store`에 남아 있던 cached runtime/raw를 새 `native/store`로 가져올지 검토한다.
   - 가져온다면 registry path를 새 위치로 rewrite하고, hash와 manifest가 맞는 entry만 migration한다.
   - 실패한 cache migration은 active runtime 실행을 막으면 안 된다. 실패 항목은 doctor warn 또는 tracking finding으로 남긴다.

4. Rollback edge test 추가
   - raw pointer replacement 실패 시 current/verified/state/registry가 원래 상태로 돌아가는 테스트를 추가한다.
   - verified pointer replacement 실패 시 current/state/registry가 원래 상태로 돌아가는 테스트를 추가한다.
   - `codex_prune_runtime_store`가 current와 verified symlink target을 절대 삭제하지 않는지 테스트한다.

5. Installer E2E 보강
   - `support` command가 npm/network를 호출하지 않고 manager와 public launcher만 갱신하는지 fixture test로 검증한다.
   - `setup` command가 legacy runtime이 있을 때 network 없이 migration으로 끝나는지 fixture test로 검증한다.
   - public launcher가 계속 `native/manager/managed.sh`를 실행하는지 marker와 target 문자열을 확인한다.

## 금지 사항

- Upstream Codex binary를 직접 수정하는 새 patch를 추가하지 않는다.
- `codex doctor --wrapper-*` 같은 public 서브 인자를 늘리지 않는다.
- profile config, auth file, approval/network policy를 migration이나 doctor 목적으로 수정하지 않는다.
- 기존 사용자의 비관리형 `$PREFIX/bin/codex` backup 정책을 약화하지 않는다.

## 완료 기준

- `bash -n lib/codex-termux-lib.sh bin/install-runtime.sh install.sh tests/*.sh`
- `git diff --check`
- `for f in tests/*.sh; do bash "$f"; done`
- live Termux 설치에서 `bash bin/install-runtime.sh setup`
- live Termux 설치에서 `bash bin/install-runtime.sh doctor --summary`
- live Termux 설치에서 `bash tests/network-boundary.sh`

Network나 live 설치 쓰기가 sandbox 정책으로 막히면 승인 요청으로 실행하고, 실패 원인이 sandbox인지 wrapper인지 분리해서 보고한다.
