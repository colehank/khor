"""Mutable source development; immutable review continues to use lab start/update."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import socket
import subprocess
import time

from .artifacts import write_json
from .contract import generated_digest


def development_plan(config, home, *, target='desktop', device=None):
    if config['builder'] != 'local':
        raise ValueError('lab dev runs source on this computer; use --builder local here or on your development worker. lab start --builder remote provides fixed review builds')
    if target == 'ios' and (config['mode'] != 'full' or platform.system() != 'Darwin'):
        raise ValueError('iOS development requires --mode full on a Mac')
    if device and target != 'ios':
        raise ValueError('--device belongs to --target ios')
    runtime = dict(mode=config['mode'], scene=config['scene'], tree='working-source',
                   revision=str(time.time_ns()), mutable=True)
    vite = ['npx', '--no-install', 'vite', '--config', 'dev/lab/vite.config.ts',
            '--port', str(config['port']), '--host', '127.0.0.1']
    identifier = 'io.github.colehank.khor.developer-lab.d' + hashlib.sha256(str(home).encode()).hexdigest()[:12]
    tauri = dict(identifier=identifier, productName='KhorDeveloperLab',
                 build=dict(devUrl=f'http://127.0.0.1:{config["port"]}/app/',
                            beforeDevCommand=''))
    command = vite
    if config['mode'] == 'full':
        command = ['npx', '--no-install', 'tauri']
        command += ['ios', 'dev', device] if target == 'ios' else ['dev']
        command += ['--config', str(home / 'tauri.json'), '--no-watch', '--features', 'devtools']
    return dict(command=command, vite=vite, runtime=runtime, tauri=tauri, target=target,
                url=f'http://127.0.0.1:{config["port"]}/app/')


def prepare(root, home, environment):
    from .builders import dependency_inputs, local_dependencies, run
    gui = root / 'apps/gui'
    if not shutil.which('npm'):
        raise ValueError('lab dev requires Node/npm on its execution computer; this does not install toolchains')
    modules = gui / 'node_modules'
    if not modules.exists() or modules.is_symlink():
        if modules.is_symlink() and not (modules.resolve().parent / '.lab-install.json').is_file():
            raise ValueError('Preserving node_modules linked to another tool; install project dependencies or select its original development cache')
        dependencies = local_dependencies(gui, home)
        target = dependencies / 'node_modules'
        if modules.is_symlink() and modules.resolve() != target.resolve():
            modules.unlink()
        if not modules.exists():
            modules.symlink_to(target, target_is_directory=True)
    else:
        key, _linked = dependency_inputs(gui)
        receipt = home / 'dependencies.json'
        if not receipt.exists() or json.loads(receipt.read_text()).get('key') != key:
            run('npm', 'ci', cwd=gui, env=environment, timeout=900)
            write_json(receipt, {'key': key})
    stamp = home / 'generated.json'
    contract = generated_digest(root)
    generated = gui / 'src/gen'
    if not stamp.exists() or json.loads(stamp.read_text()).get('contract') != contract or not (generated / 'catalog.ts').exists():
        run('npm', 'run', 'gen', cwd=gui, env=environment, timeout=1800)
        write_json(stamp, {'contract': contract})


def development_home(config):
    root = Path(config['root'])
    key = hashlib.sha256(str(root).encode()).hexdigest()[:16] + '-' + str(config['port'])
    return Path(config['cache_root']) / (key + '-dev')


def develop(config, *, target='desktop', device=None, prepare_only=False):
    root = Path(config['root'])
    home = development_home(config)
    plan = development_plan(config, home, target=target, device=device)
    if target == 'ios':
        device = simulator(device)
        plan = development_plan(config, home, target=target, device=device)
        for tool in ('xcodebuild', 'xcodegen', 'pod'):
            if not shutil.which(tool):
                raise ValueError(f'iOS development requires {tool}; install the Xcode/Tauri iOS prerequisites on this development Mac')
    with socket.socket() as listener:
        try:
            listener.bind(('127.0.0.1', config['port']))
        except OSError:
            raise ValueError(f'Port {config["port"]} is already occupied; stop its Lab or choose another --port before preparing dependencies') from None
    home.mkdir(parents=True, exist_ok=True)
    home.chmod(0o700)
    environment = dict(os.environ, KHOR_LAB_RUNTIME=str(home / 'runtime.json'),
                       KHOR_HOME=str(home / 'store'), CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR', str(home / 'native-target')),
                       CARGO_PROFILE_DEV_DEBUG='0')
    if target == 'ios':
        environment.pop('KHOR_HOME', None)
    write_json(home / 'runtime.json', plan['runtime'])
    write_json(home / 'tauri.json', plan['tauri'])
    prepare(root, home, environment)
    native = None
    if config['mode'] == 'full':
        from .workspace import materialize
        native = materialize(config, home, environment, plan)
    receipt = {'url': plan['url'], 'mutable': True, 'mode': config['mode'], 'native_source': str(native) if native else None,
                      'target': target, 'store': environment.get('KHOR_HOME', 'isolated iOS application container'),
                      'identifier': plan['tauri']['identifier'], 'device': device,
                      'review': 'Use lab start/update to create an immutable review version'}
    write_json(home / 'development.json', receipt)
    print(json.dumps(receipt, ensure_ascii=False), flush=True)
    if prepare_only:
        return
    children = []
    interrupted = False
    def stop(_signal=None, _frame=None):
        nonlocal interrupted
        interrupted |= _signal is not None
        for child in reversed(children):
            if child.poll() is None:
                if os.name == 'nt':
                    subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'], check=False, capture_output=True)
                else:
                    try:
                        os.killpg(child.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
    previous = {s: signal.signal(s, stop) for s in (signal.SIGINT, signal.SIGTERM)}
    try:
        children.append(subprocess.Popen(plan['vite'], cwd=root / 'apps/gui', env=environment, start_new_session=True))
        if native:
            children.append(subprocess.Popen(plan['command'], cwd=native / 'apps/gui', env=environment, start_new_session=True))
        deployed = False
        while not interrupted:
            exited = next((child for index, child in enumerate(children) if child.poll() is not None
                           and not (index == 1 and target == 'ios' and child.returncode == 0)), None)
            if exited is not None:
                raise ValueError(f'Source development stopped unexpectedly ({exited.returncode}); inspect the terminal output')
            if target == 'ios' and len(children) > 1 and children[1].poll() == 0 and not deployed:
                deployed = True
                print('iOS deployment completed; keeping the frontend hot server alive until Ctrl+C', flush=True)
            time.sleep(.1)
    finally:
        stop()
        for child in children:
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                if os.name != 'nt':
                    os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=5)
        if native and target == 'ios':
            subprocess.run(['xcrun', 'simctl', 'terminate', device, plan['tauri']['identifier']], capture_output=True, timeout=30)
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def simulator(device):
    if platform.system() != 'Darwin':
        raise ValueError('iOS simulators require a Mac')
    result = json.loads(subprocess.check_output(['xcrun', 'simctl', 'list', 'devices', 'available', '--json'], text=True, timeout=30))
    devices = [d for runtime, values in result['devices'].items() if '.iOS-' in runtime for d in values if d.get('isAvailable')]
    matches = [d for d in devices if device in (d['name'], d['udid'])] if device else []
    if len(matches) != 1:
        choices = ', '.join(f'{d["name"]} ({d["udid"]})' for d in devices)
        raise ValueError('Choose an available iOS simulator with --device: ' + (choices or 'install an iOS runtime in Xcode'))
    name = matches[0]['name']
    if sum(d['name'] == name for d in devices) != 1:
        raise ValueError('Give the selected simulator a unique name in Xcode; Tauri selects simulators by name')
    return name
