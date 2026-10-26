#!/usr/bin/env python3
"""Keep the standalone installer embedded copy identical to its only source."""
import argparse
from pathlib import Path
import subprocess
import sys


def synchronize(write=False):
    root = Path(__file__).resolve().parent
    installer = root / "install.sh"
    canonical = root / "install-components.sh"
    source = installer.read_bytes().decode("utf-8")
    begin = "# KHOR_INSTALL_DECLARATIONS_BEGIN\n"
    main = "khor_install_main() {\n"
    if source.count(begin) != 1 or source.count(main) != 1:
        raise ValueError("installer embedding boundaries changed")
    prefix, remainder = source.split(begin)
    _, body = remainder.split(main)
    generated = prefix + begin + canonical.read_bytes().decode("utf-8") + main + body
    subprocess.run(["/bin/sh", "-n", str(canonical)], check=True)
    if source == generated:
        return 0
    if write:
        installer.write_bytes(generated.encode("utf-8"))
        return 0
    print("installer components differ; run python3 scripts/sync-install-components.py --write", file=sys.stderr)
    return 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    sys.exit(synchronize(parser.parse_args().write))
