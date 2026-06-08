# CLI 계약

## `codex`

입력: 인자 없음, upstream option, upstream command, 또는 prompt처럼 보이는 문자열.  
출력: upstream Codex의 stdout/stderr와 wrapper의 update 안내.  
오류: runtime이 없고 cached raw도 없으면 `runtime missing and no cached raw package is available; run codex setup` 계열 오류를 낸다. 첫 인자가 upstream command가 아니고 option도 아니면 `exec <args>`로 변환해 upstream Codex를 호출한다.

## `codex -- <args>`

입력: `--` 뒤의 인자 전체.  
출력: wrapper 라우팅 없이 upstream Codex 실행 결과.  
오류: runtime 준비 실패는 일반 `codex`와 같다. `--` 뒤 인자는 prompt 라우팅 대상이 아니므로 사용자가 upstream contract를 직접 책임진다.

## `codex setup [version]`

입력: 선택적 upstream version 또는 dist-tag 의미의 빈 값.  
출력: support file과 launcher를 갱신하고 runtime이 없으면 설치한 뒤 version 정보를 출력한다.  
오류: Termux dependency나 package fetch가 실패하면 setup은 runtime을 추측해 만들지 않고 실패한다.

## `codex update [version]`

입력: 선택적 version. `0.137.0`처럼 숫자 version만 주면 Linux ARM64 package spec으로 정규화한다.  
출력: npm package fetch, raw 저장, runtime rebuild, state/registry 기록 후 installed version을 출력한다.  
오류: npm pack 실패, tar extract 실패, raw vendor 필수 path 누락, binary patch 실패 중 하나가 발생하면 기존 runtime을 성공으로 표시하지 않는다.

## `codex doctor [--json|--upstream]`

입력: wrapper doctor option 또는 `--upstream` 뒤 upstream doctor 인자.  
출력: `--json`은 machine-readable JSON, option이 없으면 pretty JSON, `--upstream`은 upstream doctor 결과.  
오류: wrapper doctor의 `overallStatus`는 runtime, raw, bwrap, rg, resolver, cert, state, registry, DNS patch check 중 하나라도 실패하면 `fail`이다.

## `codex use [--list|selection]`

입력: `--list`, 숫자 선택, version, runtime hash prefix.  
출력: cached runtime 목록 또는 선택한 runtime promotion 결과와 version 정보.  
오류: 선택값이 cached runtime이나 remote latest와 맞지 않으면 `unknown cached runtime selection` 계열 오류를 낸다.

## `codex profile [name] [args...]`

입력: profile name이 없으면 interactive list, 있으면 `default` 또는 named profile. profile 뒤의 args는 upstream Codex args다.  
출력: 선택형 실행에서는 profile 목록을 stderr에 보여준다. profile 실행은 `CODEX_HOME`을 해당 profile directory로 설정한 upstream Codex 결과를 출력한다.  
오류: profile name에 slash, dot-prefix, whitespace, `native`, option-like prefix가 있으면 invalid profile이다. named profile directory가 없으면 실행하지 않는다.

## `codex remove`

입력: 없음.  
출력: 관리형 launcher/runtime 제거 결과와 backup 복구 메시지.  
오류: marker 없는 launcher는 직접 삭제 대상이 아니다. backup이 없으면 해당 launcher는 복구하지 않는다.

## `bash install.sh [version]`

입력: 선택적 version.  
출력: dependency check, dependency install, managed runtime setup 진행 메시지.  
오류: Termux가 아니거나 `$PREFIX/bin/pkg`가 없으면 시작 전에 실패한다.

## `bash bin/install-runtime.sh <command>`

입력: `setup`, `support`, `update`, `remove`, `doctor`.  
출력: public `codex` wrapper command와 같은 내부 작업 결과.  
오류: 알 수 없는 command는 usage를 출력하고 exit code 2로 실패한다.
