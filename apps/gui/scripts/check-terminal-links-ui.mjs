import {createHash} from 'node:crypto';
// Real xterm renderer + browser pointer events and production providers/claim adapter.
// The terminal has no PTY; emitted mouse bytes and opening intents are observed, not executed.
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {join,basename} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),output=process.env.TERMINAL_LINK_UI_DIR;
assert(output,'Set TERMINAL_LINK_UI_DIR');mkdirSync(output,{recursive:true});
const fixture=mkdtempSync(join(gui,'.terminal-links-check-'));
writeFileSync(join(fixture,'index.html'),'<link rel="icon" href="data:,"><div id="terminal"></div><script type="module" src="./probe.ts"></script>');
writeFileSync(join(fixture,'probe.ts'),String.raw`
import {Terminal} from '@xterm/xterm';
import {WebLinksAddon} from '@xterm/addon-web-links';
import {pathLinks} from '/src/lib/term-links';
import {claimTerminalLinkClicks} from '/src/lib/term-link-gesture';
import {canActivateTerminalLink,linkIntent} from '/src/lib/content-link-intent';
import '@xterm/xterm/css/xterm.css';
document.body.style.background='#111';document.body.style.margin='24px';
const box=document.getElementById('terminal')!;
const wire=window.__terminalLinkWire={opened:[],bytes:[],ready:false};
const open=(event:MouseEvent,uri:string)=>{if(canActivateTerminalLink(event,term))wire.opened.push({uri,intent:linkIntent(event)});};
const term=new Terminal({cols:60,rows:12,fontSize:16,macOptionClickForcesSelection:true,linkHandler:{allowNonHttpProtocols:true,activate:open}});
term.open(box);term.loadAddon(new WebLinksAddon(open));
term.registerLinkProvider(pathLinks(term,'fixture',()=>'/fixture',(target,intent)=>wire.opened.push({uri:target.path,intent})));
const at=(x:number,y:number)=>{const screen=box.querySelector('.xterm-screen')!.getBoundingClientRect();return {col:Math.floor((x-screen.left)/(screen.width/term.cols)),row:Math.floor((y-screen.top)/(screen.height/term.rows))+term.buffer.active.viewportY};};
claimTerminalLinkClicks(term,box,at,10);
term.onData(bytes=>wire.bytes.push(bytes));box.addEventListener('contextmenu',event=>event.preventDefault());
window.__terminalLinkPoint=(col:number,row:number)=>{const b=box.querySelector('.xterm-screen')!.getBoundingClientRect();return {x:b.left+(col+.5)*b.width/term.cols,y:b.top+(row+.5)*b.height/term.rows};};
window.__terminalLinkCurrent=()=>term._core.linkifier.currentLink?.link.text;
window.__terminalLinkSelection=()=>term.getSelection();
term.write("\u001b[?1000h\u001b[?1006hhttps://example.org/verified\r\n/fixture/a.md\r\n\u001b]8;;https://example.org/osc8\u0007OSC8 link\u001b]8;;\u0007\r\nplain program text",()=>{wire.ready=true;});
`);
const fingerprint=()=>Object.fromEntries(['src/lib/term-link-gesture.ts','src/lib/term-links.ts','src/lib/content-link-intent.ts','src/views/TerminalPane.tsx'].map(file=>[file,createHash('sha256').update(readFileSync(join(gui,file))).digest('hex')]));
const sources=fingerprint();
let server,browser;const results=[];
try{
 server=await createServer({configFile:join(gui,'vite.config.ts'),root:gui,cacheDir:join(fixture,'.vite'),optimizeDeps:{entries:[join(fixture,'index.html')]},server:{host:'127.0.0.1',port:23835,strictPort:true,hmr:false},logLevel:'error',plugins:[{name:'private-terminal-ls',configureServer(s){s.middlewares.use(async(req,res,next)=>{if(req.url!=='/api/ls')return next();for await(const _ of req){}res.setHeader('Content-Type','application/json');res.end(JSON.stringify({path:'/fixture',entries:[{name:'a.md',dir:false,size:10,at_ms:1,from:null,received_ms:null}],truncated:false}));});}}]});await server.listen();
 const url=`http://127.0.0.1:${server.config.server.port}/${basename(fixture)}/index.html`;
 if(process.env.TERMINAL_LINK_LAB==='1'){console.log('Private terminal Lab '+url);await new Promise(r=>{process.once('SIGINT',r);process.once('SIGTERM',r);});}
 else{
 browser=await chromium.launch({channel:'chrome',headless:true});const page=await browser.newPage({viewport:{width:1100,height:600}});page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(url);await page.waitForFunction(()=>window.__terminalLinkWire?.ready);
 const point=(col,row)=>page.evaluate(([c,r])=>window.__terminalLinkPoint(c,r),[col,row]);
 const state=()=>page.evaluate(()=>window.__terminalLinkWire);
 const hover=async(col,row)=>{const p=await point(col,row);await page.mouse.move(p.x,p.y);await page.waitForFunction(()=>!!window.__terminalLinkCurrent());return p;};
 for(const [name,row,uri] of [['URL',0,'https://example.org/verified'],['path',1,'/fixture/a.md'],['OSC8',2,'https://example.org/osc8']]){
   for(const modifier of [null,process.platform==='darwin'?'Meta':'Control']){
     const p=await hover(3,row),before=await state();
     if(modifier)await page.keyboard.down(modifier);await page.mouse.click(p.x,p.y);if(modifier)await page.keyboard.up(modifier);
     const after=await state();assert.equal(after.opened.length,before.opened.length+1,name+' opens once');assert.deepEqual(after.opened.at(-1),{uri,intent:modifier?'system':'internal'});assert.equal(after.bytes.length,before.bytes.length,'claimed click emits no TUI mouse bytes');
     results.push({name:name+' '+(modifier||'plain'),passed:true});
   }
 }
 let p=await point(3,3),before=await state();await page.mouse.click(p.x,p.y);let after=await state();assert.equal(after.opened.length,before.opened.length);assert(after.bytes.length>before.bytes.length,'nonlink click still reaches program');results.push({name:'nonlink program mouse',passed:true});
 p=await hover(3,0);before=await state();await page.mouse.move(p.x,p.y);await page.mouse.down();await page.mouse.move(p.x+80,p.y+20,{steps:8});await page.mouse.up();after=await state();assert.equal(after.opened.length,before.opened.length,'drag is never also opening');assert(after.bytes.length>before.bytes.length,'drag returns program mouse ownership');results.push({name:'link drag returns program ownership',passed:true});
 p=await hover(3,0);before=await state();await page.mouse.click(p.x,p.y,{button:'right'});after=await state();assert.equal(after.opened.length,before.opened.length,'right click never opens');results.push({name:'right click stays separate',passed:true});
 const selectionModifier=process.platform==='darwin'?'Alt':'Shift';await page.keyboard.down(selectionModifier);p=await point(0,0);await page.mouse.move(p.x,p.y);await page.mouse.down();const end=await point(15,0);await page.mouse.move(end.x,end.y,{steps:8});await page.mouse.up();await page.keyboard.up(selectionModifier);assert(await page.evaluate(()=>window.__terminalLinkSelection()),'forced selection remains usable');results.push({name:'forced selection',passed:true});assert.deepEqual(errors,[]);assert.deepEqual(fingerprint(),sources,'production sources stayed fixed');await page.screenshot({path:join(output,'terminal.png')});
 }
}catch(error){results.push({passed:false,error:String(error)});if(browser)for(const context of browser.contexts())for(const page of context.pages())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});throw error;}
finally{writeFileSync(join(output,'report.json'),JSON.stringify({platform:process.platform,sources,results},null,2));await browser?.close();await server?.close();rmSync(fixture,{recursive:true,force:true});}
console.log(JSON.stringify(results));
