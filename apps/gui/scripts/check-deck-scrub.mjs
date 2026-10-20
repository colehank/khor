import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:`export * from './src/lib/deck-scrub';`,resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {deckScrubber,nearbySlots,holdTiming,LONG_PRESS_MS,HOLD_MS,SETTLE_MS}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
function fixture(count=8,initial=0){
 let now=0,id=0;const frames=new Map(),commits=[];
 const clock={now:()=>now,request(run){frames.set(++id,{at:now+16,run});return id;},cancel(id){frames.delete(id);}};
 const advance=ms=>{const until=now+ms;for(;;){const next=[...frames].filter(([,v])=>v.at<=until).sort((a,b)=>a[1].at-b[1].at)[0];if(!next)break;now=next[1].at;frames.delete(next[0]);next[1].run();}now=until;};
 const band=deckScrubber(count,initial,()=>{},i=>commits.push(i),clock);
 return {band,advance,commits,frames};
}
{
 const {band,advance,commits,frames}=fixture();
 band.start(0,0,250);band.move(-100,0);
 assert.equal(band.read().position,.4,'direct drag follows without smoothing');
 advance(680);
 assert.equal(band.read().position,1,'held direction lands the next whole item');
 advance(650);assert.equal(band.read().position,1,'each item rests rather than continuously drifting');
 advance(160);assert(band.read().position>1&&band.read().position<2,'travel resumes promptly after dwelling');
 assert.equal(band.read().mode,'swipe','horizontal movement locks out long press');assert.deepEqual(commits,[]);
 band.pause();const stopped=band.read().position;assert(Number.isInteger(stopped));advance(1000);assert.equal(band.read().position,stopped);
 band.cancel();assert.equal(band.read().position,0);assert.equal(frames.size,0);
}
{
 const {band,advance,commits}=fixture();
 band.start(0,0);advance(LONG_PRESS_MS+16);assert.equal(band.read().mode,'rail');
 band.move(-92,0);assert.equal(band.read().position,1);advance(370);assert.equal(band.read().position,2,'long press responds promptly without a second full hold');
 advance(400);assert.equal(band.read().position,2,'faster initial response retains a readable full-title dwell');assert.deepEqual(commits,[]);
 band.pause();const stopped=band.read().position;advance(1000);assert.equal(band.read().position,stopped,'explicit pause stops automatic steps');
 band.release();assert.equal(band.read().opened,0);advance(SETTLE_MS+32);assert.equal(band.read().opened,2);assert.deepEqual(commits,[2]);
 band.release();advance(1000);assert.deepEqual(commits,[2],'only final release opens content');
}
{
 const {band,advance}=fixture();
 band.start(0,0);band.move(-150,0);advance(680);assert.equal(band.read().position,1);
 band.move(150,0);advance(64);assert(band.read().position<1,'reversing a hold responds before the initial hold delay');
 band.cancel();
}
for(const rail of [false,true]) for(const [initial,dx,expected] of [[0,150,7],[7,-150,0]]){
 const {band,advance,commits,frames}=fixture(8,initial);
 band.start(0,0,250);if(rail)advance(LONG_PRESS_MS+16);
 band.move(rail?Math.sign(dx)*92:dx,0);
 assert.equal(band.read().preview,expected,`preview wraps ${initial} to ${expected} in ${rail?'rail':'swipe'}`);
 const slots=nearbySlots(band.read().position,8,2);assert(slots.some(s=>s.index===initial)&&slots.some(s=>s.index===expected),'rendered neighbours span the seam');
 band.release();advance(SETTLE_MS+32);assert.equal(band.read().opened,expected);assert.equal(band.read().position,expected,'rebase invisibly after landing');assert.deepEqual(commits,[expected]);assert.equal(frames.size,0);
}
{
 const {band,advance,commits}=fixture();
 band.start(0,0);band.move(150,0);advance(16000);
 assert(band.read().position < -8,'holding continues across repeated complete cycles');assert(band.read().preview>=0&&band.read().preview<8);assert.deepEqual(commits,[]);
 band.pause();const target=band.read().preview;band.release(true);assert.equal(band.read().opened,target);assert.deepEqual(commits,target===0?[]:[target]);
}
{
 const {band,advance,commits,frames}=fixture();
 band.start(0,0);advance(LONG_PRESS_MS+16);band.move(-92,100);assert.equal(band.read().mode,'rail','long press retains selection outside pill until release');band.cancel();
 band.start(0,0);band.move(1,30);assert.equal(band.release(),'none','vertical drag cannot become a tap');
 band.start(0,0);assert.equal(band.release(),'tap');
 band.start(0,0);advance(16);band.move(20,0);band.release();advance(SETTLE_MS+32);assert.equal(band.read().opened,7,'short flick also wraps');
 band.start(0,0);band.move(-150,0);band.cancel();advance(2000);assert.equal(band.read().opened,7);assert.deepEqual(commits,[7]);assert.equal(frames.size,0);
 band.start(0,0);band.move(-150,0);band.release(true);assert.equal(band.read().opened,0);assert.equal(frames.size,0,'reduced-motion release has no outstanding frame');
}
{
 const {band,advance,commits,frames}=fixture(1);
 band.start(0,0);band.move(-180,0);advance(1000);band.release();advance(SETTLE_MS+32);assert.equal(band.read().opened,0);assert.equal(band.read().position,0);assert.deepEqual(commits,[]);assert.equal(frames.size,0);
 assert.deepEqual(nearbySlots(0,1,3),[{slot:0,index:0}],'one item is not repeated visually');
}
{
 const near=fixture(),far=fixture();
 near.band.start(0,0);near.band.move(-75,0);near.advance(1400);
 far.band.start(0,0);far.band.move(-170,0);far.advance(1400);
 assert.equal(near.band.read().position,1,'small displacement gives a long readable stop');
 assert(far.band.read().position>=2,'larger displacement advances faster');
 assert.deepEqual(near.commits,[]);assert.deepEqual(far.commits,[]);
 near.band.cancel();far.band.cancel();
}
{
 const {band,advance}=fixture();band.start(0,0);band.move(-75,0);advance(680);
 assert.equal(band.read().position,1);
 band.move(-170,0);assert.equal(band.read().position,1,'changing rate during dwell never pulls the track');
 advance(390);assert.equal(band.read().position,1,'even fast mode preserves at least 480ms full-title dwell');
 advance(80);assert(band.read().position>1&&band.read().position<2);
 const before=band.read().position;band.move(-80,0);assert.equal(band.read().position,before,'changing rate during travel never jumps the track');
 advance(300);assert.equal(band.read().position,2);
 band.move(-45,0);assert.equal(band.read().direction,1,'hysteresis prevents jitter at activation boundary');
 band.move(-30,0);const stopped=band.read().position;advance(1500);assert.equal(band.read().position,stopped,'centre dead zone stops the rate controller');
 band.cancel();
}
{
 const near=holdTiming(60,250),far=holdTiming(170,250);
 assert(near.travelMs>far.travelMs&&near.dwellMs>far.dwellMs);
 assert.deepEqual(holdTiming(-120,250),holdTiming(120,250),'direction has symmetric speed');
 assert.deepEqual(holdTiming(120,250),holdTiming(240,500),'same relative displacement has same speed');
 assert(holdTiming(10000,250).dwellMs>=480,'speed has a readable upper cap');
}
{
 const {band,advance,commits,frames}=fixture();
 band.start(0,0);band.move(-150,0);advance(100);band.release();band.arrive();
 assert.equal(band.read().opened,1);assert.deepEqual(commits,[1]);advance(500);assert.deepEqual(commits,[1]);assert.equal(frames.size,0,'interrupting release settles once and clears its frame');
 band.start(0,0);band.move(-150,0);advance(50);band.move(-80,0);band.release(true);
 assert.equal(band.read().opened,1,'a reverse flick lets go of the original drag');
}
console.log('dual pill: direct tracking, distance-controlled cadence, minimum item dwell, no rate-change jumps, hysteresis, both held gestures, long-press ownership beyond pill bounds, centre pause/reverse, cyclic seams in both directions, repeated cycles, one final commit, cancellation and cleanup passed');

// Tuned interaction budgets, not reaction-time claims. Picker entry and
// hold response are separate from the time to read a title after landing.
{
 const {band,advance}=fixture();band.start(0,0,300,100);advance(340);
 assert.equal(band.read().mode,'rail','long press exposes the picker within 340ms');band.cancel();
}
{
 const {band,advance,commits}=fixture();band.start(0,0,300,100);advance(LONG_PRESS_MS+16);
 band.move(-30,0);
 assert.equal(band.read().direction,1,'long-press threshold uses item width, not entire pill');
 assert.equal(band.read().position,.3,'rail follows the finger immediately');
 advance(240);assert(band.read().position>.6,'held rail starts responding within 240ms');
 assert.deepEqual(commits,[]);band.cancel();
}
{
 const {band,advance,commits}=fixture();band.start(0,0,300,100);advance(LONG_PRESS_MS+16);band.move(-30,0);
 for(let i=0;i<8;i++){advance(30);band.move(-30+(i%2?1.5:-1.5),0);}
 assert(band.read().position>.6,'small finger jitter must not indefinitely postpone a held rail');
 assert.deepEqual(commits,[]);band.cancel();
}
{
 const {band,advance}=fixture();band.start(0,0,300,100);advance(LONG_PRESS_MS+16);band.move(-25,0);
 assert.equal(band.read().direction,1);band.move(-22,0);assert.equal(band.read().direction,1,'small reverse jitter is ignored');
 band.move(-15,0);assert.equal(band.read().direction,-1,'deliberate return reverses without crossing origin');band.pause();
 const stopped=band.read().position;advance(1000);assert.equal(band.read().position,stopped);band.cancel();
}
{
 const {band,advance}=fixture();band.start(0,0,300,100);advance(LONG_PRESS_MS+16);band.move(-30,0);advance(120);
 band.move(-60,0);assert.equal(band.read().position,.6,'deliberate movement tracks directly');
 advance(120);assert.equal(band.read().position,.6,'deliberate movement restarts hold');band.cancel();
}
{
 const {band,advance}=fixture();band.start(0,0,300,100);band.move(-30,0);advance(1200);
 assert.equal(band.read().mode,'swipe');assert.equal(band.read().direction,0);assert.equal(band.read().position,.1,'ordinary short swipe retains original threshold');band.cancel();
}
console.log('held picker responsiveness: entry, item-scaled threshold/hysteresis, prompt motion, jitter tolerance and ordinary swipe isolation passed');

// A turning point is local: reversal must work without crossing touch-down.
{
 const {band,advance,commits,frames}=fixture();
 band.start(200,100,250,92);advance(400);band.move(100,100);advance(500);
 const left=band.read();assert.equal(left.direction,1);assert(left.holding);
 band.move(112,100);const right=band.read();
 assert.equal(right.direction,-1,'reverse while the finger remains left of touch-down');
 assert.equal(right.position,left.position,'changing direction preserves the displayed position');
 assert(right.dwellMs>left.dwellMs,'the new direction starts at a readable slow rate');
 advance(80);assert(band.read().position<right.position,'reverse motion starts without the initial hold delay');
 const before=band.read().position;band.move(100,100);
 assert.equal(band.read().direction,1);assert.equal(band.read().position,before,'second same-side reversal does not jump');
 band.move(103,100);assert.equal(band.read().direction,1,'small jitter does not reverse again');
 advance(80);assert(band.read().position>before);assert.deepEqual(commits,[]);
 band.release(true);assert.equal(commits.length,1);assert.equal(frames.size,0);
}
console.log('long-press steering: same-side reversals, continuous position, slow restart, jitter filtering and one final commit passed');
