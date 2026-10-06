// The snapshot gate (item 46): what the host's screen model hands a
// version-3 attacher (`crates/node/src/host.rs` `Model::snapshot`) shows
// in xterm exactly what the bytes it stands for would have shown — cell
// for cell, with colours and attributes, the cursor, the buffer, and the
// history above the screen. Run in Node against a real xterm, on the
// recorded streams in `scripts/fixtures/snapshot/` (`<name>.raw` is the
// program's bytes, `<name>.snap` the host's snapshot of them, written by
// `cargo test -p khor-node --lib writes_snapshot_fixtures -- --ignored`).
// Each cell's OSC 8 link is compared too (item 66: the snapshot carries
// the links the host kept beside its model), read from xterm's own link
// service as its link provider reads them.
import { build } from "esbuild";
import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const out = await build({
  stdin: { contents: `import { Terminal } from "@xterm/xterm"; globalThis.lab = { Terminal };`, resolveDir: gui, loader: "ts" },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { Terminal } = globalThis.lab;

let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`snapshot gate: ${m}`);
};

/** A fresh 80x24 terminal that has read `bytes`. */
async function shows(bytes) {
  const t = new Terminal({ cols: 80, rows: 24, scrollback: 5000, allowProposedApi: true });
  await new Promise((r) => t.write(bytes, r));
  return t;
}
/** The OSC 8 target of a cell, or "". */
function linkOf(t, c) {
  const id = c.hasExtendedAttrs?.() ? c.extended?.urlId : 0;
  return id ? (t._core._oscLinkService.getLinkData(id)?.uri ?? "?") : "";
}
/** A line as text plus each cell's look, and each cell's link. */
function line(t, i) {
  const l = t.buffer.active.getLine(i);
  if (!l) return null;
  const cells = [];
  const links = [];
  for (let x = 0; x < l.length; x++) {
    const c = l.getCell(x);
    cells.push([c.getChars(), c.getWidth(), c.getFgColorMode(), c.getFgColor(), c.getBgColorMode(), c.getBgColor(), c.isBold(), c.isDim(), c.isItalic(), c.isUnderline(), c.isInverse()].join(","));
    links.push(linkOf(t, c));
  }
  return { text: l.translateToString(true), cells: cells.join("|"), links: links.join("|"), wrapped: l.isWrapped };
}

const dir = `${gui}/scripts/fixtures/snapshot`;
const names = readdirSync(dir).filter((f) => f.endsWith(".raw")).map((f) => f.slice(0, -4));
for (const name of names) {
  const raw = readFileSync(`${dir}/${name}.raw`);
  const snap = readFileSync(`${dir}/${name}.snap`);
  const a = await shows(raw);
  const b = await shows(snap);
  const A = a.buffer.active;
  const B = b.buffer.active;
  const same = [];
  if (A.type !== B.type) fail(`${name}: the buffer is ${B.type}, the stream's is ${A.type}`);
  if (A.cursorX !== B.cursorX || A.cursorY !== B.cursorY) fail(`${name}: the cursor is at ${B.cursorX},${B.cursorY}, the stream's at ${A.cursorX},${A.cursorY}`);
  if (A.length !== B.length) fail(`${name}: ${B.length} lines, the stream's ${A.length}`);
  const n = Math.min(A.length, B.length);
  let wrongCells = 0;
  let linked = 0;
  for (let i = 0; i < n; i++) {
    const x = line(a, i);
    const y = line(b, i);
    if (x.text !== y.text) {
      fail(`${name}: line ${i} reads ${JSON.stringify(y.text)}, the stream's ${JSON.stringify(x.text)}`);
    } else if (x.wrapped !== y.wrapped) {
      fail(`${name}: line ${i} is ${y.wrapped ? "" : "not "}a wrapped continuation, the stream's ${x.wrapped ? "is" : "is not"}`);
    } else if (x.cells !== y.cells) {
      wrongCells++;
      if (wrongCells <= 3) fail(`${name}: line ${i}'s cells differ in look (${JSON.stringify(x.text)})`);
    } else if (x.links !== y.links) {
      fail(`${name}: line ${i}'s links differ (${JSON.stringify(x.text)}): ${JSON.stringify(y.links.replace(/\|+/g, "|"))}, the stream's ${JSON.stringify(x.links.replace(/\|+/g, "|"))}`);
    } else {
      same.push(i);
      if (y.links.replace(/\|/g, "")) linked++;
    }
  }
  if (name === "links" && linked < 40) fail(`${name}: only ${linked} lines carry links`);
  const modes = (t) => `${t.modes.mouseTrackingMode}/${t.modes.bracketedPasteMode}/${t.modes.applicationCursorKeysMode}`;
  if (modes(a) !== modes(b)) fail(`${name}: the modes are ${modes(b)}, the stream's ${modes(a)}`);
  console.log(`snapshot gate: ${name}: ${A.length} lines, ${same.length} identical (${linked} with links), buffer ${B.type}, cursor ${B.cursorX},${B.cursorY}, ${snap.length} B for ${raw.length} B`);
  a.dispose();
  b.dispose();
}
if (names.length === 0) fail("no fixtures");
if (bad) process.exit(1);
console.log("snapshot gate: the host's snapshot shows in xterm what its bytes would have shown");
