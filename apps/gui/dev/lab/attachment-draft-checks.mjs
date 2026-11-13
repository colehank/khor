import assert from 'node:assert/strict';
import path from 'node:path';
import {readFile} from 'node:fs/promises';
export async function checkAttachmentDraft(browser,{port,report,engine,width=390}){
 const height=width===320?568:844,mobile=width<500,name=`attachment-draft-${engine}-${width}`;
 const context=await browser.newContext({viewport:{width,height},hasTouch:mobile,isMobile:mobile});
 const page=await context.newPage(),errors=[],imagePreviews=[],sizes=[];page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)));
 const environment={platform:mobile?width===412?'android':'ios':'desktop',theme:width===412?'dark':'light',material:'solid',background:'dark',opacity:100,keyboard:0,safeTop:mobile?width===412?24:47:0,safeBottom:mobile?width===412?24:34:0};
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 const picker=page.locator('[data-attachment-picker]:visible'),directory=page.locator('[data-attachment-directory]:visible'),preview=page.locator('[data-attachment-file]:visible');
 const selected=name=>picker.locator(`[data-files-selected="${name}"]`);
 const choose=file=>picker.locator('[data-files-input]').setInputFiles(file);
 const grid=()=>page.locator('[data-terminal] .xterm-screen').evaluate(node=>({width:node.style.width,height:node.style.height}));
 const green=async()=>assert(await picker.locator('[data-files-send]').evaluate(button=>{const p=document.createElement('i');p.style.backgroundColor='var(--brand-strong)';document.body.append(p);const expected=getComputedStyle(p).backgroundColor;p.remove();return !button.disabled&&getComputedStyle(button).backgroundColor===expected;}));
 let originalGrid,originalSizes;
 const unchanged=async label=>{assert.deepEqual(await grid(),originalGrid,`${label}: terminal cell grid unchanged`);const current=await api('__ai_preview/terminal-sizes');sizes.push({label,count:current.length});assert.equal(current.length,originalSizes,`${label}: no PTY resize`);};
 const image=async label=>{
  const media=JSON.parse(await readFile(new URL('./data/media.generated.json',import.meta.url),'utf8'));
  await choose({name:'photo.png',mimeType:'image/png',buffer:Buffer.from(media.image,'base64')});assert.equal(await selected('photo.png').locator('[data-files-open] [data-icon="open"]').count(),1);await selected('photo.png').locator('[data-files-open]').click();await preview.waitFor();
  await page.waitForFunction(()=>{const img=document.querySelector('[data-attachment-file] [data-file-page] img');return img?.complete&&img.naturalWidth>0;});
  assert.equal(await preview.locator('[data-file-more]').count(),0,'Workspace file page has no More action in attachments');
  const focus=await preview.evaluate(root=>({editable:!!document.activeElement?.matches('input,textarea,[contenteditable="true"]'),inside:root.contains(document.activeElement)}));assert.equal(focus.editable,false);imagePreviews.push({label,focus});
  await page.screenshot({path:path.join(report,name+'-'+label+'-image.png')});
  await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});await selected('photo.png').locator('[data-files-select]').click();await selected('photo.png').waitFor({state:'hidden'});
 };
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);await page.locator('[data-terminal] .xterm').waitFor();await page.waitForTimeout(500);
  originalGrid=await grid();originalSizes=(await api('__ai_preview/terminal-sizes')).length;
  await page.locator('[data-terminal]').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['original attachment'],'draft.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});
  await picker.waitFor();await selected('draft.txt').waitFor();await unchanged('open fixed panel');
  assert.equal(await picker.evaluate(node=>getComputedStyle(node).position),'fixed');
  await page.waitForFunction(()=>document.querySelector('#root').getBoundingClientRect().bottom<=document.querySelector('[data-attachment-picker]').getBoundingClientRect().top+1);
  await unchanged('chrome above the keyboard panel');assert.equal(await picker.locator('[data-files-actions]').innerText(),'取消\n发送 (1)');
  assert.equal(await picker.locator('[data-files-input]').getAttribute('accept'),null);assert.match(await picker.locator('[data-send-into-path]').innerText(),/khor-drop/);await green();assert.equal((await api('__ai_preview/attachment-events')).length,0);
  await image('terminal');await unchanged('local file page and return');
  await picker.locator('[data-files-folder-input]').evaluate(input=>{const file=new File(['nested original bytes'],'inside.txt',{type:'text/plain'});Object.defineProperty(file,'webkitRelativePath',{value:'local-folder/nested/inside.txt'});const data=new DataTransfer();data.items.add(file);input.files=data.files;input.dispatchEvent(new Event('change',{bubbles:true}));});
  await selected('local-folder').locator('button').first().click();await directory.waitFor();await directory.locator('[data-files-name="nested"] button').first().click();await directory.locator('[data-files-name="inside.txt"]').waitFor();assert.equal(await directory.locator('[data-files-select="inside.txt"]').isDisabled(),true,'Selected local folder includes its descendants');
  await directory.locator('[data-files-name="inside.txt"] [data-files-open]').click();await preview.locator('.cm-editor').waitFor();assert.match(await preview.innerText(),/nested original bytes/);await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});assert.match(await directory.locator('[data-files-search]').inputValue(),/nested/);await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected('local-folder').locator('[data-files-select]').click();await unchanged('local folder navigation');

  await selected('draft.txt').locator('[data-files-open]').click();await preview.locator('[data-code-editor],.cm-editor').first().waitFor();await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});
  await picker.locator('[data-files-machine][data-files-session]').click();await directory.waitFor();await directory.locator('[data-files-entry]').first().waitFor();
  assert.equal(await directory.locator('[data-files-search]').evaluate(node=>node===document.activeElement),false,'No automatic path/search keyboard');
  if(await directory.locator('[data-files-entry="file"]').count()===0)await directory.locator('[data-files-name="project"] button').first().click();
  await directory.locator('[data-files-entry="file"]').first().waitFor();
  const row=directory.locator('[data-files-entry="file"]').first(),remoteName=await row.getAttribute('data-files-name');
  await row.locator('button').first().click();assert.equal(await row.locator('[data-files-select]').getAttribute('aria-pressed'),'true');assert.equal(await preview.count(),0,'File row selects without opening');
  await row.locator('[data-files-open]').click();await preview.locator('[data-file-page]').waitFor();await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});await directory.waitFor();
  await unchanged('remote directory/file navigation');await page.screenshot({path:path.join(report,name+'-directory.png')});
  const field=directory.locator('[data-files-search]');const previous=await field.inputValue();await field.fill(previous+remoteName.slice(0,3));await page.waitForTimeout(250);assert(await directory.locator(`[data-files-name="${remoteName}"]`).count());await field.fill(previous);
  await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected(remoteName).waitFor();
  await selected(remoteName).locator('button').first().click();await selected(remoteName).waitFor({state:'hidden'});
  await picker.locator('[data-files-machine][data-files-session]').click();await directory.locator('[data-files-name="project"]').waitFor();await directory.locator('[data-files-select="project"]').click();assert.equal(await directory.locator('[data-files-select="project"]').getAttribute('aria-pressed'),'true');
  await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected('project').locator('button').first().click();await directory.waitFor();await directory.locator('[data-files-entry]').first().waitFor();assert.match(await directory.locator('[data-files-search]').inputValue(),/project/);await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});
  await selected('project').locator('[data-files-select]').click();await selected('project').waitFor({state:'hidden'});
  await choose(Array.from({length:12},(_,i)=>({name:`long-${i}.txt`,mimeType:'text/plain',buffer:Buffer.from('draft')})));
  const layout=await picker.evaluate(root=>{const source=root.querySelector('[data-files-source]'),list=root.querySelector('[data-files-selection-scroll]');const before=source.scrollTop;list.scrollTop=list.scrollHeight;const rect=root.getBoundingClientRect();return {left:rect.left,right:rect.right,top:rect.top,bottom:rect.bottom,scrolled:list.scrollTop,independent:source.scrollTop===before};});assert(layout.left>=0&&layout.right<=width+1&&layout.top>=0&&layout.bottom<=height+1);assert(layout.scrolled>0&&layout.independent);
  for(let i=0;i<12;i++)await selected(`long-${i}.txt`).locator('[data-files-select]').click();
  await unchanged('selection and navigation');await page.screenshot({path:path.join(report,name+'-panel.png')});
  await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});await page.locator('[data-term-attachment-resume]:visible').click();await picker.waitFor();await selected('draft.txt').waitFor();await unchanged('cancel and reopen');
  await api('__ai_preview/upload-mode',{mode:'fail'});await picker.locator('[data-files-send]').click();await picker.getByRole('alert').waitFor();await selected('draft.txt').waitFor();await api('__ai_preview/upload-mode',{mode:'ok'});await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  const events=await api('__ai_preview/attachment-events');assert(events.some(e=>e.operation==='term_drop_stage'&&e.name==='draft.txt'&&e.content==='original attachment'));assert(events.some(e=>(e.operation==='term_drop'||e.operation==='term_drop_items')&&e.into===null));
  if(mobile){
   const keyboardEnvironment={...environment,keyboard:232};await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(keyboardEnvironment))}`);await page.locator('[data-terminal] .xterm').waitFor();await page.waitForTimeout(500);originalGrid=await grid();originalSizes=(await api('__ai_preview/terminal-sizes')).length;
   await page.locator('[data-terminal]').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['keyboard draft'],'keyboard.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});await picker.waitFor();assert.equal(Math.round((await picker.boundingBox()).height),232);await image('keyboard');await unchanged('replacement for simulated keyboard');await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});
  }
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);await page.locator('[data-chat-input]:visible').waitFor();await page.locator('[data-lab-add]:visible').click();await page.locator('[data-lab-attach-device]').click();await picker.waitFor();
  await choose({name:'chat.txt',mimeType:'text/plain',buffer:Buffer.from('chat attachment')});await selected('chat.txt').waitFor();await image('gui');
  assert.equal((await api('__ai_preview/attachment-events')).length,0);await green();await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  await page.waitForFunction(async()=>((await(await fetch('/api/__ai_preview/says',{method:'POST',body:'{}'})).json()).length>0));assert((await api('__ai_preview/attachment-events')).some(e=>e.operation==='chat_put'&&e.name==='chat.txt'&&e.content==='chat attachment'));await page.screenshot({path:path.join(report,name+'-gui.png')});
 }catch(e){errors.push(String(e.stack??e));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}finally{await context.close();}
 return {name,imagePreviews,sizes,errors};
}
