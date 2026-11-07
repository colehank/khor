import assert from 'node:assert/strict';

export async function checkMobileNavigation(page,scene,items,capture,operation='browse') {
  const gui=scene==='gui',content=scene==='content';
  const selector=gui?'[data-lab-fan-avatar]:visible':content?'[data-content-visible="true"] [data-deck-band]:visible':'[data-session-screen] > [data-session-bar] [data-deck-band]:visible';
  const trigger=page.locator(selector).first();
  const bounds=await trigger.boundingBox();assert(bounds,'The production navigation control must be visible');
  const origin={x:bounds.x+bounds.width/2,y:bounds.y+bounds.height/2};
  const touch=await trigger.evaluate(el=>parseFloat(getComputedStyle(el).getPropertyValue('--touch')));
  const session=await page.context().newCDPSession(page);
  const send=(type,point)=>session.send('Input.dispatchTouchEvent',{type,touchPoints:point?[{...point,id:1}]:[]});
  const snapshot=()=>page.evaluate(async content=>content
    ? await (await fetch('/api/tabs_list',{method:'POST',body:JSON.stringify({kind:'content'})})).json()
    : JSON.parse(localStorage.getItem('khor.sessions.tabs')),content);
  const count=value=>value.tabs.length;
  const start=async()=>{
    await send('touchStart',origin);
    await page.locator('[data-lab-left-fan]').waitFor();
  };
  const move=async(dx,dy)=>{await send('touchMove',{x:origin.x+dx,y:origin.y+dy});};
  const action=async(distance)=>{
    await move(gui?-distance*Math.SQRT1_2:0,gui?-distance*Math.SQRT1_2:-distance);
  };
  const before=await snapshot();
  const created=()=>page.waitForFunction(async({content,n})=>{
    const state=content?await(await fetch('/api/tabs_list',{method:'POST',body:JSON.stringify({kind:'content'})})).json():JSON.parse(localStorage.getItem('khor.sessions.tabs'));
    return state.tabs.length===n+1;
  },{content,n:count(before)});
  try {
    if(operation==='create'){
      await start();await action(touch*(gui?3.5:2.2));
      await page.locator('[data-lab-fan-new][data-active]').waitFor();
      assert.deepEqual(await snapshot(),before,'Near action only previews before release');
      await send('touchEnd');await created();
      await page.waitForTimeout(300);
      assert.equal(count(await snapshot()),count(before)+1,'Pointer and touch completion create exactly one tab');
      await capture('created');return;
    }
    await start();
    if(gui){await move(-touch,-touch);await move(-touch,-2*touch);}else await move(-36,0);
    await page.waitForFunction(()=>Number(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-pulse'))>=1);
    if(items===2)await page.waitForFunction(()=>Number(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-pulse'))>=3);
    const geometry=await page.evaluate(()=>{
      const fan=document.querySelector('[data-lab-left-fan]');
      const box=el=>{const r=el.getBoundingClientRect();return {left:r.left,top:r.top,right:r.right,bottom:r.bottom};};
      const visible=[...fan.querySelectorAll('[data-lab-fan-session]')].filter(el=>Number(getComputedStyle(el).opacity)>.3);
      const titles=visible.map(el=>box(el.querySelector('[data-lab-fan-title]')));
      const identities=visible.map(box);
      const actions=[...fan.querySelectorAll('[data-lab-fan-new],[data-lab-fan-overview]')].map(box);
      const intersects=(a,b)=>a.left<b.right&&a.right>b.left&&a.top<b.bottom&&a.bottom>b.top;
      return {frame:box(fan),slots:Number(fan.dataset.fanSlots),titles,actions,identities,ids:visible.map(el=>el.getAttribute('data-lab-fan-session')),
        selected:fan.querySelectorAll('[data-selected]').length,opened:fan.querySelectorAll('[data-opened]').length,
        collision:actions.some(a=>identities.some(b=>intersects(a,b))),height:innerHeight,width:innerWidth};
    });
    await capture('following',geometry);
    assert.equal(geometry.slots,5,'The normal phone viewport retains the existing five candidate seats');
    assert.equal(geometry.selected,1,'A cyclic presentation selects exactly one slot');
    assert(geometry.opened<=1,'The actual opened identity has one marker rather than marking every cyclic copy');
    assert(geometry.titles.length>=3,'Both sides of a two-tab cycle remain visible');
    assert.equal(geometry.collision,false,'Action prompts must stay outside full candidate identities, including entering avatars');
    for(const r of [...geometry.identities,...geometry.actions])assert(r.left>=-1&&r.right<=geometry.width+1&&r.top>=0&&r.bottom<=geometry.height+1,'Arc identities and actions must fit the actual App viewport');
    assert.deepEqual(await snapshot(),before,'Following and cycling only preview; the opened state remains unchanged');
    await move(0,0);
    await page.locator('[data-lab-fan-cancel-hint]').waitFor();
    await capture('home');
    await send('touchEnd');
    await page.locator('[data-lab-left-fan]').waitFor({state:'detached'});
    assert.deepEqual(await snapshot(),before,'Returning home and releasing keeps the original session or content tab');

    await start();await action(touch*(gui?3.5:2.2));
    await page.locator('[data-lab-fan-new][data-active]').waitFor();
    assert.deepEqual(await snapshot(),before,'Near action is a preview before release');
    await page.waitForFunction(()=>{const label=document.querySelector('[data-lab-fan-new][data-active]>span');return label&&Number(getComputedStyle(label).opacity)>.99;});
    await capture('new-preview');
    await action(touch*(gui?5:4));
    await page.locator('[data-lab-fan-overview][data-active]').waitFor();
    assert.deepEqual(await snapshot(),before,'Passing new on the way to all must not create a tab');
    await page.waitForFunction(()=>{const current=document.querySelector('[data-lab-fan-overview][data-active]>span'),previous=document.querySelector('[data-lab-fan-new]>span');return current&&Number(getComputedStyle(current).opacity)>.99&&(!previous||Number(getComputedStyle(previous).opacity)<.01);});
    const actionBounds=await page.locator('[data-lab-fan-overview][data-active]').boundingBox();
    const viewport=page.viewportSize();assert(actionBounds.x>=0&&actionBounds.x+actionBounds.width<=viewport.width&&actionBounds.y>=0&&actionBounds.y+actionBounds.height<=viewport.height,'The expanded All prompt must fit after its glass transition');
    await capture('all-preview');
    await send('touchEnd');
    const rows=page.locator('.mobile-content-overview-list:visible,[data-tab-sheet]:visible');
    await rows.waitFor();
    assert(await rows.locator('[data-tab-sheet-new]').evaluate(el=>!!(el.compareDocumentPosition(el.parentElement.querySelector('[data-tab-sheet-row]'))&Node.DOCUMENT_POSITION_FOLLOWING)),'New tab is the first list action');
    assert.deepEqual(await snapshot(),before,'Opening all tabs preserves the current tab identities');
    await capture('all-tabs');
    {
      const sheet=page.locator('.mobile-content-sheet:visible,[data-tab-sheet]:visible');
      const grab=sheet.locator('[data-place-grab],[data-sheet-grabber]');
      assert.equal(await grab.count(),1,'The actual content sheet owns its expansion handle');
      const height=await sheet.evaluate(el=>el.closest('[data-session-screen]')?.clientHeight??visualViewport?.height??innerHeight);
      if(await sheet.getAttribute('data-detent')!=='full')await grab.click();
      await page.waitForFunction(height=>[...document.querySelectorAll('.mobile-content-sheet,[data-tab-sheet]')].some(el=>el.getClientRects().length&&el.getAttribute('data-detent')==='full'&&Math.abs(el.getBoundingClientRect().height-height)<=1),height);
      assert(Math.abs((await sheet.boundingBox()).height-height)<=1,'The existing highest sheet detent fills its actual session viewport');
      await capture('full-sheet');
    }
    await rows.locator('[data-tab-sheet-new]').click();
    await created();
  }finally{
    await send('touchCancel').catch(()=>{});
    await session.detach();
  }
}
