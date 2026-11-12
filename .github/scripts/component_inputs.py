"""Identity of declared Android Core and Manager production inputs.

The selected locked package trees are conservative inventories, not the minimal
activated Cargo compilation graph. Runtime and TUI keep their own producers;
matching an input inventory does not assert that qualification passed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from runtime_inputs import assets, cargo_dependencies, locked_packages, canonical

COMPONENTS = {'core': ('crates/core', 'codex'), 'manager': ('crates/manager', 'codex-manager')}
BUILD_INPUTS = 'release/android-build-inputs-v1.json'
PRODUCER_INPUTS = [BUILD_INPUTS, '.github/workflows/android-component.yml',
                   '.github/scripts/component_inputs.py', '.github/scripts/component_artifact.py']


def git(source, *args):
    return subprocess.check_output(['git', '-C', str(source), *args], stderr=subprocess.DEVNULL)


def build_config(data):
    value=json.loads(data)
    expected={'schema','target','android_api','ndk_version','profile','rustflags','incremental'}
    if (set(value)!=expected or value['schema']!='codex-android-build-inputs-v1'
            or value['target']!='aarch64-linux-android' or value['profile']!='release'
            or type(value['android_api']) is not int or not 24<=value['android_api']<=35
            or re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',value['ndk_version']) is None
            or value['rustflags']!='' or value['incremental'] is not False):
        raise ValueError('unsupported Android compiler inputs')
    return value


def inputs(source, component, revision='HEAD'):
    if component not in COMPONENTS:
        raise ValueError('unknown Android component')
    def read(name):
        return git(source,'show',revision+':'+name)
    recipe=json.loads(read('release/upstream.json'))
    if re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',recipe['toolchains']['core_rust']) is None:
        raise ValueError('invalid component Rust toolchain')
    selected=cargo_dependencies(read,COMPONENTS[component][0]+'/Cargo.toml')
    workspace=tomllib.loads(read('Cargo.toml').decode())
    roots=list(PRODUCER_INPUTS)
    members=[]
    for member in workspace['workspace']['members']:
        package=tomllib.loads(read(member+'/Cargo.toml').decode())['package']
        if (package['name'],package['version'],None) in selected:
            roots.append(member)
            members.append(member)
    workspace['workspace']['members']=members
    result={'schema':'codex-component-build-inputs-v1','component':component,
            'compiler':build_config(read(BUILD_INPUTS)),
            'rust':recipe['toolchains']['core_rust'],'workspace':workspace,
            'lock':locked_packages(selected)}
    if component=='core':
        # release-builder/build.rs embeds every recipe asset AND the complete
        # Cargo.lock. Do not claim Manager-only lock independence while those
        # actual bytes remain compiled into Core.
        roots.extend(name for name,role in assets(read('release/runtime-build-inputs-v1').decode()).items() if role=='recipe')
        # local_recipe() also directly embeds the complete source recipe. It
        # is an input-role asset rather than part of build.rs's recipe bundle.
        roots.append('release/upstream.json')
        result['embedded_lock_sha256']=hashlib.sha256(read('Cargo.lock')).hexdigest()
    tree=git(source,'ls-tree','-r',revision,'--',*roots).decode().splitlines()
    files={}
    for row in tree:
        identity,name=row.split('\t')
        mode,kind,digest=identity.split()
        if kind!='blob' or mode not in {'100644','100755'}:
            raise ValueError('component input must be a regular Git blob')
        files[name]=[mode,digest]
    if not all(name in files for name in PRODUCER_INPUTS):
        raise ValueError('missing Android producer input')
    result['files']=files
    return result


def fingerprint(source, component, revision='HEAD'):
    return hashlib.sha256(canonical(inputs(source,component,revision))).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--component',choices=COMPONENTS,required=True)
    parser.add_argument('--revision',default='HEAD')
    parser.add_argument('--env',action='store_true')
    args=parser.parse_args()
    value=inputs(args.source,args.component,args.revision)
    if args.env:
        settings=value['compiler']
        for name,data in [('INPUTS_SHA256',hashlib.sha256(canonical(value)).hexdigest()),
                          ('RUST_VERSION',value['rust']),('ANDROID_API',settings['android_api']),
                          ('ANDROID_NDK_VERSION',settings['ndk_version']),('ANDROID_TARGET',settings['target']),
                          ('COMPONENT_PACKAGE',COMPONENTS[args.component][1])]:
            print(f'{name}={data}')
    else:
        print(hashlib.sha256(canonical(value)).hexdigest())

if __name__=='__main__':main()
