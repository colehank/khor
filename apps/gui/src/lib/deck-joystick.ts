import { DECK_JOYSTICK_HOLD_MS, DECK_JOYSTICK_REPEAT_MS } from './tempo';

export type DeckJoystickTuning = {
  holdMs: number; neutral: number; reach: number;
  slowMs: number; mediumMs: number; fastMs: number;
};
export const DECK_JOYSTICK_DEFAULTS: Readonly<DeckJoystickTuning> = Object.freeze({
  holdMs: DECK_JOYSTICK_HOLD_MS, neutral: 10, reach: 56,
  slowMs: DECK_JOYSTICK_REPEAT_MS[0], mediumMs: DECK_JOYSTICK_REPEAT_MS[1], fastMs: DECK_JOYSTICK_REPEAT_MS[2],
});
export type DeckJoystickState = {
  phase: 'idle' | 'pending' | 'swipe' | 'pull' | 'steering';
  preview: number; position: number; target: number;
  dx: number; dy: number; direction: number; interval: number;
  overview: boolean; tier: number; pulse: number;
};
export type DeckJoystickClock = { now: () => number; request: (f: () => void) => number; cancel: (id: number) => void };
type Motion = { from: number; to: number; at: number };

const FOLLOW_SLOP = 4;
const FOLLOW_STEP = 64;
const SNAP_THRESHOLD = 32;
const SNAP_MS = 160;
const REVERSE_SLOP = 8;

export function deckJoystick(
  count: number, initial: number, changed: (s: DeckJoystickState) => void,
  commit: (index: number) => void, overview: () => void,
  clock: DeckJoystickClock = {
    now: () => performance.now(), request: run => requestAnimationFrame(run), cancel: id => cancelAnimationFrame(id),
  },
  getTuning: () => Readonly<DeckJoystickTuning> = () => DECK_JOYSTICK_DEFAULTS,
) {
  const wrap = (index: number) => count > 0 ? ((index % count) + count) % count : 0;
  const idle = (): DeckJoystickState => ({
    phase: 'idle', preview: wrap(initial), position: wrap(initial), target: wrap(initial),
    dx: 0, dy: 0, direction: 0, interval: DECK_JOYSTICK_DEFAULTS.slowMs,
    overview: false, tier: 0, pulse: 0,
  });
  let state = idle(), tuning = { ...getTuning() };
  let origin = { x: 0, y: 0 }, began = 0, frame = 0;
  let motion: Motion | null = null, autoSide = 0, nextAt = Infinity;
  let parked = false, crossedAt: number | null = null;
  let followX = 0, followPosition = state.position;
  const publish = () => changed({ ...state });
  const stop = () => { clock.cancel(frame); frame = 0; };
  const animate = (target: number) => {
    motion = Math.abs(target - state.position) > 1e-8
      ? { from: state.position, to: target, at: clock.now() } : null;
  };
  const park = () => {
    autoSide = 0; nextAt = Infinity; parked = true; crossedAt = null;
    state.direction = 0; state.tier = 0;
    if (!motion || motion.to !== state.target) animate(state.target);
  };
  const pause = () => { if (state.phase === 'idle') return; park(); publish(); };
  const cancel = () => {
    stop(); motion = null; autoSide = 0; nextAt = Infinity;
    parked = false; crossedAt = null; state = idle(); publish();
  };
  const setGear = (magnitude: number, direction: number) => {
    const previousInterval = state.interval;
    let tier = direction === state.direction ? Math.max(1, state.tier) : 1;
    const medium = tuning.reach * .65, fast = tuning.reach;
    if (direction === state.direction) {
      if (tier < 2 && magnitude >= medium + 2) tier = 2;
      if (tier < 3 && magnitude >= fast + 2) tier = 3;
      if (tier === 3 && magnitude < fast - 8) tier = 2;
      if (tier === 2 && magnitude < medium - 8) tier = 1;
    }
    state.direction = direction; state.tier = tier;
    state.interval = [tuning.slowMs, tuning.slowMs, tuning.mediumMs, tuning.fastMs][tier];
    if (autoSide && !motion && previousInterval !== state.interval) nextAt = clock.now() + state.interval;
  };
  const accept = (side: number) => {
    state.target -= side;
    state.preview = wrap(state.target); state.pulse++;
    autoSide = side; state.direction = -side; nextAt = Infinity;
    animate(state.target);
    if (!motion) nextAt = clock.now() + state.interval;
  };
  const tick = () => {
    frame = 0;
    const now = clock.now();
    if (state.phase === 'pending' && now - began >= tuning.holdMs) {
      state.phase = 'steering'; publish();
    }
    if (state.phase === 'steering') {
      if (motion) {
        const progress = Math.min(1, (now - motion.at) / SNAP_MS);
        state.position = motion.from + (motion.to - motion.from) * (1 - (1 - progress) ** 3);
        if (progress === 1) {
          motion = null;
          nextAt = autoSide && !state.overview ? now + state.interval : Infinity;
        }
        publish();
      } else if (autoSide && !state.overview && now >= nextAt) {
        accept(autoSide); publish();
      }
    }
    if (state.phase !== 'idle') frame = clock.request(tick);
  };
  return {
    read: () => ({ ...state }), cancel, pause,
    restartReading() { if (autoSide && !motion) nextAt = clock.now() + state.interval; },
    dispose() { stop(); motion = null; autoSide = 0; state.phase = 'idle'; },
    start(x: number, y: number) {
      cancel(); tuning = { ...getTuning() }; origin = { x, y }; began = clock.now();
      followX = 0; followPosition = state.position; state.phase = 'pending';
      publish(); frame = clock.request(tick);
    },
    move(x: number, y: number) {
      if (state.phase === 'idle') return;
      const dx = x - origin.x, dy = y - origin.y;
      state.dx = dx; state.dy = dy;
      if (state.phase === 'pending') {
        if (Math.hypot(dx, dy) < 8) return;
        if (Math.abs(dy) >= Math.abs(dx)) {
          if (dy >= 0) { cancel(); return; }
          state.phase = 'pull';
        } else state.phase = 'swipe';
      }
      if (state.phase === 'swipe') { publish(); return; }
      state.overview = dy < -(state.overview ? 32 : 48);
      if (state.phase === 'pull') { publish(); return; }
      if (state.overview) { park(); publish(); return; }
      if (count < 2) { publish(); return; }
      if (autoSide) {
        if (Math.abs(dx) <= tuning.neutral) { park(); publish(); return; }
        if (Math.sign(dx) !== autoSide) {
          park(); crossedAt = dx; publish(); return;
        }
        setGear(Math.abs(dx), -autoSide); publish(); return;
      }
      if (crossedAt !== null) {
        if (Math.abs(dx) <= tuning.neutral) { crossedAt = null; publish(); return; }
        if (Math.sign(dx) !== Math.sign(crossedAt)) { crossedAt = dx; publish(); return; }
        if (Math.abs(dx) < Math.abs(crossedAt) + REVERSE_SLOP) { publish(); return; }
        followX = crossedAt; followPosition = state.position;
        crossedAt = null; parked = false; motion = null;
      } else if (parked) {
        if (Math.abs(dx) <= tuning.neutral) { publish(); return; }
        followX = Math.sign(dx) * tuning.neutral; followPosition = state.position;
        parked = false; motion = null;
      }
      const displacement = dx - followX, magnitude = Math.abs(displacement);
      const excess = Math.max(0, Math.min(SNAP_THRESHOLD, magnitude) - FOLLOW_SLOP);
      const nextTarget = state.target - Math.sign(displacement);
      const distance = Math.min(excess / FOLLOW_STEP, Math.abs(nextTarget - followPosition));
      state.position = followPosition - Math.sign(displacement) * distance;
      if (magnitude <= FOLLOW_SLOP) {
        state.direction = 0; state.tier = 0;
      } else setGear(Math.abs(dx), -Math.sign(displacement));
      if (magnitude >= SNAP_THRESHOLD) accept(Math.sign(displacement));
      publish();
    },
    release() {
      const before = { ...state };
      cancel();
      if (before.phase === 'pending') return 'tap';
      if ((before.phase === 'steering' || before.phase === 'pull') && before.overview) {
        overview(); return 'overview';
      }
      if (before.phase === 'steering' && before.preview !== wrap(initial)) {
        commit(before.preview); return 'commit';
      }
      return 'none';
    },
  };
}
