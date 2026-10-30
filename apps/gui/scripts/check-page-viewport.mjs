// Real production geometry functions; no browser, native views or mock API.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=fs.readFileSync(new URL('../src/views/BrowserPane.tsx',import.meta.url),'utf8');
const compile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
if(process.argv.includes('--legacy')){
 const old=execFileSync('git',['show','2ba5e983:apps/gui/src/views/BrowserPane.tsx'],{encoding:'utf8'});
 const file=ts.createSourceFile('old.tsx',old,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);let initializer;
 function visit(node){if(ts.isVariableDeclaration(node)&&node.name.getText(file)==='at'&&node.initializer?.getText(file).includes('native.getBoundingClientRect().bottom')){assert.equal(initializer,undefined);initializer=node.initializer.getText(file);}ts.forEachChild(node,visit);}visit(file);assert(initializer);
 const context=vm.createContext({r:{left:0,top:80,width:390,bottom:844},native:{getBoundingClientRect:()=>({top:660,bottom:844})},window:{innerHeight:844}});
 const actual=vm.runInContext(compile(`(${initializer})`),context);
 assert.equal(actual.h,580,'the existing native viewport must end at deck.top, not extend to deck.bottom');
 process.exit(0);
}
const file=ts.createSourceFile('source.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const functions=[];for(const node of file.statements)if(ts.isFunctionDeclaration(node)&&['pageBox','hostBox'].includes(node.name?.text))functions.push(node.getText(file).replace(/^export /,''));assert.equal(functions.length,2);
const context=vm.createContext({window:{innerHeight:844}});vm.runInContext(compile(functions.join('\n')+'\nglobalThis.geometry={pageBox,hostBox};'),context);
const {pageBox,hostBox}=context.geometry;let passed=0;function check(name,run){run();console.log('PASS',name);passed++;}
const rect=(top,bottom,width=390,left=0)=>({left,top,bottom,width,height:bottom-top});
const result=(...args)=>({...pageBox(...args)});
check('expanded deck is outside the actual native page viewport',()=>assert.deepEqual(result(rect(80,844),660,844),{x:0,y:80,w:390,h:580,view:844}));
check('folded deck releases only its real layout height',()=>assert.equal(pageBox(rect(80,844),780,844).h,700));
check('desktop without a deck preserves its original host rectangle',()=>assert.deepEqual(result(rect(60,760,900,240),null,800),{x:240,y:60,w:900,h:700,view:800}));
check('resized keyboard viewport and safe-area origin retain correct coordinates',()=>assert.deepEqual(result(rect(44,420,358,16),320,420),{x:16,y:44,w:358,h:276,view:420}));
check('overlapping/smaller hosts never invent height or extend below themselves',()=>{assert.equal(pageBox(rect(400,500),300,600).h,0);assert.equal(pageBox(rect(40,240),500,600).h,200);});
check('the same production host measurement uses whichever visible deck is higher',()=>{
 const el={getBoundingClientRect:()=>rect(80,844),closest:selector=>({querySelector:()=>({getBoundingClientRect:()=>selector==='[data-session-screen]'?rect(700,844):rect(660,760)})})};
 assert.equal(hostBox(el).h,580);
 el.closest=selector=>({querySelector:()=>({getBoundingClientRect:()=>selector==='[data-session-screen]'?rect(700,844):rect(0,0,0)})});
 assert.equal(hostBox(el).h,620,'a hidden retained local deck has no geometry');
});
console.log(`${passed} native page viewport geometry checks passed`);
