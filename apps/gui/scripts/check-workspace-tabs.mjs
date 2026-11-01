import assert from "node:assert/strict";
import { build } from "esbuild";
const source = await build({entryPoints:["src/views/workspace/workspace-tabs.ts"],bundle:true,write:false,platform:"node",format:"esm"});
const m = await import(`data:text/javascript;base64,${Buffer.from(source.outputFiles[0].text).toString("base64")}`);
const {emptyWorkspaceTabs:empty,workspaceTabId:id,openWorkspaceTab:open,pinWorkspaceTab:pin,closeWorkspaceTab:close,selectWorkspaceTab:select,renameWorkspaceFiles:rename}=m;
const file=(path)=>({kind:"file",path});
const commit=(last)=>({kind:"commit",commit:{id:"a".repeat(39)+last,subject:"same prefix"}});
let count=0;
function check(name,run){run();count++;console.log(`ok ${count} - ${name}`);}
check("one preview across files and full-hash commits",()=>{
 let t=open(empty(),file("/a"),false);t=open(t,commit("1"),false);assert.deepEqual(t.items,[commit("1")]);
 t=pin(t,id(commit("1")));t=open(t,file("/b"),false);t=open(t,commit("2"),false);
 assert.deepEqual(t.items,[commit("1"),commit("2")]);t=pin(t,id(commit("2")));t=open(t,commit("1"),false);assert.equal(t.preview,null);
});
check("edited preview remains pinned when a commit opens",()=>{
 let t=open(empty(),file("/a"),false);t=pin(t,id(file("/a")));t=open(t,commit("1"),false);assert.equal(t.items.length,2);
});
check("file diff and commit identities do not collide",()=>{
 let t=open(empty(),file("/a"),true);t=open(t,{kind:"diff",path:"/a"},true);t=open(t,commit("1"),true);assert.equal(new Set(t.items.map(id)).size,3);
});
check("closing selects adjacent content, inactive close preserves active",()=>{
 let t=open(empty(),file("/a"),true);t=open(t,commit("1"),true);t=open(t,file("/b"),true);t=select(t,id(commit("1")));t=close(t,id(commit("1")));assert.equal(t.active,id(file("/b")));t=close(t,id(file("/a")));assert.equal(t.active,id(file("/b")));
});
check("selecting existing pinned content does not consume other preview",()=>{
 let t=open(empty(),commit("1"),true);t=open(t,file("/a"),false);t=open(t,commit("1"),false);assert.equal(t.preview,id(file("/a")));assert.equal(open(t,commit("1"),false),t);
});
check("rename preserves commit and changes file identities only",()=>{
 let t=open(empty(),commit("1"),true);t=open(t,file("/a"),false);const original=t.items[0];t=rename(t,p=>p+"2");assert.equal(t.items[0],original);assert.equal(t.active,id(file("/a2")));assert.equal(t.preview,id(file("/a2")));
});
console.log(`${count} workspace tab checks passed`);
