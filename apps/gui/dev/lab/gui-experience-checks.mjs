import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import path from 'node:path';
export async function checkGuiExperience(browser,{port,report,engine}){
 const context=await browser.newContext({viewport:{width:1280,height:900}});
 const page=await context.newPage();page.setDefaultTimeout(15000);
 const errors=[];page.on('pageerror',error=>errors.push(String(error)));
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&items=3&machines=1&short-pages&handoff`);
  const input=page.locator('[data-chat-input]:visible');await input.waitFor();
  await page.locator('[data-chat-doc]').waitFor();
  await page.waitForFunction(()=>{const send=document.querySelector('[data-chat-send]');return send&&!send.disabled;});
  assert.equal(await input.inputValue(),'','A document alone offers Send without dummy text');
  await input.fill('Draft belongs to the original session');
  await api('__ai_preview/upload-mode',{mode:'fail'});
  await page.locator('input[type=file]').last().setInputFiles({name:'failed.txt',mimeType:'text/plain',buffer:Buffer.from('synthetic attachment')});
  const failed=page.locator('[data-chat-pending-file="failed.txt"]');await page.waitForFunction(()=>document.querySelector('[data-chat-pending-file="failed.txt"]')?.title.includes('上传失败'));
  assert.equal(await page.locator('[data-chat-send]').isDisabled(),true,'An upload failure blocks the entire send, including keyboard Enter');
  await input.press('Enter');assert.equal((await api('__ai_preview/says')).length,0);
  const original='gui/ai-elements-review',other='gui/ai-elements-plan';
  await page.locator(`[data-session-tabs] [data-tab-open="${other}"]`).click();
  await page.locator(`[data-session-tabs] [data-tab-open="${original}"]`).click();
  await page.locator('[data-chat-doc]').waitFor();await failed.waitFor();
  assert.equal(await input.inputValue(),'Draft belongs to the original session');
  await failed.locator('[data-chat-pending-drop]').click();
  await page.waitForFunction(()=>{const send=document.querySelector('[data-chat-send]');return send&&!send.disabled;});
  // Read older pages using a real reader gesture, without touching Load older.
  const scroll=page.locator('[data-chat-scroll]:visible');await scroll.hover();
  for(let index=0;index<5;index++){await page.mouse.wheel(0,-1600);await page.waitForTimeout(200);}
  await page.waitForFunction(()=>document.querySelectorAll('[data-chat-tool]').length>0);
  await page.locator('[data-chat-media-preview]').waitFor();await page.locator('[data-chat-media-preview]').click();
  await page.getByRole('dialog').waitFor();assert.equal(await page.getByRole('dialog').locator('img').evaluate(image=>image.complete&&image.naturalWidth>0),true);await page.keyboard.press('Escape');
  const anchor=await page.locator('[data-chat-scroll]').evaluate(element=>{
   const top=element.getBoundingClientRect().top,node=[...element.querySelectorAll('[data-history-key]')].find(node=>node.getBoundingClientRect().bottom>top);
   return {key:node.dataset.historyKey,top:node.getBoundingClientRect().top-top};
  });
  await page.locator(`[data-session-tabs] [data-tab-open="${other}"]`).click();
  await page.locator(`[data-session-tabs] [data-tab-open="${original}"]`).click();
  await page.waitForFunction(anchor=>{
   const root=document.querySelector('[data-chat-scroll]'),node=[...root.querySelectorAll('[data-history-key]')].find(node=>node.dataset.historyKey===anchor.key);
   return node&&Math.abs(node.getBoundingClientRect().top-root.getBoundingClientRect().top-anchor.top)<3;
  },anchor);
  assert(await page.locator('[data-copy-text]').count()>0,'Answer and code copying are discoverable');
  await page.locator('[data-lab-add]').click();await page.locator('[role=menuitem][data-history-find-open]').click();
  await page.locator('[data-history-search] input').waitFor();
  await page.screenshot({path:path.join(report,`gui-experience-${engine}.png`)});
  await page.keyboard.press('Escape');
  const download=page.waitForEvent('download');
  await page.locator('[data-lab-add]').click();await page.locator('[data-history-readable-export]').click();
  const exported=await download,body=await readFile(await exported.path(),'utf8');
  assert(body.includes('修一下编辑器')&&!body.includes('iVBOR'),'Readable export reads canonical text and media names, never encoded payloads');
  await api('__ai_preview/poll-mode',{mode:'fail'});
  await page.locator('[data-session-tabs] [data-tab-notice="rejoin"]').waitFor();
  assert.equal(await input.inputValue(),'Draft belongs to the original session');
  await api('__ai_preview/poll-mode',{mode:'ok'});
  await page.locator('[data-session-tabs] [data-tab-notice="rejoin"]').waitFor({state:'detached'});
  assert.equal((await api('__ai_preview/says')).length,0,'Reconnect only resumes reads; it never resends a prompt');
  assert.deepEqual(errors,[]);
  return {name:`GUI composer and reader ${engine}`,passed:true,errors};
 }catch(error){await page.screenshot({path:path.join(report,`gui-experience-${engine}-failure.png`)}).catch(()=>{});throw error;}finally{await context.close();}
}

export async function checkWorkspaceSessions(browser,{port,report,engine,mobile=false}){
 const viewport=mobile?{width:390,height:844}:{width:1500,height:900};
 const context=await browser.newContext({viewport,isMobile:mobile,hasTouch:mobile});
 const page=await context.newPage();page.setDefaultTimeout(15000);const errors=[];page.on('pageerror',error=>errors.push(String(error)));
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=workspace&items=3&machines=1`);
  if(mobile){await page.locator('[data-ws-mid] [data-session-bar] [data-back]:visible').waitFor();await page.locator('[data-ws-mid] [data-session-bar] [data-back]:visible').click();}
  const nav=page.locator('[data-workspace-navigation][data-seat="workspace"]');await nav.waitFor();
  await nav.locator('[data-workspace-view="sessions"]').click();
  const rows=page.locator('[data-ws-sessions-list] [data-row-open]');await rows.first().click();
  if(mobile){
   await page.locator('[data-ws-sessions] [data-chat-input]').waitFor();
   assert.equal(await page.locator('[data-workspace-session-tabs]:visible').count(),0);
   await page.evaluate(()=>window.dispatchEvent(new CustomEvent('khor:open-path',{detail:{machine:'',path:'/preview/project/README.md',dir:false,line:null,session:'gui/ai-elements-review'}})));
   await page.locator('[data-content-visible="true"] [data-file-page]').waitFor();
   assert.equal(await page.locator('[data-content-visible="true"] [data-session-bar]:visible').count(),1,'Workspace session files use the same bottom content pill');
   assert.deepEqual(page.viewportSize(),viewport);assert.deepEqual(errors,[]);
   await page.screenshot({path:path.join(report,`workspace-sessions-mobile-${engine}.png`)});
   return {name:`workspace shared mobile session ${engine}`,passed:true,errors};
  }
  await page.locator('[data-workspace-session-tabs]').waitFor();
  const id=await page.locator('[data-workspace-session-tabs] [data-tab-open]').first().getAttribute('data-tab-open');
  await page.locator(`[data-workspace-session-tabs] [data-tab-open="${id}"]`).click();
  await page.locator('[data-session-content-head]').waitFor();
  const readGeometry=()=>page.evaluate(()=>{
   const box=element=>{const r=element.getBoundingClientRect();return {top:r.top,height:r.height,width:r.width,left:r.left,right:r.right};};
   const session=document.querySelector('[data-workspace-session-tabs]'),content=document.querySelector('[data-session-content-head]');
   return {session:box(session.closest('[data-region-head]')),content:box(content.closest('[data-region-head]')),nested:session.contains(content),titles:document.querySelectorAll('[data-ws-sessions] [data-detail-header]').length};
  });
  const oldStyle=await page.addStyleTag({content:'[data-session-primary],[data-ws-sessions]{min-width:0!important}'});
  const oldGeometry=await readGeometry();
  assert(oldGeometry.session.width<200,'Removing the two column floors reproduces the squeezed-away session');
  await oldStyle.evaluate(element=>element.remove());
  const geometry=await readGeometry();
  assert(geometry.session.width>=200&&geometry.content.width>=200,`Both workspace columns must remain usable: ${JSON.stringify(geometry)}`);
  assert(!geometry.nested&&geometry.content.left>=geometry.session.right-1);
  assert(Math.abs(geometry.session.top-geometry.content.top)<=1&&Math.abs(geometry.session.height-geometry.content.height)<=1);
  assert.equal(geometry.titles,0,'Workspace sessions use the same tab header without an extra detail title');
  assert.deepEqual(page.viewportSize(),{width:1500,height:900});assert.deepEqual(errors,[]);
  await page.screenshot({path:path.join(report,`workspace-sessions-${engine}.png`)});
  return {name:`workspace shared session frame ${engine}`,passed:true,geometry,errors};
 }catch(error){await page.screenshot({path:path.join(report,`workspace-sessions-${mobile?"mobile":"desktop"}-${engine}-failure.png`)}).catch(()=>{});throw error;}finally{await context.close();}
}
