// The links gate (item 53's addendum): paths and URLs in a terminal's
// output, found the way `src/lib/term-links.ts` finds them, against a
// real xterm in node and a machine whose directories are a table here.
//
//   - a path the grid wrapped onto the next rows (a phone's 44 columns)
//     is one link, its line mark kept, found from either row;
//   - a URL the grid wrapped is read whole under a tap on its second row;
//   - a name in any script (`报告/图.png`) is a path;
//   - a quoted name with a space in it (`'my file.txt'`, what `ls` prints)
//     is a path;
//   - a path the machine does not have is no link.
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const mocks = {
  name: "mocks",
  setup(b) {
    b.onResolve({ filter: /^@\/api$/ }, () => ({ path: "api", namespace: "mock" }));
    b.onLoad({ filter: /.*/, namespace: "mock" }, () => ({
      contents: "export const fetchLs = (machine, dir) => globalThis.fakeLs(machine, dir);",
      loader: "js",
    }));
    b.onResolve({ filter: /^@\// }, (a) => b.resolve(`./src/${a.path.slice(2)}`, { resolveDir: gui, kind: a.kind }));
  },
};
const out = await build({
  stdin: {
    contents: `import { Terminal } from "@xterm/xterm"; import * as links from "@/lib/term-links"; globalThis.lab = { Terminal, ...links };`,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
  define: { "import.meta.env.DEV": "false" },
  plugins: [mocks],
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { Terminal, pathsOn, urlAt, covers } = globalThis.lab;

let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`links gate: ${m}`);
};

const tree = {
  "/proj": { src: true, "报告": true, "my file.txt": false },
  "/proj/src": { very: true },
  "/proj/src/very": { long: true },
  "/proj/src/very/long": { "name.rs": false },
  "/proj/报告": { "图.png": false },
};
// The listings' expiry timer: nothing expires during a gate.
globalThis.window = { setTimeout: () => 0 };
globalThis.fakeLs = async (_machine, dir) => {
  if (dir === "") return { path: "/home/u", entries: [] };
  const names = tree[dir];
  if (!names) throw new Error("no such directory");
  return { path: dir, entries: Object.entries(names).map(([name, d]) => ({ name, dir: d })) };
};

const t = new Terminal({ cols: 20, rows: 12, allowProposedApi: true });
await new Promise((r) =>
  t.write(
    "error: /proj/src/very/long/name.rs:12:3 bad\r\n" +
      "see https://example.com/a/very/long/path?q=1 ok\r\n" +
      "/proj/报告/图.png\r\n" +
      "open '/proj/my file.txt'\r\n" +
      "/proj/src/nothing.rs\r\n" +
      "\x1b]8;;https://example.com/docs\x1b\\read the docs\x1b]8;;\x1b\\ x\r\n" +
      "\x1b]8;id=f;file:///proj/src/a.rs\x1b\\a.rs\x1b]8;;\x1b\\\r\n",
    r,
  ),
);
const lineOf = (needle) => {
  const b = t.buffer.active;
  for (let y = 0; y < b.length; y++) if (b.getLine(y).translateToString(true).includes(needle)) return y;
  return -1;
};
const cwdAt = () => "/proj";
const said = {};

// The wrapped path: it begins on row `w` and runs on two more rows.
const w = lineOf("error:");
const fromFirst = await pathsOn(t, "m", cwdAt, w);
const fromNext = await pathsOn(t, "m", cwdAt, w + 1);
const wrapped = fromNext.find((f) => f.target.path === "/proj/src/very/long/name.rs");
said.wrapped = wrapped ? `${wrapped.target.path}:${wrapped.target.line}:${wrapped.target.col} ${wrapped.from.y - w}..${wrapped.to.y - w}` : "none";
if (!wrapped) fail("a path the grid wrapped is not a link from its second row");
else {
  if (wrapped.target.line !== 12 || wrapped.target.col !== 3) fail(`the wrapped path lost its line mark: ${said.wrapped}`);
  if (wrapped.to.y === wrapped.from.y) fail("the wrapped path's range does not span rows");
  if (!covers(wrapped, w + 1, 2)) fail("a cell on the wrapped path's second row is not inside it");
}
if (!fromFirst.some((f) => f.target.path === "/proj/src/very/long/name.rs")) fail("a path the grid wrapped is not a link from its first row");

// The wrapped URL, tapped on its second row.
const u = lineOf("see https");
said.url = urlAt(t, u + 1, 3);
if (said.url !== "https://example.com/a/very/long/path?q=1") fail(`a tap on a wrapped URL's second row gave ${JSON.stringify(said.url)}`);

const named = await pathsOn(t, "m", cwdAt, lineOf("/proj/报告"));
said.script = named.map((f) => f.target.path).join(",");
if (!named.some((f) => f.target.path === "/proj/报告/图.png")) fail(`a name in another script is no link (${said.script})`);

const quoted = await pathsOn(t, "m", cwdAt, lineOf("open '"));
said.quoted = quoted.map((f) => f.target.path).join(",");
if (!quoted.some((f) => f.target.path === "/proj/my file.txt")) fail(`a quoted name with a space is no link (${said.quoted})`);

// **A tap on an OSC 8 link** (item 68): its target, though its text is
// not a URL; a file:// link's too (the pane sends it where paths go); a
// cell beside it, nothing.
const d = lineOf("read the docs");
said.osc8 = [urlAt(t, d, 5), urlAt(t, d + 1, 1), urlAt(t, d, 15)];
if (said.osc8[0] !== "https://example.com/docs") fail(`a tap on an OSC 8 link's text gave ${JSON.stringify(said.osc8[0])}`);
if (said.osc8[1] !== "file:///proj/src/a.rs") fail(`a tap on an OSC 8 file link gave ${JSON.stringify(said.osc8[1])}`);
if (said.osc8[2] !== null) fail(`a tap beside an OSC 8 link gave ${JSON.stringify(said.osc8[2])}`);

const missing = await pathsOn(t, "m", cwdAt, lineOf("nothing.rs"));
said.missing = missing.map((f) => f.target.path).join(",");
if (missing.some((f) => f.target.path.endsWith("nothing.rs"))) fail("a path the machine does not have became a link");

t.dispose();
console.log(`links gate: ${JSON.stringify(said)}`);
if (bad) process.exit(1);
console.log("links gate: OSC 8 links tap to their targets, wrapped paths and URLs are whole, any script's names and quoted names with spaces are paths, and only what exists is a link");
