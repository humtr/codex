#!/usr/bin/env python3
"""Classify changed paths conservatively; unknown changes require full proof."""
import sys


def scope(paths):
    result='docs'
    for path in paths:
        if path.endswith('.md') or path=='.gitignore':continue
        provider_helper = (path.startswith(('scripts/qualify_provider_', 'scripts/d3_', 'scripts/fixtures/d3/'))
                           or path in ('scripts/provider_qualification.py', 'scripts/test_provider_qualification.py',
                                       'scripts/chat_entry_client.mjs', 'scripts/chat_entry_client.test.mjs', 'scripts/test_d3_notify.py', 'scripts/test_provider_f.py', 'scripts/test_provider_f_agents.py',
                                       'scripts/f_cdp_relay.mjs', 'scripts/f_cdp_relay.test.mjs'))
        if path.startswith('providers/chatgpt/') or path=='.github/workflows/provider-qualification.yml' or provider_helper:
            if result=='docs':result='provider'
            continue
        if path.endswith('.py') and path.startswith(('scripts/','.github/scripts/','frontend/')) or path=='scripts/check.sh':
            result='python'
        else:return 'full'
    return result


if __name__=='__main__':
    paths=sys.stdin.buffer.read().decode('utf-8',errors='surrogateescape').split('\0') if sys.argv[1:]==['--stdin'] else sys.argv[1:]
    print(scope([p for p in paths if p]))
