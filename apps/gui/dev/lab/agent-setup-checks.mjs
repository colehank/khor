import assert from 'node:assert/strict';
import {checkMobileNavigation} from './navigation-checks.mjs';

export async function checkAgentSetup(page,mobile,capture){
  if(mobile)await checkMobileNavigation(page,'terminal',2,async()=>{},'create');
  else await page.locator('[data-session-tabs] [data-tab-new]:visible').first().click();
  const body=page.locator('[data-new-session-page]:visible');await body.waitFor();
  const launch=body.locator('[data-new-session-launch] textarea');
  const select=async label=>{await launch.click();await page.getByRole('option').filter({hasText:label}).click();};
  const read=async()=>{
    const setup=body.locator('[data-acp-setup]');await setup.waitFor();
    if(!await setup.evaluate(el=>el.open))await setup.locator('summary').click();
    await setup.getByRole('button',{name:'读取智能体能力',exact:true}).click();
    return setup;
  };
  const machine=await body.locator('[data-new-session-machines]').getAttribute('data-machine');
  for(const name of ['Codex','Claude']){
    await select(name+' · 对话');const setup=await read();
    await setup.locator('[data-acp-setup-ready]').waitFor();
    assert.match(await setup.innerText(),new RegExp('已连接 '+name));
    assert.match(await setup.innerText(),new RegExp('登录由此机器上的 '+name+' 管理'));
    assert.equal(await setup.locator('[role=alert]').count(),0);
  }
  await select('Codex · 终端');assert.equal(await body.locator('[data-acp-setup]').count(),0,'Native terminal never sends ACP setup');
  await select('Codex（ACP 扩展） · 对话');
  await page.evaluate(()=>fetch('/api/__ai_preview/acp-setup',{method:'POST',body:JSON.stringify({fail:true})}));
  let setup=await read();await setup.locator('[role=alert]').waitFor();
  assert.equal(await setup.locator('[role=alert]').innerText(),'此机器的 npm 缓存不可用，修复后重试');
  await capture('setup-failure');
  await page.evaluate(()=>fetch('/api/__ai_preview/acp-setup',{method:'POST',body:JSON.stringify({fail:false})}));
  setup=await read();await setup.locator('[data-acp-setup-ready]').waitFor();
  assert.equal(await setup.locator('[role=alert]').count(),0,'Retry clears the prior error');
  const calls=await page.evaluate(async()=>await(await fetch('/api/__ai_preview/acp-setup-calls',{method:'POST',body:'{}'})).json());
  assert.deepEqual(calls.map(call=>call.agent),['codex','claude','codex-acp','codex-acp']);
  assert(calls.every(call=>call.machine===machine&&call.operation==='acp_initialize'),'Setup follows the selected execution machine');
  assert(await body.evaluate(el=>el.scrollWidth<=el.clientWidth),'The actual new session page must fit its viewport');
  await capture('setup-ready');
}
