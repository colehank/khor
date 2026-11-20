import assert from 'node:assert/strict';
import path from 'node:path';
import {readFile} from 'node:fs/promises';
export async function checkAttachmentDraft(browser,{port,report,engine,width=390}){
 const height=width===320?568:844,mobile=width<500,name=`attachment-draft-${engine}-${width}`;
 const context=await browser.newContext({viewport:{width,height},hasTouch:mobile,isMobile:mobile});
 const page=await context.newPage(),errors=[],imagePreviews=[],sizes=[],directoryLayouts=[],panelWidths=[],previewLayouts=[];page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)));
 const environment={platform:mobile?width===412?'android':'ios':'desktop',theme:width===412||!mobile?'dark':'light',material:mobile?'solid':'frost',background:'dark',opacity:mobile?100:20,keyboard:0,safeTop:mobile?width===412?24:47:0,safeBottom:mobile?width===412?24:34:0};
 const api=(command,body={})=>page.evaluate(async({command,body})=>(await fetch('/api/'+command,{method:'POST',body:JSON.stringify(body)})).json(),{command,body});
 const picker=page.locator('[data-attachment-picker]:visible'),directory=page.locator('[data-attachment-directory]:visible'),preview=page.locator('[data-attachment-file]:visible');
 const selected=name=>picker.locator(`[data-files-selected="${name}"]`);
 const choose=file=>picker.locator('[data-files-input]').setInputFiles(file);
 const machinesMenu=page.locator('[data-files-machines-menu]:visible');
 const openMachines=async()=>{const more=picker.locator('[data-files-machines]:visible');if(!await more.count())return false;await more.click();await machinesMenu.waitFor();const count=Number(await more.getAttribute('data-files-machine-count'));assert(count>0,'Only overflowing machines use More');assert.equal(await machinesMenu.locator('[data-files-machine]').count(),count,'Overflow menu contains exactly the hidden machines');return true;};
 const pickMachine=async name=>{const chip=picker.locator('[data-files-machine-chip="'+name+'"]:visible');if(await chip.count())await chip.click();else{assert(await openMachines(),'A hidden machine remains available in More');await machinesMenu.locator('[data-files-machine="'+name+'"]').click();}};
 const sourceMachine=async()=>{const sources=(await api('devices')).filter(d=>d.hosts&&!d.me);assert(sources.length);await pickMachine(sources[0].name);};

 const grid=()=>page.locator('[data-terminal] .xterm-screen').evaluate(node=>({width:node.style.width,height:node.style.height}));
 const measurePanel=async(label,selector,view=page)=>{
  await view.waitForFunction(selector=>{
   const owner=document.querySelector(selector)?.getBoundingClientRect();
   const panel=document.querySelector('[data-attachment-picker]')?.getBoundingClientRect();
   return owner&&panel&&Math.abs(owner.x-panel.x)<=1&&Math.abs(owner.width-panel.width)<=1;
  },selector.replace(':visible',''));
  const owner=await view.locator(selector).boundingBox(),panel=await view.locator('[data-attachment-picker]:visible').boundingBox();panelWidths.push({label,owner,panel});
 };
 const assertPanelWidths=()=>{const issues=panelWidths.filter(({owner,panel})=>Math.abs(owner.x-panel.x)>1||Math.abs(owner.width-panel.width)>1);assert.deepEqual(issues,[],'Attachment panel follows its session width instead of a fixed dialog width');};
 const green=async()=>assert(await picker.locator('[data-files-send]').evaluate(button=>{const p=document.createElement('i');p.style.backgroundColor='var(--brand-strong)';document.body.append(p);const expected=getComputedStyle(p).backgroundColor;p.remove();const face=getComputedStyle(button,'::before');return !button.disabled&&(face.content!=='none'?face.backgroundColor:getComputedStyle(button).backgroundColor)===expected;}));
 const terminal=page.locator('[data-terminal]');
 let originalGrid,originalSizes;
 const unchanged=async label=>{assert.deepEqual(await grid(),originalGrid,`${label}: terminal cell grid unchanged`);const current=await api('__ai_preview/terminal-sizes');sizes.push({label,count:current.length});assert.equal(current.length,originalSizes,`${label}: no PTY resize`);};
 const image=async label=>{
  const media=JSON.parse(await readFile(new URL('./data/media.generated.json',import.meta.url),'utf8'));
  await choose({name:'photo.png',mimeType:'image/png',buffer:Buffer.from(media.image,'base64')});assert.equal(await selected('photo.png').locator('[data-files-open] [data-icon="preview"]').count(),1);await selected('photo.png').locator('[data-files-open]').click();await preview.waitFor();
  assert(await preview.evaluate(node=>getComputedStyle(node).backgroundColor!=='rgba(0, 0, 0, 0)'),'Attachment previews retain an opaque owning surface');
  await page.waitForFunction(()=>{const img=document.querySelector('[data-attachment-file] [data-file-page] img');return img?.complete&&img.naturalWidth>0;});
  assert.equal(await preview.locator('[data-file-more]').count(),0,'Workspace file page has no More action in attachments');
  await page.waitForFunction(()=>{const root=document.querySelector('[data-attachment-file][data-state="open"]');return root&&root.getAnimations().every(animation=>animation.playState==='finished');});
  const layout=await preview.evaluate(root=>{const box=node=>{const r=node.getBoundingClientRect();return {left:r.left,right:r.right,width:r.width};};const sheet=box(root),style=getComputedStyle(root),left=parseFloat(style.borderLeftWidth),right=parseFloat(style.borderRightWidth);return {sheet,frame:{left:sheet.left+left,width:sheet.width-left-right},file:box(root.querySelector('[data-file-page]')),back:box(root.querySelector('[data-file-back]'))};});previewLayouts.push({label,...layout});
  const focus=await preview.evaluate(root=>({editable:!!document.activeElement?.matches('input,textarea,[contenteditable="true"]'),inside:root.contains(document.activeElement)}));assert.equal(focus.editable,false);imagePreviews.push({label,focus});
  await page.screenshot({path:path.join(report,name+'-'+label+'-image.png')});
  await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});await selected('photo.png').locator('[data-files-select]').click();await selected('photo.png').waitFor({state:'hidden'});
 };
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);await page.locator('[data-terminal] .xterm').waitFor();await page.waitForTimeout(500);
  originalGrid=await grid();originalSizes=(await api('__ai_preview/terminal-sizes')).length;
  const originalTerminal=await terminal.boundingBox();
  await page.locator('[data-terminal]').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['original attachment'],'draft.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});
  await picker.waitFor();await selected('draft.txt').waitFor();await unchanged('open fixed panel');
  if(mobile){
   const action=await picker.locator('[data-files-send]').evaluate(button=>{const box=button.getBoundingClientRect(),face=getComputedStyle(button,'::before'),root=getComputedStyle(document.documentElement);return {hit:box.height,width:box.width,face:parseFloat(face.height),touch:parseFloat(root.getPropertyValue('--touch')),compact:parseFloat(root.getPropertyValue('--ctl-lg')),background:getComputedStyle(button).backgroundColor};});
   assert(action.hit>=action.touch&&action.width>=action.touch,'The compact send face retains the full mobile touch target');assert.equal(action.face,action.compact,'Send paints the shared compact task-button height');assert.equal(action.background,'rgba(0, 0, 0, 0)');
  }
  assert.equal(await picker.locator('[data-files-destination]').getByRole('heading',{name:'发送到',exact:true}).count(),1);
  const selectionPaint=await selected('draft.txt').locator('[data-files-select] > span').evaluate(mark=>{
   const probe=document.createElement('span');probe.style.backgroundColor='var(--brand-strong)';probe.style.color='var(--on-brand)';mark.append(probe);
   const expected=getComputedStyle(probe),actual=getComputedStyle(mark),paint={actual:{background:actual.backgroundColor,color:actual.color,border:actual.borderTopColor},expected:{background:expected.backgroundColor,color:expected.color,border:expected.backgroundColor}};probe.remove();return paint;
  });
  assert.deepEqual(selectionPaint.actual,selectionPaint.expected,'Selected attachment mark uses the settings active green and contrasting check');
  if(!mobile)assert.equal(await page.locator('[data-term-attachments]').count(),0,'Desktop uses direct drops without a redundant header attachment button');
  const surface=await picker.evaluate(node=>{const color=getComputedStyle(node).backgroundColor;const probe=document.createElement('span');probe.style.background='var(--canvas-solid)';node.append(probe);const expected=getComputedStyle(probe).backgroundColor;probe.remove();return {color,expected};});
  assert.equal(surface.color,surface.expected,'Attachment selection panel must have an opaque surface even in a frosted window');
  assert.equal(await picker.locator('[data-files-source]').getByRole('heading',{name:'选择文件',exact:true}).count(),1,'Sources have their own visible section title');
  assert.equal(await picker.locator('[data-files-selection]').getByRole('heading',{name:/已选文件/}).count(),1,'Selected files have their own visible section title');
  assert.equal(await picker.locator('[data-files-source] [data-files-selected]').count(),0,'Selected files never belong to a source-machine group');
  assert.equal(await selected('draft.txt').locator('[data-files-size]').innerText(),'19 B','Browser file size is visible in the selected row');
  assert.equal(await picker.evaluate(node=>getComputedStyle(node).position),'fixed');
  await measurePanel('initial-'+width,'[data-terminal]');
  assertPanelWidths();
  if(!mobile){
   for(const next of [760,width]){await page.setViewportSize({width:next,height});await measurePanel('resize-'+next,'[data-terminal]');}assertPanelWidths();
   const probe=await context.newPage();probe.on('pageerror',e=>errors.push(String(e)));
   try{await probe.setViewportSize({width:680,height});await probe.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);await probe.locator('[data-terminal] .xterm').waitFor();await probe.waitForTimeout(500);await probe.locator('[data-terminal]').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['width probe'],'width.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});await probe.locator('[data-attachment-picker]:visible').waitFor();await measurePanel('narrow-680','[data-terminal]',probe);assertPanelWidths();}finally{await probe.close();}
  }

  if(mobile){
   const panel=await picker.boundingBox();
   assert.deepEqual(await terminal.boundingBox(),originalTerminal,'Opening attachments preserves the terminal viewport');
   assert(Math.abs(panel.y+panel.height-originalTerminal.y-originalTerminal.height)<=1,'Attachment overlay meets the bottom edge of the terminal');
   assert(panel.y>=originalTerminal.y,'The attachment overlay stays within the terminal viewport');
  }
  await unchanged('terminal behind the attachment overlay');assert.equal(await picker.locator('[data-files-actions]').innerText(),'取消\n发送 (1)');
  const regions=await picker.evaluate(root=>{
   const box=selector=>{const r=root.querySelector(selector).getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom};};
   return {source:box('[data-files-source]'),selected:box('[data-files-selection]'),destination:box('[data-files-destination]'),local:box('[data-files-local]'),machines:box('[data-files-machine-row]')};
  });
  assert(regions.local.bottom<regions.machines.top,'Local files/folders and network machines occupy separate spaced rows');
  assert(regions.source.bottom<regions.selected.top&&regions.selected.bottom<regions.destination.top,'Sources, selection and destination have ordered separated sections');
  assert.equal(regions.source.left,regions.selected.left);assert.equal(regions.selected.left,regions.destination.left,'Section boundaries align');
  for(const button of await picker.locator('[data-files-pick]').all())assert(await button.evaluate(node=>{const outer=node.getBoundingClientRect();return [...node.children].every(child=>{const r=child.getBoundingClientRect();return r.top>=outer.top&&r.bottom<=outer.bottom;});}),'Source button contents fit without overlapping adjacent elements');
  const chips=picker.locator('[data-files-machine-chip]:visible');assert(await chips.count()>0,'At least one source machine shows its avatar and name inline');
  assert(await chips.first().locator('svg').count()>0);assert((await chips.first().innerText()).trim());
  const machineRow=picker.locator('[data-files-machine-row]');
  await machineRow.evaluate(node=>node.style.width='80px');
  await picker.locator('[data-files-machines]:visible').waitFor();assert(await openMachines(),'Narrow source space exposes the overflow selector');
  assert.equal(await machinesMenu.locator('[data-files-machine]').count(),Number(await machineRow.getAttribute('data-files-source-count')),'Every machine remains selectable when all chips overflow');
  await picker.locator('[data-files-source] h3').click();await machinesMenu.waitFor({state:'hidden'});await machineRow.evaluate(node=>node.style.removeProperty('width'));await chips.first().waitFor();
  assert.equal(await picker.locator('[data-files-destination]').getByRole('heading',{name:'发送到',exact:true}).count(),1);
  await selected('draft.txt').locator('[data-files-select]').click();await selected('draft.txt').waitFor({state:'hidden'});assert.equal(await picker.locator('[data-files-selection]').count(),0,'Empty selection hides its entire section');
  await choose({name:'draft.txt',mimeType:'text/plain',buffer:Buffer.from('original attachment')});await selected('draft.txt').waitFor();
  await page.screenshot({path:path.join(report,name+'-layout.png')});
  assert.match(await picker.locator('[data-send-to-machine]').getAttribute('aria-label'),/发送到.*保存文件夹/);assert.equal(await picker.locator('[data-send-into-label]').count(),0);assert.equal((await picker.locator('[data-send-to-machine]').innerText()).replace(/\s/g,''),'：'+(await picker.locator('[data-send-into-path]').innerText()).replace(/\s/g,''),'Destination displays only avatar, colon and path');assert.equal((await picker.locator('[data-send-into-change]').innerText()).trim(),'更改');
  const other=(await api('devices')).filter(d=>d.hosts&&!d.me).at(-1);assert(other);await pickMachine(other.name);await directory.waitFor();assert.equal(await directory.locator('[data-files-directory-avatar]').getAttribute('aria-label'),other.shown,'Choosing another machine opens that machine');await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await unchanged('other machine choice');
  assert.equal(await picker.locator('[data-files-input]').getAttribute('accept'),null);assert.match(await picker.locator('[data-send-into-path]').innerText(),/khor-drop/);await green();assert.equal((await api('__ai_preview/attachment-events')).length,0);
  // Native/internal drops only carry paths. Read metadata from the source,
  // without staging/uploading files or borrowing the destination's facts.
  const nativeSources=[null,(await api('devices')).find(d=>d.hosts&&!d.me)?.name].filter(source=>source!==undefined);
  for(const from of nativeSources){
   let listing=await api('ls',{machine:from??'',path:''});
   if(listing.entries.some(e=>e.name==='project'&&e.dir))listing=await api('ls',{machine:from??'',path:listing.path.replace(/\/$/,'')+'/project'});
   const file=listing.entries.find(e=>!e.dir);assert(file,'Native-drop fixture has a source file');const filePath=listing.path.replace(/\/$/,'')+'/'+file.name;
   const expected=await api('stat_text',{machine:from??'',path:filePath});
   await page.evaluate(()=>{window.__attachmentSizeReads=[];const fetch=window.fetch;window.fetch=(input,init)=>{if(String(input).endsWith('/api/stat_text'))window.__attachmentSizeReads.push(JSON.parse(init.body));return fetch(input,init);};});
   await page.locator('[data-terminal]').evaluate((node,{from,filePath})=>{const data=new DataTransfer();data.setData('application/x-khor-path',JSON.stringify({machine:from,path:filePath}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));},{from,filePath});
   const native=selected(file.name);await native.waitFor();
   const size=expected.total<1024?expected.total+' B':null;if(size)await native.locator('[data-files-size]').filter({hasText:size}).waitFor();
   else await native.locator('[data-files-size]').waitFor();
   const reads=await page.evaluate(()=>window.__attachmentSizeReads);assert(reads.some(read=>read.machine===(from??'')&&read.path===filePath),'Unknown file sizes are read from the source machine');
   await native.locator('[data-files-select]').click();await native.waitFor({state:'hidden'});
  }
  assert.equal((await api('__ai_preview/attachment-events')).length,0,'Metadata lookup never uploads or sends a file');
  await image('terminal');await unchanged('local file page and return');
  await picker.locator('[data-files-folder-input]').evaluate(input=>{const file=new File(['nested original bytes'],'inside.txt',{type:'text/plain'});Object.defineProperty(file,'webkitRelativePath',{value:'local-folder/nested/inside.txt'});const data=new DataTransfer();data.items.add(file);input.files=data.files;input.dispatchEvent(new Event('change',{bubbles:true}));});
  await selected('local-folder').locator('button').first().click();await directory.waitFor();await directory.locator('[data-files-name="nested"] button').first().click();await directory.locator('[data-files-name="inside.txt"]').waitFor();assert.equal(await directory.locator('[data-files-select="inside.txt"]').isDisabled(),true,'Selected local folder includes its descendants');
  await directory.locator('[data-files-name="inside.txt"] [data-files-open]').click();await preview.locator('.cm-editor').waitFor();assert.match(await preview.innerText(),/nested original bytes/);await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});assert.match(await directory.locator('[data-files-search] input').inputValue(),/nested/);await directory.locator('[data-up]').click();await directory.locator('[data-files-name="nested"]').waitFor();await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected('local-folder').locator('[data-files-select]').click();await unchanged('local folder navigation');

  await selected('draft.txt').locator('[data-files-open]').click();await preview.locator('[data-code-editor],.cm-editor').first().waitFor();await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});
  await sourceMachine();await directory.waitFor();await directory.locator('[data-files-entry]').first().waitFor();
  assert(await directory.evaluate(node=>getComputedStyle(node).backgroundColor!=='rgba(0, 0, 0, 0)'),'Source directory candidates retain an opaque owning surface');
  assert.equal(await directory.locator('[data-files-search] input').evaluate(node=>node===document.activeElement),false,'No automatic path/search keyboard');
  // Measure the settled sheet, rather than its entrance below the viewport.
  await page.waitForFunction(()=>{const root=document.querySelector('[data-attachment-directory][data-state="open"]');if(!root)return false;const sheet=root.getBoundingClientRect(),back=root.querySelector('[data-files-directory-back]')?.getBoundingClientRect();return back&&back.top>=sheet.top&&back.bottom<=Math.min(sheet.bottom,window.innerHeight)+1;});
  const directoryBounds=await directory.evaluate(root=>{
   const rect=node=>{const r=node.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height};};
   const back=root.querySelector('[data-files-directory-back]'),pill=root.querySelector('[data-files-search]'),avatar=root.querySelector('[data-files-directory-avatar]');
   const probe=document.createElement('span');probe.style.cssText='position:absolute;display:block;width:var(--glass-item-h);height:0';root.append(probe);const seat=rect(probe).width;probe.remove();
   const session=document.querySelector('[data-terminal]')?.closest('[data-session-screen]')?.querySelector('[data-session-deck]');
   return {sheet:rect(root),headers:root.querySelectorAll('[data-files-directory-head]').length,back:rect(back),pill:rect(pill),avatar:rect(avatar),machine:avatar.getAttribute('aria-label'),list:rect(root.querySelector('[data-files-list]')),glass:pill.hasAttribute('data-glass-capsule')||pill.classList.contains('glass'),seat,viewport:window.innerHeight,reference:session?{back:rect(session.querySelector('[data-back]')),pill:rect(session.querySelector('[data-deck-pill]'))}:null};
  });directoryLayouts.push(directoryBounds);
  assert.equal(directoryBounds.headers,0,'The directory has no redundant machine heading');assert(directoryBounds.machine?.trim(),'The machine is identified by its avatar in the path capsule');
  assert(directoryBounds.back.top>=directoryBounds.sheet.top&&directoryBounds.back.bottom<=Math.min(directoryBounds.sheet.bottom,directoryBounds.viewport)+1,'Directory return action remains in view on short screens: '+JSON.stringify(directoryBounds));
  assert(directoryBounds.list.height>=44&&directoryBounds.list.bottom<=Math.min(directoryBounds.back.top,directoryBounds.pill.top),'The file list scrolls above both fixed navigation capsules');
  assert(directoryBounds.glass&&directoryBounds.pill.left>=directoryBounds.back.right&&Math.abs(directoryBounds.pill.top-directoryBounds.back.top)<=1&&Math.abs(directoryBounds.pill.height-directoryBounds.back.height)<=1,'Back and path capsules share one bottom navigation row');
  assert(directoryBounds.avatar.left>=directoryBounds.pill.left&&directoryBounds.avatar.right<=directoryBounds.pill.right&&Math.abs((directoryBounds.avatar.top+directoryBounds.avatar.bottom)/2-(directoryBounds.pill.top+directoryBounds.pill.bottom)/2)<=1,'Machine avatar sits inside and vertically centered in the path capsule');
  assert(directoryBounds.pill.right<=directoryBounds.sheet.right+1,'The path capsule stays inside the sheet');
  if(mobile){assert(directoryBounds.reference,'The current session exposes its navigation alignment reference');assert(Math.abs(directoryBounds.reference.back.left-directoryBounds.back.left)<=1&&Math.abs(directoryBounds.reference.pill.left-directoryBounds.pill.left)<=1&&Math.abs(directoryBounds.reference.pill.height-directoryBounds.pill.height)<=1,'Directory navigation aligns with the session navigation');}


  if(await directory.locator('[data-files-entry="file"]').count()===0)await directory.locator('[data-files-name="project"] button').first().click();
  await directory.locator('[data-files-entry="file"]').first().waitFor();
  const kindBoxes=await directory.locator('[data-files-kind]').evaluateAll(nodes=>nodes.map(node=>{const icon=node.querySelector('svg'),a=node.getBoundingClientRect(),b=icon.getBoundingClientRect();return {slot:a.width,width:b.width,height:b.height};}));for(const box of kindBoxes){assert(box.width<=box.slot+1&&box.height<=box.slot+1,'File and folder marks stay inside their common icon slot');}

  const row=directory.locator('[data-files-entry="file"]').first(),remoteName=await row.getAttribute('data-files-name');
  await row.locator('button').first().click();assert.equal(await row.locator('[data-files-select]').getAttribute('aria-pressed'),'true');assert.equal(await preview.count(),0,'File row selects without opening');
  await row.locator('[data-files-open]').click();await preview.locator('[data-file-page]').waitFor();await preview.locator('[data-file-back]').click();await preview.waitFor({state:'hidden'});await directory.waitFor();
  await unchanged('remote directory/file navigation');await page.screenshot({path:path.join(report,name+'-directory.png')});
  const field=directory.locator('[data-files-search] input');const previous=await field.inputValue();await field.fill(previous+remoteName.slice(0,3));await page.waitForTimeout(250);assert(await directory.locator(`[data-files-name="${remoteName}"]`).count());await field.fill(previous);
  await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected(remoteName).waitFor();
  await selected(remoteName).locator('button').first().click();await selected(remoteName).waitFor({state:'hidden'});
  await sourceMachine();await directory.locator('[data-files-name="project"]').waitFor();await directory.locator('[data-files-select="project"]').click();assert.equal(await directory.locator('[data-files-select="project"]').getAttribute('aria-pressed'),'true');
  await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});await selected('project').locator('button').first().click();await directory.waitFor();await directory.locator('[data-files-entry]').first().waitFor();assert.match(await directory.locator('[data-files-search] input').inputValue(),/project/);await directory.locator('[data-files-directory-back]').click();await directory.waitFor({state:'hidden'});
  await selected('project').locator('[data-files-select]').click();await selected('project').waitFor({state:'hidden'});
  await choose(Array.from({length:12},(_,i)=>({name:`long-${i}.txt`,mimeType:'text/plain',buffer:Buffer.from('draft')})));
  const layout=await picker.evaluate(root=>{const source=root.querySelector('[data-files-source]'),list=root.querySelector('[data-files-selection-scroll]');const before=source.scrollTop;list.scrollTop=list.scrollHeight;const rect=root.getBoundingClientRect();return {left:rect.left,right:rect.right,top:rect.top,bottom:rect.bottom,scrolled:list.scrollTop,independent:source.scrollTop===before};});assert(layout.left>=0&&layout.right<=width+1&&layout.top>=0&&layout.bottom<=height+1);assert(layout.scrolled>0&&layout.independent);
  for(let i=0;i<12;i++)await selected(`long-${i}.txt`).locator('[data-files-select]').click();
  await unchanged('selection and navigation');await page.screenshot({path:path.join(report,name+'-panel.png')});
  await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});await page.locator('[data-term-attachment-resume]:visible').click();await picker.waitFor();await selected('draft.txt').waitFor();await unchanged('cancel and reopen');
  await api('__ai_preview/upload-mode',{mode:'fail'});await picker.locator('[data-files-send]').click();await picker.getByRole('alert').waitFor();await selected('draft.txt').waitFor();await api('__ai_preview/upload-mode',{mode:'ok'});const retrySend=picker.locator('[data-files-send]'),retryBox=await retrySend.boundingBox();await retrySend.click(mobile?{position:{x:retryBox.width/2,y:2}}:{});await picker.waitFor({state:'hidden'});
  const events=await api('__ai_preview/attachment-events');assert(events.some(e=>e.operation==='term_drop_stage'&&e.name==='draft.txt'&&e.content==='original attachment'));assert(events.some(e=>(e.operation==='term_drop'||e.operation==='term_drop_items')&&e.into===null));
  if(mobile){
   const keyboardEnvironment={...environment,keyboard:232};await page.goto(`http://127.0.0.1:${port}/app/?scene=terminal&machines=3&environment=${encodeURIComponent(JSON.stringify(keyboardEnvironment))}`);await page.locator('[data-terminal] .xterm').waitFor();await page.waitForTimeout(500);originalGrid=await grid();originalSizes=(await api('__ai_preview/terminal-sizes')).length;
   await page.locator('[data-terminal]').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['keyboard draft'],'keyboard.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});await picker.waitFor();const band=await picker.boundingBox();assert.equal(Math.round(band.height),232,'The picker fills the reserved keyboard band');assert.equal(Math.round(band.y+band.height),height,'The picker ends at the window bottom');const shortLayout=await picker.evaluate(node=>({height:node.getBoundingClientRect().height,regions:Object.fromEntries(['[data-files-actions]','[data-files-source]','[data-files-selection]','[data-files-selection-scroll]','[data-attachment-destination]'].map(selector=>[selector,node.querySelector(selector).getBoundingClientRect().height]))}));assert(shortLayout.regions['[data-files-selection-scroll]']>=44,'The shorter keyboard panel still exposes a full selected-file touch row: '+JSON.stringify(shortLayout));await image('keyboard');await unchanged('replacement for simulated keyboard');await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});
  }
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&machines=3&environment=${encodeURIComponent(JSON.stringify(environment))}`);await page.locator('[data-chat-input]:visible').waitFor();await page.locator('[data-lab-add]:visible').click();await picker.waitFor();assert.equal(await page.locator('[data-lab-attachments]').count(),0,'GUI plus opens the shared picker directly');await measurePanel('gui-'+width,'[data-chat-searchable]:visible');assertPanelWidths();
  if(!mobile){
   await picker.locator('[data-files-cancel]').click();await picker.waitFor({state:'hidden'});
   const input=page.locator('[data-chat-input]:visible');await input.fill('Message with a dropped file');
   await page.locator('[data-chat-searchable]:visible').evaluate(node=>{const data=new DataTransfer();data.items.add(new File(['chat attachment'],'chat.txt',{type:'text/plain'}));node.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}));});
   await picker.waitFor();assert.equal(await input.inputValue(),'Message with a dropped file','Direct drops preserve the current message');
  }else await choose({name:'chat.txt',mimeType:'text/plain',buffer:Buffer.from('chat attachment')});
  await selected('chat.txt').waitFor();await image('gui');
  assert.equal((await api('__ai_preview/attachment-events')).length,0);await green();await picker.locator('[data-files-send]').click();await picker.waitFor({state:'hidden'});
  await page.waitForFunction(async()=>((await(await fetch('/api/__ai_preview/says',{method:'POST',body:'{}'})).json()).length>0));assert((await api('__ai_preview/attachment-events')).some(e=>e.operation==='chat_put'&&e.name==='chat.txt'&&e.content==='chat attachment'));await page.screenshot({path:path.join(report,name+'-gui.png')});
  const layoutIssues=[];
  for(const sample of previewLayouts)if(Math.abs(sample.file.left-sample.frame.left)>1||Math.abs(sample.file.width-sample.frame.width)>1)layoutIssues.push({issue:'File preview must use the complete content-sheet width',...sample});
  for(const sample of directoryLayouts)if(Math.abs(sample.avatar.width-sample.seat)>1)layoutIssues.push({issue:'Machine avatar must use the same face seat as the session pill',avatar:sample.avatar,seat:sample.seat});
  assert.deepEqual(layoutIssues,[],'Attachment pages share workspace width and the session face seat');
 }catch(e){errors.push(String(e.stack??e));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}finally{await context.close();}
 return {name,imagePreviews,sizes,directoryLayouts,panelWidths,previewLayouts,errors};
}
