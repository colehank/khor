// Production FaceMark + TabStrip: SVG viewport geometry in Chrome, painted
// concentricity in native WebKit. Fractional layout rectangles alone passed
// the old implementation; the native screenshot must compare the actual ink.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.AVATAR_RING_PORT??23868);
const scratch=mkdtempSync(join(process.env.KHOR_GATE_TMP??tmpdir(),'khor-avatar-ring-'));
const entry=join(gui,'scripts',`.avatar-ring-${process.pid}.tsx`);
const pause=ms=>new Promise(r=>setTimeout(r,ms));
let vite,browser;
try{
 writeFileSync(entry,`import {createRoot} from 'react-dom/client';
import {TabStrip} from '../src/browse/TabStrip';
import {SessionFace} from '../src/components/SessionFace';
import {SettingsProvider} from '../src/settings';
import {TooltipProvider} from '../src/components/ui/tooltip';
import '../src/styles/tokens.css';import '../src/styles/app.css';import '../src/styles/session-content.css';
const face={canvas:80,background:'#e03030',radius_ratio:.5,mark:{ink:'dark',veil:0},paint:{variant:'bauhaus',shapes:[]}};
const states=['busy','blocked','done','errored','failed','idle'];
const faces=states.map(word=>({id:word,label:word,path:'',face,session:{word,kind:'agent',category:'codex'}}));
function Fixture(){return <SettingsProvider value={[]}><TooltipProvider><main style={{padding:40}}>{[18,22].flatMap(size=>[0,.25,.5,.75].map(shift=>({size,shift}))).map(({size,shift},i)=><section key={i} data-ring-case style={{marginLeft:shift,marginTop:24,'--tab-face':size+'px'}}><TabStrip grouped faces={faces} active="busy" onFocus={()=>{}} onClose={()=>{}}/><div style={{display:'flex',gap:48,marginTop:32}}>{states.map(word=><SessionFace key={word} face={face} word={word} kind="agent" category="codex"/>)}</div></section>)}<div style={{display:'flex',gap:48,marginTop:32}}>{states.map(word=><SessionFace key={word} face={face} word={word} kind="agent" category="codex" size="dialMe"/>)}</div></main></TooltipProvider></SettingsProvider>}
createRoot(document.getElementById('root')).render(<Fixture/>);`);
 const html=`<!doctype html><html data-theme="dark"><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.avatar-ring-${process.pid}.tsx"></script></body></html>`;
 const htmlPath=join(gui,`.avatar-ring-${process.pid}.html`);writeFileSync(htmlPath,html);
 vite=spawn('node_modules/.bin/vite',['--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false,log='';vite.stdout.on('data',b=>ready||=String(b).includes(`:${port}`));vite.stderr.on('data',b=>log+=b);
 for(let i=0;i<100&&!ready;i++)await pause(100);assert(ready,log||'Vite did not start');
 const url=`http://localhost:${port}/.avatar-ring-${process.pid}.html`;
 browser=await chromium.launch({channel:'chrome',headless:false});
 const page=await browser.newPage({viewport:{width:1200,height:1300},reducedMotion:'reduce'});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(url);await page.locator('[data-halo-ring]').first().waitFor();
 const geometry=await page.locator('[data-side-face]:has([data-halo-ring])').evaluateAll(es=>es.map(el=>{const rect=e=>{const r=e.getBoundingClientRect();return{x:r.x,y:r.y,w:r.width,h:r.height}};const svg=el.querySelector('[data-halo-ring]');return{face:rect(el),viewport:rect(svg),circle:rect(svg.querySelector('circle')),stroke:parseFloat(getComputedStyle(svg).strokeWidth),tab:el.closest('[data-tab]')?rect(el.closest('[data-tab]')):null};}));
 assert.equal(geometry.length,102);
 for(const g of geometry){
  for(const k of ['x','y','w','h'])assert(Math.abs(g.face[k]-g.viewport[k])<.02,'halo and face must share a raster viewport: '+JSON.stringify(g));
  assert(Math.abs(g.circle.x+g.circle.w/2-g.face.x-g.face.w/2)<.02);
  assert(Math.abs(g.circle.y+g.circle.h/2-g.face.y-g.face.h/2)<.02);
  if(g.tab){assert(g.circle.y-g.stroke/2>=g.tab.y-.02,'tab clips ring top');assert(g.circle.y+g.circle.h+g.stroke/2<=g.tab.y+g.tab.h+.02,'tab clips ring bottom');assert.equal(g.stroke,1.5);}else assert.equal(g.stroke,2);
 }
 assert.deepEqual(errors,[]);
 const shot=join(scratch,'chrome.png');await page.screenshot({path:shot});
 if(process.platform==='darwin'){
  const probe=join(scratch,'avatar-ring-probe');
  execFileSync('swiftc',[join(gui,'scripts/avatar-ring-probe.swift'),'-o',probe],{timeout:60000});
  const result=JSON.parse(execFileSync(probe,[url,join(scratch,'webkit.png')],{encoding:'utf8',timeout:45000}));
  assert.equal(result.measurements.length,51,'native gate must paint busy/done/idle rows and tabs for every fractional position and tab size');
  for(const m of result.measurements){
   assert(m.faceMass>10&&m.ringMass>10,'native screenshot measured no visible ink: '+JSON.stringify(m));
   assert(Math.abs(m.dx)<.15&&Math.abs(m.dy)<.15,'native ring ink is off centre from its face: '+JSON.stringify(m));
  }
  console.log('native WebKit:',JSON.stringify({samples:result.measurements.length,maxDx:Math.max(...result.measurements.map(m=>Math.abs(m.dx))),maxDy:Math.max(...result.measurements.map(m=>Math.abs(m.dy))),states:[...new Set(result.measurements.map(m=>m.state))],faceSizes:[...new Set(result.measurements.map(m=>m.size))]}));writeFileSync(join(scratch,'native-measurements.json'),JSON.stringify(result,null,2));
 }else console.log('native WebKit unavailable on this platform; Chrome geometry only');
 console.log('PASS: 102 row/tab/large rings at 18/22/32/72 preserve shared SVG viewport, same centre, all six states; native fractional-position pixel concentricity. Artifacts:',scratch);
 rmSync(htmlPath,{force:true});
}catch(error){console.error(error);process.exitCode=1;}
finally{if(browser)await browser.close();vite?.kill('SIGTERM');rmSync(entry,{force:true});rmSync(join(gui,`.avatar-ring-${process.pid}.html`),{force:true});}
