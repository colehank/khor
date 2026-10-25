"""Run the actual Rust ownership rule through onset's isolated SSH boundary."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class StoreOwnerTest(unittest.TestCase):
    @unittest.skipUnless(shutil.which("rustc"), "store ownership integration requires rustc")
    def test_production_rules_and_uninstall_receipts(self):
        script = Path(__file__).resolve().parents[1] / "check-store-owner.py"
        with tempfile.TemporaryDirectory(prefix="khor-owner-unittest-") as directory:
            result = subprocess.run(
                [sys.executable, str(script)],
                env={**os.environ, "STORE_OWNER_OUT": directory, "PYTHONDONTWRITEBYTECODE": "1"},
                capture_output=True,
                text=True,
                timeout=180,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
