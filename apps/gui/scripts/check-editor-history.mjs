// Actual FilePage/CodeEditor lifecycle, with only the disk transport mocked.
// No running khor node, sessions or user files are touched.
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,rmSync,mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium,webkit} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const fixture=mkdtempSync(join(gui,'.editor-history-check-'));
const evidence=process.env.EDITOR_HISTORY_CHECK_DIR||mkdtempSync(join(tmpdir(),'khor-editor-history-'));
mkdirSync(evidence,{recursive:true});
const wait=ms=>new Promise(r=>setTimeout(r,ms));
const BASE='# Notes\n\nOriginal text.\n';
let server,browser;
const failures=[];
writeFileSync(join(fixture,'index.html'),'<html><body style="margin:0"><div id="root"></div><script type="module" src="./probe.tsx"></script></body></html>');
writeFileSync(join(fixture,'probe.tsx'),`
import React,{useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {FilePage} from '/src/views/FilePage';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {undoDepth,redoDepth} from '@codemirror/commands';
import '/src/styles/tokens.css';
import '/src/styles/app.css';
document.documentElement.dataset.theme='light';
function Probe(){
 const [path,setPath]=useState('/probe/a.md'),[machine,setMachine]=useState('history-test'),[shown,setShown]=useState(true),[generation,setGeneration]=useState(0);
 const [drafts,setDrafts]=useState({});
 const slot=useRef({});
 window.__historyProbe={
  hide:()=>setShown(false),show:()=>setShown(true),path:setPath,machine:setMachine,
  fresh:()=>{slot.current={};setGeneration(n=>n+1);},
  read:()=>{const v=document.querySelector('[data-code-view]')?.__view;return v?{text:v.state.doc.toString(),undo:undoDepth(v.state),redo:redoDepth(v.state),selection:v.state.selection.main.head}:null;},
  end:()=>{const v=document.querySelector('[data-code-view]')?.__view;if(v){v.focus();v.dispatch({selection:{anchor:v.state.doc.length}});}}
 };
 return <TooltipProvider><div style={{height:'100vh',display:'flex',flexDirection:'column'}}>{shown&&<FilePage key={path+generation} machine={machine} path={path} here={false} editorHistory={slot.current} draft={drafts[path]} onDraft={d=>setDrafts(w=>w[path]===d?w:{...w,[path]:d||undefined})} onBack={()=>{}} onGone={()=>{}} onMoved={()=>{}}/>}</div></TooltipProvider>;
}
createRoot(document.getElementById('root')).render(<Probe/>);
`);

async function check(engine,name){
 browser=await engine.launch(name==='chrome'?{channel:'chrome',headless:process.env.EDITOR_HISTORY_HEADLESS==='1'}:{headless:false});
 for(const scenario of ['ordinary','preview','remount','redo-remount','fresh-slot','other-path','other-machine','external-text']){
  const page=await browser.newPage({viewport:{width:1000,height:760}});page.setDefaultTimeout(7000);
  const disk=new Map([['/probe/a.md',BASE],['/probe/b.md',BASE]]),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  const digest=text=>Buffer.from(text).toString('hex').padEnd(64,'0').slice(0,64);
  await page.route('**/api/*',async route=>{
   const cmd=new URL(route.request().url()).pathname.split('/').pop(),args=route.request().postDataJSON()||{},text=disk.get(args.path)||BASE;
   let result=null;
   if(cmd==='read_text')result={text,digest:digest(text),total:Buffer.byteLength(text),at_ms:1,cut:false,binary:false};
   else if(cmd==='stat_text')result={digest:digest(text),total:Buffer.byteLength(text),at_ms:1};
   else if(cmd==='write_text'){disk.set(args.path,args.text);result={digest:digest(args.text),at_ms:2};}
   else if(cmd==='git_diff')result={hunks:[],more:false};
   else if(cmd==='git_log')result={commits:[],more:false};
   await route.fulfill({json:result});
  });
  const read=()=>page.evaluate(()=>window.__historyProbe.read());
  const mounted=()=>page.locator('.cm-content[contenteditable=true]').waitFor();
  const remount=async()=>{await page.evaluate(()=>window.__historyProbe.hide());await page.locator('.cm-content').waitFor({state:'detached'});await wait(150);await page.evaluate(()=>window.__historyProbe.show());await mounted();};
  try{
   await page.goto(`http://localhost:${server.config.server.port}/${fixture.split('/').pop()}/index.html`);
   await mounted();if(scenario==='ordinary')await wait(2500);await page.evaluate(()=>window.__historyProbe.end());await page.keyboard.type('added');await wait(1300);
   assert.equal((await read()).text,BASE+'added');assert((await read()).undo>0,'typing creates history');
   if(scenario==='preview'){
    await page.locator('[data-file-face-toggle]').click();await page.locator('[data-file-preview=md]').waitFor();assert.equal(await page.locator('.cm-content').count(),0,'preview unmounts editor');await page.locator('[data-file-more-menu]').waitFor({state:'detached'});await wait(200);
    await page.locator('[data-file-face-toggle]').click();await mounted();
   }else if(scenario==='remount')await remount();
   else if(scenario==='redo-remount'){
    await page.keyboard.press('Meta+z');assert.equal((await read()).text,BASE);await wait(1300);await remount();
    await page.evaluate(()=>window.__historyProbe.end());await page.keyboard.press('Meta+Shift+z');await wait(80);assert.equal((await read()).text,BASE+'added','redo survives remount');
   }else if(scenario==='fresh-slot'){
    await page.evaluate(()=>window.__historyProbe.fresh());await mounted();assert.equal((await read()).undo,0,'closed tab/new slot has no old history');
   }else if(scenario==='other-path'){
    await page.evaluate(()=>window.__historyProbe.path('/probe/b.md'));await mounted();assert.equal((await read()).text,BASE);assert.equal((await read()).undo,0,'another path cannot inherit history');
   }else if(scenario==='other-machine'){
    await page.evaluate(()=>window.__historyProbe.machine('second-machine'));await mounted();assert.equal((await read()).undo,0,'same path on another machine cannot inherit history');
   }else if(scenario==='external-text'){
    await page.evaluate(()=>window.__historyProbe.hide());await page.locator('.cm-content').waitFor({state:'detached'});await wait(200);disk.set('/probe/a.md','# External replacement\n');await page.evaluate(()=>window.__historyProbe.show());await mounted();assert.equal((await read()).text,'# External replacement\n');assert.equal((await read()).undo,0,'external text cannot inherit old history');
   }
   if(['ordinary','preview','remount'].includes(scenario)){
    await page.evaluate(()=>window.__historyProbe.end());await page.keyboard.press('Meta+z');await wait(80);assert.equal((await read()).text,BASE,scenario+' restores undo');await page.keyboard.press('Meta+Shift+z');await wait(80);assert.equal((await read()).text,BASE+'added',scenario+' restores redo');
   }
   assert.deepEqual(errors,[]);console.log('PASS',name,scenario);
  }catch(error){failures.push(name+' '+scenario+': '+error.message);console.error('FAIL',failures.at(-1));await page.screenshot({path:join(evidence,name+'-'+scenario+'.png')}).catch(()=>{});}
  finally{await page.close();}
 }
 await browser.close();browser=null;
}
try{
 process.env.KHOR_FACE_GATE='1';server=await createServer({root:gui,optimizeDeps:{entries:[join(fixture,'index.html')]},server:{port:Number(process.env.EDITOR_HISTORY_VITE_PORT||23861),strictPort:true},clearScreen:false});await server.listen();
 await check(chromium,'chrome');
 if(process.env.EDITOR_HISTORY_WEBKIT==='1')await check(webkit,'webkit');
 if(failures.length)throw Error(failures.join('\n'));
 console.log('editor history: clean',evidence);
}finally{await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
