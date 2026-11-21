"""A full Lab has its own data and agent home; it is not an OS sandbox."""
import os
from pathlib import Path


def native_environment(home, name, inherited=None):
    home = Path(home).resolve()
    profile = home / 'os-home'
    profile.mkdir(parents=True, exist_ok=True, mode=0o700)
    environment = dict(os.environ if inherited is None else inherited)
    # Do not discover the developer's real transcripts or reuse ambient model
    # credentials when creating a fixture session. Explicit Lab accounts still
    # work through its own store and settings.
    for key in list(environment):
        if key in {'KHOR_SESSION', 'KHOR_RUNTIME_INSTALL_SOURCE', 'CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'PI_CODING_AGENT_DIR',
                   'PI_AGENT_DIR', 'ZDOTDIR', 'BASH_ENV', 'ENV', 'TMUX', 'STY', 'SSH_AUTH_SOCK',
                   'AWS_SHARED_CREDENTIALS_FILE', 'GOOGLE_APPLICATION_CREDENTIALS', 'NPM_CONFIG_USERCONFIG'} \
                or key.endswith(('_KEY', '_KEY_ID', '_TOKEN', '_SECRET', '_PASSWORD')):
            environment.pop(key)
    environment.update(HOME=str(profile), KHOR_HOME=str(home / 'store'), KHOR_NAME=name, KHOR_WEB_PORT='0',
                       KHOR_RUNTIME_DIR=str(home / 'runtime'),
                       XDG_CONFIG_HOME=str(profile / '.config'), XDG_CACHE_HOME=str(profile / '.cache'),
                       XDG_DATA_HOME=str(profile / '.local/share'))
    # A WebView2 override would force the app and every borrowed exit to use
    # one environment. The app instead assigns profiles beneath KHOR_HOME.
    environment.pop('WEBVIEW2_USER_DATA_FOLDER', None)
    desktop = {'APPDATA': profile / 'AppData/Roaming',
               'LOCALAPPDATA': profile / 'AppData/Local'}
    for key, value in desktop.items():
        environment[key] = str(value)
        value.mkdir(parents=True, exist_ok=True, mode=0o700)
    return environment
