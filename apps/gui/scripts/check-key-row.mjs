import assert from 'node:assert/strict';
import { build } from 'esbuild';
import ts from 'typescript';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';

const STORE='khor.term-key-row', values=new Map();
let refuseStorage=false;
const windowBefore=globalThis.window;
const storage={getItem:key=>values.get(key)??null,setItem:(key,value)=>{if(refuseStorage)throw Error('fixture storage unavailable');values.set(key,value)}};
globalThis.window=Object.assign(new EventTarget(),{localStorage:storage});
const alias={name:'alias',setup(b){b.onResolve({filter:/^@\//},args=>b.resolve(`./src/${args.path.slice(2)}`,{resolveDir:process.cwd(),kind:args.kind}))}};
const built=await build({stdin:{contents:`export * from './src/lib/term-key-model';export * from './src/lib/term-key-codec';export * from './src/lib/term-keys';export * from './src/lib/term-kitty';export {Terminal} from '@xterm/xterm';`,resolveDir:process.cwd(),loader:'ts'},bundle:true,write:false,format:'cjs',platform:'node',plugins:[alias]});
const module={exports:{}};Function('module','exports',built.outputFiles[0].text)(module,module.exports);
const m=module.exports, names=m.KEYS.map(key=>key.name);
const stroke=(key,mods={})=>({key,ctrl:false,alt:false,shift:false,...mods});
const custom=(id='one',changes={})=>{const entry={id:`custom:${id}`,on:true,label:'',kind:'chord',stroke:stroke('ArrowLeft',{shift:true}),...changes};if(entry.kind==='sequence')delete entry.stroke;return entry;};
const raw=entries=>JSON.stringify({version:1,entries});
const decode=value=>m.decodeKeyRow(value,names);
const refresh=()=>{const event=new Event('storage');Object.defineProperties(event,{key:{value:STORE},storageArea:{value:storage}});window.dispatchEvent(event)};
const tests=[];const test=(name,run)=>tests.push([name,run]);

test('defaults and old builtin visibility/order migrate without a second execution format',()=>{
  assert.deepEqual(m.defaultRow().filter(entry=>entry.on).map(entry=>entry.id),['esc','ctrl','tab','backtab','paste','/','-','|',':']);
  const old=JSON.stringify([{name:'tab',on:false},{name:'esc',on:true}]);
  const result=decode(old);assert(result.ok&&result.migrated);
  assert.deepEqual(result.entries.slice(0,2).map(({id,on})=>({id,on})),[{id:'tab',on:false},{id:'esc',on:true}]);
  assert.equal(result.entries.length,names.length);
  const stored=JSON.parse(m.serializeKeyRow(result.entries,names));assert.equal(stored.version,1);assert.equal(stored.entries[0].kind,'builtin');
});
test('custom actions roundtrip bounded Unicode labels and preserve sequence order',()=>{
  const entries=[custom('unicode',{label:'🙂'.repeat(32)}),custom('tmux',{kind:'sequence',label:'next',steps:[stroke('KeyB',{ctrl:true}),stroke('KeyN')]})];
  const result=decode(m.serializeKeyRow(entries,names));assert(result.ok);
  assert.equal(result.entries[0].label,'🙂'.repeat(32));
  assert.deepEqual(result.entries[1].steps.map(step=>step.key),['KeyB','KeyN']);
});
test('unknown schema, executable fields, Meta and duplicate identities are rejected',()=>{
  const bad=[
    JSON.stringify({version:2,entries:[]}),
    JSON.stringify({version:1,entries:[],script:'anything'}),
    raw([custom('one'),custom('one')]),
    raw([custom('one',{stroke:{...stroke('KeyC'),meta:true}})]),
    raw([custom('one',{stroke:stroke('Paste')})]),
    raw([custom('one',{stroke:stroke('KeyC',{ctrl:'true'})})]),
    raw([custom('one',{kind:'script',command:'anything'})]),
    raw([{id:'esc',kind:'builtin',name:'unknown',on:true}]),
  ];for(const value of bad)assert.equal(decode(value).ok,false,value);
});
test('row, label, sequence and input-byte boundaries reject before publication',()=>{
  const room=m.KEY_ROW_LIMITS.entries-names.length;
  assert(decode(raw(Array.from({length:room},(_,i)=>custom(String(i))))).ok);
  assert.equal(decode(raw(Array.from({length:room+1},(_,i)=>custom(String(i))))).ok,false);
  assert(decode(raw([custom('one',{kind:'sequence',steps:Array.from({length:16},()=>stroke('KeyN'))})])).ok);
  assert.equal(decode(raw([custom('one',{kind:'sequence',steps:[]})])).ok,false);
  assert.equal(decode(raw([custom('one',{kind:'sequence',steps:Array.from({length:17},()=>stroke('KeyN'))})])).ok,false);
  assert.equal(decode(raw([custom('one',{label:'a'.repeat(33)})])).ok,false);
  assert.equal(decode(' '.repeat(m.KEY_ROW_LIMITS.configurationBytes+1)).ok,false);
});
test('malformed saved data remains intact until explicit save and same-window publication is validated',()=>{
  const broken=JSON.stringify({version:20,entries:[]});values.set(STORE,broken);refresh();
  assert.deepEqual(m.keyRow(),m.defaultRow());assert.equal(values.get(STORE),broken);
  let events=0;const count=()=>events++;window.addEventListener('khor:term-key-row',count);
  m.setKeyRow([custom()]);assert.equal(events,1);assert.equal(JSON.parse(values.get(STORE)).version,1);
  const before=values.get(STORE);assert.throws(()=>m.setKeyRow([custom('bad',{stroke:stroke('not-a-key')})]));
  assert.equal(events,1);assert.equal(values.get(STORE),before);window.removeEventListener('khor:term-key-row',count);
});
test('storage failure keeps local edits and cross-window updates replace that cache',()=>{
  refuseStorage=true;m.setKeyRow([custom('local')]);assert.equal(m.keyRow()[0].id,'custom:local');
  refuseStorage=false;values.set(STORE,raw([custom('other')]));refresh();assert.equal(m.keyRow()[0].id,'custom:other');
  values.delete(STORE);refresh();assert.deepEqual(m.keyRow(),m.defaultRow());
});

async function terminal(stream='') {
  const term=new m.Terminal({cols:40,rows:10,allowProposedApi:true});
  const protocol=m.kittyProtocol(term,()=>{});
  if(stream)await new Promise(done=>term.write(stream,done));
  return {term,dispose(){protocol.dispose();term.dispose()}};
}
test('legacy examples and application cursor state use semantic chords',async()=>{
  for(const output of ['', '\x1b[?1h']){
    const t=await terminal(output);
    try {
      assert.equal(m.encodeStroke(t.term,stroke('ArrowLeft',{shift:true})),'\x1b[1;2D');
      assert.equal(m.encodeStroke(t.term,stroke('ArrowLeft',{ctrl:true,shift:true})),'\x1b[1;6D');
      assert.equal(m.encodeStroke(t.term,stroke('KeyC',{ctrl:true})),'\x03');
      assert.equal(m.encodeStroke(t.term,stroke('Tab',{shift:true})),'\x1b[Z');
      assert.equal(m.encodeStroke(t.term,stroke('ArrowLeft')),output?'\x1bOD':'\x1b[D');
      assert.equal(m.encodeKeyAction(t.term,{kind:'sequence',steps:[stroke('KeyB',{ctrl:true}),stroke('KeyN')]}),'\x02n');
      assert.equal(m.encodeStroke(t.term,stroke('Digit1',{shift:true})),'!');
    } finally {t.dispose()}
  }
});
test('negotiated kitty, event release and modifyOtherKeys are read at activation',async()=>{
  const t=await terminal('\x1b[>1u');
  try {
    assert.equal(m.encodeStroke(t.term,stroke('KeyC',{ctrl:true})),'\x1b[99;5u');
    await new Promise(done=>t.term.write('\x1b[<u\x1b[>4;2m',done));
    assert.equal(m.encodeStroke(t.term,stroke('Enter',{shift:true})),'\x1b[27;2;13~');
    await new Promise(done=>t.term.write('\x1b[>3u',done));
    assert.equal(m.encodeStroke(t.term,stroke('ArrowLeft',{shift:true})),'\x1b[1;2D\x1b[1;2:3D');
    await new Promise(done=>t.term.write('\x1b[<u\x1b[>4m',done));
    assert.equal(m.encodeStroke(t.term,stroke('KeyC',{ctrl:true})),'\x03');
  } finally {t.dispose()}
});
test('trial encoding has no input side effect and invalid sequences cannot send a prefix',async()=>{
  const t=await terminal(),sent=[];t.term.onData(data=>sent.push(data));
  try {
    assert.equal(m.encodeKeyAction(t.term,{kind:'chord',stroke:stroke('KeyC',{ctrl:true})}),'\x03');assert.deepEqual(sent,[]);
    assert.throws(()=>m.pressKeyAction(t.term,{kind:'sequence',steps:[stroke('KeyB',{ctrl:true}),stroke('not-a-key')]}));assert.deepEqual(sent,[]);
    m.pressKeyAction(t.term,{kind:'sequence',steps:[stroke('KeyB',{ctrl:true}),stroke('KeyN')]});assert.deepEqual(sent,['\x02n']);
  } finally {t.dispose()}
});
test('preencoded scope is per terminal and restored even when input throws',()=>{
  const other={};const target={modes:{applicationCursorKeysMode:false},input(){assert.equal(m.isEncodedKeyInput(target),true);assert.equal(m.isEncodedKeyInput(other),false);throw Error('input failed')}};
  assert.throws(()=>m.pressKeyAction(target,{kind:'chord',stroke:stroke('KeyN')}),/input failed/);
  assert.equal(m.isEncodedKeyInput(target),false);
});
test('production onData keeps locks out of custom input and preserves ordinary input processing',async()=>{
  const source=readFileSync(resolve('src/views/TerminalPane.tsx'),'utf8');
  const ast=ts.createSourceFile('pane.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);let callback;
  function visit(node){if(ts.isCallExpression(node)&&ts.isPropertyAccessExpression(node.expression)&&node.expression.name.text==='onData'&&node.expression.expression.getText(ast)==='term')callback=node.arguments[0].getText(ast);ts.forEachChild(node,visit)}visit(ast);assert(callback);
  const js=ts.transpileModule(`const handler=${callback};`,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
  const t=await terminal(),sent=[],latches=[];
  try {
    const parameters=['term','isEncodedKeyInput','isReply','replaying','isFocusReport','selectionHolds','ownSize','modsRef','MOUSE_REPORT','blink','kittyTyped','typed','latch','afterKey','send','predict'];
    const handler=Function(...parameters,`${js};return handler;`)(t.term,m.isEncodedKeyInput,()=>false,0,()=>false,()=>false,()=>{},{current:{ctrl:true,alt:true,lock:{ctrl:true,alt:true}}},/^not-a-mouse-report$/, {typed(){}},m.kittyTyped,m.typed,value=>latches.push(value),m.afterKey,value=>sent.push(value),{typed(){}});
    t.term.onData(handler);
    m.pressKeyAction(t.term,{kind:'chord',stroke:stroke('KeyN')});assert.deepEqual(sent,['n']);assert.deepEqual(latches,[]);
    t.term.input('n',true);assert.deepEqual(sent,['n','\x1b\x0e']);assert.equal(latches.length,1);
  } finally {t.dispose()}
});

try {for(const [name,run] of tests){await run();console.log(`ok ${name}`)}console.log(JSON.stringify({passed:tests.length}))}
finally {if(windowBefore===undefined)delete globalThis.window;else globalThis.window=windowBefore;}
