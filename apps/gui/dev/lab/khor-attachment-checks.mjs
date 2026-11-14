import assert from 'node:assert/strict';
import path from 'node:path';
export async function checkKhorAttachments(browser,{port,report,engine,width=390}){
 const name=`khor-attachments-${engine}-${width}`,mobile=width<500;
 const context=await browser.newContext({viewport:{width,height:844},hasTouch:mobile,isMobile:mobile});
 const page=await context.newPage(),errors=[];let observed=null;page.setDefaultTimeout(12000);page.on('pageerror',error=>errors.push(String(error)));
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=khor&machines=3`);
  const composer=page.locator('[data-chat-composer]:visible');await composer.waitFor();
  await page.locator('[data-lab-add]:visible').click();await page.locator('[data-lab-attach-device]').click();
  const input=page.locator('[data-attachment-picker] [data-files-input],[data-machine-chat] [data-file-input]');
  await input.setInputFiles({name:'khor-note.txt',mimeType:'text/plain',buffer:Buffer.from('Retained Khor attachment')});
  await page.locator('[data-chat-pending-file="khor-note.txt"],[data-files-selected="khor-note.txt"]').first().waitFor();await page.waitForTimeout(250);
  const picker=page.locator('[data-attachment-picker]:visible'),events=await api('__ai_preview/attachment-events');
  observed={sharedPicker:await picker.count(),beforeSend:events};
  assert.equal(observed.sharedPicker,1,'Khor uses the same retained attachment picker as terminal and ACP conversations');
  assert.deepEqual(events,[],'Selecting a Khor attachment never stages, uploads or prompts');
  const selected=picker.locator('[data-files-selected="khor-note.txt"]');
  await selected.locator('[data-files-open]').click();
  const preview=page.locator('[data-attachment-file]:visible');await preview.locator('.cm-content').waitFor();
  assert.match(await preview.innerText(),/Retained Khor attachment/);assert.equal(await preview.locator('[data-file-more]').count(),0);
  assert.equal(await preview.evaluate(()=>!!document.activeElement?.matches('input,textarea,[contenteditable="true"]')),false);
  await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});await selected.waitFor();
  await api('__ai_preview/upload-mode',{mode:'fail'});await picker.locator('[data-files-send]').click();await picker.getByRole('alert').waitFor();
  assert.equal((await api('__ai_preview/attachment-events')).filter(event=>event.operation==='khor_prompt').length,0,'Failed file delivery never submits a Khor prompt');await selected.waitFor();
  await api('__ai_preview/upload-mode',{mode:'ok'});await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const sent=await api('__ai_preview/attachment-events'),prompts=sent.filter(event=>event.operation==='khor_prompt');
  assert.equal(prompts.length,1);assert.match(prompts[0].text,/khor-note.txt/);assert.match(prompts[0].text,/khor-drop/);assert.match(prompts[0].session,/^wide\/agent-/);
  assert(sent.some(event=>event.operation==='khor_put_paths'),'Khor attachment delivery uses the canonical conversation home');
  await page.waitForFunction(()=>document.activeElement?.matches('[data-chat-composer]'));
  assert.equal(await page.locator('[data-chat-pending-file="khor-note.txt"]').count(),0);
  await page.screenshot({path:path.join(report,name+'.png')});
 }catch(error){errors.push(String(error.stack??error));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}finally{await context.close();}
 return {name,observed,errors};
}
