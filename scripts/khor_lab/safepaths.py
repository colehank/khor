"""Ownership checks for every path a cleanup command is allowed to touch.

Nothing in this module deletes. It answers one question per path: may the Lab
own it? A wrong answer here is the difference between clearing a build cache
and clearing a user profile, so every rejection is deliberate.
"""
import json
import os
import tempfile
from pathlib import Path

FILE_ATTRIBUTE_REPARSE_POINT = 0x400
PROBE_MARKER = '.khor-probe.json'
RESERVED_NAMES = {'con', 'prn', 'aux', 'nul'} | {'com%d' % n for n in range(1, 10)} \
    | {'lpt%d' % n for n in range(1, 10)}


class UnsafePath(ValueError):
    """Raised when a path is not provably owned by the Lab."""


def _resolved_environ(name):
    value = os.environ.get(name)
    return Path(value).expanduser().resolve() if value else None


def _denied_roots():
    roots = {Path.home().resolve(), (Path.home() / '.dsh').resolve()}
    for name in ('USERPROFILE', 'APPDATA', 'LOCALAPPDATA', 'SystemRoot', 'DSH_HOME'):
        root = _resolved_environ(name)
        if root is not None:
            roots.add(root)
    return tuple(sorted(roots))


DENIED_ROOTS = _denied_roots()

# The OS temp tree is disposable scratch. Paths there are still required to sit
# inside an allowed root, but the profile deny does not apply to them: otherwise
# a test or probe running in %TEMP% under the profile would look like user data.
TEMP_ROOTS = (Path(tempfile.gettempdir()).resolve(),)


def _within(child, parent):
    try:
        child.relative_to(parent)
        return True
    except ValueError:
        return False


def _check_component_names(raw):
    for part in raw.parts:
        if part in ('/', '\\') or len(part) == 2 and part[1] == ':':
            continue
        if part != part.rstrip(' .'):
            raise UnsafePath('trailing space or dot is not a path the Lab owns: %r' % str(raw))
        if part.split('.')[0].lower() in RESERVED_NAMES:
            raise UnsafePath('reserved device name is not a path the Lab owns: %r' % str(raw))


def assert_no_reparse(path):
    """Reject a path whose chain contains a junction or symbolic link."""
    current = Path(path)
    for part in [current, *current.parents]:
        try:
            attributes = part.lstat().st_file_attributes
        except (OSError, AttributeError):
            continue
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT:
            raise UnsafePath('reparse point in path chain: %s' % part)


def resolve_owned(path, allow_roots):
    """Return the resolved path, or raise UnsafePath when the Lab does not own it."""
    raw = Path(os.path.expandvars(str(path))).expanduser()
    _check_component_names(raw)
    assert_no_reparse(raw)
    resolved = raw.resolve()
    if resolved.anchor and str(resolved) == str(Path(resolved.anchor)):
        raise UnsafePath('drive root is never owned: %s' % resolved)
    for denied in DENIED_ROOTS:
        if _within(resolved, denied) and not any(_within(resolved, temp) for temp in TEMP_ROOTS):
            raise UnsafePath('path is inside a protected root (%s): %s' % (denied, resolved))
    for root in allow_roots:
        if _within(resolved, Path(root).expanduser().resolve()):
            return resolved
    raise UnsafePath('path is outside every allowed root: %s' % resolved)


def require_marker(directory, marker=PROBE_MARKER):
    """Return the marker record of an owned directory, or raise UnsafePath."""
    record = Path(directory) / marker
    if not record.is_file():
        raise UnsafePath('directory has no ownership marker: %s' % directory)
    try:
        data = json.loads(record.read_text(encoding='utf-8'))
    except (OSError, ValueError) as error:
        raise UnsafePath('unreadable ownership marker in %s: %s' % (directory, error))
    if not isinstance(data, dict):
        raise UnsafePath('ownership marker is not an object: %s' % directory)
    return data
