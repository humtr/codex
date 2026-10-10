#!/usr/bin/env python3
"""Normalize only public release workspace versions; retain every external binding."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import tomllib


def normalize(source: Path) -> dict:
    manifest = tomllib.loads((source / 'Cargo.toml').read_text())
    target = manifest['workspace']['package']['version']
    if target != '0.161.0':
        raise ValueError('unexpected upstream release version')
    for member in manifest['workspace']['members']:
        paths = list(source.glob(member))
        if not paths or any(not (p / 'Cargo.toml').is_file() for p in paths):
            raise ValueError('missing workspace member')
    names = set()
    # Release local crates include path dependencies omitted from members.
    for path in sorted(source.rglob('Cargo.toml')):
        if path.relative_to(source).parts[0] == 'target':
            continue
        package = tomllib.loads(path.read_text()).get('package', {})
        if package.get('version') == {'workspace': True}:
            if package['name'] in names:
                raise ValueError('duplicate workspace package')
            names.add(package['name'])
    lock = source / 'Cargo.lock'
    before = lock.read_bytes()
    old = tomllib.loads(before.decode())
    selected = {p['name'] for p in old['package'] if 'source' not in p and p['name'] in names}
    if selected != names:
        raise ValueError('workspace lock inventory mismatch')
    if any(p['version'] != '0.0.0' for p in old['package'] if 'source' not in p and p['name'] in names):
        raise ValueError('unexpected workspace lock version')
    blocks = re.split(r'(?m)(?=^\[\[package\]\]$)', before.decode())
    changed = 0
    for i, block in enumerate(blocks):
        if not block.startswith('[[package]]'):
            continue
        package = tomllib.loads(block)['package'][0]
        if 'source' not in package and package['name'] in names:
            block, count = re.subn(r'(?m)^version = "0\.0\.0"$', f'version = "{target}"', block)
            if count != 1:
                raise ValueError('nonunique workspace version')
            changed += 1
        if 'source' not in package:
            for name in sorted(names):
                block = block.replace(f'"{name} 0.0.0"', f'"{name} {target}"')
        blocks[i] = block
    after = ''.join(blocks).encode()
    expected = copy.deepcopy(old)
    for p in expected['package']:
        if 'source' not in p and p['name'] in names:
            p['version'] = target
        if 'source' not in p and 'dependencies' in p:
            p['dependencies'] = [f'{s.rsplit(" ",1)[0]} {target}' if s.rsplit(' ',1)[0] in names and s.endswith(' 0.0.0') else s for s in p['dependencies']]
    if tomllib.loads(after.decode()) != expected or changed != len(names):
        raise ValueError('normalization changed unrelated lock data')
    lock.write_bytes(after)
    return {'original_lock_sha256': hashlib.sha256(before).hexdigest(), 'normalized_lock_sha256': hashlib.sha256(after).hexdigest(), 'workspace_versions_changed': changed, 'external_packages_unchanged': sum('source' in p for p in old['package'])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('source', type=Path)
    parser.add_argument('--audit', type=Path, required=True)
    args = parser.parse_args()
    result = normalize(args.source)
    args.audit.write_text(json.dumps(result, sort_keys=True)+'\n')
    print(json.dumps(result, sort_keys=True))
