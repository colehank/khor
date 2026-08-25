// The one motion number the TS side owns: how long a pressed control
// wears its outcome word (复制好了, 开不了, 收不下…) before yielding
// the offer back. CSS twin: `--time-yield` in tokens.css — setTimeout
// cannot read a custom property without a layout pass, so the number
// lives twice under one name and `check-type.mjs` holds the two equal.
export const YIELD_MS = 2_000;
