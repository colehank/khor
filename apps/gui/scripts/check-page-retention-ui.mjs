// Production React BrowserPane/MachineWeb and ledgers; native handles are a fixture.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {writeFileSync, readFileSync, mkdirSync, rmSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const engines=(process.env.PAGE_RETENTION_ENGINES??'chrome,webkit').split(',').filter(Boolean);
assert(engines.length,'Select at least one browser engine');
assert.equal(engines.length,new Set(engines).size,'Browser engines must not repeat');
for(const engine of engines) assert(['chrome','webkit'].includes(engine),`Unknown engine ${engine}`);
const prefix=`.page-retention-${process.pid}`, paths=[];
const out=process.env.PAGE_RETENTION_UI_OUT||join(tmpdir(),prefix);
const port=Number(process.env.PAGE_RETENTION_UI_PORT||23850);
const files=['src/views/held.ts','src/lib/native-page-open.ts','src/views/BrowserPane.tsx','src/views/MachineWeb.tsx','src/views/PagesKind.tsx','src/views/SessionContent.tsx','scripts/check-page-retention-ui.mjs'];
const fingerprint=()=>Object.fromEntries(files.map(file=>[file,createHash('sha256').update(readFileSync(join(gui,file))).digest('hex')]));
const sources=fingerprint(),results=[];
mkdirSync(out,{recursive:true});
const put=(name,text)=>{const p=join(gui,name);paths.push(p);writeFileSync(p,text);return p;};
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let server,browser;
try {
 const api=put(`scripts/${prefix}-api.ts`, `
 export const inApp=true,calls=[];const pages=new Map(),opens=[],closes=[];let holdOpen=true,holdClose=false;
 export const openPage=(tab,machine,url)=>{calls.push({kind:'open',tab,machine,url});const finish=()=>{pages.set(tab,{url,visible:true});return {session:machine?'shared-session':'',addr:'addr'};};return holdOpen?new Promise(resolve=>opens.push(()=>resolve(finish()))):Promise.resolve(finish());};
 export const closePage=tab=>{calls.push({kind:'close',tab});const finish=()=>{pages.delete(tab);};return holdClose?new Promise(resolve=>closes.push(()=>{finish();resolve();})):Promise.resolve(finish());};
 export const dropWebBorrow=session=>{calls.push({kind:'drop',session});return Promise.resolve();};
 export const hidePage=tab=>{calls.push({kind:'hide',tab});if(pages.has(tab))pages.get(tab).visible=false;return Promise.resolve();};
 export const showPage=tab=>{calls.push({kind:'show',tab});if(pages.has(tab))pages.get(tab).visible=true;return Promise.resolve();};
 export const movePage=()=>Promise.resolve(),pageIcon=()=>Promise.resolve(''),onPageWord=()=>()=>{};
 export const fetchRecents=()=>Promise.resolve([]),pageStep=()=>Promise.resolve(),pageSteps=()=>Promise.resolve(null),hear=()=>()=>{};
 export const menuRows=()=>Promise.resolve(null),menuPick=()=>Promise.resolve(),sizeMenu=()=>Promise.resolve(),showMenu=()=>Promise.resolve(true),hideMenu=()=>Promise.resolve();
 window.nativeFixture={calls,state:tab=>pages.get(tab),holdOpen:value=>holdOpen=value,holdClose:value=>holdClose=value,settleOpen:()=>opens.splice(0).forEach(f=>f()),settleClose:()=>closes.splice(0).forEach(f=>f())};
 `);
 put(`scripts/${prefix}.tsx`, `
 import {StrictMode,useEffect} from 'react';import {createRoot} from 'react-dom/client';import {flushSync} from 'react-dom';
 import {BrowserPane} from '../src/views/BrowserPane';import {MachineWeb} from '../src/views/MachineWeb';import {releasePages,releaseBorrows} from '../src/views/held';import {TooltipProvider} from '../src/components/ui/tooltip';
 import '../src/styles/tokens.css';import '../src/styles/app.css';
 const root=createRoot(document.getElementById('root')),no=()=>{},machine={id:'exit',name:'remote',shown:'Remote',me:false,hop:'direct',paused:false,face:null};
 function Host({tab='test',wanted=true,mounted=true,url='https://one.test'}){
  useEffect(()=>{releasePages(wanted?[tab]:[],'fixture');releaseBorrows(wanted?['exit']:[],'fixture');},[tab,wanted]);
  return mounted&&<div style={{height:600,width:800}}><BrowserPane retentionOwner='fixture' tab={tab} device='exit' machine='remote' here={false} url={url} onOpened={no} onWent={no} onPopped={no} onTitle={no}/></div>;
 }
 window.fixture={render:props=>flushSync(()=>root.render(<Host {...props}/>)),strict:props=>flushSync(()=>root.render(<StrictMode><Host {...props}/></StrictMode>)),machine:()=>flushSync(()=>root.render(<TooltipProvider><MachineWeb machine={machine} devices={[machine]}/></TooltipProvider>)),clear:()=>flushSync(()=>root.render(null))};
 `);
 const config=put(`scripts/${prefix}-config.ts`, `import base from '../vite.config';import path from 'node:path';export default {...base,optimizeDeps:{entries:[${JSON.stringify(`${prefix}.html`)}]},cacheDir:${JSON.stringify(join(out,'vite-cache'))},resolve:{alias:[{find:'@/api',replacement:${JSON.stringify(api)}},{find:'@',replacement:path.resolve(import.meta.dirname,'../src')}]}};`);
 put(`${prefix}.html`, `<!doctype html><html><body><div id="root"></div><script type="module" src="/scripts/${prefix}.tsx"></script></body></html>`);
 server=await createServer({configFile:config,root:gui,optimizeDeps:{entries:[`${prefix}.html`]},server:{host:'127.0.0.1',port,strictPort:true,hmr:false},logLevel:'error'});await server.listen();
 for(const engine of engines) {
  browser=await (engine==='chrome'?chromium:webkit).launch({headless:true,...(engine==='chrome'?{channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined}:{})});
  for(const name of ['late-close','strict-late-close','close-reopen','machine-late-close']) {
   const page=await browser.newPage({viewport:{width:1100,height:800}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(15000);
   try {
    await page.goto(`http://127.0.0.1:${port}/${prefix}.html`,{waitUntil:'domcontentloaded',timeout:60000});await page.waitForFunction(()=>window.fixture&&window.nativeFixture);
    if(name==='machine-late-close') {
     await page.evaluate(()=>fixture.machine());const address=page.locator('[data-page-address]');await address.fill('https://machine.test');await address.press('Enter');
     await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.kind==='open'&&c.tab==='machine-web:exit'));
     await page.evaluate(()=>{fixture.clear();nativeFixture.settleOpen();});
     await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.kind==='close'&&c.tab==='machine-web:exit'));
     assert.equal(await page.evaluate(()=>nativeFixture.state('machine-web:exit')),undefined);
     assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.kind==='drop').length),1);
    } else if(name==='close-reopen') {
     await page.evaluate(()=>{nativeFixture.holdOpen(false);fixture.render({});});await page.waitForFunction(()=>nativeFixture.state('test'));
     await page.evaluate(()=>{nativeFixture.holdClose(true);fixture.render({wanted:false,mounted:false});});await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.kind==='close'));
     await page.evaluate(()=>fixture.render({url:'https://new.test'}));await pause(100);
     assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.kind==='open').length),1,'new owner cannot open before old close completes');
     await page.evaluate(()=>nativeFixture.settleClose());await page.waitForFunction(()=>nativeFixture.state('test')?.url==='https://new.test');
    } else {
     await page.evaluate(strict=>strict?fixture.strict({}):fixture.render({}),name==='strict-late-close');
     await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.kind==='open'));
     assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.kind==='open').length),1,'StrictMode shares the pending open');
     await page.evaluate(strict=>strict?fixture.strict({wanted:false,mounted:false}):fixture.render({wanted:false,mounted:false}),name==='strict-late-close');
     await page.evaluate(()=>nativeFixture.settleOpen());await page.waitForFunction(()=>nativeFixture.calls.some(c=>c.kind==='close'));
     assert.equal(await page.evaluate(()=>nativeFixture.state('test')),undefined);
     assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.kind==='close').length),1);
     assert.equal(await page.evaluate(()=>nativeFixture.calls.filter(c=>c.kind==='drop').length),1);
    }
    assert.deepEqual(errors,[]);results.push({engine,name,ok:true,calls:await page.evaluate(()=>nativeFixture.calls)});console.log('PASS',engine,name);
   }catch(error){results.push({engine,name,ok:false,error:error.stack,errors,calls:await page.evaluate(()=>window.nativeFixture?.calls).catch(()=>[])});console.error('FAIL',engine,name,error.message);await page.screenshot({path:join(out,`${engine}-${name}.png`)}).catch(()=>{});}
   finally{await page.close();}
  }
  await browser.close();browser=null;
 }
 assert.deepEqual(fingerprint(),sources,'Sources stayed fixed during component checks');writeFileSync(join(out,'report.json'),JSON.stringify({sources,results,boundary:'Real React components and ledger; controlled native API, no actual native handles'},null,2));assert(results.every(r=>r.ok),'Page retention component checks failed');console.log('page retention components:',results.length,'passed',out);
}finally{await browser?.close();await server?.close();for(const path of paths)rmSync(path,{force:true});}
