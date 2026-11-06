"""Acquire hosted artifacts without forwarding GitHub credentials to signed storage URLs."""
import hashlib
import subprocess
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener, urlopen

from .artifacts import unpack


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


def acquire(repository, artifact, destination):
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
        response = urlopen(location, timeout=60)
    archive = destination / '.download.zip'
    try:
        with response, archive.open('wb') as output:
            total = 0
            while chunk := response.read(1024 * 1024):
                total += len(chunk)
                if total > 512 * 1024 * 1024:
                    raise ValueError('Lab download exceeds its size budget')
                output.write(chunk)
        expected = artifact.get('digest')
        if expected and expected != 'sha256:' + hashlib.sha256(archive.read_bytes()).hexdigest():
            raise ValueError('GitHub artifact digest differs')
        unpack(archive, destination)
    except Exception as error:
        # Signed URLs are credentials; never include them in public status or logs.
        raise ValueError(f'Artifact acquisition failed ({type(error).__name__}); retry acquisition without rebuilding') from None
    finally:
        archive.unlink(missing_ok=True)
