"""Validated build inputs, shared by preparation, frontend and release jobs.

This file supplies inputs, never qualification or publication authority. The
existing exact-run/source/artifact and Android gates must still accept them.
"""
import argparse
import json
from pathlib import Path
import re


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate release input key')
        result[key] = value
    return result


def exact(value, keys):
    if not isinstance(value, dict) or set(value) != set(keys):
        raise ValueError('unsupported release input fields')


def match(value, pattern):
    if not isinstance(value, str) or re.fullmatch(pattern, value) is None:
        raise ValueError('invalid release input')
    return value


def load(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 16384:
        raise ValueError('invalid release input file')
    value = json.loads(path.read_text(), object_pairs_hook=unique)
    exact(value, ['schema', 'upstream', 'toolchains', 'frontend', 'qualification'])
    if type(value['schema']) is not int or value['schema'] != 1:
        raise ValueError('unsupported release input schema')
    upstream, toolchains, frontend = (value[k] for k in ['upstream', 'toolchains', 'frontend'])
    exact(upstream, ['version', 'source', 'archive_sha256'])
    version = r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)'
    match(upstream['version'], version)
    match(upstream['source'], r'[0-9a-f]{40}')
    match(upstream['archive_sha256'], r'[0-9a-f]{64}')
    exact(toolchains, ['core_rust', 'frontend_rust'])
    for item in toolchains.values():
        match(item, version)
    if frontend is not None:
        exact(frontend, ['source', 'run_id', 'sha256', 'input_url', 'input_sha256'])
        match(frontend['source'], r'[0-9a-f]{40}')
        match(frontend['run_id'], r'[1-9][0-9]{0,19}')
        for field in ['sha256', 'input_sha256']:
            match(frontend[field], r'[0-9a-f]{64}')
        prefix = 'https://github.com/humtr/codex/releases/download/'
        match(frontend['input_url'], re.escape(prefix) + r'[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+\.tar\.gz')
    proof = value['qualification']
    exact(proof, ['focused_tests', 'lock_audit'])
    if type(proof['focused_tests']) is not int or not 0 < proof['focused_tests'] <= 100000:
        raise ValueError('native proof requires an executed test set')
    audit = proof['lock_audit']
    exact(audit, ['external_packages_unchanged', 'workspace_versions_changed',
                  'original_lock_sha256', 'normalized_lock_sha256'])
    for key in ['external_packages_unchanged', 'workspace_versions_changed']:
        if type(audit[key]) is not int or not 0 < audit[key] <= 100000:
            raise ValueError('invalid native dependency proof')
    for key in ['original_lock_sha256', 'normalized_lock_sha256']:
        match(audit[key], r'[0-9a-f]{64}')
    return value


def build_recipe(value):
    # Accepted artifact references are output, not inputs to their own build.
    # Updating run/digests must not invalidate a byte-identical source recipe.
    return {key: value[key] for key in ['schema', 'upstream', 'toolchains', 'qualification']}


def environment(value, component):
    upstream, frontend = value['upstream'], value['frontend']
    result = {'UPSTREAM_VERSION': upstream['version'], 'UPSTREAM_SHA': upstream['source'],
              'UPSTREAM_ARCHIVE_SHA256': upstream['archive_sha256'],
              'RUST_VERSION': value['toolchains']['frontend_rust' if component == 'frontend' else 'core_rust']}
    if component == 'frontend':
        result['RUSTUP_TOOLCHAIN'] = result['RUST_VERSION']
    if frontend is not None:
        result.update(MANAGER_TUI_VERSION=upstream['version'],
                      MANAGER_TUI_ARCHIVE_SHA256=upstream['archive_sha256'],
                      MANAGER_TUI_SOURCE_SHA=frontend['source'], MANAGER_TUI_RUN_ID=frontend['run_id'],
                      MANAGER_TUI_SHA256=frontend['sha256'], MANAGER_TUI_INPUT_URL=frontend['input_url'],
                      MANAGER_TUI_INPUT_SHA256=frontend['input_sha256'])
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path)
    parser.add_argument('--component', choices=['core', 'frontend'], required=True)
    args = parser.parse_args()
    try:
        for key, value in environment(load(args.path), args.component).items():
            print(key + '=' + value)
    except (OSError, ValueError, TypeError) as error:
        parser.exit(1, str(error) + '\n')
