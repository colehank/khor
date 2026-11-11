// The production recent-place row, with long domain/path/machine data.
// Keep the action reachable at sidebar widths, for both pointer classes.
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,rmSync,mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium,webkit} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const fixture=mkdtempSync(join(gui,'.recent-places-check-'));
const out=process.env.RECENT_PLACES_OUT||join(process.env.RUNNER_TEMP||tmpdir(),'khor-recent-places-ui');
mkdirSync(out,{recursive:true});
writeFileSync(join(fixture,'index.html'),'<link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture,'probe.tsx'),`
import React,{useState} from 'react';import{createRoot}from'react-dom/client';
import{NewTabPage}from'/src/browse/NewTabPage';import{PAGES}from'/src/views/PagesKind';
import{TooltipProvider}from'/src/components/ui/tooltip';import'/src/styles/tokens.css';import'/src/styles/app.css';
const original=[
 {device:'a',name:'Mac-mini-长机器名称-Developer-Machine',place:'https://very-long-subdomain.example.com/a/very/long/document?query=long',at_ms:Date.now()-60000},
 {device:'b',name:'工作站-with-a-long-name',place:'/Users/developer/very-long-project-name/深层目录/README-with-a-long-name.md',at_ms:Date.now()-86400000},
];
window.opened=[];window.forgotten=[];
function Probe(){
 const [rows,setRows]=useState(original),[config,setConfig]=useState({width:264,narrow:false,kind:'page'});
 window.configureRecentProbe=(next)=>{setRows(original);window.opened=[];window.forgotten=[];setConfig(next);};
 const kind=config.kind==='page'?PAGES:{...PAGES,kind:'dir',leafOf:place=>place.split('/').pop()};
 return <TooltipProvider><section data-probe style={{width:config.width}}><NewTabPage kind={kind}
   places={{recent:rows,asked:true,reload:()=>{}}} devices={[]} narrow={config.narrow}
   onOpen={(device,place)=>window.opened.push({device,place})}
   onForget={place=>{window.forgotten.push(place);setRows(rows=>rows.filter(row=>row.place!==place.place));}}/>
 </section></TooltipProvider>;
}
createRoot(document.getElementById('root')).render(<Probe/>);`);

let server;
const results=[],failures=[];
try{
 server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:23845,strictPort:true,hmr:false},logLevel:'error'});
 await server.listen();
 for(const [engine,driver] of [['chromium',chromium],['webkit',webkit]]){
  const browser=await driver.launch({headless:true,...(engine==='chromium'?{channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined}:{})});
  try{
   for(const narrow of [false,true]){
    const context=await browser.newContext({viewport:{width:1000,height:800},hasTouch:narrow});
    const page=await context.newPage(),errors=[];page.on('pageerror',error=>errors.push(error.message));
    try{
     await page.goto(`http://127.0.0.1:23845/${fixture.split('/').pop()}/`);
     await page.locator('[data-new-tab]').waitFor();
     await page.evaluate(()=>document.fonts.ready);
     for(const kind of ['page','dir'])for(const width of [160,200,264,420,900]){
      const name=`${engine}-${narrow?'touch':'pointer'}-${kind}-${width}`;
      try{
       await page.evaluate(config=>window.configureRecentProbe(config),{width,narrow,kind});
       await page.waitForFunction(width=>document.querySelector('[data-probe]').getBoundingClientRect().width===width,width);
       const row=page.locator('[data-place]').first().locator('..');
       await row.hover();
       const geometry=await page.evaluate(()=>{
        const root=document.querySelector('[data-new-tab]'),box=el=>{const r=el.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width};};
        const rows=[...root.querySelectorAll('[data-place]')].map(open=>{
         const close=open.parentElement.querySelector('[data-forget-place]'),r=close.getBoundingClientRect();
         return {open:box(open),close:box(close),glyph:box(close.querySelector('svg')),hit:close.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))};
        });
        return {root:box(root),client:root.clientWidth,scroll:root.scrollWidth,body:document.documentElement.scrollWidth,viewport:innerWidth,rows};
       });
       assert(geometry.scroll<=geometry.client+1,`Recent places cannot scroll horizontally: ${JSON.stringify(geometry)}`);
       assert(geometry.body<=geometry.viewport+1,'A narrow recent-place list cannot widen the document');
       for(const row of geometry.rows){
        assert(row.open.right<=row.close.left+1,'Recent text must end before the action slot');
        assert(row.close.left>=geometry.root.left&&row.close.right<=geometry.root.right+1,'The entire close target stays inside the list');
        assert(row.glyph.left>=row.close.left&&row.glyph.right<=row.close.right,'The close glyph stays inside its own hit area');
        assert(row.hit,'The close target must be hit-testable');
       }
       const first=page.locator('[data-place]').first(),place=await first.getAttribute('data-place'),device=await first.getAttribute('data-place-device');
       await first.click();assert.deepEqual(await page.evaluate(()=>window.opened),[{device,place}]);
       await page.locator('[data-forget-place]').first().click();
       assert.equal(await page.locator('[data-place]').count(),1,'Forgetting removes only the chosen recent');
       assert.equal((await page.evaluate(()=>window.forgotten))[0].place,place);
       assert.equal((await page.evaluate(()=>window.opened)).length,1,'Closing must not also navigate');
       results.push({name,passed:true,geometry});
       if(width===200)await page.screenshot({path:join(out,name+'.png')});
      }catch(error){failures.push({name,error:String(error)});await page.screenshot({path:join(out,name+'-failure.png')});}
     }
     assert.deepEqual(errors,[]);
    }finally{await context.close();}
   }
  }finally{await browser.close();}
 }
 writeFileSync(join(out,'results.json'),JSON.stringify({results,failures},null,2));
 assert.deepEqual(failures,[],'Recent-place row layout/action regressions');
 console.log(`Recent places: ${results.length} production-row checks passed`);
}finally{
 await server?.close();rmSync(fixture,{recursive:true,force:true});
}
