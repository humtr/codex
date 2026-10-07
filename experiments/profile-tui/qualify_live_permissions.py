"""Actual preview frontend permission menus and native stock-server settings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_permissions import qualify
from qualify_native_profile_display import preview_assets

if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    for name in ['core','native','manager','generation','parent']:parser.add_argument('--'+name,type=Path,required=True)
    a=parser.parse_args()
    def prepare(home,prefix):
        assets=preview_assets(home);assets.mkdir(parents=True,mode=0o700)
        for source,name in [(a.native,'native'),(a.manager,'manager'),(a.generation/'core','stable-core')]:
            shutil.copy2(source,assets/name);os.chmod(assets/name,0o755)
        os.symlink(shutil.which('openssl'),prefix/'bin/openssl')
    for mode in ['shared','named']:qualify(a.core.resolve(),a.generation.resolve(),a.parent.resolve(),mode,prepare_preview=prepare)
    print(json.dumps({'native_sha256':hashlib.sha256(a.native.read_bytes()).hexdigest(),'core_sha256':hashlib.sha256(a.core.read_bytes()).hexdigest(),'proof':'shared/named/actual-menu-four-selections/two-shortcuts/native-no-sandbox-reviewer-settings/no-model-turn'},sort_keys=True))
