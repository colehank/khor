"""Acquire hosted artifacts without forwarding GitHub credentials to signed storage URLs."""
import hashlib
import subprocess
from concurrent.futures import ThreadPoolExecutor
import threading
import json
import time
from urllib.error import URLError
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener, urlopen

from .artifacts import unpack, prune


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


def _ranges(location, archive, size, progress, receipt=None):
    from .artifacts import write_json
    lock = threading.Lock()
    completed = set(json.loads(receipt.read_text()) if receipt and receipt.exists() else [])
    received = sum(min(4 * 1024 * 1024, size - start) for start in completed)
    with archive.open('ab') as output:
        output.truncate(size)
    def part(start):
        nonlocal received
        if start in completed:
            return
        end = min(start + 4 * 1024 * 1024, size) - 1
        offset = start
        for attempt in range(4):
            request = Request(location, headers={'Range': f'bytes={offset}-{end}'})
            try:
                with urlopen(request, timeout=60) as response, archive.open('r+b') as output:
                    if response.status != 206 or response.headers.get('Content-Range') != f'bytes {offset}-{end}/{size}':
                        raise ValueError('Artifact server did not honor the requested range')
                    output.seek(offset)
                    while chunk := response.read(64 * 1024):
                        offset += len(chunk)
                        if offset > end + 1:
                            raise ValueError('Artifact range exceeded its boundary')
                        output.write(chunk)
                        with lock:
                            received += len(chunk)
                            progress(received, size)
                    if offset != end + 1:
                        raise OSError('Incomplete artifact range')
                with lock:
                    completed.add(start)
                    if receipt:
                        write_json(receipt, sorted(completed))
                return
            except (OSError, URLError):
                if attempt == 3:
                    raise
                time.sleep(1 + attempt)
    with ThreadPoolExecutor(max_workers=8) as pool:
        list(pool.map(part, range(0, size, 4 * 1024 * 1024)))


def acquire(repository, artifact, destination, progress=lambda _received, _total: None, cache=None):
    token = subprocess.check_output(['gh', 'auth', 'token'], text=True, timeout=20).strip()
    api = f'https://api.github.com/repos/{repository}/actions/artifacts/{int(artifact["id"])}/zip'
    request = Request(api, headers={'Authorization': 'Bearer ' + token})
    try:
        response = build_opener(NoRedirect).open(request, timeout=30)
    except HTTPError as error:
        if error.code != 302:
            error.close()
            raise ValueError(f'GitHub artifact request failed ({error.code})') from None
        location = error.headers['Location']
        error.close()
        parts = urlsplit(location)
        if parts.scheme != 'https' or not (parts.hostname or '').endswith('.blob.core.windows.net') or parts.username:
            raise ValueError('Unexpected artifact storage destination')
        # A fresh request has no GitHub authorization header.
        response = None
    cache = (cache or destination / '.downloads') / str(int(artifact['id']))
    cache.mkdir(parents=True, exist_ok=True)
    prune(cache.parent, 2, [cache.name])
    archive = cache / 'artifact.zip'
    receipt = cache / 'ranges.json'
    if not archive.exists():
        receipt.unlink(missing_ok=True)
    try:
        size = int(artifact['size_in_bytes'])
        if not 0 < size <= 512 * 1024 * 1024:
            raise ValueError('Lab download exceeds its size budget')
        if response is None and size > 16 * 1024 * 1024:
            _ranges(location, archive, size, progress, receipt)
        else:
            response = response or urlopen(location, timeout=60)
            with response, archive.open('wb') as output:
                total = 0
                while chunk := response.read(64 * 1024):
                    total += len(chunk)
                    if total > size:
                        raise ValueError('Lab download exceeds its declared size')
                    output.write(chunk)
                    progress(total, size)
                if total != size:
                    raise ValueError('Incomplete artifact download')
        expected = artifact.get('digest')
        with archive.open('rb') as stream:
            actual_digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if expected and expected != 'sha256:' + actual_digest:
            archive.unlink(missing_ok=True)
            receipt.unlink(missing_ok=True)
            raise ValueError('GitHub artifact digest differs')
        unpack(archive, destination)
        archive.unlink(missing_ok=True)
        receipt.unlink(missing_ok=True)
    except Exception as error:
        # Signed URLs are credentials; never include them in public status or logs.
        raise ValueError(f'Artifact acquisition failed ({type(error).__name__}); retry acquisition without rebuilding') from None
