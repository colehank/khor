/** Right-thumb coordinates adapt the production pill joystick, not its rhythm. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '../src/lib/deck-joystick';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;armed:boolean;cancelArmed?:boolean;direction:number;tier:number;interval:number;pulse:number;deflection:number;cursor?:number;acquired:boolean};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function fanControl(origin:FanPoint,point:FanPoint,unit=44,placement:'right'|'bottom'='right',armed=false){
  const dx=origin.x-point.x,up=origin.y-point.y,distance=Math.hypot(dx,up),angle=Math.atan2(up,dx)*180/Math.PI,neutral=placement==='bottom'?90:45;
  return {distance,angle,overview:Math.abs(angle-neutral)<=18&&distance>unit*(armed?2.7:3.2),gain:distance>unit*.55&&Math.abs(angle-neutral)<=40?(angle-neutral)/30*DECK_JOYSTICK_DEFAULTS.reach:0};
}
/** Acquire a comfortable landing; steer by thumb travel rather than polar angle. */
export function fanStick(origin:FanPoint,unit=44,placement:'right'|'bottom'='right'){
  let neutralPoint={...origin},radius=0,acquired=false,side=0,gear=0,peak=0,wasOverview=false,paused=false,reverseFrom:number|null=null,motionPoint={...origin};
  return {move(point:FanPoint,armed=false){
    const raw=fanControl(origin,point,unit,placement,armed),defaultAngle=placement==='bottom'?90:45;
    const gate=Math.max(unit*3.2,radius+unit*1.5),overview=Math.abs(raw.angle-defaultAngle)<=22&&raw.distance>gate-(armed?unit*.5:0);
    const rest={...raw,overview:false,gain:0,acquired,moved:false,cursor:0};
    if(overview){wasOverview=true;side=0;gear=0;peak=0;paused=true;reverseFrom=null;return {...rest,overview:true};}
    if(!acquired&&raw.distance>=unit*.7&&Math.abs(raw.angle-defaultAngle)<=60){neutralPoint={...point};motionPoint={...point};radius=Math.min(raw.distance,unit*1.2);acquired=true;wasOverview=false;return {...rest,acquired};}
    if(wasOverview){neutralPoint={...point};motionPoint={...point};wasOverview=false;return rest;}
    // Up/down remains usable with either diagonal or a straight thumb path.
    // The small horizontal contribution tolerates natural right-hand drift.
    const travel=placement==='bottom'?point.x-neutralPoint.x:neutralPoint.y-point.y+.2*(neutralPoint.x-point.x);
    const magnitude=Math.abs(travel),next=Math.sign(travel),stop=unit*.32;
    const cursor=acquired?Math.max(-2,Math.min(2,travel/(unit*.6))):0;
    if(!acquired)return {...rest,cursor};
    // A return stroke stops sooner and stays paused through small overshoot.
    // A coalesced event crossing the whole neutral zone brakes first; further
    // deliberate travel, rather than a repeated stationary event, reverses.
    const returning=magnitude<=unit*.16||magnitude<peak-unit*.02;
    if(side&&((magnitude<=stop&&returning)||next!==side)){
      reverseFrom=next!==side&&magnitude>stop?travel:null;
      side=0;gear=0;peak=0;paused=true;return {...rest,cursor};
    }
    if(reverseFrom!==null){
      if(magnitude<=stop)reverseFrom=null;
      else if(next!==Math.sign(reverseFrom)){reverseFrom=travel;return {...rest,cursor};}
      else if(magnitude<Math.abs(reverseFrom)+unit*.12)return {...rest,cursor};
      else reverseFrom=null;
    }
    if(!side&&magnitude<=unit*(paused?.45:.25))return {...rest,cursor};
    // Gear changes only adjust the stationary repeat rate. Wider overlaps
    // keep thumb tremor from accelerating or skipping another candidate.
    const previous=gear;
    if(next!==side)gear=1;
    while(gear<3&&magnitude>=unit*(gear===1?.85:1.3))gear++;
    while(gear>1&&magnitude<unit*(gear===3?1:.6))gear--;
    side=next;
    peak=Math.max(peak,magnitude);
    const gain=next*[0,18,35,DECK_JOYSTICK_DEFAULTS.reach][gear];
    const moved=gear!==previous||Math.hypot(point.x-motionPoint.x,point.y-motionPoint.y)>=unit*.12;if(moved)motionPoint={...point};
    return {...rest,gain,moved,cursor};
  }};
}
export function thumbFanJoystick(changed:(state:FanState)=>void,clock?:DeckJoystickClock){
  const idle=():FanState=>({phase:'idle',lane:null,held:false,origin:{x:0,y:0},point:{x:0,y:0},ids:[],hot:null,armed:false,cancelArmed:false,direction:0,tier:0,interval:DECK_JOYSTICK_DEFAULTS.slowMs,pulse:0,deflection:0,cursor:0,acquired:false});
  let state=idle(),travelled=false,initial=0,unit=44,cancelSeat:FanPoint={x:0,y:0};
  let engine:ReturnType<typeof deckJoystick>|null=null,stick=fanStick({x:0,y:0});
  const read=()=>({...state,origin:{...state.origin},point:{...state.point},ids:[...state.ids]});
  const publish=()=>changed(read());
  const update=()=>{
    const distance=Math.hypot(state.point.x-state.origin.x,state.point.y-state.origin.y);
    const homeDistance=Math.min(distance,Math.hypot(state.point.x-cancelSeat.x,state.point.y-cancelSeat.y));
    travelled ||= distance>unit*.55;
    if(state.held&&(state.acquired||state.armed||state.cancelArmed)&&homeDistance<=unit*(state.cancelArmed?.45:.3)){
      state.cancelArmed=true;state.armed=false;state.phase='fan';state.lane='fan';state.cursor=0;state.deflection=0;
      engine?.move(0,0);publish();return;
    }
    state.cancelArmed=false;
    const control=state.held?stick.move(state.point,state.armed):{...fanControl(state.origin,state.point,unit,'right',state.armed),acquired:false,moved:false,cursor:0};
    state.deflection=control.gain/DECK_JOYSTICK_DEFAULTS.reach;state.acquired=control.acquired;state.cursor=control.cursor;
    // Short thumb travel steers; deliberate outward travel opens overview.
    // The radial overlap prevents tremor from flickering the overview target.
    const outward=control.overview;
    state.armed=outward;state.lane=outward?'overview':state.held?'fan':null;
    state.phase=outward?'overview':state.held?'fan':'pending';
    if(outward){if(state.held)engine?.move(0,0);publish();return;}
    if(!state.held){publish();return;}
    engine?.move(control.gain,0);
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
    start(origin:FanPoint,ids:readonly string[],active:string,touch=44,home:FanPoint=origin){
      dispose();state={...idle(),phase:'pending',origin:{...origin},point:{...origin},ids:[...ids]};initial=Math.max(0,ids.indexOf(active));travelled=false;
      unit=touch;cancelSeat={...home};stick=fanStick(origin,touch);
      engine=deckJoystick(ids.length,initial,sync,()=>{},()=>{},clock);engine.start(0,0);
    },
    move(point:FanPoint){if(state.phase==='idle')return;state.point={...point};update();},
    cancel(){dispose();state=idle();publish();},
    release():FanResult{
      if(state.phase==='idle')return {kind:'none'};
      const before=read();dispose();let result:FanResult={kind:'none'};
      if(before.cancelArmed)result={kind:'none'};
      else if(before.lane==='overview'&&before.armed)result={kind:'overview'};
      else if(before.held&&before.hot!==null&&before.hot!==initial)result={kind:'focus',id:before.ids[before.hot]};
      else if(!before.held&&!travelled&&before.lane===null)result={kind:'tap'};
      state=idle();publish();return result;
    },
  };
}
