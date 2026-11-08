// Complete production App via the Lab data boundary, with a real xterm and pointer.
// Only disposable fixture terminals receive bytes; no PTY or user session is used.
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,mkdirSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium,webkit} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),output=process.env.TERMINAL_LINK_UI_DIR;
assert(output);mkdirSync(output,{recursive:true});
const scratch=mkdtempSync(join(gui,'.terminal-app-check-'));
process.env.KHOR_LAB_VITE_PORT='23836';process.env.KHOR_LAB_RUNTIME=join(scratch,'runtime.json');
writeFileSync(process.env.KHOR_LAB_RUNTIME,JSON.stringify({mode:'frontend',scene:'terminal',tree:'terminal-link-check',revision:'one'}));
let server;const results=[];
try {
 server=await createServer({root:gui,configFile:join(gui,'dev/lab/vite.config.ts'),logLevel:'error'});await server.listen();
 for(const [engine,type] of [['chromium',chromium],['webkit',webkit]]) {
  const browser=await type.launch({headless:true});
  try {
   const context=await browser.newContext({viewport:{width:1400,height:900}}),page=await context.newPage();page.setDefaultTimeout(12000);
   await page.addInitScript(()=>{window.__linkEvents=[];window.addEventListener('khor:open-url',event=>window.__linkEvents.push(event.detail));});
   await page.route('**/terminal-target*',route=>route.fulfill({contentType:'text/html',body:'<p>Fixture destination</p>'}));
   await page.goto('http://127.0.0.1:23836/?scene=terminal&machines=3');
   await page.waitForFunction(()=>document.querySelector('[data-terminal]')?.__xterm?.buffer.active.getLine(0)?.translateToString().includes('layout-lab'));
   for(const tracking of ['none','vt200','any'])for(const kind of ['URL','OSC8'])for(const modifier of [null,'Control','Meta']) {
    // Reload gives each activation a fresh, genuinely selected session. The
    // production App receives the event and owns the resulting content tab.
    await page.reload();await page.waitForFunction(()=>document.querySelector('[data-terminal]')?.__xterm?.buffer.active.getLine(0)?.translateToString().includes('layout-lab'));
    const uri='http://127.0.0.1:23836/terminal-target?case='+tracking+'-'+kind+'-'+modifier;
    const paint=kind==='URL'?uri:'\x1b]8;;'+uri+'\x07Open fixture\x1b]8;;\x07';
    await page.evaluate(async({tracking,paint})=>{
     const term=document.querySelector('[data-terminal]').__xterm;window.__mouseBytes=[];term.onData(bytes=>window.__mouseBytes.push(bytes));
     const mode=tracking==='none'?'\x1b[?1000l\x1b[?1003l':tracking==='vt200'?'\x1b[?1000h':'\x1b[?1003h';
     await new Promise(resolve=>term.write('\x1b[2J\x1b[H'+mode+'\x1b[?1006h'+paint,resolve));
    },{tracking,paint});
    const p=await page.evaluate(()=>{const term=document.querySelector('[data-terminal]').__xterm,b=term.element.querySelector('.xterm-screen').getBoundingClientRect();return {x:b.left+3.5*b.width/term.cols,y:b.top+.5*b.height/term.rows};});
    await page.mouse.move(p.x,p.y);await page.waitForFunction(()=>document.querySelector('[data-terminal]').__xterm._core.linkifier.currentLink);
    const bytes=await page.evaluate(()=>window.__mouseBytes.length);
    if(modifier)await page.keyboard.down(modifier);await page.mouse.down();
    await page.evaluate(async paint=>{
     const term=document.querySelector('[data-terminal]').__xterm,old=term._core.linkifier.currentLink.link;
     await new Promise(resolve=>term.write('\x1b7\x1b[1;1H\x1b[2K'+paint+'\x1b8',resolve));
     await new Promise(resolve=>{const poll=()=>term._core.linkifier.currentLink?.link&&term._core.linkifier.currentLink.link!==old?resolve():requestAnimationFrame(poll);poll();});
    },paint);
    const popup=modifier?page.waitForEvent('popup'):null;await page.mouse.up();if(modifier)await page.keyboard.up(modifier);
    if(popup){const opened=await popup;await opened.waitForURL(uri);await opened.close();assert.deepEqual(await page.evaluate(()=>window.__linkEvents),[]);}
    else {await page.waitForFunction(()=>window.__linkEvents.length===1);const events=await page.evaluate(()=>window.__linkEvents);assert.equal(events[0].url,uri);assert(events[0].machine,'session machine remains attached');await page.locator('[data-content-toolbar="page"]:visible').waitFor();}
    if(tracking!=='none')assert.equal(await page.evaluate(()=>window.__mouseBytes.length),bytes,'tracked activation is not also sent to the program');
    results.push({engine,tracking,kind,modifier,passed:true});
   }
   await page.screenshot({path:join(output,'app-'+engine+'.png')});await context.close();
  } finally {await browser.close();}
 }
} catch(error){results.push({passed:false,error:String(error)});throw error;}
finally{writeFileSync(join(output,'app-report.json'),JSON.stringify({results},null,2));await server?.close();rmSync(scratch,{recursive:true,force:true});}
console.log(JSON.stringify(results));
