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
const files=['src/hooks/use-session-content.ts','src/hooks/use-content-navigation.ts','src/views/SessionContent.tsx','src/views/MobileSessionContent.tsx','src/views/SideWorkspace.tsx','src/views/WorkspacePane.tsx','src/views/CommitPage.tsx','src/components/WorkspaceGitNavigation.tsx','src/browse/TabStrip.tsx','src/components/icons.tsx','src/styles/workspace-navigation.css'];
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
 await page.goto(url);const picker=page.locator('[data-content-picker]:visible');await picker.locator('[data-workspace-navigation]').waitFor();
 const box=await picker.locator('[data-workspace-navigation]').evaluate(nav=>{const b=nav.querySelector('button:last-child').getBoundingClientRect(),close=nav.closest('[data-content-picker]').querySelector('[data-content-close]')?.getBoundingClientRect();const first=nav.querySelector('button').getBoundingClientRect();return {history:b.x+b.width/2,close:close?close.x+close.width/2:null,widths:[first.width,b.width],heights:[first.height,b.height]};});
 assert.equal(box.history,box.close);assert.equal(box.widths[0],box.widths[1]);assert.equal(box.heights[0],box.heights[1]);
 const source=await page.evaluate(()=>window.__workspaceProbe.content.tabs[0].id);
 await picker.getByRole('button',{name:'提交历史',exact:true}).click();
 await picker.locator('[data-workspace-log-commit]').first().waitFor();
 await picker.locator('[data-workspace-log-older]').click();await picker.locator('[data-workspace-log-commit]').filter({hasText:'Commit 41'}).waitFor();
 await picker.locator('[data-workspace-log-newer]').click();
 const first=commits[0].id,second=commits[1].id;
 await picker.locator(`[data-workspace-log-commit="${first}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${first}"]`).waitFor();
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1'){
   const previewTitle=page.locator('[data-content-on="true"] [data-tab-name]');
   assert.equal(await previewTitle.evaluate(el=>getComputedStyle(el).fontStyle),'italic','temporary commit title uses italic');
   assert.equal(await page.locator('[data-tab-preview]').count(),0,'temporary title has no hollow ring');
 }
 assert.equal(tabs.tabs.length,2,'commit does not replace source picker');assert.equal(tabs.tabs.find(t=>t.id===source).view,'workspace');
 await page.locator('[data-content-visible="true"] [data-commit-collapse]').click();assert.equal(await page.locator('[data-content-visible="true"] [data-commit-collapse]').getAttribute('aria-expanded'),'false');
 await page.locator(`[data-content-tab="${source}"]`).click();await picker.locator(`[data-workspace-log-commit="${second}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${second}"]`).waitFor();assert.equal(tabs.tabs.length,2,'second commit replaces preview');
 const preview=await page.evaluate(()=>window.__workspaceProbe.content.active('session').id);
 await page.locator(`[data-content-tab="${preview}"] [data-tab-open]`).dblclick();
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1')assert.equal(await page.locator(`[data-content-tab="${preview}"] [data-tab-name]`).evaluate(el=>getComputedStyle(el).fontStyle),'normal','kept commit title returns to normal');
 await page.locator(`[data-content-tab="${source}"]`).click();await picker.locator(`[data-workspace-log-commit="${first}"]`).click();await page.locator(`[data-content-visible="true"] [data-commit-page="${first}"]`).waitFor();await until(()=>tabs.tabs.length===3);assert.equal(tabs.tabs.length,3,'double-click kept the earlier commit');
 await page.locator(`[data-content-tab="${preview}"] [data-tab-open]`).click();await page.locator('[data-content-visible="true"] [data-commit-collapse]').waitFor();assert.equal(await page.locator('[data-content-visible="true"] [data-commit-collapse]').getAttribute('aria-expanded'),'true','new commit starts expanded');
 await page.locator(`[data-content-tab="${source}"]`).click();await picker.getByRole('button',{name:'文件树',exact:true}).click();await picker.locator('[data-file-tree]').waitFor();
 await page.screenshot({path:join(output,'session-workspace.png')});results.push({name:'original session deck, independent commit tabs, preview keep, pagination and collapse',passed:true,alignment:box});
 await picker.locator('[data-file-path="/fixture/a.md"]').click();
 const editor=page.locator('[data-content-visible="true"] .cm-content');await editor.waitFor();await editor.fill('A private unsaved draft');
 await page.waitForFunction(()=>{const c=window.__workspaceProbe.content,t=c.active('session');return t?.view==='file'&&!c.isPreview(t);});
 if(process.env.WORKSPACE_PREVIEW_REQUIRE==='1')assert.equal(await page.locator('[data-content-on="true"] [data-tab-name]').evaluate(el=>getComputedStyle(el).fontStyle),'normal','typing keeps file title in normal style');
 const dirtyId=await page.evaluate(()=>window.__workspaceProbe.content.active('session').id);
 await page.locator(`[data-content-tab="${source}"] [data-tab-open]`).click();await picker.locator('[data-file-path="/fixture/b.md"]').click();await editor.waitFor();
 assert(tabs.tabs.some(t=>t.id===dirtyId&&t.url==='/fixture/a.md'),'editing preserves a kept file while the next file opens');
 await page.locator(`[data-content-tab="${dirtyId}"] [data-tab-open]`).click();assert.equal(await editor.innerText(),'A private unsaved draft');
 results.push({name:'actual file editing keeps preview and draft across another selection',passed:true});
 await page.goto(url+'?workspace=1&theme=light');await page.locator('[data-workspace-navigation]').waitFor();await page.screenshot({path:join(output,'workspace-light.png')});
 await page.locator('[data-workspace-navigation]').getByRole('button',{name:'提交历史',exact:true}).click();await page.locator('[data-workspace-log-commit]').first().waitFor();await page.locator('[data-workspace-log-commit]').first().click();await page.locator('[data-commit-page]').waitFor();await page.screenshot({path:join(output,'workspace-commit.png')});results.push({name:'actual standalone workspace header and commit tabs',passed:true});
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
 phone.on('pageerror',e=>errors.push(e.message));await phone.goto(url+'?mobile=1');await phone.waitForFunction(()=>window.__workspaceProbe?.content.ready);await phone.evaluate(()=>{const {content,navigation}=window.__workspaceProbe,source=content.tabs.find(t=>t.id==='source');navigation.select(source);void content.focus(source.id);});const mobilePicker=phone.locator('[data-content-picker]:visible');
 await mobilePicker.locator('[data-workspace-navigation]').waitFor();await mobilePicker.getByRole('button',{name:'提交历史',exact:true}).click();
 await mobilePicker.locator('[data-workspace-log-commit]').first().click();await phone.locator('[data-content-visible="true"] [data-commit-page]').waitFor();
 await phone.locator('[data-content-visible="true"] [data-content-deck]').waitFor();await phone.screenshot({path:join(output,'mobile-commit.png')});
 await phone.locator('[data-content-visible="true"] [data-content-deck]').getByRole('button',{name:'返回',exact:true}).click();await mobilePicker.locator('[data-workspace-navigation]').waitFor();
 results.push({name:'mobile commit retains original content deck and can return to workspace',passed:true});await phone.close();
 assert.deepEqual(errors,[]);assert.deepEqual(fingerprint(),before,'test evidence must match unchanged source');
 }
}catch(e){if(browser){for(const context of browser.contexts())for(const page of context.pages())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}results.push({passed:false,error:String(e)});throw e;}
finally{writeFileSync(join(output,'report.json'),JSON.stringify({sources:before,results,calls},null,2));await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
console.log(JSON.stringify(results));
