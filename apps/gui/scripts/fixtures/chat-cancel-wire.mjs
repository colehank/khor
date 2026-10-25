// Browser-side transport fixture. It never calls a user backend or filesystem.
export function installChatCancelWire({scenario, http = false}) {
  const empty = {frames: [], next: 0, gone: false, lost: false, sending: 0, sending_of: 0, stall: null};
  const wire = window.__cancelWire = {
    freshNextOpen: false, calls: [], sends: [], polls: [], revoked: [], created: [], history: new Map(),
    emit(id, frames, gone = false, change = {}) {
      const poll = wire.polls.find(p => !p.done && p.id === id);
      if (!poll) throw new Error(`no held poll for ${id}`);
      const history = wire.history.get(id);
      history.frames.push(...frames); history.gone = gone;
      poll.done = true; poll.resolve({...empty, next: history.frames.length, frames, gone, ...change});
    },
    cancel(index, reconnect = false) {
      const send = wire.sends[index];
      wire.emit(send.args.id, [{kind: 'send_cancelled', token: send.args.token ?? null, reconnect}], reconnect);
      send.reject(new Error('synthetic cancellation completion'));
    },
    commit(index) {
      const send = wire.sends[index];
      wire.emit(send.args.id, [{kind: 'send_committed', token: send.args.token}]);
      send.resolve(null);
    },
  };
  const revoke = URL.revokeObjectURL.bind(URL), create = URL.createObjectURL.bind(URL);
  URL.revokeObjectURL = url => { wire.revoked.push(url); revoke(url); };
  URL.createObjectURL = file => { const url = create(file); wire.created.push(url); return url; };
  const invoke = async (cmd, args = {}, options = {}) => {
    wire.calls.push({cmd, args: cmd === 'chat_put' ? {name: options.headers?.['x-khor-name']} : args});
    if (cmd === 'chat_open') {
      const previous = wire.history.get(args.id);
      if (previous?.gone || previous?.frames.some(frame => frame.kind === 'send_cancelled' && frame.reconnect)) wire.history.delete(args.id);
      const fresh = wire.freshNextOpen; wire.freshNextOpen = false; return fresh;
    }
    if (cmd === 'chat_wait' || cmd === 'chat_poll') {
      if (!wire.history.has(args.id)) wire.history.set(args.id, {frames: [{kind: 'agent', name: 'fixture', replays: false, needs_login: false, images: true, embeds: true, lists: false}], gone: false});
      const history = wire.history.get(args.id);
      if (args.since < history.frames.length || history.gone) return {...empty, next: history.frames.length, frames: history.frames.slice(args.since), gone: history.gone};
      return new Promise(resolve => wire.polls.push({id: args.id, resolve, done: false}));
    }
    if (cmd === 'chat_say') {
      if (http) { wire.sends.push({args, resolve() {}, reject() {}}); return null; }
      return new Promise((resolve, reject) => wire.sends.push({args, resolve, reject}));
    }
    if (cmd === 'chat_stop' && scenario === 'stop-error') throw new Error('synthetic Stop denied');
    if (cmd === 'chat_put') return '/fixture/inbox/' + decodeURIComponent(options.headers?.['x-khor-name'] ?? 'file');
    if (cmd === 'devices' || cmd === 'workspaces') return [];
    if (cmd === 'ls') return {path: args.path ?? '', entries: [], unavailable: false, unreported: []};
    return null;
  };
  window.__TAURI_INTERNALS__ = {
    metadata: {currentWindow: {label: 'main'}, currentWebview: {label: 'main', windowLabel: 'main'}},
    transformCallback: () => 0, unregisterCallback: () => {}, convertFileSrc: path => path, invoke,
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
  if (http) {
    const original = window.fetch.bind(window);
    window.fetch = async (input, init = {}) => {
      const url = new URL(typeof input === 'string' ? input : input.url, location.href);
      const cmd = url.pathname.split('/').pop();
      if (!cmd.startsWith('chat_')) return original(input, init);
      let value;
      if (cmd === 'chat_put') value = await invoke(cmd, {}, {headers: {'x-khor-name': url.searchParams.get('name')}});
      else value = await invoke(cmd, JSON.parse(init.body || '{}'));
      return new Response(JSON.stringify(value), {status: 200, headers: {'content-type': 'application/json'}});
    };
    // Actual App cases use ?bridge= on the same fixture server and keep the
    // native UI surface; the web case uses the ordinary /api/ fetch path.
    if (scenario === 'web-admission') delete window.__TAURI_INTERNALS__;
  }
}
