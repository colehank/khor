import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import apple_release as apple


class MacSigningRetryTest(unittest.TestCase):
    def exercise(self, failures, timestamp=True):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        counter = Path(temporary.name) / 'attempts'
        script = '''
import pathlib, sys
p = pathlib.Path(sys.argv[1])
n = int(p.read_text()) + 1 if p.exists() else 1
p.write_text(str(n))
print('build output is streamed', flush=True)
if n <= int(sys.argv[2]):
    print('A timestamp was expected but was not found.' if sys.argv[3] == 'yes' else 'invalid signing identity')
    print('failed to bundle project: failed codesign application: failed to sign app')
    sys.exit(1)
print('signed with secure timestamp', flush=True)
'''
        command = [sys.executable, '-c', script, str(counter), str(failures), 'yes' if timestamp else 'no']
        return command, counter

    def test_transient_timestamp_failure_recovers_in_same_build_directory(self):
        command, counter = self.exercise(1)
        output = io.StringIO()
        with contextlib.redirect_stdout(output), patch('apple_release.time.sleep'):
            apple.run_signed_mac_build(command)
        self.assertEqual(counter.read_text(), '2')
        self.assertIn('build output is streamed', output.getvalue())
        self.assertIn('signed with secure timestamp', output.getvalue())

    def test_timestamp_retry_is_bounded(self):
        command, counter = self.exercise(99)
        with contextlib.redirect_stdout(io.StringIO()), patch('apple_release.time.sleep'), self.assertRaises(RuntimeError):
            apple.run_signed_mac_build(command)
        self.assertEqual(counter.read_text(), '3')

    def test_non_timestamp_signing_errors_fail_without_retry(self):
        command, counter = self.exercise(99, timestamp=False)
        with contextlib.redirect_stdout(io.StringIO()), patch('apple_release.time.sleep') as sleep, self.assertRaises(RuntimeError):
            apple.run_signed_mac_build(command)
        self.assertEqual(counter.read_text(), '1')
        sleep.assert_not_called()


if __name__ == '__main__':
    unittest.main()
