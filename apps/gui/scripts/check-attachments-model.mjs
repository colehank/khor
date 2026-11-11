// Exercise production selection/staging helpers with a controlled byte
// transport. Browser integration is FACE_ONLY=term's attachments section.
import assert from "node:assert/strict";
import { build } from "esbuild";
const calls = [];
globalThis.__attachmentTransport = (method, args) => {
  calls.push([method, ...args]);
  if (method === "termDropFolder") return Promise.resolve(`/stage/${args[0]}`);
  if (method === "termDropStage") return Promise.resolve(`/stage/${args[0].name}`);
  return Promise.resolve("/stage/child");
};
const result = await build({ entryPoints: ["src/lib/term-attachments.ts"], bundle: true, write: false, format: "esm", platform: "node", plugins: [{ name: "transport", setup(build) { build.onResolve({ filter: /^@\/api$/ }, () => ({ path: "api", namespace: "fixture" })); build.onLoad({ filter: /.*/, namespace: "fixture" }, () => ({ contents: ["termDropFolder", "termDropForget", "termDropStage", "termDropStageInto"].map((n) => `export const ${n} = (...args) => globalThis.__attachmentTransport("${n}", args);`).join("\n") })); } }] });
const m = await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`);
const child = m.pathAttachment("/data/a.txt", "peer", "peer");
const parent = m.pathAttachment("/data", "peer", "peer", true);
const nearName = m.pathAttachment("/database/a", "peer", "peer");
const other = m.pathAttachment("/data/a.txt", "another", "another");
assert.deepEqual(m.addAttachments([child, nearName, other], [parent]), [nearName, other, parent]);
assert.deepEqual(m.addAttachments([parent], [child, parent]), [parent]);
assert(!m.includesAttachment(parent, nearName));
const win = m.pathAttachment(String.raw`C:\Work\DATA`, "win", "win", true);
assert(m.includesAttachment(win, m.pathAttachment("c:/work/data/sub/a", "win", "win")));
assert.equal(m.parentAttachmentPath(String.raw`C:\Work`), "C:\\");
assert.equal(m.parentAttachmentPath("C:\\"), null);
assert.equal(m.parentAttachmentPath("/"), null);
assert.equal(m.joinAttachmentPath(String.raw`C:\Work`, "a"), String.raw`C:\Work\a`);
const f = (name, path) => { const file = new File(["abc"], name); Object.defineProperty(file, "webkitRelativePath", { value: path }); return file; };
const picked = m.deviceAttachments([f("a.txt", "data/nested/a.txt"), f("b.txt", "data/b.txt")], true, "here");
assert.deepEqual(picked[0].device.dirs, ["nested"]);
assert.equal(picked[0].size, 6);
const browse = m.deviceFolderEntries(picked[0]);
assert.deepEqual(browse.map(row=>[row.name,row.dir]),[["nested",true],["b.txt",false]]);
assert.equal(m.deviceFolderEntries(picked[0],"nested")[0].file,picked[0].device.files[0].file,'Preview retains the original selected File without staging or changing selection');
assert.deepEqual(calls,[],'Listing a selected folder never contacts the staging transport');
assert.throws(()=>m.deviceFolderEntries(picked[0],"../other"));
assert.deepEqual(m.deviceFolderEntries({...picked[0],device:{files:[],dirs:["empty"]}},"empty"),[]);
const names=m.deviceAttachments([f(".hidden","data/.hidden"),f("z.txt","data/z.txt"),f("A.txt","data/A.txt"),f("inside","data/B/inside")],true,"here");
assert.deepEqual(m.deviceFolderEntries(names[0]).map(row=>row.name),["B","A.txt","z.txt",".hidden"],'Local ordering matches node/files.rs instead of browser enumeration order');
const native = m.pathAttachment("/native/data", null, "here", true, null, true);
const made = await m.prepareAttachments([other, native, ...picked], () => false);
assert.deepEqual(made.items, [{ path: other.path, from: "another", keep: false }, { path: native.path, from: null, keep: true }, { path: "/stage/data", from: null, keep: false }]);
assert.deepEqual(made.staged, ["/stage/data"]);
assert.deepEqual(calls.filter(([method]) => method === "termDropStageInto").map(([, , path]) => path), ["nested/a.txt", "b.txt"]);
calls.length = 0;
let stopped = false;
globalThis.__attachmentTransport = (method, args) => { calls.push([method, ...args]); if (method === "termDropFolder") { stopped = true; return Promise.resolve("/stage/cancelled"); } return Promise.resolve(null); };
assert.equal(await m.prepareAttachments(picked, () => stopped), null);
assert.deepEqual(calls.map(([method]) => method), ["termDropFolder", "termDropForget"]);
assert.deepEqual(calls[1][1], ["/stage/cancelled"]);
// Browser drop enumeration retains empty directories and continues
// through readEntries batches (browser may cap a batch at 100 entries).
const fileEntry = { name: "a.txt", fullPath: "/data/a.txt", isFile: true, isDirectory: false, file: (ok) => ok(f("a.txt", "")) };
const dirEntry = (name, children) => ({ name, fullPath: `/${name}`, isFile: false, isDirectory: true, createReader: () => { let next = 0; return { readEntries: (ok) => ok(next++ === 0 ? children : []) }; } });
const root = dirEntry("data", [dirEntry("empty", []), fileEntry]);
const drop = await m.droppedAttachments({ items: [{ kind: "file", webkitGetAsEntry: () => root }], files: [] }, "here");
assert.deepEqual(drop[0].device.dirs, ["empty"]);
assert.equal(drop[0].device.files[0].relative, "a.txt");
assert.throws(() => m.deviceAttachments([f("a", "data/../a")], true, "here"));
console.log("attachment model: source-aware selection, Windows paths, hierarchy, empty folders, staging ownership and cancellation passed");
