#!/usr/bin/env python3
"""Classify changed paths conservatively; unknown changes require full proof."""
import sys


def scope(paths):
    result='docs'
    for path in paths:
        if path.endswith('.md') or path=='.gitignore':continue
        if path.endswith('.py') and path.startswith(('scripts/','.github/scripts/','frontend/')) or path=='scripts/check.sh':
            result='python'
        else:return 'full'
    return result


if __name__=='__main__':
    paths=sys.stdin.buffer.read().decode('utf-8',errors='surrogateescape').split('\0') if sys.argv[1:]==['--stdin'] else sys.argv[1:]
    print(scope([p for p in paths if p]))
