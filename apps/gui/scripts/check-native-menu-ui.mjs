// Retained, dormant native adapter/queue/snapshot checks. Browser fields
// now use DOM candidates; check-browser-yield covers their active lifecycle.
// No user session, application settings or production browser is modified.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync,mkdtempSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import {chromium} from 'playwright-core';
import {build} from 'esbuild';
const gui=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.NATIVE_MENU_UI_PORT??23884);
const prefix=`.native-menu-ui-${process.pid}`;
const paths=[];
const put=(name,text)=>{const p=join(gui,name);paths.push(p);writeFileSync(p,text);return p;};
const artifact=mkdtempSync(join(process.env.KHOR_GATE_TMP??tmpdir(),'khor-native-menu-ui-'));
let vite,browser;
try {
 // Exercise the production IPC queue independently of React scheduling.
 const built=await build({stdin:{contents:"export {nativeMenuQueue} from './src/lib/native-menu-queue';",resolveDir:gui,loader:'ts'},bundle:true,format:'esm',write:false});
 const {nativeMenuQueue}=await import('data:text/javascript;base64,'+Buffer.from(built.outputFiles[0].text).toString('base64'));
 const mutations=[],release=[];
 const queue=nativeMenuQueue({show:request=>{mutations.push('show:'+request.id);return new Promise(done=>release.push(done));},hide:async id=>mutations.push('hide:'+id)});
 const a=queue.show({id:'a'});await Promise.resolve();
 const b=queue.show({id:'b'}),c=queue.show({id:'c'}),hide=queue.hide('a');
 assert.deepEqual(mutations,['show:a'],'new native mutations wait for pending old show');
 release.shift()(true);await a;await b;
 for(let i=0;i<10&&release.length===0;i++)await Promise.resolve();
 assert.deepEqual(mutations,['show:a','show:c'],'superseded queued show must not reach native');
 const d=queue.show({id:'d'}),cancel=queue.hide('d');release.shift()(true);await Promise.all([c,hide,d,cancel]);
 assert.deepEqual(mutations,['show:a','show:c','hide:a','hide:d'],'closing a pending lease cancels its show without blocking later operations');
 const api=put(`scripts/${prefix}-api.ts`, `
export const inApp=true;
const bus=new Map();export function hear(name,fn){const set=bus.get(name)||new Set();set.add(fn);bus.set(name,set);return()=>set.delete(fn);}
export const calls=[];let pending=[];let hold=false;
export function showMenu(request){calls.push({cmd:'show',request});return hold?new Promise(done=>pending.push(done)):Promise.resolve(true);}
export const hideMenu=id=>{calls.push({cmd:'hide',id});return Promise.resolve(null)};
export const sizeMenu=(id,h)=>{calls.push({cmd:'size',id,h});return Promise.resolve(null)},menuPick=()=>Promise.resolve(null),menuRows=()=>Promise.resolve(null);
window.nativeFixture={calls,emit:(name,payload)=>{for(const fn of bus.get(name)||[])fn(payload)},hold:value=>hold=value,settle:()=>{const old=pending;pending=[];old.forEach(done=>done(true));}};`);
 put(`scripts/${prefix}.tsx`, `import {useRef,useState} from 'react';import {createRoot} from 'react-dom/client';
import {OmniMenu,MenuFace,useFloatedMenu} from '../src/components/OmniMenu';
import {nativeMenuSnapshot} from '../src/lib/native-menu-snapshot';
import '../src/styles/tokens.css';import '../src/styles/app.css';
const face={canvas:80,background:'#e03030',radius_ratio:.5,mark:{ink:'dark',veil:0},paint:{variant:'bauhaus',shapes:[]}};
const rows={groups:[{key:'machines',label:'机器',candidates:Array.from({length:30},(_,i)=>({key:i===0?'m:local':i===1?'m:remote':'row:'+i,label:'机器 '+i+' <script>alert(1)</script>',face}))}],at:0};
const picks=[];
function Fixture(){const anchor=useRef(null),[value,setValue]=useState(null);const floated=useFloatedMenu(anchor,value,key=>picks.push(key),true,{above:true,onDismiss:()=>setValue(null)});
window.fixture={open:()=>setValue(rows),update:()=>setValue({...rows,at:2}),close:()=>setValue(null),picks,snapshot:()=>nativeMenuSnapshot(document.querySelector('[data-test-menu] [data-omni-menu]'))};
return <><input ref={anchor} style={{position:'fixed',left:30,top:380,width:300}}/><div data-floated={floated}/><div data-test-menu style={{width:300}}><OmniMenu rows={rows} onPick={()=>{}}/></div><div data-test-face style={{width:300}}><MenuFace/></div></>}
createRoot(document.getElementById('root')).render(<Fixture/>);`);
 const config=put(`scripts/${prefix}-config.ts`, `import base from '../vite.config';import path from 'node:path';export default {...base,resolve:{alias:[{find:'@/api',replacement:${JSON.stringify(api)}},{find:'@',replacement:path.resolve(import.meta.dirname,'../src')}]}};`);
 put(`${prefix}.html`,`<!doctype html><html data-theme="dark"><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/${prefix}.tsx"></script></body></html>`);
 vite=spawn('node_modules/.bin/vite',['--config',config,'--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false,log='';vite.stdout.on('data',b=>ready||=String(b).includes(`:${port}`));vite.stderr.on('data',b=>log+=b);
 for(let i=0;i<100&&!ready;i++)await new Promise(r=>setTimeout(r,100));assert(ready,log||'Vite did not start');
 browser=await chromium.launch({channel:'chrome',headless:process.env.NATIVE_MENU_UI_HEADED!=='1'});
 const url=`http://localhost:${port}/${prefix}.html`;
 const page=await browser.newPage({viewport:{width:1100,height:800}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));await page.goto(url);await page.waitForFunction(()=>window.fixture);
 // Compatibility callers must always draw their DOM fallback and must
 // not acquire a native menu, even in an App or on a phone.
 await page.evaluate(()=>fixture.open());await page.waitForFunction(()=>document.querySelector('[data-floated]')?.dataset.floated==='false');
 await page.evaluate(()=>fixture.update());await page.evaluate(()=>fixture.close());
 assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='show'||c.cmd==='hide').length),0);
 // The dormant host still accepts its protocol and measures the actual
 // shared rows; retaining it does not make it a browser presentation path.
 await page.evaluate(()=>nativeFixture.emit('menu:rows',{id:'measure-long',rows:{groups:[{key:'g',label:'',candidates:Array.from({length:30},(_,i)=>({key:String(i),label:'row '+i}))}],at:0}}));
 await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.cmd==='size'&&c.id==='measure-long'));
 const long=await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='size'&&c.id==='measure-long').at(-1).h);
 await page.evaluate(()=>nativeFixture.emit('menu:rows',{id:'measure-short',rows:{groups:[{key:'g',label:'',candidates:[{key:'one',label:'one'}]}],at:0}}));
 await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.cmd==='size'&&c.id==='measure-short'));
 const short=await page.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='size'&&c.id==='measure-short').at(-1).h);
 assert(short<long&&long<=240,'native MenuFace must shrink with actual rows, independently of document viewport height');
 assert.deepEqual(errors,[]);
 const mobile=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true,userAgent:'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15'});
 mobile.on('pageerror',e=>errors.push(e.message));await mobile.goto(url);await mobile.waitForFunction(()=>window.fixture);
 await mobile.evaluate(()=>fixture.open());assert.equal(await mobile.locator('[data-floated]').getAttribute('data-floated'),'false');
 assert.equal(await mobile.evaluate(()=>nativeFixture.calls.filter(c=>c.cmd==='show'||c.cmd==='hide').length),0);
 const request={html:await mobile.evaluate(()=>fixture.snapshot())};
 const rimProperties=['background-image','background-color','mask-image','mask-clip','mask-origin','mask-composite','-webkit-mask-composite','padding-top','border-radius','pointer-events'];
 const sourceRims=await mobile.locator('[data-test-menu] [data-omni-menu]').evaluate((menu,properties)=>[menu,menu.querySelector('[data-on="true"]')].map(el=>Object.fromEntries(properties.map(key=>[key,getComputedStyle(el,'::before').getPropertyValue(key)]))),rimProperties);
 writeFileSync(join(artifact,'menu.html'),request.html);
 if(process.env.NATIVE_MENU_SNAPSHOT_OUT)writeFileSync(process.env.NATIVE_MENU_SNAPSHOT_OUT,request.html);
 assert(request.html.length<2_000_000,'snapshot must fit native payload budget');
 const snapshot=await browser.newPage({viewport:{width:300,height:400}});
 const outgoing=[];snapshot.on('request',r=>outgoing.push(r.url()));await snapshot.setContent(request.html);
 assert.equal(await snapshot.locator('script,iframe,object,embed,link').count(),0);
 assert.equal(await snapshot.locator('[data-omni-item]').count(),30);
 assert.equal(await snapshot.locator('[data-omni-item]').first().textContent(),'机器 0 <script>alert(1)</script>');
 const metrics=await snapshot.locator('[data-menu-viewport]').evaluate(el=>({height:el.clientHeight,scroll:el.scrollHeight,overflow:getComputedStyle(el).overflowY}));
 assert(metrics.height<=240&&metrics.scroll>metrics.height);assert.equal(metrics.overflow,'auto');
 assert(await snapshot.locator('svg').count()>0,'actual shared SVG avatars must survive');
 const snapshotRims=await snapshot.locator('[data-omni-menu]').evaluate((menu,properties)=>[menu,menu.querySelector('[data-on="true"]')].map(el=>{const rim=el.querySelector(':scope > [data-native-menu-pseudo="before"]');return rim?Object.fromEntries(properties.map(key=>[key,getComputedStyle(rim).getPropertyValue(key)])):null;}),rimProperties);
 assert.deepEqual(snapshotRims,sourceRims,'menu and selected-row glass rims preserve the source pseudo-element styles');
 assert.equal(await snapshot.locator('[data-native-menu-pseudo][aria-hidden="true"]').count(),2,'decorative spans stay inert and outside accessibility content');
 assert.deepEqual(outgoing,[],'private document must not load app or network resources');
 await snapshot.screenshot({path:join(artifact,'snapshot.png')});
 // Sanitization is checked against actual DOM, including styles with URLs.
 const hostile=await mobile.evaluate(()=>{const menu=document.querySelector('[data-test-menu] [data-omni-menu]');const el=document.createElement('img');el.setAttribute('onerror','alert(1)');el.setAttribute('src','https://invalid.example/secret');menu.append(el);return fixture.snapshot();});
 await snapshot.setContent(hostile);assert.equal(await snapshot.locator('[onerror],[src],script').count(),0);assert.deepEqual(outgoing,[]);
 assert.deepEqual(errors,[]);
 console.log('PASS: active callers use DOM; retained native queue and host sizing, shared mobile SVG/style snapshot, bounded scrolling and inert content.',artifact);
}finally {await browser?.close();vite?.kill('SIGTERM');for(const path of paths)rmSync(path,{force:true});}
