import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkWorkspaceBack(browser,{port,report}){
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 const page=await context.newPage(),errors=[];page.on('pageerror',error=>errors.push(error.message));
 try{
  await page.addInitScript(()=>{
   const fetchFromBrowser=window.fetch.bind(window);
   let fetchCurrent=fetchFromBrowser;
   let opened=null;
   const metadata=new Set(['checkout','git_status','touched']);
   const timing={click:null,directory:[],metadataInFlight:0,metadataAtClick:0};
   Object.defineProperty(window,'__workspaceDirectoryTiming',{value:timing});
   document.addEventListener('click',event=>{
    if(opened!==null||!event.target?.closest?.('[data-ws-row]'))return;
    opened=performance.now();
    timing.click=opened;
    timing.metadataAtClick=timing.metadataInFlight;
   },true);
   // The Lab installs a fetch implementation for the fixture. Observe its
   // requests as well as regular browser fetch, without changing the replies.
   Object.defineProperty(window,'fetch',{
    configurable:true,
    get(){
     // The fixture captures the preceding fetch when it replaces it.
     const fetch=fetchCurrent;
     return async function(input,init){
      const url=new URL(input instanceof Request?input.url:input,location.href);
      const operation=url.origin===location.origin&&url.pathname.startsWith('/api/')?url.pathname.slice(5):null;
      const background=metadata.has(operation);
      if(background)timing.metadataInFlight++;
      const directory=opened!==null&&operation==='ls'&&typeof init?.body==='string'
       ?{path:JSON.parse(init.body).path,submitted:performance.now(),reply:null}:null;
      if(directory)timing.directory.push(directory);
      try{return await fetch(input,init);}
      finally{
       if(background)timing.metadataInFlight--;
       if(directory)directory.reply=performance.now();
      }
     };
    },
    set(fetch){fetchCurrent=fetch;},
   });
  });
  await page.goto(`http://127.0.0.1:${port}/app/?scene=workspaces&machines=3&items=3`);
  await page.locator('[data-ws-row]:visible').first().click();
  await page.locator('[data-workspace-navigation]:visible').waitFor();
  await page.locator('[data-file-tree] [data-tree-entry]:visible').first().waitFor();
  const directoryTiming=await page.evaluate(async()=>{
   const contentVisible=await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
   const timing=window.__workspaceDirectoryTiming;
   const root=[...document.querySelectorAll('[data-file-tree]')].find(tree=>tree.getClientRects().length)?.getAttribute('data-tree-root');
   const first=timing.directory.find(({path,reply})=>path===root&&reply!==null&&reply<=contentVisible);
   return first&&timing.click!==null?{
    clickToVisibleMs:contentVisible-timing.click,
    clickToSubmitMs:first.submitted-timing.click,
    submitToReplyMs:first.reply-first.submitted,
    replyToVisibleMs:contentVisible-first.reply,
    metadataInFlightAtClick:timing.metadataAtClick,
   }:null;
  });
  assert(directoryTiming&&directoryTiming.clickToSubmitMs>=0&&directoryTiming.submitToReplyMs>=0&&directoryTiming.replyToVisibleMs>=0,
   'The workspace row click must lead to a directory reply and visible entries in that order');
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
  return {name:'workspace mobile back: one opaque page, gesture and button',passed:true,geometry,directoryTiming,errors};
 }catch(error){errors.push(String(error));await page.screenshot({path:path.join(report,'workspace-back-failure.png')}).catch(()=>{});return {name:'workspace mobile back',passed:false,errors};}
 finally{await context.close();}
}
