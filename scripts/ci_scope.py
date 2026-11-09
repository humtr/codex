#!/usr/bin/env python3
"""Classify changed paths conservatively; unknown changes require full proof."""
import sys


def scope(paths):
    result='docs'
    for path in paths:
        # Some Markdown files are signed assets, release inputs or live test fixtures.
        if path == 'release/notes.md':return 'full'
        if (path.endswith('.md') and path != 'providers/chatgpt/NOTICE.md'
                and not path.startswith('scripts/fixtures/native_trial/')) or path=='.gitignore':continue
        provider_helper = (path.startswith(('scripts/qualify_provider_', 'scripts/fixtures/native_trial/'))
                           or path in ('scripts/setup_chatgpt_app_metadata.mjs', 'scripts/qualify_mcp_fault_proxy.py',
                                       'scripts/notify_native_qualification.py',
                                       'scripts/refresh_chatgpt_connector.mjs', 'scripts/provider_qualification.py',
                                       'scripts/test_provider_qualification.py',
                                       'scripts/test_provider_native_compiler.py',
                                       'scripts/test_provider_live_tui.py',
                                       'scripts/test_provider_result_recovery.py',
                                       'scripts/chat_entry_client.mjs', 'scripts/chat_entry_client.test.mjs', 'scripts/test_native_qualification_notice.py', 'scripts/test_provider_combined_routes.py', 'scripts/test_provider_agent_fixtures.py',
                                       'scripts/cdp_qualification_relay.mjs', 'scripts/cdp_qualification_relay.test.mjs',
                                       'scripts/release_native_qualification_observer.py',
                                       'scripts/test_provider_installed_entrypoint.py'))
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
