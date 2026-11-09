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
    assert.ok(preview.height>40,'The actual terminal preview retains its rendered height in the scroll body');
    assert.equal(await page.locator('[data-terminal-appearance-preview] .xterm-helper-textarea').evaluate(el=>getComputedStyle(el).opacity),'0','The preview owns its renderer stylesheet even before any terminal tab is opened');
    if(!mobile)assert.equal(await more.locator('[data-more-open="session"]').getAttribute('aria-current'),'page');
    await page.screenshot({path:path.join(report,`settings-terminal-${engine}-${mobile?'mobile':'desktop'}.png`)});
    assert.equal(await page.locator('[data-terminal-settings] > h2').count(),0,'Only Terminal appearance and Terminal features identify the groups');
    const peerHeadings=await page.locator('[data-session-general] > h2,[data-more-section="terminal-appearance"] > h2,[data-more-section="terminal-functions"] > h2').evaluateAll(headings=>headings.map(node=>{
      const style=getComputedStyle(node);
      return {size:style.fontSize,weight:style.fontWeight,height:style.lineHeight,color:style.color,inset:style.marginLeft,padding:style.paddingLeft,gap:getComputedStyle(node.parentElement).rowGap};
    }));
    assert.equal(peerHeadings.length,3);
    assert.deepEqual(peerHeadings,[peerHeadings[0],peerHeadings[0],peerHeadings[0]],'General, Terminal appearance and Terminal features are peer headings');
    const titleEdges=await page.locator('[data-session-general],[data-more-section="terminal-appearance"],[data-more-section="terminal-functions"]').evaluateAll(sections=>sections.map(section=>{
      const textEdge=node=>{
        const walker=document.createTreeWalker(node,NodeFilter.SHOW_TEXT);
        for(let text=walker.nextNode();text;text=walker.nextNode())if(text.textContent.trim()){
          const range=document.createRange();range.selectNodeContents(text);return range.getBoundingClientRect().left;
        }
        return NaN;
      };
      return {title:textEdge(section.querySelector('[data-settings-heading]')),row:textEdge(section.querySelector('[data-setting-row]'))};
    }));
    assert.ok(titleEdges.every(edge=>Math.abs(edge.title-edge.row)<=1),`Titles align with row text without hierarchical indentation: ${JSON.stringify(titleEdges)}`);
    const sample=page.locator('[data-terminal-appearance-preview] [data-term-theme-preview]');
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.dataset.previewRows);
    const sizeToContent=await sample.evaluate(el=>{
      const t=el.__xterm,style=getComputedStyle(el),screen=el.querySelector('.xterm-screen').getBoundingClientRect();
      return {rows:t.rows,used:t.buffer.active.cursorY+1,base:t.buffer.active.baseY,extra:el.clientHeight-parseFloat(style.paddingTop)-parseFloat(style.paddingBottom)-screen.height,
        first:t.buffer.active.getLine(0).translateToString(true)};
    });
    assert.equal(sizeToContent.rows,sizeToContent.used,'Preview ends at the current prompt');
    assert.equal(sizeToContent.base,0,'Wrapping never drops the sample start');
    assert.ok(Math.abs(sizeToContent.extra)<=2,`Preview has no unused bottom rows: ${JSON.stringify(sizeToContent)}`);
    assert.match(sizeToContent.first,/khor/);
    const blocks=page.locator('[data-setting-row-name="term-block-lines"] [role="switch"]');
    if(await blocks.getAttribute('aria-checked')!=="true")await blocks.click();
    await page.waitForFunction(()=>document.querySelectorAll('[data-terminal-appearance-preview] .term-block-hair').length===2);
    const stripeBounds=await sample.evaluate(el=>{
      const t=el.__xterm,screen=el.querySelector('.xterm-screen').getBoundingClientRect(),stripe=el.querySelector('.term-block-tint').getBoundingClientRect();
      return {bottom:stripe.bottom,expected:screen.top+t.buffer.active.cursorY*screen.height/t.rows};
    });
    assert.ok(Math.abs(stripeBounds.bottom-stripeBounds.expected)<=1,'Completed block covers the final output row and ends before the current prompt');
    await page.screenshot({path:path.join(report,`settings-command-stripe-${engine}-${mobile?'mobile':'desktop'}.png`)});
    await blocks.click();
    await page.waitForFunction(()=>!document.querySelector('[data-terminal-appearance-preview] .term-block-hair'));
    const blinkSwitch=page.locator('[data-setting-row-name="term-cursor-blink"] [role="switch"]');
    if(await blinkSwitch.getAttribute('aria-checked')!=="true")await blinkSwitch.click();
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] .term-preview-cursor')?.dataset.blink==="true");
    assert.ok(await sample.locator('.term-preview-cursor').evaluate(el=>el.getAnimations().some(a=>a.playState==='running')),'Cursor blinks while a settings control owns focus');
    assert.equal(await sample.locator('.xterm-helper-textarea').evaluate(el=>document.activeElement===el),false);
    await page.locator('[data-setting-select="term-cursor"]').selectOption('bar');
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] .term-preview-cursor')?.dataset.shape==='bar');
    await page.locator('[data-setting-select="term-cursor"]').selectOption('underline');
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] .term-preview-cursor')?.dataset.shape==='underline');
    await page.locator('[data-setting-select="term-cursor"]').selectOption('block');await blinkSwitch.click();
    await sample.evaluate(el=>{el.dataset.testBellCount='0';const observer=new MutationObserver(()=>{if(el.dataset.previewBell==='true')el.dataset.testBellCount=String(Number(el.dataset.testBellCount)+1);});observer.observe(el,{attributes:true,attributeFilter:['data-preview-bell']});});
    const cue=page.locator('[data-setting-row-name="term-bell"] [role="switch"]');
    if(await cue.getAttribute('aria-checked')!=="true")await cue.click();
    await sample.click();await page.waitForFunction(()=>Number(document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]').dataset.testBellCount)>0);
    await cue.click();
    await page.waitForFunction(()=>!document.querySelector('[data-terminal-appearance-preview] [data-preview-bell="true"]'));
    const bellCount=await sample.getAttribute('data-test-bell-count');await sample.click();await page.waitForTimeout(200);
    assert.equal(await sample.getAttribute('data-test-bell-count'),bellCount,'Disabled completion cue stays silent in preview');await cue.click();

    // These are the production controls and real renderer, not a picture of them.
    await page.locator('[data-term-size-number]').fill('18');
    await page.locator('[data-term-size-number]').blur();
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.__xterm?.options.fontSize===18);
    await page.locator('[data-term-cursor-color]').fill('#aabbcc');
    await page.waitForFunction(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.__xterm?.options.theme.cursor==='#aabbcc');
    const themePicker=page.locator('[data-setting-select="term-theme"]'),originalTheme=await themePicker.inputValue();
    for(const [theme,colors] of [['latte',['#ffaaaa','#aaaaff']],['mocha',['#660000','#000066']]]) {
      await themePicker.selectOption(theme);
      const cursorPaint=[];
      for(const color of colors) {
        await page.locator('[data-term-cursor-color]').fill(color);
        await page.waitForFunction(color=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]')?.__xterm?.options.theme.cursor===color,color);
        // Wait for the renderer pass, then measure the visible decoration.
        await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
        cursorPaint.push(await sample.locator('.term-preview-cursor').evaluate(el=>getComputedStyle(el).backgroundColor));
      }
      assert.notEqual(cursorPaint[0],cursorPaint[1],`${theme}: changing cursor colour changes visible paint, not only configuration`);
    }
    await themePicker.selectOption(originalTheme);
    await page.locator('[data-term-cursor-auto]').click();
    const activePalette=await page.evaluate(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]').__xterm.options.theme.background);
    await page.locator('[data-setting-select="term-theme"]').selectOption('custom');
    assert.equal(await page.evaluate(()=>document.querySelector('[data-terminal-appearance-preview] [data-term-theme-preview]').__xterm.options.theme.background),activePalette,'Entering custom mode preserves the current terminal paint');
    assert.equal(await page.locator('[data-term-theme-custom] > span').innerText(),'自定义配色');
    assert.equal(await page.locator('[data-term-theme-custom] button').count(),2,'Custom theme exposes Import and Settings only');
    await page.locator('[data-term-theme-edit]').click();
    const dialog=page.locator('[data-term-theme-editor]');await dialog.waitFor();
    await dialog.evaluate(async el=>{await Promise.all(el.getAnimations({subtree:true}).map(a=>a.finished.catch(()=>{})));});
    const geometry=await dialog.evaluate(el=>{
      const bounds=el.getBoundingClientRect();
      const controls=[...el.querySelectorAll('.term-theme-hex,[data-term-theme-save]')].map(node=>node.getBoundingClientRect());
      const fits=bounds.left>=0&&bounds.right<=innerWidth&&controls.every(rect=>rect.left>=bounds.left&&rect.right<=Math.min(bounds.right,innerWidth));
      const preview=el.querySelector('[data-term-theme-preview]').getBoundingClientRect();
      const section=el.querySelector('.term-theme-grid').getBoundingClientRect();
      const body=el.querySelector('[data-slot="dialog-body"]');
      const node=el.querySelector('[data-term-theme-preview]'),style=getComputedStyle(node);
      const extra=node.clientHeight-parseFloat(style.paddingTop)-parseFloat(style.paddingBottom)-node.querySelector('.xterm-screen').getBoundingClientRect().height;
      return {height:preview.height,bottom:preview.bottom,formTop:section.top,body:!!body,extra,fits};
    });
    assert.ok(geometry.height>40&&geometry.bottom<=geometry.formTop,'The palette preview cannot shrink or overlap colour fields');
    assert.ok(Math.abs(geometry.extra)<=2,'Opening scale animation cannot change the fitted terminal height');
    assert.equal(geometry.fits,true,'Palette editor and every colour/action control fit the viewport');
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
    await page.goto(`http://127.0.0.1:${port}/app/?scene=list&machines=3&notices=1`);
    const notice=page.locator('[data-notification-banner]').first();
    await notice.waitFor({timeout:20000});
    const notificationGeometry=await notice.boundingBox();
    assert.ok(notificationGeometry.y<100&&notificationGeometry.x+notificationGeometry.width<=size.width,'The actual App presents notices at the top inside the viewport');
    await page.screenshot({path:path.join(report,`app-notification-${engine}-${mobile?'mobile':'desktop'}.png`)});
    await notice.locator('[data-notification-open]').click();
    await page.locator('[data-chat-box]:visible').waitFor();
    await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active==='gui/ai-elements-plan');
    const metadata=await page.evaluate(()=>JSON.parse(localStorage.getItem('khor.notification-preferences')));
    assert.equal('items' in metadata,false,'Actual App never archives notification content');
    assert.deepEqual(errors,[]);
    return {name:`settings-navigation-${engine}-${mobile?'mobile':'desktop'}`,errors};
  } catch(error) {
    await page.screenshot({path:path.join(report,`settings-failure-${engine}-${mobile?'mobile':'desktop'}.png`)}).catch(()=>{});
    throw error;
  } finally {await context.close();}
}
