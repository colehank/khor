import assert from 'node:assert/strict';
import path from 'node:path';
export async function checkAttachmentDraft(browser,{port,report,engine,width=390}){
 const height=width===320?568:844;
 const context=await browser.newContext({viewport:{width,height},hasTouch:width<500,isMobile:width<500});
 const page=await context.newPage(),errors=[];page.setDefaultTimeout(15000);page.on('pageerror',error=>errors.push(String(error)));
 const theme=width===412?'dark':'light',name=`attachment-draft-${engine}-${width}`;let splitLayout=null;
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 const green=async selector=>assert.equal(await page.locator(selector).evaluate(button=>{const probe=document.createElement('i');probe.style.backgroundColor='var(--brand-strong)';document.body.append(probe);const expected=getComputedStyle(probe).backgroundColor;probe.remove();return !button.disabled&&getComputedStyle(button).backgroundColor===expected;}),true);
 const picker=page.locator('[data-attachment-picker]:visible');
 const sources=async()=>{for(let n=0;n<6&&await picker.locator('[data-files-machine]').count()===0;n++)await picker.locator('[data-files-up]').click();await picker.locator('[data-files-machine]').first().waitFor();};
 const choose=async file=>{await picker.locator('[data-files-input]').setInputFiles(file);};
 const selected=name=>picker.locator(`[data-files-selected="${name}"]`);
 const remove=async name=>{await selected(name).getByRole('button',{name:`移除 ${name}`,exact:true}).click();};
 try{
  const mobile=width<500,environment={platform:mobile?width===412?'android':'ios':'desktop',theme,material:'solid',background:'dark',opacity:100,keyboard:0,safeTop:mobile?width===412?24:47:0,safeBottom:mobile?width===412?24:34:0};
  await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);
  await page.locator('[data-terminal] .xterm').waitFor();
  if(!mobile){
   await page.locator('[data-term-attachments]:visible').click();await picker.waitFor();
   assert.equal(await picker.locator('[data-files-send]').isDisabled(),true);
   await picker.locator('[data-files-source]').waitFor();await picker.locator('[data-files-selection]').waitFor();
   assert.equal(await picker.locator('[data-files-pick="photos"]').count(),0);
   assert.equal(await picker.locator('[data-files-input]').getAttribute('accept'),null,'The unified system picker must not be constrained to photos or files');
   await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});
  }
  await page.locator('[data-terminal]').evaluate(element=>{const data=new DataTransfer();data.items.add(new File(['original attachment'],'draft.txt',{type:'text/plain'}));element.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});
  await picker.waitFor();await selected('draft.txt').waitFor();
  assert.equal((await api('__ai_preview/attachment-events')).length,0,'Selection never uploads');
  assert.match(await picker.locator('[data-send-into-path]').innerText(),/khor-drop/);await green('[data-files-send]:visible');
  await selected('draft.txt').locator('[data-files-card-preview]').click();
  await page.locator('[data-attachment-text]').fill('edited attachment');await page.locator('[data-attachment-name]').fill('edited.txt');
  await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});await selected('edited.txt').waitFor();
  await picker.locator('[data-files-machine][data-files-session]').click();await picker.locator('[data-files-entry]').first().waitFor();
  if(await picker.locator('[data-files-entry="file"]').count()===0)await picker.locator('[data-files-entry="dir"][data-files-name="project"] > button').first().click();
  const row=picker.locator('[data-files-entry="file"]').first(),remoteName=await row.getAttribute('data-files-name');
  await row.locator('button').first().click();
  await selected(remoteName).waitFor();
  assert.equal(await page.locator('[data-file-preview]:visible').count(),0,'A source row selects without requiring preview');
  assert.equal(await row.locator('[data-files-select]').getAttribute('aria-pressed'),'true');
  assert.equal(await selected('edited.txt').count(),1,'The lower list persists while browsing another source');
  await sources();await picker.locator('[data-files-machine][data-files-session]').click();await picker.locator('[data-files-select="project"]').click();
  await selected('project').waitFor();
  assert.equal(await selected(remoteName).count(),0,'Selecting a directory folds its selected child into the single directory row');
  await selected('project').locator('[data-files-card-preview]').click();
  const folder=page.locator('[data-attachment-folder]:visible');await folder.waitFor();await folder.locator('[data-attachment-folder-entry="file"]').first().click();
  await page.locator('[data-file-preview]:visible').waitFor();await page.locator('[data-file-preview-close]:visible').click();await folder.waitFor();
  await folder.locator('[data-task-action="leading"] button').click();await folder.waitFor({state:'hidden'});
  await page.screenshot({path:path.join(report,name+'-chooser.png')});
  // A long lower list must not replace or move source browsing.
  const extras=Array.from({length:10},(_,i)=>({name:`extra-${i}.txt`,mimeType:'text/plain',buffer:Buffer.from('queued')}));
  await choose(extras);
  splitLayout=await picker.evaluate(root=>{
   const source=root.querySelector('[data-files-source]'),chosen=root.querySelector('[data-files-selection]'),scroll=root.querySelector('[data-files-selection-scroll]'),sourceScroll=root.querySelector('[data-files-source-scroll]');
   const rect=node=>{const r=node.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height};};
   const before=sourceScroll.scrollTop;scroll.scrollTop=scroll.scrollHeight;
   return {dialog:rect(root),source:rect(source),chosen:rect(chosen),lowerScroll:scroll.scrollTop,sourceUnchanged:sourceScroll.scrollTop===before,
    rows:[...root.querySelectorAll('[data-files-selected]')].map(row=>({row:rect(row),list:rect(row.parentElement),open:rect(row.querySelector('button')),remove:rect(row.querySelector('[data-chat-pending-drop]'))}))};
  });
  assert(splitLayout.source.bottom<=splitLayout.chosen.top+1,'The source stays above the persistent selected list');
  assert(splitLayout.source.height>44&&splitLayout.chosen.height>44,'Both areas remain usable in short windows');
  assert(splitLayout.lowerScroll>0&&splitLayout.sourceUnchanged,'The selected list scrolls independently');
  assert(splitLayout.dialog.left>=0&&splitLayout.dialog.right<=width+1&&splitLayout.dialog.top>=0&&splitLayout.dialog.bottom<=height+1,'The task fits the viewport');
  for(const row of splitLayout.rows){assert(Math.abs(row.row.width-row.list.width)<=1,'One full-width row per file/directory');assert(row.open.right<=row.remove.left+1,'Preview and remove have separate targets');}
  for(let n=0;n<10;n++)await remove(`extra-${n}.txt`);
  await remove('project');
  const unchanged=Buffer.from('\ufeffpreserve original bytes');await choose({name:'keep.txt',mimeType:'text/plain',buffer:unchanged});
  await selected('keep.txt').locator('[data-files-card-preview]').click();await page.locator('[data-attachment-text]').waitFor();await page.locator('[data-attachment-name]').fill('renamed.txt');
  await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});
  await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});
  await page.locator('[data-term-attachment-resume]:visible').click();await picker.waitFor();await selected('edited.txt').waitFor();await selected('renamed.txt').waitFor();
  await page.screenshot({path:path.join(report,name+'-terminal.png')});
  await api('__ai_preview/upload-mode',{mode:'fail'});await picker.locator('[data-files-send]').click();await picker.getByRole('alert').waitFor();
  assert.equal(await selected('edited.txt').count(),1,'Failure retains selected rows');assert.equal((await api('__ai_preview/attachment-events')).length,0);
  await api('__ai_preview/upload-mode',{mode:'ok'});await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const terminal=await api('__ai_preview/attachment-events');
  assert(terminal.some(event=>event.operation==='term_drop_stage'&&event.name==='edited.txt'&&event.content==='edited attachment'));
  assert(terminal.some(event=>event.operation==='term_drop_stage'&&event.name==='renamed.txt'&&event.bytes===unchanged.length),'Rename preserves BOM bytes');
  assert(terminal.some(event=>(event.operation==='term_drop'||event.operation==='term_drop_items')&&event.into===null));
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);
  await page.locator('[data-chat-input]:visible').waitFor();await page.locator('[data-lab-add]:visible').click();await page.locator('[data-lab-attach-device]').click();await picker.waitFor();
  await choose({name:'chat.txt',mimeType:'text/plain',buffer:Buffer.from('chat attachment')});await selected('chat.txt').waitFor();
  await selected('chat.txt').locator('[data-files-card-preview]').click();await page.locator('[data-attachment-text]').fill('edited chat');
  await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});
  await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const chip=page.locator('[data-chat-pending-file="chat.txt"]');await chip.waitFor();assert.equal((await api('__ai_preview/attachment-events')).length,0);
  await green('[data-chat-send]:visible');await page.screenshot({path:path.join(report,name+'-gui.png')});
  await page.locator('[data-chat-send]:visible').click();await chip.waitFor({state:'hidden'});
  const events=await api('__ai_preview/attachment-events');assert(events.some(event=>event.operation==='chat_put'&&event.name==='chat.txt'&&event.content==='edited chat'));
  await page.waitForFunction(async()=>((await(await fetch('/api/__ai_preview/says',{method:'POST',body:'{}'})).json()).length>0));
 }catch(error){errors.push(String(error.stack??error));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}
 finally{await context.close();}
 return {name,theme,splitLayout,errors};
}
