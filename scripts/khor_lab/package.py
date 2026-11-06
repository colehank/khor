"""Write the artifact contract after Vite has built the real App Lab entry."""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.contract import digest
from khor_lab.artifacts import write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('source', type=Path)
    args = parser.parse_args()
    info = json.loads(args.source.read_text())
    write_json(args.output / 'lab-manifest.json', {**info, 'schema': 1, 'entry': 'production-app',
               'native_contract': digest(args.root), 'capabilities': ['fixtures', 'real-layout', 'browser-interaction'],
               'native_verified': False, 'user_accepted': False})


if __name__ == '__main__':
    main()
