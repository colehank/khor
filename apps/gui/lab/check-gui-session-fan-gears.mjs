import assert from 'node:assert/strict';import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
function make(){let now=0,next=0;const jobs=new Map(),clock={now:()=>now,request:f=>{jobs.set(++next,f);return next;},cancel:id=>jobs.delete(id)},fan=thumbFanJoystick(()=>{},clock);fan.start({x:350,y:500},['a','b','c','d','e'],'a');const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...jobs.values()];jobs.clear();run.forEach(f=>f());}};const move=up=>fan.move({x:325-up*.35,y:475-up});advance(288);move(0);return {fan,move,advance,jobs};}
let checks=0;
for(const sign of [-1,1]){
 const {fan,move,advance,jobs}=make();move(sign*17);assert.equal(fan.read().pulse,1);move(sign*30);assert.equal(fan.read().tier,1,'ordinary travel remains in a wide first gear');assert.equal(fan.read().pulse,1,'increasing the offset does not skip another item');checks++;
 move(sign*36);assert.equal(fan.read().tier,2);assert.equal(fan.read().interval,1000);for(const up of [31,34,30,33,35]){move(sign*up);assert.equal(fan.read().tier,2,'second gear survives boundary tremor');assert.equal(fan.read().pulse,1);}checks++;
 advance(720);move(sign*24);assert.equal(fan.read().tier,1);advance(1200);assert.equal(fan.read().pulse,1,'downshift gives a full reading interval');advance(240);assert.equal(fan.read().pulse,2);checks++;
 move(sign*55);assert.equal(fan.read().tier,3);assert.equal(fan.read().pulse,2,'a deliberate fast gear only changes cadence');for(const up of [48,45,50]){move(sign*up);assert.equal(fan.read().tier,3);}move(sign*40);assert.equal(fan.read().tier,2);checks++;
 move(0);const paused=fan.read().pulse;advance(5000);assert.equal(fan.read().pulse,paused);move(-sign*17);assert.equal(fan.read().pulse,paused+1);assert.equal(fan.read().tier,1,'reverse starts gently');fan.cancel();assert.equal(jobs.size,0);checks++;
}
console.log(`${checks} wide gears, directional tremor, no manual skips, full reading interval, pause and reverse checks passed.`);
