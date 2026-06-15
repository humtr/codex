# 운영 절차

## 초기 설치

Termux 안에서 실행해야 하며 `PREFIX`가 설정되어 있고 `$PREFIX/bin/pkg`가 실행 가능해야 한다.

```bash
bash install.sh
```

이 명령은 필요한 Termux package를 설치한 뒤 `bin/install-runtime.sh setup`을 실행한다. 의존성 설치가 먼저 끝나야 npm package fetch와 Python runtime build가 가능하므로 순서를 바꾸지 않는다.
설치가 끝나면 public `$PREFIX/bin/codex version`과 `bash bin/install-runtime.sh doctor --json` 검증이 뒤따른다.

## Support file만 갱신

```bash
bash bin/install-runtime.sh support
```

이 명령은 `native/manager` support file과 public Codex launcher만 갱신하며 upstream Codex package fetch, raw repair, runtime rebuild를 수행하지 않는다. `lib/`, `tools/`, launcher 관련 변경을 live 설치에 반영할 때 사용한다.

## Upstream runtime 업데이트

```bash
codex update
```

또는 repo에서 직접:

```bash
bash bin/install-runtime.sh update
```

명시 버전이 필요하면 `codex update 0.137.0`처럼 버전만 넘긴다. wrapper는 Linux ARM64 package spec으로 정규화한 뒤 npm package를 받고, raw vendor tree를 저장하고, runtime을 rebuild하고, smoke test를 통과한 artifact만 immutable store에 publish한다. 그 다음 current/verified/raw pointer와 state/registry metadata를 한 transaction으로 갱신하며, 중간 실패 시 active installation을 rollback한다.
interactive `codex update`는 단계별 진행 문구를 더 자세히 보여 주고, 성공 뒤에는 새 runtime을 바로 실행할지 물어본다.

## Runtime 선택

```bash
codex use --list
codex use 1
codex use 0
codex use 0.137.0

interactive `codex use`에서 현재 active가 latest가 아니면 `0`은 latest target을 뜻한다. latest가 아직 cached runtime으로 없으면 install/update를 수행하고, 이미 cached면 그 latest cached runtime으로 전환한다. `1..n`은 나머지 cached runtime이다.
```

cached runtime은 registry의 runtime/raw path가 각각 managed store의 직접 자식을 가리키고 검증 가능한 artifact일 때만 선택지에 남는다. remote latest를 선택하면 update와 같은 fetch/rebuild 경로를 탄다. human menu는 `-linux-arm64` suffix를 숨기고, local cache에 latest가 없을 때 `0` row를 `⬇ update`로 표시한다.

## Profile 실행

```bash
codex profile
codex profile api
codex profile default
```

`default`는 `~/.codex`를 쓰고 named profile은 `~/.codex-profiles/<name>`을 쓴다. interactive menu에서는 `default`가 `0`번이고, 마지막으로 사용한 profile은 `recent` badge로 표시된다. bare `codex` 실행은 마지막으로 사용한 profile을 다시 사용한다. named profile directory가 없으면 실행하지 않는다. named profile에 `plugins` 항목이 없으면 default plugin directory로 symlink를 만든다.

## 검증

전체 local 구조 검증:

```bash
bash ci/check-structure.sh
```

이 명령은 shell syntax, Python compile, `codex_native` import boundary, 초기 line/function size guardrail, C launcher syntax, diff whitespace를 한 번에 확인한다.

Shell syntax:

```bash
bash -n install.sh
bash -n bin/install-runtime.sh
bash -n lib/codex-termux-lib.sh
bash -n tests/profile-behavior.sh
bash -n tests/doctor-contract.sh
bash -n tests/installer-layout.sh
bash -n tests/network-boundary.sh
bash -n tests/runtime-integrity.sh
```

Profile 동작 회귀 테스트:

```bash
bash tests/profile-behavior.sh
bash tests/runtime-bwrap-path.sh
bash tests/doctor-contract.sh
bash tests/installer-layout.sh
bash tests/network-boundary.sh
bash tests/runtime-integrity.sh
```

Runtime 진단:

```bash
codex doctor --json
```

개발자용 wrapper 진단과 install regression:

```bash
bash tests/runtime-smoke.sh
bash tests/transactional-update.sh
bash tests/auto-update-failure.sh
bash tests/launcher-transaction.sh
bash tests/tarball-safety.sh
bash tests/lock-behavior.sh
bash tests/install-verification.sh
bash tests/use-cache-activation.sh
bash tests/immutable-store.sh
bash tests/prune-pointer-protection.sh
bash tests/pointer-rollback.sh
bash tests/legacy-store-migration.sh
bash tests/pointer-activation.sh
```

Repo diff hygiene:

```bash
git diff --check
```

`codex doctor`는 upstream 진단 뒤 wrapper 진단을 출력한다. 개발자용 `bash bin/install-runtime.sh doctor --json`은 runtime, manager, current/verified/raw pointer, runtime/raw store, registry/state alignment, build manifest, 실제 hash, DNS-only patch, runtime-private bwrap, network off/on/reset, legacy store migration report를 점검한다. 상위 sandbox가 baseline socket을 막으면 network boundary 결과는 `inconclusive`다. legacy store migration이 `pending` 또는 `issues`여도 active runtime과 필수 check가 건강하면 wrapper human doctor는 warning만 내고 계속 성공할 수 있다.

## 제거

```bash
codex remove
```

관리형 marker가 있는 `$PREFIX/bin/codex`만 제거하고, setup/update 때 보존한 Codex launcher backup이 있으면 복구한다. Managed native root의 manager/store/current/verified/raw layout은 제거하지만 state directory는 backup 추적을 위해 남긴다. Public `$PREFIX/bin/bwrap`은 이 wrapper의 관리 대상이 아니다.

## 주요 환경 변수

- `CODEX_NATIVE_HOME`: 관리형 runtime과 profile root의 기준 home이다.
- `CODEX_NATIVE_PREFIX`: Termux prefix이며 기본값은 `$PREFIX` 또는 `/data/data/com.termux/files/usr`다.
- `CODEX_NATIVE_MANAGER_DIR`: 설치된 manager support file 위치다.
- `CODEX_NATIVE_RUNTIME_DIR`: active runtime pointer 기본 경로다.
- `CODEX_NATIVE_VERIFIED_LINK`: last-known-good runtime pointer 기본 경로다.
- `CODEX_NATIVE_RAW_DIR`: active raw cache pointer 기본 경로다.
- `CODEX_NATIVE_STORE_DIR`: immutable runtime/raw store root다.
- `CODEX_NATIVE_LEGACY_STORE_DIR`: 예전 state-side cache store migration input 경로다.
- `CODEX_NATIVE_STORE_MIGRATION_REPORT`: legacy store migration report 경로다.
- `CODEX_NATIVE_AUTO_UPDATE`: `0`이면 auto-update check를 끈다.
- `CODEX_NATIVE_AUTO_UPDATE_MODE`: `prompt`, `force`, `off` 계열 값을 받는다.
- `CODEX_NATIVE_RUNTIME_RETENTION`: compatible cached runtime 보존 개수이며 기본값은 `3`이다.
- `CODEX_NATIVE_SHARED_PLUGINS_DIR`: named profile이 공유할 plugin directory다.
- `CODEX_NATIVE_RESOLV_CONF`: fd 33으로 열 resolver file path다.
