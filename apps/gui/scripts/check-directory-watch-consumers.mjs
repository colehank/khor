import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
const files={tree:"src/views/FileTree.tsx",workspace:"src/views/WorkspacePane.tsx",store:"src/views/StorePage.tsx",side:"src/views/SideWorkspace.tsx",session:"src/views/SessionContent.tsx",picker:"src/views/MobileSessionContent.tsx"};
const parsed=Object.fromEntries(Object.entries(files).map(([key,path])=>[key,ts.createSourceFile(path,readFileSync(path,"utf8"),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX)]));
function find(file,predicate){const source=parsed[file],found=[];const visit=node=>{if(predicate(node,source))found.push(node);ts.forEachChild(node,visit);};visit(source);assert.equal(found.length,1,`one real ${file} callback must match`);return {node:found[0],source};}
function effect(file,needle){const {node,source}=find(file,(node,source)=>ts.isCallExpression(node)&&node.expression.getText(source)==="useEffect"&&node.arguments[0]?.getText(source).includes(needle));return node.arguments[0].getText(source);}
function callback(text,values){const compiled=ts.transpileModule(`return (${text});`,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;return Function(...Object.keys(values),compiled)(...Object.values(values));}
const treeEffect=effect("tree","const batch = observed.batch");const workspaceEffect=effect("workspace","const batch = observation.batch");const storeEffect=effect("store","const batch = observation.batch");
const batch=updates=>({cursor:"one",reset:false,dirs:[],workspace:false,coverage:[],retry_ms:0,...updates});
let count=0;function check(name,run){run();console.log(`ok ${++count} - ${name}`);}
check("tree event matches canonical alias directories without relisting unrelated levels",()=>{
 const changed=[];const values={hidden:false,watchActive:true,visibleDirs:["/asked","/asked/link","/asked/other"],listings:new Map([["/asked/link",{path:"/physical"}]]),refresh:dirs=>changed.push(dirs)};
 callback(treeEffect,{...values,observed:{status:"watching",batch:batch({dirs:["/physical"]})}})();assert.deepEqual(changed,[["/asked/link"]]);
 callback(treeEffect,{...values,observed:{status:"watching",batch:batch({reset:true})}})();assert.deepEqual(changed[1],values.visibleDirs);
});
check("hidden, inactive, failed and still-pending observers cannot trigger tree reads",()=>{
 for(const patch of [{hidden:true},{watchActive:false},{observed:{status:"failed",batch:batch({reset:true})}},{observed:{status:"pending",batch:null}}]){
  let reads=0;callback(treeEffect,{hidden:false,watchActive:true,visibleDirs:["/a"],listings:new Map(),observed:{status:"watching",batch:batch({reset:true})},refresh:()=>reads++,...patch})();assert.equal(reads,0);
 }
});
check("busy directory invalidations coalesce as intent while idle levels become readable",()=>{
 const {node,source}=find("tree",node=>ts.isVariableDeclaration(node)&&node.name.getText(parsed.tree)==="refresh");
 const asked={current:new Set(["/busy","/idle"])},reading={current:new Set(["/busy"])},rereadNeeded={current:new Set()};let updates=0;
 const refresh=callback(node.initializer.getText(source),{asked,reading,rereadNeeded,setWant:fn=>{updates++;fn(new Set());}});
 for(let i=0;i<100;i++)refresh(["/busy","/idle"]);
 assert.deepEqual([...rereadNeeded.current],["/busy"]);assert.equal(asked.current.has("/busy"),true);assert.equal(asked.current.has("/idle"),false);assert.equal(updates,100,"React updates coalesce render; no physical read occurs in invalidation callback");
});
check("workspace refreshes metadata and current file parent through separate existing seams",()=>{
 let metadata=0,readAt=0;const values={memory:{metadata:{invalidate:()=>metadata++}},fileParent:"/outside",setReadAt:fn=>{readAt=fn(readAt);}};
 callback(workspaceEffect,{...values,observation:{status:"watching",batch:batch({workspace:true,dirs:["/elsewhere"]})}})();assert.equal(metadata,1);assert.equal(readAt,0);
 callback(workspaceEffect,{...values,observation:{status:"watching",batch:batch({dirs:["/outside"]})}})();assert.equal(metadata,1);assert.equal(readAt,1);
 callback(workspaceEffect,{...values,observation:{status:"watching",batch:batch({reset:true})}})();assert.equal(metadata,2);assert.equal(readAt,2);
 callback(workspaceEffect,{...values,observation:{status:"failed",batch:batch({reset:true})}})();assert.equal(metadata,2);assert.equal(readAt,2);
});
check("storage accepts its own parent pulses and forwards its read counter to actual FilePage",()=>{
 let again=0,readAt=0;const values={dir:"/receive",fileParent:"/old-place",setReadAt:fn=>{readAt=fn(readAt);},level:path=>path.replace(/\/+$/,"")||"/",setAgain:fn=>{again=fn(again);}};
 callback(storeEffect,{...values,observation:{status:"watching",batch:batch({dirs:["/receive/"]})}})();assert.equal(again,1);assert.equal(readAt,0);
 callback(storeEffect,{...values,observation:{status:"watching",batch:batch({dirs:["/old-place"]})}})();assert.equal(again,1);assert.equal(readAt,1,"a file from a previous receive location keeps its own parent");
 const {node,source}=find("store",node=>ts.isJsxAttribute(node)&&node.name.getText(parsed.store)==="readAt");assert.equal(callback(`()=>${node.initializer.expression.getText(source)}`,{readAt})(),1);
});
check("cached tree fallback entry rereads immediately even when begin fails and no focus event arrives",()=>{
 const fallbackEffect=effect("tree","const slow = window.setInterval(again, UNHEARD_MS)");let reads=[];
 const window={setInterval:()=>1,clearInterval:()=>{},addEventListener:()=>{},removeEventListener:()=>{}};
 callback(fallbackEffect,{hidden:false,watchActive:true,fallback:["/cached"],listings:new Map([["/cached",{path:"/cached"}]]),refresh:dirs=>reads.push(dirs),window,document:{hidden:false},UNHEARD_MS:60000})();assert.deepEqual(reads,[["/cached"]]);
});
check("storage fallback reentry reads the retained location without relying on focus",()=>{
 const fallbackEffect=effect("store","const slow = window.setInterval(reread, UNHEARD_MS)");let reads=0;
 const window={setInterval:()=>1,clearInterval:()=>{},addEventListener:()=>{},removeEventListener:()=>{}};
 callback(fallbackEffect,{dir:"/receive",hidden:false,covered:false,fileCovered:true,fileParent:null,listed:{dir:"/receive",rows:[]},setAgain:fn=>{reads=fn(reads);},window,document:{hidden:false},UNHEARD_MS:60000})();assert.equal(reads,1);
});
check("sidebar visible file has its own exact parent observer and readAt seam",()=>{
 const notify=effect("side","const batch = observation.batch");let readAt=0;const values={visible:true,fileParent:"/outside",setReadAt:fn=>{readAt=fn(readAt);}};
 callback(notify,{...values,observation:{status:"watching",batch:batch({dirs:["/outside"]})}})();assert.equal(readAt,1);
 callback(notify,{...values,observation:{status:"watching",batch:batch({dirs:["/elsewhere"]})}})();assert.equal(readAt,1);
 const {node,source}=find("side",node=>ts.isJsxAttribute(node)&&node.name.getText(parsed.side)==="readAt");assert.equal(callback(`()=>${node.initializer.expression.getText(source)}`,{readAt})(),1);
});
check("sidebar file visibility return rechecks without a focus event",()=>{
 const returned=effect("side","const returned = !wasVisible.current");let readAt=0;const wasVisible={current:false};
 callback(returned,{wasVisible,visible:true,fileParent:"/outside",setReadAt:fn=>{readAt=fn(readAt);}})();assert.equal(readAt,1);assert.equal(wasVisible.current,true);
});
check("invisible sidebar tree cannot retain its own lease under a file or commit",()=>{
 const {node,source}=find("side",node=>ts.isJsxAttribute(node)&&node.name.getText(parsed.side)==="watchActive");const expression=`()=>${node.initializer.expression.getText(source)}`;
 assert.equal(callback(expression,{active:true,file:null,commit:null})(),true);assert.equal(callback(expression,{active:true,file:"/a",commit:null})(),false);assert.equal(callback(expression,{active:true,file:null,commit:{id:"hash"}})(),false);assert.equal(callback(expression,{active:false,file:null,commit:null})(),false);
});
check("a retained inactive sidebar cannot consume a late directory pulse",()=>{
 const notify=effect("side","const batch = observation.batch");let reads=0;
 callback(notify,{visible:false,fileParent:"/outside",setReadAt:()=>reads++,observation:{status:"watching",batch:batch({reset:true})}})();
 assert.equal(reads,0,"inactive retained sidebar must not request fresh file content");
});
check("a retained inactive sidebar owns no fallback timer or focus read",()=>{
 const fallback=effect("side","const slow = window.setInterval(refresh, 60_000)");let timers=0,listeners=0;
 callback(fallback,{visible:false,hidden:false,fileParent:"/outside",fileCovered:false,window:{setInterval:()=>++timers,addEventListener:()=>listeners++},document:{hidden:false}})();
 assert.equal(timers,0,"inactive retained sidebar must not start fallback IO");assert.equal(listeners,0);
});
check("retained host visibility and content source reach the actual sidebar observer",()=>{
 function activeAt(file,tag,values){const {node,source}=find(file,(node,source)=>ts.isJsxSelfClosingElement(node)&&node.tagName.getText(source)===tag);const prop=node.attributes.properties.find(prop=>ts.isJsxAttribute(prop)&&prop.name.getText(source)==="active");assert.ok(prop,`${tag} must receive its retained host activity`);return callback(`()=>${prop.initializer.expression.getText(source)}`,values)();}
 const {node,source}=find("side",(node,source)=>ts.isCallExpression(node)&&node.expression.getText(source)==="useDirectoryWatch");
 for(const pickerShow of [false,true])for(const selected of ["workspace","browser"]){
  const active=activeAt("session","ContentPicker",{pickerShow});
  const sidebar=activeAt("picker","SideWorkspace",{active,state:{source:selected}});
  assert.equal(callback(`()=>${node.arguments[2].getText(source)}`,{active:sidebar})(),pickerShow&&selected==="workspace");
 }
});
console.log(`${count} actual directory-watch consumer callback checks passed (no DOM)`);
