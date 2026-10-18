// Timing model only: no browser, PTY, input contents or network connection.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:`export {scrollHealth,notesScrollHealth} from './src/lib/term-scroll-health';`,resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {scrollHealth,notesScrollHealth}=await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString('base64')}`);
const health=scrollHealth();
health.start(0,'mouse');health.frame(16);health.frame(32);health.sent(20,50,30);
assert.equal(health.finish(100,40),null,'smooth gestures are silent');
health.start(200,'mouse');health.frame(216);health.frame(400);health.parsed();health.rendered();
const stalled=health.finish(450,60);assert.equal(stalled.frameGap,184);assert.equal(stalled.parses,1);assert.equal(stalled.renders,1);
health.start(500,'mouse');health.frame(900);assert.equal(health.finish(1000,90),null,'reports are rate limited');
health.start(31_000,'history');health.frame(31_016);health.sent(31_010,31_360,400);
const waiting=health.finish(31_500,50);assert.equal(waiting.sendMs,350);assert.equal(waiting.queued,400);assert.equal(waiting.route,'history');
health.start(62_000,'mouse');health.frame(62_500);assert.equal(health.finish(62_600,0),null,'a long press is not a scroll');
health.start(63_000,'mouse');health.sent(62_000,62_500,8000);health.frame(63_016);assert.equal(health.finish(63_050,30),null,'old completions do not contaminate a new gesture');
health.start(64_000,'mouse');health.frame(64_500);health.cancel();assert.equal(health.finish(64_600,80),null,'background/cancel drops the sample');
health.start(65_000,'mouse');health.frame(65_016);health.sending(65_010,42);assert.equal(health.finish(65_400,30).sendMs,390,'an unresolved send is visible before its response');
console.log('scroll health: smooth/slow frames, send wait, counters, rate limit, long press, stale completions and cancellation passed');
// Passive binding lifecycle, with deterministic frames/timers instead of UI.
const saved=Object.fromEntries(['window','document','performance','requestAnimationFrame','cancelAnimationFrame','clearTimeout'].map(k=>[k,globalThis[k]]));
try{
 let clock=0,id=0,subscribed=0;const raf=new Map(),timers=new Map(),notes=[];
 const doc=new EventTarget();doc.hidden=false;const win=new EventTarget(),box=new EventTarget();
 win.setTimeout=(run)=>{timers.set(++id,run);return id;};
 Object.assign(globalThis,{window:win,document:doc,performance:{now:()=>clock},requestAnimationFrame:run=>{const key=++id;raf.set(key,at=>{raf.delete(key);run(at);});return key;},cancelAnimationFrame:id=>raf.delete(id),clearTimeout:id=>timers.delete(id)});
 const term={onWriteParsed:()=>{subscribed++;return {dispose:()=>subscribed--};},onRender:()=>{subscribed++;return {dispose:()=>subscribed--};}};
 const disabled=notesScrollHealth({enabled:false,term,scroller:box,shown:()=>true,route:()=>'',note:()=>{}});disabled.stop();assert.equal(subscribed,0);
 const binding=notesScrollHealth({enabled:true,term,scroller:box,shown:()=>true,route:()=> 'mouse',note:line=>notes.push(line)});
 const touch=(type,y)=>{const e=new Event(type,{cancelable:true});e.touches=[{clientY:y}];box.dispatchEvent(e);assert.equal(e.defaultPrevented,false);};
 touch('touchstart',100);touch('touchmove',30);clock=180;[...raf.values()][0](clock);win.dispatchEvent(new Event('touchend'));
 [...timers.values()].at(-1)();assert.equal(notes.length,1);assert.match(notes[0],/frame gap 180 ms/);
 assert.equal(raf.size,0);assert.equal(timers.size,0);
 clock=32_000;touch('touchstart',100);touch('touchmove',30);doc.hidden=true;doc.dispatchEvent(new Event('visibilitychange'));
 assert.equal(raf.size,0);assert.equal(timers.size,0);assert.equal(notes.length,1,'backgrounding does not report artificial stalls');
 binding.stop();assert.equal(subscribed,0);doc.hidden=false;touch('touchstart',100);assert.equal(raf.size,0,'unmounted bindings stay silent');
}finally{Object.assign(globalThis,saved);}
console.log('scroll health: passive touch binding, coarse-only activation, bounded sampler, background cancellation and cleanup passed');
