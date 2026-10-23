// Isolated Lab, real App/ChatView/MachineChat; never connects to a node.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795';
const shots=process.env.LAYOUT_LAB_SHOTS??'/Volumes/disk0/khor-gui-session-layout-proof';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:true});
const errors=[];
let checks=0;
try{
  const page=await browser.newPage({viewport:{width:390,height:844}});
  page.on('pageerror',e=>errors.push(e.message));
  async function open(session,theme='light',keyboard=false,width=390,height=844,deck='top'){
    await page.setViewportSize({width,height});
    const q=new URLSearchParams({bridge:new URL(base).port,labSession:session,theme,keyboard:keyboard?'1':'0',phone:width<760?'1':'0',deck});
    await page.goto(`${base}/?${q}`);
    await page.locator('[data-composer-text]').waitFor();
    if(width<760)await page.locator(`[data-session-screen][data-deck-at="${deck}"]`).waitFor();
    await page.waitForFunction(()=>document.querySelector('[data-composer-text]')?.getBoundingClientRect().height>0);
  }
  for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all']){
    for(const theme of ['light','dark'])for(const keyboard of [false,true]){
      await open(session,theme,keyboard);
      const deck=await page.locator('[data-session-deck]').boundingBox();
      const thread=await page.locator('[data-session-screen] > [data-session-content]').boundingBox();
      const composer=await page.locator('[data-session-composer]').boundingBox();
      assert.equal(Math.round(deck.y),47,'deck starts after status safe area');
      assert(deck.y+deck.height<=thread.y+1,'navigation is above thread');
      assert(thread.y+thread.height<=composer.y+1,'composer has its own bottom room');
      const visibleHeight=await page.evaluate(()=>window.visualViewport.height);
      assert(Math.abs(composer.y+composer.height-visibleHeight)<1,'composer follows visible viewport');
      const text=page.locator('[data-composer-text]');await text.fill('检查输入框的位置');
      assert.equal(await text.inputValue(),'检查输入框的位置');
      await text.fill('第一行\n第二行\n第三行\n第四行');
      const raised=await page.locator('[data-session-composer]').boundingBox();
      assert(Math.abs(raised.y+raised.height-visibleHeight)<1,'multiline composer grows toward thread');
      await text.fill('');
      await page.locator('[data-chat-scroll], [data-machine-chat-scroll]').evaluate(el=>{el.scrollTop=0;el.dispatchEvent(new Event('scroll'));});
      assert.equal(await page.locator('[data-deck-strip]').count(),0,'top deck stays visible when reading history');
      await page.screenshot({animations:'disabled',path:`${shots}/${session==='wide/all'?'khor':session.includes('codex')?'codex':'claude'}-${theme}-${keyboard?'keyboard':'rest'}.png`});
      checks++;
    }
  }
  for(const [width,height] of [[430,932],[667,375]])for(const keyboard of [false,true]){
    await open('gui/codex-review','dark',keyboard,width,height);
    const deck=await page.locator('[data-session-deck]').boundingBox(),footer=await page.locator('[data-session-composer]').boundingBox();
    assert(deck.y+deck.height<=footer.y,'header and composer do not overlap');
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'no page overflow');checks++;
  }
  // Real pointer events: down-pull has room below a top pill and opens tabs.
  await open('gui/ai-elements-review');
  const band=page.locator('[data-session-deck] [data-deck-band]');
  let b=await band.boundingBox(),x=b.x+b.width/2,y=b.y+b.height/2;
  await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y+65,{steps:5});
  await page.locator('[data-deck-selector][data-below]').waitFor();
  const selector=await page.locator('[data-deck-selector]').boundingBox();
  assert(selector.y>=b.y+b.height-1&&selector.height>400,'top selector opens into visible content');
  const target=await page.locator('[data-deck-overview-target]').boundingBox();
  assert(target.y>=b.y+b.height&&target.y+target.height<844,'overview hint is below pill');
  await page.screenshot({path:`${shots}/down-pull.png`});await page.mouse.up();
  await page.locator('[data-tab-sheet], .mobile-content-overlay').first().waitFor();checks++;
  // Held steering uses the same selection engine; cancel does not open a tab.
  await open('gui/ai-elements-review','dark');b=await band.boundingBox();x=b.x+b.width/2;y=b.y+b.height/2;
  await page.mouse.move(x,y);await page.mouse.down();
  await page.locator('[data-deck-band][data-band-steer-phase="steering"]').waitFor();
  await page.mouse.move(x+60,y,{steps:4});
  await page.screenshot({path:`${shots}/held-selector.png`});
  await page.keyboard.press('Escape');await page.mouse.up();
  assert.equal(await page.locator('[data-deck-selector]').count(),0);checks++;
  // Old placement remains available for review, including its upward selector.
  await open('gui/ai-elements-review','light',false,390,844,'bottom');
  const old=await page.locator('[data-session-deck]').boundingBox();assert(old.y>700);checks++;
  for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all']){
    await open(session,'light',false,1280,800);
    assert.equal(await page.locator('[data-session-screen]').count(),0,'wide App keeps its existing top tabs');
    assert((await page.locator('[data-composer-text]').boundingBox()).y>600,'wide composer remains at bottom');checks++;
  }
  await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);
  await page.frameLocator('iframe').locator('[data-session-screen][data-deck-at="top"]').waitFor();
  await page.screenshot({path:`${shots}/lab.png`,fullPage:true});
  await page.getByLabel('会话',{exact:true}).selectOption('wide/all');
  await page.frameLocator('iframe').locator('[data-machine-chat]').waitFor();
  await page.getByLabel('模拟键盘升起').check();
  await page.frameLocator('iframe').locator('[data-lab-keyboard]').waitFor();checks++;
  assert.deepEqual(errors,[]);console.log(`${checks} Lab layout / pointer / keyboard / session checks passed. Screenshots: ${shots}`);
}finally{await browser.close();}
