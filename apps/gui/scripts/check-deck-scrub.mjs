import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:`export * from './src/lib/deck-scrub';`,resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {deckScrubber,nearbySlots,holdTiming,LONG_PRESS_MS,SETTLE_MS}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const REST=200,READ=1200,PARK=REST+SETTLE_MS+32,AUTO=REST+SETTLE_MS+READ+80;
function fixture(count=8,initial=0,rail=true,width=250){
 let now=0,id=0;const frames=new Map(),commits=[];
 const clock={now:()=>now,request(run){frames.set(++id,{at:now+16,run});return id;},cancel(id){frames.delete(id);}};
 const advance=ms=>{const until=now+ms;for(;;){const next=[...frames].filter(([,v])=>v.at<=until).sort((a,b)=>a[1].at-b[1].at)[0];if(!next)break;now=next[1].at;frames.delete(next[0]);next[1].run();}now=until;};
 const band=deckScrubber(count,initial,()=>{},i=>commits.push(i),clock);band.start(200,100,width,92);if(rail)advance(LONG_PRESS_MS+16);
 const move=dx=>band.move(200+dx,100),auto=(dx=-70)=>{move(dx);advance(AUTO);assert(band.read().holding);};
 return {band,advance,move,auto,commits,frames};
}
{
 const {band,move,advance,commits}=fixture();move(-70);advance(180);assert.notEqual(band.read().position,1,'parking waits 200ms');advance(PARK-180);
 assert.equal(band.read().position,1);advance(READ-100);assert.equal(band.read().position,1,'parked item gets 1200ms reading');advance(200);
 assert(band.read().holding&&band.read().position>1);assert.deepEqual(commits,[]);
 const candidate=band.read().preview,before=band.read().position;move(-58);assert.equal(band.read().position,before);assert(!band.read().holding);
 advance(PARK);assert.equal(band.read().preview,(candidate+7)%8,'same-side reverse corrects preceding candidate');
 const previous=band.read().position;advance(READ-300);assert.equal(band.read().position,previous,'correction gets a new delay');advance(500);
 assert.equal(band.read().direction,-1);assert(band.read().holding&&band.read().position<previous);
 const parked=band.read().preview;move(0);advance(PARK+3000);assert.equal(band.read().preview,parked);assert(!band.read().holding);band.cancel();assert.deepEqual(commits,[]);
}
for(const sign of [-1,1]){
 const {band,move,advance,auto,commits}=fixture();auto(sign*170);const fast=band.read();move(sign*158);
 assert(band.read().dwellMs>fast.dwellMs+300,'reversal resets cadence to slow');advance(PARK);
 for(let i=0;i<24;i++){move(sign*158+(i%2?3:-3));advance(80);}assert(band.read().holding);assert.equal(band.read().direction,sign);
 const moving=band.read().position;for(let i=0;i<20;i++){move(sign*158+(i%2?3:-3));advance(80);}
 assert(band.read().holding&&Math.abs(band.read().position-moving)>.1,'tremor cannot repeatedly brake auto');assert.deepEqual(commits,[]);band.cancel();
}
for(const extra of [0,40,80,120,160,200]){
 const {band,move,advance,auto}=fixture();auto();advance(extra);const target=(band.read().preview+7)%8;
 for(const dx of [-68,-66,-64,-62,-60]){move(dx);advance(16);}advance(PARK);
 assert.equal(band.read().preview,target,'sampled reverse corrects exactly one even at midpoint');band.cancel();
}
{
 const {band,move,advance,auto}=fixture();auto(-170);const original=band.read().preview;move(-158);advance(40);const before=band.read().position;move(-170);
 assert.equal(band.read().position,before);advance(PARK);assert.equal(band.read().preview,original,'latest turn overrides correction without skipping');
 const rest=band.read().position;advance(READ-300);assert.equal(band.read().position,rest);advance(500);assert.equal(band.read().direction,1);band.cancel();
}
for(const width of [120,250,500]){
 const {band,move,advance,auto}=fixture(8,0,true,width);auto(-30);move(8);advance(PARK);const rest=band.read().preview;advance(READ+1000);
 assert.equal(band.read().preview,rest);assert(!band.read().holding,'neutral area reachable across sizes');band.cancel();
}
{
 const {band,move,advance}=fixture();for(let i=1;i<=20;i++){move(-i*5);advance(40);}assert(!band.read().holding);advance(AUTO);assert(band.read().holding);band.cancel();
}
for(const sign of [-1,1]){
 const {band,move,advance,auto,commits,frames}=fixture();auto(sign*100);advance(16000);assert(Math.abs(band.read().position)>8);assert.deepEqual(commits,[]);
 const target=band.read().preview;band.release();for(let i=0;i<12;i++){move(-sign*170);band.release();advance(16);assert.equal(band.read().preview,target);}
 assert.deepEqual(commits,target===0?[]:[target]);assert.equal(frames.size,0);
}
{
 const {band,move,advance,commits}=fixture(8,0,false);move(-112.5);assert.equal(band.read().preview,1);band.release();assert.equal(band.read().preview,1,'40-percent swipe locks on release');
 for(let i=0;i<12;i++){move(170);advance(16);assert.equal(band.read().preview,1);}assert.deepEqual(commits,[1]);
}
for(const rail of [false,true])for(const [initial,dx,expected] of [[0,150,7],[7,-150,0]]){
 const {band,move,advance,commits,frames}=fixture(8,initial,rail);move(rail?Math.sign(dx)*92:dx);assert.equal(band.read().preview,expected);
 const slots=nearbySlots(band.read().position,8,2);assert(slots.some(s=>s.index===initial)&&slots.some(s=>s.index===expected));
 band.release();advance(SETTLE_MS+32);assert.equal(band.read().opened,expected);assert.deepEqual(commits,[expected]);assert.equal(frames.size,0);
}
{
 const {band,move,advance,commits,frames}=fixture(8,0,false);move(-100);assert.equal(band.read().position,.4);advance(680);assert.equal(band.read().position,1);
 advance(650);assert.equal(band.read().position,1);advance(160);assert(band.read().position>1);band.pause();const stopped=band.read().position;
 advance(2000);assert.equal(band.read().position,stopped);band.cancel();assert.equal(frames.size,0);assert.deepEqual(commits,[]);
}
{
 const {band,move,advance}=fixture(8,0,false);move(-75);advance(680);move(-170);const before=band.read().position;advance(100);assert(band.read().position>=before);
 move(-30);const stopped=band.read().position;advance(1500);assert.equal(band.read().position,stopped);band.cancel();
}
{
 const {band,move,advance,commits,frames}=fixture();move(-70);advance(PARK);const target=band.read().preview;band.release();band.start(200,100,250,92);advance(LONG_PRESS_MS+16);
 assert.equal(band.read().mode,'rail');advance(3000);assert.equal(band.read().preview,target);assert.deepEqual(commits,[target]);band.cancel();advance(1000);assert.equal(frames.size,0);
 band.start(200,100);band.move(201,130);assert.equal(band.release(),'none');band.start(200,100);assert.equal(band.release(),'tap');
}
{
 const {band,move,advance,commits,frames}=fixture(1);move(-180);advance(6000);band.release();advance(SETTLE_MS+32);assert.equal(band.read().opened,0);assert.deepEqual(commits,[]);assert.equal(frames.size,0);
 assert.deepEqual(nearbySlots(0,1,3),[{slot:0,index:0}]);const timing=holdTiming(170,250);assert(timing.dwellMs>=480);assert.deepEqual(timing,holdTiming(-170,250));
}
// Existing ordinary swipe regressions retained from the previous gate.
{
 const {band,advance}=fixture();
 band.start(0,0);band.move(-150,0);advance(680);assert.equal(band.read().position,1);
 band.move(150,0);advance(64);assert(band.read().position<1,'reversing a hold responds before the initial hold delay');
 band.cancel();
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
 const {band,advance,commits,frames}=fixture();
 band.start(0,0);band.move(-150,0);advance(100);band.release();band.arrive();
 assert.equal(band.read().opened,1);assert.deepEqual(commits,[1]);advance(500);assert.deepEqual(commits,[1]);assert.equal(frames.size,0,'interrupting release settles once and clears its frame');
 band.start(0,0);band.move(-150,0);advance(50);band.move(-80,0);band.release(true);
 assert.equal(band.read().opened,1,'a reverse flick lets go of the original drag');
}

console.log('pill gestures: approved 200/1200ms defaults, parking, local corrections, repeated turns, tremor tolerance, slow restart, neutral pause, release lock, cycles, swipe isolation and lifecycle passed');
