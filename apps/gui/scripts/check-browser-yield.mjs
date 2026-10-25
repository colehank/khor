// Production BrowserPane, PageField, OmniMenu and Tooltip. Only native IPC is
// replaced: this checks lifecycle/ownership decisions, not OS compositor timing.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync,mkdtempSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),port=Number(process.env.BROWSER_YIELD_PORT||23887);
const prefix=`.browser-yield-${process.pid}`,paths=[];
const artifact=mkdtempSync(join(process.env.KHOR_GATE_TMP||tmpdir(),'khor-browser-yield-'));
const put=(name,text)=>{const p=join(gui,name);paths.push(p);writeFileSync(p,text);return p;};
const pause=ms=>new Promise(r=>setTimeout(r,ms));
let vite,browser;
try {
 const api=put(`scripts/${prefix}-api.ts`, `
export const inApp=true,calls=[];const pages=new Map(),destinations=new Map(),opening=new Set(),pending=[];let hold=false,moveHold=false;const moving=[];
export const openPage=(tab,machine,url,at)=>{calls.push({cmd:'open',tab,machine,url,at});if(opening.has(tab)){calls.push({cmd:'label-conflict',tab});return Promise.reject(Error('webview label already exists'));}opening.add(tab);const run=()=>{opening.delete(tab);destinations.set(tab,{machine,url});if(!pages.has(tab))pages.set(tab,{visible:true,scroll:417,form:'kept'});else pages.get(tab).visible=true;return {addr:'',session:''};};return hold?new Promise(done=>pending.push(()=>done(run()))):Promise.resolve(run());};
export const movePage=(tab,at)=>{calls.push({cmd:'move',tab,at});return moveHold?new Promise(done=>moving.push(done)):Promise.resolve();};
export const hidePage=tab=>{calls.push({cmd:'hide',tab});if(pages.has(tab))pages.get(tab).visible=false;return Promise.resolve();};
export const showPage=tab=>{calls.push({cmd:'show',tab});if(pages.has(tab))pages.get(tab).visible=true;return Promise.resolve();};
export const closePage=tab=>{calls.push({cmd:'close',tab});pages.delete(tab);return Promise.resolve();};
export const pageIcon=()=>Promise.resolve(''),dropWebBorrow=()=>Promise.resolve(),onPageWord=()=>()=>{};
export const fetchRecents=()=>Promise.resolve([]),pageStep=()=>Promise.resolve(),pageSteps=()=>Promise.resolve(null);
export const hear=()=>()=>{},menuRows=()=>Promise.resolve(null),menuPick=()=>Promise.resolve(),sizeMenu=()=>Promise.resolve();
export const showMenu=request=>{calls.push({cmd:'show-menu',request});return Promise.resolve(true);};
export const hideMenu=id=>{calls.push({cmd:'hide-menu',id});return Promise.resolve();};
window.nativeFixture={calls,state:tab=>pages.get(tab),target:tab=>destinations.get(tab),hold:value=>hold=value,settle:()=>pending.splice(0).forEach(run=>run()),holdMoves:value=>moveHold=value,settleMoves:()=>moving.splice(0).forEach(done=>done())};`);
 put(`scripts/${prefix}.tsx`, `import {StrictMode,useRef,useState} from 'react';import {createRoot} from 'react-dom/client';import {flushSync} from 'react-dom';
import {BrowserPane} from '../src/views/BrowserPane';import {PageField} from '../src/browse/PageField';import {OmniMenu,useFloatedMenu} from '../src/components/OmniMenu';import {TooltipProvider,Tooltip,TooltipTrigger,TooltipContent} from '../src/components/ui/tooltip';
import '../src/styles/tokens.css';import '../src/styles/app.css';
const root=createRoot(document.getElementById('root')),no=()=>{};
const machine={id:'local',name:'local',shown:'Local',me:true,hop:'direct',paused:false,face:null};
function Fixture({tab='page',suspended=false,hidden=false,owner='a',width=800,url='https://example.test',exit='local',here=true}){const anchor=useRef(null),[rows,setRows]=useState(null);const floated=useFloatedMenu(anchor,rows,no,true,{onDismiss:()=>setRows(null)});window.showSharedMenu=()=>setRows({groups:[{key:'g',label:'',candidates:[{key:'one',label:'One'}]}],at:0});return <TooltipProvider delayDuration={0}><div><PageField tab={tab} machine={machine} devices={[machine]} at='https://example.test' steps={false} suggestAddresses={false} onGo={no}/><button data-outside>Outside</button><button ref={anchor} data-shared-anchor>Shared menu</button>{rows&&!floated&&<OmniMenu rows={rows} onPick={no}/>}</div><div style={{position:'absolute',left:0,top:100,width,height:600}} aria-hidden={hidden||undefined} inert={hidden||undefined}><BrowserPane retentionOwner="fixture" key={owner} tab={tab} machine={exit} device={exit} url={url} here={here} suspended={suspended} onOpened={no} onWent={no} onPopped={no} onTitle={no}/></div><Tooltip><TooltipTrigger asChild><button data-tooltip-trigger style={{position:'absolute',left:300,top:70}}>Hover</button></TooltipTrigger><TooltipContent side='bottom'>Safe tooltip</TooltipContent></Tooltip></TooltipProvider>;}
window.fixture={render:props=>flushSync(()=>root.render(<Fixture {...props}/>)),strict:props=>flushSync(()=>root.render(<StrictMode><Fixture {...props}/></StrictMode>)),clear:()=>flushSync(()=>root.render(null))};window.fixture.render({});`);
 const config=put(`scripts/${prefix}-config.ts`, `import base from '../vite.config';import path from 'node:path';export default {...base,cacheDir:${JSON.stringify(join(artifact,'vite-cache'))},resolve:{alias:[{find:'@/api',replacement:${JSON.stringify(api)}},{find:'@',replacement:path.resolve(import.meta.dirname,'../src')}]}};`);
 put(`${prefix}.html`, `<!doctype html><html data-theme="dark"><body><div id="root"></div><script type="module">import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/${prefix}.tsx"></script></body></html>`);
 vite=spawn('node_modules/.bin/vite',['--config',config,'--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false,log='';vite.stdout.on('data',d=>ready||=String(d).includes(`:${port}`));vite.stderr.on('data',d=>log+=d);
 for(let i=0;i<100&&!ready;i++)await pause(100);assert(ready,log||'Vite did not start');
 browser=await chromium.launch({channel:'chrome',headless:process.env.BROWSER_YIELD_HEADED!=='1'});const page=await browser.newPage({viewport:{width:1100,height:800}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`http://localhost:${port}/${prefix}.html`);await page.waitForFunction(()=>window.nativeFixture?.state('page')?.visible);
 const opens=()=>page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='open').length);
 const initial=await opens();await page.locator('[data-page-address]').focus();await pause(100);
 assert.equal(await page.evaluate(()=>nativeFixture.state('page').visible),false,'address editing without suggestions must hide the retained native page');
 assert.equal(await opens(),initial,'editing reopened the page');
 await page.locator('[data-outside]').click();await page.waitForFunction(()=>nativeFixture.state('page').visible);
 assert.deepEqual(await page.evaluate(()=>nativeFixture.state('page')),{visible:true,scroll:417,form:'kept'});
 await page.evaluate(()=>fixture.render({suspended:true}));await page.waitForFunction(()=>!nativeFixture.state('page').visible);await page.evaluate(()=>fixture.render({suspended:false}));await page.waitForFunction(()=>nativeFixture.state('page').visible);assert.equal(await opens(),initial,'suspension remounted or reopened BrowserPane');
 for(const attrs of [{'data-omni-menu':''},{'data-deck-selector':''},{role:'dialog'},{role:'listbox'},{role:'alertdialog'}]) {
  const before=await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='move').length);
  await page.evaluate(attrs=>{const el=document.createElement('div');Object.entries(attrs).forEach(([k,v])=>el.setAttribute(k,v));el.dataset.testOverlay='';Object.assign(el.style,{position:'fixed',left:'10px',top:'120px',width:'200px',height:'100px'});document.body.append(el);},attrs);
  await page.waitForFunction(()=>!nativeFixture.state('page').visible);assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='move').length),before,'overlay cropped/moved the native page');
  await page.evaluate(()=>document.querySelector('[data-test-overlay]').remove());await page.waitForFunction(()=>nativeFixture.state('page').visible);
 }
 // A modal question can miss the page while its scrim still covers it.
 await page.evaluate(()=>{const el=document.createElement('div');el.dataset.testOverlay='';el.setAttribute('role','alertdialog');el.setAttribute('aria-modal','true');Object.assign(el.style,{position:'fixed',left:'900px',top:'20px',width:'100px',height:'30px'});document.body.append(el);});
 await page.waitForFunction(()=>!nativeFixture.state('page').visible);await page.evaluate(()=>document.querySelector('[data-test-overlay]').remove());await page.waitForFunction(()=>nativeFixture.state('page').visible);
 // Resize while restore is awaiting a native move. The latest placement
 // still owes a show, and skipped geometry must not be marked as applied.
 await page.evaluate(()=>fixture.render({suspended:true}));await page.waitForFunction(()=>!nativeFixture.state('page').visible);
 await page.evaluate(()=>{nativeFixture.holdMoves(true);fixture.render({width:790});});await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.cmd==='move'&&c.at.w===790));
 await page.evaluate(()=>fixture.render({width:780}));await page.evaluate(()=>{nativeFixture.holdMoves(false);nativeFixture.settleMoves();});await page.waitForFunction(()=>nativeFixture.state('page').visible);
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='move').at(-1).at.w),780,'restoration lost latest full host geometry');
 await page.evaluate(()=>fixture.render({}));
 await page.locator('[data-tooltip-trigger]').hover();await pause(200);
 assert.equal(await page.evaluate(()=>nativeFixture.state('page').visible),true,'tooltip hid the page');
 assert.equal(await page.locator('[data-slot="tooltip-content"]:visible').count(),0,'tooltip still draws under the native page');
 await page.mouse.move(1000,750);await page.evaluate(()=>showSharedMenu());await page.locator('[data-omni-menu]').waitFor();assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>/menu/.test(c.cmd)).length),0,'shared candidate hook still invokes native menu');
 await page.evaluate(()=>fixture.clear());await page.waitForFunction(()=>!nativeFixture.state('page').visible);
 // A late opening from an abandoned owner cannot restore itself over a
 // replacement which is suspended. Release/recompute belongs to the owner.
 await page.evaluate(()=>{nativeFixture.hold(true);fixture.render({tab:'late',owner:'old'});});await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.cmd==='open'&&c.tab==='late'));
 await page.evaluate(()=>{fixture.render({tab:'late',owner:'new',suspended:true});nativeFixture.settle();});await pause(100);
 assert.equal(await page.evaluate(()=>nativeFixture.state('late').visible),false,'late old open restored a suspended replacement');
 await page.evaluate(()=>{nativeFixture.hold(false);fixture.render({tab:'late',owner:'new',suspended:false});});await page.waitForFunction(()=>nativeFixture.state('late').visible);
 await page.evaluate(()=>fixture.render({tab:'late',owner:'new',hidden:true}));await page.waitForFunction(()=>!nativeFixture.state('late').visible);await page.evaluate(()=>fixture.render({tab:'late',owner:'new'}));await page.waitForFunction(()=>nativeFixture.state('late').visible);
 // StrictMode abandons and remounts an effect while cold open_page is
 // still constructing a native view. Identical pending work shares one
 // native call; the latest owner still finishes its own UI lifecycle.
 await page.evaluate(()=>{fixture.clear();nativeFixture.hold(true);fixture.strict({tab:'strict'});});await pause(100);
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='open'&&c.tab==='strict').length),1,'StrictMode must share an identical pending native page opening');
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='label-conflict').length),0,'cold double mount tried to construct the same native webview twice');
 await page.evaluate(()=>nativeFixture.settle());await page.waitForFunction(()=>nativeFixture.state('strict')?.visible);
 assert.equal(await page.locator('[data-web-status="failed"]').count(),0);
 // Once that work has completed, returning still asks open_page to show
 // the existing native history. Pending sharing is never a settled cache.
 await page.evaluate(()=>fixture.clear());await page.evaluate(()=>fixture.strict({tab:'strict'}));await pause(100);
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='open'&&c.tab==='strict').length),2,'a completed opening must not suppress the normal native resume path');
 await page.evaluate(()=>nativeFixture.settle());await page.waitForFunction(()=>nativeFixture.state('strict')?.visible);
 assert.deepEqual(await page.evaluate(()=>nativeFixture.state('strict')),{visible:true,scroll:417,form:'kept'});
 // Different destinations for the same label are serialized, including
 // A -> B -> A and changing only the exit. Consecutive equal queued work
 // may share; an old in-flight A cannot skip the A requested after B.
 await page.evaluate(()=>{fixture.clear();fixture.render({tab:'serial',owner:'one',url:'https://a.test',exit:'one',here:false});});await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.cmd==='open'&&c.tab==='serial'));
 await page.evaluate(()=>{fixture.render({tab:'serial',owner:'two',url:'https://b.test',exit:'one',here:false});fixture.render({tab:'serial',owner:'three',url:'https://a.test',exit:'one',here:false});fixture.render({tab:'serial',owner:'four',url:'https://a.test',exit:'two',here:false});fixture.render({tab:'serial',owner:'five',url:'https://a.test',exit:'two',here:false});});await pause(100);
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='open'&&c.tab==='serial').length),1,'different same-label destinations must wait for the native construction in flight');
 for(const count of [2,3,4]){await page.evaluate(()=>nativeFixture.settle());await page.waitForFunction(count=>nativeFixture.calls.filter(c=>c.cmd==='open'&&c.tab==='serial').length===count,count);}
 await page.evaluate(()=>{nativeFixture.hold(false);nativeFixture.settle();});await page.waitForFunction(()=>nativeFixture.state('serial')?.visible);
 assert.deepEqual(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='open'&&c.tab==='serial').map(c=>[c.machine,c.url])),[['one','https://a.test'],['one','https://b.test'],['one','https://a.test'],['two','https://a.test']]);
 assert.deepEqual(await page.evaluate(()=>nativeFixture.target('serial')),{machine:'two',url:'https://a.test'});
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='label-conflict').length),0);
 assert.deepEqual(errors,[]);writeFileSync(join(artifact,'calls.json'),JSON.stringify(await page.evaluate(()=>nativeFixture.calls),null,2));await page.screenshot({path:join(artifact,'browser-policy.png')});console.log('PASS browser retained hide/restore, no native candidate overlay, full geometry, tooltip suppression, late owner and visibility guards',artifact);
} finally {await browser?.close();vite?.kill('SIGTERM');for(const path of paths)rmSync(path,{force:true});}
