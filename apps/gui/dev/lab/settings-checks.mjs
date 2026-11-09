import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkSettingsNavigation(browser,{port,report,mobile=false,engine}) {
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900},isMobile:mobile,hasTouch:mobile});
  const page=await context.newPage();page.setDefaultTimeout(15000);
  const errors=[];page.on('pageerror',error=>errors.push(String(error)));
  const open=async()=>{
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').click();
    await page.locator('[data-nav-settings]:visible').click();
  };
  try {
    await page.goto(`http://127.0.0.1:${port}/app/?scene=devices&machines=3`);
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').waitFor();
    const size=page.viewportSize();
    await open();
    const navigation=page.locator('[data-settings-navigation]:visible');
    await navigation.waitFor();
    assert.equal(await navigation.locator('[data-more-open]').count(),7,'Settings categories are not mixed with machines, storage or diagnostics');
    assert.equal(await navigation.locator('[data-more-me]').count(),0);
    if(!mobile)await page.locator('[data-more-page="appearance"]:visible').waitFor();
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
      await page.locator('[data-more-close]:visible').click();
    }
    await page.waitForFunction(()=>!document.querySelector('[data-more-page="workspace"]')?.getClientRects().length);
    await open();
    if(mobile) {
      await navigation.waitFor();
      assert.equal(await navigation.locator('[data-more-open="workspace"]').getAttribute('aria-current'),'page');
      await navigation.locator('[data-more-open="workspace"]').click();
    }
    await page.locator('[data-more-page="workspace"]:visible').waitFor();
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
  } finally {await context.close();}
}
