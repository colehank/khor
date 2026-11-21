import assert from 'node:assert/strict';
import path from 'node:path';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';

export async function checkSessionImport(browser,{port,report,engine,width,theme}){
 const name=`session-import-${engine}-${width}-${theme}`,context=await browser.newContext({viewport:{width,height:900},isMobile:width<500,hasTouch:width<500}),page=await context.newPage(),errors=[];
 page.setDefaultTimeout(15000);page.on('pageerror',error=>errors.push(String(error)));
 const calls=()=>page.evaluate(async()=>await(await fetch('/api/__ai_preview/import-calls',{method:'POST',body:'{}'})).json());
 const open=async()=>{await page.locator('[data-pane-new]:visible').first().click();await page.locator('[data-new-item="import"]:visible').click();await page.locator('[data-import-row]').first().waitFor();};
 try{
  const environment=JSON.stringify({platform:width<500?'ios':'desktop',theme,material:'frost',opacity:20,keyboard:0,safeTop:0,safeBottom:width<500?34:0,background:'dark'});
  if(width===1280&&theme==='dark'){
    // The token uses origin storage. Reload it on a static page so the
    // browser doesn't abort outstanding App bootstrap requests mid-check.
    await page.goto(`http://127.0.0.1:${port}/lab/example`);
    const source=await readFile(new URL('../../src/lib/session-import-intent.ts',import.meta.url),'utf8');
    const compiled=ts.transpile(source,{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS});
    const install=()=>page.evaluate(code=>{window.__importRetry=Function('exports',code+'\nreturn exports;')({});},compiled);
    await install();
    const tokens=await page.evaluate(()=>Promise.all([0,1].map(()=>window.__importRetry.claimImportIntent('test-home','test-agent','full-native','fork'))));
    assert.equal(tokens[0],tokens[1],'Concurrent callers serialize one unresolved action token');
    await page.reload();await install();
    const retry=await page.evaluate(()=>window.__importRetry.claimImportIntent('test-home','test-agent','full-native','fork'));
    assert.equal(retry,tokens[0],'A reload retries exactly the same token');
    assert(await page.evaluate(async()=>{try{await window.__importRetry.claimImportIntent('test-home','test-agent','full-native','import');return false;}catch{return true;}}),'Changing action cannot overwrite an unresolved fork');
    await page.evaluate(token=>window.__importRetry.settleImportIntent('test-home','test-agent','full-native','fork',token),retry);
    const next=await page.evaluate(()=>window.__importRetry.claimImportIntent('test-home','test-agent','full-native','fork'));assert.notEqual(next,retry,'A confirmed result permits a later independent action');
  }
  await page.goto(`http://127.0.0.1:${port}/app/?scene=list&machines=3&environment=${encodeURIComponent(environment)}`);
  await open();
  const dialog=page.locator('[data-import-dialog]'),read=await calls();
  assert(read.length>=2&&read.every(call=>call.action.kind==='read'),'Opening reads each source without starting providers');
  for(const agent of ['claude-acp','codex-acp','dsh','generic-acp'])assert(await dialog.locator(`[data-import-agent="${agent}"]`).count()>0,agent+' uses the same dialog');
  assert(await dialog.locator('[data-import-agent="dsh"]').first().isDisabled());
  assert.match(await dialog.locator('[data-import-agent="dsh"]').first().innerText(),/归档/);
  const unknown=dialog.locator('[data-import-row]').filter({hasText:'占用状态未知'}).first();assert(await unknown.isDisabled());
  const source=dialog.locator('[data-import-source]').first(),home=await source.getAttribute('data-import-source');
  await source.locator('[data-agent-ask="generic-acp"]').click();
  const refreshed=(await calls()).filter(call=>call.action.kind==='refresh');assert.equal(refreshed.length,1);assert.equal(refreshed[0].home,home);assert.equal(refreshed[0].action.agent,'generic-acp');
  await dialog.locator('[data-import-field]').scrollIntoViewIfNeeded();
  await dialog.locator('[data-import-field]').fill('项目讨论');
  const same=dialog.locator('[data-import-agent="claude-acp"]');assert((await same.count())>=4,'Identical titles preserve independent native identities');
  const identity=JSON.parse(await same.first().getAttribute('data-import-row'));
  const tabsBefore=await page.evaluate(()=>localStorage.getItem('khor.sessions.tabs'));
  await page.screenshot({path:path.join(report,name+'-same-title.png')});
  const layout=await dialog.evaluate(el=>({width:el.clientWidth,scroll:el.scrollWidth,left:el.getBoundingClientRect().left,right:el.getBoundingClientRect().right,viewport:innerWidth}));
  assert(layout.scroll<=layout.width+1&&layout.left>=-1&&layout.right<=layout.viewport+1,'The actual import overlay fits the screen');
  await same.first().click();await dialog.waitFor({state:'detached'});
  assert.equal(await page.evaluate(()=>localStorage.getItem('khor.sessions.tabs')),tabsBefore,'Import registers history without opening or replacing a conversation tab');
  await page.locator('[data-row^="observe/lab-import-"]').first().waitFor();
  await open();
  assert.equal(await dialog.locator('[data-import-row]').evaluateAll((rows,key)=>rows.filter(row=>row.dataset.importRow===key).length,JSON.stringify(identity)),0,'Already managed history leaves the import catalog');
  await dialog.locator('[data-import-field]').fill('其他宿主运行中的任务');
  await dialog.locator('[data-import-intent="fork"]').first().click();await dialog.waitFor({state:'detached'});
  await page.locator('[data-chat-scroll]:visible').filter({hasText:'独立分支'}).waitFor();
  assert.match(await page.locator('[data-chat-scroll]:visible').innerText(),/独立分支/);
  const applied=(await calls()).filter(call=>call.action.kind==='apply');assert.equal(applied.length,2);assert.deepEqual(applied.map(call=>call.action.intent),['import','fork']);
  assert.notEqual(applied[0].action.token,applied[1].action.token);
  await page.screenshot({path:path.join(report,name+'-branch.png')});
 }catch(error){errors.push(String(error));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}
 finally{await context.close();}
 return {name,errors};
}
