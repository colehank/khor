import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkWorkspaceBack(browser,{port,report}){
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 const page=await context.newPage(),errors=[];page.on('pageerror',error=>errors.push(error.message));
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=workspaces&machines=3&items=3`);
  await page.locator('[data-ws-row]:visible').first().click();
  await page.locator('[data-workspace-navigation]:visible').waitFor();
  await page.waitForTimeout(350);
  const touch=async(type,x)=>page.evaluate(({type,x})=>{
   const host=document.querySelector('[data-swipe-host]');
   const point=new Touch({identifier:1,target:host,clientX:x,clientY:360,pageX:x,pageY:360,screenX:x,screenY:360});
   host.dispatchEvent(new TouchEvent(type,{bubbles:true,cancelable:true,touches:type==='touchend'?[]:[point],changedTouches:[point]}));
  },{type,x});
  await touch('touchstart',4);await touch('touchmove',184);
  const geometry=await page.evaluate(()=>{
   const front=document.querySelector('[data-screen-front]'),head=document.querySelector('[data-side-l0]'),screen=document.querySelector('[data-swipe-host] > [data-screen]');
   return {swiping:document.documentElement.hasAttribute('data-swiping'),shared:!!front&&front.contains(head)&&front.contains(screen),
    surface:front&&getComputedStyle(front).backgroundColor,front:front?.getBoundingClientRect().x,head:head.getBoundingClientRect().x,screen:screen.getBoundingClientRect().x,
    nestedTransform:getComputedStyle(head).transform};
  });
  assert(geometry.swiping&&geometry.shared,'A back gesture must carry the title and document on the same page');
  assert.equal(geometry.front,geometry.head);assert.equal(geometry.head,geometry.screen,'Title and document cannot diverge during a swipe');
  assert.equal(geometry.nestedTransform,'none','The title is not transformed again inside its moving page');
  assert(geometry.surface&&!['transparent','rgba(0, 0, 0, 0)'].includes(geometry.surface),'The moving glass title has its own opaque page below it');
  await page.screenshot({path:path.join(report,'workspace-back-partial.png')});
  await touch('touchend',184);
  await page.locator('[data-screen="list"] [data-ws-row]:visible').first().waitFor();
  assert.equal(await page.locator('[data-workspace-navigation]:visible').count(),0,'The previous workspace header leaves with its page');
  await page.locator('[data-ws-row]:visible').first().click();await page.locator('[data-workspace-navigation]:visible').waitFor();
  await page.locator('[data-side-l0] [data-back]:visible').click();
  await page.locator('[data-screen="list"] [data-ws-row]:visible').first().waitFor();
  assert.equal(await page.locator('[data-workspace-navigation]:visible').count(),0);
  assert.deepEqual(errors,[]);
  return {name:'workspace mobile back: one opaque page, gesture and button',passed:true,geometry,errors};
 }catch(error){errors.push(String(error));await page.screenshot({path:path.join(report,'workspace-back-failure.png')}).catch(()=>{});return {name:'workspace mobile back',passed:false,errors};}
 finally{await context.close();}
}
