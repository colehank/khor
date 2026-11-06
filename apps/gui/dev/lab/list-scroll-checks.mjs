import assert from 'node:assert/strict';

// Exercise the complete App's real list and heading, including a content
// update during an active pull. Synthetic events cover DOM ownership only;
// Chromium input below separately exercises its native scrolling pipeline.
export async function checkListScroll(page, shot, {nativePan = true} = {}) {
  const port = page.locator('[data-list] [data-pane-body]').first();
  const facts = () => port.evaluate(el => ({
    owner: el.hasAttribute('data-band-scroll'), height: el.clientHeight,
    content: el.scrollHeight, padding: getComputedStyle(el).paddingTop,
    top: el.scrollTop, pulling: !!el.closest('[data-pull-host]'),
  }));
  await port.evaluate((el, pan) => { el.scrollTop = pan ? 220 : 0; }, nativePan);
  const before = await facts();
  assert(before.owner, 'The actual App heading must own its list scroll port');
  if (nativePan) assert(before.content > before.height + 400, 'The actual App list must have enough content to pan');
  const nativeInput = nativePan && page.context().browser().browserType().name() === 'chromium';
  const trace = [];
  if (nativeInput) {
    const cdp = await page.context().newCDPSession(page);
    const input = async (type, y) => {
      await cdp.send('Input.dispatchTouchEvent', {type, touchPoints: type === 'touchEnd' ? [] : [{x:200,y}]});
      await page.waitForTimeout(30);
      trace.push(await facts());
    };
    try {
      await input('touchStart', 280);
      for (const y of [300,330,360,390,420,390,360,330,300,270,240,210,180]) await input('touchMove', y);
      await input('touchEnd', 180);
      await shot('native-pan', {nativeInput, trace, before});
      assert(trace.at(-1).top > before.top + 40, `A native pan must continue after reversing direction: ${JSON.stringify(trace)}`);
      assert(trace.every(step => step.owner && !step.pulling && step.height === before.height && step.content === before.content), 'A native pan must retain one stable scroll port');
    } finally {
      await cdp.detach();
    }
  }
  // Let any native fling settle before testing a separate pull gesture.
  if (nativeInput) await page.waitForTimeout(500);
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
  return {nativeInput, trace, rest, pulled, physicalIphone:'pending'};
}
