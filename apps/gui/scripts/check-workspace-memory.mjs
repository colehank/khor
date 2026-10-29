import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {readFileSync} from 'node:fs';
const output=(await build({entryPoints:['src/views/workspace/workspace-memory.ts'],bundle:true,write:false,format:'esm'})).outputFiles[0].text;
const {WorkspaceMemories,workspaceIdentity}=await import('data:text/javascript;base64,'+Buffer.from(output).toString('base64'));
const results=[];
function test(name,run){try{run();results.push({name,pass:true});console.log('PASS',name);}catch(error){results.push({name,pass:false,error:error.message});console.error('FAIL',name,error.message);}}
const identity=(device,path)=>({device,path});
test('the same path on two devices has separate drafts and selected tabs',()=>{
 const owners=new WorkspaceMemories(),a=owners.retain(identity('machine-a','/project')),b=owners.retain(identity('machine-b','/project'));
 a.ensure('drafts',()=>new Map());b.ensure('drafts',()=>new Map());a.ensure('file',()=>null);b.ensure('file',()=>null);
 a.update('drafts',()=>new Map([['/project/a.txt',{text:'dirty A',by:'A'}]]));a.update('file','/project/a.txt');
 assert.equal(b.draftCount(),0);assert.equal(b.read('file'),null);assert.notEqual(workspaceIdentity(identity('a b','c')),workspaceIdentity(identity('a','b c')));
});
test('removing every view subscription retains exact draft and history owners for reselect',()=>{
 const owners=new WorkspaceMemories(),key=identity('a','/project'),a=owners.retain(key),draft={text:'last edit',by:'draft'};
 a.ensure('drafts',()=>new Map());a.ensure('tabs',()=>[]);a.ensure('file',()=>null);
 let notifications=0;const detach=a.subscribe(()=>notifications++);
 a.update('drafts',new Map([['/project/a.txt',draft]]));a.update('tabs',['/project/a.txt']);a.update('file','/project/a.txt');
 const history={};a.histories.set('/project/a.txt',history);detach();
 const returned=owners.retain(key);assert.equal(returned,a);assert.equal(returned.read('drafts').get('/project/a.txt'),draft);assert.equal(returned.histories.get('/project/a.txt'),history);assert.equal(returned.draftCount(),1);
 a.update('file',null);assert.equal(notifications,3,'a detached view is not notified by later model changes');
});
test('an old view completion still updates its own workspace after another selection',()=>{
 const owners=new WorkspaceMemories(),a=owners.retain(identity('a','/one')),b=owners.retain(identity('a','/two'));
 a.ensure('drafts',()=>new Map([['/one/a',{text:'A'}]]));b.ensure('drafts',()=>new Map([['/two/b',{text:'B'}]]));
 const completed=()=>a.update('drafts',new Map());completed();assert.equal(a.draftCount(),0);assert.equal(b.read('drafts').get('/two/b').text,'B');
});
test('functional updates read the latest owner value and unchanged values do not notify',()=>{
 const owner=new WorkspaceMemories().retain(identity('a','/project'));owner.ensure('tabs',()=>[]);let calls=0;owner.subscribe(()=>calls++);
 owner.update('tabs',tabs=>[...tabs,'a']);owner.update('tabs',tabs=>[...tabs,'b']);const same=owner.read('tabs');owner.update('tabs',same);
 assert.deepEqual(owner.read('tabs'),['a','b']);assert.equal(calls,2);owner.ensure('tabs',()=>['discarded initial value']);assert.equal(owner.read('tabs'),same);
});
test('the actual App workspace back callback clears only selection and display state',()=>{
 const app=readFileSync('src/App.tsx','utf8');
 const pane=app.slice(app.indexOf('<WorkspacePane'));
 const body=pane.match(/onBack=\{\(\) => \{ ([^}]+) \}\}/)?.[1];
 assert(body,'actual workspace back callback exists');
 const calls=[];
 new Function('setOpenWs','setWsTree','setScreen',body)(value=>calls.push(['selection',value]),value=>calls.push(['tree',value]),value=>calls.push(['screen',value]));
 assert.deepEqual(calls,[['selection',null],['tree',false],['screen','list']]);
});
console.log(JSON.stringify({passed:results.filter(r=>r.pass).length,total:results.length,results},null,2));assert(results.every(r=>r.pass),'workspace memory contract');
