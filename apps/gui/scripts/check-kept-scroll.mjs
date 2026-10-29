// Execute the production restoration core with a deterministic frame clock.
// DOM sizing and CodeMirror restoration are covered separately in the Lab.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { transform } from "esbuild";

const source = readFileSync("src/hooks/use-kept-scroll.ts", "utf8");
const start = source.indexOf("function restoreScroll(");
const end = source.indexOf("\nexport function useKeptScrollRef", start);
assert(start >= 0 && end > start, "production restoration core exists");
const { code } = await transform(source.slice(start, end), { loader: "ts" });

function fixture() {
  let id = 0, settles = 0, completed = 0;
  const frames = new Map();
  const restore = new Function("settleBands", "requestAnimationFrame", "cancelAnimationFrame", `${code}\nreturn restoreScroll;`)(
    () => { settles++; },
    run => { frames.set(++id, run); return id; },
    key => frames.delete(key),
  );
  return {
    restore: (wanted, nodes) => restore(wanted, () => nodes, () => { completed++; }),
    frame: () => {
      const work = [...frames.values()];
      frames.clear();
      for (const run of work) run();
    },
    get pending() { return frames.size; },
    get completed() { return completed; },
    get settles() { return settles; },
  };
}
const node = (height = 1000) => ({ scrollTop: 0, scrollHeight: height, clientHeight: 100, style: { overflowAnchor: "" } });
const results = [];
function test(name, run) {
  run();
  results.push({ name, pass: true });
  console.log("PASS", name);
}

test("ready content restores synchronously and settles band geometry on both sides", () => {
  const f = fixture(), current = node();
  f.restore([{ top: 700, height: 1000 }], [current]);
  assert.equal(current.scrollTop, 700);
  assert.equal(current.style.overflowAnchor, "");
  assert.equal(f.settles, 2);
  assert.equal(f.completed, 1);
  assert.equal(f.pending, 0);
});
test("late content preserves the target until its recorded height is ready", () => {
  const f = fixture(), current = node(400);
  f.restore([{ top: 200, height: 1000 }], [current]);
  assert.equal(current.scrollTop, 0);
  assert.equal(current.style.overflowAnchor, "none");
  f.frame();
  current.scrollHeight = 1000;
  f.frame();
  assert.equal(current.scrollTop, 200);
  assert.equal(f.completed, 1);
  assert.equal(f.pending, 0);
});
test("the original 45-frame height grace remains bounded", () => {
  const f = fixture(), current = node(400);
  f.restore([{ top: 200, height: 1000 }], [current]);
  for (let i = 0; i < 45; i++) f.frame();
  assert.equal(current.scrollTop, 0);
  f.frame();
  assert.equal(current.scrollTop, 200);
  assert.equal(f.pending, 0);
  assert.equal(f.completed, 1);
});
test("an unreachable position stops after the original 60-frame budget", () => {
  const f = fixture(), current = node(120);
  f.restore([{ top: 700, height: 1000 }], [current]);
  for (let i = 0; i < 59; i++) f.frame();
  assert.equal(current.scrollTop, 0);
  assert.equal(f.pending, 0);
  assert.equal(f.completed, 1);
  assert.equal(current.style.overflowAnchor, "");
});
test("detaching cancels only its own pending restoration and releases anchoring", () => {
  const f = fixture(), a = node(200), b = node(200);
  const stopA = f.restore([{ top: 700, height: 1000 }], [a]);
  f.restore([{ top: 700, height: 1000 }], [b]);
  assert.equal(f.pending, 2);
  stopA();
  assert.equal(f.pending, 1);
  assert.equal(a.style.overflowAnchor, "");
  assert.equal(b.style.overflowAnchor, "none");
  assert.equal(f.completed, 0);
});
console.log(JSON.stringify({ passed: results.length, total: 5, results }, null, 2));
