"""Apply the existing common Termux path policy to an unsigned preview frontend."""
import argparse
import hashlib
import json
from pathlib import Path
import re

UPSTREAM = 'a956835d020762cb2b570053af06f643a11c0ecc'


def policy():
    # One policy owner: do not duplicate or relax the production builder table.
    source = (Path(__file__).resolve().parents[2] / 'crates/release-builder/src/lib.rs').read_text()
    table = source.split('const PATCHES: [(&[u8], &[u8], usize); 4] = [', 1)[1].split('];', 1)[0]
    entries = re.findall(r'\(\s*b"([^"]+)",\s*b"([^"]+)",\s*(\d+),?\s*\)', table)
    remaps = [(old.encode(), new.encode(), int(count)) for old, new, count in entries]
    assert len(remaps) == 4 and [count for _, _, count in remaps] == [2, 1, 1, 1]
    assert all(len(old) == len(new) for old, new, _ in remaps)
    assert sum(sum(a != b for a, b in zip(old, new)) * count for old, new, count in remaps) == 54
    return remaps


def adapt(raw):
    remaps = policy()
    for old, new, count in remaps:
        assert raw.count(old) == count and raw.count(new) == 0, 'frontend path policy mismatch'
    adapted = raw
    for old, new, _ in remaps:
        adapted = adapted.replace(old, new)
    assert len(raw) == len(adapted)
    restored = adapted
    for old, new, count in remaps:
        assert adapted.count(old) == 0 and adapted.count(new) == count
        restored = restored.replace(new, old)
    assert restored == raw, 'change outside selected path remaps'
    return adapted


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('raw', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--expected-sha256', required=True)
    a = parser.parse_args()
    source = dict(line.split('=', 1) for line in (a.raw.parent / 'source.txt').read_text().splitlines())
    assert source['upstream_sha'] == UPSTREAM and source['target'] == 'aarch64-unknown-linux-musl'
    raw = a.raw.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == a.expected_sha256, 'hosted artifact digest mismatch'
    adapted = adapt(raw)
    with a.output.open('xb') as out:
        out.write(adapted)
    a.output.chmod(0o755)
    print(json.dumps({'upstream_sha': UPSTREAM, 'experiment_sha': source['experiment_sha'],
                     'raw_sha256': a.expected_sha256, 'adapted_sha256': hashlib.sha256(adapted).hexdigest(),
                     'source_counts': [2, 1, 1, 1], 'changed_bytes': 54, 'size': len(adapted)}, sort_keys=True))
