#!/usr/bin/env python3
"""Separate qualification-only work from explicit base/full publication lanes."""
import argparse
import re
import sys

from rald3_preflight import PreflightError, lower_sha256, positive_decimal, stable_version


def decide(a):
    current, upstream = map(stable_version, (a.current,a.upstream))
    lower_sha256(a.archive,'official archive')
    positive_decimal(a.sequence,'next release sequence')
    if not re.fullmatch('[0-9a-f]{40}',a.source): raise PreflightError('source must be an immutable commit')
    base_only=a.base_only=='true'
    variant=getattr(a, 'variant', 'full')
    if variant not in ('base','full'):
        raise PreflightError('unknown release variant')
    if base_only:
        if a.publish!='false':
            raise PreflightError('base qualification cannot publish')
        if a.ref!='refs/heads/rewrite/rust-core' or a.event not in ('push','workflow_dispatch'):
            raise PreflightError('base qualification requires a rewrite source event')
        if a.rebuild_current=='true' and upstream!=current:
            raise PreflightError('base current qualification requires the authenticated current version')
        suffix=a.upstream.replace('.','-')+'-'+a.source[:12]+'-s'+a.sequence
        return {'candidate':'true','decision_reason':'base-qualification',
                'generation_id':'base-qualification-'+suffix,'upstream_version':a.upstream,
                'archive_sha256':a.archive,'artifact_name':'base-candidate-'+suffix,
                'qualified_artifact_name':'base-qualified-'+suffix,
                'signed_artifact_name':'base-not-publishable-'+suffix}
    if a.ref != 'refs/heads/main' or a.event not in ('schedule','workflow_dispatch'):
        raise PreflightError('release caller must be main schedule or manual dispatch')
    if variant=='base':
        if a.rebuild_current=='true' and (a.event!='workflow_dispatch' or upstream!=current):
            raise PreflightError('base current rebuild requires manual dispatch of its authenticated version')
        if a.event=='schedule' and a.publish!='true':
            raise PreflightError('scheduled caller must select ordinary publication')
        bootstrap=getattr(a, 'bootstrap_base', 'false')=='true'
        candidate=upstream>current or a.rebuild_current=='true' or bootstrap and upstream==current
        suffix=a.upstream.replace('.','-')+'-'+a.source[:12]+'-s'+a.sequence
        return {'candidate':str(candidate).lower(),'decision_reason':'base-candidate' if candidate else 'upstream-not-newer',
                'generation_id':'local-base-relay-hosted-'+suffix,'upstream_version':a.upstream,
                'archive_sha256':a.archive,'artifact_name':'base-candidate-'+suffix,
                'qualified_artifact_name':'base-qualified-'+suffix,'signed_artifact_name':'base-signed-'+suffix}
    if not a.qualified or not a.qualified_archive:
        suffix=a.upstream.replace('.','-')+'-'+a.source[:12]+'-s'+a.sequence
        return {'candidate':'false','decision_reason':'frontend-unqualified',
                'generation_id':'local-relay-hosted-'+suffix,'upstream_version':a.upstream,
                'archive_sha256':a.archive,'artifact_name':'candidate-'+suffix,
                'qualified_artifact_name':'qualified-'+suffix,'signed_artifact_name':'signed-'+suffix}
    qualified=stable_version(a.qualified)
    lower_sha256(a.qualified_archive,'qualified archive')
    if a.rebuild_current=='true' and (a.event!='workflow_dispatch' or current!=qualified or upstream!=current):
        raise PreflightError('current rebuild requires manual dispatch of the authenticated qualified version')
    if a.event=='schedule' and a.publish!='true':
        raise PreflightError('scheduled caller must select ordinary publication')
    paired=upstream==qualified and a.archive==a.qualified_archive
    candidate=paired and (upstream>current or a.rebuild_current=='true')
    reason=('integrated-candidate' if candidate else 'upstream-not-newer' if upstream<=current
            else 'frontend-unqualified' if upstream!=qualified else 'archive-unqualified')
    suffix=a.upstream.replace('.','-')+'-'+a.source[:12]+'-s'+a.sequence
    return {'candidate':str(candidate).lower(),'decision_reason':reason,'generation_id':'local-relay-hosted-'+suffix,
            'upstream_version':a.upstream,'archive_sha256':a.archive,
            'artifact_name':'candidate-'+suffix,'qualified_artifact_name':'qualified-'+suffix,
            'signed_artifact_name':'signed-'+suffix}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for key in ('current','upstream','archive','source','sequence','event','ref'):
        p.add_argument('--'+key,required=True)
    for key in ('qualified','qualified-archive'):p.add_argument('--'+key)
    for key in ('publish','rebuild-current'):p.add_argument('--'+key,choices=('true','false'),required=True)
    p.add_argument('--base-only',choices=('true','false'),default='false')
    p.add_argument('--variant',choices=('base','full'),default='full')
    p.add_argument('--bootstrap-base',choices=('true','false'),default='false')
    try:
        for k,v in decide(p.parse_args()).items():print(k+'='+v)
    except PreflightError as e:
        print(str(e),file=sys.stderr);return 1
    return 0


if __name__=='__main__':sys.exit(main())
