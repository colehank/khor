// Production registration dialog; disposable transport, no user node or login.
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {writeFileSync,rmSync,mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {chromium} from 'playwright-core';
const root=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.SUBSCRIPTION_PORT||23610);
const serving=process.argv.includes('--serve');
const entry=path.join(root,`scripts/fixtures/.subscription-import-${process.pid}.tsx`);
const shots=process.env.SUBSCRIPTION_SHOTS||'/private/tmp/khor-subscription-import';
const id=n=>String(n).repeat(64);
const devices=[1,2,3,4].map(n=>({id:id(n),name:['','MacLab','turing','hinton','lecun'][n],shown:['','MacLab','turing','hinton','lecun'][n],me:n===1,hosts:true,sponsor:null,paused:false,face:null,hop:n===3?'quiet':'direct',down_ms:n===3?60000:null}));
let server,browser,imports=[],discovery=[],attempts=0,retries=0;
writeFileSync(entry,`import React,{useState} from 'react';import{createRoot}from'react-dom/client';
import{RegisterDialog}from'../../src/views/QuotaLanding';import{TooltipProvider}from'../../src/components/ui/tooltip';
import{speakLang}from'../../src/lang';import{NARROW_QUERY}from'../../src/hooks/use-narrow';
import'../../src/styles/tokens.css';import'../../src/styles/app.css';speakLang('zh');
const posture=()=>{document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches);document.documentElement.toggleAttribute('data-coarse',matchMedia('(pointer:coarse)').matches)};posture();addEventListener('resize',posture);
function Fixture(){const[open,setOpen]=useState(false);const[devices,setDevices]=useState(${JSON.stringify(devices)});window.refreshMachines=()=>setDevices(d=>d.map((row,i)=>i===3?{...row,paused:true}:row));return <TooltipProvider><button onClick={()=>setOpen(true)}>+</button><RegisterDialog open={open} onOpenChange={setOpen} devices={devices} registered={['existing']} onRegistered={id=>{window.imported=id}}/></TooltipProvider>}createRoot(document.getElementById('root')).render(<Fixture/>);`);
try{
 server=await createServer({root,configFile:path.join(root,'vite.config.ts'),server:{port,host:'127.0.0.1',strictPort:true},optimizeDeps:{entries:[entry]},plugins:[{name:'subscription-fixture',configureServer(s){s.middlewares.use(async(req,res,next)=>{
 if(req.url.startsWith('/subscription-proof')){res.setHeader('Content-Type','text/html');return res.end(`<!doctype html><html data-theme="light"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/fixtures/${path.basename(entry)}"></script></html>`)}
 if(req.method!=='POST')return next();let body='';for await(const b of req)body+=b;const args=JSON.parse(body||'{}');let result;
 if(req.url==='/subscription_candidates'){
 discovery.push(args.machine);assert.deepEqual(Object.keys(args),['machine']);
 if(args.machine===id(1))result=[{provider:'claude',vendor:'Claude Code',account:'existing'},{provider:'codex',vendor:'Codex',account:null}];
 else if(args.machine===id(2))result=[{provider:'claude',vendor:'Claude Code',account:null},{provider:'codex',vendor:'Codex',account:null}];
 else if(args.machine===id(3)){if(retries++===0){await new Promise(r=>setTimeout(r,1500));res.statusCode=500;result='hinton 不通'}else result=[]}
 else if(serving)result=[{provider:'claude',vendor:'Claude Code',account:null}];
 else{res.statusCode=500;result='这台机器的 khor 还不支持发现订阅，升级后重试'}
 }else if(req.url==='/import_subscription'){
 imports.push(args);assert(devices.some(d=>d.id===args.machine)&&['claude','codex'].includes(args.provider));
 await new Promise(r=>setTimeout(r,300));if(!serving&&attempts++===0){res.statusCode=500;result='turing 上的登录已失效'}else result='remote-account';
 }else if(req.url==='/register_account'){imports.push(args);result='billed-account'}
 else{res.statusCode=500;result='Unexpected fixture command '+req.url}
 res.setHeader('Content-Type','application/json');res.end(JSON.stringify(result));
 })}}]});await server.listen();
 if(serving){
  console.log(`Subscription fixture: http://127.0.0.1:${port}/subscription-proof?bridge=${port}`);
  for(const signal of ['SIGINT','SIGTERM'])process.on(signal,async()=>{await server.close();rmSync(entry,{force:true});process.exit(0)});
  await new Promise(()=>{});
 }
 browser=await chromium.launch({channel:'chrome',headless:true});mkdirSync(shots,{recursive:true});
 const errors=[];
 for(const width of [1100,390]){
  imports=[];discovery=[];attempts=0;retries=0;
  const page=await browser.newPage({viewport:{width,height:800},hasTouch:width<700});page.on('pageerror',e=>errors.push(e.message));
  await page.goto(`http://127.0.0.1:${port}/subscription-proof?bridge=${port}`);
  await page.getByRole('button',{name:'+',exact:true}).click();
  await page.locator('[data-quota-kind-pick="sub"]').click();
  const search=page.getByRole('searchbox',{name:'搜索可导入的订阅'});
  const machine=page.locator('[data-subscription-machines] [role="combobox"]');
  const rows=page.locator('[data-subscription-provider]');
  const option=n=>page.locator(`[data-choice-option="${n===0?'all':id(n)}"]`);
  const choose=async n=>{await machine.click();await option(n).click();await search.click()};
  const remote=page.locator(`[data-subscription-machine="${id(2)}"][data-subscription-provider="codex"]`);
  await remote.waitFor();assert.equal(await machine.inputValue(),'全部');
  assert.equal(await page.locator(`[data-subscription-machine="${id(1)}"][data-subscription-provider="claude"]`).count(),0,'registered subscriptions are omitted');
  assert.equal(imports.length,0,'discovery must never import');
  assert.equal(await page.locator('[data-subscription-import]').count(),0,'tap result imports without a second action');
  assert.equal(await page.locator('[data-slot="dialog-footer"] button').count(),1,'footer has only cancel');
  await choose(3);
  await page.locator(`[data-subscription-status="${id(3)}"]`).getByText('hinton · hinton 不通',{exact:true}).waitFor();
  await page.locator(`[data-subscription-status="${id(3)}"]`).getByRole('button',{name:'重试',exact:true}).click();
  await page.getByText('没有可导入的订阅',{exact:true}).waitFor();
  assert.equal(await rows.count(),0,'explicit offline selection excludes other machines');
  await machine.click();await machine.fill('turing');
  assert.equal(await page.getByRole('option').count(),1,'machine row supports search');
  await option(2).click();await search.click();
  await page.waitForFunction(()=>document.querySelector('[data-subscription-machines] input')?.value==='已选 2 台机器',{},{timeout:5000});
  assert.equal(await rows.count(),2,'machine selection uses OR');
  await choose(0);assert.equal(await machine.inputValue(),'全部');
  await page.getByText('lecun · 这台机器的 khor 还不支持发现订阅，升级后重试',{exact:true}).waitFor();
  await choose(2);assert.equal(await rows.count(),2);
  await search.fill('not-a-subscription');await page.getByText('没有匹配项',{exact:true}).waitFor();
  await page.locator('[data-import-clear]').click();assert.equal(await search.inputValue(),'');
  await search.fill('turing codex');assert.equal(await rows.count(),1,'search covers vendor and source machine');
  for(const theme of ['light','dark']){
   await page.evaluate(t=>document.documentElement.dataset.theme=t,theme);
   await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
   await page.screenshot({animations:'disabled',path:`${shots}/subscriptions-${width}-${theme}.png`});
  }
  await remote.click();await page.getByText('turing 上的登录已失效',{exact:true}).waitFor();
  assert.equal(await search.inputValue(),'turing codex');assert.equal(await machine.inputValue(),'turing','failed import keeps filters');
  await search.press('Enter');await remote.waitFor({state:'visible'});
  assert.equal(await remote.getAttribute('aria-busy'),'true');assert(await machine.isDisabled(),'pending import holds machine filter');
  await remote.click({force:true});await page.evaluate(()=>window.refreshMachines());
  await page.waitForFunction(()=>window.imported==='remote-account');
  assert.deepEqual(imports,[{machine:id(2),provider:'codex'},{machine:id(2),provider:'codex'}],'one request per explicit attempt');
  assert.deepEqual(new Set(discovery),new Set(devices.map(d=>d.id)));
  await page.getByRole('button',{name:'+',exact:true}).click();
  await page.locator('[data-quota-name]').fill('Gateway');await page.locator('[data-quota-address]').fill('https://fixture.invalid');await page.locator('[data-quota-key]').fill('fixture-key');await page.locator('[data-quota-create]').click();
  await page.waitForFunction(()=>window.imported==='billed-account');assert.equal(imports[2].kind,'bill','billed registration still uses its key form');
  await page.close();console.log(`subscription import · ${width}: shared machine picker, search/multiselect/all, independent discovery, empty/down/old peer, retry, direct import, bill regression`);
 }
 assert.deepEqual(errors,[],'browser errors');console.log(`subscription import: clean; screenshots ${shots}`);
}finally{await browser?.close();await server?.close();rmSync(entry,{force:true});}
