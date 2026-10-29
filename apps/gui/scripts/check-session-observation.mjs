import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const path = new URL('../src/views/DetailPane.tsx', import.meta.url);
const source = readFileSync(path, 'utf8');
const file = ts.createSourceFile(path.pathname, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const declaration = file.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'ObservedSession');
assert(declaration, 'real observation component exists');
const compiled = ts.transpile(declaration.getText(file), { target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.React });
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const flush = () => new Promise(resolve => setImmediate(resolve));
function fixture(row = {}) {
  const values = [], dependencies = [], cleanups = [], effects = [], requests = [], calls = [];
  let stateIndex = 0, effectIndex = 0, updates = 0;
  let currentRow = {id: 'tui/session', home: 'local', word: 'idle', category: 'custom-acp-agent', ...row};
  const bindings = {
    useState(initial) {
      const index = stateIndex++;
      if (!(index in values)) values[index] = initial;
      return [values[index], next => { values[index] = typeof next === 'function' ? next(values[index]) : next; updates++; }];
    },
    useEffect(callback, next) {
      const index = effectIndex++;
      if (!dependencies[index] || next.some((value, n) => value !== dependencies[index][n])) {
        cleanups[index]?.();
        dependencies[index] = next;
        effects.push(() => { cleanups[index] = callback(); });
      }
    },
    fetchSessionContent(...args) {
      calls.push(args);
      const request = deferred(); requests.push(request); return request.promise;
    },
    React: { createElement: (type, props, ...children) => ({ type, props: props ?? {}, children }) },
    ChatView: 'real-chat-view', StatusWord: 'status-word', Button: 'button',
    gui: new Proxy({}, { get: (_, name) => name }),
  };
  const component = Function(...Object.keys(bindings), compiled + '\nreturn ObservedSession;')(...Object.values(bindings));
  return {
    requests, calls, updates: () => updates,
    updateRow: next => { currentRow = {...currentRow, ...next}; },
    render() {
      stateIndex = 0; effectIndex = 0;
      const result = component({ row: currentRow });
      for (const effect of effects.splice(0)) effect();
      return result;
    },
    dispose() { for (const cleanup of cleanups) cleanup?.(); },
  };
}
function findButton(node) {
  if (!node || typeof node !== 'object') return null;
  if (node.type === 'button') return node;
  for (const child of node.children ?? []) {
    const found = Array.isArray(child) ? child.map(findButton).find(Boolean) : findButton(child);
    if (found) return found;
  }
  return null;
}
const cases = [
  ['any registered agent routes by actual readable source, never by vendor', async () => {
    const f = fixture();
    assert.equal(f.render().props['data-session-content'], 'loading');
    f.requests[0].resolve({kind: 'available', following: true}); await flush();
    const view = f.render(); assert.equal(view.type, 'real-chat-view'); assert.equal(view.props.still, true);
    assert.equal(view.props.category, 'custom-acp-agent'); assert.equal(view.props.watch, true); assert.deepEqual(f.calls, [['tui/session', null]]);
  }],
  ['snapshot source renders existing content without claiming a follow capability', async () => {
    const f = fixture(); f.render(); f.requests[0].resolve({kind: 'available', following: false}); await flush();
    assert.equal(f.render().props.watch, false);
  }],
  ['source unavailable is explicit and retry rechecks only this opened session', async () => {
    const f = fixture(); f.render(); f.requests[0].resolve({kind: 'unavailable', reason: 'no_source'}); await flush();
    const view = f.render(); assert.equal(view.props['data-session-content'], 'unavailable');
    findButton(view).props.onClick(); f.render(); assert.equal(f.requests.length, 2); assert.equal(f.calls[1][0], 'tui/session');
    f.dispose(); f.requests[1].resolve({kind: 'available', following: true});
  }],
  ['remote home is sent intact and cannot fall through to local file guessing', async () => {
    const f = fixture({source: {machine: 'remote'}, home: 'Remote Device'}); f.render();
    assert.deepEqual(f.calls, [['tui/session', 'Remote Device']]);
    f.requests[0].resolve({kind: 'unavailable', reason: 'remote_not_connected'}); await flush();
    assert.equal(f.render().props['data-session-content'], 'unavailable');
  }],
  ['same ID changing home cannot reuse a previous readable capability', async () => {
    const f = fixture(); f.render(); f.requests[0].resolve({kind: 'available', following: true}); await flush();
    assert.equal(f.render().type, 'real-chat-view');
    f.updateRow({source: {machine: 'new-home'}, home: 'New Home'});
    assert.equal(f.render().props['data-session-content'], 'loading');
    assert.deepEqual(f.calls.at(-1), ['tui/session', 'New Home']);
    f.dispose(); f.requests.at(-1).resolve({kind: 'unavailable', reason: 'remote_not_connected'});
  }],
  ['leaving during probe rejects late state without starting live control', async () => {
    const f = fixture(); f.render(); f.dispose(); const updates = f.updates();
    f.requests[0].resolve({kind: 'available', following: true}); await flush(); assert.equal(f.updates(), updates);
  }],
  ['query failure is visible and remains retryable', async () => {
    const f = fixture(); f.render(); f.requests[0].reject(new Error('home unavailable')); await flush();
    const view = f.render(); assert.equal(view.props['data-session-content'], 'failed'); assert(findButton(view));
  }],
];
for (const [name, run] of cases) { await run(); console.log('PASS', name); }
console.log(`${cases.length}/${cases.length} observation callback checks passed`);
