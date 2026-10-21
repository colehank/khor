// Production OmniMenu: dense material survives a nested backdrop root.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync,mkdtempSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import {chromium,webkit} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.MENU_MATERIAL_PORT??23748);
const prefix=`.menu-material-${process.pid}`,paths=[];
const put=(name,text)=>{const p=join(gui,name);paths.push(p);writeFileSync(p,text);return p;};
const evidence=mkdtempSync(join(process.env.KHOR_GATE_TMP??tmpdir(),'khor-menu-material-'));
let vite,browser;
try{
 put(`${prefix}.html`,`<!doctype html><html data-theme="dark"><body><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/${prefix}.tsx"></script></body></html>`);
 put(`scripts/${prefix}.tsx`,`import {createRoot} from 'react-dom/client';import {OmniMenu} from '../src/components/OmniMenu';import '../src/styles/tokens.css';import '../src/styles/app.css';
 const rows={groups:[{key:'paths',label:'',candidates:['khor-codex-identity','khor-ios-open','khor-session-indicators'].map(label=>({key:label,label}))}],at:-1};
 createRoot(document.getElementById('root')).render(<><div style={{position:'fixed',inset:0,color:'var(--foreground)',background:'var(--background)',fontSize:20,lineHeight:'28px'}}>{Array.from({length:30},(_,i)=><div key={i}>{'Underlying text 11f45799 '.repeat(12)}</div>)}</div><div id="host" className="glass" style={{position:'absolute',top:30,left:40,width:650,height:36,borderRadius:20}}><input aria-label="Directory" defaultValue="/Users/zgh/Desktop/working/khor"/><OmniMenu rows={rows} onPick={()=>{}} className="absolute inset-x-0 top-full z-20 mt-2"/></div><div id="normal" style={{position:'absolute',left:40,top:230,width:650}}><OmniMenu rows={rows} onPick={()=>{}}/></div></>);`);
 vite=spawn('node_modules/.bin/vite',['--host','127.0.0.1','--port',String(port),'--strictPort'],{cwd:gui,stdio:['ignore','pipe','pipe']});
 let ready=false,log='';vite.stdout.on('data',b=>ready||=String(b).includes(`:${port}`));vite.stderr.on('data',b=>log+=b);
 for(let i=0;i<100&&!ready;i++)await new Promise(r=>setTimeout(r,100));assert(ready,log||'Vite did not start');
 for(const [name,engine,opts] of [['chrome',chromium,{channel:'chrome'}],...(process.env.MENU_MATERIAL_WEBKIT==='1'?[['webkit',webkit,{}]]:[])]){
  browser=await engine.launch({headless:false,...opts});const page=await browser.newPage({viewport:{width:1000,height:450}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`http://127.0.0.1:${port}/${prefix}.html`);await page.locator('[data-omni-menu]').first().waitFor();
  for(const theme of ['light','dark'])for(const opacity of ['0%','70%','100%']){
   await page.evaluate(({theme,opacity})=>{document.documentElement.dataset.theme=theme;document.documentElement.toggleAttribute('data-window-transparent',true);document.documentElement.style.setProperty('--window-opacity',opacity);},{theme,opacity});
   const material=await page.locator('#host [data-menu-frame]').evaluate(el=>{const style=getComputedStyle(el),probe=document.createElement('div');probe.style.background='var(--popover)';document.body.append(probe);const thick=getComputedStyle(probe).backgroundColor;probe.remove();return {background:style.backgroundColor,thick};});
   assert.equal(material.background,material.thick,`${name} ${theme}/${opacity}: nested backdrop must not expose unblurred text`);
   const normal=await page.locator('#normal [data-menu-frame]').evaluate(el=>getComputedStyle(el).backdropFilter||getComputedStyle(el).webkitBackdropFilter);assert(normal.includes('blur('),'ordinary DOM menus keep their glass');
  }
  await page.locator('#normal').evaluate(el=>el.setAttribute('data-menu-native',''));
  const native=await page.locator('#normal [data-menu-frame]').evaluate(el=>getComputedStyle(el).backgroundColor);
  const thick=await page.locator('#host [data-menu-frame]').evaluate(el=>getComputedStyle(el).backgroundColor);assert.equal(native,thick,'independent native windows use the readable fallback');
  await page.screenshot({path:join(evidence,`${name}-dark.png`)});assert.deepEqual(errors,[]);
  console.log(`${name}: light/dark, 0/70/100% window tint, nested/native fallbacks and ordinary glass passed`);await browser.close();browser=null;
 }
 console.log('menu material evidence:',evidence);
}finally{if(browser)await browser.close();if(vite){vite.kill('SIGTERM');await new Promise(r=>{if(vite.exitCode!==null)r();else vite.once('exit',r);});}for(const p of paths)rmSync(p,{force:true});}
