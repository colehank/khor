import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkSessionAttention(browser, { port, report, engine, width }) {
  const context = await browser.newContext({ viewport: { width, height: 900 } });
  const page = await context.newPage(); page.setDefaultTimeout(20000);
  const errors = []; page.on('pageerror', error => errors.push(String(error)));
  const api = (command, body = {}) => page.evaluate(async ({command, body}) => {
    const response = await fetch('/api/'+command, {method:'POST',body:JSON.stringify(body)});
    if (!response.ok) throw Error(await response.text());
    return response.json();
  }, {command, body});
  try {
    await page.goto(`http://127.0.0.1:${port}/app/?scene=list&items=5&machines=3`);
    const list = page.locator('[data-list] [data-side-list]').first();
    await list.locator('[data-side-row]').first().waitFor();
    const ids = () => list.locator('[data-side-row]').evaluateAll(rows => rows.map(row => row.dataset.sideRow));
    const initial = await ids(); assert(initial.length >= 5);
    const [pinned, one, two, three, four] = initial;
    for (const id of initial) await api('__ai_preview/session-state', {id,word:'idle'});
    await api('__ai_preview/session-state', {id:two,word:'busy'});
    await api('__ai_preview/session-state', {id:three,word:'errored'});
    await api('session_draft_edit', {revision:{id:'attention-seed',key:(four.startsWith('wide/')?'khor:':'gui:')+four,text:'unsubmitted draft',at_ms:Date.now()},parents:[]});
    await page.waitForFunction(({three,two}) => {
      const rows=[...document.querySelectorAll('[data-list] [data-side-row]')].map(row=>row.dataset.sideRow);
      return rows.indexOf(three)<rows.indexOf(two);
    },{three,two});
    const draft = list.locator(`[data-side-row="${four}"] [data-session-draft]`);
    await draft.waitFor();
    assert.equal(await draft.textContent(), '[草稿] unsubmitted draft');
    assert.equal((await ids())[0],pinned,'A pinned row remains above ordinary attention');
    const before = await ids();
    const target = list.locator(`[data-side-row="${one}"]`);
    await target.scrollIntoViewIfNeeded();
    const bounds = await target.boundingBox();
    await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);await page.mouse.down();
    await api('__ai_preview/session-state',{id:one,word:'blocked'});
    await page.waitForTimeout(2600);
    assert.deepEqual(await ids(),before,'An update cannot move a row out from under a held pointer');
    await page.mouse.move(1,899);
    await page.mouse.up();
    await page.waitForFunction(({one}) => document.querySelectorAll('[data-list] [data-side-row]')[1]?.getAttribute('data-side-row')===one,{one});
    await page.screenshot({path:path.join(report,`attention-${engine}-${width}.png`)});
    // Exercise the real producer boundary, not just the editor model: upload
    // preparation yields while the user continues typing into this composer.
    await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&items=3&machines=1`);
    const input = page.locator('[data-chat-input]:visible'); await input.waitFor();
    await input.fill('message before upload');
    await api('__ai_preview/upload-mode',{mode:'delay'});
    await page.locator('input[type=file]').last().setInputFiles({name:'attention-race.bin',mimeType:'application/octet-stream',buffer:Buffer.from('isolated upload race')});
    await page.waitForFunction(()=>{const send=document.querySelector('[data-chat-send]');return send&&!send.disabled;});
    await page.locator('[data-chat-send]').click();
    await input.fill('draft typed while uploading');
    await page.waitForFunction(async()=>{
      const calls=await(await fetch('/api/__ai_preview/says',{method:'POST',body:'{}'})).json();
      return calls.some(call=>String(call.text).startsWith('message before upload'));
    });
    assert.equal(await input.inputValue(),'draft typed while uploading','The old send never clears text entered during attachment preparation');
    await page.waitForFunction(async()=>{
      const drafts=await(await fetch('/api/session_drafts',{method:'POST',body:'{}'})).json();
      return drafts.some(draft=>draft.text==='draft typed while uploading')&&!drafts.some(draft=>draft.text==='message before upload');
    });
    assert.equal(await input.inputValue(),'draft typed while uploading','A commit receipt preserves the newer editor revision');
    await page.screenshot({path:path.join(report,`attention-upload-${engine}-${width}.png`)});
    assert.deepEqual(errors,[]);
    return {name:`${engine} ${width}: draft preview, shared attention, held-pointer stability and upload/commit race`,passed:true,errors};
  } finally { await context.close(); }
}
