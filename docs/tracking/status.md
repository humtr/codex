# 현재 상태

## 구현됨

- 공식 `@openai/codex` Linux ARM64 package를 npm에서 받아 raw vendor tree로 보관하고, Termux용 runtime tree로 rebuild하는 경로가 구현되어 있다.
- Runtime rebuild는 resolver path rewrite, support `bwrap`/`rg` tool 설치, package metadata 복사, executable permission 보정을 수행한다.
- Public `$PREFIX/bin/codex` launcher만 설치하며, bwrap compatibility launcher는 runtime-private `codex-path/bwrap`에 둔다.
- Wrapper support file은 `native/manager`에 설치하고, active runtime은 `native/current` pointer가 immutable `native/store/runtime/<tuple>`을 가리키는 방식으로 실행한다.
- Last-known-good runtime은 `native/verified` pointer로 보존하며, readiness 실패 시 verified pointer rollback을 시도한다.
- 기존 `native/runtime` legacy layout은 setup 또는 readiness 경로에서 current/verified pointer layout으로 migration한다.
- `state.json`과 `registry.json`은 raw package, wrapper version/commit, runtime hash, active tuple을 기록한다.
- `state.json`은 verified tuple도 기록해 last-known-good smoke-tested runtime을 보호한다.
- `codex setup`, `update`, `doctor`, `version`, `help`, `use`, `profile`, `remove`, `--` passthrough가 구현되어 있다.
- Auto-update check는 interactive 실행에서 prompt를 띄우고, non-interactive 실행에서는 prompt 없이 현재 runtime을 계속 실행한다.
- Runtime drift repair는 cached raw package가 있을 때 support tool mismatch를 runtime rebuild와 pointer activation으로 복구한다.
- Named profile의 `plugins` 항목이 없으면 default `~/.codex/plugins`를 공유 symlink로 연결한다.
- Termux profile 실행은 profile config와 upstream network·approval policy를 변경하지 않는다.
- Network-off seccomp와 명시적 network-on/reset 경로를 wrapper doctor와 회귀 테스트로 검증한다.
- Runtime binary는 DNS-only patch manifest와 실제 hash를 매 실행 전 검증하고 compatible cache는 최신 세 개만 보존한다.
- Immutable store tuple collision은 기존 artifact를 보존한 채 activation을 거부한다.
- Prune는 current/verified/raw pointer target을 직접 보호한다.
- Legacy state-side store cache는 validation 통과 entry만 새 immutable store로 best-effort migration한다.
- Wrapper human doctor는 manager/current/verified/raw/store/migration 상태를 upstream 형식에 맞춰 표시한다.
- Phase 0 refactor guardrail로 `tools/codex_native` internal Python package와 `ci/check-structure.sh`/`ci/check-python-imports.py`가 추가됐다.
- Phase 1 refactor로 state/registry schema SSOT가 `tools/codex_native/schemas.py`로 이동했고, state/registry read/write/record/lookup shell 함수는 internal Python CLI facade를 사용한다.
- Phase 2 refactor로 file SHA-256, tree digest, immutable publish, runtime/raw artifact validation이 `hashing.py`와 `store.py`로 이동했다. Shell과 legacy migration은 같은 Python 구현을 호출한다.
- State/registry schema mismatch와 malformed JSON은 fail-fast이며, immutable collision은 기존 target을 보존하고 실패한다.
- Phase 3 refactor로 current/verified/raw pointer, state, registry activation transaction이 `tools/codex_native/activation.py`로 이동했다.
- `codex_activate_tuple_unlocked`, `codex_commit_runtime_candidate`, `codex_try_verified_rollback_unlocked`, `codex_refresh_runtime_metadata_unlocked`, `codex_bootstrap_store_unlocked`는 activation engine facade로 축소됐다.
- Activation engine은 성공한 단계만 역순 rollback하며 rollback cleanup/snapshot restore 실패를 aggregate error로 반환한다.
- Legacy runtime/raw directory는 artifact validation을 통과한 경우 transaction backup 뒤 pointer layout으로 승격된다.
- Phase 4 refactor로 prune plan/apply가 `tools/codex_native/prune.py`로 이동했고, shell prune은 internal CLI facade를 사용한다.
- Phase 5 refactor로 legacy store migration engine이 `tools/codex_native/migration.py`로 이동했고, migration report schema는 `schemas.py` 기준을 사용한다.
- Phase 6 refactor로 wrapper doctor machine report와 human renderer가 `tools/codex_native/doctor_report.py`, `tools/codex_native/doctor_render.py`로 이동했다.
- Phase 7 refactor로 cached runtime filtering/selection이 `registry.py`와 `use.py`로 이동했고, shell `codex use`는 internal CLI facade를 사용한다.
- Phase 8 refactor로 shell의 package-field, runtime/raw integrity, metadata-current, upstream command parsing, doctor/network JSON rendering embedded Python이 internal CLI로 이동했다.
- `lib/codex-termux-lib.sh`는 `codex-termux-runtime.sh`, `codex-termux-interactive.sh`를 source하는 구조로 분리돼 900줄 이하로 축소됐다.

## 검증됨

- 최근 작업 범위에서 `bash -n lib/codex-termux-lib.sh`가 통과했다.
- `bash tests/profile-behavior.sh`가 profile config 비변경과 plugin 공유를 검증한다.
- `bash tests/network-boundary.sh`, `bash tests/runtime-integrity.sh`, `bash tests/doctor-contract.sh`가 network, runtime, public doctor 계약을 검증한다.
- `bash tests/pointer-activation.sh`, `bash tests/verified-rollback.sh`, `bash tests/legacy-migration.sh`가 current/verified/raw pointer activation, last-known-good rollback, legacy layout migration을 검증한다.
- `bash tests/transactional-update.sh`, `bash tests/runtime-smoke.sh`, `bash tests/auto-update-failure.sh`, `bash tests/launcher-transaction.sh`, `bash tests/tarball-safety.sh`, `bash tests/lock-behavior.sh`, `bash tests/install-verification.sh`, `bash tests/use-cache-activation.sh`가 rollback, smoke test, auto-update isolation, launcher safety, tarball safety, lock fallback, install verification, cached raw/runtime activation을 검증한다.
- `bash tests/immutable-store.sh`, `bash tests/prune-pointer-protection.sh`, `bash tests/pointer-rollback.sh`, `bash tests/legacy-store-migration.sh`, `bash tests/installer-layout.sh`가 immutable publish collision, pointer target prune protection, raw/verified rollback, legacy store migration, support/setup orchestration을 검증한다.
- 최근 작업 범위에서 `git diff --check`가 통과했다.
- `bash ci/check-structure.sh`가 shell syntax, Python compile, import boundary, 초기 size guardrail, C launcher syntax, diff whitespace를 검증한다.
- `bash tests/state-registry.sh`가 direct CLI와 shell facade의 state/registry strict validation과 malformed registry 보존을 검증한다.
- `bash tests/immutable-store.sh`가 identical reuse, content/permission/symlink collision, special-file rejection, identical/different concurrent publish를 검증한다.
- Phase 1과 Phase 2 Verification Agent가 각 phase gate와 독립 failure-path probe를 pass로 확인했다.
- Phase 3 gate인 `bash ci/check-structure.sh`, `bash tests/pointer-activation.sh`, `bash tests/pointer-rollback.sh`, `bash tests/verified-rollback.sh`, `bash tests/transactional-update.sh`, `bash tests/use-cache-activation.sh`가 통과했다.
- 최근 전체 `for f in tests/*.sh; do bash "$f"; done`가 통과했다.
- 최근 live 설치에서 `codex doctor`의 overall status가 ok로 확인됐다.
- DNS 문제는 sandbox 밖 resolver query와 curl이 성공했고, sandbox 안에서는 network denial이 DNS 실패처럼 보이는 것으로 분리 확인됐다.

## 남은 일

- Profile 동작, runtime-private bwrap 경로, pointer activation, verified rollback, legacy migration, immutable publish, pointer-target prune protection, installer orchestration에는 자동화된 regression test가 있다. 남은 검증은 최종 구조 게이트와 전체 suite 재실행, live `codex doctor` 확인이다.
- Phase 9 test refactor는 진행 중이다. `tests/fixtures/` landing zone은 추가됐지만 내부 shell override 축소는 아직 일부 남아 있다.
- `BuildManifestV2`와 `DoctorReportV4`는 현재 runtime/doctor contract에서 사용되지만 별도 validator coverage와 fixture dedup는 추가 보강 여지가 있다.
- Immutable store의 `EXDEV` cross-device publish fallback은 구현돼 있으나 이를 강제로 실행하는 직접 회귀 테스트는 아직 없다.
- 일반 Linux에서 real bubblewrap 격리를 지원하는 별도 mode는 이 repo의 현재 범위가 아니다. 필요하면 Termux compat 경로와 분리된 새 정책 결정이 먼저 필요하다.
- Approval 요청 생성 여부는 upstream Codex agent 동작이며 wrapper 회귀 테스트 범위 밖이다.

## 현재 차이

Wrapper source version은 `1.1.0`이다. Profile config 비변경, runtime-private bwrap, DNS-only runtime 무결성, cache retention, network 경계, public doctor 합성 계약, current/verified/raw pointer activation, verified rollback, legacy migration, immutable store collision, prune pointer protection, installer orchestration을 각각 회귀 테스트한다.
