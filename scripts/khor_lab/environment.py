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
        if key in {'KHOR_SESSION', 'CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'PI_CODING_AGENT_DIR',
                   'PI_AGENT_DIR', 'ZDOTDIR', 'BASH_ENV', 'ENV', 'TMUX', 'STY', 'SSH_AUTH_SOCK'} \
                or key.endswith(('_API_KEY', '_ACCESS_TOKEN', '_AUTH_TOKEN', '_SECRET_KEY')):
            environment.pop(key)
    environment.update(HOME=str(profile), KHOR_HOME=str(home / 'store'), KHOR_NAME=name, KHOR_WEB_PORT='0',
                       XDG_CONFIG_HOME=str(profile / '.config'), XDG_CACHE_HOME=str(profile / '.cache'),
                       XDG_DATA_HOME=str(profile / '.local/share'))
    return environment
