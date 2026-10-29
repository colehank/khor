import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
const text=readFileSync('src/views/ChatView.tsx','utf8');
const source=ts.createSourceFile('ChatView.tsx',text,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const wanted=new Map(['retryCapture','capture','receiveCapture'].map(name=>[name,null]));
function visit(node){
  if(ts.isVariableDeclaration(node)&&ts.isIdentifier(node.name)&&wanted.has(node.name.text))wanted.set(node.name.text,`const ${node.getText(source)};`);
  ts.forEachChild(node,visit);
}
visit(source);assert([...wanted.values()].every(Boolean));
const js=ts.transpileModule([...wanted.values()].join('\n'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
const timers=new Map(),calls=[],installed=[],errors=[];let serial=0,wakes=0;
const historyView={current:null};
const document={hidden:false};
const window={setTimeout:(run,delay)=>{assert(delay<=5000);timers.set(++serial,run);return serial;},clearTimeout:id=>timers.delete(id)};
const control=Function('window','document','crypto','attachment','installHistory','setCaptureWhy','historyView','scroller','gui',`
let stopped=false,finished=false,captureTimer=null,captureFailures=0,captureTransient=false,captureAwaiting=false,captureToken=null,captureVersion=0,wakeVersion=0;
${js}
return {capture,receiveCapture,wake:()=>{wakeVersion++;},stop:()=>{stopped=true;}};
`)(window,document,{randomUUID:()=>String(++serial).padStart(32,'0')},{capture:async token=>calls.push(token)},(binding,cursor)=>{installed.push([binding,cursor]);historyView.current={wake:()=>wakes++};},why=>errors.push(why),historyView,{current:{clientHeight:100}},{history_read_failed:'read failed'});
const fire=()=>{const [id,run]=timers.entries().next().value;timers.delete(id);run();};
const answer=(token,reason)=>({kind:'captured',token,result:reason?{kind:'unavailable',reason}:{kind:'available',binding:'canonical',cursor:'trustedEOF'}});
control.capture();const first=calls[0];
control.receiveCapture(answer(first,'not_ready'));assert.equal(timers.size,1);
document.hidden=true;fire();assert.equal(calls.length,1);assert.equal(timers.size,1);
document.hidden=false;fire();assert.equal(calls.length,2);const current=calls[1];
control.receiveCapture(answer(first));assert.deepEqual(installed,[]);
control.wake();control.receiveCapture(answer(current));assert.deepEqual(installed,[['canonical','trustedEOF']]);assert.equal(timers.size,0);assert.equal(wakes,1);
control.capture();control.receiveCapture(answer(calls.at(-1),'replay_in_flight'));assert.equal(timers.size,1);
fire();assert.equal(calls.length,4);control.receiveCapture(answer(calls.at(-1),'unavailable'));assert.equal(timers.size,0);
control.capture();assert.equal(calls.length,5,'an unavailable source retains an explicit retry');
control.receiveCapture(answer(calls.at(-1),'not_ready'));control.stop();fire();assert.equal(calls.length,5);assert.equal(timers.size,0);
assert(errors.includes('read failed'));
console.log('PASS production capture closures retry transient readiness once per timer, pause hidden IO, reject old tokens, preserve wake, expose explicit retry, and stop after the owner is retired');
console.log('1/1 journal capture lifecycle check passed (controlled transport/clock; not host timing or browser evidence)');
