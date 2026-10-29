"""Discover one official stable tuple; submit inputs, never invent a Termux port."""
import base64
import copy
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

from rald3_preflight import parse_upstream, stable_version, lower_sha256
from upstream_release import load

LATEST='repos/openai/codex/releases/latest'


def proposal(current, metadata, resolve):
    version,digest=metadata
    lower_sha256(digest,'official archive')
    if stable_version(version)<=stable_version(current['upstream']['version']):return None
    source,toolchain,audit=resolve(version)
    if re.fullmatch('[0-9a-f]{40}',source) is None or re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',toolchain) is None:
        raise ValueError('official source/toolchain identity is invalid')
    result=copy.deepcopy(current)
    result['upstream']={'version':version,'source':source,'archive_sha256':digest}
    result['toolchains']['frontend_rust']=toolchain
    result['frontend']=None
    result['qualification']['lock_audit']=audit
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
        # A new release has its own workspace/dependency inventory. Derive the
        # exact normalization audit instead of copying the preceding release's.
        with tempfile.TemporaryDirectory(prefix='upstream-lock-') as directory:
            checkout=Path(directory)/'source'
            subprocess.run(['git','init','-q',str(checkout)],check=True)
            subprocess.run(['git','-C',str(checkout),'fetch','--quiet','--no-tags','--depth=1',
                            'https://github.com/openai/codex.git',sha],check=True)
            subprocess.run(['git','-C',str(checkout),'checkout','--quiet','--detach','FETCH_HEAD'],check=True)
            if subprocess.check_output(['git','-C',str(checkout),'rev-parse','HEAD'],text=True).strip()!=sha:
                raise ValueError('official lock source identity differs')
            audit_path=Path(directory)/'lock-audit.json'
            builder=os.environ.get('CODEX_RELEASE_BUILDER')
            if not builder:raise ValueError('prepared Rust release-builder is required for upstream lock admission')
            subprocess.run([builder,'normalize-lock','--source',str(checkout/'codex-rs'),
                            '--openssl','/usr/bin/openssl','--audit',str(audit_path)],
                           check=True,stdout=subprocess.DEVNULL)
            audit=json.loads(audit_path.read_text())
        return sha,toolchain,audit
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
