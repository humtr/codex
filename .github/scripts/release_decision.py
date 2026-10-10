#!/usr/bin/env python3
"""One admitted backend/frontend pair; schedule newer, manual current rebuild."""
import argparse
import re
import sys

from rald3_preflight import PreflightError, lower_sha256, positive_decimal, stable_version


def decide(a):
    current, upstream, qualified = map(stable_version, (a.current,a.upstream,a.qualified))
    lower_sha256(a.archive,'official archive')
    lower_sha256(a.qualified_archive,'qualified archive')
    positive_decimal(a.sequence,'next release sequence')
    if not re.fullmatch('[0-9a-f]{40}',a.source): raise PreflightError('source must be an immutable commit')
    if a.ref != 'refs/heads/main' or a.event not in ('schedule','workflow_dispatch'):
        raise PreflightError('release caller must be main schedule or manual dispatch')
    if a.rebuild_current=='true' and (a.event!='workflow_dispatch' or current!=qualified or upstream!=current):
        raise PreflightError('current rebuild requires manual dispatch of the authenticated qualified version')
    if a.event=='schedule' and a.publish!='true':
        raise PreflightError('scheduled caller must select ordinary publication')
    paired=upstream==qualified and a.archive==a.qualified_archive
    candidate=paired and (upstream>current or a.rebuild_current=='true')
    suffix=a.upstream.replace('.','-')+'-'+a.source[:12]+'-s'+a.sequence
    return {'candidate':str(candidate).lower(),'generation_id':'local-hosted-'+suffix,
            'upstream_version':a.upstream,'archive_sha256':a.archive,
            'artifact_name':'candidate-'+suffix,'qualified_artifact_name':'qualified-'+suffix,
            'signed_artifact_name':'signed-'+suffix}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for key in ('current','upstream','qualified','archive','qualified-archive','source','sequence','event','ref'):
        p.add_argument('--'+key,required=True)
    for key in ('publish','rebuild-current'):p.add_argument('--'+key,choices=('true','false'),required=True)
    try:
        for k,v in decide(p.parse_args()).items():print(k+'='+v)
    except PreflightError as e:
        print(str(e),file=sys.stderr);return 1
    return 0


if __name__=='__main__':sys.exit(main())
