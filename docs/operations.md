# 운영 절차

## 초기 설치

Termux 안에서 실행해야 하며 `PREFIX`가 설정되어 있고 `$PREFIX/bin/pkg`가 실행 가능해야 한다.

```bash
bash install.sh
```

이 명령은 필요한 Termux package를 설치한 뒤 `bin/install-runtime.sh setup`을 실행한다. 의존성 설치가 먼저 끝나야 npm package fetch와 Python runtime build가 가능하므로 순서를 바꾸지 않는다.

## Support file만 갱신

```bash
bash bin/install-runtime.sh support
```

이 명령은 public Codex launcher와 runtime support scripts를 갱신하지만 upstream Codex package를 새로 받지 않는다. `lib/`, `tools/`, launcher 관련 변경을 live 설치에 반영할 때 사용한다.

## Upstream runtime 업데이트

```bash
codex update
```

또는 repo에서 직접:

```bash
bash bin/install-runtime.sh update
```

명시 버전이 필요하면 `codex update 0.137.0`처럼 버전만 넘긴다. wrapper는 Linux ARM64 package spec으로 정규화한 뒤 npm package를 받고, raw vendor tree를 저장하고, runtime을 rebuild하고, state/registry를 갱신한다.

## Runtime 선택

```bash
codex use --list
codex use 1
codex use 0.137.0
```

cached runtime은 registry의 runtime path가 실제 store root 아래에 있고 `codex` binary가 있을 때만 선택지에 남는다. remote latest를 선택하면 update와 같은 fetch/rebuild 경로를 탄다.

## Profile 실행

```bash
codex profile
codex profile api
codex profile default
```

`default`는 `~/.codex`를 쓰고 named profile은 `~/.codex-profiles/<name>`을 쓴다. named profile directory가 없으면 실행하지 않는다. named profile에 `plugins` 항목이 없으면 default plugin directory로 symlink를 만든다.

## 검증

Shell syntax:

```bash
bash -n install.sh
bash -n bin/install-runtime.sh
bash -n lib/codex-termux-lib.sh
bash -n tests/profile-behavior.sh
```

Profile 동작 회귀 테스트:

```bash
bash tests/profile-behavior.sh
bash tests/runtime-bwrap-path.sh
```

Runtime 진단:

```bash
codex doctor --json
```

Repo diff hygiene:

```bash
git diff --check
```

`doctor --json`에서 runtime, raw, runtime-private bwrap, rg, resolver, cert, state, registry, DNS patch 관련 check가 모두 true여야 live 설치가 정상이다. 네트워크 증상은 `doctor`만으로 결론 내리지 말고 sandbox 밖 DNS query와 profile network 설정을 같이 확인한다.

## 제거

```bash
codex remove
```

관리형 marker가 있는 `$PREFIX/bin/codex`만 제거하고, setup/update 때 보존한 Codex launcher backup이 있으면 복구한다. Runtime tree는 제거하지만 state directory는 backup 추적을 위해 남긴다. Public `$PREFIX/bin/bwrap`은 이 wrapper의 관리 대상이 아니다.

## 주요 환경 변수

- `CODEX_NATIVE_HOME`: 관리형 runtime과 profile root의 기준 home이다.
- `CODEX_NATIVE_PREFIX`: Termux prefix이며 기본값은 `$PREFIX` 또는 `/data/data/com.termux/files/usr`다.
- `CODEX_NATIVE_AUTO_UPDATE`: `0`이면 auto-update check를 끈다.
- `CODEX_NATIVE_AUTO_UPDATE_MODE`: `prompt`, `force`, `off` 계열 값을 받는다.
- `CODEX_NATIVE_PROFILE_NETWORK_ACCESS`: 기본값은 `1`이며 profile 실행 때 workspace-write network access를 true로 보장한다. `0`이면 profile config를 자동 변경하지 않는다.
- `CODEX_NATIVE_SHARED_PLUGINS_DIR`: named profile이 공유할 plugin directory다.
- `CODEX_NATIVE_RESOLV_CONF`: fd 33으로 열 resolver file path다.
