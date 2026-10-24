/** Right-thumb coordinates adapt the production pill joystick, not its rhythm. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '../src/lib/deck-joystick';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;armed:boolean;direction:number;tier:number;interval:number;pulse:number};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function thumbFanJoystick(changed:(state:FanState)=>void,clock?:DeckJoystickClock){
  const idle=():FanState=>({phase:'idle',lane:null,held:false,origin:{x:0,y:0},point:{x:0,y:0},ids:[],hot:null,armed:false,direction:0,tier:0,interval:DECK_JOYSTICK_DEFAULTS.slowMs,pulse:0});
  let state=idle(),travelled=false,initial=0,backReach=DECK_JOYSTICK_DEFAULTS.reach;
  let engine:ReturnType<typeof deckJoystick>|null=null;
  const read=()=>({...state,origin:{...state.origin},point:{...state.point},ids:[...state.ids]});
  const publish=()=>changed(read());
  const update=()=>{
    const dx=state.point.x-state.origin.x,up=state.origin.y-state.point.y,distance=Math.hypot(dx,up),neutral=DECK_JOYSTICK_DEFAULTS.neutral;
    travelled ||= distance>neutral;
    if(!state.lane&&distance>neutral){
      if(up>=neutral&&Math.abs(dx)<=up*.25){state.lane='overview';engine?.cancel();}
      else if(state.held&&Math.abs(dx)>neutral&&up>=-20&&up<=Math.abs(dx)*2.5)state.lane='fan';
    }
    if(state.lane==='overview'){
      state.phase='overview';state.hot=null;state.direction=0;state.tier=0;
      state.armed=up>=(state.armed?38.4:48)&&Math.abs(dx)<=Math.max(44,up*.65);
      publish();return;
    }
    if(!state.held){publish();return;}
    // Forward uses the full inward travel. At the right screen edge, the
    // shorter reverse travel still reaches the pill's three unchanged gears.
    const reverse=Math.max(0,dx-neutral),scaled=neutral+reverse*(DECK_JOYSTICK_DEFAULTS.reach-neutral)/(backReach-neutral);
    engine?.move(state.lane==='fan'?(dx>neutral?-scaled:-dx):0,0);
  };
  const sync=(next:DeckJoystickState)=>{
    if(next.phase!=='pending'&&next.phase!=='steering')return;
    const entered=next.phase==='steering'&&!state.held;
    state.held ||= next.phase==='steering';
    state.phase=state.held?'fan':'pending';state.hot=state.held?next.preview:null;
    state.direction=next.direction;state.tier=next.tier;state.interval=next.interval;state.pulse=next.pulse;
    publish();if(entered)update();
  };
  const dispose=()=>{engine?.dispose();engine=null;};
  return {
    read,dispose,
    start(origin:FanPoint,ids:readonly string[],active:string,reverseReach=DECK_JOYSTICK_DEFAULTS.reach){
      dispose();state={...idle(),phase:'pending',origin:{...origin},point:{...origin},ids:[...ids]};initial=Math.max(0,ids.indexOf(active));travelled=false;
      backReach=Math.max(DECK_JOYSTICK_DEFAULTS.neutral+1,reverseReach);
      engine=deckJoystick(ids.length,initial,sync,()=>{},()=>{},clock);engine.start(0,0);
    },
    move(point:FanPoint){if(state.phase==='idle')return;state.point={...point};update();},
    cancel(){dispose();state=idle();publish();},
    release():FanResult{
      if(state.phase==='idle')return {kind:'none'};
      const before=read();dispose();let result:FanResult={kind:'none'};
      if(before.lane==='overview'&&before.armed)result={kind:'overview'};
      else if(before.held&&before.hot!==null&&before.hot!==initial)result={kind:'focus',id:before.ids[before.hot]};
      else if(!before.held&&!travelled&&before.lane===null)result={kind:'tap'};
      state=idle();publish();return result;
    },
  };
}
