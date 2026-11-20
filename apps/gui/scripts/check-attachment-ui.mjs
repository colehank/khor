// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync,existsSync,copyFileSync,rmSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium,webkit} from 'playwright-core';
import {fakeHome,fakeUsage} from './fake-home.mjs';
const gui=process.env.CONTENT_UI_ROOT||fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(gui,'../..');
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-attachment-ui-check');
const out=process.env.CONTENT_UI_OUTPUT||join(scratch,'ui');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||1482), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||1483);
const children=[],sessions=[];
let browser,usage,page;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{const failed=children.find(c=>c.exitCode!==null&&c.exitCode!==0);if(failed)throw Error('Private test service exited: '+failed.exitCode);try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
const run=(cmd,args,env,cwd=gui)=>{const c=spawn(cmd,args,{env,cwd,detached:true,stdio:['ignore','pipe','pipe']});children.push(c);c.stdout.resume();c.stderr.on('data',d=>process.stderr.write(d));return c;};
const api=async(cmd,args={})=>{const r=await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(args)});const value=await r.json();if(!r.ok)throw Error(JSON.stringify(value));return value;};
try{
  rmSync(scratch,{recursive:true,force:true});mkdirSync(out,{recursive:true});
  usage=await fakeUsage();
  const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'content-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
  for(const name of ['bridge','khor'])copyFileSync(join(target,'debug',name),join(scratch,name));
  run(join(scratch,'bridge'),[],env);
  await until(()=>api('devices').then(rows=>rows.length));
  const device=(await api('devices')).find(d=>d.me);
  const projects=['alpha','beta'].map(name=>join(scratch,'project-'+name));
  for(const [i,dir] of projects.entries()){
    mkdirSync(dir,{recursive:true});for(let j=0;j<24;j++)writeFileSync(join(dir,'extra-'+String(j).padStart(2,'0')+'.txt'),'Neighbor '+j+'\n'+(j===15?Array.from({length:80},(_,k)=>'Line '+k).join('\n'):''));writeFileSync(join(dir,'notes.md'),`# ${i===0?'Alpha':'Beta'}\n\nSession contents.\n`);writeFileSync(join(dir,'only-'+i+'.txt'),'This file belongs to '+i+'\n');
    const id=await api('open_on',{machine:'',title:'Content '+(i===0?'Alpha':'Beta'),cwd:dir,cols:100,rows:35});sessions.push(id);
  }
  // CI already verified this dist. Exercise those exact production bytes,
  // rather than starting a second dev optimizer after the other UI gates.
  const mode=process.env.KHOR_TEST_FRONTEND==='dist'?['preview']:[];
  if(mode.length)assert(existsSync(join(gui,'dist/index.html')),'The verified production frontend is missing');
  console.log('attachment frontend:',mode.length?'production dist':'development');
  run(join(gui,'node_modules/.bin/vite'),[...mode,'--host','127.0.0.1','--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});
  await until(()=>fetch(`http://127.0.0.1:${port}/`,{signal:AbortSignal.timeout(1000)}).then(r=>r.ok));
  const useWebKit=process.env.ATTACHMENT_BROWSER==='webkit';
  const channel=(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined;
  const headless=process.env.CONTENT_HEADED!=='1';
  browser=useWebKit?await webkit.launch({headless}):await chromium.launch({channel,headless,args:['--no-proxy-server']});
  console.log('attachment browser:',useWebKit?'WebKit':channel||'bundled Chromium',browser.version());
  const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
  // The LAN HTTP face has no secure-context randomUUID API.
  await context.addInitScript(()=>Object.defineProperty(globalThis.crypto,'randomUUID',{configurable:true,value:undefined}));
  // Simulate the visible space taken by a phone keyboard in the production UI.
  await context.addInitScript(()=>{
    const viewport=new EventTarget();
    let cover=0;
    Object.assign(viewport,{scale:1,offsetTop:0,offsetLeft:0});
    Object.defineProperties(viewport,{height:{get:()=>innerHeight-cover},width:{get:()=>innerWidth}});
    Object.defineProperty(window,'visualViewport',{configurable:true,value:viewport});
    window.__keyboard=(covered,pan=0)=>{
      cover=covered;
      viewport.offsetTop=pan;
      viewport.dispatchEvent(new Event('resize'));
    };
  });
  await context.addInitScript(ids=>{
    localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');
  },sessions);
  page=await context.newPage();page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('PAGEERROR',e.message);});
  const url=`http://127.0.0.1:${port}/?bridge=${bridgePort}`;
  // Initial browser bootstrap has a separate cold-start budget. Every
  // attachment interaction and its real transport assertions retain 12s.
  await page.goto(url,{waitUntil:'domcontentloaded',timeout:60000});
  assert(await page.evaluate(()=>typeof globalThis.crypto.randomUUID==='undefined'),'LAN compatibility probe did not run');
  const shown=()=>page.locator('[data-session-content][data-content-visible=true]');
  const results=[],failures=[],pickerResizes=[],resizes=[];
  await page.route('**/term_resize',async route=>{
    const size=route.request().postDataJSON();
    resizes.push(size);
    if(await page.locator('[data-attachment-picker]').count())pickerResizes.push(size);
    await route.fallback();
  });
  const note=(name,data)=>{results.push({name,...data});console.log(name,JSON.stringify(data));if(data.error)failures.push(name+': '+data.error);};
  const step=async(name,fn)=>{try{await fn();}catch(e){note(name,{error:String(e.stack??e)});await page.screenshot({path:join(out,'attachment-error-'+name+'.png')}).catch(()=>{});}};
  const folder=join(projects[0],'folder');mkdirSync(folder,{recursive:true});writeFileSync(join(folder,'child.md'),'# Child\n');
  const remote={...device,id:'a'.repeat(64),name:'audit-remote',shown:'远端 A',me:false,hop:'direct',hosts:true,paused:false};
  await page.route('**/devices',r=>r.fulfill({json:[device,remote]}));
  await page.route('**/ls',r=>{const asked=r.request().postDataJSON().path;const path=asked||projects[0];const child=path===folder;return r.fulfill({json:{path,entries:child?[{name:'child.md',dir:false,size:8,at_ms:0}]:[{name:'folder',dir:true,size:0,at_ms:0},{name:'notes.md',dir:false,size:8,at_ms:0}],truncated:false}});});
  await page.route('**/read_text',r=>r.fulfill({json:{total:8,at_ms:0,text:'# Audit\n',cut:false,binary:false,digest:'audit'}}));
  await page.route('**/stat_text',r=>r.fulfill({json:{total:8,at_ms:0,digest:'audit'}}));
  let hold=null,release;const staged=[],sent=[],forgotten=[],cancelled=[];
  await page.route(/\/term_drop_stage\?/,async r=>{const name=new URL(r.request().url()).searchParams.get('name');staged.push(name);if(hold)await hold;await r.fulfill({json:'/stage/'+staged.length+'/'+name});});
  await page.route('**/term_drop',r=>{const args=r.request().postDataJSON();sent.push(args);return r.fulfill({json:{landed:args.paths,miss:null}});});
  await page.route('**/term_drop_items',r=>{const args=r.request().postDataJSON();sent.push(args);return r.fulfill({json:{landed:args.items.map(x=>x.path),miss:null}});});
  await page.route('**/term_drop_cancel',r=>{cancelled.push(r.request().postDataJSON());return r.fulfill({json:null});});
  await page.route('**/term_drop_forget',r=>{const {paths}=r.request().postDataJSON();forgotten.push(...paths);return r.fulfill({json:null});});
  await page.route('**/term_drop_progress',r=>r.fulfill({json:null}));
  if(!(await page.locator('[data-deck-title]').isVisible()))await page.locator('[data-session-title]').filter({hasText:/^Content Alpha$/}).click();
  const panel=()=>page.locator('[data-files-panel]');
  const reopen=async()=>{
    if(await panel().count()===0){
      const keys=page.locator('[data-key-row-switch]:visible');
      if(await keys.getAttribute('aria-expanded')!=='true')await keys.click();
      await page.locator('[data-term-key=attach]:visible').click();
    }
    await panel().waitFor();
  };
  await reopen();
  const sentToKeyboard=async(mode='none')=>{
    await panel().waitFor({state:'detached'});
    await page.locator('[data-terminal]:visible .xterm-helper-textarea:focus').waitFor({state:'attached'});
    assert.equal(await page.locator('[data-terminal]:visible .xterm-helper-textarea').getAttribute('inputmode'),mode,'Closing preserves the original keyboard state');
  };
  const directory=()=>page.locator('[data-attachment-directory]:visible');
  const preview=()=>page.locator('[data-attachment-file]:visible');
  const closePreview=async()=>{await preview().locator('[data-file-back]').click();await preview().waitFor({state:'hidden'});};
  const openMachine=async()=>{
    const chip=panel().locator('[data-files-machine-chip="audit-remote"]:visible');
    if(await chip.count())await chip.click();
    else{await panel().locator('[data-files-machines]:visible').click();await page.locator('[data-files-machines-menu] [data-files-machine="audit-remote"]').click();}
    await directory().locator('[data-files-select="notes.md"]').waitFor();
  };
  const previewReady=async()=>{
    await preview().locator('[data-file-view]').waitFor();
    assert.equal(await preview().locator('[data-file-more]').count(),0,'Attachment opens the workspace file page without More');
    assert.equal(await preview().evaluate(()=>!!document.activeElement?.matches('input,textarea,[contenteditable=true]')),false,'Opening a file never summons the keyboard');
  };
  await step('preview-selection',async()=>{
    await openMachine();
    await directory().locator('[data-files-name="notes.md"] [data-files-open]').click();await previewReady();
    assert.equal(sent.length,0);assert.equal(staged.length,0);await closePreview();
    await directory().locator('[data-files-select=folder]').click();
    await directory().locator('[data-files-name=folder]>button').first().click();
    assert(await directory().locator('[data-files-select="child.md"]').isDisabled());
    note('directory-inclusion',{childDisabled:true});await directory().locator('[data-up]').click();
    await directory().locator('[data-files-select=folder]').click();
    assert.equal(await directory().locator('[data-files-select=folder]').getAttribute('aria-pressed'),'false');
    await directory().locator('[data-files-select="notes.md"]').click();await directory().locator('[data-files-directory-back]').click();
    const selected=panel().locator('[data-files-selected="notes.md"]');await selected.waitFor();
    const header=await panel().locator('[data-files-actions]').boundingBox(),send=await panel().locator('[data-files-send]').boundingBox();
    assert(send.y>=header.y&&send.y+send.height<=header.y+header.height+1,'Send stays in the top toolbar, clear of the home gesture');
    await selected.locator('[data-files-open]').click();await previewReady();await closePreview();
    await panel().locator('[data-files-send]').click();await until(()=>sent.length>0);await sentToKeyboard();
    const delivered=sent[0].items??sent[0].paths.map(path=>({path,from:sent[0].from}));
    assert.equal(sent[0].id,sessions[0],'The original terminal owns the transfer');
    assert.equal(delivered[0].from,'audit-remote','Selection preserves the source host');
    assert.equal(delivered[0].path,join(projects[0],'notes.md'));
    note('attachment-send',{sent:sent.length,operation:sent[0]});
  });
  await step('local-file-preview',async()=>{
    await reopen();const before=staged.length,uploads=sent.length;
    await panel().locator('[data-files-input]').setInputFiles([
      {name:'preview-local.txt',mimeType:'text/plain',buffer:Buffer.from('Previewed from original browser bytes')},
      {name:'keep-local.txt',mimeType:'text/plain',buffer:Buffer.from('Kept in selection')},
    ]);
    const card=panel().locator('[data-files-selected="preview-local.txt"]'),keep=panel().locator('[data-files-selected="keep-local.txt"]');
    await card.locator('[data-files-open]').click();await previewReady();
    await preview().getByText('Previewed from original browser bytes',{exact:false}).last().waitFor();
    assert.equal(await preview().locator('[contenteditable=true]').count(),0,'Unsent browser bytes are read only');
    assert.equal(staged.length,before,'Preview never stages browser bytes');assert.equal(sent.length,uploads);
    await closePreview();await card.locator('[data-files-select]').click();await card.waitFor({state:'detached'});
    await keep.waitFor();await keep.locator('[data-files-select]').click();await keep.waitFor({state:'detached'});
    note('local-file-preview',{originalBytes:true,noEagerUpload:true,independentSelection:true});
  });
  await step('local-folder-preview',async()=>{
    await reopen();
    const pickedFolder=join(scratch,'browser-folder');mkdirSync(join(pickedFolder,'nested'),{recursive:true});
    writeFileSync(join(pickedFolder,'README.txt'),'Root file');writeFileSync(join(pickedFolder,'.hidden'),'Hidden file');
    writeFileSync(join(pickedFolder,'nested','folder-preview.txt'),'Actual bytes inside the selected browser folder');
    const before=sent.length,stagedBefore=staged.length;
    const folderInput=panel().locator('[data-files-folder-input]');
    await folderInput.evaluate(input=>{window.__folderInputEvents=[];for(const kind of ['input','change'])input.addEventListener(kind,()=>window.__folderInputEvents.push({kind,files:[...input.files].map(file=>({name:file.name,path:file.webkitRelativePath}))}),{capture:true});});
    try{await folderInput.setInputFiles(pickedFolder);}catch(error){
      const diagnostic={browser:browser.version(),exists:existsSync(pickedFolder),production:await page.evaluate(()=>({events:window.__folderInputEvents,selected:document.querySelectorAll('[data-files-selected]').length,alerts:[...document.querySelectorAll('[role=alert]')].map(node=>node.textContent)})),controls:[]};
      for(const reset of [false,true]){
        const control=await context.newPage();
        try{
          await control.setContent('<input type="file" multiple webkitdirectory>');
          await control.locator('input').evaluate((input,reset)=>{window.__files=[];input.addEventListener('change',()=>{window.__files=[...input.files].map(file=>file.webkitRelativePath);if(reset)input.value='';});},reset);
          let failure=null;try{await control.locator('input').setInputFiles(pickedFolder,{timeout:3000});}catch(error){failure=String(error);}
          diagnostic.controls.push({reset,failure,files:await control.evaluate(()=>window.__files)});
        }finally{await control.close();}
      }
      writeFileSync(join(out,'folder-input-diagnostic.json'),JSON.stringify(diagnostic,null,2));throw error;
    }
    const card=panel().locator('[data-files-selected="browser-folder"]');await card.locator('[data-files-name]>button').first().click();
    await directory().locator('[data-files-name="nested"]').waitFor();
    const entries=await directory().locator('[data-files-name]').evaluateAll(nodes=>nodes.map(node=>node.dataset.filesName));
    // The WebKit folder picker can omit dotfiles before the app gets them.
    assert.deepEqual(useWebKit?entries.filter(name=>name!=='.hidden'):entries,useWebKit?['nested','README.txt']:['nested','README.txt','.hidden']);
    await directory().locator('[data-files-name="nested"]>button').first().click();
    await directory().locator('[data-files-name="folder-preview.txt"] [data-files-open]').click();await previewReady();
    await preview().getByText('Actual bytes inside the selected browser folder',{exact:false}).last().waitFor();await closePreview();
    assert.match(await directory().locator('[data-files-search] input').inputValue(),/nested/);
    assert.equal(await panel().locator('[data-files-selected]').count(),1,'Browsing preserves the folder as one selected attachment');
    assert.equal(sent.length,before);assert.equal(staged.length,stagedBefore,'Folder browsing and previewing never stages');
    await directory().locator('[data-up]').click();await directory().locator('[data-files-name="nested"]').waitFor();
    await directory().locator('[data-files-directory-back]').click();await card.locator('[data-files-select]').click();await card.waitFor({state:'detached'});
    note('local-folder-preview',{actualBytes:true,selectionPreserved:true,noEagerUpload:true});
  });
  const terminalSize=()=>page.locator('[data-terminal]:visible').evaluate(box=>{
    const {bottom,height}=box.getBoundingClientRect();
    return {bottom,height,rows:box.__xterm.rows,cols:box.__xterm.cols};
  });
  await step('attachment-keyboard-and-size',async()=>{
    await panel().locator('[data-files-cancel]').click();
    await sentToKeyboard();
    const attach=page.locator('[data-term-key=attach]:visible');
    const field=page.locator('[data-terminal]:visible .xterm-helper-textarea');
    const down=await terminalSize();
    // Model a system keyboard which follows focus and can pan the visible
    // viewport as far as its old bottom. Focusout and refocus happen in
    // one user gesture; sample after that gesture and animate the height.
    await page.evaluate(()=>{
      let target=0,timer=0;
      window.__followTerminalKeyboard=()=>{
        const next=document.activeElement?.matches('.xterm-helper-textarea[inputmode=text]')?300:0;
        if(next===target)return;
        target=next;
        clearTimeout(timer);
        window.__keyboard(160,next?160:0);
        timer=setTimeout(()=>window.__keyboard(next,next),45);
      };
      document.addEventListener('focusin',window.__followTerminalKeyboard);
      document.addEventListener('focusout',()=>queueMicrotask(window.__followTerminalKeyboard));
    });
    const measured=()=>page.evaluate(()=>({height:innerHeight,visible:visualViewport.height,focus:document.activeElement?.outerHTML.slice(0,150),picker:!!document.querySelector('[data-files-panel]')}));
    for(const mode of ['none','text']){
      const shows=mode==='text';
      const beforeKeyboard=resizes.length;
      if(shows){
        const cursor=await field.evaluate(element=>{
          const box=element.closest('[data-terminal]'),term=box.__xterm,b=term.buffer.active;
          const screen=term.element.querySelector('.xterm-screen').getBoundingClientRect();
          return {
            x:screen.x+(b.cursorX+.5)*screen.width/term.cols,
            y:screen.y+(b.baseY+b.cursorY-b.viewportY+.5)*screen.height/term.rows,
          };
        });
        await page.touchscreen.tap(cursor.x,cursor.y);
      }
      await page.waitForFunction(shows=>visualViewport.height===innerHeight-(shows?300:0),shows).catch(async error=>{throw new Error('Keyboard fixture expected '+mode+': '+JSON.stringify(await measured()),{cause:error});});
      if(shows){
        await until(async()=>(await terminalSize()).rows<down.rows && resizes.length>beforeKeyboard);
        const resized=await terminalSize();
        assert(resized.height<down.height,'A panned keyboard reduces the visible terminal height');
        assert.equal(resizes.at(-1).rows,resized.rows,'The PTY receives the keyboard-reduced row count');
      }
      await page.waitForTimeout(300);
      const size=await terminalSize(),resizeCount=resizes.length;
      for(const close of ['cancel','toggle','send']){
        await reopen();
        if(shows){
          await page.waitForFunction(()=>visualViewport.height===innerHeight);
          assert.equal(await field.getAttribute('inputmode'),'none','The picker lowers the system keyboard');
          assert.deepEqual(await panel().evaluate(node=>{
            const {top,bottom}=node.getBoundingClientRect();
            return {top:Math.round(top),bottom:Math.round(bottom),screen:innerHeight};
          }),{top:544,bottom:844,screen:844},'The picker occupies the keyboard band');
        }
        assert.deepEqual(await terminalSize(),size,`Opening with inputmode=${mode} leaves the terminal in place`);
        if(close==='cancel')await panel().locator('[data-files-cancel]').tap();
        if(close==='toggle')await attach.tap();
        if(close==='send'){
          const before=staged.length;
          await panel().locator('[data-files-input]').setInputFiles([{name:'keyboard.txt',mimeType:'text/plain',buffer:Buffer.from('keyboard')}]);
          hold=new Promise(r=>release=r);
          try{
            await panel().locator('[data-files-send]').tap();
            await until(()=>staged.length===before+1);
            if(shows)await panel().waitFor({state:'detached'});
            assert.equal(await field.getAttribute('inputmode'),mode,'Sending restores the original keyboard preference in the send gesture');
            assert.deepEqual(await terminalSize(),size,'A pending transfer keeps the terminal in place');
          }finally{release();hold=null;}
        }
        await sentToKeyboard(mode);
        await page.waitForFunction(shows=>visualViewport.height===innerHeight-(shows?300:0),shows).catch(async error=>{throw new Error(close+' did not restore '+mode+': '+JSON.stringify(await measured()),{cause:error});});
        assert.equal(await attach.getAttribute('aria-expanded'),'false');
        assert.deepEqual(await terminalSize(),size,`Closing with ${close} leaves the terminal in place`);
      }
      await page.waitForTimeout(300); // Include any delayed PTY resize after unmount.
      assert.equal(resizes.length,resizeCount,`Attachment interactions send no PTY resize with inputmode=${mode}`);
    }
    await field.evaluate(element=>element.blur());
    await page.waitForFunction(()=>visualViewport.height===innerHeight);
    await until(async()=>{
      const current=await terminalSize();
      return current.rows>=down.rows && resizes.at(-1)?.rows===current.rows;
    });
    await page.waitForTimeout(250); // Let the keyboard-down key-row choice settle before reopening.
    note('attachment-keyboard-and-size',{keyboardUp:true,keyboardDown:true,close:['cancel','toggle','send'],ptyStable:true});
  });
  await step('cancel-immediate-retry',async()=>{
    await reopen();
    const file=panel().locator('[data-files-input]');const stagedBefore=staged.length,forgottenBefore=forgotten.length,sentBefore=sent.length;await file.setInputFiles([{name:'retry-a.txt',mimeType:'text/plain',buffer:Buffer.from('a')},{name:'retry-b.txt',mimeType:'text/plain',buffer:Buffer.from('b')}]);
    let finishCancel;const ack=new Promise(r=>finishCancel=r);
    await page.unroute('**/term_drop_cancel');await page.route('**/term_drop_cancel',async r=>{cancelled.push(r.request().postDataJSON());await ack;await r.fulfill({json:null});});
    hold=new Promise(r=>release=r);
    try{
      await page.locator('[data-files-send]').click();await until(()=>staged.length===stagedBefore+1);
      await page.locator('[data-files-panel]:has([data-files-transfer]) [data-files-cancel]').click();
      assert(await panel().locator('[data-files-send]').isDisabled(),'cancel exposed an early retry');
      await page.locator('[data-files-transfer]').waitFor();
      assert.equal(await panel().locator('[data-files-cancel]').isDisabled(),true,'cancel cannot be repeated during cleanup');
      release();hold=null;await until(()=>forgotten.length>forgottenBefore);
      assert(await panel().locator('[data-files-send]').isDisabled(),'retry unlocked before cancellation acknowledgement');
      finishCancel();await until(async()=>!(await panel().locator('[data-files-send]').isDisabled()));await panel().locator('[data-files-send]').click();
      await until(()=>sent.length===sentBefore+1);await sentToKeyboard();
      assert(forgotten.slice(forgottenBefore).every(path=>path.startsWith('/stage/')),'Cleanup only retires staged copies');
      note('cancel-retry',{staged:staged.length,sent:sent.length,cancels:cancelled.length});
    }finally{release?.();hold=null;finishCancel?.();}
  });
  await step('empty-system-change',async()=>{
    await reopen();
    await panel().locator('input[type=file]:not([webkitdirectory])').setInputFiles([]);
    assert.equal(await panel().locator('[data-files-selected]').count(),0,'empty change entered a selection');
    assert(await panel().locator('[data-files-pick=device]').isVisible());assert(await panel().locator('[data-files-pick=folder]').isVisible());
    const row=panel().locator('[data-files-machine-row]'),more=row.locator('[data-files-machines]');
    await row.filter({has:page.locator('[data-files-machine-chip="audit-remote"]')}).waitFor();
    assert.equal(await row.getAttribute('data-files-source-count'),'1','The remote source remains available beside both native local pickers');
    const visible=await row.locator('[data-files-machine-chip]:visible').evaluateAll(nodes=>nodes.map(node=>node.dataset.filesMachineChip));
    const hidden=Number(await more.getAttribute('data-files-machine-count'));
    assert.equal(visible.length+hidden,1,'Visible chips and overflowing choices account for every remote source');
    let overflow=[];
    if(hidden){await more.click();overflow=await page.locator('[data-files-machines-menu] [data-files-machine]').evaluateAll(nodes=>nodes.map(node=>node.dataset.filesMachine));assert.equal(overflow.length,hidden);await page.keyboard.press('Escape');}
    else assert.equal(await more.isVisible(),false,'More is hidden when every machine fits');
    assert.deepEqual([...visible,...overflow],['audit-remote'],'The original remote source remains discoverable without duplicating this device');
    note('empty-system-change',{sources:2,localPickers:true,remoteSources:1});
  });
  assert.deepEqual(pickerResizes,[],'Choosing sources, opening files and changing keyboard focus never resizes the real PTY');
  writeFileSync(join(out,'attachment-results.json'),JSON.stringify({results,errors,pickerResizes},null,2));assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);console.log('attachment UI gate: clean');
}catch(e){
  await page?.screenshot({path:join(out,'failed.png')}).catch(()=>{});
  const state=await page?.evaluate(()=>({ready:document.readyState,body:document.body?.innerText.slice(0,2048)})).catch(()=>null);
  writeFileSync(join(out,'failure.json'),JSON.stringify({error:String(e),state},null,2));
  console.error(e);throw e;
}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close().catch(()=>{});
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  // Keep screenshots and fixture state, not a full binary pair per run.
  for(const name of ['bridge','khor'])rmSync(join(scratch,name),{force:true});
  await usage?.close();
}
