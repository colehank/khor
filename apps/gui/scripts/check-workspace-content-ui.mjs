// Actual workspace/session views and tab owners; only node IO uses an isolated in-memory fixture.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync,mkdirSync,readFileSync,writeFileSync,rmSync} from 'node:fs';
import {join,basename} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const output=process.env.WORKSPACE_CONTENT_DIR;assert(output,'Set WORKSPACE_CONTENT_DIR to an evidence directory');mkdirSync(output,{recursive:true});
const fixture=mkdtempSync(join(gui,'.workspace-content-check-'));
writeFileSync(join(fixture,'index.html'),'<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"><link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture,'probe.tsx'),readFileSync(join(gui,'scripts/fixtures/workspace-content-ui.tsx')));
const files=['src/hooks/use-session-content.ts','src/hooks/use-content-navigation.ts','src/views/SessionContent.tsx','src/views/MobileSessionContent.tsx','src/views/SideWorkspace.tsx','src/views/WorkspacePane.tsx','src/views/CommitPage.tsx','src/components/WorkspaceGitNavigation.tsx','src/browse/TabStrip.tsx','src/components/icons.tsx','src/styles/workspace-navigation.css','src/styles/app.css','src/lib/scroll-edge.ts','src/views/MobileContentPage.tsx','src/components/ContentToolbar.tsx'];
const fingerprint=()=>Object.fromEntries(files.map(file=>[file,createHash('sha256').update(readFileSync(join(gui,file))).digest('hex')]));
const before=fingerprint(),calls=[],commits=Array.from({length:82},(_,i)=>({id:(i+1).toString(16).padStart(40,'a'),subject:`Commit ${i+1}`,author:'Fixture author',when_ms:Date.UTC(2026,10,1)-i*3600000,cut:false}));
let tabs={tabs:[{id:'source',session:'session',device:'fixture',url:'',view:'workspace',pinned:false,title:'',touched:1}],active:'source'},sequence=0,server,browser;
const answer=(cmd,a)=>{
 calls.push({cmd,args:a});
 if(cmd==='tabs_list')return tabs;
 if(cmd==='tabs_open'){const known=tabs.tabs.find(t=>t.request===a.request&&t.session===a.session);if(known)return known.id;const id='tab-'+(++sequence);tabs.tabs.push({id,...a,pinned:false,title:'',touched:Date.now()});tabs.active=id;return id;}
 if(cmd==='tabs_go'){const tab=tabs.tabs.find(t=>t.id===a.id);assert(tab);Object.assign(tab,a);return tabs;}
 if(cmd==='tabs_focus'){tabs.active=a.id;return tabs;}
 if(cmd==='tabs_close'){tabs.tabs=tabs.tabs.filter(t=>t.id!==a.id);return tabs;}
 if(cmd==='tabs_pin'){tabs.tabs.find(t=>t.id===a.id).pinned=a.on;return tabs;}
 if(cmd==='tabs_reorder')return tabs;
 if(cmd==='ls')return {path:a.path||'/fixture',entries:['src','a.md','b.md'].map(name=>({name,dir:name==='src',size:100,at_ms:1,from:null,received_ms:null})),truncated:false};
 if(cmd==='checkout')return true;
 if(cmd==='git_status')return {branch:'integrate/round3',rows:[],more:false,ahead:0,behind:0,fetched_ms:0};
 if(cmd==='git_log')return {commits:[{...commits[0],body:'Commit 1\n\nReal fixture body'}],more:false};
 if(cmd==='git_history'){const start=Number(a.cursor||0),limit=a.limit||40;return {head:commits[0].id,commits:commits.slice(start,start+limit),next:start+limit<commits.length?String(start+limit):null};}
 if(cmd==='git_commit')return {commit:{...commits.find(c=>c.id===a.rev),body:'Commit body for '+a.rev.slice(-2)+'\n\nComplete metadata from node'},total:1,more:false,files:[{path:'src/long.ts',from:'',status:'M',added:1,removed:1,binary:false,big:false,capped:false,before:Array.from({length:90},(_,i)=>`const line${i} = "before";`).join('\n'),after:Array.from({length:90},(_,i)=>`const line${i} = "after";`).join('\n')}]};
 if(cmd==='read_text')return {total:100,at_ms:1,text:'# '+a.path+'\n\nA real file view fixture.\n',cut:false,binary:false,digest:'a'.repeat(64)};
 if(cmd==='dirs_begin')throw Error('native watch unavailable in browser fixture');
 if(cmd==='code_here')return {version:'',name:'',command:''};
 if(cmd==='trash_here')return false;
 if(cmd==='page_icons')return {};
 if(cmd==='git_diff')return {hunks:[]};
 if(cmd==='git_blame')return {lines:[],commits:{},more:false};
 if(cmd==='git_windows'||cmd==='recents'||cmd==='devices'||cmd==='workspaces'||cmd==='code_windows')return [];
 return null;
};
const until=async check=>{const end=Date.now()+10000;while(!check()){assert(Date.now()<end,'Controlled IO did not settle');await new Promise(r=>setTimeout(r,20));}};
const results=[];
try{
 server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:Number(process.env.WORKSPACE_CONTENT_PORT||23834),strictPort:true,hmr:false},clearScreen:false,logLevel:'error',plugins:[{name:'private-workspace-io',configureServer(s){s.middlewares.use(async(req,res,next)=>{if(!req.url?.startsWith('/api/'))return next();try{let raw='';for await(const part of req)raw+=part;const value=answer(req.url.slice(5),raw?JSON.parse(raw):{});res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value));}catch(e){res.statusCode=400;res.end(JSON.stringify(String(e)));}});}}]});
 await server.listen();const url=`http://127.0.0.1:${server.config.server.port}/${basename(fixture)}/index.html`;
 if(process.env.WORKSPACE_CONTENT_LAB==='1'){console.log('Private fixture Lab '+url);await new Promise(resolve=>{process.once('SIGINT',resolve);process.once('SIGTERM',resolve);});}
 else{
 browser=await chromium.launch({channel:'chrome',headless:true});
 const page=await browser.newPage({viewport:{width:1400,height:900},reducedMotion:'reduce'});page.setDefaultTimeout(20000);const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('Fixture page error:',e.message);});
 await page.goto(url+(process.env.WORKSPACE_REVIEW_THEME==='light'?'?theme=light':''));const picker=page.locator('[data-content-picker]:visible');await picker.locator('[data-workspace-navigation]').waitFor();
 if(process.env.WORKSPACE_REVIEW_REQUIRE==='1'){
   await picker.locator('[data-content-source="browser"]').click();
   const back=await picker.locator('[data-content-back]').count();
   results.push({name:'browser picker has no workspace return',passed:back===0,back});
   await picker.locator('[data-content-source="workspace"]').click();
   await picker.locator('[data-file-path="/fixture/a.md"]').click();await page.locator('[data-content-visible="true"] .cm-content').waitFor();
   results.push({name:'new starting tab converts into first document',passed:tabs.tabs.length===1,tabs:structuredClone(tabs)});
   const layout=await page.locator('[data-content-visible="true"] [data-file-page]').evaluate(box=>{
     const head=box.querySelector('[data-content-toolbar]'),scroll=box.querySelector('.cm-scroller'),line=box.querySelector('.cm-line'),content=box.querySelector('.cm-content');
     const rect=e=>{const r=e.getBoundingClientRect();return {top:r.top,bottom:r.bottom,height:r.height};};
     const scope=head.querySelector('[data-content-scope]'),back=head.querySelector('[data-content-back]'),location=head.querySelector('[data-content-location-layer]');
     const order=scope.getBoundingClientRect().right<=back.getBoundingClientRect().left&&back.getBoundingClientRect().right<=location.getBoundingClientRect().left;
     return {order,head:rect(head),scroll:rect(scroll),line:rect(line),scrollPadding:getComputedStyle(scroll).paddingTop,contentPadding:getComputedStyle(content).paddingTop,port:scroll.outerHTML.slice(0,450),parent:box.outerHTML.slice(0,250)};
   });results.push({name:'file first line clears original toolbar',passed:layout.line.top>=layout.head.bottom&&layout.order,layout});
   await page.screenshot({path:join(output,'review-file.png')});
   await page.evaluate(()=>window.__workspaceProbe.navigation.browse('session'));
   await picker.getByRole('button',{name:'提交历史',exact:true}).click();await picker.locator('[data-workspace-log-commit]').first().click();
   const scroller=page.locator('[data-content-visible="true"] [data-commit-scroll]');await scroller.locator('.cm-editor').waitFor();
   await scroller.evaluate(el=>{el.scrollTop=el.querySelector('[data-commit-file]').offsetTop+150;});
   await page.waitForTimeout(150);
   const sticky=await scroller.locator('[data-commit-file]').first().evaluate(group=>{
     const head=group.querySelector('[data-sticky-head]'),body=group.querySelector('[data-sticky-body]');const h=head.getBoundingClientRect(),b=body.getBoundingClientRect();
     return {head:{top:h.top,bottom:h.bottom},body:{top:b.top,bottom:b.bottom},held:head.hasAttribute('data-sticky-held'),fading:body.hasAttribute('data-sticky-fading'),mask:getComputedStyle(body).maskImage,start:body.style.getPropertyValue('--sticky-fade-start'),length:body.style.getPropertyValue('--sticky-fade-length'),scroller:group.parentElement.outerHTML.slice(0,200)};
   });results.push({name:'commit body fades beneath held title',passed:sticky.held&&sticky.fading,sticky});
   await page.waitForTimeout(500);
   await scroller.evaluate(el=>{el.scrollTop=Math.max(0,el.scrollHeight-el.clientHeight-12);});await page.waitForTimeout(60);
   const beforeScroll=await scroller.locator('[data-sticky-body="flush"]').first().evaluate(el=>parseFloat(el.style.getPropertyValue('--sticky-fade-start')));
   await scroller.evaluate(el=>{el.scrollTop+=8;});await page.waitForTimeout(60);
   const afterScroll=await scroller.locator('[data-sticky-body="flush"]').first().evaluate(el=>parseFloat(el.style.getPropertyValue('--sticky-fade-start')));
   results.push({name:'settled commit scroll updates shared material',passed:afterScroll-beforeScroll>6,beforeScroll,afterScroll});
   await page.screenshot({path:join(output,'review-commit.png')});
   console.log(JSON.stringify(results));assert(results.every(r=>r.passed),'review regressions reproduced');
 } else {
 const box=await picker.locator('[data-workspace-navigation]').evaluate(nav=>{const b=nav.querySelector('button:last-child').getBoundingClientRect(),close=nav.closest('[data-content-picker]').querySelector('[data-content-close]')?.getBoundingClientRect();const first=nav.querySelector('button').getBoundingClientRect();return {history:b.x+b.width/2,close:close?close.x+close.width/2:null,widths:[first.width,b.width],heights:[first.height,b.height]};});
 assert.equal(box.history,box.close);assert.equal(box.widths[0],box.widths[1]);assert.equal(box.heights[0],box.heights[1]);
 const source=await page.evaluate(()=>window.__workspaceProbe.content.tabs[0].id);
 const browse=()=>page.locator('[data-tab-open="session"]').click();
 const browseHistory=async()=>{await browse();await picker.getByRole('button',{name:'提交历史',exact:true}).click();};
 await picker.getByRole('button',{name:'提交历史',exact:true}).click();
 await picker.locator('[data-workspace-log-commit]').first().waitFor();
 await picker.locator('[data-workspace-log-older]').click();await picker.locator('[data-workspace-log-commit]').filter({hasText:'Commit 41'}).waitFor();
 await picker.locator('[data-workspace-log-newer]').click();
 const first=commits[0].id,second=commits[1].id;
 await picker.locator(`[data-workspace-log-commit="${first}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${first}"]`).waitFor();
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1'){
   const selectedTitle=page.locator('[data-content-on="true"] [data-tab-name]');
   assert.equal(await selectedTitle.evaluate(el=>getComputedStyle(el).fontStyle),'normal','single-clicked session commit is kept immediately');
   assert.equal(await page.locator('[data-tab-preview]').count(),0,'temporary title has no hollow ring');
 }
 await until(()=>tabs.tabs.find(t=>t.id===source)?.view==='commit');assert.equal(tabs.tabs.length,1,'first commit replaces the starting tab');
 const commitToolbar=page.locator('[data-content-visible="true"] [data-content-toolbar="commit"]');
 assert.equal(await commitToolbar.locator('[data-content-back]').count(),1,'session commit needs the workspace Back');
 const commitOrder=await commitToolbar.evaluate(head=>{const a=head.querySelector('[data-content-scope]').getBoundingClientRect(),b=head.querySelector('[data-content-back]').getBoundingClientRect(),c=head.querySelector('[data-content-location-layer]').getBoundingClientRect();return a.right<=b.left&&b.right<=c.left;});assert(commitOrder,'commit Back belongs between source and content pill');
 const commitPort=page.locator('[data-content-visible="true"] [data-commit-scroll]');await commitPort.locator('.cm-editor').waitFor();
 await commitPort.evaluate(el=>{el.scrollTop=el.scrollHeight;});await page.waitForTimeout(60);
 const heldTop=await commitPort.locator('[data-commit-file-bar]').first().evaluate(el=>el.getBoundingClientRect().top),toolbarBottom=await commitToolbar.evaluate(el=>el.getBoundingClientRect().bottom);
 assert(heldTop>=toolbarBottom-1,`held commit file title ${heldTop} must clear toolbar ${toolbarBottom}`);
 await commitPort.evaluate(el=>{el.scrollTop=0;});await page.screenshot({path:join(output,'session-commit-back.png')});


 await page.locator('[data-content-visible="true"] [data-commit-collapse]').click();assert.equal(await page.locator('[data-content-visible="true"] [data-commit-collapse]').getAttribute('aria-expanded'),'false');
 await commitToolbar.locator('[data-content-back]').click();await picker.locator('[data-workspace-navigation]').waitFor();
 assert.equal(tabs.tabs.find(t=>t.id===source).view,'commit','Back keeps the commit tab identity');
 await page.locator(`[data-tab-open="${source}"]`).click();await page.locator('[data-content-visible="true"] [data-commit-collapse]').waitFor();assert.equal(await page.locator('[data-content-visible="true"] [data-commit-collapse]').getAttribute('aria-expanded'),'false','Back and reopen preserves collapse state');

 await browseHistory();await picker.locator(`[data-workspace-log-commit="${second}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${second}"]`).waitFor();await until(()=>tabs.tabs.length===2);assert.equal(tabs.tabs.length,2,'single click keeps both commits');
 const secondTab=await page.evaluate(()=>{const c=window.__workspaceProbe.content;return c.handle(c.active('session'));});
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1')assert.equal(await page.locator(`[data-tab="${secondTab}"] [data-tab-name]`).evaluate(el=>getComputedStyle(el).fontStyle),'normal','kept commit title returns to normal');
 await browseHistory();await picker.locator(`[data-workspace-log-commit="${first}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${first}"]`).waitFor();await until(()=>tabs.tabs.length===2);assert.equal(tabs.tabs.length,2,'reopening a kept commit reuses its tab');
 await page.locator(`[data-tab="${secondTab}"] [data-tab-open]`).click();await page.locator('[data-content-visible="true"] [data-commit-collapse]').waitFor();assert.equal(await page.locator('[data-content-visible="true"] [data-commit-collapse]').getAttribute('aria-expanded'),'true','new commit starts expanded');
 await browse();await picker.getByRole('button',{name:'文件树',exact:true}).click();await picker.locator('[data-file-tree]').waitFor();
 await page.screenshot({path:join(output,'session-workspace.png')});results.push({name:'original session deck, starting tab conversion, single-click commit retention and deduplication',passed:true,alignment:box});
 await picker.locator('[data-file-path="/fixture/a.md"]').click();
 const editor=page.locator('[data-content-visible="true"] .cm-content');await editor.waitFor();
 const keptFile=await page.evaluate(()=>{const c=window.__workspaceProbe.content;return c.handle(c.active('session'));});
 await browse();await picker.locator('[data-file-path="/fixture/b.md"]').click();await editor.waitFor();
 await until(()=>tabs.tabs.some(t=>t.url==='/fixture/b.md'));
 assert(tabs.tabs.some(t=>t.url==='/fixture/a.md'),'single-clicking another file preserves the first without editing or double-click');
 const fileCount=tabs.tabs.length;
 await page.locator(`[data-tab="${keptFile}"] [data-tab-open]`).click();await editor.fill('A private unsaved draft');
 await browse();await picker.locator('[data-file-path="/fixture/b.md"]').click();await editor.waitFor();assert.equal(tabs.tabs.length,fileCount,'same file reuses its tab');
 await page.locator(`[data-tab="${keptFile}"] [data-tab-open]`).click();assert.equal(await editor.innerText(),'A private unsaved draft');
 assert.equal(await page.locator('[data-content-on="true"] [data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'normal');
 results.push({name:'single-click file retention, deduplication and original draft retention',passed:true});
 await page.goto(url+'?workspace=1&theme=light');await page.locator('[data-workspace-navigation]').waitFor();await page.screenshot({path:join(output,'workspace-light.png')});
 await page.locator('[data-workspace-navigation]').getByRole('button',{name:'提交历史',exact:true}).click();await page.locator('[data-workspace-log-commit]').first().waitFor();await page.locator('[data-workspace-log-commit]').first().click();await page.locator('[data-commit-page]').waitFor();await page.screenshot({path:join(output,'workspace-commit.png')});
 const standalone=page.locator('[data-ws-mid]');
 assert.equal(await standalone.locator('[data-tab][data-on="true"] [data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'italic','standalone single-click preview remains italic');
 await page.locator('[data-workspace-log-commit]').nth(1).click();
 assert.equal(await standalone.locator('[data-tab]').count(),1,'standalone next single click replaces preview');
 await page.locator('[data-workspace-log-commit]').nth(1).dblclick();
 assert.equal(await standalone.locator('[data-tab][data-on="true"] [data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'normal','standalone double click keeps preview');
 await page.locator('[data-workspace-log-commit]').first().click();assert.equal(await standalone.locator('[data-tab]').count(),2);
 results.push({name:'standalone workspace still previews on single click and keeps on double click',passed:true});
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1'){
   await page.goto(url+'?preview-samples=1');const sample=page.locator('[data-tab="preview"]');
   assert.equal(await sample.locator('[data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'italic');
   assert.equal(await sample.locator('[data-tab-sub]').evaluate(el=>getComputedStyle(el).fontStyle),'normal','two-line subtitle keeps normal style');
   assert.equal(await sample.locator('[data-bell-rang]').count(),1,'unread mark remains independent');
   assert.equal(await sample.locator('[data-tab-dirty]').count(),1,'unsaved mark remains independent');
   await page.screenshot({path:join(output,'preview-two-line.png')});
   await sample.locator('[data-tab-open]').dblclick();assert.equal(await sample.locator('[data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'normal');
   await page.setViewportSize({width:390,height:844});await page.goto(url+'?preview-samples=1&mobile=1');
   assert.equal(await page.locator('[data-tab="preview"] [data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'italic');await page.screenshot({path:join(output,'preview-narrow.png')});
   results.push({name:'two-line/narrow title typography preserves subtitle, unread and unsaved marks',passed:true});
 }
 const phone=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
 phone.on('pageerror',e=>errors.push(e.message));await phone.goto(url+'?mobile=1');await phone.waitForFunction(()=>window.__workspaceProbe?.content.ready);await phone.locator('[data-content-visible="true"]').waitFor();await phone.evaluate(()=>window.__workspaceProbe.navigation.browse('session'));const mobilePicker=phone.locator('[data-content-picker]:visible');
 await mobilePicker.locator('[data-workspace-navigation]').waitFor();await mobilePicker.getByRole('button',{name:'提交历史',exact:true}).click();
 await mobilePicker.locator('[data-workspace-log-commit]').first().click();await phone.locator('[data-content-visible="true"] [data-commit-page]').waitFor();
 await phone.locator('[data-content-visible="true"] [data-content-deck]').waitFor();await phone.screenshot({path:join(output,'mobile-commit.png')});
 await phone.locator('[data-content-visible="true"] [data-content-deck]').getByRole('button',{name:'返回',exact:true}).click();await mobilePicker.locator('[data-workspace-navigation]').waitFor();
 results.push({name:'mobile commit retains original content deck and can return to workspace',passed:true});await phone.close();
 }
 assert.deepEqual(errors,[]);assert.deepEqual(fingerprint(),before,'test evidence must match unchanged source');
 }
}catch(e){if(browser){for(const context of browser.contexts())for(const page of context.pages())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}results.push({passed:false,error:String(e)});throw e;}
finally{writeFileSync(join(output,'report.json'),JSON.stringify({sources:before,results,calls},null,2));await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
console.log(JSON.stringify(results));
