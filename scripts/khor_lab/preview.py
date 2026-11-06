"""Serve an already-built Lab bundle for CI or a pinned offline review."""
import argparse
from http.server import ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import tempfile

from .config import load
from .server import Handler, Lab


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--port', type=int, default=23910)
    parser.add_argument('--scene', default='gui')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='khor-pinned-lab-') as directory:
        home = Path(directory)
        manifest = json.loads((args.bundle / 'lab-manifest.json').read_text())
        shutil.copytree(args.bundle, home / 'versions/pinned')
        lab = Lab(load(Path.cwd(), {'port': args.port, 'scene': args.scene, 'mode': 'frontend', 'builder': 'local'}), home)
        lab.value.update(version='pinned', available=True, source=manifest, manifest=manifest)
        lab.save()
        with ThreadingHTTPServer(('127.0.0.1', args.port), Handler) as server:
            server.lab = lab
            server.serve_forever()


if __name__ == '__main__':
    main()
