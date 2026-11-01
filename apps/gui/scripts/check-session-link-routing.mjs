import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
const legacy=process.argv.includes('--legacy');
const source=legacy?execFileSync('git',['show','e7c335c8:apps/gui/src/App.tsx'],{encoding:'utf8'}):readFileSync('src/App.tsx','utf8');
const body=source.match(/const sessionHome = useMemo<LinkHome \| null>\(\(\) => \{([\s\S]*?)\n  \}, \[[^\]]*\]\);/)?.[1];assert(body,'session context callback exists');
function fixture({root='/project',selected=true}={}){
 const calls=[];const env={selectedRow:selected?{id:'session'}:null,sessionMachine:selected?'agent-machine':'',sessionRoot:selected?root:'',sessionDevice:'device',sessionKnown:()=>true,seeInSession:()=>{},askOpenPath:(...args)=>calls.push(['path',...args]),askOpenUrl:(...args)=>calls.push(['url',...args]),fetchWorkspaces:async()=>[],holding:()=>null,launchOn:(...args)=>{calls.push(['system-host',...args]);return Promise.resolve();},setMorePage:()=>{},setLanding:()=>{},setOpenWs:()=>{},setReveal:()=>{},setScreen:()=>{}};
 const home=Function(...Object.keys(env),body)(...Object.values(env));return {home,calls};
}
let failed=0;
for(const [name,run]of [
 ['ordinary file enters session content at its line',async()=>{const f=fixture();f.home.open('/project/a.ts',19);await new Promise(r=>setImmediate(r));assert.deepEqual(f.calls,[['path','agent-machine','/project/a.ts',false,19]]);} ],
 ['ordinary URL retains its exact address and session machine',async()=>{const f=fixture();f.home.openPage('http://localhost:8080/a?q=1');assert.deepEqual(f.calls,[['url','agent-machine','http://localhost:8080/a?q=1']]);}],
 ['URL can open when the session has no working directory',async()=>{const f=fixture({root:''});assert(f.home);f.home.openPage('https://example.invalid/');assert.deepEqual(f.calls,[['url','agent-machine','https://example.invalid/']]);}],
 ['no selected conversation has no session routing context',async()=>{assert.equal(fixture({selected:false}).home,null);} ],
]){try{await run();console.log('PASS',name);}catch(e){failed++;console.error('FAIL',name,e.message);}}
assert.equal(failed,0,'session content routing regressions');
