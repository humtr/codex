"""Open only the authenticated native trial browser in the existing private viewer."""
import json
from pathlib import Path
import sys
from fit_browser import fit_browser
from open_viewer import viewer_url
import subprocess

root=Path(sys.argv[1]).resolve(strict=True)
env=json.loads((root/"environment.json").read_text())
fit_browser(root,owner_file="native-view-process.json")
url=viewer_url(json.loads((root/"viewer.json").read_text()))
result=subprocess.run([env["PREFIX"]+"/bin/termux-am-socket","start -a android.intent.action.VIEW -d "+url],env=env,capture_output=True,timeout=20)
print("native_viewer_activity_exit",result.returncode)
raise SystemExit(result.returncode)
