from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.environment import native_environment


class LabEnvironmentTests(unittest.TestCase):
    def test_native_profile_does_not_discover_real_agent_homes_or_credentials(self):
        original = {'HOME': '/real-home', 'CODEX_HOME': '/real-codex', 'CLAUDE_CONFIG_DIR': '/real-claude',
                    'KHOR_SESSION': 'real-session', 'OPENAI_API_KEY': 'fixture-key',
                    'ANTHROPIC_AUTH_TOKEN': 'fixture-token', 'PATH': '/tools', 'LANG': 'en_US.UTF-8'}
        with tempfile.TemporaryDirectory() as directory:
            result = native_environment(Path(directory), 'Fixture Lab', original)
            self.assertEqual(result['HOME'], str(Path(directory) / 'os-home'))
            self.assertTrue(Path(result['HOME']).is_dir())
            self.assertEqual(result['KHOR_HOME'], str(Path(directory) / 'store'))
            self.assertEqual(result['PATH'], '/tools')
            self.assertEqual(result['LANG'], original['LANG'])
            for key in ['CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'KHOR_SESSION', 'OPENAI_API_KEY', 'ANTHROPIC_AUTH_TOKEN']:
                self.assertNotIn(key, result)
            self.assertEqual(original['HOME'], '/real-home', 'parent environment is untouched')


if __name__ == '__main__':
    unittest.main()
