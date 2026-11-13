import assert from 'node:assert/strict';
import path from 'node:path';

const independentKhorSession = 'wide/agent-' + 'a'.repeat(64);

export async function checkConversationNavigation(browser,{port,report,engine,workspace=false,khor=true,width=390}) {
  const name=`conversation-navigation-${workspace?'workspace':'main'}-${khor?'khor':'acp'}-${width}-${engine}`;
  const context=await browser.newContext({viewport:{width,height:844},isMobile:true,hasTouch:true});
  const page=await context.newPage(),errors=[],geometry=[];
  page.setDefaultTimeout(15000);
  page.on('pageerror',error=>errors.push(String(error)));
  try {
    const scene=workspace?'workspace':khor?'khor':'gui';
    await page.goto(`http://127.0.0.1:${port}/app/?scene=${scene}&items=3&machines=1${workspace&&khor?'&khor=independent':''}`);
    if(workspace) {
      const back=page.locator('[data-ws-mid] [data-session-bar] [data-back]:visible');
      await back.waitFor();await back.click();
      await page.locator('[data-workspace-navigation][data-seat="workspace"] [data-workspace-view="sessions"]').click();
      await page.locator(`[data-ws-sessions-list] [data-row="${khor?independentKhorSession:'gui/ai-elements-review'}"] [data-row-open]`)
        .getByText(khor?'khor · 网络任务':'AI Elements · 组件总览',{exact:true}).click();
    }
    const frame=page.locator(workspace?'[data-ws-sessions] [data-session-screen]:visible':'[data-session-screen][data-session-navigation-root]:visible');
    await frame.locator(khor?'[data-machine-chat-scroll]':'[data-chat-scroll]').waitFor();
    await page.evaluate(async()=>{await document.fonts.ready;await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));});
    assert.equal(await frame.locator('[data-lab-model-header]:visible').count(),1,`${name}: every conversation has its top model header`);
    assert.equal(await frame.locator('[data-lab-model-header] [data-lab-session-settings]:visible').count(),1,'Model settings belong to the top header');
    assert.equal(await frame.locator('[data-deck-pill-row]:visible').count(),0,'A conversation must not fall back to the terminal bottom deck');
    await frame.locator('[data-lab-compose-avatar]:visible').waitFor();
    const measure=()=>frame.evaluate(root=>{
      const box=element=>{if(!element)return null;const r=element.getBoundingClientRect();return {top:r.top,bottom:r.bottom,left:r.left,right:r.right};};
      const header=root.querySelector('[data-lab-model-header]');
      const capsule=root.querySelector('[data-chat-box]'),avatar=root.querySelector('[data-lab-compose-avatar]');
      return {header:box(header),scroll:box(root.querySelector('[data-machine-chat-scroll],[data-chat-scroll]')),capsule:box(capsule),avatar:box(avatar),height:innerHeight,
        independentAvatar:!!avatar&&!capsule?.contains(avatar),backInHeader:!!header?.querySelector('[data-lab-model-back]'),newInHeader:!!header?.querySelector('[data-lab-new-session]')};
    });
    const check=async phase=>{
      const g=await measure();geometry.push({phase,...g});
      assert(g.header&&g.scroll&&g.capsule&&g.avatar,'The actual thread, header, composer and avatar must be mounted');
      assert(g.header.bottom<=g.scroll.top+1,'The model header precedes the conversation');
      assert(g.backInHeader&&g.newInHeader,'Back and New use the same top header');
      assert(g.independentAvatar,'The session switcher is the composer capsule’s sibling');
      assert(Math.abs(g.avatar.bottom-g.capsule.bottom)<1,'Avatar and composer bottom edges align');
      assert(g.capsule.bottom<=g.height+1&&g.header.top>=0,'Controls stay inside the visible viewport');
      await page.screenshot({path:path.join(report,`${name}-${phase}.png`)});
    };
    await check('rest');
    await frame.locator('[data-lab-session-settings]:visible').click();
    await page.locator('[data-lab-preferences]:visible').waitFor();
    await page.keyboard.press('Escape');
    await page.setViewportSize({width,height:480});
    await page.waitForFunction(()=>document.getElementById('root')?.getBoundingClientRect().bottom<=innerHeight+1);
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    await check('short-viewport');
    assert.deepEqual(errors,[]);
  } catch(error) {
    errors.push(String(error));
    await page.screenshot({path:path.join(report,`${name}-failure.png`)}).catch(()=>{});
  } finally {await context.close();}
  return {name,errors,geometry};
}
