import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,rmSync,mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join,basename} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const fixture=mkdtempSync(join(gui,'.updates-check-'));
const out=join(process.env.RUNNER_TEMP||tmpdir(),'khor-updates-ui');mkdirSync(out,{recursive:true});
writeFileSync(join(fixture,'index.html'),'<link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture,'probe.tsx'),`
import React from 'react';import{createRoot}from'react-dom/client';import{UpdateSettings}from'/src/views/more/UpdateSettings';
import{followUpdatePreparation}from'/src/lib/update-lifecycle';followUpdatePreparation();
import{TooltipProvider}from'/src/components/ui/tooltip';import'/src/styles/tokens.css';import'/src/styles/app.css';
createRoot(document.getElementById('root')).render(<TooltipProvider><UpdateSettings/></TooltipProvider>);`);
let server,browser;
try {
  server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:23839,strictPort:true,hmr:false},logLevel:'error'});await server.listen();
  browser=await chromium.launch({headless:true,...(process.env.KHOR_TEST_CHROME?{channel:'chrome'}:{})});
  const page=await browser.newPage({viewport:{width:800,height:760},locale:'zh-CN'});const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>{
    const callbacks=new Map();let next=1;
    window.updateCalls=[];
    window.updateState={source:'direct',phase:'idle',current:'v0.2.0-rc.1',built_from:'v0.2.0-rc.1',source_commit:'a'.repeat(40),available:null,notes:'',received:0,total:null,checked_at:null,failure:null,blocked:[],preferences:{channel:'beta',automatic_check:true,automatic_download:true,install_on_start:false}};
    let events=[];
    window.emitUpdate=patch=>{Object.assign(window.updateState,patch);for(const event of events)if(event.event==='app-update:changed')callbacks.get(event.handler)?.({event:'app-update:changed',id:event.id,payload:structuredClone(window.updateState)});};
    window.emitPreparation=(name,nonce,automatic=false)=>{for(const event of events)if(event.event===name)callbacks.get(event.handler)?.({event:name,id:event.id,payload:name==='app-update:prepare'?{nonce,automatic}:nonce});};
    window.__TAURI_INTERNALS__={transformCallback:callback=>{const id=next++;callbacks.set(id,callback);return id;},unregisterCallback:id=>callbacks.delete(id),invoke:async(cmd,args={})=>{
      window.updateCalls.push({cmd,args});
      if(cmd==='plugin:event|listen'){const id=next++;events.push({...args,id});return id;}
      if(cmd==='plugin:event|unlisten'){events=events.filter(event=>event.id!==args.eventId);return;}
      if(cmd==='app_update_status')return structuredClone(window.updateState);
      if(cmd==='app_update_prepared')return null;
      if(cmd==='app_update_check')window.emitUpdate({phase:'available',available:'v0.2.0-rc.2',total:100,checked_at:'2026-11-05T00:00:00Z'});
      else if(cmd==='app_update_preferences')window.emitUpdate({preferences:args.preferences,phase:'idle',available:null});
      else if(cmd==='app_update_download')window.emitUpdate({phase:'ready',received:100,total:100});
      else if(cmd==='app_update_install')window.emitUpdate({phase:'ready',blocked:['unsaved-files','attachments']});
      else throw new Error('unexpected native command '+cmd);
      return structuredClone(window.updateState);
    }};
  });
  await page.goto(`http://127.0.0.1:23839/${basename(fixture)}/`);
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='plugin:event|listen'&&call.args.event==='app-update:prepare'));
  await page.evaluate(()=>{const input=document.createElement('textarea');input.id='composition-fixture';document.body.append(input);input.focus();input.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true}));window.emitPreparation('app-update:prepare','ime');});
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='ime'));
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.find(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='ime').args.blocked),['input-in-progress']);
  assert.equal(await page.evaluate(()=>document.documentElement.inert),false,'preparing an update must not cancel an unfinished IME composition by freezing its editor');
  await page.evaluate(()=>{window.emitPreparation('app-update:resume','ime');document.getElementById('composition-fixture').dispatchEvent(new CompositionEvent('compositionend',{bubbles:true}));window.emitPreparation('app-update:prepare','settled');});
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='settled'));
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.find(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='settled').args.blocked),[]);
  assert.equal(await page.evaluate(()=>document.documentElement.inert),true);
  await page.evaluate(()=>{window.emitPreparation('app-update:resume','settled');document.getElementById('composition-fixture').remove();});
  const action=page.locator('[data-setting-row-name="update-check"] button');
  await action.click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_check'));
  await page.evaluate(()=>window.emitPreparation('app-update:prepare','after-use',true));
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='after-use'));
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.find(call=>call.cmd==='app_update_prepared'&&call.args.nonce==='after-use').args.blocked),['user-active']);
  assert.equal(await page.evaluate(()=>document.documentElement.inert),false,'an automatic startup install must defer after the user begins using the app');
  await page.evaluate(()=>window.emitPreparation('app-update:resume','after-use'));
  await action.click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_download'));
  await action.click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='app_update_install'));
  assert.match(await page.locator('[data-setting-row-name="update-check"]').innerText(),/保存|Save/,'a blocked restart explains the unresolved work');
  assert.match(await page.locator('[data-setting-row-name="update-check"]').innerText(),/附件|attachments/);
  await page.locator('[data-setting-select="update-channel"]').selectOption('stable');
  await page.waitForFunction(()=>window.updateState.preferences.channel==='stable');
  const prefs=await page.evaluate(()=>window.updateState.preferences);
  assert.deepEqual(prefs,{channel:'stable',automatic_check:true,automatic_download:true,install_on_start:false},'changing the channel must preserve the other preferences');
  await page.locator('[data-setting-row-name="update-install-on-start"] button').click();
  await page.waitForFunction(()=>window.updateState.preferences.install_on_start===true);
  await page.evaluate(()=>window.emitUpdate({phase:'downloading',received:42,total:100}));
  await page.waitForFunction(()=>document.querySelector('[data-setting-row-name="update-check"]').textContent.includes('42%'));
  assert.equal(await action.isDisabled(),true,'a native background operation owns its action');
  for(const source of ['store','external','development']) {
    await page.evaluate(source=>window.emitUpdate({source,phase:'idle',blocked:[]}),source);
    await page.locator('[data-more-fact="update-source"]').waitFor();
    assert.equal(await action.count(),0,'an externally owned installation has no direct-update action');
    assert.equal(await page.locator('[data-setting-row-name="update-channel"]').count(),0);
  }
  assert.deepEqual(errors,[]);
  await page.screenshot({path:join(out,'update-source.png')});
  console.log('PASS update UI: IME admission/freeze/resume, check/download/restart, blocked work, preserved channel preferences, native progress, installation ownership');
} finally {await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
