import unittest
import subprocess
import sys
from pathlib import Path
from scripts.ci_scope import scope


class ScopeTests(unittest.TestCase):
    def test_changed_paths(self):
        for paths,want in [(['README.md','CURRENT.md'],'docs'),([], 'docs'),
                           (['scripts/check.sh','SPEC.md'],'python'),
                           (['.github/scripts/release_signing.py'],'python'),
                           (['scripts/test_ci_scope.py','crates/core/src/main.rs'],'full'),
                           (['Cargo.lock'],'full'),(['install.sh'],'full'),
                           (['.github/workflows/auto-release-termux.yml'],'full'),
                           (['providers/chatgpt/desktop_protocol.mjs'],'provider'),
                           (['scripts/provider_qualification.py'],'provider'),
                           (['scripts/qualify_provider_combined_routes.py','scripts/test_provider_combined_routes.py','scripts/cdp_qualification_relay.mjs'],'provider'),
                           (['scripts/cdp_qualification_relay.test.mjs'],'provider'),
                           (['scripts/release_native_qualification_observer.py'],'provider'),
                           (['scripts/test_provider_installed_entrypoint.py'],'provider'),
                           (['scripts/qualify_provider_agent_fixtures.py','scripts/test_provider_agent_fixtures.py'],'provider'),
                           (['scripts/refresh_chatgpt_connector.mjs'],'provider'),
                           (['scripts/qualify_provider_native_trial.py','scripts/refresh_chatgpt_connector.mjs'],'provider'),
                           (['scripts/setup_chatgpt_app_metadata.mjs'],'provider'),
                           (['scripts/notify_native_qualification.py'],'provider'),
                           (['scripts/qualify_mcp_fault_proxy.py'],'provider'),
                           (['scripts/fixtures/native_trial/AGENTS.md'],'provider'),
                           (['scripts/chat_entry_client.mjs','scripts/fixtures/native_trial/fixture.json'],'provider'),
                           (['scripts/chat_entry_client.test.mjs'],'provider'),
                           (['.github/workflows/provider-qualification.yml'],'provider'),
                           (['.github/workflows/provider-qualification.yml','crates/core/src/main.rs'],'full'),
                           (['providers/chatgpt/desktop_protocol.mjs','SPEC.md'],'provider'),
                           (['providers/chatgpt/test_desktop_profile.py','scripts/test_check.py'],'python'),
                           (['scripts/test_check.py','providers/chatgpt/desktop_protocol.mjs'],'python'),
                           (['providers/chatgpt/desktop_protocol.mjs','crates/core/src/main.rs'],'full'),
                           (['providers/chatgpt-unknown/file.py'],'full'),
                           (['unknown'], 'full')]:
            with self.subTest(paths=paths):self.assertEqual(scope(paths),want)

    def test_nul_paths_without_shell_splitting(self):
        script=Path(__file__).with_name('ci_scope.py')
        result=subprocess.run([sys.executable,str(script),'--stdin'],input=b'NOTES with spaces.md\0crates/odd name.rs\0',capture_output=True)
        self.assertEqual(result.returncode,0)
        self.assertEqual(result.stdout,b'full\n')


if __name__=='__main__':unittest.main()
