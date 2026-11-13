import assert from 'node:assert/strict';
import path from 'node:path';
export async function checkAttachmentDraft(browser,{port,report,engine,width=390}){
 const context=await browser.newContext({viewport:{width,height:844},hasTouch:width<500,isMobile:width<500});
 const page=await context.newPage(),errors=[];page.setDefaultTimeout(15000);page.on('pageerror',error=>errors.push(String(error)));
 const name=`attachment-draft-${engine}-${width}`;let dropCapabilities=null;
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 const readyGreen=async selector=>{
  assert.equal(await page.locator(selector).evaluate(button=>{const probe=document.createElement('i');probe.style.backgroundColor='var(--brand-strong)';document.body.append(probe);const green=getComputedStyle(probe).backgroundColor;probe.remove();return !button.disabled&&getComputedStyle(button).backgroundColor===green;}),true,'A ready send uses the shared green');
 };
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3`);
  await page.locator('[data-terminal] .xterm').waitFor();
  if(width>=500){
    await page.locator('[data-term-attachments]:visible').click();
    await page.locator('[data-attachment-picker]:visible').waitFor();
    assert.equal(await page.locator('[data-files-send]:visible').isDisabled(),true,'An empty draft cannot send');
    assert.equal(await page.locator('[data-files-send]:visible').evaluate(button=>{const probe=document.createElement('i');probe.style.backgroundColor='var(--brand-strong)';document.body.append(probe);const green=getComputedStyle(probe).backgroundColor;probe.remove();return getComputedStyle(button).backgroundColor===green;}),false,'An empty draft is neutral');
    await page.locator('[data-files-cancel]:visible').click();await page.locator('[data-attachment-picker]').waitFor({state:'hidden'});
  }
  dropCapabilities=await page.locator('[data-terminal]').evaluate(element=>{const data=new DataTransfer();data.items.add(new File(['original attachment'],'draft.txt',{type:'text/plain'}));const capabilities={types:[...data.types],files:data.files.length,entryMethods:[...data.items].map(item=>typeof item.webkitGetAsEntry)};element.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));return capabilities;});
  const picker=page.locator('[data-attachment-picker]:visible');await picker.waitFor();
  await picker.locator('[data-files-selected="draft.txt"]').waitFor();
  assert.equal((await api('__ai_preview/attachment-events')).length,0,'Drop enters a draft without uploading or pasting');
  assert.match(await picker.locator('[data-send-into-path]').innerText(),/khor-drop/);
  await readyGreen('[data-files-send]:visible');
  await picker.locator('[data-files-selected="draft.txt"] [data-files-card-preview]').click();
  await page.locator('[data-attachment-text]').fill('edited attachment');
  await page.locator('[data-attachment-name]').fill('edited.txt');
  await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});
  await picker.locator('[data-files-selected="edited.txt"]').waitFor();
  await picker.locator('[data-files-add-more]').click();
  await picker.locator('[data-files-input]').setInputFiles({name:'remove.txt',mimeType:'text/plain',buffer:Buffer.from('remove me')});
  await picker.getByRole('button',{name:'移除 remove.txt',exact:true}).click();
  assert.equal(await picker.locator('[data-files-selected]').count(),1);
  await picker.locator('[data-files-add-more]').click();
  const unchanged=Buffer.from('\ufeffpreserve original bytes');
  await picker.locator('[data-files-input]').setInputFiles({name:'keep.txt',mimeType:'text/plain',buffer:unchanged});
  await picker.locator('[data-files-selected="keep.txt"] [data-files-card-preview]').click();
  await page.locator('[data-attachment-text]').waitFor();
  await page.locator('[data-attachment-name]').fill('renamed.txt');
  await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});
  await page.screenshot({path:path.join(report,name+'-terminal.png')});
  const box=await picker.boundingBox();assert(box.x>=0&&box.y>=0&&box.x+box.width<=width+1&&box.y+box.height<=845,'Attachment task fits the viewport');
  await api('__ai_preview/upload-mode',{mode:'fail'});
  await picker.locator('[data-files-send]').click();
  await picker.getByRole('alert').waitFor();
  assert.equal(await picker.locator('[data-files-selected="edited.txt"]').count(),1,'A failure preserves the draft');
  assert.equal((await api('__ai_preview/attachment-events')).length,0,'Failed preparation does not paste a terminal path');
  await api('__ai_preview/upload-mode',{mode:'ok'});
  await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const terminal=await api('__ai_preview/attachment-events');
  assert(terminal.some(event=>event.operation==='term_drop_stage'&&event.name==='edited.txt'&&event.content==='edited attachment'));
  assert(terminal.some(event=>event.operation==='term_drop_stage'&&event.name==='renamed.txt'&&event.bytes===unchanged.length),'Renaming preserves the original bytes, including a UTF-8 BOM');
  assert(terminal.some(event=>event.operation.startsWith('term_drop')&&event.into==='/preview/khor-drop'),'The terminal lands in its target receive directory');
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&machines=3`);
  await page.locator('[data-chat-input]:visible').waitFor();
  await page.locator('[data-lab-add]:visible').click();await page.locator('[data-lab-attach-device]').click();
  await picker.waitFor();
  await picker.locator('[data-files-input]').setInputFiles({name:'chat.txt',mimeType:'text/plain',buffer:Buffer.from('chat attachment')});
  await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const chip=page.locator('[data-chat-pending-file="chat.txt"]');await chip.waitFor();
  assert.equal((await api('__ai_preview/attachment-events')).length,0,'Adding to a GUI draft does not upload before Send');
  await chip.locator('[data-chat-pending-preview]').click();
  await page.locator('[data-attachment-text]').fill('edited chat');await page.locator('[data-attachment-save]').click();await page.locator('[data-attachment-preview]').waitFor({state:'hidden'});
  await readyGreen('[data-chat-send]:visible');
  await page.screenshot({path:path.join(report,name+'-gui.png')});
  await page.locator('[data-chat-send]:visible').click();await chip.waitFor({state:'hidden'});
  const gui=await api('__ai_preview/attachment-events');
  assert(gui.some(event=>event.operation==='chat_put'&&event.name==='chat.txt'&&event.content==='edited chat'),'Only the edited file is uploaded');
  await page.waitForFunction(async()=>((await(await fetch('/api/__ai_preview/says',{method:'POST',body:'{}'})).json()).length>0));
 }catch(error){errors.push(String(error.stack??error));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}
 finally{await context.close();}
 return {name,dropCapabilities,errors};
}
