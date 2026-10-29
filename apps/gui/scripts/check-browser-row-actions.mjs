// Executes the production landing menu and SideList swipe projection; no DOM/API/native.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const read = name => fs.readFileSync(new URL(`../src/${name}`, import.meta.url), 'utf8');
const landing = read('browse/BrowseLanding.tsx');
const list = read('components/SideList.tsx');
function find(source, name, property = false) {
  const file = ts.createSourceFile('source.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const matches = [];
  function visit(node) {
    if ((property ? ts.isPropertyAssignment(node) : ts.isVariableDeclaration(node)) && node.name.getText(file) === name) matches.push(node.initializer.getText(file));
    ts.forEachChild(node, visit);
  }
  visit(file);
  assert.equal(matches.length, 1, `${name} has one production definition`);
  return matches[0];
}
const compile = source => ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function fixture({ pinned = false, bare = false, name = 'page', fail = false } = {}) {
  const calls = [];
  let refused = new Map();
  const context = vm.createContext({
    Error,
    gui: { pin: 'pin', unpin: 'unpin', close: 'close', delete: 'delete' },
    kind: { kind: name, words: { copy: 'copy' } }, bare,
    t: { id: 'one', pinned, path: '', device: null },
    deviceOf: () => null, openLink: () => { throw Error('unexpected external open'); },
    navigator: { clipboard: { writeText: () => { throw Error('unexpected clipboard'); } } },
    tabs: {
      pin: async (...args) => { calls.push(['pin', ...args]); },
      close: async id => { calls.push(['close', id]); if (fail) throw Error('fixture refusal'); },
    },
    setRefused: update => { refused = update(refused); context.refused = refused; }, refused,
    narrow: true, asking: false,
  });
  vm.runInContext(compile(`const menuOf = ${find(landing, 'menuOf')}; const row = { menu: menuOf(t), swipe: ${find(landing, 'swipe', true)} }; globalThis.row = row;`), context);
  const project = vm.createContext({ narrow: true, asking: false, row: context.row });
  vm.runInContext(compile(`const swipeOf = ${find(list, 'swipeOf')}; const menuOf = ${find(list, 'menuOf')}; globalThis.actions = swipeOf(row);`), project);
  return { actions: Array.from(project.actions), menu: Array.from(context.row.menu), calls, refused: () => refused };
}
let passed = 0;
async function test(name, run) { await run(); console.log(`PASS ${name}`); passed++; }
for (const bare of [false, true]) for (const pinned of [false, true]) {
  await test(`${bare ? 'bare' : 'full'} ${pinned ? 'pinned' : 'regular'} page exposes pin and delete`, async () => {
    const f = fixture({ bare, pinned });
    assert.deepEqual(f.actions.map(a => a.key), ['pin', 'close']);
    assert.equal(f.actions[0].label, pinned ? 'unpin' : 'pin');
    assert.equal(f.actions[1].label, 'delete');
    assert.equal(f.actions[1].destructive, true);
    await f.actions[1].run();
    assert.deepEqual(f.calls, [['close', 'one']], 'one atomic close, never unpin then close');
  });
}
await test('pin still toggles the same row', async () => {
  const f = fixture({ pinned: true }); await f.actions[0].run();
  assert.deepEqual(f.calls, [['pin', 'one', false]]);
});
await test('delete rejection reaches the existing row refusal and rejects caller', async () => {
  const f = fixture({ bare: true, fail: true });
  await assert.rejects(f.actions.find(a => a.key === 'close').run(), /fixture refusal/);
  assert.equal(f.refused().get('one:close'), 'fixture refusal');
  assert.deepEqual(f.calls, [['close', 'one']]);
});
await test('non-page bare kind keeps its own close contract', async () => {
  const f = fixture({ bare: true, name: 'code' });
  assert.deepEqual(f.actions.map(a => a.key), ['close']); assert.equal(f.actions[0].label, 'close');
});
console.log(`${passed} browser row action checks passed`);
