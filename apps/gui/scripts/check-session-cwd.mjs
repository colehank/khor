import assert from "node:assert/strict";
import { sessionCwd, sessionCwdOnScreen, workspaceCwd, followsSessionCwd, sessionInWorkspace } from "../src/lib/session-cwd.ts";
const row = { place: "/birth", doing: { cwd: "/after-cd" } };
assert.equal(sessionCwd(row), "/after-cd", "workspace follows cd, not birthplace");
assert.equal(sessionCwd(row, "/live-prompt"), "/after-cd", "runtime shell reading is authoritative");
row.mux = true;
assert.equal(sessionCwd(row, "/old-pane"), "/after-cd", "active pane snapshot outranks a stale terminal OSC mark");
row.doing.cwd = "/second-pane";
assert.equal(sessionCwd(row, "/old-pane"), "/second-pane", "pane switch changes the default workspace");
row.doing.cwd = "/after-cd";
assert.equal(sessionCwd(row), "/after-cd", "returning to the original pane reads its current directory");
assert.equal(sessionCwd({ place: "/birth" }), "/birth");
assert.equal(sessionCwd({ place: "/birth", doing: { cwd: "" } }, ""), "/birth");
assert.equal(sessionCwd(null), null);
assert.equal(workspaceCwd(row, { dir: "/previous", follow: true }), "/after-cd", "default side workspace follows cwd");
assert.equal(workspaceCwd(row, { dir: "/chosen", follow: false }), "/chosen", "manual workspace navigation stays pinned");
assert.equal(workspaceCwd(row, { dir: "/chosen" }), "/chosen", "saved manually navigated locations stay pinned");
globalThis.document = { querySelector: () => ({ dataset: { keptTerminal: "other-tab" }, querySelector: () => ({ __marks: { cwd: () => "/wrong-tab" } }) }) };
assert.equal(sessionCwdOnScreen({ ...row, id: "current-tab", mux: false }), "/after-cd", "kept terminal from another tab cannot supply cwd");
globalThis.document.querySelector = () => ({ dataset: { keptTerminal: "current-tab" }, querySelector: () => ({ __marks: { cwd: () => "/immediate-cd" } }) });
assert.equal(sessionCwdOnScreen({ ...row, id: "current-tab", mux: false }), "/after-cd", "nested tmux keeps its runtime cwd even with native shell transport");
assert.equal(sessionCwdOnScreen({ id: "current-tab", place: "/birth" }), "/immediate-cd", "OSC remains a fallback when runtime cwd is unknown");
assert.equal(sessionCwdOnScreen({ ...row, id: "current-tab", mux: true }), "/after-cd");
delete globalThis.document;
assert(followsSessionCwd(row, { dir: "/birth", start: null, spot: null }), "old birthplace default migrates to following");
assert(!followsSessionCwd(row, { dir: "/birth", start: { path: "/birth/file" } }), "old explicit file remains pinned");
assert(!followsSessionCwd(row, { dir: "/birth", spot: { file: "/birth/file" } }), "old open file remains pinned");
assert(!followsSessionCwd(row, { dir: "/chosen" }), "old explicit directory remains pinned");
console.log("session cwd: passed");

// Exercise the real filter options and matching table, not a duplicate policy.
const { build } = await import("esbuild");
const bundle = await build({
  stdin: { contents: 'export { SESSION_SIEVE, sessionOptions, visibleSessions, sessionSearchText } from "./src/lib/session-filter"; export { sessionName } from "./src/lib/session-name";', resolveDir: new URL("..", import.meta.url).pathname, loader: "ts" },
  bundle: true, write: false, format: "esm", platform: "node",
});
const { SESSION_SIEVE, sessionOptions, visibleSessions, sessionSearchText, sessionName } = await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`);
const moving = { id: "moving", title: "shell", home: "MacLab", word: "busy", category: "codex", place: "/birth", doing: { cwd: "/work/ACL2027" } };
const axis = SESSION_SIEVE.find((a) => a.key === "at:");
assert.equal(axis.pick(moving), "/work/ACL2027", "location filter follows cd");
assert(sessionOptions([moving], []).some((o) => o.key === "at:/work/ACL2027"), "new cwd offered in menu");
assert(!sessionOptions([moving], []).some((o) => o.key === "at:/birth"), "birthplace not offered for moved row");
assert.deepEqual(visibleSessions([moving], "", ["at:/work/ACL2027"]), [moving]);
assert.deepEqual(visibleSessions([moving], "", ["at:/birth"]), [], "old directory no longer matches");
assert.deepEqual(visibleSessions([moving], "acl2027", []), [moving], "search uses live cwd");
assert.deepEqual(visibleSessions([moving], "/birth", []), [], "search does not use birthplace");
assert(sessionSearchText(moving).includes("/work/ACL2027"));
assert(sessionInWorkspace(moving, "MacLab", "/work"));
assert(sessionInWorkspace(moving, "MacLab", "/work/ACL2027"));
assert(!sessionInWorkspace(moving, "MacLab", "/birth"));
assert(!sessionInWorkspace(moving, "other-machine", "/work"));
assert(!sessionInWorkspace(moving, null, "/work"));
assert(!sessionInWorkspace(moving, "MacLab", "/wor"), "directory component boundary");
assert(!sessionInWorkspace(moving, "MacLab", ""));
moving.doing.cwd = "/other-pane";
assert.equal(axis.pick(moving), "/other-pane", "filter follows pane switch");
assert(sessionOptions([moving], ["at:/work/ACL2027"]).some((o) => o.key === "at:/work/ACL2027"), "previous tick remains removable");
assert(!sessionInWorkspace(moving, "MacLab", "/work"), "pane switch leaves former workspace");
assert(sessionInWorkspace(moving, "MacLab", "/other-pane"));
for (const cwd of ["", "  ", null]) {
  moving.doing.cwd = cwd;
  assert.equal(axis.pick(moving), "/birth");
  assert(sessionInWorkspace(moving, "MacLab", "/birth"));
}
moving.place = null;
assert.equal(axis.pick(moving), "");
assert(!sessionInWorkspace(moving, "MacLab", "/birth"));
moving.doing.cwd = String.raw`C:\work\project`;
assert(sessionInWorkspace(moving, "MacLab", String.raw`C:\work`));
assert(!sessionInWorkspace(moving, "MacLab", String.raw`C:\wor`));
console.log("session location filter, search and workspace membership: passed");

const namedRow = {title:'shell',place:'/birth',doing:{cwd:'/work/ACL2027',title:'✳ Claude generated title',command:'claude'}};
assert.equal(sessionName(namedRow),'ACL2027','automatic OSC titles do not name sessions');
namedRow.doing.cwd=String.raw`C:\work\project`;
assert.equal(sessionName(namedRow),'project','Windows current directory names match Unix');
assert.equal(sessionName({...namedRow,title:'My session'}),'My session','explicit user name wins');
assert.equal(sessionName({...namedRow,title:'✳ My session'}),'✳ My session','user emoji names remain literal');
assert.equal(sessionName({title:'shell',place:'/birth'}),'birth','birthplace is only a fallback');
assert.equal(sessionName({title:'shell',doing:{cwd:'/',title:'automatic'}}),'/');
assert.equal(sessionName({title:'shell',doing:{title:'automatic'}}),'shell','missing cwd never restores an agent title');
console.log('stable session names: passed');
