import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const path = new URL('../src/views/DetailPane.tsx', import.meta.url);
const source = readFileSync(path, 'utf8');
const file = ts.createSourceFile(path.pathname, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
assert.equal(file.parseDiagnostics.length, 0, 'real detail view has valid TSX syntax');
const declaration = file.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'ObservedSession');
assert(declaration, 'real observation component exists');
const compiled = ts.transpile(declaration.getText(file), { target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.React });

const terminalDeclaration = file.statements.flatMap(node => ts.isVariableStatement(node) ? [...node.declarationList.declarations] : [])
  .find(node => node.name.getText(file) === 'isTerminal');
assert(terminalDeclaration?.initializer);
const terminalSource = ts.transpile(`const isTerminal = ${terminalDeclaration.initializer.getText(file)};`, { target: ts.ScriptTarget.ES2022 });
const isTerminal = Function(terminalSource + '\nreturn isTerminal;')();

const observedDeclaration = file.statements.flatMap(node => ts.isVariableStatement(node) ? [...node.declarationList.declarations] : [])
  .find(node => node.name.getText(file) === 'isObserved');
assert(observedDeclaration?.initializer);
const observedSource = ts.transpile(`const isObserved = ${observedDeclaration.initializer.getText(file)};`, {target: ts.ScriptTarget.ES2022});
const isObserved = Function(observedSource + '\nreturn isObserved;')();
const statusPath = new URL('../src/components/StatusWord.tsx', import.meta.url);
const statusFile = ts.createSourceFile(statusPath.pathname, readFileSync(statusPath, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const statusDeclaration = statusFile.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'StatusWord');
assert(statusDeclaration);
const statusSource = ts.transpile(statusDeclaration.getText(statusFile).replace(/^export /, ''), {target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.React});
const StatusWord = Function('React', 'cn', 'word', statusSource + '\nreturn StatusWord;')(
  {createElement: (type, props, ...children) => ({type, props: props ?? {}, children})},
  (...classes) => classes.filter(Boolean).join(' '), key => key === null ? 'Unknown execution' : key,
);
const wordsPath = new URL('../src/words.ts', import.meta.url);
const wordsFile = ts.createSourceFile(wordsPath.pathname, readFileSync(wordsPath, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const wordDeclaration = wordsFile.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'word');
assert(wordDeclaration);
const wordSource = ts.transpile(wordDeclaration.getText(wordsFile).replace(/^export /, ''), {target: ts.ScriptTarget.ES2022});
const executionWord = Function('gui', 'state', wordSource + '\nreturn word;')(
  {session_state_unknown: 'Unknown execution', thread_unbound: 'Unbound thread'}, {busy: 'Busy'},
);
const workspacePath = new URL('../src/views/WorkspacePane.tsx', import.meta.url);
const workspaceFile = ts.createSourceFile(workspacePath.pathname, readFileSync(workspacePath, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let busyEffect;
function findBusyEffect(node) {
  if (ts.isCallExpression(node) && node.expression.getText(workspaceFile) === 'useEffect') {
    const callback = node.arguments[0];
    if (callback?.getText(workspaceFile).includes('said.current') && callback.getText(workspaceFile).includes('setGitAt')) busyEffect = callback;
  }
  ts.forEachChild(node, findBusyEffect);
}
findBusyEffect(workspaceFile); assert(busyEffect, 'real workspace refresh effect exists');
const busySource = ts.transpile(`const effect = ${busyEffect.getText(workspaceFile)};`, {target: ts.ScriptTarget.ES2022});
const importPath = new URL('../src/views/ImportDialog.tsx', import.meta.url);
const importSource = ts.createSourceFile(importPath.pathname, readFileSync(importPath, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const markDeclaration = importSource.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'markOf');
assert(markDeclaration);
const markSource = ts.transpile(markDeclaration.getText(importSource), {target: ts.ScriptTarget.ES2022});
const markOf = Function(markSource + '\nreturn markOf;')();

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
  ['journal capture scope is distinct from execution and keeps the real read-only reader', async () => {
    const f=fixture({word:null,kind:'observe',can_control:false});
    f.render();
    f.requests[0].resolve({kind:'available',following:true,capture:{opening:'resume',gaps:2}});
    await flush();
    const tree=f.render();
    const header=tree.children[0],reader=tree.children[1];
    assert.equal(header.props['data-history-capture'],true);
    assert.equal(header.children[0].children[0],'history_capture_scope');
    assert.equal(header.children.length,1,'the header does not freeze a once-queried interval count as a current total');
    assert.equal(reader.type,'real-chat-view');
    assert.equal(reader.props.still,true);
    assert.equal(reader.props.watch,true);
    f.render(); assert.equal(f.calls.length,1,'scope does not create another polling owner');
    f.dispose();
  }],
  ['workspace refresh requires a known busy-to-finished transition', async () => {
    const said = {current: new Map()}; let refreshes = 0;
    const run = word => Function('mine', 'said', 'setGitAt', busySource + '\neffect();')(
      [{id: 'session', word}], said, change => { refreshes = change(refreshes); },
    );
    run('busy'); run(null); assert.equal(refreshes, 0);
    run('idle'); assert.equal(refreshes, 0);
    run('busy'); run('done'); assert.equal(refreshes, 1);
    run('done'); assert.equal(refreshes, 1);
  }],
  ['missing execution never becomes an idle or unbound-thread word', async () => {
    assert.equal(executionWord(null), 'Unknown execution'); assert.equal(executionWord(''), 'Unknown execution');
    assert.equal(executionWord('busy'), 'Busy');
  }],
  ['an observation role is neutral text and never an execution word', async () => {
    const role = StatusWord({word: null, absent: 'Read-only record'});
    assert.equal(role.props['data-status-word'], undefined);
    assert.equal(role.props['data-status-unavailable'], true);
    assert.deepEqual(role.children, ['Read-only record']);
    const execution = StatusWord({word: 'busy', absent: 'Read-only record'});
    assert.equal(execution.props['data-status-word'], 'busy');
  }],
  ['displayed shell form never grants an outside agent input', async () => {
    const outside = {kind: 'shell', mux: true, attachable: true, can_control: false, ended: null};
    assert.equal(isTerminal(outside), false); assert.equal(isObserved(outside), true);
    assert.equal(isObserved({...outside, can_control: true}), false);
    assert.equal(isObserved({...outside, ended: {kind: 'exit', code: 0}}), false);
    assert.equal(isObserved({kind: 'observe', can_control: true}), true);
  }],
  ['positive and negative agent-list answers only apply to their execution home', async () => {
    assert.equal(markOf(undefined, 'A'), 'reading');
    assert.equal(markOf({said: 'unasked'}, 'A'), 'unasked');
    for (const said of ['cannot', 'sessions']) {
      const answer = {said, rows: [], home: 'B'};
      assert.equal(markOf(answer, 'A'), 'unasked');
      assert.equal(markOf({...answer, home: null}, 'A'), 'unknown-home');
      assert.equal(markOf({...answer, home: 'A'}, 'A'), said === 'cannot' ? 'cannot' : 'none');
    }
  }],
  ['foreign agent multiplexers cannot bypass read-only routing', async () => {
    assert.equal(isTerminal({kind: 'tui', mux: true, attachable: true, can_control: false}), false);
    assert.equal(isTerminal({kind: 'gui', mux: true, attachable: true, can_control: false}), false);
    const ownGui = {kind: 'gui', mux: true, attachable: true, can_control: true};
    assert.equal(isTerminal(ownGui), false); assert.equal(isObserved(ownGui), false);
    assert.equal(isTerminal({kind: 'observe', mux: true, attachable: true, can_control: true}), false);
    assert.equal(isTerminal({kind: 'tui', mux: true, attachable: true, can_control: true}), true);
    assert.equal(isTerminal({kind: 'shell', mux: true, attachable: true, can_control: true}), true);
  }],
  ['observation metadata owner stays separate from content home', async () => {
    const f = fixture({id: 'observe/source', kind: 'observe', observation_owner: 'metadata-owner', home: 'content-home'});
    f.render(); assert.deepEqual(f.calls, [['observe/source', 'metadata-owner']]);
    f.requests[0].resolve({kind: 'available', following: true}); await flush();
    const view = f.render(); assert.equal(view.props.machine, 'metadata-owner'); assert.equal(view.props.home, 'content-home');
  }],
  ['same observation changing metadata owner cannot reuse old capability', async () => {
    const f = fixture({id: 'observe/source', observation_owner: 'first-owner'});
    f.render(); f.requests[0].resolve({kind: 'available', following: true}); await flush();
    assert.equal(f.render().type, 'real-chat-view');
    f.updateRow({observation_owner: 'second-owner'});
    assert.equal(f.render().props['data-session-content'], 'loading');
    assert.deepEqual(f.calls.at(-1), ['observe/source', 'second-owner']); f.dispose();
  }],
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
