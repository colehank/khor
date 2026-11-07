import assert from 'node:assert/strict';

// Exercise the complete App's real list and heading, including a content
// update during an active pull. Synthetic events cover DOM ownership only;
// Chromium input below separately exercises its native scrolling pipeline.
export async function checkListScroll(page, shot, {nativePan = true} = {}) {
  const port = page.locator('[data-list] [data-pane-body]').first();
  const facts = () => port.evaluate(el => {
    const viewport=el.parentElement;
    const css=getComputedStyle(el), surface=getComputedStyle(viewport);
    return {
      owner: el.hasAttribute('data-band-scroll'), height: el.clientHeight,
      content: el.scrollHeight, padding: css.paddingTop,
      top: el.scrollTop, pulling: !!el.closest('[data-pull-host]'),
      nativeMask: css.maskImage || css.webkitMaskImage,
      visualMask: surface.maskImage || surface.webkitMaskImage,
      visualSurface: viewport.hasAttribute('data-scroll-viewport') && viewport.hasAttribute('data-sticky-surface'),
      strength: Number(surface.getPropertyValue('--sticky-fade-strength')),
    };
  });
  await port.evaluate((el, pan) => { el.scrollTop = pan ? 220 : 0; }, nativePan);
  const before = await facts();
  assert(before.owner, 'The actual App heading must own its list scroll port');
  if (nativePan) assert(before.content > before.height + 400, 'The actual App list must have enough content to pan');
  const nativeInput = nativePan && page.context().browser().browserType().name() === 'chromium';
  const trace = [];
  const boundaryTrace = [];
  if (nativeInput) {
    const cdp = await page.context().newCDPSession(page);
    const input = async (type, y, records = trace) => {
      await cdp.send('Input.dispatchTouchEvent', {type, touchPoints: type === 'touchEnd' ? [] : [{x:200,y}]});
      await page.waitForTimeout(30);
      records.push(await facts());
    };
    try {
      await input('touchStart', 280);
      for (const y of [300,330,360,390,420,390,360,330,300,270,240,210,180]) await input('touchMove', y);
      await input('touchEnd', 180);
      await shot('native-pan', {nativeInput, trace, before});
      assert(trace.at(-1).top > before.top + 40, `A native pan must continue after reversing direction: ${JSON.stringify(trace)}`);
      assert(trace.every(step => step.owner && !step.pulling && step.height === before.height && step.content === before.content), 'A native pan must retain one stable scroll port');
      await page.waitForTimeout(500);
      await port.evaluate(el => { el.scrollTop=0; });
      await input('touchStart',280,boundaryTrace);
      for (const y of [260,230,200,170,200,230,260,290,320,290,260,230,200,170]) await input('touchMove',y,boundaryTrace);
      await input('touchEnd',170,boundaryTrace);
      await shot('fade-boundary',{boundaryTrace});
      assert(boundaryTrace.at(-1).top>40,'The same native gesture must cross rest and fade onset without losing scrolling');
      assert(boundaryTrace.every(step => step.nativeMask === 'none' && step.visualSurface && step.visualMask !== 'none'), 'Only the persistent visual viewport may carry a mask at the top boundary');
    } finally {
      await cdp.detach();
    }
  }
  // End native inertia before probing exact one-pixel positions.
  if (nativeInput) await page.waitForTimeout(500);
  const fadeSamples=[];
  for (const top of [0,1,3,52,0]) {
    await port.evaluate((el,top)=>{el.scrollTop=top;},top);
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const sample=await facts();fadeSamples.push(sample);
    assert(sample.nativeMask==='none' && sample.visualSurface && sample.visualMask!=='none','Rest and fade onset must keep the native port unmasked and the visual surface present');
  }
  if (before.content-before.height>=3) assert(fadeSamples[1].strength>0&&fadeSamples[1].strength<fadeSamples[2].strength&&fadeSamples[2].strength<1,'The first pixels must strengthen the fade continuously');
  await port.evaluate(el => { el.scrollTop = 0; });
  const rest = await facts();
  const touch = (type, y) => port.evaluate((el, {type,y}) => {
    const target = el.querySelector('[data-side-open]') ?? el;
    const point = {identifier:1,target,clientX:200,clientY:y};
    const event = new Event(type, {bubbles:true,cancelable:true});
    Object.defineProperties(event, {touches:{value:type === 'touchcancel' ? [] : [point]},changedTouches:{value:[point]}});
    target.dispatchEvent(event);
  }, {type,y});
  let pulled;
  try {
    await touch('touchstart', 250);
    await touch('touchmove', 330);
    await page.waitForSelector('[data-pull-host]');
    await port.evaluate(el => {
      // A polling/render update causes the production heading observer to
      // rediscover its scroll port. An empty text node keeps layout intact.
      el.querySelector('[data-side-title]').appendChild(document.createTextNode(''));
    });
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    pulled = await facts();
    await shot('during-pull', {rest, pulled});
    assert(pulled.owner && pulled.height === rest.height && pulled.content === rest.content && pulled.padding === rest.padding,
      `A content update during pull must preserve the actual scroll port and scrollbar geometry: ${JSON.stringify({rest,pulled})}`);
  } finally {
    await touch('touchcancel', 330);
  }
  return {nativeInput, trace, boundaryTrace, fadeSamples, rest, pulled, physicalIphone:'pending'};
}
