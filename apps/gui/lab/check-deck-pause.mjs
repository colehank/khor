import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({entryPoints:['lab/deck-scrub-pause-model.ts'],bundle:true,format:'esm',write:false});
const {deckScrubber,LONG_PRESS_MS,SETTLE_MS,DEFAULT_TUNING,getTuning,setTuning}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
// Preserve regression coverage of the earlier 600/120 trial as an override.
setTuning({...DEFAULT_TUNING,readMs:600,restMs:120},false);
function fixture(count=8,rail=true,width=250){let now=0,id=0;const frames=new Map(),commits=[];
 const clock={now:()=>now,request(run){frames.set(++id,{at:now+16,run});return id;},cancel(id){frames.delete(id);}};
 const advance=ms=>{const end=now+ms;for(;;){const next=[...frames].filter(([,v])=>v.at<=end).sort((a,b)=>a[1].at-b[1].at)[0];if(!next)break;now=next[1].at;frames.delete(next[0]);next[1].run();}now=end;};
 const band=deckScrubber(count,0,()=>{},i=>commits.push(i),clock);band.start(200,100,width,92);if(rail)advance(LONG_PRESS_MS+16);
 const move=dx=>band.move(200+dx,100);
 return {band,move,advance,commits,frames};}
{
 const {band,move,advance,commits,frames}=fixture();move(-70);advance(350);
 assert.equal(band.read().position,1,'first park the selected candidate');advance(400);
 assert.equal(band.read().position,1,'candidate gets a full reading delay after landing');
 advance(500);assert(band.read().holding,'stationary offset starts auto after reading');assert(band.read().position>1);
 const candidate=band.read().preview,before=band.read().position;move(-58);
 assert.equal(band.read().position,before,'reverse does not teleport');assert(!band.read().holding,'reverse stops old motor immediately');
 advance(250);assert.equal(band.read().preview,(candidate+7)%8,'reverse returns to preceding candidate while still left of origin');
 const previous=band.read().position;advance(300);assert.equal(band.read().position,previous,'reverse gets a new reading delay');
 advance(600);assert(band.read().holding);assert.equal(band.read().direction,-1,'new auto direction follows latest deliberate motion');assert(band.read().position<previous);
 const center=band.read().preview;move(0);advance(400);assert.equal(band.read().preview,center);advance(4000);assert.equal(band.read().preview,center);assert(!band.read().holding,'origin is an indefinite pause');
 assert.deepEqual(commits,[]);band.release();advance(SETTLE_MS+32);assert.deepEqual(commits,center===0?[]:[center]);assert.equal(frames.size,0);
}
for(const sign of [-1,1]){
 const {band,move,advance}=fixture();move(sign*30);advance(350);const parked=band.read().position;
 for(let i=0;i<12;i++){move(sign*30+(i%2?1.5:-1.5));advance(80);}
 assert(band.read().holding,'small jitter does not indefinitely postpone stationary auto');assert.equal(band.read().direction,-sign);
 move(0);advance(350);const rest=band.read().position;
 for(let i=0;i<20;i++){move(i%2?4:-4);advance(100);}
 assert.equal(band.read().position,rest,'neutral jitter never arms auto');
 band.cancel();advance(1000);assert.equal(band.read().position,0);
}
{
 const {band,move,advance}=fixture();for(let i=1;i<=20;i++){move(-i*5);advance(40);}
 assert(!band.read().holding,'continuous deliberate movement does not start the auto motor');
 const before=band.read().position;advance(1000);assert(band.read().holding);assert(band.read().position>before);
 band.cancel();
}
{
 const {band,move,advance,commits}=fixture(1);move(-100);advance(4000);move(-80);advance(2000);band.release(true);
 assert.equal(band.read().position,0);assert.deepEqual(commits,[]);
}
{
 const {band,move,advance}=fixture();move(-70);advance(1300);const before=band.read().preview;
 for(const dx of [-68,-66,-64,-62,-60]){move(dx);advance(16);}
 advance(250);assert.equal(band.read().preview,(before+7)%8,'small consecutive reverse samples still correct to preceding item');
 assert(!band.read().holding);band.cancel();
}
for(const elapsed of [900,940,980,1020,1060,1100]){
 const {band,move,advance}=fixture();move(-70);advance(elapsed);
 assert(band.read().holding);const target=(band.read().preview+7)%8;
 for(const dx of [-66,-62,-58]){move(dx);advance(16);}
 advance(250);assert.equal(band.read().preview,target,'slow reversal near a candidate midpoint corrects exactly one item');
 band.cancel();
}
{
 const {band,move,advance}=fixture();move(-170);advance(1300);const fast=band.read();assert(fast.holding);
 move(-158);const slow=band.read();
 assert(slow.dwellMs>fast.dwellMs+300,'reversal resets fast cadence to a slow reading dwell');
 assert(slow.travelMs>fast.travelMs,'reversal does not inherit the fast travel');
 advance(1200);assert.equal(band.read().direction,-1);assert(band.read().dwellMs>=850);
 band.cancel();
}
{
 const {band,move,advance,commits}=fixture(8,false);move(-112.5);
 assert.equal(band.read().preview,1);band.release();
 assert.equal(band.read().preview,1,'release immediately locks the chosen preview, including ordinary 40-percent swipes');
 for(let i=0;i<12;i++){move(170);advance(16);assert.equal(band.read().preview,1,'settle frames cannot change the locked preview');}
 assert.deepEqual(commits,[1]);
}
for(const elapsed of [900,980,1060,1800]){
 const {band,move,advance,commits,frames}=fixture();move(-170);advance(elapsed);
 const target=band.read().preview;band.release();
 for(let i=0;i<12;i++){move(170);band.release();advance(16);assert.equal(band.read().preview,target,'release locks even mid-auto travel');}
 assert.deepEqual(commits,target===0?[]:[target]);assert.equal(frames.size,0);
}
{
 const {band,move,advance}=fixture();move(-170);advance(1300);const original=band.read().preview;
 move(-158);advance(40);const before=band.read().position;move(-170);
 assert.equal(band.read().position,before,'turn again during correction without teleporting');
 advance(250);assert.equal(band.read().preview,original,'latest turn during correction restores that item, without skipping');
 assert(!band.read().holding);const resting=band.read().position;advance(350);assert.equal(band.read().position,resting,'new correction gets its own reading delay');
 advance(600);assert.equal(band.read().direction,1,'auto then follows the latest turn');band.cancel();
}
{
 const {band,move,advance}=fixture();move(-70);advance(350);
 for(let i=0;i<20;i++){move(-70+(i%2?3:-3));advance(60);}
 assert(band.read().holding,'small natural tremor does not keep restarting the read timer');
 const moving=band.read().position;
 for(let i=0;i<20;i++){move(-70+(i%2?3:-3));advance(60);}
 assert(band.read().holding&&band.read().position>moving,'same tremor does not repeatedly brake auto');band.cancel();
}
for(const width of [120,250,500]){
 const {band,move,advance}=fixture(8,true,width);move(-30);advance(1300);assert(band.read().holding,'offset still works across pill sizes');
 move(8);advance(350);const parked=band.read().preview;advance(1800);
 assert.equal(band.read().preview,parked);assert(!band.read().holding,'near-origin pause is reachable even in narrow nested pills');band.cancel();
}
{
 const {band,move,advance,commits}=fixture();move(-70);advance(350);const target=band.read().preview;band.release();
 band.start(200,100,250,92);advance(LONG_PRESS_MS+16);assert.equal(band.read().mode,'rail');
 assert.equal(band.read().preview,target);assert.deepEqual(commits,[target]);
 advance(2000);assert.equal(band.read().preview,target,'a subsequent contact starts neutral without the old direction');
 band.cancel();
}
{
 setTuning({...DEFAULT_TUNING,restMs:300,readMs:1400});
 const {band,move,advance}=fixture();move(-70);advance(250);
 assert.notEqual(band.read().position,1,'custom rest delay is actually used');
 setTuning({restMs:40,readMs:300});
 advance(250);assert.equal(band.read().position,1);
 advance(1200);assert(!band.read().holding,'current contact retains its original long reading delay');
 advance(300);assert(band.read().holding,'long custom waiting time eventually starts auto');band.cancel();
 const next=fixture();next.move(-70);next.advance(650);assert(next.band.read().holding,'next contact uses the new shorter settings');next.band.cancel();
 setTuning(DEFAULT_TUNING);
}
{
 setTuning({longPressMs:600,followGain:2});const {band,move,advance}=fixture();assert.equal(band.read().mode,'pending');
 advance(300);assert.equal(band.read().mode,'rail');move(-46);assert.equal(band.read().position,1,'follow sensitivity is actually used');band.cancel();
 setTuning({readMs:Infinity,restMs:-1,dwellSlowMs:500,dwellFastMs:1500});
 const tuning=getTuning();assert.equal(tuning.readMs,DEFAULT_TUNING.readMs);assert.equal(tuning.restMs,40);assert.equal(tuning.dwellFastMs,500,'fast cadence cannot exceed slow dwell');
 setTuning(DEFAULT_TUNING);
}
console.log('PASS: settle then 600ms reading, stationary offset auto, local reverse to preceding candidate, renewed dwell, latest-motion direction, neutral pause, jitter, reverse speed reset, immutable release preview, repeated corrections, tremor filtering, reachable neutral regions, fresh contacts, adjustable timings/sensitivity, per-contact snapshot, range validation, final commit and cancel');
