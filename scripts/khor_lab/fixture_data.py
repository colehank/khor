"""Acquire domain-generated Lab records; ordinary UI edits reuse the checked data."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

OUTPUT = Path('apps/gui/dev/lab/data/world.generated.json')
INPUTS = (
    'Cargo.lock',
    'crates/core/src/avatar.rs',
    'crates/core/src/avatar/mark.rs',
    'crates/node/src/files.rs',
    'apps/gui/core/src/bin/lab_data.rs',
    'apps/gui/core/src/qr.rs',
    'apps/gui/core/src/lib.rs',
    'crates/node/src/config.rs',
    'crates/catalog/zh.toml',
    'crates/catalog/en.toml',
)


def source_inputs(root):
    sources = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in INPUTS}
    sources.update({str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
                    for path in sorted((root / 'apps/gui/dev/lab/data/workspace').rglob('*')) if path.is_file()})
    return sources


def validate(data):
    ids = ['e' * 64] + [format(index, 'x') * 64 for index in range(2, 7)]
    if data.get('schema') != 1 or data.get('machine_ids') != ids or set(data.get('avatars', {})) != set(ids):
        raise ValueError('Lab data must contain the six generated machine identities')
    if not all(0 <= face['radius_ratio'] <= .5 and face['mark']['ink'] in ('light', 'dark')
               for face in data['avatars'].values()):
        raise ValueError('Lab avatar records are incomplete')
    if not {'', 'project', 'project/src', 'project/docs', 'project/.config'} <= data.get('directories', {}).keys():
        raise ValueError('Lab directory records are incomplete')
    if not {'project/README.md', 'project/src/navigation.ts', 'project/docs/中文说明.md'} <= data.get('documents', {}).keys():
        raise ValueError('Lab file records are incomplete')
    ticket = data.get('pairing_ticket')
    if (not isinstance(ticket, dict) or not isinstance(ticket.get('token'), str)
            or not ticket['token'].startswith('khorLAB_ONLY_') or not isinstance(ticket.get('qr'), dict)
            or not ticket['qr'].get('path')):
        raise ValueError('Lab must contain a native-encoded non-production pairing QR code')

    settings = data.get('settings', {})
    for language in ('zh', 'en'):
        rows = settings.get(language, [])
        if not rows or not all(row.get('name') and row.get('label') and row.get('choices') for row in rows):
            raise ValueError('Lab settings require localized native configuration rows')
    if [row['name'] for row in settings['zh']] != [row['name'] for row in settings['en']]:
        raise ValueError('Lab settings languages must share the same native keys')


def payload_checksum(data):
    payload = {key: value for key, value in data.items() if not key.startswith('_')}
    encoded = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    return hashlib.sha256(encoded).hexdigest()


def cache_record(data, root):
    return data | {'_source': source_inputs(root), '_payload_sha256': payload_checksum(data)}


def ready(root):
    try:
        data = json.loads((root / OUTPUT).read_text())
        validate(data)
        return data.get('_source') == source_inputs(root) and data.get('_payload_sha256') == payload_checksum(data)
    except (OSError, ValueError, KeyError, TypeError):
        return False


def ensure(root):
    if ready(root):
        return
    result = subprocess.run(['cargo', 'run', '--locked', '-q', '-p', 'khor-gui-core', '--bin', 'lab_data'],
                            cwd=root, check=True, stdout=subprocess.PIPE, text=True)
    data = json.loads(result.stdout)
    validate(data)
    data = cache_record(data, root)
    output = root / OUTPUT
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix('.tmp')
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')
    temporary.replace(output)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--github-output', type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    current = ready(root)
    if args.github_output:
        with args.github_output.open('a') as output:
            output.write('ready=' + str(current).lower() + '\n')
    elif args.check:
        raise SystemExit(0 if current else 1)
    else:
        ensure(root)


if __name__ == '__main__':
    main()
