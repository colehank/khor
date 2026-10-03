// The reply detector (`src/lib/term-replies.ts`): what xterm says back to a
// terminal query is told from what a key, a paste or a mouse report sends.
// The replies are xterm 6's own (src/common/InputHandler.ts), the keys are
// what xterm's own keyboard mapping and our key row produce.
import { isReply } from "../src/lib/term-replies.ts";

const E = "\x1b";
const replies = {
  "DA1 (xterm's own)": `${E}[?1;2c`,
  DA2: `${E}[>0;276;0c`,
  "OSC 11 colour, ST": `${E}]11;rgb:0000/0000/0000${E}\\`,
  "OSC 10 colour, BEL": `${E}]10;rgb:f4f4/f4f4/efef\x07`,
  "DECRQSS cursor style": `${E}P1$r2 q${E}\\`,
  DECRPM: `${E}[?2026;2$y`,
  "DECRPM, ANSI": `${E}[4;2$y`,
  "DSR status": `${E}[0n`,
  CPR: `${E}[12;40R`,
  XTVERSION: `${E}P>|xterm.js(6.0.0)${E}\\`,
  "window size report": `${E}[8;24;80t`,
  "the owner's run, as one write": `${E}]11;rgb:0000/0000/0000${E}\\${E}[?1;2c${E}P1$r2 q${E}\\${E}[?12;2$y`,
};
const keys = {
  letter: "a",
  word: "hello",
  "Chinese commit": "你好",
  enter: "\r",
  esc: E,
  "Alt+x": `${E}x`,
  up: `${E}[A`,
  "up, application mode": `${E}OA`,
  "Ctrl+Left": `${E}[1;5D`,
  home: `${E}[H`,
  pgup: `${E}[5~`,
  F1: `${E}OP`,
  F5: `${E}[15~`,
  backtab: `${E}[Z`,
  "focus in": `${E}[I`,
  "SGR mouse": `${E}[<0;10;5M`,
  "bracketed paste": `${E}[200~text${E}[201~`,
  "a reply with a key after it": `${E}[?1;2ca`,
};

let bad = 0;
for (const [what, s] of Object.entries(replies)) {
  if (!isReply(s)) {
    console.error(`replies gate: ${what} ${JSON.stringify(s)} is a reply and was not taken for one`);
    bad++;
  }
}
for (const [what, s] of Object.entries(keys)) {
  if (isReply(s)) {
    console.error(`replies gate: ${what} ${JSON.stringify(s)} is input and was taken for a reply`);
    bad++;
  }
}
if (bad) process.exit(1);
console.log(`replies gate: ${Object.keys(replies).length} replies told from ${Object.keys(keys).length} kinds of input`);
