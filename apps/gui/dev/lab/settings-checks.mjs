import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkSettingsNavigation(browser,{port,report,mobile=false,engine}) {
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900},isMobile:mobile,hasTouch:mobile});
  const page=await context.newPage();page.setDefaultTimeout(15000);
  const errors=[];page.on('pageerror',error=>errors.push(String(error)));
  const open=async()=>{
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').click();
    await page.locator('[data-nav-more-page]:visible').click();
    await page.locator('[data-nav-menu]').waitFor({state:'detached'});
  };
  try {
    await page.goto(`http://127.0.0.1:${port}/app/?scene=devices&machines=3`);
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').waitFor();
    const size=page.viewportSize();
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').click();
    assert.deepEqual(await page.locator('[data-nav-menu] [role="menuitem"]').evaluateAll(items=>items.map(item=>item.getAttribute('data-more-landing')??'more')),['browser','more']);
    await page.screenshot({path:path.join(report,`navigation-chooser-${engine}-${mobile?'mobile':'desktop'}.png`)});
    await page.locator('[data-nav-menu] [data-more-landing="browser"]').click();
    await page.locator('[data-nav-menu]').waitFor({state:'detached'});
    await page.locator('[data-browse-landing]:visible').waitFor();
    assert.equal(await page.locator('[data-more-list]:visible').count(),0,'Browser has its own list outside More');
    await open();
    const more=page.locator('[data-more-list]:visible');
    const inventory=()=>more.locator('[data-more-open]').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-more-open')));
    const navigation=page.locator('[data-settings-navigation]:visible');
    await navigation.waitFor();
    assert.equal(await navigation.locator('[data-more-open]').count(),7,'Settings categories are not mixed with machines, storage or diagnostics');
    assert.equal(await more.locator('[data-more-me]').count(),1);
    const original=await inventory();
    assert.deepEqual(original,['me','appearance','terminal','session','workspace','connection','security','about','store','notifications','doctor']);
    assert.equal(await more.locator('[data-more-landing]').count(),0,'Hidden landings are not More utilities');
    assert.equal(await page.locator('[data-nav-menu]').count(),0,'Choosing More dismisses the navigation chooser');
    if(!mobile)await page.locator('[data-more-page="me"]:visible').waitFor();
    await navigation.locator('[data-more-open="workspace"]').click();
    await page.locator('[data-more-page="workspace"]:visible').waitFor();
    const picker=page.locator('[data-more-page="workspace"] select').first();
    await picker.waitFor();
    const chosen=await picker.locator('option').evaluateAll((options,current)=>options.find(option=>option.value!==current)?.value,await picker.inputValue());
    assert.ok(chosen,'The Lab exposes real configurable native choices');
    await picker.selectOption(chosen);
    await page.waitForFunction(value=>JSON.parse(localStorage.getItem('khor.lab.settings-values')??'{}')['files-click']===value,chosen);
    if(!mobile)assert.equal(await navigation.locator('[data-more-open="workspace"]').getAttribute('aria-current'),'page');
    if(mobile) {
      await page.locator('[data-more-back]:visible').click();
      await navigation.waitFor();
      await page.locator('[data-nav-item="devices"]:visible').click();
    } else {
      assert.equal(await page.locator('[data-more-close]:visible').count(),0,'More has no redundant close button');
      assert.equal(await page.locator('[data-main] [data-page-title]').first().innerText(),'','The selected sidebar row identifies the page without another title');
      await page.locator('[data-landing="devices"]:visible').click();
    }
    await page.waitForFunction(()=>!document.querySelector('[data-more-page="workspace"]')?.getClientRects().length);
    await open();
    if(mobile) {
      await navigation.waitFor();
      assert.equal(await navigation.locator('[data-more-open="workspace"]').getAttribute('aria-current'),'page');
      await navigation.locator('[data-more-open="workspace"]').click();
    }
    await page.locator('[data-more-page="workspace"]:visible').waitFor();
    const pick=async key=>{
      if(mobile){await page.locator('[data-more-back]:visible').click();await more.waitFor();}
      await more.locator(`[data-more-open="${key}"]`).click();
      await page.locator(`[data-more-page="${key}"]:visible`).waitFor();
      if(!mobile)assert.deepEqual(await inventory(),original,'Every More page keeps the same navigation');
    };
    for(const key of ['doctor','store','me'])await pick(key);
    await page.locator('[data-more-page="me"][data-dial-view="map"]:visible').waitFor();
    assert.equal(await page.locator('[data-network-profile]').count(),0,'My Khor opens the original map without a redundant profile');
    assert.equal(await page.locator('[data-dial-segment] [data-dial-view="overview"]').count(),0);
    await page.screenshot({path:path.join(report,`more-map-${engine}-${mobile?'mobile':'desktop'}.png`)});
    if(!mobile) {
      assert.equal(await page.locator('[data-dial-top]').count(),0,'My Khor shares the page header');
      assert.ok(await page.locator('[data-side]:visible').count(),'My Khor keeps the More sidebar');
      const marks=await more.locator('[data-more-glyph] svg').evaluateAll(items=>items.map(item=>({width:item.getBoundingClientRect().width,height:item.getBoundingClientRect().height})));
      assert.ok(marks.length>1&&marks.every(mark=>mark.width===marks[0].width&&mark.height===marks[0].height),'My Khor uses the same glyph dimensions as every other entry');
    }
    await pick('workspace');
    await page.reload();
    if(mobile) {
      // Mobile cold start intentionally restores the landing, not a pushed page.
      await open();
      await navigation.waitFor();
      assert.equal(await navigation.locator('[data-more-open="workspace"]').getAttribute('aria-current'),'page');
      await navigation.locator('[data-more-open="workspace"]').click();
    }
    await page.locator('[data-more-page="workspace"]:visible').waitFor();
    assert.equal(await page.locator('[data-more-page="workspace"] select').first().inputValue(),chosen,'Lab settings survive reopening and reload');
    assert.equal(context.pages().length,1,'Opening settings keeps one application surface');
    assert.deepEqual(page.viewportSize(),size,'Opening and switching settings keeps the viewport');
    await page.screenshot({path:path.join(report,`settings-${engine}-${mobile?'mobile':'desktop'}.png`)});
    assert.deepEqual(errors,[]);
    return {name:`settings-navigation-${engine}-${mobile?'mobile':'desktop'}`,errors};
  } catch(error) {
    await page.screenshot({path:path.join(report,`settings-failure-${engine}-${mobile?'mobile':'desktop'}.png`)}).catch(()=>{});
    throw error;
  } finally {await context.close();}
}
