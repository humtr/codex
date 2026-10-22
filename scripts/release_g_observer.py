#!/usr/bin/env python3
"""Release a G qualification observer lease after a saved native settlement."""
import argparse
import json
from pathlib import Path

from qualification_diagnostics import release_observer_keeper


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('nonce')
    parser.add_argument('settlement', type=Path,
                        help='saved read-only native settlement receipt')
    args = parser.parse_args()
    receipt = json.loads(args.settlement.read_text())
    release_observer_keeper(args.root, args.nonce, receipt)
    print(json.dumps({'released': True, 'root': str(args.root)}))


if __name__ == '__main__':
    main()
