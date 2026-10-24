/** Right-thumb coordinates adapt the production pill joystick, not its rhythm. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '../src/lib/deck-joystick';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;armed:boolean;direction:number;tier:number;interval:number;pulse:number;deflection:number;acquired:boolean};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function fanControl(origin:FanPoint,point:FanPoint,unit=44,placement:'right'|'bottom'='right',armed=false){
  const dx=origin.x-point.x,up=origin.y-point.y,distance=Math.hypot(dx,up),angle=Math.atan2(up,dx)*180/Math.PI,neutral=placement==='bottom'?90:45;
  return {distance,angle,overview:Math.abs(angle-neutral)<=18&&distance>unit*(armed?2.7:3.2),gain:distance>unit*.55&&Math.abs(angle-neutral)<=40?(angle-neutral)/30*DECK_JOYSTICK_DEFAULTS.reach:0};
}
/** Short throw: acquire a comfortable neutral, then move through detents. */
export function fanStick(origin:FanPoint,unit=44,placement:'right'|'bottom'='right'){
  let neutral=placement==='bottom'?90:45,radius=0,acquired=false,side=0,peak=0,wasOverview=false,resume:number|null=null,motionAngle=neutral,motionPoint={...origin};
  return {move(point:FanPoint,armed=false){
    const raw=fanControl(origin,point,unit,placement,armed),defaultAngle=placement==='bottom'?90:45;
    const gate=Math.max(unit*3.2,radius+unit*1.5),overview=Math.abs(raw.angle-defaultAngle)<=22&&raw.distance>gate-(armed?unit*.5:0);
    if(overview){wasOverview=true;side=0;peak=0;return {...raw,overview:true,gain:0,extraSteps:0,acquired,moved:false};}
    if(!acquired&&raw.distance>=unit*.7&&Math.abs(raw.angle-defaultAngle)<=60){neutral=raw.angle;motionAngle=raw.angle;motionPoint={...point};radius=Math.min(raw.distance,unit*1.2);acquired=true;return {...raw,overview:false,gain:0,extraSteps:0,acquired,moved:false};}
    if(wasOverview){resume=raw.angle;wasOverview=false;}
    if(resume!==null&&Math.abs(raw.angle-resume)>3)resume=null;
    const delta=raw.angle-neutral,magnitude=Math.abs(delta),threshold=side?8:12;
    const active=acquired&&resume===null&&raw.distance>unit*.45&&magnitude>threshold&&Math.abs(delta)<=65;
    if(!active){side=0;peak=0;return {...raw,overview:false,gain:0,extraSteps:0,acquired,moved:false};}
    const next=Math.sign(delta),level=Math.min(3,1+Math.floor(Math.max(0,magnitude-12)/12));
    const extraSteps=next!==side?0:Math.min(1,Math.max(0,level-peak));
    peak=next===side?Math.max(peak,level):level;side=next;
    const gain=next*(11+Math.min(1,Math.max(0,(magnitude-12)/24))*45);
    const moved=Math.abs(raw.angle-motionAngle)>=3||Math.hypot(point.x-motionPoint.x,point.y-motionPoint.y)>=unit*.12;if(moved){motionAngle=raw.angle;motionPoint={...point};}
    return {...raw,overview:false,gain,extraSteps,acquired,moved};
  }};
}
export function thumbFanJoystick(changed:(state:FanState)=>void,clock?:DeckJoystickClock){
  const idle=():FanState=>({phase:'idle',lane:null,held:false,origin:{x:0,y:0},point:{x:0,y:0},ids:[],hot:null,armed:false,direction:0,tier:0,interval:DECK_JOYSTICK_DEFAULTS.slowMs,pulse:0,deflection:0,acquired:false});
  let state=idle(),travelled=false,initial=0,unit=44;
  let engine:ReturnType<typeof deckJoystick>|null=null,stick=fanStick({x:0,y:0});
  const read=()=>({...state,origin:{...state.origin},point:{...state.point},ids:[...state.ids]});
  const publish=()=>changed(read());
  const update=()=>{
    const control=state.held?stick.move(state.point,state.armed):{...fanControl(state.origin,state.point,unit,'right',state.armed),extraSteps:0,acquired:false,moved:false},{distance}=control;
    state.deflection=control.gain/DECK_JOYSTICK_DEFAULTS.reach;state.acquired=control.acquired;
    travelled ||= distance>unit*.55;
    // Angular steering and deliberate outward travel have independent axes.
    // The radial overlap prevents tremor from flickering the overview target.
    const outward=control.overview;
    state.armed=outward;state.lane=outward?'overview':state.held?'fan':null;
    state.phase=outward?'overview':state.held?'fan':'pending';
    if(outward){if(state.held)engine?.move(0,0);publish();return;}
    if(!state.held){publish();return;}
    engine?.move(control.gain,0);
    for(let i=0;i<Math.min(control.extraSteps,Math.max(0,state.ids.length-2));i++){engine?.move(0,0);engine?.move(control.gain,0);}
    if(control.moved)engine?.restartReading();
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
      unit=touch;stick=fanStick(origin,touch);
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
