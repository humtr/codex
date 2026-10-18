"""Ask the current candidate controller to show its Linux browser."""
import json
from pathlib import Path
import sys
from linux_desktop_trial import private, write
from open_dual_trial import record, owner


def main():
    data = Path.home()/'.local/share/codex-web-experiment/linux-desktop'
    assert len(sys.argv) in (1, 2)
    marker = Path(sys.argv[1]) if len(sys.argv) == 2 else data/'session.json'
    state = record(marker); owner(state)
    root = private(Path(state['root']))
    write(root/'open-browser.json', {'show': True})
    print(json.dumps({'linuxBrowserRequested': True}))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
