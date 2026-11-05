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
import React from 'react';import{createRoot}from'react-dom/client';import{UpdateSettings}from'/src/views/more/UpdateSettings';import{MachineUpdates}from'/src/views/MachineUpdates';
import{followUpdatePreparation}from'/src/lib/update-lifecycle';followUpdatePreparation();
import{TooltipProvider}from'/src/components/ui/tooltip';import'/src/styles/tokens.css';import'/src/styles/app.css';
createRoot(document.getElementById('root')).render(<TooltipProvider><UpdateSettings/><MachineUpdates row={window.machineRows[0]} rows={window.machineRows}/></TooltipProvider>);`);
let server,browser;
try {
  server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:23839,strictPort:true,hmr:false},logLevel:'error'});await server.listen();
  browser=await chromium.launch({headless:true,...(process.env.KHOR_TEST_CHROME?{channel:'chrome'}:{})});
  const page=await browser.newPage({viewport:{width:800,height:760},locale:'zh-CN'});const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>{
    const callbacks=new Map();let next=1;
    window.updateCalls=[];
    window.machineRows=['Alpha','Beta','App node'].map((shown,i)=>({id:String.fromCharCode(97+i).repeat(64),name:'host-'+i,shown,hosts:true,me:false,sponsor:null,face:null,vitals:null}));
    window.machineQueue={revision:0,jobs:[]};
    window.machineInfos=Object.fromEntries(window.machineRows.map((row,i)=>[row.id,{source:i===2?'app':'cli',running:'v0.2.0-rc.1',selected:'v0.2.0-rc.1',channel:'beta',service_ready:true,failed:null}]));
    window.updateState={source:'direct',phase:'idle',current:'v0.2.0-rc.1',built_from:'v0.2.0-rc.1',source_commit:'a'.repeat(40),available:null,notes:'',received:0,total:null,checked_at:null,failure:null,blocked:[],preferences:{channel:'beta',automatic_check:true,automatic_download:true,install_on_start:false}};
    let events=[];
    window.emitUpdate=patch=>{Object.assign(window.updateState,patch);for(const event of events)if(event.event==='app-update:changed')callbacks.get(event.handler)?.({event:'app-update:changed',id:event.id,payload:structuredClone(window.updateState)});};
    window.emitMachineQueue=patch=>{window.machineQueue=structuredClone(patch);for(const event of events)if(event.event==='machine-updates:changed')callbacks.get(event.handler)?.({event:'machine-updates:changed',id:event.id,payload:structuredClone(patch)});};
    window.emitPreparation=(name,nonce,automatic=false)=>{for(const event of events)if(event.event===name)callbacks.get(event.handler)?.({event:name,id:event.id,payload:name==='app-update:prepare'?{nonce,automatic}:nonce});};
    window.__TAURI_INTERNALS__={transformCallback:callback=>{const id=next++;callbacks.set(id,callback);return id;},unregisterCallback:id=>callbacks.delete(id),invoke:async(cmd,args={})=>{
      window.updateCalls.push({cmd,args});
      if(cmd==='plugin:event|listen'){const id=next++;events.push({...args,id});return id;}
      if(cmd==='plugin:event|unlisten'){events=events.filter(event=>event.id!==args.eventId);return;}
      if(cmd==='machine_update_queue')return structuredClone(window.machineQueue);
      if(cmd==='machine_update_status')return structuredClone(window.machineInfos[args.machine]);
      if(cmd==='machine_update_check')return {current:'v0.2.0-rc.1',available:'v0.2.0-rc.2',enabled:false};
      if(cmd==='machine_update_start'){
        if(window.machineStartFailOnce){window.machineStartFailOnce=false;throw new Error('fixture queue persistence failure');}
        const jobs=args.machines.map((machine,i)=>({machine,request:'choice-'+i,source:null,phase:'queued',channel:args.channel,target:'v0.2.0-rc.2',failure:null}));
        window.emitMachineQueue({revision:window.machineQueue.revision+1,jobs});return structuredClone(window.machineQueue);
      }
      if(cmd==='machine_update_cancel'){
        window.emitMachineQueue({revision:window.machineQueue.revision+1,jobs:window.machineQueue.jobs.filter(job=>job.machine!==args.machine||job.request!==args.request)});return structuredClone(window.machineQueue);
      }
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
  const machines=page.locator('[data-machine-updates]');
  await machines.getByRole('button',{name:/检查更新|Check for updates/}).click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='machine_update_check'));
  await machines.getByRole('button',{name:/开始更新|Start update/}).click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='machine_update_start'));
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.find(call=>call.cmd==='machine_update_start').args.machines),['a'.repeat(64)],'a machine action targets its stable identity, never its display name or the whole network');
  await page.evaluate(()=>window.emitMachineQueue({revision:10,jobs:[{...window.machineQueue.jobs[0],phase:'waiting'}]}));
  await machines.getByRole('button',{name:/取消等待|Cancel waiting/}).click();
  await page.waitForFunction(()=>window.updateCalls.some(call=>call.cmd==='machine_update_cancel'));
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.find(call=>call.cmd==='machine_update_cancel').args),{machine:'a'.repeat(64),request:'choice-0'});
  await machines.getByRole('button',{name:/更新多台机器|Update multiple machines/}).click();
  const dialog=page.getByRole('dialog');
  await dialog.locator('label').filter({hasText:'Beta'}).getByRole('checkbox').check();
  await page.evaluate(()=>{window.machineStartFailOnce=true;});
  await dialog.getByRole('button',{name:/开始更新|Start update/}).click();
  await dialog.getByRole('alert').waitFor();
  assert.equal(await dialog.locator('label').filter({hasText:'Beta'}).getByRole('checkbox').isChecked(),true,'a failed durable enqueue must preserve the reviewed selection');
  await dialog.getByRole('button',{name:/开始更新|Start update/}).click();
  await page.waitForFunction(()=>window.updateCalls.filter(call=>call.cmd==='machine_update_start').length===3);
  assert.deepEqual(await page.evaluate(()=>window.updateCalls.filter(call=>call.cmd==='machine_update_start').at(-1).args.machines),['b'.repeat(64)],'batch update installs only the reviewed selection');
  const job=await page.evaluate(()=>window.machineQueue.jobs[0]);
  await page.evaluate(job=>window.emitMachineQueue({revision:100,jobs:[{...job,phase:'installing'}]}),job);
  await page.waitForFunction(()=>/正在更新|Updating|Installing/.test(document.querySelector('[role="dialog"]').textContent));
  await page.evaluate(job=>window.emitMachineQueue({revision:99,jobs:[{...job,phase:'complete'}]}),job);
  assert.match(await dialog.innerText(),/正在更新|Updating/,'an older cross-window reply cannot overwrite newer progress');
  await page.evaluate(job=>window.emitMachineQueue({revision:101,jobs:[{...job,machine:'a'.repeat(64),source:'cli',phase:'complete'},{...job,source:'cli',phase:'failed'},{...job,machine:'c'.repeat(64),source:'app',phase:'failed'}]}),job);
  assert.match(await dialog.innerText(),/更新完成|Update complete/);
  assert.match(await dialog.innerText(),/更新未完成|Update.*failed|Update.*complete/);
  assert.match(await dialog.innerText(),/设置.*关于|Settings.*About/,'externally owned App installations keep their own update route');
  await page.setViewportSize({width:390,height:800});
  await page.screenshot({path:join(out,'machine-updates-mobile.png')});
  await dialog.locator('[data-slot="dialog-footer"]').getByRole('button',{name:/关闭|Close/}).click();
  await machines.getByRole('button',{name:/更新多台机器|Update multiple machines/}).click();
  assert.match(await page.getByRole('dialog').innerText(),/更新未完成|Update/,'closing the chooser does not lose the per-machine result');
  await page.getByRole('dialog').locator('[data-slot="dialog-footer"]').getByRole('button',{name:/关闭|Close/}).click();
  assert.deepEqual(errors,[]);
  await page.screenshot({path:join(out,'update-source.png')});
  console.log('PASS update UI: IME admission/freeze/resume, check/download/restart, blocked work, preserved channel preferences, native progress, installation ownership; machine stable IDs, explicit batch selection, waiting cancellation, independent outcomes and stale reply ordering');
} finally {await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
