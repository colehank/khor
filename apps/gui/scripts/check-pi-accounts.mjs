// Production account/model controls; only the authenticated node boundary is replaced.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync,mkdirSync,readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)), port=Number(process.env.PI_ACCOUNTS_PORT??23781);
const output=process.env.PI_ACCOUNTS_OUT??'/Volumes/disk0/khor-pi-account-proof';mkdirSync(output,{recursive:true});
const entry=`${gui}/scripts/.pi-accounts-${process.pid}.tsx`;
const providers=JSON.parse(readFileSync(`${gui}/../agent/providers.json`,'utf8'));
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let vite,browser;
try {
 vite=spawn('node_modules/.bin/vite',['--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false;vite.stdout.on('data',b=>{ready ||=String(b).includes(`:${port}`)});
 for(let n=0;n<100&&!ready;n++)await pause(100);assert(ready,'vite startup');
 writeFileSync(entry,`import React,{useState} from 'react';import{createRoot}from'react-dom/client';import{KhorAgentSettings}from'../src/views/KhorAgent';import{TooltipProvider}from'../src/components/ui/tooltip';import '../src/styles/tokens.css';import '../src/styles/app.css';
 function Gate(){const[view,setView]=useState(window.fixture);return <TooltipProvider><div className="flex h-screen flex-col p-pane"><KhorAgentSettings view={view} onChanged={setView} disabled={true}/></div></TooltipProvider>}; createRoot(document.getElementById('root')).render(<Gate/>);`);
 browser=await chromium.launch({channel:'chrome',headless:!process.env.PI_ACCOUNTS_HEADED,args:['--no-proxy-server','--disable-features=LocalNetworkAccessChecks']});
 for(const [width,theme]of [[1280,'light'],[1280,'dark'],[390,'light'],[390,'dark']]){
  const context=await browser.newContext({viewport:{width,height:844},isMobile:width<600,hasTouch:width<600});const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('response',r=>{if(r.status()>=400)errors.push(`${r.status()} ${r.url()}`)});
  const old={id:'existing',name:'Existing',home:'local',home_name:'Fixture Mac',provider:'anthropic',default_model:'account-default',configured:true,base_url:null,auth_type:'api_key'};
  let view={config:{home:'local',account:'existing',model:'conversation-old'},accounts:[old],run:null,data:null};const calls=[],jobs=new Map();let counter=0;
  await page.route(`http://127.0.0.1:${port}/**`,async route=>{
   const cmd=new URL(route.request().url()).pathname.slice(1),args=route.request().postDataJSON()??{};calls.push({cmd,args});let result=null;
   if(cmd==='devices')result=[{id:'local',name:'Fixture Mac',me:true,face:null,hosts:true,sponsor:null}];
   if(cmd==='khor_agent'){
    result=structuredClone(view);const p=args.payload??{};
    if(args.action==='catalog')result.data=providers;
    if(args.action==='configure'){view.config={home:args.machine,account:p.account,model:p.model};result=structuredClone(view)}
    if(args.action==='manage'){
     if(p.operation==='models'){const id=`models-${++counter}`;jobs.set(id,{status:'done',result:{models:[{id:'model-one',name:'Model One',contextWindow:128000,reasoning:true,image:true},{id:'model-two',name:'Model Two',contextWindow:128000,reasoning:false,image:false}]}});result.data={job:id,status:'running'}}
     if(p.operation==='login'){const id=`login-${++counter}`;jobs.set(id,{status:'waiting',ask:1,prompt:{type:p.authType==='oauth'?'manual_code':'secret',message:p.authType==='oauth'?'Fixture verification code':'Fixture API key'},events:p.authType==='oauth'?[{type:'device_code',userCode:'TEST-1234',verificationUri:'https://example.invalid/auth'}]:[],setup:p});result.data={job:id,status:'running'}}
     if(p.operation==='poll'){const job=jobs.get(p.job);const{setup,...safe}=job;result.data=safe}
     if(p.operation==='reply'){const job=jobs.get(p.job);assert.equal(p.ask,job.ask);const setup=job.setup;const account={...old,id:setup.account??'added',name:setup.name,provider:setup.provider,default_model:'',configured:true,base_url:setup.baseUrl||null,auth_type:setup.authType};view.accounts=[...view.accounts.filter(a=>a.id!==account.id),account];jobs.set(p.job,{status:'done',account:account.id,result:{credentialType:setup.authType}});result.data={status:'running'}}
     if(p.operation==='cancel'){jobs.set(p.job,{status:'cancelled'});result.data={status:'cancelled'}}
     if(p.operation==='default'){view.accounts.find(a=>a.id===p.account).default_model=p.model;result.data={status:'done'}}
    }
   }
   await route.fulfill({contentType:'application/json',body:JSON.stringify(result)});
  });
  await page.route('**/__pi_accounts?**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme="${theme}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script>window.fixture=${JSON.stringify(view)};localStorage.setItem('khor.lang','zh')</script><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module">import '/scripts/.pi-accounts-${process.pid}.tsx';</script></body></html>`}));
  await page.goto(`http://localhost:${port}/__pi_accounts?bridge=${port}`);
  const settings=page.locator('[data-khor-agent-settings]');await settings.waitFor().catch(async e=>{throw new Error(e.message+'\n'+errors.join('\n')+'\n'+await page.locator('body').innerText())});await settings.click();
  await page.getByText('回复进行中，切换从下一轮生效',{exact:true}).waitFor();
  const model=page.locator('[data-khor-agent-model] input');await model.fill('conversation-new');await model.press('Escape');await page.locator('[data-khor-agent-save]').click();
  assert.equal(view.config.model,'conversation-new');assert.equal(old.default_model,'account-default');
  await settings.click();await page.getByRole('combobox',{name:'账户',exact:true}).click();await page.locator('[data-choice-option="add"]').click();
  const provider=page.getByRole('combobox',{name:'模型提供商',exact:true});await provider.fill('anth');await page.locator('[data-choice-option="anthropic"]').click();
  await page.locator('[data-pi-connect]').click();await page.locator('[data-pi-auth-answer]').fill('fixture-only-key');await page.locator('[data-pi-auth-continue]').click();
  await page.locator('[data-pi-save-default]').waitFor();
  const defaultModel=page.locator('[data-pi-account-form] [data-khor-agent-model] input');await defaultModel.click();await page.locator('[data-choice-option="model-one"]').click();
  await page.screenshot({path:`${output}/${width}-${theme}-default.png`});
  const bounds=await page.locator('[data-pi-account-dialog]').boundingBox();assert(bounds&&bounds.x>=-1&&bounds.x+bounds.width<=width+1,'dialog width');assert(bounds.y>=-1&&bounds.y+bounds.height<=845,'dialog height');
  await page.locator('[data-pi-save-default]').click();await page.locator('[data-khor-agent-save]').click();assert.equal(view.config.account,'added');assert.equal(view.config.model,'model-one');
  await settings.click();await model.fill('model-two');await model.press('Escape');await page.locator('[data-khor-agent-save]').click();
  assert.equal(view.config.model,'model-two');assert.equal(view.accounts.find(a=>a.id==='added').default_model,'model-one','live switch overwrote default');
  // A cancelled provider-owned OAuth/manual-code interaction must stop on its home.
  await settings.click();await page.getByRole('combobox',{name:'账户',exact:true}).click();await page.locator('[data-choice-option="add"]').click();
  await provider.fill('openai-codex');await page.locator('[data-choice-option="openai-codex"]').click();await page.locator('[data-pi-connect]').click();await page.getByText('TEST-1234',{exact:true}).waitFor();
  await page.screenshot({path:`${output}/${width}-${theme}-oauth.png`});
  await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('[data-pi-account-dialog]'));
  await pause(200);assert(calls.some(c=>c.args.payload?.operation==='cancel'),'closing left the authentication flow alive');assert.deepEqual(errors,[]);
  console.log(`${width}/${theme}: provider search, secret prompt, defaults, live model switch, OAuth and cancel passed`);await context.close();
 }
}finally{await browser?.close();vite?.kill('SIGTERM');rmSync(entry,{force:true});}
