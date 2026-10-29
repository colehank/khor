import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
import {build} from 'esbuild';

// Keep the real owner callback, while withholding React's commit until asked.
const owners = ['MachineFiles', 'SideWorkspace', 'SessionContent'];
const adapters = owners.map(name => {
  const path = `src/views/${name}.tsx`;
  const source = ts.createSourceFile(path, readFileSync(path, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  let declaration, count, callback;
  function visit(node) {
    if (ts.isVariableStatement(node) && node.declarationList.declarations.some(d => ts.isArrayBindingPattern(d.name) && d.name.elements[0]?.getText(source) === 'drafts')) declaration = node.getText(source);
    if (ts.isExpressionStatement(node) && ts.isCallExpression(node.expression) && node.expression.expression.getText(source) === 'useDraftCount') count = node.getText(source);
    if (!callback && ts.isJsxAttribute(node) && node.name.getText(source) === 'onDraft' && node.initializer && ts.isJsxExpression(node.initializer)) callback = node.initializer.expression.getText(source);
    ts.forEachChild(node, visit);
  }
  visit(source);
  assert(declaration && callback, `${name} production draft state/callback located`);
  return `export function ${name}(file, draftKey=file) { ${declaration} ${count ?? ''} return {drafts, onDraft: ${callback}}; }`;
});
const fileSource=ts.createSourceFile('FileView.tsx',readFileSync('src/views/FileView.tsx','utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
let fileCount, publish, onText;
const fileDeclarations={};
function visitFile(node) {
  if(ts.isCallExpression(node)&&node.expression.getText(fileSource)==='useDraftCount') {
    let statement=node;
    while(!ts.isVariableStatement(statement)&&!ts.isExpressionStatement(statement))statement=statement.parent;
    fileCount=statement.getText(fileSource);
  }
  if(ts.isVariableStatement(node))for(const declaration of node.declarationList.declarations) {
    const name=ts.isArrayBindingPattern(declaration.name)?declaration.name.elements[0].getText(fileSource):declaration.name.getText(fileSource);
    fileDeclarations[name]=node.getText(fileSource);
    if(name==='publishDraft')publish=node.getText(fileSource);
  }
  if(ts.isJsxAttribute(node)&&node.name.getText(fileSource)==='onText')onText=node.initializer.expression.getText(fileSource);
  ts.forEachChild(node,visitFile);
}
visitFile(fileSource);
assert(fileCount&&publish&&onText,'FileBody reporting and editor callback located');
const acceptedDeclarations=['buffer','accepted','bufferLive','publishAccepted','acceptBuffer','setEdit'].map(name=>{assert(fileDeclarations[name],name);return fileDeclarations[name];}).join('\n');
adapters.push(`export function StandaloneFile(mode) { const draft={text:'A',was:'A',by:'line'}; const refresh={current:{invalidate(){}}}; const onDraft=mode==='delegated'?()=>{}:undefined; ${acceptedDeclarations} const {file,edit}=buffer; const dirty=file!==null&&edit!==null&&edit.text!==file.text; ${fileCount} ${publish} publishAccepted.current=()=>publishDraft(accepted.current.edit); return {onText:${onText}, ready:()=>acceptBuffer(was=>({...was,file:{text:'A'}}))}; }`);
const bundle = await build({stdin:{contents:`import * as hooks from './src/hooks/use-draft-count'; import {useState,useRef,useCallback} from 'react'; const {useDraftCount,useDraftState}=hooks; ${adapters.join('\n')} export {draftCount} from './src/lib/drafts';`,resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,plugins:[{name:'aliases',setup(b){b.onResolve({filter:/^@\/lib\/drafts$/},()=>({path:process.cwd()+'/src/lib/drafts.ts'}));b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:`export const useState=(...a)=>globalThis.draftFixture.state(...a); export const useRef=(...a)=>globalThis.draftFixture.ref(...a); export const useCallback=(...a)=>globalThis.draftFixture.callback(...a); export const useLayoutEffect=(...a)=>globalThis.draftFixture.effect(...a);`}));}}]});
const api = await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
function mount(name) {
  const slots = [];
  let index=0, key='one', result;
  const same = (a,b) => a && b && a.length===b.length && a.every((v,i)=>Object.is(v,b[i]));
  const fixture = {
    state(initial) {
      const slot=slots[index++] ??= {value:typeof initial==='function'?initial():initial, pending:[]};
      return [slot.value, next=>slot.pending.push(next)];
    },
    ref(current) {return slots[index++] ??= {current};},
    callback(fn,deps) {
      const at=index++, old=slots[at];
      if (!old || !same(old.deps,deps)) slots[at]={fn,deps};
      return slots[at].fn;
    },
    effect(fn,deps) {
      const slot=slots[index++] ??= {deps:null,cleanup:null,setup:null,pending:null};
      if (!same(slot.deps,deps)) slot.pending={fn,deps};
    },
  };
  function render(next=key) {key=next; index=0; globalThis.draftFixture=fixture; result=api[name](key); return result;}
  function commit() {
    for (const slot of slots) if (slot?.pending instanceof Array) {
      for (const action of slot.pending) slot.value=typeof action==='function'?action(slot.value):action;
      slot.pending=[];
    }
    render();
    for (const slot of slots) if (slot?.pending && !(slot.pending instanceof Array)) {
      slot.cleanup?.(); slot.setup=slot.pending.fn; slot.deps=slot.pending.deps; slot.pending=null; slot.cleanup=slot.setup();
    }
    return result;
  }
  render(); commit(); if(result.ready){result.ready();commit();}
  return {render,commit,unmount(){for(const s of slots)s?.cleanup?.();},replay(){for(const s of slots)s?.cleanup?.();for(const s of slots)if(s?.setup)s.cleanup=s.setup();}};
}
const results=[];
function test(name, run) {
  const live=[];
  try {run(kind=>{const owner=mount(kind);live.push(owner);return owner;});results.push({name,pass:true});}
  catch(error){results.push({name,pass:false,error:error.message});}
  finally {for(const owner of live)owner.unmount();assert.equal(api.draftCount(),0,'all test-owned registrations released');}
}
for (const name of owners) {
  test(name+' reports before React commit and preserves same-task updates', make=>{
    const owner=make(name), first=owner.render('one').onDraft, second=owner.render('two').onDraft;
    const draft={text:'unsaved'};
    first(draft); assert.equal(api.draftCount(),1,'first edit must protect Quit before React commits');
    second(draft); assert.equal(api.draftCount(),2,'second file must not overwrite accepted first edit');
    first(draft); assert.equal(api.draftCount(),2,'same draft does not double count');
    const value=owner.commit().drafts;
    assert.equal(value instanceof Map?value.size:Object.values(value).filter(Boolean).length,2,'React projection catches up to accepted collection');
    first(null); assert.equal(api.draftCount(),1,'save/discard is synchronous');
    second(null); assert.equal(api.draftCount(),0);
  });
  test(name+' retains hidden drafts, isolates owners and rejects late callbacks', make=>{
    const one=make(name), two=make(name), callback=one.render().onDraft;
    callback({text:'one'}); two.render().onDraft({text:'two'});
    one.commit(); two.commit(); assert.equal(api.draftCount(),2);
    one.replay(); assert.equal(api.draftCount(),2,'StrictMode restores retained owner count');
    one.unmount(); assert.equal(api.draftCount(),1);
    callback({text:'late'}); assert.equal(api.draftCount(),1,'unmounted owner cannot revive registration');
  });
}
test('standalone FileBody protects edit and clears undo before React commit',make=>{
  const owner=make('StandaloneFile'), onText=owner.render().onText;
  onText('typed');assert.equal(api.draftCount(),1,'standalone edit must protect Quit before React commits');
  onText('A');assert.equal(api.draftCount(),0,'undo to accepted disk contents clears protection');
});
test('delegated FileBody never adds a second count',make=>{
  const owner=make('StandaloneFile');owner.render('delegated').onText('typed');assert.equal(api.draftCount(),0);
});
console.log(JSON.stringify({passed:results.filter(r=>r.pass).length,total:results.length,results},null,2));
assert(results.every(r=>r.pass),'synchronous production-owner draft protection');
