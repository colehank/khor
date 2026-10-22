// The motion numbers the TS side owns.
//
// How long a pressed control wears its outcome word (复制好了, 开不了,
// 收不下…) before yielding the offer back. CSS twin: `--time-yield` in
// tokens.css — setTimeout cannot read a custom property without a
// layout pass, so the number lives twice under one name and
// `check-type.mjs` holds the two equal.
export const YIELD_MS = 2_000;

// One blink of the file editor's caret, shown then hidden: VS Code's
// default (`ViewCursors.BLINK_INTERVAL` 500, toggled — the owner
// 2026-10-16:「按 VS Code 来」). CodeMirror restarts it shown on every
// move and every key, so it holds still while keys come and blinks once
// they stop. No CSS twin: CodeMirror writes it on the caret layer
// itself; `npm run caret` reads it there.
export const BLINK_MS = 1_000;

// Original-file transfer progress, independent of animation timing.
export const FILE_HANDOFF_POLL_MS = 200;

// Approved joystick trial: first candidate responds at once, then a stable
// slow / medium / fast reading rhythm. These are not universal reaction times.
export const DECK_JOYSTICK_HOLD_MS = 280;
export const DECK_JOYSTICK_REPEAT_MS = [1400,1000,700] as const;
