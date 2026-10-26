// Controlled component diagnosis; no native keyboard or Safari acceptance.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdirSync,mkdtempSync,statfsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {chromium,webkit} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const scenarios=process.env.COMPOSE_SCENARIOS??'all';
assert(['all','parent','transitions'].includes(scenarios),'Unknown COMPOSE_SCENARIOS');
const transitionsOnly=scenarios==='transitions';
const port=Number(process.env.COMPOSE_PORT??23844),base=`http://127.0.0.1:${port}`;
const engines=(process.env.COMPOSE_ENGINES??'chrome,webkit').split(',');
for(const engine of engines)assert(['chrome','webkit'].includes(engine),`Unknown engine ${engine}`);
assert.equal(new Set(engines).size,engines.length,'Duplicate engines');
const out=process.env.COMPOSE_OUT??mkdtempSync(join(tmpdir(),'khor-compose-'));mkdirSync(out,{recursive:true});
const files=['src/hooks/use-compose-layout.ts','src/hooks/use-chat-scroll.ts','src/views/Composer.tsx','src/components/SessionBar.tsx','src/styles/session-layout.css','lab/scroll-compose.tsx','lab/scroll-compose.html','scripts/check-scroll-compose.mjs'];
const hashes=()=>Object.fromEntries(files.map(file=>[file,createHash('sha256').update(readFileSync(join(gui,file))).digest('hex')]));
const report={head:execFileSync('git',['rev-parse','HEAD'],{cwd:gui,encoding:'utf8'}).trim(),files:hashes(),boundary:'Real production components/CSS; fixed parent epoch, stable/fresh log reference; browser component diagnosis only. No synthetic keyboard or native Safari claim.',results:[]};
const save=()=>writeFileSync(join(out,'results.json'),JSON.stringify(report,null,2));
const free=()=>{for(const path of ['/Volumes/disk0','/Users/zgh']){const s=statfsSync(path);assert(s.bavail*s.bsize>=5*1024**3,`${path} below5GiB`);}};
let server,browser,log='';
try{
  free();
  server=spawn(process.execPath,['--input-type=module','-e',`import{createServer}from'vite';const s=await createServer({configFile:'vite.config.ts',cacheDir:${JSON.stringify(join(out,'vite-cache'))},server:{host:'127.0.0.1',port:${port},strictPort:true,hmr:false}});await s.listen();console.log('COMPOSE_READY');process.on('SIGTERM',async()=>{await s.close();process.exit(0)});`],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},detached:true,stdio:['ignore','pipe','pipe']});
  report.server={pid:server.pid,pgid:server.pid,port,cwd:gui};save();
  for(const stream of [server.stdout,server.stderr])stream.on('data',chunk=>{log+=chunk;});
  const deadline=Date.now()+30000;while(!log.includes('COMPOSE_READY')){assert(server.exitCode===null&&Date.now()<deadline,log);await new Promise(resolve=>setTimeout(resolve,100));}
  for(const engine of engines){
    browser=await(engine==='chrome'?chromium:webkit).launch({headless:true,...(engine==='chrome'?{channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined}:{})});
    for(const probe of (transitionsOnly?[]:[false,true]))for(const long of [false,true])for(const reader of [false,true])for(const changeRevision of [false,true]){
      free();const context=await browser.newContext({viewport:{width:402,height:714},hasTouch:true,isMobile:false});const page=await context.newPage();
      const result={engine,version:browser.version(),probe,long,reader,changeRevision,errors:[]};report.results.push(result);page.on('pageerror',error=>result.errors.push(error.message));
      try{
        await page.goto(`${base}/lab/scroll-compose.html?probe=${Number(probe)}`);const field=page.locator('[data-composer-text]');await field.waitFor();
        await field.fill(long?'draft words '.repeat(24):'short');await field.evaluate(el=>el.blur());
        await page.evaluate(()=>window.__composeFixture.toBottom());await page.waitForTimeout(300);
        if(reader){const bounds=await page.locator('[data-compose-scroll]').boundingBox();await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);await page.mouse.wheel(0,-500);await page.waitForTimeout(350);}
        result.before=await page.evaluate(()=>window.__composeFixture.snapshot('setup'));
        assert.equal(result.before.active,'BODY');assert(reader?result.before.gap>100:Math.abs(result.before.gap)<=2,'setup establishes the selected scroll owner');
        result.after=await page.evaluate(change=>window.__composeFixture.rerender(change),changeRevision);
        result.readings=await page.evaluate(()=>window.__composeFixture.observe(600));
        assert.deepEqual(result.errors,[]);
        const visible=result.readings.filter(value=>value.reason==='after-parent-render'||value.reason==='animation-frame');
        if(reader)assert(visible.every(value=>value.first===result.before.first&&Math.abs(value.firstOffset-result.before.firstOffset)<=2),'Parent render changes the reading anchor');
        else assert(visible.every(value=>Math.abs(value.gap)<=2),'Parent render temporarily leaves the live tail');
        result.status='passed';
      }catch(error){result.status='failed';result.error=error.stack??String(error);process.exitCode=1;}
      finally{console.log(`${result.status} ${engine} probe=${probe} long=${long} reader=${reader} freshRevision=${changeRevision}`);save();await context.close();}
    }
    for(const reader of (scenarios!=='parent'?[false,true]:[])){
      const context=await browser.newContext({viewport:{width:402,height:714},hasTouch:true});
      const page=await context.newPage();
      const result={engine,scenario:'draft-and-width-transitions',reader,errors:[],steps:[]};report.results.push(result);
      page.on('pageerror',error=>result.errors.push(error.message));
      try{
        await page.goto(`${base}/lab/scroll-compose.html`);const field=page.locator('[data-composer-text]');await field.waitFor();
        await field.fill('draft words '.repeat(24));await field.evaluate(el=>el.blur());
        await page.evaluate(()=>window.__composeFixture.toBottom());await page.waitForTimeout(300);
        if(reader){const b=await page.locator('[data-compose-scroll]').boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);await page.mouse.wheel(0,-500);await page.waitForTimeout(350);}
        const anchor=await page.evaluate(()=>window.__composeFixture.snapshot('transition-setup'));
        for(const [name,draft,width] of [['short','short',402],['empty','',402],['long-again','draft words '.repeat(24),402],['wide','draft words '.repeat(24),700],['narrow','draft words '.repeat(24),320],['clear-narrow','',320]]){
          await page.setViewportSize({width,height:714});await field.fill(draft);await field.evaluate(el=>el.blur());await page.waitForTimeout(300);
          const value=await page.evaluate(name=>window.__composeFixture.snapshot(name),name);result.steps.push(value);
          assert.equal(await page.locator('[data-chat-box]').evaluate(el=>el.style.minHeight),'','Measurement restores inline minimum height');
          if(reader){assert(value.gap>2,'Draft changes must preserve reading intent');if(width===402)assert(value.first===anchor.first&&Math.abs(value.firstOffset-anchor.firstOffset)<=2,'Draft height changes move the reading anchor');}
          else assert(Math.abs(value.gap)<=2,'Draft or width transition leaves the live tail');
          if(!draft)assert(value.cap<=50,'Clearing the draft restores the compact composer');
        }
        assert.deepEqual(result.errors,[]);result.status='passed';
      }catch(error){result.status='failed';result.error=error.stack??String(error);process.exitCode=1;}
      finally{console.log(`${result.status} ${engine} draft-and-width-transitions reader=${reader}`);save();await context.close();}
    }
    await browser.close();browser=null;
  }
}catch(error){report.error=error.stack??String(error);process.exitCode=1;}
finally{
  await browser?.close();if(server?.exitCode===null){process.kill(-server.pid,'SIGTERM');await new Promise(resolve=>{server.once('exit',resolve);setTimeout(()=>{if(server.exitCode===null)process.kill(-server.pid,'SIGKILL');resolve();},4000);});}
  report.finalFiles=hashes();report.changedDuringRun=files.filter(file=>report.files[file]!==report.finalFiles[file]);if(report.changedDuringRun.length)process.exitCode=1;
  writeFileSync(join(out,'server.log'),log);save();console.log(out);
}
