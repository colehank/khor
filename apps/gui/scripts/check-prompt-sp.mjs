// zsh's prompt mark is made for the face's width (`src/lib/term-prompt-sp.ts`).
import { promptMarks } from "../src/lib/term-prompt-sp.ts";

const E = "\x1b";
const enc = (s) => Uint8Array.from(Buffer.from(s, "latin1"));
const str = (b) => Buffer.from(b).toString("latin1");
const mark = `${E}[1m${E}[7m%${E}[27m${E}[1m${E}[0m`;
const at = (cols, s) => str(promptMarks(() => cols)(enc(s)));
let bad = 0;
const check = (name, ok, why) => { if (!ok) { bad++; console.log(`prompt-sp gate: ${name}: ${why}`); } };
const wide = `out\r\n${mark}${" ".repeat(139)}\r \r${E}]133;A${E}\\prompt % `;
check("narrower", at(48, wide) === `out\r\n${mark}${" ".repeat(47)}\r \r${E}]133;A${E}\\prompt % `, "not refilled to 47");
check("same width", at(140, wide) === wide, "changed at its own width");
check("root's #", at(48, `${E}[1m${E}[7m#${E}[27m${E}[1m${E}[0m${" ".repeat(79)}\r \r`).includes(" ".repeat(47) + "\r \r"), "# mark not refilled");
const text = "50% done\r\nno mark here  \r\n";
check("ordinary text", at(48, text) === text, "text changed");
console.log(bad ? `prompt-sp gate: ${bad} wrong` : "prompt-sp gate: marks refilled to the face's width, text untouched");
process.exit(bad ? 1 : 0);
