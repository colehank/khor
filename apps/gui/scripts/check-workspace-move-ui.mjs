// Production FileTree. Only the disk transport is replaced; drag events hit
// the real rows and existing rename_path endpoint. No alternate tree UI.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.WORKSPACE_MOVE_PORT||23896);
const entry=join(gui,`scripts/.workspace-move-${process.pid}.tsx`);
let server,browser,page;
const errors=[],calls=[],reads=[];
const entries=new Map([
 ['/project',[{name:'source',dir:true},{name:'target',dir:true},{name:'conflict.md',dir:false},{name:'notes.md',dir:false}]],
 ['/project/source',[{name:'child',dir:true},{name:'draft.md',dir:false}]],
 ['/project/source/child',[]],['/project/target',[{name:'conflict.md',dir:false}]],
]);
let refuse=false;
try {
 writeFileSync(entry,`import React from 'react';import {createRoot} from 'react-dom/client';
import {FileTree} from '../src/views/FileTree.tsx';import {TooltipProvider} from '../src/components/ui/tooltip.tsx';
import {speakLang} from '../src/lang.ts';import '../src/styles/tokens.css';import '../src/styles/app.css';
speakLang('zh');window.moves=[];
createRoot(document.getElementById('root')).render(<TooltipProvider><FileTree machine='fixture' root='/project' here marks={null} open={null} onOpen={()=>{}} onGone={()=>{}} onMoved={(from,to)=>window.moves.push({from,to})}/></TooltipProvider>);`);
 server=spawn(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 server.stderr.on('data',data=>process.stderr.write(data));
 server.stdout.on('data',data=>process.stdout.write(data));
 let ready=false;
 for(let i=0;i<300&&!ready;i++){
  assert.equal(server.exitCode,null);
  ready=await fetch(`http://localhost:${port}/`,{signal:AbortSignal.timeout(1000)}).then(response=>response.ok).catch(()=>false);
  if(!ready)await new Promise(resolve=>setTimeout(resolve,100));
 }
 assert(ready,'Vite HTTP readiness');
 browser=await chromium.launch({headless:true,args:['--no-proxy-server']});
 page=await browser.newPage({viewport:{width:700,height:700},hasTouch:false});page.on('pageerror',error=>{errors.push(error.message);console.error(error.message)});
 page.on('requestfailed',request=>console.error('fixture request failed',request.url(),request.failure()));
 await page.route(`http://127.0.0.1:${port}/**`,async route=>{
  const request=route.request();
  const headers={'access-control-allow-origin':'*','access-control-allow-headers':'content-type'};
  if(request.method()==='OPTIONS'){await route.fulfill({status:204,headers});return;}
  const command=new URL(request.url()).pathname.slice(1),args=request.postDataJSON()||{};let result=null;
  if(command==='ls'){reads.push(args.path);result={path:args.path,entries:entries.get(args.path)||[],truncated:false};}
  else if(command==='rename_path'){
   calls.push(args);if(refuse){await route.fulfill({status:409,headers,json:'fixture occupied'});return;}
   const parent=path=>path.slice(0,path.lastIndexOf('/'))||'/',name=path=>path.slice(path.lastIndexOf('/')+1);
   const source=entries.get(parent(args.from)).find(entry=>entry.name===name(args.from));assert(source);
   entries.set(parent(args.from),entries.get(parent(args.from)).filter(entry=>entry!==source));
   entries.set(parent(args.to),[...(entries.get(parent(args.to))||[]),{...source,name:name(args.to)}]);
   if(source.dir)for(const [path,value] of [...entries])if(path===args.from||path.startsWith(args.from+'/')){entries.delete(path);entries.set(args.to+path.slice(args.from.length),value);}
  }else if(command==='dirs_wait')result={seq:0,dirs:[],unavailable:true,unreported:[]};
  else if(command==='dirs_begin')result={};
  else if(command==='about')result={hosts:true,faces:[]};
  else if(command==='trash_here')result=false;
  await route.fulfill({headers,json:result});
 });
 await page.route('**/__workspace_move?**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme="dark"><body><div id="root" class="h-screen"></div><script type="module">import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.workspace-move-${process.pid}.tsx"></script></body></html>`}));
 await page.goto(`http://localhost:${port}/__workspace_move?bridge=${port}`);
 const tree=page.locator('[data-file-tree]'),file=name=>tree.locator(`[data-file="${name}"] button[data-file-path]`),dir=name=>tree.locator(`[data-dir="${name}"]`);
 await file('notes.md').waitFor();await dir('target').click();await file('conflict.md').first().waitFor();
 await file('notes.md').dragTo(dir('target'));
 await tree.locator('button[data-file-path="/project/target/notes.md"]').waitFor();
 assert.equal(calls.length,1);assert.deepEqual(calls[0],{machine:'fixture',from:'/project/notes.md',to:'/project/target/notes.md'});
 assert.deepEqual(await page.evaluate(()=>window.moves),[{from:'/project/notes.md',to:'/project/target/notes.md'}]);
 assert(reads.includes('/project')&&reads.includes('/project/target'));
 // Move back to the root blank and make sure no duplicate request fires.
 await file('notes.md').dragTo(tree.locator('[data-tree-blank]'));
 await tree.locator('button[data-file-path="/project/notes.md"]').waitFor();assert.equal(calls.length,2);
 await dir('source').click();await dir('child').waitFor();
 for(const [from,to] of [[dir('source'),dir('source')],[dir('source'),dir('child')],[file('notes.md'),tree.locator('[data-tree-blank]')],[file('notes.md'),file('draft.md')]])await from.dragTo(to);
 assert.equal(calls.length,2,'self, descendants, same parent and file targets must not invoke move');
 await tree.locator('button[data-file-path="/project/conflict.md"]').dragTo(dir('target'));assert.equal(calls.length,2,'known occupied name is refused');
 refuse=true;await file('draft.md').dragTo(dir('target'));await tree.locator('[role=alert]').waitFor();assert.equal(calls.length,3);assert.equal((await page.evaluate(()=>window.moves)).length,2);await file('draft.md').waitFor();
 refuse=false;await dir('source').dragTo(dir('target'));await tree.locator('button[data-file-path="/project/target/source"]').waitFor();
 await tree.locator('button[data-file-path="/project/target/source/draft.md"]').waitFor();
 assert.equal(calls.length,4);assert.equal((await page.evaluate(()=>window.moves)).length,3);
 assert.deepEqual(errors,[]);
 console.log('PASS production FileTree DOM drag: files, directories, root blank, target refresh, subtree expansion, onMoved, invalid targets, occupied target and backend refusal');
} catch(error) {
 console.error({errors,reads,calls,body:await page?.locator('body').innerText().catch(()=>null)});
 throw error;
} finally {
 await browser?.close();server?.kill('SIGTERM');rmSync(entry,{force:true});
}
