# CLI 계약

## `codex`

입력: 인자 없음, upstream option, upstream command, 또는 prompt처럼 보이는 문자열.  
출력: upstream Codex의 stdout/stderr와 wrapper의 update 안내. bare 실행은 마지막으로 사용한 profile을 다시 사용하고, exec 직전에 선택된 profile 한 줄을 출력한다.
오류: runtime이 없고 cached raw도 없으면 `runtime missing and no cached raw package is available; run codex setup` 계열 오류를 낸다. 첫 인자가 upstream command가 아니고 option도 아니면 `exec <args>`로 변환해 upstream Codex를 호출한다. update prompt에서 `Esc`를 누르면 실행 전체를 취소한다.

## `codex -- <args>`

입력: `--` 뒤의 인자 전체.  
출력: wrapper 라우팅 없이 upstream Codex 실행 결과.  
오류: runtime 준비 실패는 일반 `codex`와 같다. `--` 뒤 인자는 prompt 라우팅 대상이 아니므로 사용자가 upstream contract를 직접 책임진다.

## `codex setup [version]`

입력: 선택적 upstream version 또는 dist-tag 의미의 빈 값.  
출력: manager support file과 launcher를 갱신하고, legacy layout이 있으면 current/verified/raw pointer layout으로 이관한 뒤, healthy runtime이 없을 때만 cached raw repair 또는 install을 수행하고 version 정보를 출력한다. setup 완료 뒤 public launcher의 `version` 확인과 wrapper doctor 검증이 이어진다.
오류: Termux dependency나 package fetch가 실패하면 setup은 runtime을 추측해 만들지 않고 실패한다.

## `codex update [version]`

입력: 선택적 version. `0.137.0`처럼 숫자 version만 주면 Linux ARM64 package spec으로 정규화한다.  
출력: package fetch, raw/runtime staging, smoke test, immutable store publish, current/verified/raw pointer와 state/registry의 rollback-safe activation 후 installed version을 출력한다. interactive `codex update`는 성공 뒤 새 runtime을 바로 실행할지 묻고, yes면 최근 profile의 bare 실행 경로로 넘긴다.
오류: npm pack 실패, tar extract 실패, raw vendor 필수 path 누락, tarball safety 실패, binary patch 실패, smoke test 실패, promotion rollback 실패 중 하나가 발생하면 기존 runtime을 성공으로 표시하지 않는다.

## `codex doctor [--json|--summary|--all]`

입력: option이 없으면 combined human doctor, option이 있으면 upstream doctor option.
출력: option이 없으면 upstream human doctor 뒤에 upstream doctor와 비슷한 section/row 형태의 `Termux Wrapper Doctor` 진단을 출력한다. wrapper human report는 manager, current/verified/raw pointer, runtime/raw store, alignment, sandbox, legacy store migration 상태를 보여 준다. `--json`, `--summary`, `--all` 등 option이 있으면 upstream 출력과 exit code를 그대로 반환한다. Wrapper machine report는 개발자용 `bash bin/install-runtime.sh doctor --json`으로 접근하며 schema 4 path/check/migration 계약을 제공한다.
오류: 기본 doctor는 upstream 또는 wrapper 필수 검사 중 하나라도 실패하면 실패한다. 상위 sandbox 때문에 network 경계를 검증할 수 없으면 wrapper report는 `inconclusive`로 기록하지만 그 이유만으로 실패하지 않는다. legacy migration이 `pending` 또는 `issues`여도 필수 check가 건강하면 wrapper human doctor는 warning만 표시하고 성공할 수 있다.

## `codex use [--list|selection]`

입력: `--list`, 숫자 선택, version, runtime hash prefix. interactive menu에서 현재 active runtime이 upstream latest가 아니면 `0`은 latest target을 뜻한다. latest가 아직 local cache에 없으면 install/update를 수행하고, 이미 cached면 그 latest cached runtime을 고른다. `1..n`은 나머지 cached runtime이다.
출력: cached runtime 목록 또는 선택한 runtime artifact의 pointer activation 결과와 version 정보. human menu는 latest target row를 맨 위 `0`으로, 나머지 cached runtime을 그 아래 `1..n`으로 렌더링한다. version 표시는 `-linux-arm64` suffix를 숨긴다. latest가 local cache에 없을 때 `0` row badge는 `update`다.
오류: 선택값이 cached runtime이나 remote latest와 맞지 않으면 `unknown cached runtime selection` 계열 오류를 낸다.

## `codex profile [name] [args...]`

입력: profile name이 없으면 interactive list, 있으면 `default` 또는 named profile. profile 뒤의 args는 upstream Codex args다.  
출력: 선택형 실행에서는 profile 목록을 stderr에 보여준다. human menu에서 `default` profile label은 `default`로 보이고 `0`에 배치된다. 마지막으로 사용한 profile은 `recent`로 표시한다. profile 실행은 `CODEX_HOME`을 해당 profile directory로 설정한 upstream Codex 결과를 출력한다.
오류: profile name에 slash, dot-prefix, whitespace, `native`, option-like prefix가 있으면 invalid profile이다. named profile directory가 없으면 실행하지 않는다.

## `codex remove`

입력: 없음.  
출력: 관리형 launcher와 managed native root(manager/store/current/verified/raw) 제거 결과와 backup 복구 메시지.
오류: marker 없는 launcher는 직접 삭제 대상이 아니다. backup이 없으면 해당 launcher는 복구하지 않는다.

## `bash install.sh [version]`

입력: 선택적 version.  
출력: dependency check, dependency install, managed runtime setup, public launcher `version` 검증, wrapper doctor 검증 진행 메시지.
오류: Termux가 아니거나 `$PREFIX/bin/pkg`가 없으면 시작 전에 실패한다.

## `bash bin/install-runtime.sh <command>`

입력: `setup`, `support`, `update`, `remove`, `doctor`. `doctor --json`은 wrapper machine report다.
출력: public `codex` wrapper command와 같은 내부 작업 결과.  
오류: 알 수 없는 command는 usage를 출력하고 exit code 2로 실패한다.
