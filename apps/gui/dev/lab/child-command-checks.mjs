import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkCommandTab(page,{report,name,mobile,workspace=false}) {
  const rows=page.locator(workspace?'[data-ws-sessions-list]':'[data-side-list]');
  await rows.locator('[data-child-task-row="fixture-run/fixture-command"]').click();
  assert.equal(await page.locator('[role="dialog"]:visible').count(),0,'Commands open ordinary readonly tabs, never the old task dialog');
  const reader=page.locator('[data-child-session]:visible').filter({has:page.locator('[data-child-command]')});
  await reader.locator('[data-child-command]').waitFor();
  assert.equal((await reader.locator('[data-child-command-line]').innerText()).trim(),'git status');
  assert.equal((await reader.locator('[data-child-command-stdout]').innerText()).trim(),'Working tree clean');
  assert.match(await reader.locator('[data-child-command-exit]').innerText(),/0/);
  assert.equal(await page.locator('[data-chat-input]:visible,[data-chat-send]:visible').count(),0);
  if(mobile)await page.locator('[data-session-screen][data-deck-at="bottom"] [data-session-bar]:visible').waitFor();
  else assert.equal(await page.locator('[data-session-tabs] [data-tab-open]').filter({hasText:'git status'}).count(),1);
  await page.screenshot({path:path.join(report,name+'-command-tab.png')});
  const id=await reader.getAttribute('data-child-session-ref');
  await page.locator('[data-child-session-parent]:visible').click();
  await reader.waitFor({state:'detached'});
  await page.locator('[data-machine-chat-scroll]:visible').waitFor();
  if(!mobile){
    await rows.locator('[data-child-task-row="fixture-run/fixture-command"]').click();
    await reader.locator('[data-child-command]').waitFor();
    const tab=page.locator('[data-session-tabs] [data-tab]').filter({has:page.locator('[data-tab-open]').filter({hasText:'git status'})});
    assert.equal(await tab.count(),1,'Repeated command opens focus its existing tab');
    assert.equal(await reader.getAttribute('data-child-session-ref'),id);
    await tab.locator('[data-tab-close]').click();
    await reader.waitFor({state:'detached'});
    const parent=page.locator('[data-session-tabs] [data-tab-open="wide/all"]');
    assert.equal(await parent.count(),1,'Closing a command view preserves its parent tab');
    await parent.click();
    await page.locator('[data-machine-chat-scroll]:visible').waitFor();
  }
}
