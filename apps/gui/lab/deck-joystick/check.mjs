import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({entryPoints:['lab/deck-joystick/engine.ts'],bundle:true,format:'esm',write:false});
const {joystick,DEFAULTS}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
function fixture(count=12,initial=3){
 let now=0,id=0;const frames=new Map(),commits=[];let overviews=0;
 const clock={now:()=>now,request(run){frames.set(++id,{at:now+16,run});return id;},cancel(id){frames.delete(id);}};
 const advance=ms=>{const until=now+ms;for(;;){const next=[...frames].filter(([,v])=>v.at<=until).sort((a,b)=>a[1].at-b[1].at)[0];if(!next)break;now=next[1].at;frames.delete(next[0]);next[1].run();}now=until;};
 const e=joystick(count,initial,()=>{},i=>commits.push(i),()=>overviews++,clock);
 e.start(200,500);
 return {e,advance,commits,frames,overviews:()=>overviews,hold:()=>advance(DEFAULTS.holdMs+16),move:(dx,dy=0)=>e.move(200+dx,500+dy)};
}
{
 const f=fixture();f.hold();assert.equal(f.e.read().phase,'steering');f.move(12);assert.equal(f.e.read().preview,4,'a short push immediately selects the adjacent item');
 assert.deepEqual(f.commits,[]);f.advance(1500);assert(f.e.read().preview>4,'stationary offset keeps browsing without a separate dwell');
 f.move(0);const paused=f.e.read().preview;f.advance(4000);assert.equal(f.e.read().preview,paused,'return to origin pauses indefinitely');
 f.move(-12);assert.equal(f.e.read().preview,(paused+11)%12,'other side responds immediately');f.move(0);const target=f.e.read().preview;
 f.e.release();f.e.release();f.move(56);assert.deepEqual(f.commits,target===3?[]:[target]);assert.equal(f.frames.size,0);
}
{
 const slow=fixture(),fast=fixture();slow.hold();fast.hold();slow.move(12);fast.move(56);slow.advance(1200);fast.advance(1200);
 assert.equal(slow.e.read().tier,1);assert.equal(fast.e.read().tier,3);
 assert(fast.e.read().pulse>slow.e.read().pulse,'each advance supplies one arrow beat');
 assert(fast.e.read().preview>slow.e.read().preview,'larger offset advances faster over the same time');
 assert(fast.e.read().interval<slow.e.read().interval);slow.e.cancel();fast.e.cancel();
}
{
 const f=fixture();f.hold();f.move(56);f.advance(300);f.move(56,-55);const stopped=f.e.read().preview;f.advance(1000);
 assert.equal(f.e.read().preview,stopped,'overview target pauses browsing');f.e.release();assert.equal(f.overviews(),1);assert.deepEqual(f.commits,[]);f.e.release();assert.equal(f.overviews(),1);
}
{
 const f=fixture();f.hold();f.move(56,-55);f.move(0,-20);assert(!f.e.read().overview);f.e.release();assert.equal(f.overviews(),0);assert.deepEqual(f.commits,[]);
}
for(const phase of ['pending','swipe','steering']){
 const f=fixture();if(phase==='steering')f.hold();if(phase==='swipe')f.move(-35);if(phase==='steering')f.move(56);
 f.e.cancel();f.e.release();f.advance(3000);assert.deepEqual(f.commits,[]);assert.equal(f.frames.size,0);assert.equal(f.e.read().preview,3);
}
for(const count of [0,1]){
 const f=fixture(count,0);f.hold();f.move(56);f.advance(2000);assert.equal(f.e.read().preview,0);f.move(0,-55);f.e.release();assert.equal(f.overviews(),1);assert.deepEqual(f.commits,[]);
}
{
 const f=fixture();f.move(-35);assert.equal(f.e.read().phase,'swipe');f.e.release();assert.deepEqual(f.commits,[],'ordinary swipe is delegated without a second commit');
 const vertical=fixture();vertical.move(0,-55);assert.equal(vertical.e.read().phase,'pull');vertical.e.release();assert.equal(vertical.overviews(),1);assert.deepEqual(vertical.commits,[]);
 const down=fixture();down.move(0,55);assert.equal(down.e.read().phase,'idle');down.e.release();assert.deepEqual(down.commits,[]);
 const tap=fixture();assert.equal(tap.e.release(),'tap');assert.deepEqual(tap.commits,[]);
}
{
 const f=fixture();f.move(0,-20);assert.equal(f.e.read().phase,'pull');assert(!f.e.read().overview);f.move(0,-60);assert(f.e.read().overview);f.move(0,-20);assert(!f.e.read().overview);f.e.release();assert.equal(f.overviews(),0);assert.deepEqual(f.commits,[]);
}
{
 const f=fixture();f.hold();f.move(12);assert.equal(f.e.read().tier,1);f.move(30);assert.equal(f.e.read().tier,2);f.move(56);assert.equal(f.e.read().tier,3);f.move(0);assert.equal(f.e.read().tier,0);assert.equal(f.e.read().direction,0);f.e.cancel();
}
for(const [offset,tier,interval] of [[12,1,1400],[30,2,1000],[56,3,700]]){
 const f=fixture();f.hold();f.move(offset);assert.equal(f.e.read().tier,tier);assert.equal(f.e.read().interval,interval);assert.equal(f.e.read().pulse,1);
 f.advance(interval-24);assert.equal(f.e.read().pulse,1,'the first immediate candidate gets a full reading interval');
 f.advance(48);assert.equal(f.e.read().pulse,2,'repeat starts at the advertised gear interval');
 f.advance(interval*3);assert.equal(f.e.read().pulse,5,'each gear keeps a predictable repeat rhythm');f.e.cancel();
}
{
 const f=fixture();f.hold();f.move(12);f.move(25);assert.equal(f.e.read().tier,1);f.move(28);assert.equal(f.e.read().tier,2);
 for(const offset of [25,26,25,27]){f.move(offset);assert.equal(f.e.read().tier,2,'small boundary jitter does not change gear');}
 f.move(22);assert.equal(f.e.read().tier,1);f.move(44);assert.equal(f.e.read().tier,3);f.move(41);assert.equal(f.e.read().tier,3);f.move(37);assert.equal(f.e.read().tier,2);
 assert.equal(f.e.read().pulse,1,'gear changes do not skip or restart the candidate');f.e.cancel();
}
console.log('Joystick Lab: 1400/1000/700ms gears, reading time, boundary hysteresis, immediate short push, rate beats, indefinite pause, reversal, direct up/retraction, cancellation, swipe, tap and single/empty states passed');
