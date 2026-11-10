// Actual directory planner; only listing/mkdir transport is controlled.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({entryPoints:['src/lib/session-directory.ts'],bundle:true,write:false,format:'esm',platform:'node'});
const {inspectSessionDirectory:inspect,ensureSessionDirectory:ensure}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
function fixture(seed){
 const dirs=new Map(Object.entries(seed)),calls=[];
 const io={
  async list(path){calls.push(['list',path]);if(!dirs.has(path))throw new Error('read refused: '+path);return structuredClone(dirs.get(path));},
  async mkdir(path){calls.push(['mkdir',path]);if(dirs.has(path))throw new Error('already exists');const index=Math.max(path.lastIndexOf('/'),path.lastIndexOf('\\')),parent=path.slice(0,index)||'/';const row=dirs.get(parent);if(!row)throw new Error('parent unavailable');row.entries.push({name:path.slice(index+1),dir:true});dirs.set(path,{path,entries:[],truncated:false});},
 };
 return {dirs,calls,io};
}
const directory=(path,entries=[],truncated=false)=>({path,entries:entries.map(([name,dir=true])=>({name,dir})),truncated});
let f=fixture({'/':directory('/',[['srv']]),'/srv':directory('/srv')});
assert.deepEqual(await inspect('/srv/new/nested',f.io),{state:'missing',path:'/srv/new/nested',missing:['/srv/new','/srv/new/nested']});
assert(!f.calls.some(c=>c[0]==='mkdir'),'Typing and inspecting must never create directories');
assert.equal(await ensure('/srv/new/nested',f.io),'/srv/new/nested');
assert.deepEqual(f.calls.filter(c=>c[0]==='mkdir'),[['mkdir','/srv/new'],['mkdir','/srv/new/nested']]);
f.calls.length=0;assert.equal((await inspect('/srv/new/nested/',f.io)).state,'ready');await ensure('/srv/new/nested',f.io);assert(!f.calls.some(c=>c[0]==='mkdir'));
// Complete-list evidence, not a translated error substring, decides missing.
for(const [label,seed] of [
 ['permission',{'/':directory('/',[['srv']]),'/srv':directory('/srv',[['private']])}],
 ['file',{'/':directory('/',[['srv']]),'/srv':directory('/srv',[['private',false]])}],
 ['truncated',{'/':directory('/',[['srv']]),'/srv':directory('/srv',[],true)}],
 ['offline',{}],
]){
 f=fixture(seed);const target='/srv/private/child';const result=await inspect(target,f.io);
 assert.equal(result.state,'unavailable',label+' is not evidence of a missing directory');
 await assert.rejects(ensure(target,f.io));assert(!f.calls.some(c=>c[0]==='mkdir'),label+' must not write');
}
f=fixture({'':directory('/Users/target'),'/Users/target':directory('/Users/target')});
assert.deepEqual((await inspect('~/project/nested',f.io)).missing,['/Users/target/project','/Users/target/project/nested']);
f=fixture({'C:\\':directory('C:\\')});assert.deepEqual((await inspect('C:\\new\\nested',f.io)).missing,['C:\\new','C:\\new\\nested']);
f=fixture({'\\\\server\\share':directory('\\\\server\\share')});assert.equal((await inspect('\\\\server\\share\\new',f.io)).state,'missing');
f=fixture({'/srv':directory('/physical/srv')});assert.equal((await inspect('/srv/new',f.io)).path,'/physical/srv/new');
// A second creator may win between validation and mkdir; only a successful
// directory read permits continuing after that refusal.
f=fixture({'/':directory('/',[['srv']]),'/srv':directory('/srv')});const mkdir=f.io.mkdir;
f.io.mkdir=async path=>{await mkdir(path);throw new Error('concurrent creator');};assert.equal(await ensure('/srv/new',f.io),'/srv/new');
let alive=true;f=fixture({'/srv':directory('/srv')});let release;const held=new Promise(resolve=>release=resolve),list=f.io.list;
f.io.list=async path=>{await held;return list(path);};const cancelled=inspect('/srv/new',f.io,()=>alive);alive=false;release();await assert.rejects(cancelled,{name:'AbortError'});assert(!f.calls.some(c=>c[0]==='mkdir'));
alive=true;f=fixture({'/srv':directory('/srv')});const make=f.io.mkdir;f.io.mkdir=async path=>{await make(path);alive=false;};await assert.rejects(ensure('/srv/new/child',f.io,()=>alive),{name:'AbortError'});assert.deepEqual(f.calls.filter(c=>c[0]==='mkdir'),[['mkdir','/srv/new']]);
f=fixture({'/srv':directory('/srv')});await assert.rejects(ensure('/srv/missing/../other',f.io));assert(!f.calls.some(c=>c[0]==='mkdir'));
console.log('PASS session directory: read-only validation, nested creation, home/Windows/UNC/canonical paths, permission/file/offline/truncated refusals, concurrent mkdir and cancellation');

const labBundle=await build({entryPoints:['dev/lab/navigation.ts'],bundle:true,write:false,format:'esm',platform:'node'});
const {navigationFixture}=await import('data:text/javascript;base64,'+Buffer.from(labBundle.outputFiles[0].text).toString('base64'));
const lab=navigationFixture([{id:'local',name:'local',me:true}],'http://fixture.invalid',false);
const labIO={list:async path=>lab.request('ls',{machine:'local',path}),mkdir:async path=>lab.request('mkdir',{machine:'local',path})};
assert.equal((await inspect('/preview/new-session-directory/nested',labIO)).state,'missing');
assert.equal(await ensure('/preview/new-session-directory/nested',labIO),'/preview/new-session-directory/nested');
assert.equal((await inspect('/preview/new-session-directory/nested',labIO)).state,'ready');
console.log('PASS current complete-App Lab transport supports read-only probe and nested mkdir');
