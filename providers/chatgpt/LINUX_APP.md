# Linux ChatGPT 앱 운영

Linux 앱을 열거나 로그인·화면 연결을 복구할 때 따르는 운영 문서다.
[SPEC.md](../../SPEC.md)가 제품 계약과 책임 경계를,
[CURRENT.md](../../CURRENT.md)가 실제 운영 상태와 미완료 작업을 소유한다.

## 기존 환경을 먼저 사용한다

모바일 화면 크기, ChatGPT/Chromium 태스크바, Android 브라우저 로그인 연동은
이미 구현되어 있다. 앱을 띄우라는 요청에는 기존 런처를 실행한다. 새 데스크톱,
축소된 Xvnc/앱 전용 런처, 별도 창 감시·재시도 경로를 만들지 않는다.
실행만 요청받았다면 이미 통과한 검증을 처음부터 반복하지 않는다.

설치된 런처와 저장된 프로필을 확인하기 전에 재설치하거나 빈 홈을 만들지 않는다.
새 홈의 로그인 정보가 비어 있다는 사실은 기존 계정이 로그아웃됐다는 증거가 아니다.
기존 경로를 바꿔야 한다면 재사용으로 해결할 수 없는 원인을 먼저 밝히고,
화면·태스크바·로그인·저장 프로필의 동작을 함께 보존한다.

## 실행 코드와 저장 위치

설치 경로는 현재 runtime 설정에서 읽는다. 설치 디렉터리 이름이나 임시 PID를
최신 환경으로 추정하지 않는다.

| 항목 | 위치 또는 담당 코드 |
| --- | --- |
| 기존 런처 선택과 의존성 경로 | `~/.local/share/codex-web-experiment/native-provider/runtime.json` |
| 데스크톱 시작·재사용 | 선택된 `source`의 `desktop_lifecycle.py` |
| Xvnc, Openbox, tint2, 앱, Chromium, Android 로그인 전달 | `linux_desktop_trial.py` |
| 현재 화면 열기 | `open_dual_trial.py linux`, `open_viewer.py` |
| 태스크바의 Chromium 열기 | `open_linux_browser.py` |
| 모바일 키보드·터치·화면 조작 | `mobile_controls.js`, `mobile_controls.css`, `termux_viewer.ts` |
| 현재 데스크톱 소유자와 임시 경로 | `~/.local/share/codex-web-experiment/linux-desktop/session.json` |
| 저장된 Linux 계정 홈 | `~/.local/share/codex-web-experiment/linux-desktop/home` |

선택된 소스는 보통 `~/.local/lib/codex-web-experiment/*/source`에 있다.
Git의 기존 구현 경로는 `experiments/web-provider`이며, 설치 기록의 `sourceBase`로
해당 원본을 찾을 수 있다. 현재 체크아웃에 파일이 없다는 이유로 재구현하지 않는다.
설치 payload의 원본과 매니페스트를 보존하고, 유지보수 변경은 원본에 남긴다.

## 앱 화면 열기

현재 설정이 가리키는 기존 진입점을 호출한다. 살아 있는 데스크톱은 재사용하며
사용자 프로필을 초기화하지 않는다.

```sh
python3 - <<'PY'
import json
from pathlib import Path
import subprocess
import sys

runtime = Path.home() / '.local/share/codex-web-experiment/native-provider/runtime.json'
source = Path(json.loads(runtime.read_text())['source'])
subprocess.run([sys.executable, '-B', str(source / 'desktop_lifecycle.py'),
                'desktop', '--runtime', str(runtime)], check=True)
subprocess.run([sys.executable, '-B', str(source / 'open_dual_trial.py'),
                'linux'], check=True)
PY
```

Chromium은 기존 태스크바의 **Browser** 버튼으로 연다. 태스크바에서 앱과
Chromium을 선택한다. noVNC 접속만 끊겼다면 같은 데스크톱의 화면을 다시 연다.
Android 화면 열기 요청의 성공과 사용자가 실제 화면을 본 사실은 구분해 보고한다.

## 로그인과 계정

저장된 앱 프로필은 계정 홈의 `profile`, Linux Chromium 프로필은
`browser-profile`에 있다. `.codex/auth.json`도 기존 홈의 계정 상태 일부다.
저장된 계정 정보와 실제 앱의 현재 로그인 상태를 함께 확인한다.
계정별 모델 권한을 확인하며, 서로 다른 계정의 프로필이나 인증 정보를 합치지 않는다.
터널 관리용 키와 Codex/Manager 프로필 선택은 Linux 앱 로그인을 대신하지 않는다.

앱의 로그인 동작은 기존 `/ui/bin/xdg-open`과 `BROWSER` 연동을 통해
Android 브라우저에 전달된다. 이 연결을 복구할 때 기존 런처의 마운트·환경 설정을
사용한다. 인증 URL, 토큰, 쿠키, 개인 대화 내용을 출력하거나 문서에 기록하지 않는다.

## noVNC 화면 설정

화면 크기와 창 배치는 기존 모바일 런처와 noVNC의 resize 기능을 사용한다.
현재 모바일 화면은 noVNC 초기화에서 기존 `UI.toggleControlbarSide()`를 사용해
사이드바를 오른쪽에서 시작한다. 화면을 사용하면서 핸들을 반대쪽 가장자리로
옮길 수 있으며, noVNC가 `controlbar_pos`를 저장한다. 위치 변경은 같은 초기화
지점에서 처리하며, 별도의 CSS 반전이나 위치 감시기를 만들지 않는다.
이 화면 설정 때문에 Linux 앱이나 계정 홈을 다시 만들지 않는다.

## Provider와 연결할 때

기존 런처는 앱과 데스크톱 수명을 소유하고, Provider는 Chat 연결과 추론 전달을,
Native는 도구 실행·승인·정식 작업 기록을 소유한다.
현재 세션의 `root`에서 `desktop-host.json`을 읽고 기존 소유자 검증을 거쳐 연결한다.
데스크톱을 다시 시작했다면 이전 임시 descriptor나 PID를 재사용하지 않는다.
복구용 명시적 descriptor 연결을 Native/MCP의 영구적인 데스크톱 부팅 의존성으로
확장하지 않는다.

Native writer, Bridge controller, 도구 효과 예약, Linux 앱의 CDP 연결은 서로
다른 소유권이다. 목록의 reconciliation_required만으로 살아 있는 잠금을
단정하지 않는다. Native 조회가 실패한 실행 전 할당을 정리하지 못하는 문제는
Provider–Native의 수명 계약에서 확인한다. 앱 재로그인이나 새 데스크톱 생성으로
해결하려 하지 않는다. 상태 기록만 강제 삭제해 미확정 효과를 성공으로 바꾸지 않는다.

기존 외부 CDP 릴레이는 한 번에 한 관찰 연결을 받는다. 직접 CDP 새로고침과
Provider의 앱 관찰을 동시에 실행하지 않는다. 연결 점유를 앱 고장이나 로그아웃으로
판단하지 않는다. 연결성, 앱 로그인, 모델 권한, 커넥터 등록은 실제 실패 경계에서
각각 확인한다.

Core/Provider/MCP를 교체한 뒤에는 저장된 연결 설정과 현재 소유 세션으로 기존
`scripts/d3_refresh.mjs`를 사용한다. 앱 캐시 갱신과 전체 입력 스키마 일치를 확인하고,
새 Native 작업의 create/start/attach/execute/finish까지 이어 간다.
이 절차와 단순히 앱 화면을 여는 요청을 혼동하지 않는다.

운영 소스 정합성은 서비스만 확인하지 않는다. 현재 서비스 이미지, 해당 터널의
MCP 명령, 커넥터가 가리키는 터널과 Native가 확인한 실행 설정을 대조한다.
설치된 signed 패키지, 별도 개발 후보, 실제 활성 서비스는 서로 다른 사실이다.
기존 candidate inventory와 Git commit으로 시험한 바이너리를 식별하고, 실제
계정·터널 바인딩과 롤백 설정을 보존한다. 다른 계정의 서비스로 자동 전환하지 않는다.

새 로컬 MCP 프로세스와 원격 커넥터의 전체 스키마가 다르면, 옛 터널 세션의
카탈로그인지 먼저 확인한다. 활성 효과가 없고 같은 소유 터널임을 확인한 뒤
기존 supervisor로 해당 터널만 재연결하고 같은 `scripts/d3_refresh.mjs` 검사를
수행한다. 새 터널·커넥터·로그인을 만들거나 무조건 재시도하는 복구기를 넣지 않는다.
