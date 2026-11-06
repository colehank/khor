"""Build transport; all source identities come from the existing snapshot envelope."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import hashlib
import shlex
import re
import time
import tarfile
import uuid

from .artifacts import write_json, validate, prune
from .contract import digest


def run(*args, cwd=None, env=None, timeout=180):
    print('Lab:', shlex.join(map(str, args)), file=sys.stderr, flush=True)
    try:
        result = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise ValueError(f'{Path(args[0]).name} exceeded its {timeout}s execution budget; inspect manager.log') from None
    if result.returncode:
        detail = re.sub(r'(https?://[^\s?]+)\?[^\s]+', r'\1?[redacted]', result.stderr[-8000:] + result.stdout[-8000:])
        print(detail, file=sys.stderr, flush=True)
        raise ValueError(f'{Path(args[0]).name} {args[1] if len(args) > 1 else ""} failed (exit {result.returncode}); inspect the build log')
    return result.stdout.strip()


def source(config):
    import dev
    os.chdir(config['root'])
    info = dev.snapshot()
    info['native_contract'] = digest(Path(config['root']))
    return info


def local_dependencies(gui, home):
    gui = gui.resolve()
    lock = json.loads((gui / 'package-lock.json').read_text())
    key = hashlib.sha256((gui / 'package-lock.json').read_bytes() + (gui / 'package.json').read_bytes())
    linked = []
    for package in lock['packages'].values():
        resolved = package.get('resolved', '')
        if not package.get('link') or resolved.startswith('node_modules/'):
            continue
        target = (gui / resolved).resolve()
        if not target.is_relative_to(gui.resolve()) or not target.is_dir():
            raise ValueError(f'Local npm link is outside the GUI source or missing: {resolved}')
        linked.append((resolved, target))
        for path in sorted(p for p in target.rglob('*') if p.is_file()):
            key.update(str(path.relative_to(gui)).encode() + b'\0' + path.read_bytes())
    dependency_key = key.hexdigest()
    dependencies = home / 'dependencies' / dependency_key
    if not (dependencies / 'node_modules').is_dir() or not (dependencies / '.lab-install.json').is_file():
        dependencies.mkdir(parents=True, exist_ok=True)
        for name in ('package.json', 'package-lock.json'):
            shutil.copyfile(gui / name, dependencies / name)
        for relative, target in linked:
            shutil.copytree(target, dependencies / relative, dirs_exist_ok=True)
        run('npm', 'ci', cwd=dependencies, timeout=900)
        write_json(dependencies / '.lab-install.json', {'key': dependency_key})
    prune(dependencies.parent, 2, [dependency_key])
    return dependencies


def local(config, destination, publish, cancelled):
    info = source(config)
    materialized = destination / '_source'
    materialized.mkdir()
    archive = destination / 'source.tar'
    with archive.open('wb') as output:
        subprocess.run(['git', '-C', config['root'], 'archive', info['snapshot_commit'] + ':source'], stdout=output, check=True)
    with tarfile.open(archive) as source_archive:
        source_archive.extractall(materialized, filter='data')
    archive.unlink()
    run('git', 'init', '-q', str(materialized))
    run('git', '-C', str(materialized), 'add', '--force', '--all')
    if run('git', '-C', str(materialized), 'write-tree') != info['tree']:
        raise ValueError('Materialized local source differs from the requested tree')
    run('git', '-C', str(materialized), '-c', 'user.name=Khor Lab', '-c', 'user.email=lab@localhost',
        '-c', 'commit.gpgsign=false', 'commit', '-qm', 'Exact developer Lab source')
    # Materialization retains the exact tree while dependencies stay in a bounded cache.
    gui = materialized / 'apps/gui'
    if not shutil.which('npm'):
        raise ValueError('Local builder requires Node/npm. Select --builder remote on a source-only computer')
    dependencies = local_dependencies(gui, destination.parent.parent)
    (gui / 'node_modules').symlink_to(dependencies / 'node_modules', target_is_directory=True)
    build_env = dict(os.environ, CARGO_TARGET_DIR=str(destination.parent.parent / 'native-target'))
    run('npm', 'run', 'gen', cwd=gui, env=build_env, timeout=1800)
    run('npx', 'tsc', '--project', 'dev/lab/tsconfig.json', cwd=gui)
    run('npx', 'vite', 'build', '--config', 'dev/lab/vite.config.ts', '--outDir', str(destination), cwd=gui, timeout=900)
    receipt = destination / 'source.json'
    write_json(receipt, info)
    run(sys.executable, str(materialized / 'scripts/khor_lab/package.py'), str(materialized), str(destination), str(receipt))
    publish('frontend', validate(destination, info), info)
    if config['mode'] == 'full' and config.get('native_required', True) and not cancelled():
        env = dict(build_env, KHOR_LAB_PORT=str(config['port']))
        run(sys.executable, str(materialized / 'scripts/khor_lab/native.py'), str(materialized), str(destination / 'native'), str(receipt), str(destination), cwd=materialized, env=env, timeout=7200)
        publish('native', json.loads((destination / 'native/native-manifest.json').read_text()), info)
    shutil.rmtree(materialized)
    return {'status': 'completed', 'conclusion': 'built-locally', 'checks': 'not-run'}


def remote(config, destination, publish, cancelled):
    import dev
    repository = config['builder_repo']
    if not json.loads(run('gh', 'api', f'repos/{repository}'))['private']:
        raise ValueError('Source snapshots require a private build-control repository')
    resume = config.pop('resume_request', None)
    if resume:
        info, active = resume['source'], resume['run']
        request = info['request_id']
        write_json(destination / 'request.json', {**info, 'run': active})
        publish('requested', active, info)
    else:
        info = source(config)
        request = uuid.uuid4().hex
        info.update(repository=repository, request_id=request, snapshot_ref='dev-snapshots/' + request)
        write_json(destination / 'request.json', info)
        try:
            dev.upload_snapshot(info)
        except subprocess.CalledProcessError:
            # A push response can fail after GitHub accepted the unique request ref.
            try:
                actual = json.loads(run('gh', 'api', f'repos/{repository}/git/ref/heads/{info["snapshot_ref"]}'))['object']['sha']
            except ValueError:
                actual = None
            if actual != info['snapshot_commit']:
                time.sleep(2)
                dev.upload_snapshot(info)
        run('gh', 'workflow', 'run', 'developer-lab.yml', '--repo', repository, '--ref', 'main',
            '-f', 'snapshot_sha=' + info['snapshot_commit'], '-f', 'request_id=' + request,
            '-f', 'mode=' + config['mode'], '-f', 'port=' + str(config['port']),
            '-f', 'native_required=' + str(config.get('native_required', True)).lower())
        active = None
    started = time.monotonic()
    acquired = set()
    while not cancelled():
        if time.monotonic() - started > 7200:
            raise ValueError('Lab build exceeded two hours; inspect GitHub before requesting another')
        if active is None:
            candidates = json.loads(run('gh', 'run', 'list', '--repo', repository, '--workflow', 'developer-lab.yml',
                                       '--limit', '30', '--json', 'databaseId,displayTitle,url'))
            active = next((item for item in candidates if request in item['displayTitle']), None)
            if active is None:
                time.sleep(3)
                continue
            write_json(destination / 'request.json', {**info, 'run': active})
            publish('requested', active, info)
        run_id = str(active['databaseId'])
        artifacts = json.loads(run('gh', 'api', f'repos/{repository}/actions/runs/{run_id}/artifacts'))['artifacts']
        for name, kind, directory in [('developer-lab-frontend', 'frontend', destination), ('developer-lab-native', 'native', destination / 'native')]:
            if name in acquired or not any(a['name'] == name and not a['expired'] for a in artifacts):
                continue
            directory.mkdir(parents=True, exist_ok=True)
            from .download import acquire
            artifact = next(a for a in artifacts if a['name'] == name and not a['expired'])
            acquire(repository, artifact, directory, lambda received, total: publish('progress', {'kind': kind, 'received': received, 'total': total}, info), cache=destination.parent.parent / 'downloads')
            manifest = validate(directory, info) if kind == 'frontend' else json.loads((directory / 'native-manifest.json').read_text())
            if kind == 'native' and any(manifest.get(key) != info[key] for key in ('tree', 'snapshot_commit', 'native_contract')):
                raise ValueError('Native Lab provenance differs from the requested source')
            acquired.add(name)
            publish(kind, manifest, info)
        status = json.loads(run('gh', 'run', 'view', run_id, '--repo', repository, '--json', 'status,conclusion,url'))
        if status['status'] == 'completed':
            if config['mode'] == 'full' and config.get('native_required', True) and 'developer-lab-native' not in acquired:
                raise ValueError(f'Native host unavailable; inspect {active["url"]}')
            if 'developer-lab-frontend' not in acquired:
                raise ValueError(f'Frontend bundle unavailable; inspect {active["url"]}')
            return status
        time.sleep(8)
    return {'status': 'cancelled', 'checks': 'not-completed'}
