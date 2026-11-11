"""Bounded non-secret failure fields before optional CI artifact transport."""
import argparse
import json
import os
from pathlib import Path
import re
import stat

STAGES = {
    'host-directory': 'mkdir official host directory',
    'host-download': 'curl official host package',
    'host-digest': 'sha256sum official host package',
    'host-extract': 'python official host extraction',
    'host-probe': 'official host --help',
    'retained-features': 'check_upstream_features.py --run --logs',
}
TAIL_BYTES = 64 * 1024
MAX_FAILURES = 12
ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')
FAILED = re.compile(
    r'^\s*(FAIL|TIMEOUT|ABORT|EXECFAIL|LEAK)\s+\[[ 0-9.a-z]+\]\s+'
    r'(?:\(\s*\d+\s*/\s*\d+\s*\)\s+)?'
    r'([A-Za-z0-9_:-]{1,96})\s+([A-Za-z_][A-Za-z0-9_:]{0,511})\s*$')
SUMMARY = re.compile(r'^\s*Summary\s+\[[ 0-9.a-z]+\]\s+(\d{1,9}) tests? run:(.*)$')
COUNTS = re.compile(r'(?<![A-Za-z0-9_])(\d{1,9}) (passed|failed|skipped|leaky|slow|timed out)\b')


def log_fields(path):
    """Read only a bounded regular-file tail, and expose no free-form log text."""
    descriptor = None
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode):
            return {'state': 'not-regular'}
        os.lseek(descriptor, max(0, info.st_size - TAIL_BYTES), os.SEEK_SET)
        data = os.read(descriptor, TAIL_BYTES)
    except OSError:
        return {'state': 'unavailable'}
    finally:
        if descriptor is not None:
            os.close(descriptor)
    if info.st_size > TAIL_BYTES:
        data = data.partition(b'\n')[2]  # Never interpret an incomplete first line.
    failures = []
    summary = None
    compiler_errors = set()
    for line in ANSI.sub('', data.decode('utf-8', errors='replace')).splitlines():
        match = FAILED.fullmatch(line)
        if match:
            entry = {'status': match[1], 'package': match[2], 'test': match[3]}
            if entry not in failures and len(failures) < MAX_FAILURES:
                failures.append(entry)
        match = SUMMARY.fullmatch(line)
        if match:
            summary = {'run': int(match[1]), **{kind.replace(' ', '_'): int(count)
                       for count, kind in COUNTS.findall(match[2])}}
        match = re.match(r'^error\[(E[0-9]{4})\]:', line)
        if match and len(compiler_errors) < MAX_FAILURES:
            compiler_errors.add(match[1])
    return {'state': 'read', 'bytes': info.st_size, 'tail_bytes': len(data),
            'failed_tests': failures, 'summary': summary,
            'compiler_error_codes': sorted(compiler_errors)}


def report(stage, status, logs):
    if stage not in STAGES or not 1 <= status <= 255:
        raise ValueError('invalid qualification failure identity')
    return {'schema': 'codex-qualification-failure-v1', 'stage': stage,
            'command': STAGES[stage], 'command_exit': status,
            'logs': {suite: log_fields(logs / ('profile-tui-upstream-' + suite + '.log'))
                     for suite in ('tui', 'engine', 'patch')}}


def annotation(value):
    body = json.dumps(value, sort_keys=True, separators=(',', ':'))
    # Treat every payload byte as annotation data, never another workflow command.
    body = body.replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
    return '::error title=Retained feature qualification failed::' + body


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', choices=STAGES, required=True)
    parser.add_argument('--status', type=int, required=True)
    parser.add_argument('--logs', type=Path, required=True)
    args = parser.parse_args()
    value = report(args.stage, args.status, args.logs)
    # Annotation comes first: even a failed local receipt write cannot hide it.
    print(annotation(value), flush=True)
    target = args.logs / 'profile-tui-failure-summary.json'
    with target.open('x') as stream:
        json.dump(value, stream, sort_keys=True)
        stream.write('\n')


if __name__ == '__main__':
    main()
