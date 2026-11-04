import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,mkdirSync,rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join,basename} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium,webkit} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const fixture=mkdtempSync(join(gui,'.khor-conversations-check-'));
const out=join(process.env.RUNNER_TEMP||tmpdir(),'khor-conversations-ui-evidence');
mkdirSync(out,{recursive:true});
const A='wide/agent-'+'a'.repeat(64),B='wide/agent-'+'b'.repeat(64),home='c'.repeat(64);
writeFileSync(join(fixture,'index.html'),'<link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture,'probe.tsx'),`
import React from 'react';import{createRoot}from'react-dom/client';import{flushSync}from'react-dom';
import{NewSessionPage}from'/src/views/NewSessionPage';import{MachineChat}from'/src/views/MachineChat';
import{TooltipProvider}from'/src/components/ui/tooltip';import'/src/styles/tokens.css';import'/src/styles/app.css';
const root=createRoot(document.getElementById('root'));let serial=0;
const render=element=>flushSync(()=>root.render(<TooltipProvider><main style={{height:'100vh',display:'flex',flexDirection:'column'}}>{element}</main></TooltipProvider>));
window.showChat=id=>render(<MachineChat key={id} id={id} active sessions={[]}/>);
window.showNew=()=>render(<NewSessionPage key={'new-'+(++serial)} machine="fixture" dir="/fixture" onCancel={()=>{}} onOpened={window.showChat}/>);
window.showNew();`);
let server,browser;
const results=[];
try {
 server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:23837,strictPort:true,hmr:false},logLevel:'error'});await server.listen();
 const port=server.httpServer.address().port;
 for(const [name,engine] of [['chromium',chromium],...(process.env.QUALITY_ENGINES?.includes('webkit')?[['webkit',webkit]]:[])]) {
  browser=await engine.launch({headless:true});
  const page=await browser.newPage({viewport:{width:1000,height:760}});page.setDefaultTimeout(12000);
  const errors=[],calls=[],histories=new Map([[A,[]],[B,[]]]),creates=new Map();let failedCreate=false,failNextCreate=false,holdNextCreate=false,releaseCreate;
  page.on('pageerror',error=>{errors.push(error.message);console.error(error.message);});
  const config={home,account:'fixture-account',model:'fixture-model',options:{}};
  const account={id:config.account,name:'Fixture account',home,home_name:'fixture',provider:'anthropic',default_model:config.model,configured:true,base_url:null,auth_type:'api_key'};
  const record=(id,line,agent=false)=>({id,at_ms:Date.now(),text:null,from:'fixture',from_me:true,to:[],files:null,note:null,unknown:null,retracted:false,
   command:{id,line,owner:home,owner_name:'fixture',status:agent?'running':'done',started_at_ms:Date.now(),ended_at_ms:agent?null:Date.now(),stdout:agent?'':line+' result',stderr:'',data_json:null,session:null,error:null,
    agent:agent?{model:config.model,approval:null,updates:[],updates_truncated:false,updated_at_ms:Date.now()}:null}});
  await page.route(`http://127.0.0.1:${port}/**`,async route=>{
   if(route.request().method()==='OPTIONS'){await route.fulfill({status:204,headers:{'access-control-allow-origin':'*','access-control-allow-methods':'POST, OPTIONS','access-control-allow-headers':'content-type'}});return;}
   if(route.request().method()!=='POST'){await route.continue();return;}
   const cmd=new URL(route.request().url()).pathname.slice(1),args=route.request().postDataJSON()??{};calls.push({cmd,args});let result=null;
   if(cmd==='devices')result=[{id:home,name:'fixture',shown:'Fixture',me:true,sponsor:null,hosts:true,pinned:false,paused:false,vitals:null,seat:{left:0,top:0},roads:[]}];
   else if(cmd==='agents')result=[{name:'fixture-acp'}];
   else if(cmd==='launch_presets')result=[];
   else if(cmd==='ls')result={path:'/fixture',entries:[],truncated:false};
   else if(cmd==='machine_log')result={messages:histories.get(args.id)??[],broken:0};
   else if(cmd==='console_catalog')result=[{word:'status',aliases:[],usage:'status',reach:'local',mode:'read',parameters:[]}];
   else if(cmd==='console_complete')result={start:0,end:args.caret,choices:args.line==='/sta'?[{value:'/status',label:'/status',detail:'status',kind:'command'}]:[]};
   else if(cmd==='console_run'){histories.get(args.session).push(record('command-a',args.line));result='command-a';}
   else if(cmd==='khor_agent') {
    const {action,payload}=args;result={config,accounts:[account],run:null,data:{conversation:{home}}};
    if(action==='create') {
     if(!creates.has(payload.token))creates.set(payload.token,creates.size===0?A:B);
     if(!failedCreate||failNextCreate){failedCreate=true;failNextCreate=false;await route.fulfill({status:503,headers:{'access-control-allow-origin':'*'},body:'Lost create response fixture'});return;}
     if(holdNextCreate){holdNextCreate=false;await new Promise(resolve=>{releaseCreate=resolve;});}
     result.data={session:creates.get(payload.token)};
    } else if(action==='prompt'){const id=payload.session===A?'task-a':'task-b';histories.get(payload.session).push(record(id,payload.text,true));result.run=id;}
    else if(action==='stop'){const item=histories.get(payload.session).find(m=>m.id===payload.run);item.command.status='done';item.command.ended_at_ms=Date.now();}
    else if(action==='manage')result.data=payload.operation==='poll'?{status:'done',result:{configOptions:[]}}:{job:'fixture-settings',status:'running'};
   }
   await route.fulfill({contentType:'application/json',headers:{'access-control-allow-origin':'*'},body:JSON.stringify(result)});
  });
  // The page and API share one server; bridge POSTs are intercepted above.
  await page.goto(`http://localhost:${port}/${basename(fixture)}/index.html?bridge=${port}`);
  const choose=async id=>{await page.locator('[data-new-session-launch] textarea').click();await page.locator(`[data-choice-option="${id}"]`).click();};
  await choose('agent:fixture-acp');await page.locator('[data-acp-setup] > summary').click();await page.getByLabel('MCP servers',{exact:true}).fill('{');
  assert(await page.locator('[data-new-session-create]').isDisabled());
  await choose('khor');await page.locator('[data-new-session-create]').click();
  await page.locator('[data-new-session-error]').waitFor();
  await page.locator('[data-new-session-create]').click();await page.locator('[data-chat-composer]').waitFor();
  const first=calls.filter(c=>c.cmd==='khor_agent'&&c.args.action==='create');
  assert.equal(first.length,2);assert.equal(first[0].args.payload.token,first[1].args.payload.token);assert.equal(first[0].args.payload.title,'');
  await page.locator('[data-chat-composer]').fill('Task A');await page.locator('[data-chat-composer]').press('Enter');
  await page.locator('[data-khor-agent-turn=task-a]').waitFor();await page.locator('[data-chat-stop]').click();
  assert.equal(calls.find(c=>c.cmd==='khor_agent'&&c.args.action==='stop').args.payload.session,A);
  await page.evaluate(()=>window.showNew());await choose('khor');await page.locator('[data-new-session-name]').fill('Manual B');
  await page.locator('[data-new-session-create]').click();await page.locator('[data-chat-composer]').waitFor();
  assert.notEqual(calls.filter(c=>c.cmd==='khor_agent'&&c.args.action==='create').at(-1).args.payload.token,first[0].args.payload.token);
  assert.equal(await page.locator('[data-khor-agent-turn=task-a]').count(),0);
  await page.locator('[data-chat-composer]').fill('Task B');await page.locator('[data-chat-composer]').press('Enter');await page.locator('[data-khor-agent-turn=task-b]').waitFor();
  await page.evaluate(A=>window.showChat(A),A);await page.locator('[data-khor-agent-turn=task-a]').waitFor();
  assert.equal(await page.locator('[data-khor-agent-turn=task-b]').count(),0);
  await page.locator('[data-chat-composer]').fill('/sta');await page.locator('[data-console-choice=command]').waitFor();
  await page.locator('[data-chat-composer]').press('Tab');await page.locator('[data-console-choice=command]').waitFor({state:'detached'});await page.locator('[data-chat-composer]').press('Enter');await page.locator('[data-console-result]').waitFor();
  assert.equal(calls.find(c=>c.cmd==='console_run').args.session,A);
  assert(!calls.some(c=>c.cmd==='khor_agent'&&c.args.action==='prompt'&&c.args.payload.text.startsWith('/')));
  assert(!calls.some(c=>c.cmd==='post'));
  await page.evaluate(B=>window.showChat(B),B);await page.locator('[data-khor-agent-turn=task-b]').waitFor();
  assert.equal(await page.locator('[data-console-result]').count(),0);
  await page.evaluate(()=>window.showNew());await choose('khor');failNextCreate=true;
  await page.locator('[data-new-session-create]').click();await page.locator('[data-new-session-error]').waitFor();
  const abandoned=calls.filter(c=>c.cmd==='khor_agent'&&c.args.action==='create').at(-1).args.payload.token;
  await page.evaluate(()=>window.showNew());await choose('khor');
  await page.locator('[data-new-session-create]').click();await page.locator('[data-chat-composer]').waitFor();
  const replacement=calls.filter(c=>c.cmd==='khor_agent'&&c.args.action==='create').at(-1).args.payload.token;
  assert.notEqual(replacement,abandoned,'a fresh creation form must not reuse an abandoned request token');
  await page.evaluate(()=>window.showNew());await choose('khor');holdNextCreate=true;
  const requested=page.waitForRequest(request=>request.method()==='POST'&&new URL(request.url()).pathname==='/khor_agent'&&request.postDataJSON().action==='create');
  await page.locator('[data-new-session-create]').click();const oldRequest=await requested;
  await page.evaluate(()=>window.showNew());await choose('khor');
  assert.equal(typeof releaseCreate,'function');
  const answered=page.waitForResponse(response=>response.request()===oldRequest);
  releaseCreate();await answered;await page.evaluate(()=>new Promise(requestAnimationFrame));
  assert.equal(await page.locator('[data-new-session-page]').count(),1,'a response from an abandoned form must not navigate away from the new form');
  assert.equal(await page.locator('[data-chat-composer]').count(),0);
  assert.deepEqual(errors,[]);results.push({engine:name,passed:true});await browser.close();browser=null;
 }
 console.log('Khor conversations UI: create retry/abandonment, independent tabs, model task/stop and Khor slash routing passed');
} catch(error){if(browser){try{await browser.contexts()[0]?.pages()[0]?.screenshot({path:join(out,'failure.png')});}catch{}}throw error;}
finally{writeFileSync(join(out,'report.json'),JSON.stringify(results,null,2));await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
