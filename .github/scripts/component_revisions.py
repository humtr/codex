"""Release metadata for independent module revisions; no device state authority."""
import hashlib
import json
from pathlib import Path
import re
import stat
from component_inputs import fingerprint as component_fingerprint, git
from runtime_inputs import fingerprint as runtime_fingerprint, canonical
from release_preflight import parse_descriptor, sha256_file

MAX_REVISION=(1<<31)-1
COMPONENT_NAMES={'core','runtime','manager','tui'}


def validate_components(value):
    if not isinstance(value,dict) or not {'core','runtime'}<=set(value)<=COMPONENT_NAMES:
        raise ValueError('invalid selected components')
    if ('manager' in value)!=('tui' in value):
        raise ValueError('Full components must select Manager and TUI together')
    for name,entry in value.items():
        if (not isinstance(entry,dict) or set(entry)!={'revision','inputs_sha256','artifact_sha256','interface'}
                or type(entry['revision']) is not int or not 0<entry['revision']<=MAX_REVISION
                or any(not isinstance(entry[k],str) or re.fullmatch('[0-9a-f]{64}',entry[k]) is None
                       for k in ('inputs_sha256','artifact_sha256'))
                or not isinstance(entry['interface'],str)
                or re.fullmatch('[A-Za-z0-9._:-]{1,96}',entry['interface']) is None):
            raise ValueError('invalid '+name+' revision metadata')
    return value


def tui_fingerprint(source):
    from tui_artifact import INPUTS
    from upstream_release import build_recipe, load
    from qualification_signal import native_recipe
    files={}
    for name in INPUTS:
        if name.endswith('.yml'):
            files[name]=hashlib.sha256(native_recipe(git(source,'show','HEAD:'+name).decode()).encode()).hexdigest()
        else:
            files[name]=git(source,'rev-parse','HEAD:'+name).decode().strip()
    return hashlib.sha256(canonical({'recipe':build_recipe(load(source/'release/upstream.json')),'files':files})).hexdigest()


def selected(source,candidate):
    descriptor=parse_descriptor(candidate/'generation.meta')
    result={
        'core':{'inputs_sha256':component_fingerprint(source,'core'),
                'artifact_sha256':sha256_file(candidate/'core'),'interface':descriptor['core_api_identity']},
        'runtime':{'inputs_sha256':runtime_fingerprint(source,'HEAD',adapted=True),
                   'artifact_sha256':sha256_file(candidate/'runtime'),'interface':'codex-runtime-bundle-v1'}}
    if descriptor['manager_artifact_digest']!='-':
        result['manager']={'inputs_sha256':component_fingerprint(source,'manager'),
                           'artifact_sha256':sha256_file(candidate/'manager'),'interface':'codex-manager-core-v1'}
        result['tui']={'inputs_sha256':tui_fingerprint(source),
                       'artifact_sha256':sha256_file(candidate/'helpers/2'),'interface':'termux-manager-tui-v1'}
    return descriptor['upstream_package_version'],result


def assign(version,current,history):
    prior=[record for record in history if record['upstream_version']==version and 'components' in record]
    for record in prior:validate_components(record['components'])
    result={}
    for name,entry in current.items():
        records=[record['components'][name] for record in prior if name in record['components']]
        seen={}
        for record in records:
            identity=canonical({k:v for k,v in record.items() if k!='revision'})
            if record['revision'] in seen and seen[record['revision']]!=identity:
                raise ValueError('one component revision binds conflicting inputs or artifacts')
            seen[record['revision']]=identity
        matching=[record['revision'] for record in records if all(record[k]==v for k,v in entry.items())]
        revision=max(matching) if matching else max(seen,default=0)+1
        if revision>MAX_REVISION:raise ValueError('component revision exhausted')
        result[name]={'revision':revision,**entry}
    validate_components(result)
    full=[record for record in prior if 'composition_revision' in record and set(record['components'])==set(result)]
    compositions={}
    for record in full:
        identity=canonical(record['components'])
        previous=compositions.setdefault(record['composition_revision'],identity)
        if previous!=identity:raise ValueError('one composition revision binds conflicting components')
        if type(record['composition_revision']) is not int or not 0<record['composition_revision']<=MAX_REVISION:
            raise ValueError('invalid historical composition revision')
    matches=[record['composition_revision'] for record in full if record['components']==result]
    composition=max(matches) if matches else max((record['composition_revision'] for record in full),default=0)+1
    if composition>MAX_REVISION:raise ValueError('composition revision exhausted')
    return {'upstream_version':version,'components':result,'composition_revision':composition}


def candidate_digest(candidate, *, deferred=False):
    if not stat.S_ISDIR(candidate.lstat().st_mode):raise ValueError('invalid candidate root')
    descriptor=parse_descriptor(candidate/'generation.meta')
    expected={'core','runtime','codex-code-mode-host','helpers/0','helpers/1','generation.meta'}
    if descriptor['manager_artifact_digest']!='-':expected|={'manager','helpers/2'}
    files={}
    for path in candidate.rglob('*'):
        info=path.lstat()
        if stat.S_ISDIR(info.st_mode):
            if path.relative_to(candidate).as_posix()!='helpers':raise ValueError('unexpected candidate directory')
            continue
        if not stat.S_ISREG(info.st_mode):raise ValueError('candidate contains nonregular payload')
        if info.st_size>536870912:raise ValueError('candidate payload exceeds bound')
        if stat.S_ISREG(info.st_mode):
            name=path.relative_to(candidate).as_posix()
            if deferred and name=='.manager-probe-deferred':
                if (info.st_mode&0o7777!=0o644 or info.st_size!=len(b'codex-manager-probe-deferred-v1\n')
                        or path.read_bytes()!=b'codex-manager-probe-deferred-v1\n'):
                    raise ValueError('invalid deferred probe marker')
                continue
            if name not in expected:raise ValueError('candidate inventory differs')
            mode=path.stat().st_mode&0o7777
            if mode!=(0o644 if name=='generation.meta' else 0o755):raise ValueError('candidate mode differs')
            files[name]={'sha256':sha256_file(path),'mode':mode,'size':path.stat().st_size}
    if set(files)!=expected:raise ValueError('candidate inventory incomplete')
    return hashlib.sha256(canonical(files)).hexdigest()
