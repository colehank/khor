import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import path from 'node:path';
export async function checkGuiExperience(browser,{port,report,engine}){
 const context=await browser.newContext({viewport:{width:1280,height:900}});
 const page=await context.newPage();page.setDefaultTimeout(15000);
 const errors=[];let readingAnchor=null;page.on('pageerror',error=>errors.push(String(error)));
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
  const thumbnail=page.locator('[data-chat-media-preview]');await thumbnail.waitFor();
  const thumbnailBox=await thumbnail.boundingBox();await thumbnail.click();
  await page.getByRole('dialog').waitFor();const preview=page.getByRole('dialog').locator('img');
  assert.equal(await preview.evaluate(image=>image.complete&&image.naturalWidth>0),true);
  assert((await preview.boundingBox()).width>thumbnailBox.width,'The image viewer enlarges the thumbnail instead of using the small question-dialog width');
  const viewer=await page.getByRole('dialog').boundingBox(),viewport=page.viewportSize();
  assert(viewer.x>=0&&viewer.y>=0&&viewer.x+viewer.width<=viewport.width&&viewer.y+viewer.height<=viewport.height,'The shared preview stays inside the visible viewport');
  await page.keyboard.press('Escape');
  const anchor=await page.locator('[data-chat-scroll]').evaluate(element=>{
   const top=element.getBoundingClientRect().top,node=[...element.querySelectorAll('[data-history-key]')].find(node=>node.getBoundingClientRect().bottom>top);
   return {key:node.dataset.historyKey,top:node.getBoundingClientRect().top-top};
  });
  await page.locator(`[data-session-tabs] [data-tab-open="${other}"]`).click();
  await page.locator(`[data-session-tabs] [data-tab-open="${original}"]`).click();
  readingAnchor=anchor;
  await page.waitForFunction(anchor=>{
   const root=document.querySelector('[data-chat-scroll]'),node=[...root.querySelectorAll('[data-history-key]')].find(node=>node.dataset.historyKey===anchor.key);
   return node&&Math.abs(node.getBoundingClientRect().top-root.getBoundingClientRect().top-anchor.top)<3;
  },anchor);
  await page.locator('[data-chat-message] [data-message-actions]').first().waitFor();
  await page.locator('[data-lab-add]').click();await page.locator('[role=menuitem][data-history-find-open]').click();
  await page.locator('[data-history-search] input').waitFor();
  await page.screenshot({path:path.join(report,`gui-experience-${engine}.png`)});
  await page.keyboard.press('Escape');
  await api('__ai_preview/settings-hold',{hold:true});
  await page.locator('[data-lab-session-settings]:visible').click();
  await page.locator('[data-lab-option="model"]').click();
  await page.locator('[data-option-value="claude-opus-4-6"]').click();
  await page.getByRole('status').filter({hasText:'正在应用模型'}).waitFor();
  assert((await page.locator('[data-lab-session-settings]:visible').innerText()).includes('Sonnet'),'Queued transport does not replace the canonical model');
  assert.equal(await api('__ai_preview/settings-pending'),1,'Only one model change is in flight');
  await api('__ai_preview/settings-hold',{hold:false});
  await page.waitForFunction(()=>document.querySelector('[data-lab-session-settings]')?.textContent.includes('Opus'));
  await page.getByRole('status').filter({hasText:'正在应用模型'}).waitFor({state:'detached'});
  await page.keyboard.press('Escape');
  await api('__ai_preview/settings-refuse',{refuse:true});
  await page.locator('[data-lab-session-settings]:visible').click();
  await page.locator('[data-lab-option="model"]').click();
  await page.locator('[data-option-value="claude-sonnet-4-6"]').click();
  const refusal=page.getByRole('alert').filter({hasText:'未能确认模型'});await refusal.waitFor();
  assert((await page.locator('[data-lab-session-settings]:visible').innerText()).includes('Opus'),'A correlated refusal preserves the authoritative value');
  await api('__ai_preview/settings-refuse',{refuse:false});await refusal.getByRole('button',{name:'重试'}).click();
  await refusal.waitFor({state:'detached'});
  await page.waitForFunction(()=>document.querySelector('[data-lab-session-settings]')?.textContent.includes('Sonnet'));
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
  await api('__ai_preview/upload-mode',{mode:'ok'});
  await page.locator('input[type=file]').last().setInputFiles({name:'notes.txt',mimeType:'text/plain',buffer:Buffer.from('This selected file can be previewed.')});
  const card=page.locator('[data-chat-pending-file="notes.txt"][data-placed]');
  await card.locator('[data-chat-pending-preview]').click();
  const file=page.locator('[data-file-preview="/preview/uploads/notes.txt"]');
  await file.waitFor();
  await file.getByText('This selected file can be previewed.',{exact:false}).last().waitFor();
  await file.locator('[data-file-preview-close]').click();
  await card.waitFor();
  await page.locator('[data-chat-send]').click();
  await page.waitForFunction(()=>document.activeElement?.matches('[data-chat-input]'));
  await card.waitFor({state:'detached'});
  assert.deepEqual(errors,[]);
  return {name:`GUI composer and reader ${engine}`,passed:true,errors};
 }catch(error){
  errors.push(String(error));await page.screenshot({path:path.join(report,`gui-experience-${engine}-failure.png`)}).catch(()=>{});
  const reading=await page.locator('[data-chat-scroll]').evaluate(root=>({top:root.scrollTop,height:root.scrollHeight,viewport:root.clientHeight,
   items:[...root.querySelectorAll('[data-history-key]')].map(node=>({key:node.dataset.historyKey,top:node.getBoundingClientRect().top-root.getBoundingClientRect().top,text:node.textContent.slice(0,120)}))})).catch(()=>null);
  await writeFile(path.join(report,`gui-experience-${engine}-failure.json`),JSON.stringify({readingAnchor,reading,errors},null,2));
  return {name:`GUI composer and reader ${engine}`,passed:false,errors};
 }finally{await context.close();}
}

export async function checkWorkspaceSessions(browser,{port,report,engine,mobile=false,khor=false}){
 const viewport=mobile?{width:390,height:844}:{width:1500,height:900};
 const context=await browser.newContext({viewport,isMobile:mobile,hasTouch:mobile});
 const page=await context.newPage();page.setDefaultTimeout(15000);const errors=[];page.on('pageerror',error=>errors.push(String(error)));
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=workspace&items=3&machines=1`);
  if(mobile){await page.locator('[data-ws-mid] [data-session-bar] [data-back]:visible').waitFor();await page.locator('[data-ws-mid] [data-session-bar] [data-back]:visible').click();}
  const nav=page.locator('[data-workspace-navigation][data-seat="workspace"]');await nav.waitFor();
  await nav.locator('[data-workspace-view="sessions"]').click();
  const rows=page.locator('[data-ws-sessions-list] [data-row-open]');if(khor)await page.locator('[data-ws-sessions-list] [data-row="wide/all"] [data-row-open]').click();else await rows.first().click();
  if(khor){
   await page.locator('[data-ws-sessions] [data-machine-chat-scroll]:visible').waitFor();
   if(mobile)await page.locator('[data-ws-sessions] [data-lab-model-back]:visible').click();
   await page.locator('[data-ws-sessions-list] [data-side-disclosure="wide/all"]').click();
   await page.locator('[data-ws-sessions-list] [data-child-task-row]').first().click();
   const child=page.locator('[data-ws-sessions] [data-child-session]:visible');
   await child.locator('[data-child-session-message]').first().waitFor();
   assert.match(await child.innerText(),/3 checks passed/);
   assert.equal(await page.locator('[role="dialog"]:visible').count(),0);
   assert.equal(await page.locator('[data-chat-input]:visible,[data-chat-send]:visible').count(),0);
   const scopes=await page.evaluate(()=>Object.entries(localStorage).filter(([key])=>key.startsWith('khor.workspace.sessions.')).map(([,value])=>JSON.parse(value)));
   assert(scopes.some(scope=>scope.tabs.includes('wide/all')&&scope.tabs.some(id=>id.startsWith('delegated-session:'))),'The readonly child joins this workspace tab scope');
   if(mobile){
    await page.locator('[data-ws-sessions] [data-session-screen][data-deck-at="bottom"] [data-session-bar]:visible').waitFor();
    assert.equal(await page.locator('[data-ws-back]:visible').count(),0,'The session frame owns mobile navigation without an extra workspace back header');
    assert.equal(await page.locator('[data-side-l0]:visible').count(),0,'Workspace session frames suppress the landing search header just like main sessions');
   }
   else assert.equal(await page.locator('[data-workspace-session-tabs] [data-tab-open^="delegated-session:"]').count(),1);
   await page.screenshot({path:path.join(report,`workspace-child-${mobile?'mobile':'desktop'}-${engine}.png`)});
   await page.locator('[data-child-session-parent]:visible').click();
   await child.waitFor({state:'detached'});
   await page.locator('[data-ws-sessions] [data-machine-chat-scroll]:visible').waitFor();
   if(!mobile){assert.deepEqual(errors,[]);return {name:`workspace readonly child tab ${engine}`,passed:true,errors};}
  }
  if(mobile){
   await page.locator(khor?'[data-ws-sessions] [data-machine-chat]':'[data-ws-sessions] [data-chat-input]').waitFor();
   await page.locator('[data-ws-sessions] [data-lab-model-header]').waitFor();
   assert.equal(await page.locator('[data-workspace-session-tabs]:visible').count(),0);
   await page.evaluate(session=>window.dispatchEvent(new CustomEvent('khor:open-path',{detail:{machine:'',path:'/preview/project/README.md',dir:false,line:null,session}})),khor?'wide/all':'gui/ai-elements-review');
   await page.locator('[data-content-visible="true"] [data-file-page]').waitFor();
   const deck=page.locator('[data-content-visible="true"] [data-file-deck]');
   await deck.locator('[data-file-pill] [data-deck-band]').waitFor();
   // FilePage supplies the production file deck; it is not a second
   // SessionScreen. Wait for the sheet's entrance before checking reachability.
   await page.waitForFunction(()=>{
    const pill=document.querySelector('[data-content-visible="true"] [data-file-pill]');
    if(!pill)return false;const r=pill.getBoundingClientRect();
    return r.width>0&&r.height>0&&r.top>=0&&r.bottom<=innerHeight;
   });
   assert.equal(await deck.count(),1,'Workspace session files keep one reachable bottom content pill');
   assert.equal(await page.locator('[data-content-tabs]:visible').count(),0,'Mobile content never duplicates a desktop top tab strip');
   assert.deepEqual(page.viewportSize(),viewport);assert.deepEqual(errors,[]);
   await page.screenshot({path:path.join(report,`workspace-sessions-mobile-${engine}${khor?"-khor":""}.png`)});
   return {name:`workspace shared mobile session ${engine}${khor?" khor":""}`,passed:true,errors};
  }
  await page.locator('[data-workspace-session-tabs]').waitFor();
  const id=await page.locator('[data-workspace-session-tabs] [data-tab-open]').first().getAttribute('data-tab-open');
  await page.locator(`[data-workspace-session-tabs] [data-tab-open="${id}"]`).click();
  await page.locator('[data-session-content-head]').waitFor();
  await page.locator('[data-ws-sessions] [data-session-primary] [data-lab-session-settings]').waitFor();
  assert.equal(await page.locator('[data-ws-sessions] [data-lab-session-settings]:visible').count(),1,'Workspace settings stay in their own shared session header');
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
