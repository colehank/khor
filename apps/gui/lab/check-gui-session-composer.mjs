import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/composer';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:true});
try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
  await page.request.get(`${base}/__ai_preview/reset`);
  const open=async(session='gui/ai-elements-review',extra={})=>{
    const {width=390,height=844,...q}=extra;await page.setViewportSize({width,height});
    await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:session,phone:width<760?'1':'0',...q})}`);
    await page.locator('[data-composer-text]').waitFor();await page.locator('[data-lab-session-settings]').waitFor();
  };
  for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all'])for(const theme of ['light','dark'])for(const keyboard of ['0','1']){
    await open(session,{theme,keyboard});
    assert.equal(await page.locator('[data-chat-box] [data-chat-mode-menu], [data-chat-box] [data-chat-who]').count(),0,'no session settings inside input');
    assert.equal(await page.locator('[data-composer-text]').getAttribute('placeholder'),'发消息…');
    assert.equal(await page.locator('[data-send-into]').count(),0,'no upload directory without attachments');
    assert.equal(await page.locator('[data-session-composer] [data-khor-agent-settings]').count(),0,'no detached model row');
    const header=await page.locator('[data-lab-session-settings]').boundingBox(),composer=await page.locator('[data-chat-box]').boundingBox();assert(header.y+header.height<composer.y);
    await page.locator('[data-composer-text]').fill('第一行\n第二行\n第三行');
    await page.locator('[data-chat-send]').waitFor();assert((await page.locator('[data-chat-box]').boundingBox()).width>=340);
    await page.locator('[data-composer-text]').fill('');
    await page.screenshot({animations:'disabled',path:`${shots}/${session==='wide/all'?'khor':session.includes('codex')?'codex':'claude'}-${theme}-${keyboard}.png`});checks++;
  }
  await open();await page.locator('[data-lab-session-settings]').click();
  await page.locator('[data-lab-option="model"]').click();await page.getByRole('menuitemradio',{name:'Opus 4.6',exact:true}).click();await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.waitForFunction(()=>document.querySelector('[data-lab-session-settings]')?.textContent.includes('Opus 4.6'));
  await page.locator('[data-lab-session-settings]').click();await page.getByRole('menuitem',{name:/思考方式/}).click();await page.locator('[data-option-value="deep"]').click();await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.locator('[data-lab-session-settings]').click();await page.getByRole('menuitem',{name:/执行权限/}).click();await page.locator('[data-option-value="auto"]').click();await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.locator('[data-lab-execution-badge]').waitFor();
  await page.locator('[data-lab-session-settings]').click();await page.getByRole('menuitem',{name:'模型详情',exact:true}).click();
  await page.screenshot({animations:'disabled',path:`${shots}/settings.png`});await page.keyboard.press('Escape');checks++;
  // Upload directory can be chosen before actual attachment upload starts.
  await page.locator('[data-lab-add]').click();await page.locator('[data-lab-attachments]').waitFor();
  await page.screenshot({animations:'disabled',path:`${shots}/add.png`});await page.locator('[data-send-into-change]').click();
  await page.getByRole('dialog').last().waitFor();await page.keyboard.press('Escape');
  await page.locator('[data-composer-text]').click();await page.locator('[data-lab-attachments]').waitFor({state:'hidden'});
  await page.locator('[data-chat-file-input]').setInputFiles({name:'layout.md',mimeType:'text/markdown',buffer:Buffer.from('# example')});
  await page.locator('[data-chat-pending-file="layout.md"][data-placed]').waitFor();
  await page.locator('[data-send-into]').waitFor();await page.screenshot({animations:'disabled',path:`${shots}/attachment.png`});
  await page.locator('[data-chat-pending-drop]').click();await page.locator('[data-send-into]').waitFor({state:'hidden'});checks++;
  // Actual App event path: send starts the fixture turn, stop ends it.
  await page.locator('[data-composer-text]').fill('演示一轮回复');await page.locator('[data-chat-send]').click();
  await page.locator('[data-chat-box] [data-chat-stop]').waitFor();await page.locator('[data-chat-box] [data-chat-stop]').click();
  await page.locator('[data-chat-box] [data-chat-stop]').waitFor({state:'hidden'});checks++;
  await open('wide/all');await page.locator('[data-lab-session-settings]').click();
  await page.locator('[data-lab-option="model"]').waitFor();
  await page.screenshot({animations:'disabled',path:`${shots}/khor-settings.png`});
  await page.locator('[data-lab-option="model"]').click();await page.getByRole('menuitemradio',{name:'Opus 4.6',exact:true}).click();await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.waitForFunction(()=>document.querySelector('[data-lab-session-settings]')?.textContent.includes('Opus 4.6'));
  await page.locator('[data-lab-session-settings]').click();await page.getByRole('menuitemcheckbox',{name:'自动压缩'}).click();
  await page.waitForFunction(()=>document.querySelector('[role="menuitemcheckbox"]')?.getAttribute('aria-checked')==='false');
  await page.keyboard.press('Escape');
  await page.locator('[data-composer-text]').fill('检查网络');await page.locator('[data-chat-send]').click();
  await page.locator('[data-chat-box] [data-chat-stop]').waitFor();await page.locator('[data-chat-box] [data-chat-stop]').click();
  await page.locator('[data-chat-box] [data-chat-stop]').waitFor({state:'hidden'});checks++;
  for(const width of [320,667,1280]){
    await open('gui/codex-review',{width,height:width===667?375:844,theme:'dark'});
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'no horizontal page overflow');
    const header=await page.locator('[data-lab-session-settings]').boundingBox();assert(header.y<110,'model stays in top navigation');
    await page.locator('[data-lab-session-settings]').click();const panel=await page.locator('[data-lab-preferences]').boundingBox();assert(panel.x>=0&&panel.x+panel.width<=width+1);
    await page.screenshot({animations:'disabled',path:`${shots}/width-${width}.png`});checks++;
  }
  // The side-by-side controls are functional, including the example attachment.
  await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);
  const frame=page.frameLocator('iframe');await frame.locator('[data-composer-text]').waitFor();
  await page.screenshot({animations:'disabled',path:`${shots}/lab.png`,fullPage:true});await page.getByLabel('附带示例文件').check();
  await frame.locator('[data-chat-pending-file="设计说明.md"][data-placed]').waitFor();
  await page.getByLabel('对比原来的输入区').check();await frame.locator('[data-chat-box] [data-chat-who]').waitFor();checks++;
  assert.deepEqual(errors,[]);console.log(`${checks} unified composer layout and interaction checks passed. Screenshots: ${shots}`);
}finally{await browser.close();}
