/** Right-thumb travel adapts the same continuous engine as the bottom pill. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '@/lib/deck-joystick';
import {deckControl,deckHandoffBoundary,deckIntent,type DeckIntent} from '@/lib/deck-intent';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'new'|'overview';lane:'fan'|'handoff'|'new'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;position:number;target:number;armed:boolean;cancelArmed?:boolean;direction:number;tier:number;interval:number;pulse:number;deflection:number;cursor?:number;acquired:boolean;opened:number;intent:DeckIntent};
export type FanResult={kind:'none'|'tap'|'new'|'overview'}|{kind:'focus';id:string};
export function fanControl(origin:FanPoint,point:FanPoint,unit=44,placement:'right'|'bottom'='right',armed=false){
  const dx=origin.x-point.x,up=origin.y-point.y,distance=Math.hypot(dx,up),angle=Math.atan2(up,dx)*180/Math.PI,neutral=placement==='bottom'?90:45;
  return {distance,angle,overview:deckIntent({dx,up,touch:unit,placement,previous:armed?'overview':'switch'})==='overview',gain:distance>unit*.55&&Math.abs(angle-neutral)<=40?(angle-neutral)/30*DECK_JOYSTICK_DEFAULTS.reach:0};
}
export function thumbFanJoystick(changed: (state: FanState) => void, clock?: DeckJoystickClock) {
  const idle = (): FanState => ({
    phase: 'idle', lane: null, held: false, origin: { x: 0, y: 0 }, point: { x: 0, y: 0 },
    ids: [], hot: null, armed: false, cancelArmed: false, direction: 0, tier: 0,
    interval: DECK_JOYSTICK_DEFAULTS.slowMs, pulse: 0, deflection: 0, cursor: 0,
    acquired: false, position: 0, target: 0, opened: 0, intent: 'keep',
  });
  let state = idle();
  let initial = 0, unit = 44, travelled = false, wasAction = false,cursorBase=0;
  let neutral: FanPoint = { x: 0, y: 0 }, home: FanPoint = neutral;
  let previousPoint:FanPoint=neutral;
  let engine: ReturnType<typeof deckJoystick> | null = null;
  const read = () => ({ ...state, origin: { ...state.origin }, point: { ...state.point }, ids: [...state.ids] });
  const publish = () => changed(read());
  function update() {
    const control = fanControl(state.origin, state.point, unit);
    travelled ||= control.distance > unit * .55;
    if (!state.held) { publish(); return; }
    const homeDistance = Math.min(control.distance, Math.hypot(state.point.x - home.x, state.point.y - home.y));
    if(!wasAction){
      const from={dx:state.origin.x-previousPoint.x,up:state.origin.y-previousPoint.y,touch:unit,placement:'right' as const,previous:state.intent};
      const boundary=deckHandoffBoundary(from,{...from,dx:state.origin.x-state.point.x,up:state.origin.y-state.point.y});
      if(boundary){
        const destination=state.point;state.point={x:state.origin.x-boundary.dx,y:state.origin.y-boundary.up};
        update();state.point=destination;
      }
    }
    state.intent=deckControl({dx:state.origin.x-state.point.x,up:state.origin.y-state.point.y,touch:unit,placement:'right',previous:state.intent,homeDistance}).intent;
    if (state.intent === 'keep') {
      state.cancelArmed = true;
      state.armed = false;
      state.lane = 'fan'; state.phase = 'fan'; state.deflection = 0;
      if (state.acquired) engine?.pause();
      publish(); return;
    }
    state.cancelArmed = false;
    const overview = state.intent === 'overview';
    state.armed = overview;
    state.lane = state.intent === 'handoff'?'handoff':state.intent === 'new' ? 'new' : overview ? 'overview' : 'fan';
    state.phase = state.intent === 'new' ? 'new' : overview ? 'overview' : 'fan';
    if (overview || state.intent === 'new'||state.intent==='handoff') { wasAction = true; engine?.suspend(); publish(); return; }
    if (!state.acquired && control.distance >= unit * .7 && Math.abs(control.angle - 45) <= 60) {
      neutral = { ...state.point }; state.acquired = true;
    }
    if (wasAction) { neutral = { ...state.point };cursorBase=state.cursor??0;engine?.resume(0); wasAction = false;publish();return; }
    const travel = state.acquired ? neutral.y - state.point.y + .2 * (neutral.x - state.point.x) : 0;
    state.cursor = Math.max(-2, Math.min(2, cursorBase+travel / (unit * .6)));
    state.deflection = Math.max(-1, Math.min(1, travel / DECK_JOYSTICK_DEFAULTS.reach));
    engine?.move(travel, 0);
  }
  function sync(next: DeckJoystickState) {
    if (next.phase !== 'pending' && next.phase !== 'steering') return;
    const entered = next.phase === 'steering' && !state.held;
    state.held ||= next.phase === 'steering';
    state.phase = state.armed ? 'overview' : state.intent === 'new' ? 'new' : state.held ? 'fan' : 'pending';
    state.hot = state.held ? next.preview : null;
    state.position = next.position; state.target = next.target;
    state.direction = next.direction; state.tier = next.tier; state.interval = next.interval; state.pulse = next.pulse;
    publish();
    if (entered) update();
  }
  const dispose = () => { engine?.dispose(); engine = null; };
  return {
    read, dispose,
    start(origin: FanPoint, ids: readonly string[], active: string, touch = 44, cancelSeat: FanPoint = origin) {
      dispose();
      initial = Math.max(0, ids.indexOf(active)); unit = touch; travelled = wasAction = false;cursorBase=0;
      neutral = { ...origin }; home = { ...cancelSeat };
      previousPoint={...origin};
      state = { ...idle(), phase: 'pending', origin: { ...origin }, point: { ...origin }, ids: [...ids], position: initial, target: initial, opened: initial };
      engine = deckJoystick(ids.length, initial, sync, () => {}, () => {}, clock);
      engine.start(0, 0, unit);
    },
    move(point: FanPoint) { if (state.phase !== 'idle') { previousPoint=state.point;state.point = { ...point }; update(); } },
    cancel() { dispose(); state = idle(); publish(); },
    release(): FanResult {
      if (state.phase === 'idle') return { kind: 'none' };
      const before = read(); dispose();
      let result: FanResult = { kind: 'none' };
      if (!before.cancelArmed) {
        if (before.intent === 'new') result = { kind: 'new' };
        else if (before.armed) result = { kind: 'overview' };
        else if (before.intent==='switch'&&before.held && before.hot !== null && before.hot !== initial) result = { kind: 'focus', id: before.ids[before.hot] };
        else if (!before.held && !travelled) result = { kind: 'tap' };
      }
      state = idle(); publish(); return result;
    },
  };
}
