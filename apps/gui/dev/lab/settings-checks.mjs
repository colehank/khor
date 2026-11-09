import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkSettingsNavigation(browser,{port,report,mobile=false,engine}) {
  const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900},isMobile:mobile,hasTouch:mobile});
  const page=await context.newPage();page.setDefaultTimeout(15000);
  const errors=[];page.on('pageerror',error=>errors.push(String(error)));
  const back=()=>page.locator('[data-side-l0] [data-more-back]:visible').click();
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
    assert.deepEqual(await page.locator('[data-nav-menu] [role="menuitem"]').evaluateAll(items=>items.map(item=>item.hasAttribute('data-nav-me')?'me':item.getAttribute('data-more-landing')??'more')),['me','browser','more']);
    await page.locator('[data-nav-menu]').evaluate(async node=>{
      await Promise.all(node.getAnimations({subtree:true}).map(animation=>animation.finished.catch(()=>{})));
    });
    const marks=await page.locator('[data-nav-menu] [role="menuitem"] svg').evaluateAll(items=>items.map(item=>{
      const style=getComputedStyle(item),rect=item.getBoundingClientRect();
      return {width:style.width,height:style.height,paintedWidth:rect.width,paintedHeight:rect.height};
    }));
    assert.ok(marks.length===3&&marks.every(mark=>mark.width===marks[0].width&&mark.height===marks[0].height&&mark.paintedWidth>0&&Math.abs(mark.paintedWidth-marks[0].paintedWidth)<0.5&&Math.abs(mark.paintedHeight-marks[0].paintedHeight)<0.5),`My Khor matches other chooser glyphs: ${JSON.stringify(marks)}`);
    await page.screenshot({path:path.join(report,`navigation-chooser-${engine}-${mobile?'mobile':'desktop'}.png`)});
    await page.locator('[data-nav-menu] [data-more-landing="browser"]').click();
    await page.locator('[data-nav-menu]').waitFor({state:'detached'});
    await page.locator('[data-browse-tabs]:visible').waitFor();
    assert.equal(await page.locator('[data-more-list]:visible').count(),0,'Browser has its own list outside More');
    await open();
    // A swipe underlay is visible behind the current detail but inert.
    const more=page.locator('[data-swipe-host] [data-more-list]:visible,[data-side] [data-more-list]:visible');
    const inventory=()=>more.locator('[data-more-open]').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-more-open')));
    const navigation=page.locator('[data-settings-navigation]:visible');
    await navigation.waitFor();
    assert.equal(await navigation.locator('[data-more-open]').count(),8,'More has the exact grouped destinations without a separate Terminal or Notifications entry');
    assert.equal(await more.locator('[data-more-open="me"]').count(),0,'My Khor belongs only to the overflow chooser');
    const original=await inventory();
    assert.deepEqual(original,['session','workspace','appearance','connection','security','store','doctor','about']);
    assert.equal(await more.locator('[data-more-landing]').count(),0,'Hidden landings are not More utilities');
    assert.equal(await page.locator('[data-nav-menu]').count(),0,'Choosing More dismisses the navigation chooser');
    if(!mobile)await page.locator('[data-more-page="session"]:visible').waitFor();
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
      await back();
      await navigation.waitFor();
      await page.locator('[data-nav-item="devices"]:visible').click();
    } else {
      assert.equal(await page.locator('[data-more-close]:visible').count(),0,'More has no redundant close button');
      assert.equal(await page.locator('[data-main] [data-page-title]').count(),0,'The selected sidebar row identifies the page without an empty title');
      assert.equal(await page.locator('[data-main] [data-region-head]').count(),0,'Headerless More does not reserve an empty toolbar');
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
      if(mobile&&!await more.isVisible()){await back();await more.waitFor();}
      await more.locator(`[data-more-open="${key}"]`).click();
      await page.locator(`[data-more-page="${key}"]:visible`).waitFor();
      if(!mobile)assert.deepEqual(await inventory(),original,'Every More page keeps the same navigation');
    };
    await pick('session');
    assert.equal(await page.locator('[data-settings-child="terminal"]').count(),0);
    await page.locator('[data-terminal-settings]').waitFor();
    assert.equal(await page.locator('[data-more-page="terminal"]').count(),0);
    assert.equal(await page.locator('[data-terminal-appearance-preview] [data-term-theme-preview]').count(),1);
    await page.locator('[data-terminal-appearance-preview]').scrollIntoViewIfNeeded();
    const preview=await page.locator('[data-terminal-appearance-preview] [data-term-theme-preview]').boundingBox();
    assert.ok(preview.height>100,'The actual terminal preview retains its height in the scroll body');
    assert.equal(await page.locator('[data-terminal-appearance-preview] .xterm-helper-textarea').evaluate(el=>getComputedStyle(el).opacity),'0','The preview owns its renderer stylesheet even before any terminal tab is opened');
    if(!mobile)assert.equal(await more.locator('[data-more-open="session"]').getAttribute('aria-current'),'page');
    await page.screenshot({path:path.join(report,`settings-terminal-${engine}-${mobile?'mobile':'desktop'}.png`)});
    // These are the production controls and real renderer, not a picture of them.
    await page.locator('[data-term-size-number]').fill('18');
    await page.locator('[data-term-size-number]').blur();
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.__xterm?.options.fontSize===18);
    await page.locator('[data-term-cursor-color]').fill('#aabbcc');
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.__xterm?.options.theme.cursor==='#aabbcc');
    await page.locator('[data-term-cursor-auto]').click();
    await page.locator('[data-setting-select="term-theme"]').selectOption('custom');
    assert.equal(await page.locator('[data-term-theme-custom] button').count(),2,'Custom theme exposes Import and Settings only');
    await page.locator('[data-term-theme-edit]').click();
    const dialog=page.locator('[data-term-theme-editor]');await dialog.waitFor();
    await dialog.evaluate(async el=>{await Promise.all(el.getAnimations({subtree:true}).map(a=>a.finished.catch(()=>{})));});
    const geometry=await dialog.evaluate(el=>{
      const preview=el.querySelector('[data-term-theme-preview]').getBoundingClientRect();
      const section=el.querySelector('.term-theme-grid').getBoundingClientRect();
      const body=el.querySelector('[data-slot="dialog-body"]');
      return {height:preview.height,bottom:preview.bottom,formTop:section.top,body:!!body};
    });
    assert.ok(geometry.height>100&&geometry.bottom<=geometry.formTop,'The palette preview cannot shrink or overlap colour fields');
    await page.screenshot({path:path.join(report,`settings-palette-${engine}-${mobile?'mobile':'desktop'}.png`)});
    await page.keyboard.press('Escape');await dialog.waitFor({state:'detached'});

    for(const key of ['doctor','store'])await pick(key);
    if(mobile)await back();
    await page.locator('[data-landing="devices"]:visible').click();
    await page.locator('[data-rail-more]:visible,[data-nav-more]:visible').click();
    await page.locator('[data-nav-me]:visible').click();
    await page.locator('[data-nav-menu]').waitFor({state:'detached'});
    await page.locator('[data-more-page="me"][data-dial-view="map"]:visible').waitFor();
    assert.equal(await page.locator('[data-network-profile]').count(),0,'My Khor opens the original map without a redundant profile');
    assert.equal(await page.locator('[data-dial-segment] [data-dial-view="overview"]').count(),0);
    await page.screenshot({path:path.join(report,`more-map-${engine}-${mobile?'mobile':'desktop'}.png`)});
    if(!mobile) {
      assert.equal(await page.locator('[data-dial-top]').count(),1,'Standalone My Khor owns one header');
      assert.equal(await page.locator('[data-side=""]:visible').count(),0,'Standalone My Khor has no More sidebar');
    }
    assert.equal(await more.count(),0,'My Khor does not include More navigation');
    await page.locator('[data-dial-back]:visible').click();
    await page.waitForFunction(()=>!document.querySelector('[data-dial-page]'));
    assert.equal(await more.count(),0,'My Khor returns to its original landing');
    await open();
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
