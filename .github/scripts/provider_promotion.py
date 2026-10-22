#!/usr/bin/env python3
"""Non-forced promotion of an independently qualified Provider stable pointer.

Base/Full caller and update-index files are never included in this tree edit.
A lost PATCH reply is reconciled by reading the ref once; no mutation is retried.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

from provider_publication import verify_index, verify_index_record, read


def api(path, method='GET', body=None):
    command = ['gh', 'api', 'repos/humtr/codex/' + path]
    if method != 'GET':
        command += ['--method', method]
    if body is not None:
        command += ['--input', '-']
    result = subprocess.run(command, input=json.dumps(body).encode() if body is not None else None,
                            capture_output=True, timeout=45)
    if result.returncode:
        raise RuntimeError('Provider publication API request failed; inspect before retry: ' + path)
    return json.loads(result.stdout)


def validate(args):
    fields = verify_index(args.index, args.signature, args.package, args.public_key, args.openssl)
    proof = json.loads(read(args.proof, 1024 * 1024))
    if (proof.get('schema') != 'termux-codex-provider-public-consumer-v1' or proof.get('accepted') is not True
            or proof.get('generation') != fields['generation_id']
            or proof.get('source_sha') != fields['source']
            or proof.get('manifest_sha256') != hashlib.sha256(read(args.package / 'payload.manifest', 8192)).hexdigest()
            or not all(proof.get(name) is True for name in
                       ('complete_public_readback', 'actual_android', 'disposable_install', 'dispatch',
                        'rollback', 'native_defaults_preserved', 'operating_runtime_unchanged'))):
        raise ValueError('complete public readback and disposable Android consumer proof required')
    if not re.fullmatch('[0-9a-f]{40}', args.expected_main):
        raise ValueError('exact expected main commit required')
    return fields


def promote(args):
    fields = validate(args)
    paths = {'provider-index-v1': args.index.read_bytes(), 'provider-index-v1.sig': args.signature.read_bytes(),
             'provider-source-v1': (fields['source'] + '\n').encode()}
    if not args.apply:
        return dict(prepared=True, mutation=False, generation=fields['generation_id'], paths=sorted(paths))
    before = api('git/ref/heads/main')['object']['sha']
    if before != args.expected_main:
        raise ValueError('Provider promotion stale main; preserve current ref')
    # A source commit must exist in this owned repository before public promotion.
    source = api('git/commits/' + fields['source'])
    if source['sha'] != fields['source']:
        raise ValueError('Provider source commit not found')
    base_tree = api('git/commits/' + before)['tree']['sha']
    existing={entry['path']:entry for entry in api('git/trees/'+base_tree)['tree']}
    if ('provider-index-v1' in existing) != ('provider-index-v1.sig' in existing):
        raise ValueError('incomplete existing Provider authority state')
    if 'provider-index-v1' in existing:
        old_index=base64.b64decode(api('git/blobs/'+existing['provider-index-v1']['sha'])['content'])
        old_signature=base64.b64decode(api('git/blobs/'+existing['provider-index-v1.sig']['sha'])['content'])
        with tempfile.TemporaryDirectory(prefix='provider-lkg-') as temporary:
            temporary=Path(temporary);old=temporary/'index';sig=temporary/'signature'
            old.write_bytes(old_index);sig.write_bytes(old_signature)
            previous=verify_index_record(old,sig,args.public_key,args.openssl)
        if old_index==paths['provider-index-v1'] and old_signature==paths['provider-index-v1.sig']:
            return dict(promoted=False,unchanged=True,mutation=False,current_main=before)
        if int(fields['release_sequence'])<=int(previous['release_sequence']):
            raise ValueError('Provider stable sequence rollback/reuse rejected')
    tree_entries = []
    for name, content in paths.items():
        blob = api('git/blobs', 'POST', dict(content=base64.b64encode(content).decode(), encoding='base64'))['sha']
        tree_entries.append(dict(path=name, mode='100644', type='blob', sha=blob))
    tree = api('git/trees', 'POST', dict(base_tree=base_tree, tree=tree_entries))['sha']
    commit = api('git/commits', 'POST', dict(message='release: promote optional Provider ' + fields['generation_id'],
                                            tree=tree, parents=[before]))['sha']
    mutation_error = None
    try:
        api('git/refs/heads/main', 'PATCH', dict(sha=commit, force=False))
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        mutation_error = type(error).__name__
    actual = api('git/ref/heads/main')['object']['sha']
    if actual != commit:
        raise RuntimeError('Provider promotion was not confirmed; do not retry: ' + actual)
    # Contents API can transcode non-UTF-8 signature bytes while still reporting
    # the original size/blob SHA. Read the committed tree's canonical Git blobs.
    committed = {entry['path']: entry for entry in api('git/trees/' + tree)['tree']}
    for name, expected in paths.items():
        served = api('git/blobs/' + committed[name]['sha'])
        if served.get('encoding') != 'base64' or base64.b64decode(served['content']) != expected:
            raise ValueError('promoted Provider pointer readback mismatch')
    return dict(promoted=True, mutation=True, generation=fields['generation_id'], previous_main=before,
                current_main=commit, mutation_reply_error=mutation_error, base_full_heads_modified=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('index', 'signature', 'package', 'public-key', 'proof'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--openssl', type=Path, default=Path(shutil.which('openssl')))
    parser.add_argument('--expected-main', required=True)
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    print(json.dumps(promote(args)))


if __name__ == '__main__':
    main()
