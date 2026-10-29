/** Right-thumb travel adapts the same continuous engine as the bottom pill. */
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickClock,type DeckJoystickState} from '@/lib/deck-joystick';
export type FanPoint={x:number;y:number};
export type FanState={phase:'idle'|'pending'|'fan'|'overview';lane:'fan'|'overview'|null;held:boolean;origin:FanPoint;point:FanPoint;ids:string[];hot:number|null;position:number;target:number;armed:boolean;cancelArmed?:boolean;direction:number;tier:number;interval:number;pulse:number;deflection:number;cursor?:number;acquired:boolean};
export type FanResult={kind:'none'|'tap'|'overview'}|{kind:'focus';id:string};
export function fanControl(origin:FanPoint,point:FanPoint,unit=44,placement:'right'|'bottom'='right',armed=false){
  const dx=origin.x-point.x,up=origin.y-point.y,distance=Math.hypot(dx,up),angle=Math.atan2(up,dx)*180/Math.PI,neutral=placement==='bottom'?90:45;
  return {distance,angle,overview:Math.abs(angle-neutral)<=18&&distance>unit*(armed?2.7:3.2),gain:distance>unit*.55&&Math.abs(angle-neutral)<=40?(angle-neutral)/30*DECK_JOYSTICK_DEFAULTS.reach:0};
}
export function thumbFanJoystick(changed: (state: FanState) => void, clock?: DeckJoystickClock) {
  const idle = (): FanState => ({
    phase: 'idle', lane: null, held: false, origin: { x: 0, y: 0 }, point: { x: 0, y: 0 },
    ids: [], hot: null, armed: false, cancelArmed: false, direction: 0, tier: 0,
    interval: DECK_JOYSTICK_DEFAULTS.slowMs, pulse: 0, deflection: 0, cursor: 0,
    acquired: false, position: 0, target: 0,
  });
  let state = idle();
  let initial = 0, unit = 44, radius = 0, travelled = false, wasOverview = false;
  let neutral: FanPoint = { x: 0, y: 0 }, home: FanPoint = neutral;
  let engine: ReturnType<typeof deckJoystick> | null = null;
  const read = () => ({ ...state, origin: { ...state.origin }, point: { ...state.point }, ids: [...state.ids] });
  const publish = () => changed(read());
  function update() {
    const control = fanControl(state.origin, state.point, unit);
    travelled ||= control.distance > unit * .55;
    if (!state.held) { publish(); return; }
    const homeDistance = Math.min(control.distance, Math.hypot(state.point.x - home.x, state.point.y - home.y));
    if (state.acquired && homeDistance <= unit * (state.cancelArmed ? .45 : .3)) {
      state.cancelArmed = true;
      state.armed = false;
      state.lane = 'fan'; state.phase = 'fan'; state.cursor = 0; state.deflection = 0;
      engine?.pause(); publish(); return;
    }
    state.cancelArmed = false;
    const gate = Math.max(unit * 3.2, radius + unit * 1.5);
    const overview = Math.abs(control.angle - 45) <= 22 && control.distance > gate - (state.armed ? unit * .5 : 0);
    state.armed = overview;
    state.lane = overview ? 'overview' : 'fan';
    state.phase = overview ? 'overview' : 'fan';
    if (overview) { wasOverview = true; engine?.pause(); publish(); return; }
    if (!state.acquired && control.distance >= unit * .7 && Math.abs(control.angle - 45) <= 60) {
      neutral = { ...state.point }; radius = Math.min(control.distance, unit * 1.2); state.acquired = true;
    }
    if (wasOverview) { neutral = { ...state.point }; wasOverview = false; }
    const travel = state.acquired ? neutral.y - state.point.y + .2 * (neutral.x - state.point.x) : 0;
    state.cursor = Math.max(-2, Math.min(2, travel / (unit * .6)));
    state.deflection = Math.max(-1, Math.min(1, travel / DECK_JOYSTICK_DEFAULTS.reach));
    engine?.move(travel, 0);
  }
  function sync(next: DeckJoystickState) {
    if (next.phase !== 'pending' && next.phase !== 'steering') return;
    const entered = next.phase === 'steering' && !state.held;
    state.held ||= next.phase === 'steering';
    state.phase = state.armed ? 'overview' : state.held ? 'fan' : 'pending';
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
      initial = Math.max(0, ids.indexOf(active)); unit = touch; radius = 0; travelled = wasOverview = false;
      neutral = { ...origin }; home = { ...cancelSeat };
      state = { ...idle(), phase: 'pending', origin: { ...origin }, point: { ...origin }, ids: [...ids], position: initial, target: initial };
      engine = deckJoystick(ids.length, initial, sync, () => {}, () => {}, clock);
      engine.start(0, 0);
    },
    move(point: FanPoint) { if (state.phase !== 'idle') { state.point = { ...point }; update(); } },
    cancel() { dispose(); state = idle(); publish(); },
    release(): FanResult {
      if (state.phase === 'idle') return { kind: 'none' };
      const before = read(); dispose();
      let result: FanResult = { kind: 'none' };
      if (!before.cancelArmed) {
        if (before.armed) result = { kind: 'overview' };
        else if (before.held && before.hot !== null && before.hot !== initial) result = { kind: 'focus', id: before.ids[before.hot] };
        else if (!before.held && !travelled) result = { kind: 'tap' };
      }
      state = idle(); publish(); return result;
    },
  };
}
