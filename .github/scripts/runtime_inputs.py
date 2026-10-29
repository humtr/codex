#!/usr/bin/env python3
"""CI input identities for qualified runtime reuse; compilation is Rust-owned."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

SOURCE = Path(__file__).resolve().parents[2]

ASSET_MANIFEST = 'release/runtime-build-inputs-v1'


def assets(contents):
    lines = contents.splitlines()
    if not lines or lines[0] != 'codex-runtime-build-inputs-v1':
        raise ValueError('unsupported runtime build input inventory')
    result = {}
    for line in lines[1:]:
        role, separator, name = line.partition('\t')
        path = Path(name)
        if (not separator or role not in {'recipe', 'input'} or name in result
                or path.is_absolute() or any(part in {'', '.', '..'} for part in name.split('/'))):
            raise ValueError('invalid runtime build input record')
        result[name] = role
    return result


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def cargo_dependencies(read, manifest):
    """The exact locked dependency closure of one local build tool/component."""
    lock = tomllib.loads(read('Cargo.lock').decode())['package']
    packages = {(p['name'], p['version'], p.get('source')): p for p in lock}
    if len(packages) != len(lock):
        raise ValueError('duplicate Cargo build package identity')
    root = tomllib.loads(read(manifest).decode())['package']
    pending = [(root['name'], root['version'], None)]
    selected = {}
    while pending:
        key = pending.pop()
        if key in selected:
            continue
        package = packages[key]
        selected[key] = package
        for dependency in package.get('dependencies', []):
            description, separator, origin = dependency.partition(' (')
            if separator and not origin.endswith(')'):
                raise ValueError('invalid Cargo build dependency source')
            parts = description.split()
            if len(parts) not in (1, 2):
                raise ValueError('invalid Cargo build dependency identity')
            matches = [candidate for candidate in packages if candidate[0] == parts[0]
                       and (len(parts) == 1 or candidate[1] == parts[1])
                       and (not separator or candidate[2] == origin[:-1])]
            if len(matches) != 1:
                raise ValueError('ambiguous Cargo build dependency closure')
            pending.append(matches[0])
    return selected


def locked_packages(selected):
    return sorted(selected.values(), key=lambda p: (p['name'], p['version'], p.get('source', '')))


def fingerprint(source, revision=None, recipe=None, adapted=False):
    """Build inputs, independent of a caller commit or unrelated Core/Manager code."""
    def read(name):
        if revision is not None:
            return subprocess.check_output(['git', '-C', str(source), 'show', revision + ':' + name],
                                           stderr=subprocess.DEVNULL)
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError('runtime build input is not a regular source file: ' + name)
        return path.read_bytes()
    value = recipe if recipe is not None else json.loads(read('release/upstream.json'))
    # Frontend run IDs and output digests are results, not runtime build inputs.
    inputs = {key: value[key] for key in ('upstream',)}
    inputs.update(rust=value['toolchains']['frontend_rust'],
                  lock_audit=value['qualification']['lock_audit'], files={name: hashlib.sha256(read(name)).hexdigest()
                                      for name, role in assets(read(ASSET_MANIFEST).decode()).items() if role == 'recipe'})
    inputs['producer_dependencies'] = locked_packages(cargo_dependencies(read, 'crates/release-builder/Cargo.toml'))
    if adapted:
        # Published runtime bytes also consume the Rust adaptation/descriptor
        # rules. Reusing an already-adapted Base must bind those crate inputs,
        # while the raw compiler cache remains independent of adapter changes.
        tree = subprocess.check_output(['git', '-C', str(source), 'ls-tree', '-r',
            revision or 'HEAD', '--', 'crates/release-builder', 'crates/runtime-binding'])
        inputs['adaptation'] = {'core_rust': value['toolchains']['core_rust'],
                               'sources': tree.decode().splitlines(),
                               'binding_dependencies': locked_packages(cargo_dependencies(read, 'crates/runtime-binding/Cargo.toml'))}
    return hashlib.sha256(canonical(inputs)).hexdigest()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    print(fingerprint(SOURCE))
