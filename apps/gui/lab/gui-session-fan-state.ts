/** Right-thumb coordinates adapt the production pill joystick, not its rhythm. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '../src/lib/deck-joystick';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;armed:boolean;direction:number;tier:number;interval:number;pulse:number};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function thumbFanJoystick(changed:(state:FanState)=>void,clock?:DeckJoystickClock){
  const idle=():FanState=>({phase:'idle',lane:null,held:false,origin:{x:0,y:0},point:{x:0,y:0},ids:[],hot:null,armed:false,direction:0,tier:0,interval:DECK_JOYSTICK_DEFAULTS.slowMs,pulse:0});
  let state=idle(),travelled=false,initial=0,unit=44;
  let engine:ReturnType<typeof deckJoystick>|null=null;
  const read=()=>({...state,origin:{...state.origin},point:{...state.point},ids:[...state.ids]});
  const publish=()=>changed(read());
  const update=()=>{
    const dx=state.origin.x-state.point.x,up=state.origin.y-state.point.y,distance=Math.hypot(dx,up),angle=Math.atan2(up,dx)*180/Math.PI;
    travelled ||= distance>unit*.55;
    // Angular steering and deliberate outward travel have independent axes.
    // The radial overlap prevents tremor from flickering the overview target.
    const outward=Math.abs(angle-45)<=18&&distance>unit*(state.armed?2.7:3.2);
    state.armed=outward;state.lane=outward?'overview':state.held?'fan':null;
    state.phase=outward?'overview':state.held?'fan':'pending';
    if(outward){if(state.held)engine?.move(0,0);publish();return;}
    if(!state.held){publish();return;}
    const within=distance>unit*.55&&angle>=5&&angle<=85;
    engine?.move(within?(angle-45)/30*DECK_JOYSTICK_DEFAULTS.reach:0,0);
  };
  const sync=(next:DeckJoystickState)=>{
    if(next.phase!=='pending'&&next.phase!=='steering')return;
    const entered=next.phase==='steering'&&!state.held;
    state.held ||= next.phase==='steering';
    state.phase=state.armed?'overview':state.held?'fan':'pending';state.hot=state.held?next.preview:null;
    state.direction=next.direction;state.tier=next.tier;state.interval=next.interval;state.pulse=next.pulse;
    publish();if(entered)update();
  };
  const dispose=()=>{engine?.dispose();engine=null;};
  return {
    read,dispose,
    start(origin:FanPoint,ids:readonly string[],active:string,touch=44){
      dispose();state={...idle(),phase:'pending',origin:{...origin},point:{...origin},ids:[...ids]};initial=Math.max(0,ids.indexOf(active));travelled=false;
      unit=touch;
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
