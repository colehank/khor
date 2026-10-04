// An iTerm2 inline image without `size=` is given one before the image
// addon sees it (`src/lib/term-iip.ts`); everything else passes untouched.
import { iipSizes } from "../src/lib/term-iip.ts";

const E = "\x1b";
const enc = (s) => Uint8Array.from(Buffer.from(s, "latin1"));
const utf = (s) => Uint8Array.from(Buffer.from(s, "utf8"));
const str = (b) => Buffer.from(b).toString("latin1");
const png = Buffer.alloc(1234, 7);
const b64 = png.toString("base64");
const run = (chunks) => { const f = iipSizes(); return chunks.map((c) => str(f(c))).join(""); };
let bad = 0;
const check = (name, ok, why) => { if (!ok) { bad++; console.log(`iip gate: ${name}: ${why}`); } };

const unsized = `${E}]1337;File=inline=1;width=12;preserveAspectRatio=1:${b64}\x07`;
const want = `${E}]1337;File=size=1234;inline=1;width=12;preserveAspectRatio=1:${b64}\x07`;
const whole = `before\r\n${unsized}\r\nafter`;
check("no size", run([enc(whole)]) === `before\r\n${want}\r\nafter`, "size not written as the decoded length");
const st = `${E}]1337;File=inline=1:${b64}${E}\\`;
check("ST", run([enc(st)]) === `${E}]1337;File=size=1234;inline=1:${b64}${E}\\`, "ST-ended sequence not sized");
const has = `${E}]1337;File=size=1234;inline=1:${b64}\x07`;
check("has size", run([enc(has)]) === has, "a sized header was changed");
for (let cut = 0; cut <= whole.length; cut += 7) {
  const got = run([enc(whole.slice(0, cut)), enc(whole.slice(cut))]);
  if (got !== `before\r\n${want}\r\nafter`) { check(`cut ${cut}`, false, "a chunk boundary changed the output"); break; }
}
const text = "中文 ✓ é \x1b]0;title\x07 \x1b]1;x\x07 tail";
check("utf-8 and other OSC", str(iipSizes()(utf(text))) === str(utf(text)), "bytes changed");
const f = iipSizes();
const a = str(f(enc("x\x1b]1"))), b = str(f(enc("0;t\x07y")));
check("held prefix", a + b === "x\x1b]10;t\x07y", `got ${JSON.stringify(a + b)}`);
console.log(bad ? `iip gate: ${bad} wrong` : "iip gate: sizes written, everything else byte for byte");
process.exit(bad ? 1 : 0);
