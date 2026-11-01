import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './src/components/session-nav/state';export * from './src/lib/deck-joystick';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
export const {thumbFanJoystick,deckJoystick,DECK_JOYSTICK_DEFAULTS}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
export const origin={x:350,y:500},neutral={x:325,y:475};
export const pointAt=(angle,radius=35)=>({x:origin.x-radius*Math.cos(angle*Math.PI/180),y:origin.y-radius*Math.sin(angle*Math.PI/180)});
export function memoryClock(){let now=0,id=0;const jobs=new Map();return{clock:{now:()=>now,request:run=>{jobs.set(++id,run);return id;},cancel:id=>jobs.delete(id)},jobs,advance(ms){for(const end=now+ms;now<end;){now=Math.min(end,now+16);const ready=[...jobs.values()];jobs.clear();ready.forEach(run=>run());}}};}
export function fanFixture({ids=['a','b','c','d','e'],initial=0,home=origin}={}){
 const timing=memoryClock(),samples=[],fan=thumbFanJoystick(state=>samples.push(state),timing.clock);
 fan.start(origin,ids,ids[initial]??'',44,home);
 const move=(up,left=up*.35)=>fan.move({x:neutral.x-left,y:neutral.y-up});
 return{fan,...timing,samples,move,acquire(){timing.advance(288);move(0);assert.equal(fan.read().hot,initial,'landing establishes neutral without changing identity');}};
}
