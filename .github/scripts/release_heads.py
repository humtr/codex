"""Authenticate both public lanes and retain their live Pages assets.

This is publication tooling, never a device state or runtime authority. Every
head is tied to the same exact main commit and accepted signing key.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from rald3_preflight import parse_descriptor
from rald5_publication import (canonical_release_base, parse_index, parse_manifest,
                               raw_public_key, verify_release_tree, verify_signature)

HEADS = {'full': 'update-index-v1', 'base': 'update-index-base-v1'}
LIMIT = (1 << 64) - 1


def plan(heads, variant):
    if variant not in HEADS or 'full' not in heads or set(heads) - HEADS.keys():
        raise ValueError('invalid release lane state')
    for lane, head in heads.items():
        if head['generation'].startswith('local-base-') != (lane == 'base'):
            raise ValueError('signed head belongs to another lane')
        if type(head['sequence']) is not int or not 0 < head['sequence'] <= LIMIT:
            raise ValueError('invalid signed release sequence')
    sequence = max(head['sequence'] for head in heads.values())
    if sequence == LIMIT:
        raise ValueError('release sequence exhausted')
    selected = heads.get(variant, heads['full'])
    return {'current_generation': selected['generation'], 'current_version': selected['version'],
            'current_archive_sha256': selected['archive'], 'current_release_sequence': str(selected['sequence']),
            'release_sequence': str(sequence + 1), 'current_index_path': HEADS[variant if variant in heads else 'full'],
            'bootstrap_base': str(variant == 'base' and variant not in heads).lower(),
            'retained_generations': sorted({head['generation'] for head in heads.values()})}


def download(url, target, limit, *, optional=False):
    try:
        # Identify this first-party qualification client. Cloudflare rejects
        # urllib's default Python user agent (1010), including public assets
        # that the native updater/curl can download normally.
        request = Request(url, headers={'User-Agent':'codex-release-qualification/1'})
        with urlopen(request, timeout=60) as response:
            if not response.geturl().startswith('https://'):
                raise ValueError('publication transport left HTTPS')
            data = response.read(limit + 1)
    except HTTPError as error:
        if optional and error.code == 404:
            return False
        raise ValueError(f'publication download failed: HTTP {error.code}: {url}') from error
    if len(data) > limit:
        raise ValueError('publication download exceeds bound')
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return True


def authenticate(main_sha, variant, authority, root, openssl='openssl', fetch=download):
    if re.fullmatch('[0-9a-f]{40}', main_sha) is None:
        raise ValueError('expected exact main commit')
    root.mkdir(parents=True, exist_ok=False)
    scratch = root / 'authority'
    scratch.mkdir()
    raw_key = raw_public_key(openssl, authority, scratch)
    heads = {}
    for lane, index_path in HEADS.items():
        directory = root / lane
        index = directory / 'update-index-v1'
        url = f'https://raw.githubusercontent.com/humtr/codex/{main_sha}/{index_path}'
        if not fetch(url, index, 16384, optional=lane == 'base'):
            continue
        fetch(url + '.sig', directory / 'update-index-v1.sig', 64)
        verify_signature(openssl, authority, index, directory / 'update-index-v1.sig')
        values = parse_index(index)
        generation = values['generation_id']
        base = canonical_release_base(generation)
        if values['release_base'] != base:
            raise ValueError('head release base differs from public origin')
        for name, bound in [('release.manifest', 131072), ('release.sig', 64), ('generation.meta', 65536)]:
            fetch(base + name, directory / name, bound)
        verify_signature(openssl, authority, directory / 'release.manifest', directory / 'release.sig')
        manifest, files = parse_manifest(directory / 'release.manifest')
        if manifest['generation_id'] != generation or manifest['release_public_key'] != raw_key:
            raise ValueError('head manifest identity or authority differs')
        expected = next((digest for name, digest, mode in files if name == 'generation.meta' and mode == '0644'), None)
        if hashlib.sha256((directory / 'generation.meta').read_bytes()).hexdigest() != expected:
            raise ValueError('head descriptor differs from signed inventory')
        descriptor = parse_descriptor(directory / 'generation.meta')
        if descriptor['generation_id'] != generation:
            raise ValueError('head descriptor identity differs')
        heads[lane] = {'generation': generation, 'sequence': int(manifest['release_sequence']),
                       'version': descriptor['upstream_package_version'], 'archive': descriptor['source_artifact_digest']}
    result = plan(heads, variant)
    result['expected_main_sha'] = main_sha
    (root / 'heads.json').write_text(json.dumps({'heads': heads, 'plan': result}, indent=2) + '\n')
    return heads, result


def restore(root, heads, authority, site, openssl='openssl', fetch=download):
    total = 0
    def verify(target, lane, head):
        scratch = root / ('verify-' + lane)
        scratch.mkdir(exist_ok=True)
        verify_release_tree(target, target / 'update-index-v1', target / 'update-index-v1.sig',
                            authority, head['generation'], str(head['sequence']),
                            canonical_release_base(head['generation']), openssl, scratch,
                            index_inside_release=True)
    for lane, head in heads.items():
        source, target = root / lane, site / head['generation']
        if target.exists():
            # The caller may already have restored its selected LKG.
            verify(target, lane, head)
            continue
        target.mkdir(parents=True)
        _, files = parse_manifest(source / 'release.manifest')
        for name in ['release.manifest', 'release.sig', 'update-index-v1', 'update-index-v1.sig']:
            (target / name).write_bytes((source / name).read_bytes())
        base = canonical_release_base(head['generation'])
        for name, _, mode in files:
            fetch(base + name, target / name, 1073741824)
            (target / name).chmod(int(mode, 8))
            total += (target / name).stat().st_size
            if total >= 1073741824:
                raise ValueError('retained Pages payload exceeds bound')
        if fetch(base + 'compat/download-size-v1', target / 'compat/download-size-v1', 131072, optional=True):
            fetch(base + 'compat/download-size-v1.sig', target / 'compat/download-size-v1.sig', 64)
        verify(target, lane, head)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--main-sha', required=True)
    parser.add_argument('--variant', choices=HEADS, required=True)
    parser.add_argument('--public-key', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--site', type=Path)
    args = parser.parse_args()
    heads, result = authenticate(args.main_sha, args.variant, args.public_key, args.output)
    if args.site:
        restore(args.output, heads, args.public_key, args.site)
    for key, value in result.items():
        if isinstance(value, str):
            print(key + '=' + value)
