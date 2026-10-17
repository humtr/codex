"""Discover one official stable tuple; submit inputs, never invent a Termux port."""
import base64
import copy
import json
from pathlib import Path
import re
import tempfile
import tomllib

from rald3_preflight import parse_upstream, stable_version, lower_sha256
from upstream_release import load

LATEST='repos/openai/codex/releases/latest'


def proposal(current, metadata, resolve):
    version,digest=metadata
    lower_sha256(digest,'official archive')
    if stable_version(version)<=stable_version(current['upstream']['version']):return None
    source,toolchain=resolve(version)
    if re.fullmatch('[0-9a-f]{40}',source) is None or re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',toolchain) is None:
        raise ValueError('official source/toolchain identity is invalid')
    result=copy.deepcopy(current)
    result['upstream']={'version':version,'source':source,'archive_sha256':digest}
    result['toolchains']['frontend_rust']=toolchain
    result['frontend']=None
    return result


def discover(source, check_only=False):
    from release_control import api, gh, commit_files, dispatch, SOURCE_BRANCH, git_head
    current=git_head(source)
    if api('git/ref/heads/'+SOURCE_BRANCH)['object']['sha']!=current:return {'state':'obsolete-source-ignored'}
    def resolve(version):
        value=json.loads(gh('api','repos/openai/codex/git/ref/tags/rust-v'+version))['object']
        for _ in range(4):
            if value['type']=='commit':break
            if value['type']!='tag' or re.fullmatch('[0-9a-f]{40}',value['sha']) is None:
                raise ValueError('invalid official tag identity')
            value=json.loads(gh('api','repos/openai/codex/git/tags/'+value['sha']))['object']
        if value['type']!='commit':raise ValueError('official tag did not resolve to a commit')
        sha=value['sha']
        content=json.loads(gh('api','repos/openai/codex/contents/codex-rs/rust-toolchain.toml?ref='+sha))
        raw=base64.b64decode(content['content'],validate=False)
        if len(raw)>16384:raise ValueError('oversized official toolchain file')
        toolchain=tomllib.loads(raw.decode())['toolchain']['channel']
        return sha,toolchain
    with tempfile.TemporaryDirectory(prefix='upstream-discovery-') as temporary:
        metadata=Path(temporary)/'upstream.json'
        raw=gh('api',LATEST,'--jq','{tag_name,draft,prerelease,assets:[.assets[] | select(.name=="codex-package-aarch64-unknown-linux-musl.tar.gz") | {name,digest}]}')
        if len(raw)>131072:raise ValueError('official release metadata exceeds bound')
        metadata.write_bytes(raw)
        value=json.loads(raw)
        if value.get('draft') is not False or value.get('prerelease') is not False:
            raise ValueError('official release is not stable')
        updated=proposal(load(source/'release/upstream.json'),parse_upstream(metadata),resolve)
    if updated is None:return {'state':'upstream-not-newer'}
    if check_only:return {'state':'new-upstream-requires-qualification','upstream':updated['upstream']}
    commit=commit_files(SOURCE_BRANCH,current,{'release/upstream.json':(json.dumps(updated,indent=2)+'\n').encode()},
                        'Prepare official upstream '+updated['upstream']['version']+' for independent base qualification')
    dispatch('rald7-full-acceptance.yml',SOURCE_BRANCH)
    dispatch('core-runtime-qualification.yml',SOURCE_BRANCH,{'rebuild_current':'true'})
    return {'state':'new-upstream-awaiting-real-proof','source':commit,'version':updated['upstream']['version']}


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--check-only',action='store_true')
    args=parser.parse_args();print(json.dumps(discover(args.source.resolve(),args.check_only)))
